//! Shared Herdr topology models and raw-payload parsers.
//!
//! Ported from the model layer of `bridge/cmux_herdr_bridge.py`
//! (`Pane`, `Tab`, `Workspace`, `Snapshot`, `_pane_from_raw`, and the
//! session-snapshot parser). Field names and coercions match Python exactly so
//! status pills and the tree renderer produce byte-identical output.

use serde_json::Value;

/// Prefix for cmux status keys (`STATUS_PREFIX`).
pub const STATUS_PREFIX: &str = "herdr:";

/// One Herdr pane (`Pane`).
#[derive(Debug, Clone, PartialEq)]
pub struct Pane {
    pub pane_id: String,
    pub tab_id: String,
    pub workspace_id: String,
    pub agent: Option<String>,
    pub agent_status: String,
    pub label: Option<String>,
    pub cwd: Option<String>,
    pub focused: bool,
    pub terminal_title: Option<String>,
    pub agent_session_path: Option<String>,
    pub agent_session_id: Option<String>,
    pub agent_session_kind: Option<String>,
    pub revision: Option<i64>,
    pub raw: Value,
}

impl Pane {
    /// Display name for status pills (`display_name`). Label wins; else the
    /// stripped terminal title truncated to 40 chars with an ellipsis; else
    /// `agent@pane_id`; else the bare `pane_id`.
    pub fn display_name(&self) -> String {
        if let Some(label) = self.label.as_deref().filter(|l| !l.is_empty()) {
            return label.to_string();
        }
        let title = self.terminal_title.as_deref().unwrap_or("").trim();
        if !title.is_empty() {
            // Python slices by code points, then appends the ellipsis when the
            // *original* length exceeds 40.
            let chars: Vec<char> = title.chars().collect();
            let head: String = chars.iter().take(40).collect();
            return if chars.len() > 40 {
                format!("{head}\u{2026}")
            } else {
                head
            };
        }
        if let Some(agent) = self.agent.as_deref() {
            return format!("{agent}@{}", self.pane_id);
        }
        self.pane_id.clone()
    }

    /// Compact cmux status key (`status_key`): `herdr:<pane_id>`.
    pub fn status_key(&self) -> String {
        format!("{STATUS_PREFIX}{}", self.pane_id)
    }

    /// True when this pane carries an agent (`has_agent`): a non-empty agent
    /// name, or an `agent_status` that is not `""`/`unknown`.
    pub fn has_agent(&self) -> bool {
        self.agent
            .as_deref()
            .map(|a| !a.is_empty())
            .unwrap_or(false)
            || !matches!(self.agent_status.as_str(), "" | "unknown")
    }
}

/// One Herdr tab (`Tab`).
#[derive(Debug, Clone, PartialEq)]
pub struct Tab {
    pub tab_id: String,
    pub workspace_id: String,
    pub label: Option<String>,
    pub number: Option<i64>,
    pub agent_status: String,
    pub focused: bool,
    pub pane_count: i64,
    pub raw: Value,
}

/// One Herdr workspace (`Workspace`).
#[derive(Debug, Clone, PartialEq)]
pub struct Workspace {
    pub workspace_id: String,
    pub label: Option<String>,
    pub number: Option<i64>,
    pub agent_status: String,
    pub focused: bool,
    pub pane_count: i64,
    pub tab_count: i64,
    pub raw: Value,
}

/// A point-in-time Herdr topology (`Snapshot`).
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub panes: Vec<Pane>,
    pub tabs: Vec<Tab>,
    pub workspaces: Vec<Workspace>,
    pub layouts: Value,
}

impl Snapshot {
    /// Panes that carry an agent (`agent_panes`).
    pub fn agent_panes(&self) -> Vec<&Pane> {
        self.panes.iter().filter(|p| p.has_agent()).collect()
    }
}

/// `str(raw.get(k) or "")` — coerce a JSON field to a string, empty on
/// null/missing/empty; numbers/bools render like Python `str`.
fn str_field(raw: &Value, key: &str) -> String {
    match raw.get(key) {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => {
            if *b {
                "True".into()
            } else {
                "False".into()
            }
        }
        Some(other) => other.to_string(),
    }
}

/// `str(raw.get(k) or "unknown")`.
fn str_field_or_unknown(raw: &Value, key: &str) -> String {
    let v = str_field(raw, key);
    if v.is_empty() {
        "unknown".into()
    } else {
        v
    }
}

/// `raw.get(k)` as an owned string only when it is a JSON string (Python keeps
/// `label`/`cwd` as-is, which are strings or None).
fn opt_str(raw: &Value, key: &str) -> Option<String> {
    match raw.get(key) {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

/// `bool(raw.get(k))` — Python truthiness.
fn bool_field(raw: &Value, key: &str) -> bool {
    match raw.get(key) {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::Bool(true)) => true,
        Some(Value::Number(n)) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
    }
}

/// `int(raw.get(k) or 0)` — coerce integers; floats truncate; non-numbers → 0.
fn int_field(raw: &Value, key: &str) -> i64 {
    match raw.get(key) {
        Some(Value::Number(n)) => n
            .as_i64()
            .or_else(|| n.as_f64().map(|f| f as i64))
            .unwrap_or(0),
        Some(Value::String(s)) => s.trim().parse::<i64>().unwrap_or(0),
        _ => 0,
    }
}

/// `raw.get(k)` as an integer only when it is a JSON integer (`number`/`revision`
/// stay `Optional[int]`; a non-int value yields None).
fn opt_int(raw: &Value, key: &str) -> Option<i64> {
    match raw.get(key) {
        Some(Value::Number(n)) if n.is_i64() || n.is_u64() => n.as_i64(),
        _ => None,
    }
}

/// Build a [`Pane`] from a raw Herdr pane object (`_pane_from_raw`).
pub fn pane_from_raw(raw: &Value) -> Pane {
    let session = raw.get("agent_session").and_then(Value::as_object);
    let mut session_path = None;
    let mut session_id = None;
    let mut session_kind = None;
    if let Some(session) = session {
        let kind = session.get("kind");
        let value = session.get("value");
        session_kind = match kind {
            Some(Value::Null) | None => None,
            Some(Value::String(k)) if k.is_empty() => None,
            Some(Value::String(k)) => Some(k.clone()),
            // Python `str(kind) if kind` — truthy non-strings stringify.
            Some(other) if truthy(Some(other)) => Some(other.to_string()),
            _ => None,
        };
        let kind_str = kind.and_then(Value::as_str);
        let value_str = value.and_then(Value::as_str);
        match (kind_str, value_str) {
            (Some("path"), Some(v)) => session_path = Some(v.to_string()),
            (Some("id"), Some(v)) => session_id = Some(v.to_string()),
            (_, Some(v)) if v.ends_with(".jsonl") => session_path = Some(v.to_string()),
            (_, Some(v)) => session_id = Some(v.to_string()),
            _ => {}
        }
    }

    // Prefer top-level agent; fall back to agent_session.agent. Strip → None.
    let agent = raw
        .get("agent")
        .and_then(Value::as_str)
        .or_else(|| session.and_then(|s| s.get("agent")).and_then(Value::as_str))
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .map(str::to_string);

    let cwd = opt_str(raw, "cwd").or_else(|| opt_str(raw, "foreground_cwd"));
    let terminal_title =
        opt_str(raw, "terminal_title_stripped").or_else(|| opt_str(raw, "terminal_title"));
    let revision = opt_int(raw, "revision");

    Pane {
        pane_id: str_field(raw, "pane_id"),
        tab_id: str_field(raw, "tab_id"),
        workspace_id: str_field(raw, "workspace_id"),
        agent,
        agent_status: str_field_or_unknown(raw, "agent_status"),
        label: opt_str(raw, "label"),
        cwd,
        focused: bool_field(raw, "focused"),
        terminal_title,
        agent_session_path: session_path,
        agent_session_id: session_id,
        agent_session_kind: session_kind,
        revision,
        raw: raw.clone(),
    }
}

/// Build a [`Tab`] from a raw Herdr tab object.
pub fn tab_from_raw(raw: &Value) -> Tab {
    Tab {
        tab_id: str_field(raw, "tab_id"),
        workspace_id: str_field(raw, "workspace_id"),
        label: opt_str(raw, "label"),
        number: opt_int(raw, "number"),
        agent_status: str_field_or_unknown(raw, "agent_status"),
        focused: bool_field(raw, "focused"),
        pane_count: int_field(raw, "pane_count"),
        raw: raw.clone(),
    }
}

/// Build a [`Workspace`] from a raw Herdr workspace object.
pub fn workspace_from_raw(raw: &Value) -> Workspace {
    Workspace {
        workspace_id: str_field(raw, "workspace_id"),
        label: opt_str(raw, "label"),
        number: opt_int(raw, "number"),
        agent_status: str_field_or_unknown(raw, "agent_status"),
        focused: bool_field(raw, "focused"),
        pane_count: int_field(raw, "pane_count"),
        tab_count: int_field(raw, "tab_count"),
        raw: raw.clone(),
    }
}

/// Parse a `session.snapshot` result into a [`Snapshot`], or `None`
/// (`_snapshot_from_session_payload`).
pub fn snapshot_from_session_payload(result: &Value) -> Option<Snapshot> {
    let mut result = result.as_object()?;
    // Unwrap a nested {"snapshot": {...}}.
    if let Some(inner) = result.get("snapshot").and_then(Value::as_object) {
        result = inner;
    }
    let panes_raw = result.get("panes");
    // Python requires panes to be a list (falsy default `[]` then isinstance).
    let panes_raw = match panes_raw {
        Some(Value::Array(a)) => a.as_slice(),
        Some(Value::Null) | None => &[],
        Some(_) => return None,
    };
    let panes = panes_raw
        .iter()
        .filter(|p| p.is_object() && truthy(p.get("pane_id")))
        .map(pane_from_raw)
        .collect();

    let tabs = match result.get("tabs") {
        Some(Value::Array(a)) => a
            .iter()
            .filter(|t| t.is_object())
            .map(tab_from_raw)
            .collect(),
        _ => Vec::new(),
    };

    let ws_raw = result
        .get("workspaces")
        .filter(|v| !v.is_null())
        .or_else(|| result.get("workspace_list"));
    let workspaces = match ws_raw {
        Some(Value::Array(a)) => a
            .iter()
            .filter(|w| w.is_object())
            .map(workspace_from_raw)
            .collect(),
        _ => Vec::new(),
    };

    // `result.get("layouts") or result` — falsy layouts falls back to the whole
    // result object.
    let layouts = match result.get("layouts") {
        Some(v) if truthy(Some(v)) => v.clone(),
        _ => Value::Object(result.clone()),
    };

    Some(Snapshot {
        panes,
        tabs,
        workspaces,
        layouts,
    })
}

/// Python truthiness for an optional JSON value.
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pane_from_raw_prefers_top_level_agent_and_session_path() {
        let raw = json!({
            "pane_id": "p1",
            "tab_id": "t1",
            "workspace_id": "w1",
            "agent": " bot ",
            "agent_status": "working",
            "agent_session": {"kind": "path", "value": "/x/s.jsonl", "agent": "nested"},
            "terminal_title": "hello",
            "revision": 7
        });
        let p = pane_from_raw(&raw);
        assert_eq!(p.agent.as_deref(), Some("bot"));
        assert_eq!(p.agent_session_path.as_deref(), Some("/x/s.jsonl"));
        assert_eq!(p.agent_session_kind.as_deref(), Some("path"));
        assert_eq!(p.revision, Some(7));
        assert!(p.has_agent());
    }

    #[test]
    fn pane_session_jsonl_value_without_kind_is_path() {
        let raw = json!({
            "pane_id": "p1",
            "agent_session": {"value": "/logs/a.jsonl"}
        });
        let p = pane_from_raw(&raw);
        assert_eq!(p.agent_session_path.as_deref(), Some("/logs/a.jsonl"));
        assert_eq!(p.agent_session_id, None);
        // No agent, default status: has_agent false.
        assert_eq!(p.agent_status, "unknown");
        assert!(!p.has_agent());
    }

    #[test]
    fn display_name_truncates_at_40_codepoints() {
        let long = "x".repeat(45);
        let raw = json!({ "pane_id": "p1", "terminal_title": long });
        let p = pane_from_raw(&raw);
        let name = p.display_name();
        assert_eq!(name.chars().count(), 41); // 40 + ellipsis
        assert!(name.ends_with('\u{2026}'));
    }

    #[test]
    fn snapshot_parses_and_unwraps_nested() {
        let result = json!({
            "snapshot": {
                "panes": [{"pane_id": "p1", "agent_status": "idle"}],
                "tabs": [{"tab_id": "t1", "label": "L"}],
                "workspaces": [{"workspace_id": "w1"}]
            }
        });
        let snap = snapshot_from_session_payload(&result).unwrap();
        assert_eq!(snap.panes.len(), 1);
        assert_eq!(snap.tabs.len(), 1);
        assert_eq!(snap.workspaces.len(), 1);
        assert_eq!(snap.panes[0].pane_id, "p1");
    }

    #[test]
    fn snapshot_non_list_panes_is_none() {
        assert!(snapshot_from_session_payload(&json!({"panes": "nope"})).is_none());
    }

    #[test]
    fn workspace_list_fallback_key() {
        let result = json!({
            "panes": [],
            "workspace_list": [{"workspace_id": "w9"}]
        });
        let snap = snapshot_from_session_payload(&result).unwrap();
        assert_eq!(snap.workspaces.len(), 1);
        assert_eq!(snap.workspaces[0].workspace_id, "w9");
    }
}
