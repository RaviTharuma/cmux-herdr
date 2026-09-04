//! Herdr attach, detach, restore, and observability lifecycle.
//!
//! Behavioral port of `bridge/cmux_herdr_lifecycle.py`. The provider is reached
//! through a Unix socket; cmux owns only attachment intent. Detach and host close
//! never stop the Herdr server, and restore always takes a fresh snapshot path
//! (`reattach`) rather than replaying a stale split tree.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

pub const SOCKET_METHODS: [&str; 8] = [
    "remote.herdr.sessions",
    "remote.herdr.attach",
    "remote.herdr.mirror",
    "remote.herdr.window",
    "remote.herdr.detach",
    "remote.herdr.state",
    "remote.herdr.pane_surfaces",
    "remote.herdr.pane_grids",
];

const SESSION_METHODS: [&str; 5] = [
    "remote.herdr.attach",
    "remote.herdr.detach",
    "remote.herdr.state",
    "remote.herdr.pane_surfaces",
    "remote.herdr.pane_grids",
];

pub const SETTING_KEY: &str = "betaFeatures.remoteHerdrMirror";
pub const TEARDOWN_SESSION_ENDED: &str = "session_ended";
pub const TEARDOWN_EXPLICIT_DETACH: &str = "explicit_detach";
pub const POST_RESEED: &str = "reseed";
pub const POST_APPLY_CLIENT_SIZE: &str = "apply_client_size";

/// Stable short endpoint identifier. Callers can log this instead of the path.
pub fn endpoint_hash(socket_path: &str) -> String {
    let digest = Sha256::digest(socket_path.as_bytes());
    format!("{digest:x}")[..16].to_string()
}

/// Reject C0, DEL, and C1 control characters at the socket trust boundary.
pub fn has_hidden_character(value: &str) -> bool {
    value
        .chars()
        .any(|character| matches!(character as u32, 0..=31 | 127 | 128..=159))
}

/// Validate and trim an absolute Unix socket path.
pub fn validate_socket_path(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty()
        || !trimmed.starts_with('/')
        || trimmed.starts_with('-')
        || has_hidden_character(trimmed)
        || trimmed.contains('\0')
    {
        return None;
    }
    Some(trimmed.to_string())
}

/// Validate and trim a non-empty provider session name.
pub fn validate_session_name(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Decode the beta feature value with the same accepted values as Python.
pub fn decode_beta(value: Option<&Value>, default: bool) -> bool {
    match value {
        None | Some(Value::Null) => default,
        Some(Value::Bool(value)) => *value,
        Some(Value::Number(value)) if value.as_f64() == Some(0.0) => false,
        Some(Value::Number(value)) if value.as_f64() == Some(1.0) => true,
        Some(Value::String(value)) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => default,
        },
        _ => default,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredSession {
    pub session_id: String,
    pub name: String,
    pub window_count: i64,
    pub attached: bool,
}

impl DiscoveredSession {
    pub fn new(
        session_id: impl Into<String>,
        name: impl Into<String>,
        window_count: i64,
        attached: bool,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            name: name.into(),
            window_count,
            attached,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachWindowTarget {
    pub kind: String,
    pub window_id: Option<String>,
}

impl AttachWindowTarget {
    pub fn new(kind: impl Into<String>, window_id: Option<String>) -> Self {
        Self {
            kind: kind.into(),
            window_id,
        }
    }

    /// Existing live mirror affinity wins so one endpoint stays in one window.
    pub fn resolve<F>(
        &self,
        existing_mirror_window_id: Option<&str>,
        active_window_id: Option<&str>,
        is_live: F,
    ) -> Option<String>
    where
        F: Fn(&str) -> bool,
    {
        if let Some(existing) = existing_mirror_window_id.filter(|id| is_live(id)) {
            return Some(existing.to_string());
        }
        match self.kind.as_str() {
            "dedicated_new_window" | "unresolved_explicit" => None,
            "explicit" => self
                .window_id
                .as_deref()
                .filter(|id| is_live(id))
                .map(str::to_string),
            "contextual" => self
                .window_id
                .as_deref()
                .filter(|id| is_live(id))
                .or_else(|| active_window_id.filter(|id| is_live(id)))
                .map(str::to_string),
            _ => None,
        }
    }
}

/// Decode attach destination intent from socket RPC parameters.
pub fn window_target_from_params(params: &Value, dedicated: bool) -> AttachWindowTarget {
    if dedicated {
        return AttachWindowTarget::new("dedicated_new_window", None::<String>);
    }
    let object = params.as_object();
    if let Some(raw) = object.and_then(|body| body.get("window_id")) {
        if raw.is_null() || raw.as_str() == Some("") {
            return AttachWindowTarget::new("unresolved_explicit", None::<String>);
        }
        return AttachWindowTarget::new("explicit", Some(value_string(raw)));
    }
    let preferred = object
        .and_then(|body| body.get("preferred_window_id"))
        .filter(|value| json_truthy(value))
        .map(value_string);
    AttachWindowTarget::new("contextual", preferred)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorRecord {
    pub session_id: String,
    pub window_id: String,
    pub workspace_id: Option<String>,
}

impl MirrorRecord {
    pub fn new(
        session_id: impl Into<String>,
        window_id: impl Into<String>,
        workspace_id: Option<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            window_id: window_id.into(),
            workspace_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionRecord {
    pub session_id: String,
    pub started: bool,
    pub snapshot_received: bool,
    pub exited: bool,
    pub window_ids: Vec<String>,
    pub total_output_bytes: i64,
    pub pane_output_bytes: BTreeMap<String, i64>,
    pub recent_events: Vec<String>,
    pub client_size_applied: bool,
}

impl ConnectionRecord {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            started: false,
            snapshot_received: false,
            exited: false,
            window_ids: Vec::new(),
            total_output_bytes: 0,
            pane_output_bytes: BTreeMap::new(),
            recent_events: Vec::new(),
            client_size_applied: false,
        }
    }

    pub fn started(session_id: impl Into<String>) -> Self {
        let mut record = Self::new(session_id);
        record.started = true;
        record
    }
}

pub fn connection_action(existing: Option<&ConnectionRecord>) -> &'static str {
    match existing {
        None => "start",
        Some(existing) if existing.exited => "replace",
        Some(_) => "reuse",
    }
}

pub fn may_cache_connection(connection: &ConnectionRecord) -> bool {
    connection.started && !connection.exited
}

pub fn post_attach_action(replaced_dead: bool) -> &'static str {
    if replaced_dead {
        POST_RESEED
    } else {
        POST_APPLY_CLIENT_SIZE
    }
}

/// Host close never maps to provider termination.
pub fn host_close_policy(source: &str) -> &'static str {
    match source {
        "last_workspace_tab" | "window_quit" | "app_terminate" | "explicit_detach"
        | "host_tab" | "host_panel" => "detach",
        _ => "noop",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachPlan {
    pub outcome: String,
    pub window_id: Option<String>,
    pub create_window: bool,
    pub sessions_to_mirror: Vec<String>,
    pub sessions_to_reuse: Vec<String>,
    pub purge_session_ids: Vec<String>,
    pub move_workspace_ids: Vec<String>,
    pub post_attach: Option<String>,
    pub discard_window_on_fail: bool,
    pub activate: bool,
    pub reason: Option<String>,
}

impl AttachPlan {
    fn outcome(outcome: &str, reason: Option<&str>) -> Self {
        Self {
            outcome: outcome.to_string(),
            window_id: None,
            create_window: false,
            sessions_to_mirror: Vec::new(),
            sessions_to_reuse: Vec::new(),
            purge_session_ids: Vec::new(),
            move_workspace_ids: Vec::new(),
            post_attach: None,
            discard_window_on_fail: false,
            activate: false,
            reason: reason.map(str::to_string),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn plan_attach(
    target: &AttachWindowTarget,
    enabled: bool,
    app_ready: bool,
    already_attaching: bool,
    existing_mirror_window_id: Option<&str>,
    active_window_id: Option<&str>,
    live_windows: &[String],
    sessions: Option<&[DiscoveredSession]>,
    mirrors: Option<&[MirrorRecord]>,
    live_session_ids: Option<&[String]>,
    activate: bool,
    mirrored_workspace_ids: Option<&[String]>,
) -> AttachPlan {
    if !enabled {
        return AttachPlan::outcome("disabled", Some("beta_disabled"));
    }
    if !app_ready {
        return AttachPlan::outcome("unreachable", Some("app_not_ready"));
    }
    if already_attaching {
        return AttachPlan::outcome("already_attaching", Some("reentrant"));
    }

    let is_live = |window_id: &str| live_windows.iter().any(|item| item == window_id);
    if target.kind != "dedicated_new_window"
        && target
            .resolve(existing_mirror_window_id, active_window_id, is_live)
            .is_none()
    {
        return AttachPlan::outcome("invalid_target", Some("window_unresolved"));
    }
    let Some(sessions) = sessions else {
        return AttachPlan::outcome("discover", None);
    };
    if sessions.is_empty() {
        return AttachPlan::outcome("no_sessions", Some("empty_discovery"));
    }

    let records = mirrors.unwrap_or_default();
    let dead = records
        .iter()
        .filter(|record| record.workspace_id.is_none())
        .map(|record| record.session_id.clone())
        .collect::<Vec<_>>();
    let live_records = records
        .iter()
        .filter(|record| record.workspace_id.is_some())
        .collect::<Vec<_>>();
    let live_ids = live_session_ids
        .unwrap_or_default()
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    let (window_id, create_window, move_workspace_ids) =
        if target.kind == "dedicated_new_window" {
            (
                None,
                true,
                live_records
                    .iter()
                    .filter_map(|record| record.workspace_id.clone())
                    .collect(),
            )
        } else {
            let Some(window_id) = target.resolve(
                existing_mirror_window_id,
                active_window_id,
                |id| is_live(id),
            ) else {
                return AttachPlan::outcome("invalid_target", Some("window_lost"));
            };
            (Some(window_id), false, Vec::new())
        };

    let mut reuse = Vec::new();
    let mut create = Vec::new();
    for session in sessions {
        if live_ids.contains(session.session_id.as_str()) && !dead.contains(&session.session_id) {
            reuse.push(session.session_id.clone());
        } else {
            create.push(session.session_id.clone());
        }
    }

    if mirrored_workspace_ids.is_some_and(<[String]>::is_empty) {
        let mut plan = AttachPlan::outcome("failed_empty", Some("no_workspaces"));
        plan.window_id = window_id;
        plan.create_window = create_window;
        plan.purge_session_ids = dead;
        plan.discard_window_on_fail = create_window;
        return plan;
    }

    let replaced = !dead.is_empty() || reuse.iter().any(|id| !live_ids.contains(id.as_str()));
    let (outcome, mut post_attach) = if create.is_empty() && !reuse.is_empty() {
        (
            "reused",
            if replaced { Some(POST_RESEED.to_string()) } else { None },
        )
    } else {
        (
            "mirrored",
            Some(post_attach_action(!dead.is_empty() && reuse.is_empty()).to_string()),
        )
    };
    if post_attach.is_none() && !create.is_empty() {
        post_attach = Some(POST_APPLY_CLIENT_SIZE.to_string());
    }

    AttachPlan {
        outcome: outcome.to_string(),
        window_id,
        create_window,
        sessions_to_mirror: create,
        sessions_to_reuse: reuse,
        purge_session_ids: dead,
        move_workspace_ids: if create_window {
            move_workspace_ids
        } else {
            Vec::new()
        },
        post_attach,
        discard_window_on_fail: create_window,
        activate,
        reason: None,
    }
}

pub fn plan_restore(
    record: &RestoreRecord,
    enabled: bool,
    app_ready: bool,
    sessions: &[DiscoveredSession],
    live_windows: &[String],
    active_window_id: Option<&str>,
) -> AttachPlan {
    if !enabled {
        return AttachPlan::outcome("disabled", Some("beta_disabled"));
    }
    let target = if record
        .window_id
        .as_ref()
        .is_some_and(|window| live_windows.contains(window))
    {
        AttachWindowTarget::new("explicit", record.window_id.clone())
    } else if record.target_kind == "dedicated_new_window" {
        AttachWindowTarget::new("dedicated_new_window", None::<String>)
    } else {
        AttachWindowTarget::new("contextual", None::<String>)
    };
    let plan = plan_attach(
        &target,
        enabled,
        app_ready,
        false,
        None,
        active_window_id,
        live_windows,
        Some(sessions),
        Some(&[]),
        Some(&[]),
        true,
        None,
    );
    if matches!(plan.outcome.as_str(), "mirrored" | "reused") {
        AttachPlan {
            outcome: "mirrored".into(),
            window_id: plan.window_id,
            create_window: plan.create_window,
            sessions_to_mirror: if plan.sessions_to_mirror.is_empty() {
                sessions.iter().map(|item| item.session_id.clone()).collect()
            } else {
                plan.sessions_to_mirror
            },
            sessions_to_reuse: Vec::new(),
            purge_session_ids: Vec::new(),
            move_workspace_ids: Vec::new(),
            post_attach: Some(POST_RESEED.into()),
            discard_window_on_fail: plan.discard_window_on_fail,
            activate: true,
            reason: Some("restore_reattach".into()),
        }
    } else {
        plan
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreRecord {
    pub endpoint_hash: String,
    pub socket_path: String,
    pub session_ids: Vec<String>,
    pub target_kind: String,
    pub window_id: Option<String>,
}

impl RestoreRecord {
    pub fn new(
        endpoint_hash: impl Into<String>,
        socket_path: impl Into<String>,
        session_ids: Vec<String>,
        target_kind: impl Into<String>,
        window_id: Option<String>,
    ) -> Self {
        Self {
            endpoint_hash: endpoint_hash.into(),
            socket_path: socket_path.into(),
            session_ids,
            target_kind: target_kind.into(),
            window_id,
        }
    }

    /// JSON-safe restore intent. `mode` is always `reattach`.
    pub fn to_value(&self) -> Value {
        json!({
            "endpoint_hash": self.endpoint_hash,
            "socket_path": self.socket_path,
            "session_ids": self.session_ids,
            "target_kind": self.target_kind,
            "window_id": self.window_id,
            "mode": "reattach",
        })
    }

    pub fn from_value(payload: &Value) -> Option<Self> {
        let body = payload.as_object()?;
        if body.get("mode").and_then(Value::as_str) == Some("replay_tree") {
            return None;
        }
        let socket_path = validate_socket_path(
            body.get("socket_path")
                .filter(|value| json_truthy(value))
                .map(value_string)
                .as_deref(),
        )?;
        let endpoint = body.get("endpoint_hash").filter(|value| json_truthy(value)).map(value_string).unwrap_or_default();
        let target_kind = body.get("target_kind").filter(|value| json_truthy(value)).map(value_string).unwrap_or_default();
        if endpoint.is_empty() || target_kind.is_empty() {
            return None;
        }
        let session_ids = body
            .get("session_ids")?
            .as_array()?
            .iter()
            .map(value_string)
            .collect::<Vec<_>>();
        if session_ids.is_empty() {
            return None;
        }
        let window_id = body
            .get("window_id")
            .filter(|value| json_truthy(value))
            .map(value_string);
        Some(Self::new(
            endpoint,
            socket_path,
            session_ids,
            target_kind,
            window_id,
        ))
    }

    pub fn to_dict(&self) -> Value {
        self.to_value()
    }

    pub fn from_dict(payload: &Value) -> Option<Self> {
        Self::from_value(payload)
    }
}

/// Atomically persist attachment intent using a sibling `.tmp` and rename.
pub fn write_restore(path: &Path, record: &RestoreRecord) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = temporary_path(path);
    let mut encoded = serde_json::to_string_pretty(&record.to_value())?;
    encoded.push('\n');
    fs::write(&tmp, encoded)?;
    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&tmp);
            Err(error)
        }
    }
}

/// Missing, unreadable, malformed, and stale-tree restore files are ignored.
pub fn read_restore(path: &Path) -> io::Result<Option<RestoreRecord>> {
    if !path.is_file() {
        return Ok(None);
    }
    let Ok(raw) = fs::read_to_string(path) else {
        return Ok(None);
    };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        return Ok(None);
    };
    Ok(RestoreRecord::from_value(&value))
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name: OsString = path.as_os_str().to_owned();
    name.push(".tmp");
    PathBuf::from(name)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttachRegistry {
    pending: BTreeSet<String>,
}

impl AttachRegistry {
    pub fn begin_attach(&mut self, endpoint: &str) -> bool {
        self.pending.insert(endpoint.to_string())
    }

    pub fn end_attach(&mut self, endpoint: &str) {
        self.pending.remove(endpoint);
    }

    pub fn is_attaching(&self, endpoint: &str) -> bool {
        self.pending.contains(endpoint)
    }
}

pub fn existing_mirror_window(
    mirrors: &[MirrorRecord],
    live_windows: &[String],
) -> Option<String> {
    mirrors
        .iter()
        .find(|record| {
            record.workspace_id.is_some() && live_windows.contains(&record.window_id)
        })
        .map(|record| record.window_id.clone())
}

pub fn purge_dead_mirrors(mirrors: &[MirrorRecord]) -> Vec<MirrorRecord> {
    mirrors
        .iter()
        .filter(|record| record.workspace_id.is_some())
        .cloned()
        .collect()
}

pub fn session_payload(session: &DiscoveredSession) -> Value {
    json!({
        "id": session.session_id,
        "name": session.name,
        "windows": session.window_count,
        "attached": session.attached,
    })
}

pub fn grid_match(
    assigned_cols: i64,
    assigned_rows: i64,
    rendered_cols: i64,
    rendered_rows: i64,
    exact_cols: bool,
    exact_rows: bool,
) -> bool {
    let cols_ok = if exact_cols {
        rendered_cols == assigned_cols
    } else {
        rendered_cols >= assigned_cols
    };
    let rows_ok = if exact_rows {
        rendered_rows == assigned_rows
    } else {
        rendered_rows >= assigned_rows
    };
    cols_ok && rows_ok
}

#[allow(clippy::too_many_arguments)]
pub fn pane_grid_payload(
    tab_id: &str,
    panes: &[Value],
    structure_version: i64,
    zoomed: bool,
    base_cols: i64,
    base_rows: i64,
    pushed: Option<(i64, i64)>,
    visible_for_sizing: bool,
) -> Value {
    let rows = panes
        .iter()
        .map(|pane| {
            let assigned_cols = py_i64(&pane["assigned_cols"]);
            let assigned_rows = py_i64(&pane["assigned_rows"]);
            let mut entry = Map::new();
            entry.insert("pane_id".into(), pane["pane_id"].clone());
            entry.insert(
                "assigned".into(),
                json!({"cols": pane["assigned_cols"], "rows": pane["assigned_rows"]}),
            );
            entry.insert(
                "has_panel".into(),
                json!(pane.get("has_panel").map(json_truthy).unwrap_or(true)),
            );
            if let (Some(rendered_cols_value), Some(rendered_rows_value)) = (
                pane.get("rendered_cols").filter(|value| !value.is_null()),
                pane.get("rendered_rows").filter(|value| !value.is_null()),
            ) {
                let rendered_cols = py_i64(rendered_cols_value);
                let rendered_rows = py_i64(rendered_rows_value);
                entry.insert(
                    "rendered".into(),
                    json!({"cols": rendered_cols_value, "rows": rendered_rows_value}),
                );
                entry.insert(
                    "match".into(),
                    json!(grid_match(
                        assigned_cols,
                        assigned_rows,
                        rendered_cols,
                        rendered_rows,
                        pane.get("exact_cols").map(json_truthy).unwrap_or(false),
                        pane.get("exact_rows").map(json_truthy).unwrap_or(false),
                    )),
                );
            }
            Value::Object(entry)
        })
        .collect::<Vec<_>>();
    let mut payload = Map::new();
    payload.insert("tab_id".into(), json!(tab_id));
    payload.insert("structure_version".into(), json!(structure_version));
    payload.insert("zoomed".into(), json!(zoomed));
    payload.insert("base".into(), json!({"cols": base_cols, "rows": base_rows}));
    payload.insert("panes".into(), Value::Array(rows));
    payload.insert("visible_for_sizing".into(), json!(visible_for_sizing));
    if let Some((cols, rows)) = pushed {
        payload.insert("pushed".into(), json!({"cols": cols, "rows": rows}));
    }
    Value::Object(payload)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchResult {
    pub ok: bool,
    pub code: Option<String>,
    pub method: Option<String>,
    pub socket: Option<String>,
    pub session: Option<String>,
    pub target: Option<AttachWindowTarget>,
    pub activate: bool,
    pub create: bool,
}

impl DispatchResult {
    fn error(code: &str) -> Self {
        Self {
            ok: false,
            code: Some(code.into()),
            method: None,
            socket: None,
            session: None,
            target: None,
            activate: false,
            create: false,
        }
    }

    pub fn to_value(&self) -> Value {
        if let Some(code) = &self.code {
            return json!({"ok": false, "code": code});
        }
        json!({
            "ok": self.ok,
            "method": self.method,
            "socket": self.socket,
            "session": self.session,
            "target": self.target,
            "activate": self.activate,
            "create": self.create,
        })
    }
}

/// Validate a `remote.herdr.*` call without performing socket or UI work.
pub fn dispatch<'a, P>(method: &str, params: P, enabled: bool) -> DispatchResult
where
    P: Into<Option<&'a Value>>,
{
    if !SOCKET_METHODS.contains(&method) {
        return DispatchResult::error("unknown_method");
    }
    if !enabled {
        return DispatchResult::error("disabled");
    }
    let empty = Value::Object(Map::new());
    let body = params.into().unwrap_or(&empty);
    let socket = validate_socket_path(
        body.get("socket")
            .filter(|value| json_truthy(value))
            .or_else(|| body.get("socket_path"))
            .map(value_string)
            .as_deref(),
    );
    let Some(socket) = socket else {
        return DispatchResult::error("invalid_params");
    };
    let session = if SESSION_METHODS.contains(&method) {
        let session = validate_session_name(
            body.get("session")
                .filter(|value| !value.is_null())
                .map(value_string)
                .as_deref(),
        );
        let Some(session) = session else {
            return DispatchResult::error("invalid_params");
        };
        Some(session)
    } else {
        None
    };
    DispatchResult {
        ok: true,
        code: None,
        method: Some(method.to_string()),
        socket: Some(socket),
        session,
        target: Some(window_target_from_params(
            body,
            method == "remote.herdr.window",
        )),
        activate: body
            .get("activate")
            .map(json_truthy)
            .unwrap_or(false),
        create: body.get("create").map(json_truthy).unwrap_or(false),
    }
}

#[derive(Debug, Clone)]
pub struct LifecycleController {
    pub enabled: bool,
    pub app_ready: bool,
    pub registry: AttachRegistry,
    pub mirrors: BTreeMap<String, MirrorRecord>,
    pub connections: BTreeMap<String, ConnectionRecord>,
    pub live_windows: Vec<String>,
    pub active_window_id: Option<String>,
    pub persist: Option<RestoreRecord>,
    pub events: Vec<Value>,
    window_seq: u64,
    pub server_stopped: bool,
}

impl Default for LifecycleController {
    fn default() -> Self {
        Self::new(true, true)
    }
}

impl LifecycleController {
    pub fn new(enabled: bool, app_ready: bool) -> Self {
        Self {
            enabled,
            app_ready,
            registry: AttachRegistry::default(),
            mirrors: BTreeMap::new(),
            connections: BTreeMap::new(),
            live_windows: vec!["win-active".into()],
            active_window_id: Some("win-active".into()),
            persist: None,
            events: Vec::new(),
            window_seq: 0,
            server_stopped: false,
        }
    }

    fn log(&mut self, event: &str, fields: impl IntoIterator<Item = (&'static str, Value)>) {
        let mut row = Map::new();
        row.insert("event".into(), json!(event));
        row.extend(fields.into_iter().map(|(key, value)| (key.into(), value)));
        self.events.push(Value::Object(row));
    }

    fn existing_window(&self) -> Option<String> {
        existing_mirror_window(
            &self.mirrors.values().cloned().collect::<Vec<_>>(),
            &self.live_windows,
        )
    }

    pub fn attach(
        &mut self,
        socket_path: &str,
        sessions: &[DiscoveredSession],
        target: &AttachWindowTarget,
        activate: bool,
    ) -> Value {
        let hashed = endpoint_hash(socket_path);
        let existing = self.existing_window();
        let preflight = plan_attach(
            target,
            self.enabled,
            self.app_ready,
            self.registry.is_attaching(&hashed),
            existing.as_deref(),
            self.active_window_id.as_deref(),
            &self.live_windows,
            None,
            None,
            None,
            activate,
            None,
        );
        if preflight.outcome != "discover" {
            self.log(
                "attach_reject",
                [
                    ("reason", json!(preflight.reason)),
                    ("endpoint_hash", json!(hashed)),
                ],
            );
            return json!({
                "ok": false,
                "outcome": preflight.outcome,
                "reason": preflight.reason,
            });
        }
        if !self.registry.begin_attach(&hashed) {
            self.log(
                "attach_reject",
                [
                    ("reason", json!("reentrant")),
                    ("endpoint_hash", json!(hashed)),
                ],
            );
            return json!({"ok": false, "outcome": "already_attaching", "reason": "reentrant"});
        }

        let result = self.attach_started(socket_path, sessions, target, activate, &hashed);
        self.registry.end_attach(&hashed);
        result
    }

    fn attach_started(
        &mut self,
        socket_path: &str,
        sessions: &[DiscoveredSession],
        target: &AttachWindowTarget,
        activate: bool,
        hashed: &str,
    ) -> Value {
        self.mirrors.retain(|_, record| record.workspace_id.is_some());
        let live_session_ids = self
            .connections
            .iter()
            .filter(|(_, connection)| may_cache_connection(connection))
            .map(|(session_id, _)| session_id.clone())
            .collect::<Vec<_>>();
        let mirrors = self.mirrors.values().cloned().collect::<Vec<_>>();
        let existing = self.existing_window();
        let plan = plan_attach(
            target,
            self.enabled,
            self.app_ready,
            false,
            existing.as_deref(),
            self.active_window_id.as_deref(),
            &self.live_windows,
            Some(sessions),
            Some(&mirrors),
            Some(&live_session_ids),
            activate,
            None,
        );
        if matches!(plan.outcome.as_str(), "no_sessions" | "invalid_target" | "failed_empty") {
            self.log(
                "attach_reject",
                [
                    ("reason", json!(plan.reason)),
                    ("endpoint_hash", json!(hashed)),
                ],
            );
            return json!({"ok": false, "outcome": plan.outcome, "reason": plan.reason});
        }

        let mut window_id = plan.window_id.clone();
        if plan.create_window {
            self.window_seq += 1;
            let created = format!("win-new-{}", self.window_seq);
            window_id = Some(created.clone());
            self.live_windows.push(created.clone());
            for record in self.mirrors.values_mut() {
                if record.workspace_id.is_some() {
                    record.window_id.clone_from(&created);
                }
            }
        }

        let mut workspace_ids = Vec::new();
        for session_id in &plan.sessions_to_reuse {
            if let Some(workspace_id) = self
                .mirrors
                .get(session_id)
                .and_then(|record| record.workspace_id.clone())
            {
                workspace_ids.push(workspace_id);
            }
        }
        for session_id in &plan.sessions_to_mirror {
            let workspace_id = format!("ws-{session_id}");
            self.mirrors.insert(
                session_id.clone(),
                MirrorRecord::new(
                    session_id,
                    window_id.as_deref().unwrap_or("win-active"),
                    Some(workspace_id.clone()),
                ),
            );
            if connection_action(self.connections.get(session_id)) != "reuse" {
                let mut connection = ConnectionRecord::started(session_id);
                connection.snapshot_received = true;
                self.connections.insert(session_id.clone(), connection);
            }
            workspace_ids.push(workspace_id);
        }

        if workspace_ids.is_empty() {
            if plan.create_window {
                if let Some(window_id) = &window_id {
                    self.live_windows.retain(|item| item != window_id);
                }
            }
            self.log(
                "attach_reject",
                [
                    ("reason", json!("no_workspaces")),
                    ("endpoint_hash", json!(hashed)),
                ],
            );
            return json!({"ok": false, "outcome": "failed_empty", "reason": "no_workspaces"});
        }

        self.persist = Some(RestoreRecord::new(
            hashed,
            socket_path,
            sessions
                .iter()
                .map(|session| session.session_id.clone())
                .collect(),
            target.kind.clone(),
            window_id.clone(),
        ));
        if activate && window_id.is_some() {
            self.active_window_id.clone_from(&window_id);
        }
        self.log(
            "attach_ok",
            [
                ("outcome", json!(plan.outcome)),
                ("endpoint_hash", json!(hashed)),
                ("session_count", json!(workspace_ids.len())),
            ],
        );
        json!({
            "ok": true,
            "outcome": plan.outcome,
            "window_id": window_id,
            "workspace_ids": workspace_ids,
            "post_attach": plan.post_attach,
            "server_stopped": self.server_stopped,
        })
    }

    pub fn detach(&mut self, session_id: &str, reason: &str) -> Value {
        if !self.enabled {
            return json!({"ok": false, "outcome": "disabled", "reason": "beta_disabled"});
        }
        self.mirrors.remove(session_id);
        if let Some(mut connection) = self.connections.remove(session_id) {
            connection.exited = true;
        }
        self.log(
            "detach",
            [
                ("session_id", json!(session_id)),
                ("reason", json!(reason)),
            ],
        );
        json!({
            "ok": true,
            "outcome": "detached",
            "session": session_id,
            "reason": reason,
            "server_stopped": false,
        })
    }

    pub fn close_host(&mut self, source: &str) -> Value {
        if host_close_policy(source) != "detach" {
            return json!({"ok": true, "outcome": "noop", "server_stopped": false});
        }
        let session_ids = self.mirrors.keys().cloned().collect::<Vec<_>>();
        let detached = session_ids
            .iter()
            .map(|session_id| self.detach(session_id, TEARDOWN_EXPLICIT_DETACH))
            .count();
        json!({
            "ok": true,
            "outcome": "detach",
            "detached": detached,
            "server_stopped": false,
        })
    }

    pub fn restore(&mut self, sessions: &[DiscoveredSession]) -> Value {
        let Some(record) = self.persist.clone() else {
            return json!({"ok": false, "outcome": "no_persist"});
        };
        let plan = plan_restore(
            &record,
            self.enabled,
            self.app_ready,
            sessions,
            &self.live_windows,
            self.active_window_id.as_deref(),
        );
        if !matches!(plan.outcome.as_str(), "mirrored" | "reused") {
            self.log(
                "restore",
                [
                    ("outcome", json!(plan.outcome)),
                    ("reason", json!(plan.reason)),
                ],
            );
            return json!({"ok": false, "outcome": plan.outcome, "reason": plan.reason});
        }
        let target = if record
            .window_id
            .as_ref()
            .is_some_and(|window| self.live_windows.contains(window))
        {
            AttachWindowTarget::new("explicit", record.window_id.clone())
        } else if record.target_kind == "dedicated_new_window" {
            AttachWindowTarget::new("dedicated_new_window", None::<String>)
        } else {
            AttachWindowTarget::new("contextual", None::<String>)
        };
        let mut result = self.attach(&record.socket_path, sessions, &target, true);
        if result.get("ok").and_then(Value::as_bool) == Some(true) {
            if let Some(body) = result.as_object_mut() {
                body.insert("post_attach".into(), json!(POST_RESEED));
                body.insert("mode".into(), json!("reattach"));
            }
        }
        self.log(
            "restore",
            [("outcome", result.get("outcome").cloned().unwrap_or(Value::Null))],
        );
        result
    }

    pub fn state(&self, session_id: &str) -> Value {
        let Some(connection) = self
            .connections
            .get(session_id)
            .filter(|connection| !connection.exited)
        else {
            return json!({"session": session_id, "attached": false});
        };
        json!({
            "session": session_id,
            "attached": true,
            "started": connection.started,
            "snapshot_received": connection.snapshot_received,
            "exited": connection.exited,
            "window_count": connection.window_ids.len(),
            "window_ids": connection.window_ids,
            "total_output_bytes": connection.total_output_bytes,
            "pane_output_bytes": connection.pane_output_bytes,
            "recent_events": connection.recent_events,
        })
    }
}

pub fn note_output(connection: &mut ConnectionRecord, pane_id: &str, byte_count: i64) {
    connection.total_output_bytes += byte_count;
    *connection
        .pane_output_bytes
        .entry(pane_id.to_string())
        .or_default() += byte_count;
    connection
        .recent_events
        .push(format!("output pane={pane_id} bytes={byte_count}"));
    let excess = connection.recent_events.len().saturating_sub(32);
    if excess > 0 {
        connection.recent_events.drain(..excess);
    }
}

fn py_i64(value: &Value) -> i64 {
    match value {
        Value::Bool(value) => i64::from(*value),
        Value::Number(value) => value
            .as_i64()
            .or_else(|| value.as_f64().map(|number| number as i64))
            .unwrap_or(0),
        Value::String(value) => value.parse().unwrap_or(0),
        _ => 0,
    }
}

fn value_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "None".into(),
        Value::Bool(value) => {
            if *value { "True".into() } else { "False".into() }
        }
        _ => value.to_string(),
    }
}

fn json_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(value) => !value.is_empty(),
        Value::Object(value) => !value.is_empty(),
    }
}
