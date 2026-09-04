#![allow(dead_code, unused_imports)]

//! Golden parity: Rust writer-lease pure behavior must match Python.
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use serde_json::{json, Value};

#[path = "../src/handoff.rs"]
mod handoff;

fn golden() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/handoff_golden.json");
    let raw = std::fs::read_to_string(&path).expect("read golden");
    serde_json::from_str(&raw).expect("parse golden")
}

fn set_env(name: &str, value: &Value) {
    match value.as_str() {
        Some(value) => std::env::set_var(name, value),
        None => std::env::remove_var(name),
    }
}

fn lease_json(lease: Option<handoff::WriterLease>) -> Value {
    match lease {
        None => Value::Null,
        Some(lease) => {
            let mut body = lease.to_dict().as_object().unwrap().clone();
            body.insert("path".into(), json!(lease.path));
            Value::Object(body)
        }
    }
}

#[test]
fn pure_behavior_matches_python() {
    let _guard = handoff::HANDOFF_ENV_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let g = golden();

    for entry in g["env_truthy"].as_array().unwrap() {
        set_env("HANDOFF_GOLDEN_FLAG", &entry["value"]);
        assert_eq!(
            json!(handoff::env_truthy("HANDOFF_GOLDEN_FLAG")),
            entry["result"]
        );
    }

    for entry in g["lease_ttl_ms"].as_array().unwrap() {
        set_env(handoff::LEASE_TTL_ENV, &entry["value"]);
        assert_eq!(
            json!(handoff::lease_ttl_ms()),
            entry["result"],
            "TTL for {:?}",
            entry["value"]
        );
    }
    std::env::remove_var(handoff::LEASE_TTL_ENV);

    let fixture = Path::new("/tmp/cmux-herdr-handoff-golden/fixture/native-live-fp");
    std::fs::create_dir_all(fixture.parent().unwrap()).unwrap();
    let file = std::fs::File::create(fixture).unwrap();
    file.set_times(
        std::fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
    )
    .unwrap();
    drop(file);
    for entry in g["parse_lease_text"].as_array().unwrap() {
        let got = handoff::parse_lease_text(
            entry["text"].as_str().unwrap(),
            fixture,
            entry["fallback_owner"].as_str(),
            entry["fallback_fingerprint"].as_str().unwrap(),
        );
        assert_eq!(
            lease_json(got),
            entry["result"],
            "parse case {}",
            entry["name"]
        );
    }

    for entry in g["freshness"].as_array().unwrap() {
        let lease = handoff::WriterLease {
            owner: handoff::OWNER_PLUGIN.into(),
            pid: 0,
            heartbeat_ms: entry["heartbeat_ms"].as_i64().unwrap(),
            fingerprint: "fp".into(),
            endpoint_hash: String::new(),
            socket_path: String::new(),
            schema: handoff::SCHEMA,
            path: String::new(),
        };
        assert_eq!(
            json!(lease.is_fresh(
                Some(entry["now"].as_i64().unwrap()),
                Some(entry["ttl"].as_i64().unwrap())
            )),
            entry["result"]
        );
    }

    std::env::set_var("XDG_STATE_HOME", "/tmp/cmux-herdr-handoff-golden/xdg");
    std::env::set_var(
        handoff::NATIVE_STATE_ENV,
        "/tmp/cmux-herdr-handoff-golden/native",
    );
    std::env::set_var("HOME", "/tmp/cmux-herdr-handoff-golden/home");
    let paths = json!({
        "xdg_state_dir": handoff::xdg_state_dir(),
        "application_support_dir": handoff::application_support_dir(),
        "state_dirs": handoff::state_dirs(),
        "writer_plugin": handoff::writer_paths("host-a", handoff::OWNER_PLUGIN).unwrap(),
        "writer_native": handoff::writer_paths("host-a", handoff::OWNER_NATIVE).unwrap(),
        "legacy_native_marker_path": handoff::legacy_native_marker_path("host-a"),
        "plugin_marker_path": handoff::plugin_marker_path("host-a"),
        "restore_paths": handoff::restore_paths("deadbeef"),
    });
    assert_eq!(paths, g["paths"]);

    for entry in g["now_ms_formula"].as_array().unwrap() {
        let got = handoff::milliseconds_from_seconds(entry["seconds"].as_f64().unwrap());
        assert_eq!(json!(got), entry["result"]);
    }
}
