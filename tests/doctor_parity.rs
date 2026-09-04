use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

fn write_fake_herdr(path: &Path) {
    let script = r#"#!/bin/sh
printf '%s\n' "$*" >> "$FAKE_HERDR_LOG"
case "$*" in
  "--version") printf 'herdr 0.8.0\n' ;;
  "status") exit 7 ;;
  "pane list") printf '%s\n' '{"result":{"panes":[{"pane_id":"p1","tab_id":"t1","workspace_id":"w1","agent":"pi","agent_status":"working"}]}}' ;;
  "tab list") printf '%s\n' '{"result":{"tabs":[{"tab_id":"t1","workspace_id":"w1"}]}}' ;;
  "workspace list") printf '%s\n' '{"result":{"workspaces":[{"workspace_id":"w1"}]}}' ;;
  "api snapshot"|"pane layout --current") exit 9 ;;
  *) printf 'unexpected command: %s\n' "$*" >&2; exit 9 ;;
esac
"#;
    fs::write(path, script).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn isolated_doctor(temp: &tempfile::TempDir) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cmux-herdr"));
    command
        .arg("doctor")
        .arg("--json")
        .env_clear()
        .env("HOME", temp.path().join("home"))
        .env("XDG_STATE_HOME", temp.path().join("state"));
    command
}

fn report(output: &Output) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let start = stdout
        .find("{\n  \"ok\"")
        .unwrap_or_else(|| panic!("doctor JSON report missing from stdout: {stdout}"));
    serde_json::from_str(&stdout[start..]).unwrap()
}

fn checks_by_name(report: &Value) -> std::collections::HashMap<&str, &Value> {
    report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|check| (check["name"].as_str().unwrap(), check))
        .collect()
}

#[test]
fn doctor_restores_full_legacy_advisory_surface_without_mutating_state() {
    let temp = tempfile::tempdir().unwrap();
    let bin = temp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    write_fake_herdr(&bin.join("herdr"));
    let log = temp.path().join("herdr.log");
    let socket = temp.path().join("herdr.sock");
    fs::write(&socket, "not a live socket").unwrap();
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
    let state = temp.path().join("state/cmux-herdr");

    let output = isolated_doctor(&temp)
        .env("PATH", &bin)
        .env("FAKE_HERDR_LOG", &log)
        .env("HERDR_ENV", "1")
        .env("HERDR_SOCKET_PATH", &socket)
        .env("HERDR_WORKSPACE_ID", "w1")
        .env("CMUX_SURFACE_ID", "surface-doctor")
        .env("CMUX_WORKSPACE_ID", "workspace:9")
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report = report(&output);
    assert_eq!(report["ok"], Value::Bool(true));
    assert_eq!(report["hard_failures"], Value::Array(vec![]));
    let names: Vec<_> = report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|check| check["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "herdr_cli",
            "herdr_socket",
            "herdr_api",
            "host_fingerprint",
            "state_binding",
            "writer",
            "size_authority",
            "title_locks",
            "launch_agent",
            "sidebar",
            "dry_sync",
        ]
    );

    let checks = checks_by_name(&report);
    assert_eq!(checks["herdr_socket"]["ok"], Value::Bool(true));
    assert_eq!(checks["herdr_socket"]["hard"], Value::Bool(false));
    assert_eq!(checks["herdr_socket"]["socket"]["mode"], "0o600");
    assert_eq!(checks["herdr_api"]["ok"], Value::Bool(false));
    assert_eq!(checks["herdr_api"]["hard"], Value::Bool(false));
    assert!(checks["herdr_api"]["detail"]
        .as_str()
        .unwrap()
        .starts_with("ping failed:"));
    assert_eq!(checks["state_binding"]["hard"], Value::Bool(false));
    assert_eq!(checks["writer"]["hard"], Value::Bool(false));
    assert_eq!(checks["size_authority"]["hard"], Value::Bool(false));
    assert_eq!(checks["title_locks"]["locked_count"], 0);
    assert_eq!(checks["launch_agent"]["hard"], Value::Bool(false));
    assert_eq!(checks["sidebar"]["ok"], Value::Bool(true));
    assert_eq!(checks["sidebar"]["exists"], Value::Bool(false));
    assert_eq!(
        checks["dry_sync"]["dry_sync"]["skipped"],
        Value::Bool(false)
    );
    assert_eq!(checks["dry_sync"]["dry_sync"]["agent_count"], 1);
    assert_eq!(checks["dry_sync"]["dry_sync"]["workspace"], "workspace:9");
    assert!(!state.exists(), "doctor must not create or update state");

    let calls = fs::read_to_string(log).unwrap();
    assert!(calls.lines().any(|line| line == "--version"));
    assert!(calls.lines().any(|line| line == "pane list"));
    assert!(
        !calls.lines().any(|line| line == "status"),
        "socket-only API ping must not fall back to `herdr status`: {calls}"
    );
}

#[test]
fn doctor_hard_fails_only_for_missing_cli_or_nested_incomplete_fingerprint() {
    let temp = tempfile::tempdir().unwrap();
    let empty_bin = temp.path().join("empty-bin");
    fs::create_dir(&empty_bin).unwrap();
    let missing = isolated_doctor(&temp)
        .env("PATH", &empty_bin)
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
    let missing_report = report(&missing);
    assert_eq!(
        missing_report["hard_failures"],
        serde_json::json!(["herdr not found on PATH"])
    );
    let missing_checks = checks_by_name(&missing_report);
    assert_eq!(missing_checks["host_fingerprint"]["ok"], Value::Bool(true));
    assert_eq!(
        missing_checks["host_fingerprint"]["hard"],
        Value::Bool(false)
    );

    let bin = temp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    write_fake_herdr(&bin.join("herdr"));
    let incomplete = isolated_doctor(&temp)
        .env("PATH", &bin)
        .env("FAKE_HERDR_LOG", temp.path().join("herdr.log"))
        .env("HERDR_ENV", "1")
        .output()
        .unwrap();
    assert_eq!(incomplete.status.code(), Some(1));
    let incomplete_report = report(&incomplete);
    let failures = incomplete_report["hard_failures"].as_array().unwrap();
    assert_eq!(failures.len(), 1, "{incomplete_report:#}");
    assert!(failures[0]
        .as_str()
        .unwrap()
        .starts_with("incomplete host fingerprint while HERDR_ENV claims nested env"));
    let incomplete_checks = checks_by_name(&incomplete_report);
    assert_eq!(incomplete_checks["herdr_api"]["hard"], Value::Bool(false));
    assert_eq!(
        incomplete_checks["host_fingerprint"]["hard"],
        Value::Bool(true)
    );
    let stdout = String::from_utf8_lossy(&incomplete.stdout);
    assert!(stdout.contains("\nhard failures:\n  - incomplete host fingerprint"));
}
