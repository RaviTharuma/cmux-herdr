//! Status-pill styling and payload construction.
//!
//! Ported from `bridge/cmux_herdr_bridge.py`: `STATUS_STYLE`, `DEFAULT_STYLE`,
//! `map_status_to_style`, `status_value_for_pane`, `locked_display_name`,
//! `status_write_payload`, and `should_write_status_pill`. Values are exact so
//! cmux pills render identically.

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::model::{Pane, Tab};

/// One pill style: SF Symbol icon, hex color, priority (`STATUS_STYLE` values).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub icon: &'static str,
    pub color: &'static str,
    pub priority: i64,
}

/// Fallback style for an unmapped status (`DEFAULT_STYLE`).
pub const DEFAULT_STYLE: Style = Style {
    icon: "circle",
    color: "#8e8e93",
    priority: 10,
};

/// Return `(icon, color, priority)` for a Herdr `agent_status`
/// (`map_status_to_style`). Case-insensitive; trims; unknown → [`DEFAULT_STYLE`].
///
/// The map is exact (`STATUS_STYLE`):
/// working `hammer/#ff9500/80`, idle `pause.circle/#8e8e93/40`,
/// done `checkmark.circle/#34c759/30`, blocked
/// `exclamationmark.triangle/#ff3b30/90`, unknown `questionmark.circle/#8e8e93/10`.
pub fn map_status_to_style(status: Option<&str>) -> Style {
    let key = status.filter(|s| !s.is_empty()).unwrap_or("unknown").to_lowercase();
    let key = key.trim();
    match key {
        "working" => Style {
            icon: "hammer",
            color: "#ff9500",
            priority: 80,
        },
        "idle" => Style {
            icon: "pause.circle",
            color: "#8e8e93",
            priority: 40,
        },
        "done" => Style {
            icon: "checkmark.circle",
            color: "#34c759",
            priority: 30,
        },
        "blocked" => Style {
            icon: "exclamationmark.triangle",
            color: "#ff3b30",
            priority: 90,
        },
        "unknown" => Style {
            icon: "questionmark.circle",
            color: "#8e8e93",
            priority: 10,
        },
        _ => DEFAULT_STYLE,
    }
}

/// Build a compact, human-readable cmux status-pill value
/// (`status_value_for_pane`).
///
/// `locked_title` (native-title lock) replaces `pane.display_name` when set.
/// `parent_tab_id` prefers the persisted parent map over a flickering snapshot
/// `tab_id`.
pub fn status_value_for_pane(
    pane: &Pane,
    tabs_by_id: Option<&HashMap<String, Tab>>,
    locked_title: Option<&str>,
    parent_tab_id: Option<&str>,
) -> String {
    let agent = pane.agent.as_deref().filter(|a| !a.is_empty()).unwrap_or("agent");
    let status = pane.agent_status.to_lowercase();
    let mut parts = vec![format!("{agent}/{status}")];

    let tab_id = parent_tab_id
        .filter(|t| !t.is_empty())
        .unwrap_or(pane.tab_id.as_str());
    if let Some(tab) = tabs_by_id.and_then(|m| m.get(tab_id)) {
        if let Some(label) = tab.label.as_deref().filter(|l| !l.is_empty()) {
            parts.push(label.to_string());
        }
    }

    let owned_name;
    let name: &str = match locked_title.filter(|t| !t.is_empty()) {
        Some(t) => t,
        None => {
            owned_name = pane.display_name();
            &owned_name
        }
    };
    if !name.is_empty() && !parts.iter().any(|p| p == name) {
        parts.push(name.to_string());
    }
    parts.join(" \u{b7} ") // " · "
}

/// Return the locked title when the association record has a title lock
/// (`locked_display_name`).
pub fn locked_display_name(prior: Option<&Value>) -> Option<String> {
    let prior = prior?.as_object()?;
    if !truthy(prior.get("title_lock")) {
        return None;
    }
    match prior.get("locked_title") {
        Some(Value::String(s)) if !s.trim().is_empty() => Some(s.trim().to_string()),
        _ => None,
    }
}

/// Build the set-status payload, honoring title lock and parent map
/// (`status_write_payload`).
pub fn status_write_payload(
    pane: &Pane,
    tabs_by_id: Option<&HashMap<String, Tab>>,
    prior: Option<&Value>,
) -> Value {
    let empty = json!({});
    let prior = prior.filter(|p| p.is_object()).unwrap_or(&empty);
    let locked = locked_display_name(Some(prior));

    // parent_tab only when heuristic_satisfied and it is a non-blank string.
    let parent_tab = if truthy(prior.get("heuristic_satisfied")) {
        match prior.get("parent_tab_id") {
            Some(Value::String(s)) if !s.trim().is_empty() => Some(s.clone()),
            _ => None,
        }
    } else {
        None
    };

    let style = map_status_to_style(Some(&pane.agent_status));
    let value = status_value_for_pane(
        pane,
        tabs_by_id,
        locked.as_deref(),
        parent_tab.as_deref(),
    );

    json!({
        "value": value,
        "icon": style.icon,
        "color": style.color,
        "priority": style.priority,
        "title_lock": truthy(prior.get("title_lock")),
        "locked_title": locked,
    })
}

/// Return `false` when the last written pill is identical (diff-before-write)
/// (`should_write_status_pill`).
pub fn should_write_status_pill(payload: &Value, prior: Option<&Value>) -> bool {
    let Some(prior) = prior.and_then(Value::as_object) else {
        return true;
    };
    let eq = |a: &str, b: &str| prior.get(a) == payload.get(b);
    !(eq("last_status_value", "value")
        && eq("last_icon", "icon")
        && eq("last_color", "color")
        && eq("last_priority", "priority"))
}

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
    use crate::model::pane_from_raw;
    use serde_json::json;

    fn pane(status: &str, agent: Option<&str>, title: Option<&str>) -> Pane {
        let mut raw = json!({"pane_id": "p1", "tab_id": "t1", "agent_status": status});
        if let Some(a) = agent {
            raw["agent"] = json!(a);
        }
        if let Some(t) = title {
            raw["terminal_title"] = json!(t);
        }
        pane_from_raw(&raw)
    }

    #[test]
    fn style_map_exact_and_case_insensitive() {
        assert_eq!(map_status_to_style(Some("working")).icon, "hammer");
        assert_eq!(map_status_to_style(Some("WORKING")).priority, 80);
        assert_eq!(map_status_to_style(Some("Done")).icon, "checkmark.circle");
        assert_eq!(map_status_to_style(Some("blocked")).color, "#ff3b30");
        assert_eq!(map_status_to_style(None).icon, "questionmark.circle");
        assert_eq!(map_status_to_style(Some("weird")), DEFAULT_STYLE);
    }

    #[test]
    fn value_joins_agent_status_and_name() {
        let p = pane("working", Some("bot"), Some("mytitle"));
        let v = status_value_for_pane(&p, None, None, None);
        assert_eq!(v, "bot/working \u{b7} mytitle");
    }

    #[test]
    fn value_dedups_name_against_parts() {
        // display_name == agent/status impossible, but name matching a tab
        // label should not duplicate.
        let mut tabs = HashMap::new();
        tabs.insert(
            "t1".to_string(),
            crate::model::tab_from_raw(&json!({"tab_id": "t1", "label": "same"})),
        );
        let p = pane("idle", Some("a"), Some("same"));
        let v = status_value_for_pane(&p, Some(&tabs), None, None);
        // "a/idle · same" — the name equals the tab label so it is added once.
        assert_eq!(v, "a/idle \u{b7} same");
    }

    #[test]
    fn locked_title_replaces_display_name() {
        let p = pane("working", Some("bot"), Some("realtitle"));
        let v = status_value_for_pane(&p, None, Some("LOCKED"), None);
        assert_eq!(v, "bot/working \u{b7} LOCKED");
    }

    #[test]
    fn write_payload_carries_style_and_lock() {
        let p = pane("blocked", Some("bot"), None);
        let prior = json!({"title_lock": true, "locked_title": "L"});
        let payload = status_write_payload(&p, None, Some(&prior));
        assert_eq!(payload["icon"], "exclamationmark.triangle");
        assert_eq!(payload["priority"], 90);
        assert_eq!(payload["title_lock"], true);
        assert_eq!(payload["locked_title"], "L");
        assert_eq!(payload["value"], "bot/blocked \u{b7} L");
    }

    #[test]
    fn diff_before_write() {
        let payload = json!({"value": "v", "icon": "i", "color": "c", "priority": 1});
        assert!(should_write_status_pill(&payload, None));
        let same = json!({
            "last_status_value": "v", "last_icon": "i",
            "last_color": "c", "last_priority": 1
        });
        assert!(!should_write_status_pill(&payload, Some(&same)));
        let diff = json!({
            "last_status_value": "v2", "last_icon": "i",
            "last_color": "c", "last_priority": 1
        });
        assert!(should_write_status_pill(&payload, Some(&diff)));
    }
}
