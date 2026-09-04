//! Golden and state-machine parity for the intertwined runtime modules.

#[path = "../src/socket.rs"]
mod socket;
#[path = "../src/api.rs"]
mod api;
#[path = "../src/model.rs"]
mod model;
#[path = "../src/layout.rs"]
mod layout;
#[path = "../src/impose.rs"]
mod impose;
#[path = "../src/host.rs"]
mod host;
#[path = "../src/lifecycle.rs"]
mod lifecycle;
#[path = "../src/handoff.rs"]
mod handoff;
#[path = "../src/state.rs"]
mod state;
#[path = "../src/engine.rs"]
mod engine;
#[path = "../src/session.rs"]
mod session;
#[path = "../src/control.rs"]
mod control;
#[path = "../src/io.rs"]
mod io;
#[path = "../src/live.rs"]
mod live;
#[path = "../src/pump.rs"]
mod pump;

use std::collections::HashMap;

use serde_json::{json, Value};

fn golden() -> Value {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/runtime_golden.json"
    ))
    .expect("read runtime_golden.json");
    serde_json::from_str(&text).expect("parse runtime golden")
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap().to_string())
        .collect()
}

fn option_strings(value: &Value) -> Option<Vec<String>> {
    value.as_array().map(|_| strings(value))
}

fn hex_bytes(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |byte: u8| match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => panic!("invalid hex"),
            };
            digit(pair[0]) * 16 + digit(pair[1])
        })
        .collect()
}

fn window_from_json(raw: &Value) -> engine::HerdrWindow {
    engine::HerdrWindow::new(
        raw["tab_id"].as_str().unwrap(),
        raw["title"].as_str().unwrap(),
        raw["order_index"].as_i64().unwrap(),
        layout::parse_layout(&raw["layout"]).unwrap(),
        None,
        raw["zoomed"].as_bool().unwrap_or(false),
        raw["active_pane_id"].as_str().map(str::to_string),
    )
}

fn reconcile_json(result: &engine::ReconcileResult) -> Value {
    json!({
        "created_pane_ids": result.created_pane_ids,
        "closed_pane_ids": result.closed_pane_ids,
        "kept_pane_ids": result.kept_pane_ids,
        "structure_changed": result.structure_changed,
        "title_changed": result.title_changed,
        "focus_pane_id": result.focus_pane_id,
        "split_specs": result.split_specs.iter().map(|spec| json!({
            "pane_id": spec.pane_id,
            "split_from_pane_id": spec.split_from_pane_id,
            "direction": spec.direction,
            "ratio": spec.ratio,
        })).collect::<Vec<_>>(),
        "rendered_ids": result.rendered_layout.pane_ids_in_order(),
    })
}

fn session_reconcile(raw: &Value) -> engine::SessionReconcile {
    engine::SessionReconcile {
        created_tab_ids: strings(&raw["created_tab_ids"]),
        closed_tab_ids: strings(&raw["closed_tab_ids"]),
        kept_tab_ids: strings(&raw["kept_tab_ids"]),
        ordered_tab_ids: strings(&raw["ordered_tab_ids"]),
        order_changed: raw["order_changed"].as_bool().unwrap(),
    }
}

fn action_json(action: &session::SessionAction) -> Value {
    json!({
        "op": action.op,
        "tab_id": action.tab_id,
        "title": action.title,
        "ordered_tab_ids": action.ordered_tab_ids,
    })
}

#[test]
fn engine_and_session_match_python_golden() {
    let g = golden();
    let apply = &g["engine"]["apply_first"];
    let raw_window = json!({
        "layout": apply["layout"],
        "tab_id": apply["window"]["tab_id"],
        "title": apply["window"]["title"],
        "order_index": apply["window"]["order_index"],
        "active_pane_id": apply["window"]["active_pane_id"],
        "zoomed": apply["window"]["zoomed"],
    });
    let (_, result) = engine::apply_window(&window_from_json(&raw_window), None);
    assert_eq!(reconcile_json(&result), apply["result"]);

    let session_case = &g["engine"]["session"];
    let windows: Vec<_> = session_case["windows"]
        .as_array()
        .unwrap()
        .iter()
        .map(window_from_json)
        .collect();
    let reconciled = engine::reconcile_session(&windows, &strings(&session_case["previous"]));
    assert_eq!(
        json!({
            "created_tab_ids": reconciled.created_tab_ids,
            "closed_tab_ids": reconciled.closed_tab_ids,
            "kept_tab_ids": reconciled.kept_tab_ids,
            "ordered_tab_ids": reconciled.ordered_tab_ids,
            "order_changed": reconciled.order_changed,
        }),
        session_case["result"]
    );

    for case in g["engine"]["client_grid"].as_array().unwrap() {
        let a = case["args"].as_array().unwrap();
        assert_eq!(
            json!(engine::client_grid(
                a[0].as_f64().unwrap(), a[1].as_f64().unwrap(),
                a[2].as_f64().unwrap(), a[3].as_f64().unwrap(),
                a[4].as_f64().unwrap(), a[5].as_f64().unwrap(),
            )),
            case["out"]
        );
    }
    for case in g["engine"]["resize"].as_array().unwrap() {
        let a = case["args"].as_array().unwrap();
        assert_eq!(
            json!(engine::resize_cells(a[0].as_f64().unwrap(), a[1].as_f64().unwrap(), a[2].as_i64().unwrap())),
            case["out"]
        );
    }
    for case in g["engine"]["delta"].as_array().unwrap() {
        assert_eq!(
            json!(engine::output_delta(case["previous"].as_str(), case["current"].as_str().unwrap())),
            case["out"]
        );
    }

    for case in g["session"]["actions"].as_array().unwrap() {
        let titles: HashMap<String, String> = serde_json::from_value(case["titles"].clone()).unwrap();
        let previous: HashMap<String, String> = serde_json::from_value(case["previous_titles"].clone()).unwrap();
        let actions = session::session_actions(
            &session_reconcile(&case["session"]),
            Some(&titles),
            Some(&previous),
            case["defaults_open"].as_bool().unwrap(),
            case["focus"].as_str(),
        );
        assert_eq!(Value::Array(actions.iter().map(action_json).collect()), case["out"]);
    }
}

#[test]
fn control_io_and_pump_helpers_match_python_golden() {
    let g = golden();
    for case in g["control"]["keys"].as_array().unwrap() {
        let got = control::encode_named_key("w2:p1", case["name"].as_str().unwrap()).map(|item| {
            json!({
                "pane_id": item.pane_id,
                "kind": item.kind,
                "text": item.text,
                "key": item.key,
                "csi_hex": item.csi.map(|bytes| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()),
            })
        });
        assert_eq!(json!(got), case["out"], "key {}", case["name"]);
    }
    for case in g["control"]["titles"].as_array().unwrap() {
        assert_eq!(
            json!(control::apply_session_title(
                case["name"].as_str().unwrap(),
                case["current"].as_str(),
                case["propagate"].as_bool().unwrap(),
            )),
            case["out"]
        );
    }
    for case in g["control"]["close"].as_array().unwrap() {
        let got = control::close_intent(
            case["source"].as_str().unwrap(),
            case["pane_id"].as_str(),
            case["status"].as_str(),
        );
        assert_eq!(json!({"action": got.action, "pane_id": got.pane_id}), case["out"]);
    }
    for case in g["control"]["adjacent"].as_array().unwrap() {
        let node = layout::parse_layout(&case["layout"]).unwrap();
        assert_eq!(
            json!(control::adjacent_pane(&node, case["pane_id"].as_str().unwrap(), case["direction"].as_str().unwrap())),
            case["out"]
        );
    }

    let _live_filter_export = live::TitleEscapeFilter::new();
    for case in g["io"]["filters"].as_array().unwrap() {
        let mut filter = io::TitleEscapeFilter::new();
        let got: Vec<String> = case["chunks_hex"]
            .as_array().unwrap().iter()
            .map(|chunk| filter.filter(&hex_bytes(chunk.as_str().unwrap())))
            .map(|bytes| bytes.iter().map(|b| format!("{b:02x}")).collect())
            .collect();
        assert_eq!(json!(got), case["out_hex"]);
    }

    for case in g["pump"]["events"].as_array().unwrap() {
        let input = if case["input"].is_null() { None } else { Some(&case["input"]) };
        assert_eq!(pump::unwrap_event(input), case["unwrapped"]);
        assert_eq!(pump::event_type(input), case["type"].as_str().unwrap());
        assert_eq!(pump::classify_event(input), case["kind"].as_str().unwrap());
    }
    for case in g["pump"]["followup"].as_array().unwrap() {
        let result = case["result"].as_object().map(|raw| pump::PumpResult {
            kind: raw.get("kind").and_then(Value::as_str).unwrap().to_string(),
            resync: raw.get("resync").and_then(Value::as_bool).unwrap_or(false),
            routed_output: raw.get("routed_output").and_then(Value::as_bool).unwrap_or(false),
            status_updated: raw.get("status_updated").and_then(Value::as_bool).unwrap_or(false),
            ..pump::PumpResult::default()
        });
        assert_eq!(
            pump::watch_followup(result.as_ref(), case["had_event"].as_bool().unwrap(), case["event_gap"].as_bool().unwrap()),
            case["out"].as_str().unwrap()
        );
    }

    for case in g["live_helpers"]["endpoint_hash"].as_array().unwrap() {
        assert_eq!(live::endpoint_hash(case["socket"].as_str().unwrap()), case["out"].as_str().unwrap());
    }
    for case in g["live_helpers"]["decode_beta"].as_array().unwrap() {
        assert_eq!(live::decode_beta(Some(&case["value"]), case["default"].as_bool().unwrap()), case["out"].as_bool().unwrap());
    }
    for case in g["live_helpers"]["grid_match"].as_array().unwrap() {
        let a = case["args"].as_array().unwrap();
        assert_eq!(live::grid_match(a[0].as_i64().unwrap(), a[1].as_i64().unwrap(), a[2].as_i64().unwrap(), a[3].as_i64().unwrap(), a[4].as_bool().unwrap(), a[5].as_bool().unwrap()), case["out"].as_bool().unwrap());
    }
}

#[test]
fn stateful_runtime_preserves_order_bounds_focus_and_close_safety() {
    let layout = layout::parse_layout(&json!({
        "width": 200, "height": 50, "x": 0, "y": 0,
        "horizontal": [
            {"width": 100, "height": 50, "x": 0, "y": 0, "pane": "w2:p1"},
            {"width": 99, "height": 50, "x": 101, "y": 0, "pane": "w2:p2"}
        ]
    })).unwrap();
    let window = engine::HerdrWindow::new("w2:t1", "Build", 0, layout, None, false, Some("w2:p1".into()));
    let mut host = live::apply_live_windows(&[window], None, true).unwrap();
    let mirror = host.window_mut("w2:t1").unwrap();
    assert!(mirror.bonsplit.log.iter().position(|line| line == "create:w2:p1").unwrap()
        < mirror.bonsplit.log.iter().position(|line| line == "rebuild_tree").unwrap());
    assert!(mirror.route_output("w2:p1", b"ab\x1bkTitle\x1b\\cd"));
    assert_eq!(mirror.surface("w2:p1").unwrap().buffer, b"abcd");
    assert!(mirror.surface("w2:p2").unwrap().buffer.is_empty());

    assert_eq!(mirror.send_named_key("w2:p1", "C-Up"), "enqueued");
    assert_eq!(mirror.send_text("w2:p2", "ls\n"), "enqueued");
    assert_eq!(mirror.input.pending_bytes, 9);
    assert_eq!(mirror.navigate_focus("right"), Some("w2:p2".into()));
    assert!(mirror.surface("w2:p2").unwrap().first_responder);
    mirror.apply_provider_focus("w2:p1");
    assert!(mirror.surface("w2:p2").unwrap().first_responder);

    let detached = host.detach();
    assert_eq!(detached["outcome"], "detach");
    assert_eq!(detached["server_stopped"], false);
    assert!(host.windows.is_empty());
}

#[test]
fn pump_routes_events_and_flushes_input_without_cross_pane_writes() {
    let layout = layout::parse_layout(&json!({
        "width": 200, "height": 50, "horizontal": [
            {"width": 100, "height": 50, "pane": "w2:p1"},
            {"width": 99, "height": 50, "x": 101, "pane": "w2:p2"}
        ]
    })).unwrap();
    let window = engine::HerdrWindow::new("w2:t1", "Build", 0, layout, None, false, Some("w2:p1".into()));
    let mut host = live::apply_live_windows(&[window], None, true).unwrap();
    host.window_mut("w2:t1").unwrap().send_named_key("w2:p1", "Up");
    let transport = pump::MemoryTransport::new(
        HashMap::from([("w2:p1".to_string(), "alpha".to_string()), ("w2:p2".to_string(), "bravo".to_string())]),
        HashMap::new(),
    );
    let mut live_pump = pump::LivePump::new(transport);
    let event = json!({"type": "pane.updated", "pane_id": "w2:p1"});
    let result = live_pump.handle_event(Some(&event), Some(&mut host));
    assert!(result.routed_output);
    assert_eq!(host.window("w2:t1").unwrap().surface("w2:p1").unwrap().buffer, b"alpha");
    assert!(host.window("w2:t1").unwrap().surface("w2:p2").unwrap().buffer.is_empty());
    assert_eq!(live_pump.flush_input(&mut host), 1);
    assert_eq!(live_pump.transport.sent[0], ("key".into(), "w2:p1".into(), "up".into()));
}
