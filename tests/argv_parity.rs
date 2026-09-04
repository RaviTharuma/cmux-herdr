#![allow(dead_code, unused_imports)]

//! Golden parity: Rust `build_cli_argv` must match the Python bridge output
//! byte-for-byte across a battery captured from `cmux_herdr_api.build_cli_argv`.
//!
//! The golden is committed at `tests/py_argv_golden.json`; regenerate it from
//! the Python bridge if the CLI mapping ever changes (both sides move together
//! per the clean-cutover rule).

use std::path::PathBuf;

use serde_json::Value;

// The binary crate is named `cmux-herdr`; its lib target is not exposed, so we
// re-declare the two pure modules we need. They have no cross-module deps for
// `build_cli_argv`.
#[path = "../src/api.rs"]
mod api;
#[path = "../src/socket.rs"]
mod socket;

#[test]
fn argv_matches_python_golden() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/py_argv_golden.json");
    let raw = std::fs::read_to_string(&path).expect("read golden");
    let cases: Vec<Value> = serde_json::from_str(&raw).expect("parse golden");
    assert!(!cases.is_empty(), "golden battery is empty");

    let mut failures = Vec::new();
    for case in &cases {
        let method = case["method"].as_str().unwrap();
        let params = &case["params"];
        let expected: Option<Vec<String>> = match &case["argv"] {
            Value::Null => None,
            Value::Array(a) => Some(a.iter().map(|v| v.as_str().unwrap().to_string()).collect()),
            other => panic!("bad golden argv: {other}"),
        };
        let got = api::build_cli_argv(method, params);
        if got != expected {
            failures.push(format!(
                "{method} {params}\n    expected {expected:?}\n    got      {got:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "argv parity mismatches ({}):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
