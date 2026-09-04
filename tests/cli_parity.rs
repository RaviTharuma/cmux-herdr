use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
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
        assert_eq!(
            output.status.code(),
            case["status"].as_i64().map(|v| v as i32),
            "{} status; stdout={:?}; stderr={:?}",
            case["name"],
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            case["stdout"].as_str().unwrap(),
            "{} stdout",
            case["name"]
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            case["stderr"].as_str().unwrap(),
            "{} stderr",
            case["name"]
        );
        assert_eq!(
            fs::read_to_string(&log).unwrap().trim_end(),
            case["herdr_argv"].as_str().unwrap(),
            "{} herdr argv",
            case["name"]
        );
        assert_eq!(
            fs::read_to_string(&cmux_log).unwrap().trim_end(),
            case.get("cmux_argv").and_then(Value::as_str).unwrap_or(""),
            "{} cmux argv",
            case["name"]
        );
    }
}

#[test]
fn empty_path_component_searches_current_directory() {
    let temp = tempfile::tempdir().unwrap();
    write_fake_herdr(&temp.path().join("herdr"));
    let log = temp.path().join("herdr.log");
    let output = Command::new(env!("CARGO_BIN_EXE_cmux-herdr"))
        .args(["tree", "--json"])
        .current_dir(temp.path())
        .env("PATH", ":/usr/bin:/bin")
        .env("HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("HERDR_SOCKET_PATH", temp.path().join("missing.sock"))
        .env("FAKE_HERDR_LOG", &log)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn sync_fails_when_association_state_cannot_persist() {
    let temp = tempfile::tempdir().unwrap();
    write_fake_herdr(&temp.path().join("herdr"));
    write_fake_cmux(&temp.path().join("cmux"));
    fs::write(temp.path().join("state-blocker"), "not a directory").unwrap();
    let inherited_path = std::env::var("PATH").unwrap_or_default();
    let output = Command::new(env!("CARGO_BIN_EXE_cmux-herdr"))
        .args(["sync", "--workspace", "ws1"])
        .env(
            "PATH",
            format!("{}:{inherited_path}", temp.path().display()),
        )
        .env("HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state-blocker"))
        .env("HERDR_ENV", "1")
        .env("HERDR_SOCKET_PATH", temp.path().join("missing.sock"))
        .env("HERDR_WORKSPACE_ID", "w1")
        .env("CMUX_SURFACE_ID", "surface-cli")
        .env("FAKE_HERDR_LOG", temp.path().join("herdr.log"))
        .env("FAKE_CMUX_LOG", temp.path().join("cmux.log"))
        .output()
        .unwrap();
    assert!(!output.status.success(), "sync unexpectedly succeeded");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Not a directory"),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn status_requires_both_herdr_and_cmux_context_to_be_nested() {
    let temp = tempfile::tempdir().unwrap();
    write_fake_herdr(&temp.path().join("herdr"));
    let inherited_path = std::env::var("PATH").unwrap_or_default();
    let output = Command::new(env!("CARGO_BIN_EXE_cmux-herdr"))
        .arg("status")
        .env(
            "PATH",
            format!("{}:{inherited_path}", temp.path().display()),
        )
        .env("HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("HERDR_ENV", "1")
        .env("HERDR_SOCKET_PATH", temp.path().join("missing.sock"))
        .env_remove("CMUX_SOCKET_PATH")
        .env_remove("CMUX_WORKSPACE_ID")
        .env("FAKE_HERDR_LOG", temp.path().join("herdr.log"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("nested context : no"),
        "stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("statuses     : {}"),
        "stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn focus_tab_accepts_a_unique_case_insensitive_label_prefix() {
    let temp = tempfile::tempdir().unwrap();
    let herdr = temp.path().join("herdr");
    let log = temp.path().join("herdr.log");
    fs::write(
        &herdr,
        r#"#!/bin/sh
printf '%s\n' "$*" >> "$FAKE_HERDR_LOG"
case "$*" in
  "--version") printf 'herdr 0.8.0\n' ;;
  "tab list") printf '%s\n' '{"result":{"tabs":[{"tab_id":"t1","label":"Build"},{"tab_id":"t2","label":"Docs"}]}}' ;;
  "tab focus t1") printf '%s\n' '{"result":{"ok":true}}' ;;
  "status") printf '%s\n' '{"status":"ok"}' ;;
  *) exit 9 ;;
esac
"#,
    )
    .unwrap();
    fs::set_permissions(&herdr, fs::Permissions::from_mode(0o755)).unwrap();
    let inherited_path = std::env::var("PATH").unwrap_or_default();
    let output = Command::new(env!("CARGO_BIN_EXE_cmux-herdr"))
        .args(["focus-tab", "bui"])
        .env(
            "PATH",
            format!("{}:{inherited_path}", temp.path().display()),
        )
        .env("HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("HERDR_ENV", "1")
        .env("HERDR_SOCKET_PATH", temp.path().join("missing.sock"))
        .env("FAKE_HERDR_LOG", &log)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(fs::read_to_string(log).unwrap().contains("tab focus t1\n"));
}
#[test]
fn read_pane_preserves_child_streams_and_exit_code() {
    let temp = tempfile::tempdir().unwrap();
    let herdr = temp.path().join("herdr");
    fs::write(
        &herdr,
        r#"#!/bin/sh
case "$*" in
  "--version") printf 'herdr 0.8.0\n' ;;
  "status") printf '%s\n' '{"status":"ok"}' ;;
  "pane read p1") printf 'partial output'; printf 'read warning\n' >&2; exit 2 ;;
  *) exit 9 ;;
esac
"#,
    )
    .unwrap();
    fs::set_permissions(&herdr, fs::Permissions::from_mode(0o755)).unwrap();
    let inherited_path = std::env::var("PATH").unwrap_or_default();
    let output = Command::new(env!("CARGO_BIN_EXE_cmux-herdr"))
        .args(["read-pane", "p1"])
        .env(
            "PATH",
            format!("{}:{inherited_path}", temp.path().display()),
        )
        .env("HERDR_ENV", "1")
        .env("HERDR_SOCKET_PATH", temp.path().join("missing.sock"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "partial output\n"
    );
    assert_eq!(String::from_utf8(output.stderr).unwrap(), "read warning\n");
}

fn base_command(temp: &tempfile::TempDir) -> Command {
    let inherited_path = std::env::var("PATH").unwrap_or_default();
    let mut command = Command::new(env!("CARGO_BIN_EXE_cmux-herdr"));
    command
        .env(
            "PATH",
            format!("{}:{inherited_path}", temp.path().display()),
        )
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

fn update_command_without_herdr(temp: &tempfile::TempDir) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cmux-herdr"));
    command
        .env("PATH", "")
        .env_remove("HERDR_BIN")
        .env("HOME", temp.path().join("home"))
        .env("XDG_CONFIG_HOME", temp.path().join("config"))
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_DATA_HOME", temp.path().join("data"));
    command
}

fn installed_update_service_paths(temp: &tempfile::TempDir) -> (PathBuf, Vec<PathBuf>) {
    let runtime = temp
        .path()
        .join("data/cmux-herdr/bin/cmux-herdr-update-service");
    let definition_dir = if cfg!(target_os = "macos") {
        temp.path().join("home/Library/LaunchAgents")
    } else {
        temp.path().join("config/systemd/user")
    };
    let definitions = if cfg!(target_os = "macos") {
        vec![definition_dir.join("com.cmux-herdr.herdr-auto-update.plist")]
    } else {
        vec![
            definition_dir.join("com.cmux-herdr.herdr-auto-update.service"),
            definition_dir.join("com.cmux-herdr.herdr-auto-update.timer"),
        ]
    };
    fs::create_dir_all(runtime.parent().unwrap()).unwrap();
    fs::create_dir_all(&definition_dir).unwrap();
    fs::write(&runtime, "runtime").unwrap();
    for definition in &definitions {
        fs::write(definition, "configured herdr: /removed/herdr\n").unwrap();
    }
    (runtime, definitions)
}

#[test]
fn live_lifecycle_never_stops_herdr() {
    let temp = tempfile::tempdir().unwrap();
    write_fake_herdr(&temp.path().join("herdr"));
    write_fake_cmux(&temp.path().join("cmux"));
    let attached = base_command(&temp)
        .args(["attach", "--json"])
        .output()
        .unwrap();
    assert!(
        attached.status.success(),
        "{}",
        String::from_utf8_lossy(&attached.stderr)
    );
    let attach: Value = serde_json::from_slice(&attached.stdout).unwrap();
    assert_eq!(attach["ok"], Value::Bool(true));
    assert_eq!(attach["server_stopped"], Value::Bool(false));
    assert!(attach["restore_path"].as_str().is_some());

    let observed = base_command(&temp)
        .args(["observe", "--method", "pane_surfaces", "--json"])
        .output()
        .unwrap();
    assert!(
        observed.status.success(),
        "{}",
        String::from_utf8_lossy(&observed.stderr)
    );
    let observe: Value = serde_json::from_slice(&observed.stdout).unwrap();
    assert_eq!(observe["ok"], Value::Bool(true));

    let detached = base_command(&temp)
        .args(["detach", "--json"])
        .output()
        .unwrap();
    assert!(
        detached.status.success(),
        "{}",
        String::from_utf8_lossy(&detached.stderr)
    );
    let detach: Value = serde_json::from_slice(&detached.stdout).unwrap();
    assert_eq!(detach["server_stopped"], Value::Bool(false));
}

#[test]
fn update_service_status_reports_discovered_paths() {
    let temp = tempfile::tempdir().unwrap();
    let herdr = temp.path().join("herdr");
    write_fake_herdr(&herdr);
    let output = base_command(&temp)
        .args([
            "update-service",
            "status",
            "--herdr",
            herdr.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["action"], "status");
    assert_eq!(payload["installed"], Value::Bool(false));
    assert_eq!(payload["runtime_exists"], Value::Bool(false));
}

#[test]
fn update_service_status_inspects_installed_state_after_herdr_is_removed() {
    let temp = tempfile::tempdir().unwrap();
    let (runtime, definitions) = installed_update_service_paths(&temp);

    let output = update_command_without_herdr(&temp)
        .args(["update-service", "status", "--json"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["action"], "status");
    assert_eq!(payload["installed"], Value::Bool(true));
    assert_eq!(payload["runtime_binary"], runtime.display().to_string());
    assert_eq!(
        payload["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|definition| definition["path"].as_str().unwrap())
            .collect::<Vec<_>>(),
        definitions
            .iter()
            .map(|path| path.to_str().unwrap())
            .collect::<Vec<_>>()
    );
}

#[test]
fn update_service_uninstall_removes_installed_state_after_herdr_is_removed() {
    let temp = tempfile::tempdir().unwrap();
    let (runtime, definitions) = installed_update_service_paths(&temp);
    let config = temp.path().join("config/herdr/config.toml");
    let original = "[update]\nversion_check = true\n";
    let installed = cmux_herdr::update::install_settings(
        original,
        "preview",
        "https://example.com/preview.json",
    )
    .unwrap();
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::write(&config, installed).unwrap();

    let output = update_command_without_herdr(&temp)
        .args(["update-service", "uninstall", "--json"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["action"], "uninstall");
    assert_eq!(payload["config"], "changed");
    assert_eq!(fs::read_to_string(config).unwrap(), original);
    assert!(!runtime.exists());
    assert!(definitions.iter().all(|path| !path.exists()));
}

#[test]
fn update_service_install_and_run_still_require_herdr() {
    let temp = tempfile::tempdir().unwrap();
    for arguments in [
        vec![
            "update-service",
            "install",
            "--manifest-url",
            "https://example.com/preview.json",
            "--json",
        ],
        vec!["update-service", "run", "--json"],
    ] {
        let output = update_command_without_herdr(&temp)
            .args(arguments)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("herdr not found on PATH"),
            "stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
