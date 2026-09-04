#![allow(dead_code, unused_imports)]

//! Golden parity for the state-store module against Python.
//!
//! `state` depends on `crate::model`, so both source files are included at the
//! test-crate root (same idiom as `status_parity.rs`).

#[path = "../src/model.rs"]
mod model;
#[path = "../src/state.rs"]
mod state;

use model::{pane_from_raw, Snapshot};
use serde_json::{json, Value};
use state::{
    association_key_for_pane, collect_host_fingerprint, format_associations, parent_key,
    Fingerprint, HostEnv,
};

/// Golden clock value from the Python generator.
const NOW: f64 = 1_700_000_000.0;

/// Test env with pinned vars + clock; reads fall through to the real fs.
struct GoldenEnv {
    vars: std::collections::HashMap<String, String>,
}

impl GoldenEnv {
    fn new() -> Self {
        GoldenEnv {
            vars: std::collections::HashMap::new(),
        }
    }
    fn from(map: &Value) -> Self {
        let mut env = GoldenEnv::new();
        if let Some(obj) = map.as_object() {
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    env.vars.insert(k.clone(), s.to_string());
                }
            }
        }
        env
    }
}

impl HostEnv for GoldenEnv {
    fn var(&self, name: &str) -> Option<String> {
        self.vars.get(name).cloned()
    }
    fn now(&self) -> f64 {
        NOW
    }
    fn read_file(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }
}

fn golden() -> Value {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/state_golden.json"
    ))
    .expect("read state_golden.json");
    serde_json::from_str(&text).expect("parse golden")
}

fn fp_from(v: &Value) -> Fingerprint {
    let opt_str = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    Fingerprint {
        cmux_surface_id: opt_str("cmux_surface_id"),
        herdr_socket_path: opt_str("herdr_socket_path"),
        herdr_server_pid: v.get("herdr_server_pid").and_then(Value::as_i64),
        herdr_workspace_id: opt_str("herdr_workspace_id"),
    }
}

#[test]
fn parent_key_matches_python() {
    for case in golden()["parent_key"].as_array().unwrap() {
        let fp = fp_from(&case["fp"]);
        let want = case["key"].as_str().unwrap();
        assert_eq!(parent_key(&fp), want, "parent_key for {:?}", case["fp"]);
    }
}

#[test]
fn association_key_matches_python() {
    for case in golden()["association_key"].as_array().unwrap() {
        let pane = pane_from_raw(&case["raw"]);
        let want = case["key"].as_str().unwrap();
        assert_eq!(
            association_key_for_pane(&pane),
            want,
            "assoc key {:?}",
            case["raw"]
        );
    }
}

fn build_snapshot(spec: &Value) -> Option<Snapshot> {
    if spec.is_null() {
        return None;
    }
    let tabs = spec["tabs"]
        .as_array()
        .map(|arr| arr.iter().map(model::tab_from_raw).collect())
        .unwrap_or_default();
    Some(Snapshot {
        panes: vec![],
        tabs,
        workspaces: vec![],
        layouts: json!({}),
    })
}

#[test]
fn resolve_association_parents_matches_python() {
    for entry in golden()["resolve"].as_array().unwrap() {
        let case = &entry["case"];
        let env = GoldenEnv::from(&case["env"]);
        let pane = pane_from_raw(&case["raw"]);
        let prior = case["prior"].clone();
        let snapshot = build_snapshot(&case["snapshot"]);
        let got = state::resolve_association_parents(&env, &pane, &prior, snapshot.as_ref());
        assert_eq!(got, entry["result"], "resolve for {case:?}");
    }
}

#[test]
fn association_record_matches_python() {
    let env = GoldenEnv::new();
    for entry in golden()["record"].as_array().unwrap() {
        let case = &entry["case"];
        let pane = pane_from_raw(&case["raw"]);
        let prior = case["prior"].clone();
        let parents = case["parents"].clone();
        let meta = case["meta"].clone();
        let meta_ref = if meta.is_null() { None } else { Some(&meta) };
        let got = state::association_record(&env, &pane, &prior, &parents, meta_ref);
        assert_eq!(got, entry["record"], "record for {case:?}");
    }
}

#[test]
fn format_associations_matches_python() {
    for entry in golden()["format"].as_array().unwrap() {
        // Python generator ran with XDG_STATE_HOME=/tmp/goldenstate and no
        // fingerprint env, so the `file=` path is deterministic.
        let env = GoldenEnv::from(&json!({"XDG_STATE_HOME": "/tmp/goldenstate"}));
        let state_val = entry["state"].clone();
        let got = format_associations(&env, Some(&state_val));
        assert_eq!(got, entry["text"].as_str().unwrap(), "format text");
    }
}

#[test]
fn collect_fingerprint_reads_env() {
    let env = GoldenEnv::from(&json!({
        "CMUX_SURFACE_ID": "surf",
        "HERDR_SOCKET_PATH": "/tmp/h.sock",
        "HERDR_WORKSPACE_ID": "ws",
        "HERDR_SERVER_PID": "123",
    }));
    let fp = collect_host_fingerprint(&env);
    assert_eq!(fp.cmux_surface_id.as_deref(), Some("surf"));
    assert_eq!(fp.herdr_socket_path.as_deref(), Some("/tmp/h.sock"));
    assert_eq!(fp.herdr_workspace_id.as_deref(), Some("ws"));
    assert_eq!(fp.herdr_server_pid, Some(123));
}
