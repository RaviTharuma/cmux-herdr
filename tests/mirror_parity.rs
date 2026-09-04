#![allow(dead_code, unused_imports)]

//! Golden parity: Rust mirror projection and reconcile planning must match the
//! Python bridge across scope, layout, malformed, ordering, and truthiness cases.
use std::collections::HashSet;
use std::path::PathBuf;

use serde_json::{json, Value};

#[path = "../src/api.rs"]
mod api;
#[path = "../src/bridge.rs"]
mod bridge;
#[path = "../src/control.rs"]
mod control;
#[path = "../src/engine.rs"]
mod engine;
#[path = "../src/handoff.rs"]
mod handoff;
#[path = "../src/host.rs"]
mod host;
#[path = "../src/impose.rs"]
mod impose;
#[path = "../src/io.rs"]
mod io;
#[path = "../src/layout.rs"]
mod layout;
#[path = "../src/lifecycle.rs"]
mod lifecycle;
#[path = "../src/live.rs"]
mod live;
#[path = "../src/mirror.rs"]
mod mirror;
#[path = "../src/model.rs"]
mod model;
#[path = "../src/pump.rs"]
mod pump;
#[path = "../src/session.rs"]
mod session;
#[path = "../src/socket.rs"]
mod socket;
#[path = "../src/state.rs"]
mod state;
#[path = "../src/status.rs"]
mod status;

use mirror::{DesiredMirror, MirrorAction, MirrorPlan};

fn golden() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/mirror_golden.json");
    let raw = std::fs::read_to_string(path).expect("read mirror golden");
    serde_json::from_str(&raw).expect("parse mirror golden")
}

fn desired_from(value: &Value) -> DesiredMirror {
    DesiredMirror {
        pane_id: value["pane_id"].as_str().unwrap().to_string(),
        tab_id: value["tab_id"].as_str().unwrap().to_string(),
        workspace_id: value["workspace_id"].as_str().unwrap().to_string(),
        title: value["title"].as_str().unwrap().to_string(),
        role: value["role"].as_str().unwrap().to_string(),
        split_direction: value["split_direction"].as_str().unwrap().to_string(),
        agent: value["agent"].as_str().map(str::to_string),
        agent_status: value["agent_status"].as_str().unwrap().to_string(),
        split_ratio: value["split_ratio"].as_f64(),
        split_from_pane_id: value["split_from_pane_id"].as_str().map(str::to_string),
        tab_number: (!value["tab_number"].is_null()).then(|| value["tab_number"].clone()),
        tab_index: value["tab_index"].as_i64(),
        focused: value["focused"].as_bool().unwrap(),
        zoomed: value["zoomed"].as_bool().unwrap(),
        visible: value["visible"].as_bool().unwrap(),
    }
}

fn exact_floats(value: Value) -> Value {
    match value {
        Value::Number(number) if number.is_f64() => {
            json!({"__f64_bits": number.as_f64().unwrap().to_bits()})
        }
        Value::Array(items) => Value::Array(items.into_iter().map(exact_floats).collect()),
        Value::Object(items) => Value::Object(
            items
                .into_iter()
                .map(|(key, value)| (key, exact_floats(value)))
                .collect(),
        ),
        other => other,
    }
}

fn optional_string(value: &Option<String>) -> Value {
    value.as_ref().map_or(Value::Null, |value| json!(value))
}

fn optional_float(value: Option<f64>) -> Value {
    value.map_or(Value::Null, |value| json!(value))
}

fn optional_int(value: Option<i64>) -> Value {
    value.map_or(Value::Null, |value| json!(value))
}

fn desired_json(item: &DesiredMirror) -> Value {
    json!({
        "pane_id": item.pane_id,
        "tab_id": item.tab_id,
        "workspace_id": item.workspace_id,
        "title": item.title,
        "role": item.role,
        "split_direction": item.split_direction,
        "agent": optional_string(&item.agent),
        "agent_status": item.agent_status,
        "split_ratio": optional_float(item.split_ratio),
        "split_from_pane_id": optional_string(&item.split_from_pane_id),
        "tab_number": item.tab_number.clone().unwrap_or(Value::Null),
        "tab_index": optional_int(item.tab_index),
        "focused": item.focused,
        "zoomed": item.zoomed,
        "visible": item.visible,
        "key": item.key(),
    })
}

fn action_json(action: &MirrorAction) -> Value {
    json!({
        "op": action.op,
        "pane_id": action.pane_id,
        "title": action.title,
        "tab_id": action.tab_id,
        "role": action.role,
        "split_direction": action.split_direction,
        "key": action.key,
        "surface_id": optional_string(&action.surface_id),
        "split_from_surface_id": optional_string(&action.split_from_surface_id),
        "split_from_pane_id": optional_string(&action.split_from_pane_id),
        "ratio": optional_float(action.ratio),
        "tab_index": optional_int(action.tab_index),
        "reason": action.reason,
    })
}

fn pane_ids(actions: Vec<&MirrorAction>) -> Value {
    Value::Array(
        actions
            .into_iter()
            .map(|action| json!(action.pane_id))
            .collect(),
    )
}

fn plan_json(plan: &MirrorPlan) -> Value {
    json!({
        "actions": plan.actions.iter().map(action_json).collect::<Vec<_>>(),
        "scope": plan.scope,
        "desired_count": plan.desired_count,
        "creates": pane_ids(plan.creates()),
        "renames": pane_ids(plan.renames()),
        "prunes": pane_ids(plan.prunes()),
        "keeps": pane_ids(plan.keeps()),
        "ratio_updates": pane_ids(plan.ratio_updates()),
        "moves": pane_ids(plan.moves()),
        "focuses": pane_ids(plan.focuses()),
    })
}

#[test]
fn mirror_matches_python_golden() {
    let golden = golden();
    let cases = golden["cases"].as_array().unwrap();
    assert_eq!(golden["case_count"].as_u64().unwrap() as usize, cases.len());
    assert!(cases.len() >= 20, "golden battery must remain non-trivial");

    for case in cases {
        let inputs = &case["inputs"];
        let got = match case["op"].as_str().unwrap() {
            "desired_mirrors" => {
                let snapshot = model::snapshot_from_session_payload(&inputs["snapshot"])
                    .expect("golden snapshot parses");
                let previous_tab = (case["name"] == "missing-current-tab-errors")
                    .then(|| std::env::var_os("HERDR_TAB_ID"))
                    .flatten();
                if case["name"] == "missing-current-tab-errors" {
                    std::env::remove_var("HERDR_TAB_ID");
                }
                let result = mirror::desired_mirrors(
                    &snapshot,
                    inputs["scope"].as_str().unwrap(),
                    inputs["current_tab_id"].as_str(),
                    inputs["current_workspace_id"].as_str(),
                    inputs["use_layout"].as_bool().unwrap(),
                );
                if case["name"] == "missing-current-tab-errors" {
                    if let Some(value) = previous_tab {
                        std::env::set_var("HERDR_TAB_ID", value);
                    }
                }
                match result {
                    Ok(items) => json!({"ok": items.iter().map(desired_json).collect::<Vec<_>>() }),
                    Err(error) => json!({"error": error.to_string()}),
                }
            }
            "plan_mirror" => {
                let desired: Vec<DesiredMirror> = inputs["desired"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(desired_from)
                    .collect();
                let live = inputs["live_surface_ids"].as_array().map(|values| {
                    values
                        .iter()
                        .map(|value| value.as_str().unwrap().to_string())
                        .collect::<HashSet<_>>()
                });
                plan_json(&mirror::plan_mirror(
                    &desired,
                    &inputs["existing"],
                    live.as_ref(),
                    inputs["prune"].as_bool().unwrap(),
                    inputs["sync_focus"].as_bool().unwrap(),
                    inputs["sync_order"].as_bool().unwrap(),
                    inputs["sync_ratios"].as_bool().unwrap(),
                    (!inputs["engine"].is_null()).then_some(&inputs["engine"]),
                ))
            }
            "parse_cmux_json" => {
                match mirror::parse_cmux_json(inputs["stdout"].as_str().unwrap()) {
                    Ok(value) => json!({"ok": value}),
                    Err(_) => json!({"error": true}),
                }
            }
            "mirror_key_for_pane" => json!(mirror::mirror_key_for_pane(
                inputs["pane_id"].as_str().unwrap(),
            )),
            "format_mirror_plan" => json!(mirror::format_mirror_plan(&inputs["result"])),
            op => panic!("unknown mirror golden op: {op}"),
        };
        assert_eq!(
            exact_floats(got),
            case["output"],
            "mirror parity case {}",
            case["name"].as_str().unwrap(),
        );
    }
}
