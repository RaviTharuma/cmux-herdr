use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

fn write_fake_herdr(path: &Path) {
    let script = r#"#!/bin/sh
printf '%s\n' "$*" >> "$FAKE_HERDR_LOG"
case "$*" in
  "--version") printf 'herdr 0.8.0\n' ;;
  "tab create --label logs") printf '%s\n' '{"result":{"label":"logs","tab_id":"t2"}}' ;;
  "pane send-text p1 hello world") printf '%s\n' '{"result":{"sent":true}}' ;;
  "pane wait-output p1 --match ready --timeout 725") printf '%s\n' '{"result":{"matched":true}}' ;;
  "status") printf '%s\n' '{"status":"ok"}' ;;
  "pane list") printf '%s\n' '{"result":{"panes":[{"pane_id":"p1","tab_id":"t1","workspace_id":"w1","agent":"pi","agent_status":"working","label":"Bot"}]}}' ;;
  "tab list") printf '%s\n' '{"result":{"tabs":[]}}' ;;
  "workspace list") printf '%s\n' '{"result":{"workspaces":[]}}' ;;
  "api snapshot") printf '%s\n' '{"result":{"layouts":[]}}' ;;
  *) printf 'unexpected command: %s\n' "$*" >&2; exit 9 ;;
esac
"#;
    fs::write(path, script).expect("write fake herdr");
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn write_fake_cmux(path: &Path) {
    let script = r#"#!/bin/sh
printf '%s\n' "$*" >> "$FAKE_CMUX_LOG"
if [ "$1" = "list-status" ]; then
  printf '%s\n' 'herdr:p1=current' 'herdr:stale=old' 'unrelated=keep'
fi
"#;
    fs::write(path, script).expect("write fake cmux");
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

#[test]
fn representative_dispatch_matches_python_goldens() {
    let fixture: Value = serde_json::from_str(include_str!("cli_golden.json")).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let herdr = temp.path().join("herdr");
    let cmux = temp.path().join("cmux");
    let log = temp.path().join("herdr.log");
    let cmux_log = temp.path().join("cmux.log");
    write_fake_herdr(&herdr);
    write_fake_cmux(&cmux);
    let inherited_path = std::env::var("PATH").unwrap_or_default();
    let path = format!("{}:{inherited_path}", temp.path().display());

    for case in fixture["cases"].as_array().unwrap() {
        fs::write(&log, "").unwrap();
        fs::write(&cmux_log, "").unwrap();
        let argv: Vec<&str> = case["argv"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item.as_str().unwrap())
            .collect();
        let output = Command::new(env!("CARGO_BIN_EXE_cmux-herdr"))
            .args(argv)
            .env("PATH", &path)
            .env("HERDR_SOCKET_PATH", temp.path().join("missing.sock"))
            .env("FAKE_HERDR_LOG", &log)
            .env("FAKE_CMUX_LOG", &cmux_log)
            .env("HOME", temp.path())
            .env("XDG_STATE_HOME", temp.path().join("state"))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), case["status"].as_i64().map(|v| v as i32), "{} status; stdout={:?}; stderr={:?}", case["name"], String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout), case["stdout"].as_str().unwrap(), "{} stdout", case["name"]);
        assert_eq!(String::from_utf8_lossy(&output.stderr), case["stderr"].as_str().unwrap(), "{} stderr", case["name"]);
        assert_eq!(fs::read_to_string(&log).unwrap().trim_end(), case["herdr_argv"].as_str().unwrap(), "{} herdr argv", case["name"]);
        assert_eq!(fs::read_to_string(&cmux_log).unwrap().trim_end(), case.get("cmux_argv").and_then(Value::as_str).unwrap_or(""), "{} cmux argv", case["name"]);
    }
}

fn base_command(temp: &tempfile::TempDir) -> Command {
    let inherited_path = std::env::var("PATH").unwrap_or_default();
    let mut command = Command::new(env!("CARGO_BIN_EXE_cmux-herdr"));
    command
        .env("PATH", format!("{}:{inherited_path}", temp.path().display()))
        .env("HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("HERDR_ENV", "1")
        .env("HERDR_SOCKET_PATH", temp.path().join("missing.sock"))
        .env("HERDR_WORKSPACE_ID", "w1")
        .env("CMUX_SURFACE_ID", "surface-cli")
        .env("FAKE_HERDR_LOG", temp.path().join("herdr.log"))
        .env("FAKE_CMUX_LOG", temp.path().join("cmux.log"));
    command
}

#[test]
fn live_lifecycle_never_stops_herdr() {
    let temp=tempfile::tempdir().unwrap();
    write_fake_herdr(&temp.path().join("herdr"));
    write_fake_cmux(&temp.path().join("cmux"));
    let attached=base_command(&temp).args(["attach","--json"]).output().unwrap();
    assert!(attached.status.success(),"{}",String::from_utf8_lossy(&attached.stderr));
    let attach:Value=serde_json::from_slice(&attached.stdout).unwrap();
    assert_eq!(attach["ok"],Value::Bool(true));
    assert_eq!(attach["server_stopped"],Value::Bool(false));
    assert!(attach["restore_path"].as_str().is_some());

    let observed=base_command(&temp).args(["observe","--method","pane_surfaces","--json"]).output().unwrap();
    assert!(observed.status.success(),"{}",String::from_utf8_lossy(&observed.stderr));
    let observe:Value=serde_json::from_slice(&observed.stdout).unwrap();
    assert_eq!(observe["ok"],Value::Bool(true));

    let detached=base_command(&temp).args(["detach","--json"]).output().unwrap();
    assert!(detached.status.success(),"{}",String::from_utf8_lossy(&detached.stderr));
    let detach:Value=serde_json::from_slice(&detached.stdout).unwrap();
    assert_eq!(detach["server_stopped"],Value::Bool(false));
}

#[test]
fn update_service_status_reports_discovered_paths() {
    let temp=tempfile::tempdir().unwrap();
    let herdr=temp.path().join("herdr");
    write_fake_herdr(&herdr);
    let output=base_command(&temp).args(["update-service","status","--herdr",herdr.to_str().unwrap(),"--json"]).output().unwrap();
    assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
    let payload:Value=serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["action"],"status");
    assert_eq!(payload["installed"],Value::Bool(false));
    assert_eq!(payload["runtime_exists"],Value::Bool(false));
}
