//! Golden parity: Rust model parsers + status-pill builders must match the
//! Python bridge byte-for-byte across a captured battery.
//!
//! Golden committed at `tests/status_golden.json`; regenerate from the Python
//! bridge if the model/status layer changes (both sides move together per the
//! clean-cutover rule).
use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::{json, Value};

// The binary crate's lib target is not exposed; re-declare the pure modules.
// `status` depends on `model`, so both are rooted at the test-crate level and
// `crate::model` resolves.
#[path = "../src/model.rs"]
mod model;
#[path = "../src/status.rs"]
mod status;

fn golden() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/status_golden.json");
    let raw = std::fs::read_to_string(&path).expect("read golden");
    serde_json::from_str(&raw).expect("parse golden")
}

/// Reproduce the Python golden generator's `mk_pane`: only known keys land in
/// the raw object, and `tab_id` defaults to `t1`.
fn mk_pane(spec: &Value) -> model::Pane {
    let mut raw = json!({
        "pane_id": spec.get("pane_id").and_then(Value::as_str).unwrap_or("p"),
        "tab_id": spec.get("tab_id").and_then(Value::as_str).unwrap_or("t1"),
    });
    for k in ["agent", "agent_status", "label", "terminal_title"] {
        if let Some(v) = spec.get(k) {
            raw[k] = v.clone();
        }
    }
    model::pane_from_raw(&raw)
}

fn tabs_by_id() -> HashMap<String, model::Tab> {
    let mut m = HashMap::new();
    m.insert(
        "t1".into(),
        model::tab_from_raw(&json!({"tab_id": "t1", "workspace_id": "w1", "label": "TabLabel"})),
    );
    m.insert(
        "t2".into(),
        model::tab_from_raw(&json!({"tab_id": "t2", "workspace_id": "w1"})),
    );
    m
}

#[test]
fn pane_from_raw_matches_python() {
    let g = golden();
    for entry in g["panes"].as_array().unwrap() {
        let raw = &entry["raw"];
        let want = &entry["pane"];
        let p = model::pane_from_raw(raw);
        assert_eq!(json!(p.pane_id), want["pane_id"], "pane_id for {raw}");
        assert_eq!(json!(p.tab_id), want["tab_id"], "tab_id for {raw}");
        assert_eq!(json!(p.workspace_id), want["workspace_id"], "workspace_id for {raw}");
        assert_eq!(json!(p.agent), want["agent"], "agent for {raw}");
        assert_eq!(json!(p.agent_status), want["agent_status"], "agent_status for {raw}");
        assert_eq!(json!(p.label), want["label"], "label for {raw}");
        assert_eq!(json!(p.cwd), want["cwd"], "cwd for {raw}");
        assert_eq!(json!(p.focused), want["focused"], "focused for {raw}");
        assert_eq!(json!(p.terminal_title), want["terminal_title"], "terminal_title for {raw}");
        assert_eq!(json!(p.agent_session_path), want["agent_session_path"], "session_path for {raw}");
        assert_eq!(json!(p.agent_session_id), want["agent_session_id"], "session_id for {raw}");
        assert_eq!(json!(p.agent_session_kind), want["agent_session_kind"], "session_kind for {raw}");
        assert_eq!(json!(p.revision), want["revision"], "revision for {raw}");
        assert_eq!(json!(p.display_name()), want["display_name"], "display_name for {raw}");
        assert_eq!(json!(p.status_key()), want["status_key"], "status_key for {raw}");
        assert_eq!(json!(p.has_agent()), want["has_agent"], "has_agent for {raw}");
    }
}

#[test]
fn map_status_to_style_matches_python() {
    let g = golden();
    for (key, want) in g["styles"].as_object().unwrap() {
        // Key is a JSON-encoded status (a string or null).
        let status: Value = serde_json::from_str(key).unwrap();
        let style = status::map_status_to_style(status.as_str());
        assert_eq!(json!(style.icon), want[0], "icon for {key}");
        assert_eq!(json!(style.color), want[1], "color for {key}");
        assert_eq!(json!(style.priority), want[2], "priority for {key}");
    }
}

#[test]
fn status_value_for_pane_matches_python() {
    let g = golden();
    let tabs = tabs_by_id();
    for entry in g["values"].as_array().unwrap() {
        let case = &entry["case"];
        let p = mk_pane(&case["pane"]);
        let use_tabs = !case["tabs"].is_null();
        let locked = case["locked"].as_str();
        let parent = case["parent"].as_str();
        let v = status::status_value_for_pane(
            &p,
            if use_tabs { Some(&tabs) } else { None },
            locked,
            parent,
        );
        assert_eq!(json!(v), entry["value"], "value for {case}");
    }
}

#[test]
fn status_write_payload_matches_python() {
    let g = golden();
    let tabs = tabs_by_id();
    for entry in g["payloads"].as_array().unwrap() {
        let prior = &entry["prior"];
        let prior_ref = if prior.is_null() { None } else { Some(prior) };
        let p = mk_pane(&json!({
            "agent": "bot", "agent_status": "blocked", "terminal_title": "realtitle"
        }));
        let payload = status::status_write_payload(&p, Some(&tabs), prior_ref);
        assert_eq!(payload, entry["payload"], "payload for prior {prior}");
    }
}

#[test]
fn should_write_status_pill_matches_python() {
    let g = golden();
    for entry in g["should_write"].as_array().unwrap() {
        let payload = &entry["payload"];
        let prior = &entry["prior"];
        let prior_ref = if prior.is_null() { None } else { Some(prior) };
        let got = status::should_write_status_pill(payload, prior_ref);
        assert_eq!(json!(got), entry["result"], "should_write for {entry}");
    }
}

#[test]
fn locked_display_name_matches_python() {
    let g = golden();
    for entry in g["locked"].as_array().unwrap() {
        let prior = &entry["prior"];
        let prior_ref = if prior.is_null() { None } else { Some(prior) };
        let got = status::locked_display_name(prior_ref);
        assert_eq!(json!(got), entry["result"], "locked for {prior}");
    }
}
