//! Host fingerprint, parent binding, and pane/session association store.
//!
//! Port of the state-store layer of `bridge/cmux_herdr_bridge.py`
//! (`collect_host_fingerprint`, `_parent_key`, parent-binding load/save,
//! association-map load/save/update, `resolve_association_parents`,
//! `_association_record`, `set_title_lock`, `format_associations`).
//!
//! Fingerprints, keys, and record shapes match Python exactly so state files
//! written by either runtime interoperate. State is XDG-only
//! (`$XDG_STATE_HOME/cmux-herdr`, default `~/.local/state/cmux-herdr`).
//!
//! Atomic writes mirror Python: write a sibling temp file, `chmod 0600`, then
//! rename over the target. No fsync (matches Python; a documented durability
//! improvement to add later).

use std::collections::BTreeSet;
use std::io::Write;
use std::path::PathBuf;

use serde_json::{json, Map, Value};

use crate::model::{Pane, Snapshot, STATUS_PREFIX};

/// Host fingerprint fields (`collect_host_fingerprint`). Optional ints stay
/// `Option<i64>`; missing/empty strings are `None`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Fingerprint {
    pub cmux_surface_id: Option<String>,
    pub herdr_socket_path: Option<String>,
    pub herdr_server_pid: Option<i64>,
    pub herdr_workspace_id: Option<String>,
}

impl Fingerprint {
    /// Render as the JSON object Python persists into binding/association files.
    fn as_json_fields(&self) -> Vec<(&'static str, Value)> {
        vec![
            ("cmux_surface_id", opt_str_json(&self.cmux_surface_id)),
            ("herdr_socket_path", opt_str_json(&self.herdr_socket_path)),
            ("herdr_workspace_id", opt_str_json(&self.herdr_workspace_id)),
            ("herdr_server_pid", opt_i64_json(self.herdr_server_pid)),
        ]
    }
}

fn opt_str_json(v: &Option<String>) -> Value {
    match v {
        Some(s) => Value::String(s.clone()),
        None => Value::Null,
    }
}

fn opt_i64_json(v: Option<i64>) -> Value {
    match v {
        Some(n) => Value::Number(n.into()),
        None => Value::Null,
    }
}

// --- environment / clock seam ------------------------------------------------

/// Reads environment and wall clock. A trait so tests can pin values (Python
/// reads `os.environ` and `time.time()` directly).
pub trait HostEnv {
    fn var(&self, name: &str) -> Option<String>;
    /// Wall-clock seconds as a float (`time.time()`).
    fn now(&self) -> f64;
    /// Read a small text file (for the sidecar pid probe). `None` on any error.
    fn read_file(&self, path: &str) -> Option<String>;
}

/// Production [`HostEnv`]: real process environment, clock, and filesystem.
pub struct SystemEnv;

impl HostEnv for SystemEnv {
    fn var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
    fn now(&self) -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    }
    fn read_file(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }
}

/// `os.environ.get(name) or None` — empty string coerces to `None`.
fn env_nonempty(env: &dyn HostEnv, name: &str) -> Option<String> {
    env.var(name).filter(|v| !v.is_empty())
}

// --- state directory ---------------------------------------------------------

/// User-scoped state directory (`_state_dir`), honoring `XDG_STATE_HOME`.
pub fn state_dir(env: &dyn HostEnv) -> PathBuf {
    let root = env_nonempty(env, "XDG_STATE_HOME").unwrap_or_else(|| {
        let home = env.var("HOME").unwrap_or_default();
        format!("{home}/.local/state")
    });
    PathBuf::from(root).join("cmux-herdr")
}

// --- fingerprint -------------------------------------------------------------

/// Return the Herdr server pid when cheaply available (`_herdr_server_pid`).
pub fn herdr_server_pid(env: &dyn HostEnv) -> Option<i64> {
    if let Some(raw) = env.var("HERDR_SERVER_PID") {
        if !raw.is_empty() && raw.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(n) = raw.parse::<i64>() {
                return Some(n);
            }
        }
    }
    let sock = env.var("HERDR_SOCKET_PATH").filter(|s| !s.is_empty())?;
    let dir = std::path::Path::new(&sock)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let candidates = [format!("{sock}.pid"), format!("{dir}/herdr.pid")];
    for path in candidates {
        if let Some(text) = env.read_file(&path) {
            let text = text.trim();
            if !text.is_empty() && text.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(n) = text.parse::<i64>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

/// Collect the invoking-environment host fingerprint (`collect_host_fingerprint`).
pub fn collect_host_fingerprint(env: &dyn HostEnv) -> Fingerprint {
    Fingerprint {
        cmux_surface_id: env_nonempty(env, "CMUX_SURFACE_ID"),
        herdr_socket_path: env_nonempty(env, "HERDR_SOCKET_PATH"),
        herdr_server_pid: herdr_server_pid(env),
        herdr_workspace_id: env_nonempty(env, "HERDR_WORKSPACE_ID"),
    }
}

/// Required fingerprint env names that are unset (`fingerprint_missing_fields`).
pub fn fingerprint_missing_fields(fp: &Fingerprint) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if fp.cmux_surface_id.is_none() {
        missing.push("CMUX_SURFACE_ID");
    }
    if fp.herdr_socket_path.is_none() {
        missing.push("HERDR_SOCKET_PATH");
    }
    missing
}

/// Stable filename key for one parent binding / association file (`_parent_key`).
pub fn parent_key(fp: &Fingerprint) -> String {
    let surface = fp.cmux_surface_id.as_deref().unwrap_or("_nosurface_");
    let socket_path = fp.herdr_socket_path.as_deref().unwrap_or("default");
    let herdr_ws = fp.herdr_workspace_id.as_deref().unwrap_or("default");
    let pid_part = match fp.herdr_server_pid {
        Some(pid) => pid.to_string(),
        None => "nopid".into(),
    };
    let material = format!("{surface}|{socket_path}|{herdr_ws}|{pid_part}");
    let digest = sha256_hex(material.as_bytes());
    let digest16 = &digest[..16];
    let surface_slug = slugify(surface);
    let surface_slug = if surface_slug.is_empty() {
        "host".to_string()
    } else {
        surface_slug
    };
    format!("{surface_slug}-{digest16}")
}

/// `re.sub(r"[^A-Za-z0-9_.-]+", "_", surface)[:48]` — collapse runs of
/// disallowed chars to a single `_`, then truncate to 48 chars.
fn slugify(surface: &str) -> String {
    let mut out = String::new();
    let mut in_run = false;
    for ch in surface.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-') {
            out.push(ch);
            in_run = false;
        } else if !in_run {
            out.push('_');
            in_run = true;
        }
    }
    out.chars().take(48).collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut s = String::with_capacity(64);
    for b in digest {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Parent-binding file path for one host fingerprint.
pub fn binding_path(env: &dyn HostEnv, fp: &Fingerprint) -> PathBuf {
    state_dir(env).join(format!("parent-{}.json", parent_key(fp)))
}

/// Association-cache file path for one host fingerprint.
pub fn association_path(env: &dyn HostEnv, fp: &Fingerprint) -> PathBuf {
    state_dir(env).join(format!("associations-{}.json", parent_key(fp)))
}

// --- parent binding ----------------------------------------------------------

/// True when a persisted binding belongs to the invoking fingerprint
/// (`_binding_matches_fingerprint`).
fn binding_matches_fingerprint(data: &Value, fp: &Fingerprint) -> bool {
    let get = |k: &str| data.get(k);
    if get("cmux_surface_id") != Some(&opt_str_json(&fp.cmux_surface_id)) {
        return false;
    }
    if get("herdr_socket_path") != Some(&opt_str_json(&fp.herdr_socket_path)) {
        return false;
    }
    let stored_ws = get("herdr_workspace_id").and_then(Value::as_str);
    let current_ws = fp.herdr_workspace_id.as_deref();
    if let (Some(s), Some(c)) = (stored_ws.filter(|s| !s.is_empty()), current_ws) {
        if s != c {
            return false;
        }
    }
    let stored_pid = get("herdr_server_pid").and_then(as_int_value);
    if let (Some(s), Some(c)) = (stored_pid, fp.herdr_server_pid) {
        if s != c {
            return false;
        }
    }
    true
}

/// JSON integer only (Python `isinstance(x, int)`; bools excluded).
fn as_int_value(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) if n.is_i64() || n.is_u64() => n.as_i64(),
        _ => None,
    }
}

/// Load the persisted parent-binding workspace ref, or `None`
/// (`_load_parent_binding`).
pub fn load_parent_binding(env: &dyn HostEnv, fp: &Fingerprint) -> Option<String> {
    let text = env.read_file(binding_path(env, fp).to_str()?)?;
    let data: Value = serde_json::from_str(&text).ok()?;
    if !data.is_object() {
        return None;
    }
    if !binding_matches_fingerprint(&data, fp) {
        return None;
    }
    match data.get("workspace_ref") {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

/// Persist the parent-binding workspace ref (`_save_parent_binding`).
pub fn save_parent_binding(
    env: &dyn HostEnv,
    workspace: &str,
    fp: &Fingerprint,
) -> std::io::Result<()> {
    let dir = state_dir(env);
    let mut payload = Map::new();
    payload.insert("workspace_ref".into(), Value::String(workspace.to_string()));
    for (k, v) in fp.as_json_fields() {
        payload.insert(k.into(), v);
    }
    payload.insert("host_fingerprint_key".into(), Value::String(parent_key(fp)));
    payload.insert("updated_at".into(), json!(env.now()));
    let body = serde_json::to_string(&Value::Object(payload)).unwrap();
    atomic_write(
        &dir,
        ".parent-",
        &binding_path(env, fp),
        &format!("{body}\n"),
    )
}

// --- association map ---------------------------------------------------------

/// Load the association cache (`_load_association_map`); returns a fresh default
/// map when missing/invalid.
pub fn load_association_map(env: &dyn HostEnv, fp: &Fingerprint) -> Value {
    if let Some(path) = association_path(env, fp).to_str().map(str::to_string) {
        if let Some(text) = env.read_file(&path) {
            if let Ok(mut data) = serde_json::from_str::<Value>(&text) {
                let valid = data.get("version") == Some(&json!(1))
                    && data.get("panes").map(Value::is_object).unwrap_or(false);
                if valid {
                    if !data.get("mirrors").map(Value::is_object).unwrap_or(false) {
                        data["mirrors"] = json!({});
                    }
                    return data;
                }
            }
        }
    }
    default_association_map(fp)
}

fn default_association_map(fp: &Fingerprint) -> Value {
    json!({
        "version": 1,
        "panes": {},
        "mirrors": {},
        "cmux_workspace": Value::Null,
        "herdr_socket_path": opt_str_json(&fp.herdr_socket_path),
        "herdr_workspace_id": opt_str_json(&fp.herdr_workspace_id),
        "cmux_surface_id": opt_str_json(&fp.cmux_surface_id),
        "herdr_server_pid": opt_i64_json(fp.herdr_server_pid),
        "host_fingerprint_key": parent_key(fp),
        "updated_at": Value::Null,
    })
}

/// Persist the association cache (`_save_association_map`): stamps version,
/// timestamp, and fingerprint fields, then atomic-writes with sorted keys.
pub fn save_association_map(
    env: &dyn HostEnv,
    state: &Value,
    fp: &Fingerprint,
) -> std::io::Result<()> {
    let dir = state_dir(env);
    let mut state = state.as_object().cloned().unwrap_or_default();
    state.insert("version".into(), json!(1));
    state.insert("updated_at".into(), json!(env.now()));
    for (k, v) in fp.as_json_fields() {
        state.insert(k.into(), v);
    }
    state.insert("host_fingerprint_key".into(), Value::String(parent_key(fp)));
    // Python: json.dump(state, indent=2, sort_keys=True).
    let body = to_json_sorted_indent(&Value::Object(state));
    atomic_write(
        &dir,
        ".assoc-",
        &association_path(env, fp),
        &format!("{body}\n"),
    )
}

// --- parent resolution -------------------------------------------------------

/// `_nonempty_id`: stripped non-empty string, or `None`.
fn nonempty_id(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        _ => None,
    }
}

/// Python `a or b`: first operand if truthy, else the second.
fn py_or<'a>(a: Option<&'a Value>, b: Option<&'a Value>) -> Option<&'a Value> {
    if truthy(a) {
        a
    } else {
        b
    }
}

fn nonempty_str(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

/// Prior association record for a pane, or `{}` when the session instance
/// changed (`_prior_for_pane`).
pub fn prior_for_pane<'a>(pane: &Pane, previous: &'a Value) -> &'a Value {
    static EMPTY: Value = Value::Null;
    let prior = previous.get(&pane.pane_id);
    let Some(prior) = prior.filter(|p| p.is_object()) else {
        return &EMPTY;
    };
    let prior_sid = prior.get("agent_session_id").and_then(Value::as_str);
    let new_sid = pane.agent_session_id.as_deref();
    if let (Some(p), Some(n)) = (
        prior_sid.filter(|s| !s.is_empty()),
        new_sid.filter(|s| !s.is_empty()),
    ) {
        if p != n {
            return &EMPTY;
        }
    }
    prior
}

/// Prompt-time parent heuristic (`_infer_parent_once`). Returns
/// `(tab_id, workspace_id)`.
pub fn infer_parent_once(
    env: &dyn HostEnv,
    pane: &Pane,
    snapshot: Option<&Snapshot>,
) -> (Option<String>, Option<String>) {
    if let Some(env_pane) = env.var("HERDR_PANE_ID") {
        if !env_pane.is_empty() && env_pane == pane.pane_id {
            let tab = env.var("HERDR_TAB_ID").and_then(|s| nonempty_str(&s));
            let ws = env
                .var("HERDR_WORKSPACE_ID")
                .and_then(|s| nonempty_str(&s))
                .or_else(|| nonempty_str(&pane.workspace_id));
            if tab.is_some() || ws.is_some() {
                return (tab, ws);
            }
        }
    }
    if let Some(snapshot) = snapshot {
        let ws = nonempty_str(&pane.workspace_id);
        let tabs: Vec<_> = snapshot
            .tabs
            .iter()
            .filter(|t| {
                !t.tab_id.is_empty() && (ws.is_none() || Some(&t.workspace_id) == ws.as_ref())
            })
            .collect();
        if tabs.len() == 1 {
            let t = tabs[0];
            let tab_ws = nonempty_str(&t.workspace_id).or(ws);
            return (Some(t.tab_id.clone()), tab_ws);
        }
    }
    (None, None)
}

/// Two-pass parent resolution for one pane (`resolve_association_parents`).
pub fn resolve_association_parents(
    env: &dyn HostEnv,
    pane: &Pane,
    prior: &Value,
    snapshot: Option<&Snapshot>,
) -> Value {
    let prior = if prior.is_object() {
        prior
    } else {
        &Value::Null
    };
    let satisfied = truthy(prior.get("heuristic_satisfied"));
    let prior_tab = nonempty_id(py_or(prior.get("parent_tab_id"), prior.get("tab_id")));
    let prior_ws = nonempty_id(py_or(
        prior.get("parent_workspace_id"),
        prior.get("workspace_id"),
    ));
    let snap_tab = nonempty_str(&pane.tab_id);
    let snap_ws = nonempty_str(&pane.workspace_id);

    let mut tab_id = snap_tab;
    let mut workspace_id = snap_ws;
    let mut used_heuristic = false;

    if tab_id.is_none() || workspace_id.is_none() {
        if satisfied && (prior_tab.is_some() || prior_ws.is_some()) {
            tab_id = tab_id.or(prior_tab);
            workspace_id = workspace_id.or(prior_ws);
        } else {
            let (inferred_tab, inferred_ws) = infer_parent_once(env, pane, snapshot);
            if inferred_tab.is_some() || inferred_ws.is_some() {
                used_heuristic = true;
                tab_id = tab_id.or(inferred_tab);
                workspace_id = workspace_id.or(inferred_ws);
            }
        }
    }

    let now_satisfied = satisfied || (tab_id.is_some() && workspace_id.is_some()) || used_heuristic;
    json!({
        "parent_tab_id": opt_str_json(&tab_id),
        "parent_workspace_id": opt_str_json(&workspace_id),
        "heuristic_satisfied": now_satisfied,
        "used_heuristic": used_heuristic,
    })
}

/// The contract association key `pane_id:session_id` (or `pane_id`)
/// (`association_key_for_pane`).
pub fn association_key_for_pane(pane: &Pane) -> String {
    let sid = pane.agent_session_id.as_deref().unwrap_or("").trim();
    if sid.is_empty() {
        pane.pane_id.clone()
    } else {
        format!("{}:{}", pane.pane_id, sid)
    }
}

/// Build one association cache record (`_association_record`).
pub fn association_record(
    env: &dyn HostEnv,
    pane: &Pane,
    prior: &Value,
    parents: &Value,
    write_meta: Option<&Value>,
) -> Value {
    let meta = write_meta.filter(|m| m.is_object());
    let has = |k: &str| meta.and_then(|m| m.get(k)).is_some();
    // title_lock = meta['title_lock'] if 'title_lock' in meta else prior['title_lock']
    let title_lock = if has("title_lock") {
        truthy(meta.and_then(|m| m.get("title_lock")))
    } else {
        truthy(prior.get("title_lock"))
    };
    let mut locked_title = meta
        .and_then(|m| m.get("locked_title"))
        .filter(|v| !v.is_null())
        .cloned();
    if title_lock && !truthy(locked_title.as_ref()) {
        // prior.locked_title or pane.display_name
        locked_title = match prior.get("locked_title") {
            Some(v) if truthy(Some(v)) => Some(v.clone()),
            _ => Some(Value::String(pane.display_name())),
        };
    }
    if !title_lock {
        locked_title = None;
    }
    let locked_title = locked_title.unwrap_or(Value::Null);

    // meta.get(k, prior.get(k)) for last_* fields.
    let meta_or_prior = |k: &str| -> Value {
        if let Some(v) = meta.and_then(|m| m.get(k)) {
            v.clone()
        } else {
            prior.get(k).cloned().unwrap_or(Value::Null)
        }
    };

    let parent_tab = match parents.get("parent_tab_id") {
        Some(v) if truthy(Some(v)) => v.clone(),
        _ => Value::String(pane.tab_id.clone()),
    };
    let parent_ws = match parents.get("parent_workspace_id") {
        Some(v) if truthy(Some(v)) => v.clone(),
        _ => Value::String(pane.workspace_id.clone()),
    };

    let first_seen = match prior.get("first_seen_at") {
        Some(v) if truthy(Some(v)) => v.clone(),
        _ => json!(env.now()),
    };

    json!({
        "pane_id": pane.pane_id,
        "association_key": association_key_for_pane(pane),
        "tab_id": pane.tab_id,
        "workspace_id": pane.workspace_id,
        "parent_tab_id": parent_tab,
        "parent_workspace_id": parent_ws,
        "heuristic_satisfied": truthy(parents.get("heuristic_satisfied")),
        "title_lock": title_lock,
        "locked_title": locked_title,
        "last_status_value": meta_or_prior("last_status_value"),
        "last_icon": meta_or_prior("last_icon"),
        "last_color": meta_or_prior("last_color"),
        "last_priority": meta_or_prior("last_priority"),
        "agent": opt_str_json(&pane.agent),
        "agent_status": pane.agent_status,
        "status_key": pane.status_key(),
        "label": opt_str_json(&pane.label),
        "cwd": opt_str_json(&pane.cwd),
        "focused": pane.focused,
        "agent_session_path": opt_str_json(&pane.agent_session_path),
        "agent_session_id": opt_str_json(&pane.agent_session_id),
        "agent_session_kind": opt_str_json(&pane.agent_session_kind),
        "revision": opt_i64_json(pane.revision),
        "first_seen_at": first_seen,
        "last_seen_at": json!(env.now()),
    })
}

/// Rewrite the association cache from a live snapshot (`update_association_map`).
/// Returns the summary object Python returns.
pub fn update_association_map(
    env: &dyn HostEnv,
    snapshot: &Snapshot,
    cmux_workspace: Option<&str>,
    write_meta: Option<&Value>,
) -> std::io::Result<Value> {
    let fp = collect_host_fingerprint(env);
    let mut state = load_association_map(env, &fp);
    let previous = state
        .get("panes")
        .filter(|p| p.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}));
    let meta_by_pane = write_meta.filter(|m| m.is_object());

    let mut live = Map::new();
    for pane in &snapshot.panes {
        if pane.pane_id.is_empty() {
            continue;
        }
        let prior = prior_for_pane(pane, &previous).clone();
        let meta = meta_by_pane
            .and_then(|m| m.get(&pane.pane_id))
            .filter(|v| v.is_object());
        let empty_meta = json!({});
        let meta = meta.unwrap_or(&empty_meta);

        let mut parents = resolve_association_parents(env, pane, &prior, Some(snapshot));
        // meta may override parentage.
        let meta_has_parent = truthy(meta.get("parent_tab_id"))
            || meta
                .get("heuristic_satisfied")
                .map(|v| !v.is_null())
                .unwrap_or(false);
        if meta_has_parent {
            let parent_tab = meta
                .get("parent_tab_id")
                .cloned()
                .unwrap_or_else(|| parents["parent_tab_id"].clone());
            let parent_ws = meta
                .get("parent_workspace_id")
                .cloned()
                .unwrap_or_else(|| parents["parent_workspace_id"].clone());
            let heuristic = match meta.get("heuristic_satisfied") {
                Some(v) => truthy(Some(v)),
                None => truthy(parents.get("heuristic_satisfied")),
            };
            parents = json!({
                "parent_tab_id": parent_tab,
                "parent_workspace_id": parent_ws,
                "heuristic_satisfied": heuristic,
            });
        }
        let record = association_record(env, pane, &prior, &parents, Some(meta));
        live.insert(pane.pane_id.clone(), record);
    }

    // pruned = sorted(set(previous) - set(live))
    let prev_keys: BTreeSet<String> = previous
        .as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    let live_keys: BTreeSet<String> = live.keys().cloned().collect();
    let pruned: Vec<Value> = prev_keys
        .difference(&live_keys)
        .map(|k| Value::String(k.clone()))
        .collect();

    let state_obj = state.as_object_mut().unwrap();
    state_obj.insert("panes".into(), Value::Object(live.clone()));
    if !state_obj
        .get("mirrors")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state_obj.insert("mirrors".into(), json!({}));
    }
    // cmux_workspace or state.get("cmux_workspace")
    let new_ws = match cmux_workspace.filter(|s| !s.is_empty()) {
        Some(w) => Value::String(w.to_string()),
        None => state_obj
            .get("cmux_workspace")
            .cloned()
            .unwrap_or(Value::Null),
    };
    state_obj.insert("cmux_workspace".into(), new_ws.clone());
    state_obj.insert("pruned_pane_ids".into(), Value::Array(pruned.clone()));

    save_association_map(env, &state, &fp)?;
    Ok(json!({
        "path": association_path(env, &fp).to_string_lossy(),
        "pane_count": live.len(),
        "pruned": pruned,
        "cmux_workspace": new_ws,
        "host_fingerprint_key": parent_key(&fp),
    }))
}

/// Set or clear the native-title lock on one association record
/// (`set_title_lock`). Returns the updated entry, or `Err` when `pane_id` is
/// blank.
pub fn set_title_lock(
    env: &dyn HostEnv,
    pane_id: &str,
    locked: bool,
    title: Option<&str>,
) -> Result<Value, String> {
    let pane_id = pane_id.trim();
    if pane_id.is_empty() {
        return Err("pane_id is required to lock or unlock a title".into());
    }
    let fp = collect_host_fingerprint(env);
    let mut state = load_association_map(env, &fp);
    let panes = state
        .get("panes")
        .filter(|p| p.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}));
    let mut panes = panes.as_object().cloned().unwrap_or_default();
    let mut entry = panes
        .get(pane_id)
        .filter(|e| e.is_object())
        .cloned()
        .unwrap_or(Value::Null);
    if !truthy(Some(&entry)) {
        entry = json!({
            "pane_id": pane_id,
            "status_key": format!("{STATUS_PREFIX}{pane_id}"),
            "association_key": pane_id,
        });
    }
    let obj = entry.as_object_mut().unwrap();
    if locked {
        obj.insert("title_lock".into(), json!(true));
        match title.map(str::trim).filter(|t| !t.is_empty()) {
            Some(t) => {
                obj.insert("locked_title".into(), Value::String(t.to_string()));
            }
            None => {
                if !truthy(obj.get("locked_title")) {
                    let fallback = match obj.get("label") {
                        Some(v) if truthy(Some(v)) => v.clone(),
                        _ => Value::String(pane_id.to_string()),
                    };
                    obj.insert("locked_title".into(), fallback);
                }
            }
        }
    } else {
        obj.insert("title_lock".into(), json!(false));
        obj.remove("locked_title");
    }
    panes.insert(pane_id.to_string(), entry.clone());
    state
        .as_object_mut()
        .unwrap()
        .insert("panes".into(), Value::Object(panes));
    save_association_map(env, &state, &fp).map_err(|e| e.to_string())?;
    Ok(entry)
}

/// Human-readable association summary (`format_associations`).
pub fn format_associations(env: &dyn HostEnv, state: Option<&Value>) -> String {
    let loaded;
    let data = match state {
        Some(s) => s,
        None => {
            loaded = load_association_map(env, &collect_host_fingerprint(env));
            &loaded
        }
    };
    let panes = data.get("panes").filter(|p| p.is_object());
    let mirrors = data.get("mirrors").filter(|m| m.is_object());
    let pane_count = panes
        .and_then(Value::as_object)
        .map(|m| m.len())
        .unwrap_or(0);
    let mirror_count = mirrors
        .and_then(Value::as_object)
        .map(|m| m.len())
        .unwrap_or(0);

    let dash = |v: Option<&Value>| -> String {
        match v {
            Some(x) if truthy(Some(x)) => plain(x),
            _ => "-".into(),
        }
    };
    let fp = collect_host_fingerprint(env);
    let mut lines = vec![
        format!("associations: {pane_count} panes, {mirror_count} mirrored surfaces"),
        format!("  cmux_workspace={}", dash(data.get("cmux_workspace"))),
        format!("  herdr_workspace={}", dash(data.get("herdr_workspace_id"))),
        format!("  surface={}", dash(data.get("cmux_surface_id"))),
        format!("  herdr_pid={}", dash(data.get("herdr_server_pid"))),
        format!(
            "  fingerprint={}",
            match data.get("host_fingerprint_key") {
                Some(v) if truthy(Some(v)) => plain(v),
                _ => parent_key(&fp),
            }
        ),
        format!("  file={}", association_path(env, &fp).to_string_lossy()),
    ];

    if let Some(panes) = panes.and_then(Value::as_object) {
        let mut ids: Vec<&String> = panes.keys().collect();
        ids.sort();
        for pane_id in ids {
            let entry = panes.get(pane_id).filter(|e| e.is_object());
            let get = |k: &str| entry.and_then(|e| e.get(k));
            let mut session = match get("agent_session_path") {
                Some(v) if truthy(Some(v)) => plain(v),
                _ => match get("agent_session_id") {
                    Some(v) if truthy(Some(v)) => plain(v),
                    _ => "-".into(),
                },
            };
            if session.chars().count() > 60 {
                let tail: String = session.chars().skip(session.chars().count() - 57).collect();
                session = format!("\u{2026}{tail}");
            }
            let lock = if truthy(get("title_lock")) { "Y" } else { "n" };
            let heur = if truthy(get("heuristic_satisfied")) {
                "Y"
            } else {
                "n"
            };
            let parent = match get("parent_tab_id") {
                Some(v) if truthy(Some(v)) => plain(v),
                _ => match get("tab_id") {
                    Some(v) if truthy(Some(v)) => plain(v),
                    _ => "-".into(),
                },
            };
            let status = match get("agent_status") {
                Some(v) if truthy(Some(v)) => plain(v),
                _ => "?".into(),
            };
            let agent = match get("agent") {
                Some(v) if truthy(Some(v)) => plain(v),
                _ => "-".into(),
            };
            let status_key = match get("status_key") {
                Some(v) if truthy(Some(v)) => plain(v),
                _ => "-".into(),
            };
            lines.push(format!(
                "  {:10}  {:8}  {:8}  {}  parent={parent}  lock={lock}  heur={heur}  session={session}",
                pane_id, status, agent, status_key
            ));
        }
    }

    if let Some(mirrors) = mirrors.and_then(Value::as_object).filter(|m| !m.is_empty()) {
        lines.push("mirrors:".into());
        let mut ids: Vec<&String> = mirrors.keys().collect();
        ids.sort();
        for pane_id in ids {
            let entry = mirrors.get(pane_id).filter(|e| e.is_object());
            let get = |k: &str| entry.and_then(|e| e.get(k));
            let role = match get("role") {
                Some(v) if truthy(Some(v)) => plain(v),
                _ => "?".into(),
            };
            let surface = match get("cmux_surface_id") {
                Some(v) if truthy(Some(v)) => plain(v),
                _ => "-".into(),
            };
            let title = match get("title") {
                Some(v) if truthy(Some(v)) => plain(v),
                _ => "-".into(),
            };
            lines.push(format!("  {:10}  {:8}  {surface}  {title}", pane_id, role));
        }
    }
    lines.join("\n")
}

// --- helpers -----------------------------------------------------------------

/// Python truthiness of an optional JSON value.
fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::Bool(true)) => true,
        Some(Value::Number(n)) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
    }
}

/// Python `str(x)` for the scalar shapes reachable in `format_associations`.
fn plain(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => if *b { "True" } else { "False" }.into(),
        Value::Null => "None".into(),
        other => other.to_string(),
    }
}

/// Serialize with 2-space indent and sorted keys, matching Python's
/// `json.dump(indent=2, sort_keys=True)`. `preserve_order` keeps serde_json's
/// map order, so sort explicitly.
fn to_json_sorted_indent(v: &Value) -> String {
    let sorted = sort_value_keys(v);
    let mut buf = Vec::new();
    let fmt = serde_json::ser::PrettyFormatter::with_indent(b"  ");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, fmt);
    use serde::Serialize;
    sorted.serialize(&mut ser).unwrap();
    String::from_utf8(buf).unwrap()
}

fn sort_value_keys(v: &Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut sorted: Vec<(&String, &Value)> = map.iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(b.0));
            let mut out = Map::new();
            for (k, val) in sorted {
                out.insert(k.clone(), sort_value_keys(val));
            }
            Value::Object(out)
        }
        Value::Array(a) => Value::Array(a.iter().map(sort_value_keys).collect()),
        other => other.clone(),
    }
}

/// Atomic write: temp file in `dir`, `chmod 0600`, rename over `target`
/// (mirrors Python's `mkstemp` + `os.replace`). Creates `dir` mode 0700.
fn atomic_write(
    dir: &std::path::Path,
    prefix: &str,
    target: &std::path::Path,
    body: &str,
) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .or_else(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                Ok(())
            } else {
                Err(e)
            }
        })?;
    let mut tmp = tempfile::Builder::new()
        .prefix(prefix)
        .suffix(".tmp")
        .tempfile_in(dir)?;
    tmp.write_all(body.as_bytes())?;
    tmp.flush()?;
    std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o600))?;
    tmp.persist(target).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::pane_from_raw;
    use std::cell::RefCell;
    use std::collections::HashMap;

    /// Test env: fixed vars, pinned clock, in-memory files.
    struct TestEnv {
        vars: HashMap<String, String>,
        now: f64,
        files: RefCell<HashMap<String, String>>,
    }

    impl TestEnv {
        fn new() -> Self {
            TestEnv {
                vars: HashMap::new(),
                now: 1_700_000_000.0,
                files: RefCell::new(HashMap::new()),
            }
        }
        fn with(mut self, k: &str, v: &str) -> Self {
            self.vars.insert(k.into(), v.into());
            self
        }
    }

    impl HostEnv for TestEnv {
        fn var(&self, name: &str) -> Option<String> {
            self.vars.get(name).cloned()
        }
        fn now(&self) -> f64 {
            self.now
        }
        fn read_file(&self, path: &str) -> Option<String> {
            self.files
                .borrow()
                .get(path)
                .cloned()
                .or_else(|| std::fs::read_to_string(path).ok())
        }
    }

    fn pane(raw: Value) -> Pane {
        pane_from_raw(&raw)
    }

    #[test]
    fn parent_key_is_stable_and_slugged() {
        let fp = Fingerprint {
            cmux_surface_id: Some("my/surface id".into()),
            herdr_socket_path: Some("/tmp/h.sock".into()),
            herdr_server_pid: Some(42),
            herdr_workspace_id: Some("ws1".into()),
        };
        let k = parent_key(&fp);
        // slug replaces the non-allowed run "/","space" → "_".
        assert!(k.starts_with("my_surface_id-"));
        // 16 hex chars after the slug + dash.
        let digest = k.rsplit('-').next().unwrap();
        assert_eq!(digest.len(), 16);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
        // deterministic
        assert_eq!(k, parent_key(&fp));
    }

    #[test]
    fn parent_key_defaults_for_missing_fields() {
        let fp = Fingerprint::default();
        let k = parent_key(&fp);
        // surface slug falls back to "_nosurface_".
        assert!(k.starts_with("_nosurface_-"));
    }

    #[test]
    fn association_key_uses_session() {
        let p = pane(json!({"pane_id": "p1", "agent_session": {"kind": "id", "value": "s9"}}));
        assert_eq!(association_key_for_pane(&p), "p1:s9");
        let p2 = pane(json!({"pane_id": "p2"}));
        assert_eq!(association_key_for_pane(&p2), "p2");
    }

    #[test]
    fn resolve_parents_uses_snapshot_ids() {
        let env = TestEnv::new();
        let p = pane(json!({"pane_id": "p1", "tab_id": "t1", "workspace_id": "w1"}));
        let parents = resolve_association_parents(&env, &p, &Value::Null, None);
        assert_eq!(parents["parent_tab_id"], "t1");
        assert_eq!(parents["parent_workspace_id"], "w1");
        assert_eq!(parents["heuristic_satisfied"], true);
        assert_eq!(parents["used_heuristic"], false);
    }

    #[test]
    fn resolve_parents_infers_from_env_pane() {
        let env = TestEnv::new()
            .with("HERDR_PANE_ID", "p1")
            .with("HERDR_TAB_ID", "envtab")
            .with("HERDR_WORKSPACE_ID", "envws");
        let p = pane(json!({"pane_id": "p1"})); // no snapshot tab/ws
        let parents = resolve_association_parents(&env, &p, &Value::Null, None);
        assert_eq!(parents["parent_tab_id"], "envtab");
        assert_eq!(parents["parent_workspace_id"], "envws");
        assert_eq!(parents["used_heuristic"], true);
        assert_eq!(parents["heuristic_satisfied"], true);
    }

    #[test]
    fn prior_for_pane_drops_on_session_change() {
        let previous = json!({
            "p1": {"agent_session_id": "old", "title_lock": true}
        });
        let p = pane(json!({"pane_id": "p1", "agent_session": {"kind": "id", "value": "new"}}));
        let prior = prior_for_pane(&p, &previous);
        assert!(prior.is_null(), "session change should drop prior");
        // same session keeps it
        let p_same =
            pane(json!({"pane_id": "p1", "agent_session": {"kind": "id", "value": "old"}}));
        assert!(prior_for_pane(&p_same, &previous).is_object());
    }
    #[test]
    fn title_lock_roundtrip_via_files() {
        // Real temp state dir; TestEnv.read_file falls back to the fs, so both
        // the write and the reload use the same in-process env (no globals).
        let tmp = tempfile::tempdir().unwrap();
        let env = TestEnv::new()
            .with("XDG_STATE_HOME", tmp.path().to_str().unwrap())
            .with("CMUX_SURFACE_ID", "surf")
            .with("HERDR_SOCKET_PATH", "/tmp/h.sock");
        let entry = set_title_lock(&env, "p1", true, Some("LockedTitle")).unwrap();
        assert_eq!(entry["title_lock"], true);
        assert_eq!(entry["locked_title"], "LockedTitle");
        let fp = collect_host_fingerprint(&env);
        let reloaded = load_association_map(&env, &fp);
        assert_eq!(reloaded["panes"]["p1"]["locked_title"], "LockedTitle");
        // clearing removes locked_title
        set_title_lock(&env, "p1", false, None).unwrap();
        let reloaded = load_association_map(&env, &fp);
        assert_eq!(reloaded["panes"]["p1"]["title_lock"], false);
        assert!(reloaded["panes"]["p1"].get("locked_title").is_none());
    }

    #[test]
    fn update_association_map_prunes_missing() {
        let tmp = tempfile::tempdir().unwrap();
        // TestEnv.read_file falls back to the fs, so the same env drives both
        // the atomic write and the reload — no process globals, no data race.
        let env = TestEnv::new().with("XDG_STATE_HOME", tmp.path().to_str().unwrap());
        let snap1 = Snapshot {
            panes: vec![
                pane(json!({"pane_id": "p1", "tab_id": "t1", "workspace_id": "w1"})),
                pane(json!({"pane_id": "p2", "tab_id": "t1", "workspace_id": "w1"})),
            ],
            tabs: vec![],
            workspaces: vec![],
            layouts: Value::Null,
        };
        let out1 = update_association_map(&env, &snap1, Some("cws"), None).unwrap();
        assert_eq!(out1["pane_count"], 2);
        // Second pass: p2 gone → pruned.
        let snap2 = Snapshot {
            panes: vec![pane(
                json!({"pane_id": "p1", "tab_id": "t1", "workspace_id": "w1"}),
            )],
            tabs: vec![],
            workspaces: vec![],
            layouts: Value::Null,
        };
        let out2 = update_association_map(&env, &snap2, None, None).unwrap();
        assert_eq!(out2["pane_count"], 1);
        assert_eq!(out2["pruned"], json!(["p2"]));
        // cmux_workspace carried forward.
        assert_eq!(out2["cmux_workspace"], "cws");
    }
}
