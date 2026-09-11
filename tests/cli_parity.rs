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
            .env("CMUX_SURFACE_ID", "surface-golden")
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

#[test]
fn native_theme_sync_retries_legacy_color_then_deduplicates() {
    let temp = tempfile::tempdir().unwrap();
    write_fake_herdr(&temp.path().join("herdr"));
    write_fake_cmux(&temp.path().join("cmux"));
    let cmux = temp.path().join("cmux");
    let script = fs::read_to_string(&cmux).unwrap().replace(
        "if [ \"$1\" = \"list-status\" ]; then",
        "if [ \"$1\" = \"set-status\" ] && [ \"$FAIL_STATUS\" = 1 ]; then exit 7; fi\nif [ \"$1\" = \"list-status\" ]; then",
    );
    fs::write(&cmux, script).unwrap();
    let sync = |fail: bool| {
        base_command(&temp)
            .args(["sync", "--workspace", "workspace:1", "--json"])
            .env("CMUX_HERDR_FORCE_PLUGIN", "1")
            .env_remove("CMUX_HERDR_NATIVE_LIVE")
            .env("FAIL_STATUS", if fail { "1" } else { "0" })
            .output()
            .unwrap()
    };
    let output = sync(false);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let state_path = fs::read_dir(temp.path().join("state/cmux-herdr"))
        .unwrap()
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("associations-")
        })
        .unwrap()
        .path();
    let mut saved: Value = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    for pane in saved["panes"].as_object_mut().unwrap().values_mut() {
        pane["last_color"] = serde_json::json!("#ff9500");
    }
    fs::write(&state_path, serde_json::to_vec(&saved).unwrap()).unwrap();
    let log = temp.path().join("cmux.log");
    fs::write(&log, "").unwrap();
    let _failed = sync(true);
    let calls = fs::read_to_string(&log).unwrap();
    assert!(
        calls
            .lines()
            .any(|line| line.starts_with("set-status herdr:p1 ")),
        "legacy color must force a retry: {calls}"
    );
    let stdout = String::from_utf8_lossy(&_failed.stdout);
    let failed: Value = serde_json::from_str(&stdout[stdout.find('{').unwrap()..]).unwrap();
    assert_eq!(failed["applied"], serde_json::json!([]));
    assert_eq!(
        failed["errors"].as_array().unwrap().len(),
        1,
        "fake set-status failure must be observed"
    );
    let saved: Value = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    assert!(saved["panes"]
        .as_object()
        .unwrap()
        .values()
        .all(|pane| pane["last_color"] == "#ff9500"));
    fs::write(&log, "").unwrap();
    let output = sync(false);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let calls = fs::read_to_string(&log).unwrap();
    assert_eq!(calls.lines().find(|line| line.starts_with("set-status ")), Some("set-status herdr:p1 pi/working · Bot --icon hammer --priority 80 --workspace workspace:1"));
    fs::write(&log, "").unwrap();
    let output = sync(false);
    assert!(output.status.success());
    let calls = fs::read_to_string(&log).unwrap();
    assert!(
        !calls.lines().any(|line| line.starts_with("set-status ")),
        "successful color reset must deduplicate: {calls}"
    );
}

#[test]
fn native_follower_sync_preserves_all_persistent_state() {
    let temp = tempfile::tempdir().unwrap();
    write_fake_herdr(&temp.path().join("herdr"));
    write_fake_cmux(&temp.path().join("cmux"));
    let state = temp.path().join("state/cmux-herdr");
    fs::create_dir_all(&state).unwrap();
    let sentinel = state.join("associations-sentinel.json");
    let bytes = br#"{"owner":"native","panes":{"p1":{"title_lock":true}}}"#;
    fs::write(&sentinel, bytes).unwrap();
    let output = base_command(&temp)
        .args(["sync", "--workspace", "workspace:1", "--json"])
        .env("CMUX_HERDR_NATIVE_LIVE", "1")
        .env_remove("CMUX_HERDR_FORCE_PLUGIN")
        .env("CMUX_HERDR_NATIVE_STATE_DIR", temp.path().join("native"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("foreign_writer"));
    assert_eq!(fs::read(sentinel).unwrap(), bytes);
    assert_eq!(
        fs::read_dir(state).unwrap().count(),
        1,
        "follower created persistent state"
    );
    assert!(!temp.path().join("native").exists());
    assert_eq!(
        fs::read_to_string(temp.path().join("cmux.log")).unwrap_or_default(),
        ""
    );
}

#[test]
fn mirror_executes_supported_surface_creation_launch_and_rename() {
    let temp = tempfile::tempdir().unwrap();
    write_fake_herdr(&temp.path().join("herdr"));
    fs::write(
        temp.path().join("cmux"),
        r#"#!/bin/sh
printf '%s\n' "$*" >> "$FAKE_CMUX_LOG"
if [ "$2" = '--help' ]; then
  case "$1" in
    new-split) echo 'Usage: cmux new-split <direction> --surface <id>' ;;
    new-surface) echo 'Usage: cmux new-surface --pane <id>' ;;
    respawn-pane) echo 'Usage: cmux respawn-pane --surface <id> --command <cmd>' ;;
    rename-tab) echo 'Usage: cmux rename-tab --surface <id> --title <title>' ;;
    *) exit 9 ;;
  esac
  exit 0
fi
case "$1" in
  new-surface)
    [ "$2 $3" = '--type terminal' ] || exit 9
    printf '%s\n' '{"surface_ref":"surface:2","pane_id":null,"pane_ref":"pane:2"}' ;;
  rename-tab)
    [ "$2 $3" = '--surface surface:2' ] || exit 9
    case "$4" in --title=*) ;; *) exit 9 ;; esac
    printf '%s\n' '{"ok":true}' ;;
  respawn-pane)
    [ "$2 $3 $4" = '--surface surface:2 --command' ] || exit 9
    printf '%s\n' "$5" > "$LAUNCHED_COMMAND"
    printf '%s\n' '{"ok":true}' ;;
  tree|list-terminals|ids) printf '%s\n' '{"items":[]}' ;;
  set-status|list-status|clear-status) exit 0 ;;
  *) echo "unsupported: $*" >&2; exit 9 ;;
esac
"#,
    )
    .unwrap();
    fs::set_permissions(temp.path().join("cmux"), fs::Permissions::from_mode(0o755)).unwrap();
    let launched = temp.path().join("launched");
    let output = base_command(&temp)
        .args([
            "mirror",
            "--all",
            "--workspace",
            "workspace:1",
            "--no-layout",
            "--no-log",
            "--json",
        ])
        .env_remove("CMUX_HERDR_NATIVE_LIVE")
        .env_remove("CMUX_HERDR_FORCE_PLUGIN")
        .env("CMUX_HERDR_NATIVE_STATE_DIR", temp.path().join("native"))
        .env("LAUNCHED_COMMAND", &launched)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: Value = serde_json::from_str(&stdout[stdout.find('{').unwrap()..]).unwrap();
    assert_eq!(report["plan"]["errors"], serde_json::json!([]));
    assert_eq!(report["plan"]["created"], serde_json::json!(["p1"]));
    assert_eq!(report["status_sync"]["errors"], serde_json::json!([]));
    let command = fs::read_to_string(launched).unwrap_or_else(|error| {
        panic!(
            "{error}; stdout={}; calls={}",
            String::from_utf8_lossy(&output.stdout),
            fs::read_to_string(temp.path().join("cmux.log")).unwrap_or_default()
        )
    });
    assert!(command.contains("'attach-pane' 'p1'"), "{command}");
    let calls = fs::read_to_string(temp.path().join("cmux.log")).unwrap();
    assert_eq!(
        calls.lines().find(|line| line.starts_with("set-status ")),
        Some("set-status herdr:p1 pi/working · Bot --icon hammer --priority 80 --workspace workspace:1"),
        "native mirror must leave status color to cmux: {calls}"
    );
    let state = temp.path().join("state/cmux-herdr");
    let mirror = fs::read_dir(state)
        .unwrap()
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("associations-")
        })
        .unwrap();
    let saved: Value = serde_json::from_slice(&fs::read(mirror.path()).unwrap()).unwrap();
    assert_eq!(saved["mirrors"]["p1"]["cmux_surface_id"], "surface:2");
    assert_eq!(saved["mirrors"]["p1"]["cmux_pane_id"], "pane:2");
    fs::write(temp.path().join("cmux.log"), "").unwrap();
    let output = base_command(&temp)
        .args([
            "sync",
            "--workspace",
            "workspace:1",
            "--no-progress",
            "--no-log",
        ])
        .env("CMUX_HERDR_FORCE_PLUGIN", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let calls = fs::read_to_string(temp.path().join("cmux.log")).unwrap();
    assert!(
        !calls.lines().any(|line| line.starts_with("set-status ")),
        "sync must reuse the successful native mirror status: {calls}"
    );
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
fn update_service_uninstall_preserves_unowned_artifacts_without_herdr() {
    let temp = tempfile::tempdir().unwrap();
    let (runtime, definitions) = installed_update_service_paths(&temp);
    let config = temp.path().join("config/herdr/config.toml");
    let original = "[update]\nversion_check = true\n";
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::write(&config, original).unwrap();
    let output = update_command_without_herdr(&temp)
        .args(["update-service", "uninstall", "--json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ownership manifest is missing"));
    assert_eq!(fs::read_to_string(runtime).unwrap(), "runtime");
    for definition in definitions {
        assert_eq!(
            fs::read_to_string(definition).unwrap(),
            "configured herdr: /removed/herdr\n"
        );
    }
    assert_eq!(fs::read_to_string(config).unwrap(), original);
}

#[test]
fn update_service_uninstall_removes_installed_state_after_herdr_is_removed() {
    let temp = tempfile::tempdir().unwrap();
    use cmux_herdr::update::{
        CommandOutput, CommandRunner, InstallRequest, ServiceManager, ServicePaths,
    };
    struct InstallRunner;
    impl CommandRunner for InstallRunner {
        fn run(
            &self,
            _program: &std::path::Path,
            args: &[String],
        ) -> std::io::Result<CommandOutput> {
            let inactive = args
                .iter()
                .any(|arg| matches!(arg.as_str(), "print" | "is-enabled" | "is-active"));
            Ok(CommandOutput {
                status: i32::from(inactive),
                stdout: if args.iter().any(|arg| arg == "--default-config") {
                    "# manifest_url = \"https://example.com/preview.json\"\n".into()
                } else {
                    String::new()
                },
                stderr: String::new(),
            })
        }
    }
    let manager = if cfg!(target_os = "macos") {
        ServiceManager::Launchd {
            domain: format!("gui/{}", rustix::process::getuid().as_raw()),
        }
    } else {
        ServiceManager::Systemd
    };
    let config = temp.path().join("config/herdr/config.toml");
    let original = "[update]\nversion_check = true\n";
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::write(&config, original).unwrap();
    let paths = ServicePaths {
        home: temp.path().join("home"),
        config_path: config.clone(),
        state_root: temp.path().join("state/cmux-herdr"),
        data_root: temp.path().join("data/cmux-herdr"),
        definition_dir: if cfg!(target_os = "macos") {
            temp.path().join("home/Library/LaunchAgents")
        } else {
            temp.path().join("config/systemd/user")
        },
        source_binary: temp.path().join("source-cmux-herdr"),
        herdr_binary: temp.path().join("herdr"),
    };
    fs::write(&paths.source_binary, b"isolated service runtime").unwrap();
    write_fake_herdr(&paths.herdr_binary);
    let result = cmux_herdr::update::install_service(
        &InstallRequest::new(
            manager,
            paths.clone(),
            "preview".into(),
            "https://example.com/preview.json".into(),
        ),
        &InstallRunner,
    )
    .unwrap();
    let runtime = result.runtime_binary;
    let definitions = result.definitions;
    fs::remove_file(&paths.herdr_binary).unwrap();
    fs::remove_file(&paths.source_binary).unwrap();

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

#[test]
fn sync_fails_closed_without_host_fingerprint() {
    let temp = tempfile::tempdir().unwrap();
    write_fake_herdr(&temp.path().join("herdr"));
    write_fake_cmux(&temp.path().join("cmux"));
    let inherited_path = std::env::var("PATH").unwrap_or_default();
    let output = Command::new(env!("CARGO_BIN_EXE_cmux-herdr"))
        .args(["sync", "--no-progress", "--no-log"])
        .env(
            "PATH",
            format!("{}:{inherited_path}", temp.path().display()),
        )
        .env("HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("HERDR_ENV", "1")
        .env("HERDR_SOCKET_PATH", temp.path().join("missing.sock"))
        .env("CMUX_WORKSPACE_ID", "workspace:stale")
        .env_remove("CMUX_SURFACE_ID")
        .env("FAKE_HERDR_LOG", temp.path().join("herdr.log"))
        .env("FAKE_CMUX_LOG", temp.path().join("cmux.log"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("could not resolve cmux workspace"),
        "stderr={stderr}"
    );
    assert!(
        stderr.contains("will not borrow bare focused workspace"),
        "stderr={stderr}"
    );
    let cmux_log = fs::read_to_string(temp.path().join("cmux.log")).unwrap_or_default();
    assert!(
        !cmux_log.contains("identify"),
        "must not probe cmux identify without fingerprint; log={cmux_log}"
    );
}

#[test]
fn sync_resolves_via_identify_surface_when_fingerprint_complete() {
    let temp = tempfile::tempdir().unwrap();
    write_fake_herdr(&temp.path().join("herdr"));
    let cmux = temp.path().join("cmux");
    let script = r#"#!/bin/sh
printf '%s\n' "$*" >> "$FAKE_CMUX_LOG"
if [ "$1" = "identify" ]; then
  printf '%s\n' '{"caller":{"workspace_ref":"workspace:pinned"},"focused":{"workspace_ref":"workspace:other"}}'
  exit 0
fi
if [ "$1" = "list-status" ]; then
  printf '%s\n' 'herdr:p1=current'
  exit 0
fi
if [ "$1" = "set-status" ] || [ "$1" = "clear-status" ] || [ "$1" = "log" ]; then
  exit 0
fi
exit 0
"#;
    fs::write(&cmux, script).unwrap();
    let mut permissions = fs::metadata(&cmux).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&cmux, permissions).unwrap();
    let sock = temp.path().join("herdr.sock");
    fs::write(&sock, "").unwrap();
    let inherited_path = std::env::var("PATH").unwrap_or_default();
    let output = Command::new(env!("CARGO_BIN_EXE_cmux-herdr"))
        .args(["sync", "--no-progress", "--no-log"])
        .env(
            "PATH",
            format!("{}:{inherited_path}", temp.path().display()),
        )
        .env("HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("HERDR_ENV", "1")
        .env("HERDR_SOCKET_PATH", &sock)
        .env("CMUX_SURFACE_ID", "surface-sync")
        .env("CMUX_WORKSPACE_ID", "workspace:stale")
        .env("FAKE_HERDR_LOG", temp.path().join("herdr.log"))
        .env("FAKE_CMUX_LOG", temp.path().join("cmux.log"))
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ws=workspace:pinned"), "stdout={stdout}");
    let cmux_log = fs::read_to_string(temp.path().join("cmux.log")).unwrap();
    assert!(
        cmux_log.contains("identify --surface surface-sync"),
        "cmux_log={cmux_log}"
    );
    assert!(
        !cmux_log.contains("--workspace workspace:stale"),
        "must not write pills to stale env workspace; cmux_log={cmux_log}"
    );
    let state_dir = temp.path().join("state/cmux-herdr");
    let parents: Vec<_> = fs::read_dir(&state_dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("parent-"))
        .collect();
    assert_eq!(
        parents.len(),
        1,
        "plugin writer should persist parent binding"
    );
    let body = fs::read_to_string(parents[0].path()).unwrap();
    assert!(
        body.contains("workspace:pinned"),
        "parent binding body={body}"
    );
}
