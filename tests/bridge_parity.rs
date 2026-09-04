//! Golden parity for bridge topology payload parsing against Python.
//!
//! `bridge` depends on `crate::api`, `crate::model`, and api depends on socket,
//! so include all modules at the test-crate root.

#[path = "../src/socket.rs"]
mod socket;
#[path = "../src/api.rs"]
mod api;
#[path = "../src/model.rs"]
mod model;
#[path = "../src/bridge.rs"]
mod bridge;

use serde_json::Value;

fn golden() -> Value {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/bridge_golden.json"
    ))
    .expect("read bridge_golden.json");
    serde_json::from_str(&text).expect("parse golden")
}

/// Serialize Pane model into the same shape as Python dataclasses.asdict.
fn pane_json(p: &model::Pane) -> Value {
    serde_json::json!({
        "pane_id": p.pane_id,
        "tab_id": p.tab_id,
        "workspace_id": p.workspace_id,
        "agent": p.agent,
        "agent_status": p.agent_status,
        "label": p.label,
        "cwd": p.cwd,
        "focused": p.focused,
        "terminal_title": p.terminal_title,
        "agent_session_path": p.agent_session_path,
        "agent_session_id": p.agent_session_id,
        "agent_session_kind": p.agent_session_kind,
        "revision": p.revision,
        "raw": p.raw,
    })
}

fn tab_json(t: &model::Tab) -> Value {
    serde_json::json!({
        "tab_id": t.tab_id,
        "workspace_id": t.workspace_id,
        "label": t.label,
        "number": t.number,
        "agent_status": t.agent_status,
        "focused": t.focused,
        "pane_count": t.pane_count,
        "raw": t.raw,
    })
}

fn workspace_json(w: &model::Workspace) -> Value {
    serde_json::json!({
        "workspace_id": w.workspace_id,
        "label": w.label,
        "number": w.number,
        "agent_status": w.agent_status,
        "focused": w.focused,
        "pane_count": w.pane_count,
        "tab_count": w.tab_count,
        "raw": w.raw,
    })
}

#[test]
fn panes_from_list_matches_python() {
    for case in golden()["panes"].as_array().unwrap() {
        let got: Vec<Value> = bridge::panes_from_list(&case["data"])
            .iter()
            .map(pane_json)
            .collect();
        assert_eq!(Value::Array(got), case["out"], "panes data={:?}", case["data"]);
    }
}

#[test]
fn tabs_from_list_matches_python() {
    for case in golden()["tabs"].as_array().unwrap() {
        let got: Vec<Value> = bridge::tabs_from_list(&case["data"])
            .iter()
            .map(tab_json)
            .collect();
        assert_eq!(Value::Array(got), case["out"], "tabs data={:?}", case["data"]);
    }
}

#[test]
fn workspaces_from_list_matches_python() {
    for case in golden()["workspaces"].as_array().unwrap() {
        let got: Vec<Value> = bridge::workspaces_from_list(&case["data"])
            .iter()
            .map(workspace_json)
            .collect();
        assert_eq!(
            Value::Array(got),
            case["out"],
            "workspaces data={:?}",
            case["data"]
        );
    }
}

#[test]
fn agents_from_list_matches_python() {
    for case in golden()["agents"].as_array().unwrap() {
        let got = match bridge::agents_from_list(&case["data"]) {
            Some(items) => Value::Array(items.iter().map(pane_json).collect()),
            None => Value::Null,
        };
        assert_eq!(got, case["out"], "agents data={:?}", case["data"]);
    }
}

#[test]
fn parse_json_payload_matches_python() {
    for case in golden()["parse"].as_array().unwrap() {
        let got = bridge::parse_json_payload(case["stdout"].as_str().unwrap()).unwrap();
        assert_eq!(got, case["out"], "parse stdout={:?}", case["stdout"]);
    }
}
