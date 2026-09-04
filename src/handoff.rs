//! Shared plugin ↔ native writer lease and restore handoff.
//!
//! This is a behavioral port of `bridge/cmux_herdr_handoff.py`. Lease and
//! restore files are deliberately written with a temporary sibling followed by
//! rename; like the Python bridge, this does not fsync the file or directory.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};
#[cfg(test)]
pub static HANDOFF_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub const SCHEMA: i64 = 1;
pub const OWNER_PLUGIN: &str = "plugin";
pub const OWNER_NATIVE: &str = "native";
pub const NATIVE_LIVE_ENV: &str = "CMUX_HERDR_NATIVE_LIVE";
pub const FORCE_PLUGIN_ENV: &str = "CMUX_HERDR_FORCE_PLUGIN";
pub const LEASE_TTL_ENV: &str = "CMUX_HERDR_LEASE_TTL_MS";
pub const NATIVE_STATE_ENV: &str = "CMUX_HERDR_NATIVE_STATE_DIR";
pub const DEFAULT_TTL_MS: i64 = 45_000;
pub const OUTCOME_NATIVE_OWNS: &str = "native_owns";
pub const OUTCOME_PLUGIN_OWNS: &str = "plugin_owns";

fn is_owner(owner: &str) -> bool {
    owner == OWNER_PLUGIN || owner == OWNER_NATIVE
}

pub fn env_truthy(name: &str) -> bool {
    let raw = std::env::var(name).unwrap_or_default();
    matches!(
        raw.trim().to_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

pub fn milliseconds_from_seconds(seconds: f64) -> i64 {
    (seconds * 1000.0) as i64
}

pub fn now_ms() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => milliseconds_from_seconds(duration.as_secs_f64()),
        Err(error) => -milliseconds_from_seconds(error.duration().as_secs_f64()),
    }
}

pub fn lease_ttl_ms() -> i64 {
    let raw = std::env::var(LEASE_TTL_ENV).unwrap_or_default();
    let raw = raw.trim();
    if !raw.is_empty() && raw.bytes().all(|byte| byte.is_ascii_digit()) {
        if let Ok(value) = raw.parse::<i64>() {
            return value.max(1_000);
        }
    }
    DEFAULT_TTL_MS
}

#[cfg(unix)]
pub fn pid_alive(pid: i64) -> bool {
    use std::ffi::c_int;

    unsafe extern "C" {
        fn kill(pid: c_int, signal: c_int) -> c_int;
    }

    if pid <= 0 || pid > c_int::MAX as i64 {
        return false;
    }
    let result = unsafe { kill(pid as c_int, 0) };
    if result == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() == Some(1)
}

#[cfg(not(unix))]
pub fn pid_alive(pid: i64) -> bool {
    pid > 0 && pid == std::process::id() as i64
}

#[allow(deprecated)]
fn home_dir() -> PathBuf {
    std::env::home_dir().unwrap_or_else(|| PathBuf::from("~"))
}

pub fn xdg_state_dir() -> PathBuf {
    let root = std::env::var_os("XDG_STATE_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".local/state"));
    root.join("cmux-herdr")
}

pub fn application_support_dir() -> Option<PathBuf> {
    let override_dir = std::env::var(NATIVE_STATE_ENV).unwrap_or_default();
    let override_dir = override_dir.trim();
    if !override_dir.is_empty() {
        return Some(PathBuf::from(override_dir));
    }
    let mac = home_dir().join("Library/Application Support/cmux-herdr");
    mac.is_dir().then_some(mac)
}

fn comparison_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(path)
        }
    })
}

pub fn state_dirs() -> Vec<PathBuf> {
    let xdg = xdg_state_dir();
    let mut dirs = vec![xdg.clone()];
    if let Some(extra) = application_support_dir() {
        if comparison_path(&extra) != comparison_path(&xdg) {
            dirs.push(extra);
        }
    }
    dirs
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriterLease {
    pub owner: String,
    pub pid: i64,
    pub heartbeat_ms: i64,
    pub fingerprint: String,
    pub endpoint_hash: String,
    pub socket_path: String,
    pub schema: i64,
    pub path: String,
}

impl WriterLease {
    pub fn to_dict(&self) -> Value {
        json!({
            "schema": self.schema,
            "owner": self.owner,
            "pid": self.pid,
            "heartbeat_ms": self.heartbeat_ms,
            "fingerprint": self.fingerprint,
            "endpoint_hash": self.endpoint_hash,
            "socket_path": self.socket_path,
        })
    }

    pub fn is_fresh(&self, now: Option<i64>, ttl: Option<i64>) -> bool {
        let clock = now.unwrap_or_else(now_ms);
        let window = ttl.unwrap_or_else(lease_ttl_ms);
        if self.pid > 0 && !pid_alive(self.pid) {
            return false;
        }
        clock.saturating_sub(self.heartbeat_ms) <= window
    }
}

pub fn _mtime_ms(path: &Path) -> i64 {
    let modified = match path.metadata().and_then(|metadata| metadata.modified()) {
        Ok(modified) => modified,
        Err(_) => return 0,
    };
    match modified.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis().min(i64::MAX as u128) as i64,
        Err(error) => -(error.duration().as_millis().min(i64::MAX as u128) as i64),
    }
}

fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::Bool(true)) => true,
        Some(Value::Number(number)) => number.as_f64().map(|v| v != 0.0).unwrap_or(true),
        Some(Value::String(text)) => !text.is_empty(),
        Some(Value::Array(items)) => !items.is_empty(),
        Some(Value::Object(items)) => !items.is_empty(),
    }
}

fn py_repr_string(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(text.len() + 2);
    out.push(quote);
    for character in text.chars() {
        match character {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            value if value == quote => {
                out.push('\\');
                out.push(value);
            }
            value if value.is_control() => {
                let code = value as u32;
                if code <= 0xff {
                    out.push_str(&format!("\\x{code:02x}"));
                } else if code <= 0xffff {
                    out.push_str(&format!("\\u{code:04x}"));
                } else {
                    out.push_str(&format!("\\U{code:08x}"));
                }
            }
            value => out.push(value),
        }
    }
    out.push(quote);
    out
}

fn py_repr(value: &Value) -> String {
    match value {
        Value::Null => "None".into(),
        Value::String(text) => py_repr_string(text),
        Value::Bool(true) => "True".into(),
        Value::Bool(false) => "False".into(),
        Value::Number(number) => number.to_string(),
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(py_repr).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(items) => format!(
            "{{{}}}",
            items
                .iter()
                .map(|(key, value)| format!("{}: {}", py_repr_string(key), py_repr(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn py_string(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Bool(true)) => "True".into(),
        Some(Value::Bool(false)) => "False".into(),
        Some(Value::Number(number)) => number.to_string(),
        Some(value @ (Value::Array(_) | Value::Object(_))) => py_repr(value),
    }
}

fn int_value(value: Option<&Value>, default: i64) -> i64 {
    let value = if truthy(value) { value } else { None };
    match value {
        Some(Value::Bool(value)) => i64::from(*value),
        Some(Value::Number(number)) => number
            .as_i64()
            .or_else(|| number.as_u64().and_then(|v| i64::try_from(v).ok()))
            .or_else(|| number.as_f64().map(|v| v as i64))
            .unwrap_or(default),
        Some(Value::String(text)) => text.trim().parse::<i64>().unwrap_or(default),
        _ => default,
    }
}

pub fn parse_lease_text(
    text: &str,
    path: &Path,
    fallback_owner: Option<&str>,
    fallback_fingerprint: &str,
) -> Option<WriterLease> {
    let stripped = text.trim();
    if stripped.is_empty() {
        return None;
    }
    if let Ok(Value::Object(payload)) = serde_json::from_str::<Value>(stripped) {
        let owner_value = payload.get("owner");
        let owner = py_string(if truthy(owner_value) {
            owner_value
        } else {
            None
        });
        if !is_owner(&owner) {
            return None;
        }
        let pid = int_value(payload.get("pid"), 0);
        let mut heartbeat_ms = int_value(payload.get("heartbeat_ms"), 0);
        if heartbeat_ms <= 0 {
            heartbeat_ms = _mtime_ms(path);
        }
        let schema = int_value(payload.get("schema"), SCHEMA);
        let fingerprint_value = payload.get("fingerprint");
        let endpoint_value = payload.get("endpoint_hash");
        let socket_value = payload.get("socket_path");
        return Some(WriterLease {
            owner,
            pid,
            heartbeat_ms,
            fingerprint: if truthy(fingerprint_value) {
                py_string(fingerprint_value)
            } else {
                fallback_fingerprint.to_string()
            },
            endpoint_hash: if truthy(endpoint_value) {
                py_string(endpoint_value)
            } else {
                String::new()
            },
            socket_path: if truthy(socket_value) {
                py_string(socket_value)
            } else {
                String::new()
            },
            schema,
            path: path.to_string_lossy().into_owned(),
        });
    }
    if fallback_owner.is_some_and(is_owner)
        && matches!(
            stripped.to_lowercase().as_str(),
            "1" | "live" | "yes" | "on" | "true"
        )
    {
        return Some(WriterLease {
            owner: fallback_owner.unwrap().to_string(),
            pid: 0,
            heartbeat_ms: _mtime_ms(path),
            fingerprint: fallback_fingerprint.to_string(),
            endpoint_hash: String::new(),
            socket_path: String::new(),
            schema: 0,
            path: path.to_string_lossy().into_owned(),
        });
    }
    None
}

pub fn read_lease_file(path: &Path, fingerprint: &str) -> Option<WriterLease> {
    if !path.is_file() {
        return None;
    }
    let text = fs::read_to_string(path).ok()?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let fallback_owner = if name.starts_with("native-live") {
        Some(OWNER_NATIVE)
    } else if name.starts_with("plugin-live") {
        Some(OWNER_PLUGIN)
    } else {
        None
    };
    parse_lease_text(&text, path, fallback_owner, fingerprint)
}

pub fn writer_paths(fingerprint: &str, owner: &str) -> Result<Vec<PathBuf>, String> {
    if !is_owner(owner) {
        return Err(format!("unknown handoff owner: {owner}"));
    }
    let mut paths = Vec::new();
    for root in state_dirs() {
        paths.push(root.join(format!("writer-{fingerprint}.json")));
        paths.push(root.join(format!("{owner}-live-{fingerprint}")));
        paths.push(root.join(format!("{owner}-live")));
    }
    Ok(paths)
}

pub fn legacy_native_marker_path(fingerprint: &str) -> PathBuf {
    xdg_state_dir().join(format!("native-live-{fingerprint}"))
}

pub fn plugin_marker_path(fingerprint: &str) -> PathBuf {
    xdg_state_dir().join(format!("plugin-live-{fingerprint}"))
}

fn candidate_paths(fingerprint: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for root in state_dirs() {
        for name in [
            format!("writer-{fingerprint}.json"),
            format!("native-live-{fingerprint}"),
            format!("plugin-live-{fingerprint}"),
            "native-live".into(),
            "plugin-live".into(),
            "writer.json".into(),
        ] {
            let path = root.join(name);
            if seen.insert(path.clone()) {
                paths.push(path);
            }
        }
    }
    paths
}

pub fn load_leases(fingerprint: &str) -> Vec<WriterLease> {
    candidate_paths(fingerprint)
        .into_iter()
        .filter_map(|path| read_lease_file(&path, fingerprint))
        .collect()
}

pub fn pick_lease(
    leases: &[WriterLease],
    now: Option<i64>,
) -> (Option<WriterLease>, Vec<WriterLease>) {
    let clock = now.unwrap_or_else(now_ms);
    let mut live: Option<WriterLease> = None;
    let mut stale = Vec::new();
    for lease in leases {
        if lease.is_fresh(Some(clock), None) {
            let replace = live.as_ref().is_none_or(|current| {
                let lease_key = (lease.owner == OWNER_NATIVE, lease.heartbeat_ms);
                let current_key = (current.owner == OWNER_NATIVE, current.heartbeat_ms);
                lease_key > current_key
            });
            if replace {
                live = Some(lease.clone());
            }
        } else {
            stale.push(lease.clone());
        }
    }
    (live, stale)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriterDecision {
    pub writer: String,
    pub owner: Option<String>,
    pub native_live: bool,
    pub plugin_live: bool,
    pub native_detected: bool,
    pub plugin_detected: bool,
    pub force_plugin: bool,
    pub env_native_live: bool,
    pub lease_stale: bool,
    pub lease: Option<WriterLease>,
    pub fingerprint: String,
}

impl WriterDecision {
    pub fn yields(&self) -> bool {
        !self.force_plugin && self.native_live
    }

    pub fn outcome(&self) -> &'static str {
        if self.native_live {
            OUTCOME_NATIVE_OWNS
        } else if self.plugin_live {
            OUTCOME_PLUGIN_OWNS
        } else {
            "unclaimed"
        }
    }

    pub fn payload(&self, action: &str, method: Option<&str>) -> Value {
        let outcome = if self.yields() || self.plugin_live {
            self.outcome()
        } else {
            "unclaimed"
        };
        let mut body = Map::from_iter([
            ("ok".into(), json!(true)),
            ("outcome".into(), json!(outcome)),
            ("writer".into(), json!(self.writer)),
            ("action".into(), json!(action)),
            ("server_stopped".into(), json!(false)),
            ("competing".into(), json!(false)),
            ("native_live".into(), json!(self.native_live)),
            ("plugin_live".into(), json!(self.plugin_live)),
            ("lease_stale".into(), json!(self.lease_stale)),
            ("fingerprint".into(), json!(self.fingerprint)),
        ]);
        if let Some(method) = method {
            body.insert("method".into(), json!(method));
        }
        if let Some(lease) = &self.lease {
            body.insert("lease".into(), lease.to_dict());
        }
        Value::Object(body)
    }
}

pub fn resolve_writer(fingerprint: &str, our_pid: Option<i64>, now: Option<i64>) -> WriterDecision {
    let force = env_truthy(FORCE_PLUGIN_ENV);
    let env_native = env_truthy(NATIVE_LIVE_ENV);
    let leases = load_leases(fingerprint);
    let (live, stale) = pick_lease(&leases, now);
    let native_file = leases.iter().any(|item| item.owner == OWNER_NATIVE);
    let plugin_file = leases.iter().any(|item| item.owner == OWNER_PLUGIN);

    let mut owner = None;
    if env_native && !force {
        owner = Some(OWNER_NATIVE.to_string());
    } else if let Some(lease) = &live {
        owner = Some(lease.owner.clone());
    }
    let _ours = our_pid.unwrap_or(std::process::id() as i64);
    let native_live = owner.as_deref() == Some(OWNER_NATIVE) && !force;
    let plugin_live = owner.as_deref() == Some(OWNER_PLUGIN) && !native_live;
    let writer = if force
        && (env_native
            || live
                .as_ref()
                .is_some_and(|lease| lease.owner == OWNER_NATIVE))
    {
        "plugin-forced"
    } else if native_live {
        OWNER_NATIVE
    } else {
        OWNER_PLUGIN
    };
    WriterDecision {
        writer: writer.into(),
        owner: if force {
            Some(OWNER_PLUGIN.into())
        } else {
            owner
        },
        native_live,
        plugin_live,
        native_detected: env_native || native_file,
        plugin_detected: plugin_file,
        force_plugin: force,
        env_native_live: env_native,
        lease_stale: live.is_none() && !stale.is_empty(),
        lease: live,
        fingerprint: fingerprint.into(),
    }
}

fn atomic_write(path: &Path, payload: &Value) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let temporary = path.with_file_name(format!("{file_name}.tmp"));
    let mut encoded = serde_json::to_string_pretty(payload).map_err(io::Error::other)?;
    encoded.push('\n');
    fs::write(&temporary, encoded)?;
    fs::rename(temporary, path)
}

fn unlink(path: &Path) -> bool {
    match fs::remove_file(path) {
        Ok(()) => true,
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(_) => false,
    }
}

pub fn write_lease(
    owner: &str,
    fingerprint: &str,
    socket_path: &str,
    endpoint_hash: &str,
    pid: Option<i64>,
    heartbeat: Option<i64>,
) -> io::Result<WriterLease> {
    let lease = WriterLease {
        owner: owner.into(),
        pid: pid.unwrap_or(std::process::id() as i64),
        heartbeat_ms: heartbeat.unwrap_or_else(now_ms),
        fingerprint: fingerprint.into(),
        endpoint_hash: endpoint_hash.into(),
        socket_path: socket_path.into(),
        schema: SCHEMA,
        path: String::new(),
    };
    let payload = lease.to_dict();
    let paths = writer_paths(fingerprint, owner)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let mut last = String::new();
    for path in paths {
        atomic_write(&path, &payload)?;
        last = path.to_string_lossy().into_owned();
    }
    Ok(WriterLease {
        path: last,
        ..lease
    })
}

pub fn clear_owner(owner: &str, fingerprint: &str) {
    if let Ok(paths) = writer_paths(fingerprint, owner) {
        for path in paths {
            unlink(&path);
        }
    }
}

pub fn claim_plugin_writer(
    fingerprint: &str,
    socket_path: &str,
    endpoint_hash: &str,
) -> io::Result<Option<WriterLease>> {
    let decision = resolve_writer(fingerprint, None, None);
    if decision.yields() {
        return Ok(None);
    }
    if decision.force_plugin {
        clear_owner(OWNER_NATIVE, fingerprint);
    }
    write_lease(
        OWNER_PLUGIN,
        fingerprint,
        socket_path,
        endpoint_hash,
        None,
        None,
    )
    .map(Some)
}

pub fn release_plugin_writer(fingerprint: &str) {
    clear_owner(OWNER_PLUGIN, fingerprint);
}

pub fn heartbeat_plugin_writer(
    fingerprint: &str,
    socket_path: &str,
    endpoint_hash: &str,
) -> io::Result<Option<WriterLease>> {
    let decision = resolve_writer(fingerprint, Some(std::process::id() as i64), None);
    if decision.yields() {
        return Ok(None);
    }
    let Some(lease) = decision.lease.filter(|lease| lease.owner == OWNER_PLUGIN) else {
        return claim_plugin_writer(fingerprint, socket_path, endpoint_hash);
    };
    write_lease(
        OWNER_PLUGIN,
        fingerprint,
        if socket_path.is_empty() {
            &lease.socket_path
        } else {
            socket_path
        },
        if endpoint_hash.is_empty() {
            &lease.endpoint_hash
        } else {
            endpoint_hash
        },
        None,
        None,
    )
    .map(Some)
}

pub fn heartbeat_native_writer(
    fingerprint: &str,
    socket_path: &str,
    endpoint_hash: &str,
    pid: Option<i64>,
) -> io::Result<Option<WriterLease>> {
    let owner_pid = pid.unwrap_or(std::process::id() as i64);
    let decision = resolve_writer(fingerprint, None, None);
    if decision.native_live {
        if decision
            .lease
            .as_ref()
            .is_some_and(|lease| lease.pid > 0 && lease.pid != owner_pid)
        {
            return Ok(None);
        }
        let prior_socket = decision
            .lease
            .as_ref()
            .map(|lease| lease.socket_path.as_str())
            .unwrap_or("");
        let prior_hash = decision
            .lease
            .as_ref()
            .map(|lease| lease.endpoint_hash.as_str())
            .unwrap_or("");
        return write_lease(
            OWNER_NATIVE,
            fingerprint,
            if socket_path.is_empty() {
                prior_socket
            } else {
                socket_path
            },
            if endpoint_hash.is_empty() {
                prior_hash
            } else {
                endpoint_hash
            },
            Some(owner_pid),
            None,
        )
        .map(Some);
    }
    claim_native_writer(fingerprint, socket_path, endpoint_hash, Some(owner_pid))
}

pub fn claim_native_writer(
    fingerprint: &str,
    socket_path: &str,
    endpoint_hash: &str,
    pid: Option<i64>,
) -> io::Result<Option<WriterLease>> {
    let decision = resolve_writer(fingerprint, None, None);
    let owner_pid = pid.unwrap_or(std::process::id() as i64);
    if decision.plugin_live
        && decision
            .lease
            .as_ref()
            .is_some_and(|lease| lease.pid != owner_pid)
        && !env_truthy(NATIVE_LIVE_ENV)
        && !decision.force_plugin
    {
        return Ok(None);
    }
    clear_owner(OWNER_PLUGIN, fingerprint);
    write_lease(
        OWNER_NATIVE,
        fingerprint,
        socket_path,
        endpoint_hash,
        pid,
        None,
    )
    .map(Some)
}

pub fn release_native_writer(fingerprint: &str) {
    clear_owner(OWNER_NATIVE, fingerprint);
}

pub fn writer_status(fingerprint: &str) -> Value {
    let decision = resolve_writer(fingerprint, None, None);
    let marker = legacy_native_marker_path(fingerprint);
    let plugin = plugin_marker_path(fingerprint);
    let global_marker_exists = state_dirs()
        .iter()
        .any(|root| root.join("native-live").is_file());
    json!({
        "writer": decision.writer,
        "native_live": decision.native_live,
        "plugin_live": decision.plugin_live,
        "native_detected": decision.native_detected,
        "plugin_detected": decision.plugin_detected,
        "force_plugin": decision.force_plugin,
        "env_native_live": decision.env_native_live,
        "lease_stale": decision.lease_stale,
        "lease": decision.lease.as_ref().map(WriterLease::to_dict),
        "marker_path": marker,
        "marker_exists": marker.is_file(),
        "plugin_marker_path": plugin,
        "plugin_marker_exists": plugin.is_file(),
        "global_marker_exists": global_marker_exists,
        "fingerprint": fingerprint,
    })
}

pub fn restore_paths(endpoint_hash: &str) -> Vec<PathBuf> {
    state_dirs()
        .into_iter()
        .map(|root| root.join(format!("restore-{endpoint_hash}.json")))
        .collect()
}

pub fn write_shared_restore(endpoint_hash: &str, payload: &Value) -> io::Result<String> {
    let Some(payload) = payload.as_object() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "restore payload must be an object",
        ));
    };
    if payload.get("mode").and_then(Value::as_str) == Some("replay_tree") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "restore payload must not use mode=replay_tree",
        ));
    }
    let mut body = payload.clone();
    body.entry("mode").or_insert_with(|| json!("reattach"));
    let body = Value::Object(body);
    let mut last = String::new();
    for path in restore_paths(endpoint_hash) {
        atomic_write(&path, &body)?;
        last = path.to_string_lossy().into_owned();
    }
    Ok(last)
}

pub fn read_shared_restore(endpoint_hash: &str) -> Option<Value> {
    for path in restore_paths(endpoint_hash) {
        if !path.is_file() {
            continue;
        }
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        let Ok(Value::Object(payload)) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        if payload.get("mode").and_then(Value::as_str) == Some("replay_tree") {
            continue;
        }
        return Some(Value::Object(payload));
    }
    None
}

pub fn clear_shared_restore(endpoint_hash: &str) -> bool {
    let mut removed = false;
    for path in restore_paths(endpoint_hash) {
        if unlink(&path) {
            removed = true;
        }
    }
    removed
}

pub fn observe_foreign(decision: &WriterDecision, method: &str) -> Value {
    let mut body = decision.payload("observe", Some(method));
    let object = body.as_object_mut().unwrap();
    object.insert("mirrored".into(), json!(true));
    object.insert("panes".into(), json!([]));
    object.insert("windows".into(), json!([]));
    body
}

pub fn iter_stale_leases(fingerprint: &str) -> Vec<WriterLease> {
    pick_lease(&load_leases(fingerprint), None).1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::MutexGuard;
    use std::time::Duration;
    use tempfile::TempDir;

    use super::HANDOFF_ENV_LOCK;

    struct TestEnv {
        _guard: MutexGuard<'static, ()>,
        _temp: TempDir,
    }

    impl TestEnv {
        fn new() -> Self {
            let guard = HANDOFF_ENV_LOCK
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let temp = tempfile::tempdir().unwrap();
            std::env::set_var("XDG_STATE_HOME", temp.path().join("xdg"));
            std::env::set_var(NATIVE_STATE_ENV, temp.path().join("native"));
            std::env::set_var("HOME", temp.path().join("home"));
            std::env::remove_var(NATIVE_LIVE_ENV);
            std::env::remove_var(FORCE_PLUGIN_ENV);
            std::env::remove_var(LEASE_TTL_ENV);
            Self {
                _guard: guard,
                _temp: temp,
            }
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            for name in [
                "XDG_STATE_HOME",
                NATIVE_STATE_ENV,
                "HOME",
                NATIVE_LIVE_ENV,
                FORCE_PLUGIN_ENV,
                LEASE_TTL_ENV,
            ] {
                std::env::remove_var(name);
            }
        }
    }

    #[test]
    fn pidless_lease_staleness_uses_inclusive_ttl() {
        let lease = WriterLease {
            owner: OWNER_PLUGIN.into(),
            pid: 0,
            heartbeat_ms: 100,
            fingerprint: "fp".into(),
            endpoint_hash: String::new(),
            socket_path: String::new(),
            schema: SCHEMA,
            path: String::new(),
        };
        assert!(lease.is_fresh(Some(145), Some(45)));
        assert!(!lease.is_fresh(Some(146), Some(45)));
    }

    #[test]
    fn plugin_claim_resolve_release_round_trip() {
        let _env = TestEnv::new();
        let claim = claim_plugin_writer("fp", "/tmp/herdr.sock", "hash")
            .unwrap()
            .unwrap();
        assert_eq!(claim.owner, OWNER_PLUGIN);
        let decision = resolve_writer("fp", None, None);
        assert!(decision.plugin_live);
        assert_eq!(decision.writer, OWNER_PLUGIN);
        assert!(application_support_dir()
            .unwrap()
            .join("plugin-live-fp")
            .is_file());
        release_plugin_writer("fp");
        assert!(!resolve_writer("fp", None, None).plugin_live);
    }

    #[test]
    fn fresh_native_claim_blocks_plugin() {
        let _env = TestEnv::new();
        claim_native_writer("fp", "", "", None).unwrap().unwrap();
        assert!(claim_plugin_writer("fp", "", "").unwrap().is_none());
        assert!(resolve_writer("fp", None, None).native_live);
        release_native_writer("fp");
    }

    #[test]
    fn dead_pid_is_immediately_stale() {
        let _env = TestEnv::new();
        write_lease(OWNER_NATIVE, "fp", "", "", Some(999_999_999), None).unwrap();
        let decision = resolve_writer("fp", None, None);
        assert!(!decision.native_live);
        assert!(decision.lease_stale);
        assert_eq!(decision.writer, OWNER_PLUGIN);
    }

    #[test]
    fn plugin_heartbeat_preserves_unspecified_metadata() {
        let _env = TestEnv::new();
        let first = claim_plugin_writer("fp", "/tmp/herdr.sock", "hash")
            .unwrap()
            .unwrap();
        std::thread::sleep(Duration::from_millis(2));
        let heartbeat = heartbeat_plugin_writer("fp", "", "").unwrap().unwrap();
        assert_eq!(heartbeat.socket_path, "/tmp/herdr.sock");
        assert_eq!(heartbeat.endpoint_hash, "hash");
        assert!(heartbeat.heartbeat_ms >= first.heartbeat_ms);
    }

    #[test]
    fn force_plugin_replaces_native_claim() {
        let _env = TestEnv::new();
        claim_native_writer("fp", "", "", None).unwrap().unwrap();
        std::env::set_var(FORCE_PLUGIN_ENV, "1");
        claim_plugin_writer("fp", "", "").unwrap().unwrap();
        let decision = resolve_writer("fp", None, None);
        assert!(decision.plugin_live);
        assert_eq!(decision.writer, OWNER_PLUGIN);
        assert!(!legacy_native_marker_path("fp").exists());
    }

    #[test]
    fn restore_round_trip_and_replay_rejection() {
        let _env = TestEnv::new();
        let payload = json!({"socket_path": "/tmp/herdr.sock", "session_ids": ["main"]});
        write_shared_restore("deadbeef", &payload).unwrap();
        let restored = read_shared_restore("deadbeef").unwrap();
        assert_eq!(restored["mode"], "reattach");
        assert!(clear_shared_restore("deadbeef"));
        assert!(read_shared_restore("deadbeef").is_none());
        assert!(write_shared_restore("deadbeef", &json!({"mode": "replay_tree"})).is_err());
    }
}
