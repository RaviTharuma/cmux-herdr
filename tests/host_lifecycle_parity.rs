//! Golden and behavioral parity for the Python host/lifecycle modules.
use std::path::PathBuf;

use serde_json::{json, Value};

#[path = "../src/layout.rs"]
mod layout;
#[path = "../src/impose.rs"]
mod impose;
#[path = "../src/session.rs"]
mod session;
#[path = "../src/engine.rs"]
mod engine;
#[path = "../src/host.rs"]
mod host;
#[path = "../src/lifecycle.rs"]
mod lifecycle;

use engine::{apply_window, impose_after_apply, HerdrWindow};
use host::{host_actions, FakeBonsplitHost, HostAction};
use lifecycle::{
    connection_action, decode_beta, dispatch, endpoint_hash, existing_mirror_window, grid_match,
    host_close_policy, may_cache_connection, note_output, pane_grid_payload, plan_attach,
    plan_restore, post_attach_action, purge_dead_mirrors, read_restore, session_payload,
    validate_session_name, validate_socket_path, window_target_from_params, write_restore,
    AttachRegistry, AttachWindowTarget, ConnectionRecord, DiscoveredSession, LifecycleController,
    MirrorRecord, RestoreRecord, POST_APPLY_CLIENT_SIZE, POST_RESEED, SETTING_KEY,
    SOCKET_METHODS, TEARDOWN_EXPLICIT_DETACH, TEARDOWN_SESSION_ENDED,
};

fn golden() -> Value {
    serde_json::from_str(include_str!("host_lifecycle_golden.json")).unwrap()
}

fn horizontal() -> layout::LayoutNode {
    layout::parse_layout(&json!({
        "width": 200, "height": 50, "x": 0, "y": 0,
        "horizontal": [
            {"width": 100, "height": 50, "x": 0, "y": 0, "pane": "w2:p1"},
            {"width": 99, "height": 50, "x": 101, "y": 0, "pane": "w2:p2"}
        ]
    }))
    .unwrap()
}

fn window(layout: layout::LayoutNode, visible: Option<layout::LayoutNode>, zoomed: bool) -> HerdrWindow {
    HerdrWindow::new(
        "w2:t1",
        "Build",
        0,
        layout,
        visible,
        zoomed,
        Some("w2:p2".into()),
    )
}

fn action_json(action: &HostAction) -> Value {
    let mut out = serde_json::Map::new();
    out.insert("op".into(), json!(action.op));
    if let Some(value) = &action.pane_id {
        out.insert("pane_id".into(), json!(value));
    }
    if let Some(value) = &action.split_from_pane_id {
        out.insert("split_from_pane_id".into(), json!(value));
    }
    if let Some(value) = &action.orientation {
        out.insert("orientation".into(), json!(value));
    }
    if let Some(value) = action.fraction {
        out.insert("fraction".into(), json!(value));
    }
    if action.insert_first {
        out.insert("insert_first".into(), json!(true));
    }
    if let Some(value) = &action.split_key {
        out.insert("split_key".into(), json!(value));
    }
    Value::Object(out)
}

fn session() -> DiscoveredSession {
    DiscoveredSession::new("sess-1", "main", 2, false)
}

#[test]
fn ordered_host_actions_match_python_golden() {
    let node = horizontal();
    let (_, result) = apply_window(&window(node, None, false), None);
    let plan = impose_after_apply(&result, None, "Build");
    let actions = host_actions(&result, &plan);
    assert_eq!(
        Value::Array(actions.iter().map(action_json).collect()),
        golden()["host_first_apply"]
    );

    let mut fake = FakeBonsplitHost::default();
    fake.apply(&actions).unwrap();
    assert_eq!(fake.panels.len(), 2);
    assert_eq!(fake.last_tree_op.as_deref(), Some("rebuild_tree"));
    assert_eq!(fake.focus.as_deref(), Some("w2:p2"));
    assert_eq!(fake.log[..3], ["create:w2:p1", "create:w2:p2", "rebuild_tree"]);
}

#[test]
fn fake_host_validates_order_and_unknown_verbs() {
    let mut fake = FakeBonsplitHost::default();
    let error = fake
        .apply(&[HostAction::new("focus").with_pane_id("w2:p1")])
        .unwrap_err();
    assert_eq!(error.to_string(), "focus missing panel w2:p1");
    assert!(fake.apply(&[HostAction::new("unknown")]).is_err());
    assert!(fake.apply(&[HostAction::new("create_panel").with_pane_id("")]).is_err());
    fake.apply(&[HostAction::new("create_panel").with_pane_id("p")]).unwrap();
    fake.apply(&[HostAction::new("bind_surface").with_pane_id("").with_surface_id("surface")]).unwrap();
    assert!(fake.surfaces.is_empty());
    fake.apply(&[HostAction::new("close_panel").with_pane_id("")]).unwrap();
    assert!(fake.panels.contains("p"));
}

#[test]
fn host_expands_removes_and_skips_held_divider() {
    let leaf = layout::parse_layout(&json!({
        "width": 200, "height": 50, "x": 0, "y": 0, "pane": "w2:p1"
    }))
    .unwrap();
    let (state, first) = apply_window(&window(leaf.clone(), None, false), None);
    let mut fake = FakeBonsplitHost::default();
    fake.apply(&host_actions(&first, &impose_after_apply(&first, None, "Build"))).unwrap();

    let split = horizontal();
    let (state, expanded) = apply_window(&window(split.clone(), None, false), Some(&state));
    let actions = host_actions(&expanded, &impose_after_apply(&expanded, Some(&leaf), "Build"));
    assert_eq!(actions[0].op, "create_panel");
    assert_eq!(actions[1].op, "expand_leaf");
    fake.apply(&actions).unwrap();

    let (_, removed) = apply_window(&window(leaf.clone(), None, false), Some(&state));
    let actions = host_actions(&removed, &impose_after_apply(&removed, Some(&split), "Build"));
    assert_eq!(actions[0].op, "close_panel");
    assert_eq!(actions[1].op, "remove_leaf");
    fake.apply(&actions).unwrap();
    assert_eq!(fake.panels, ["w2:p1".to_string()].into_iter().collect());

    let (_, reconcile) = apply_window(&window(split, None, false), None);
    let hold = impose::begin_divider_drag("s", "horizontal", 50);
    let plan = impose::plan_from_reconcile(&reconcile, None, "Build", None, None, None, Some(&hold));
    assert!(host::divider_impose_actions(&plan.divider_tree, plan.held_split_key.as_deref(), "s").is_empty());
}

#[test]
fn lifecycle_validation_flags_and_methods_match_golden() {
    let g = golden();
    let socket = g["endpoint"]["socket"].as_str().unwrap();
    assert_eq!(endpoint_hash(socket), g["endpoint"]["hash"]);
    assert_eq!(validate_socket_path(Some("  /tmp/herdr.sock  ")), Some(socket.into()));
    for bad in ["tmp/herdr.sock", "-oProxyCommand=x", "/tmp/herdr\u{1b}sock", ""] {
        assert_eq!(validate_socket_path(Some(bad)), None);
    }
    assert_eq!(validate_session_name(Some("  main  ")), Some("main".into()));
    assert_eq!(validate_session_name(Some("  ")), None);
    assert!(!decode_beta(None, false));
    assert!(decode_beta(Some(&json!("on")), false));
    assert!(!decode_beta(Some(&json!("off")), true));
    assert!(decode_beta(Some(&json!(1)), false));
    assert!(decode_beta(Some(&json!(1.0)), false));
    assert!(!decode_beta(Some(&json!(0.0)), true));
    assert_eq!(SETTING_KEY, "betaFeatures.remoteHerdrMirror");
    assert_eq!(json!(SOCKET_METHODS), g["socket_methods"]);
}

#[test]
fn attach_targets_preserve_affinity_and_fail_closed() {
    let explicit = AttachWindowTarget::new("explicit", Some("win-other".into()));
    assert_eq!(explicit.resolve(Some("win-active"), Some("win-other"), |id| id != "dead"), Some("win-active".into()));
    assert_eq!(AttachWindowTarget::new("explicit", Some("dead".into())).resolve(None, Some("win-active"), |id| id == "win-active"), None);
    assert_eq!(AttachWindowTarget::new("unresolved_explicit", None).resolve(None, Some("win-active"), |_| true), None);
    assert_eq!(AttachWindowTarget::new("contextual", Some("dead".into())).resolve(None, Some("win-active"), |id| id == "win-active"), Some("win-active".into()));
    assert_eq!(AttachWindowTarget::new("dedicated_new_window", None).resolve(None, Some("win-active"), |_| true), None);
    assert_eq!(window_target_from_params(&json!({"window_id": null}), false).kind, "unresolved_explicit");
    assert_eq!(window_target_from_params(&json!({"window_id": "x"}), true).kind, "dedicated_new_window");
}

#[test]
fn attach_planning_matches_python_branches() {
    let active = vec!["win-active".to_string(), "win-other".to_string()];
    let sessions = vec![session()];
    let contextual = AttachWindowTarget::new("contextual", None);
    let preflight = plan_attach(&contextual, true, true, false, None, Some("win-active"), &active, None, None, None, false, None);
    assert_eq!(preflight.outcome, "discover");
    let dedicated = plan_attach(&AttachWindowTarget::new("dedicated_new_window", None), true, true, false, None, Some("win-active"), &active, Some(&sessions), None, None, false, None);
    assert_eq!(dedicated.outcome, "mirrored");
    assert!(dedicated.create_window);
    assert_eq!(dedicated.sessions_to_mirror, ["sess-1"]);
    assert_eq!(dedicated.post_attach.as_deref(), Some(POST_APPLY_CLIENT_SIZE));

    let mirrors = vec![MirrorRecord::new("sess-1", "win-active", Some("ws-1".into()))];
    let live = vec!["sess-1".into()];
    let reused = plan_attach(&contextual, true, true, false, Some("win-active"), Some("win-active"), &active, Some(&sessions), Some(&mirrors), Some(&live), false, None);
    assert_eq!(reused.outcome, "reused");
    assert_eq!(reused.sessions_to_reuse, ["sess-1"]);
    assert!(reused.sessions_to_mirror.is_empty());

    assert_eq!(plan_attach(&contextual, false, true, false, None, Some("win-active"), &active, None, None, None, false, None).outcome, "disabled");
    assert_eq!(plan_attach(&contextual, true, false, false, None, Some("win-active"), &active, None, None, None, false, None).outcome, "unreachable");
    assert_eq!(plan_attach(&contextual, true, true, true, None, Some("win-active"), &active, None, None, None, false, None).outcome, "already_attaching");
}

#[test]
fn registry_connections_purge_and_close_policy_match_python() {
    let mut registry = AttachRegistry::default();
    assert!(registry.begin_attach("abc"));
    assert!(!registry.begin_attach("abc"));
    registry.end_attach("abc");
    assert!(registry.begin_attach("abc"));

    assert_eq!(connection_action(None), "start");
    let live = ConnectionRecord::started("s");
    assert_eq!(connection_action(Some(&live)), "reuse");
    let mut dead = live.clone();
    dead.exited = true;
    assert_eq!(connection_action(Some(&dead)), "replace");
    assert!(may_cache_connection(&live));
    assert!(!may_cache_connection(&dead));
    assert_eq!(post_attach_action(true), POST_RESEED);
    assert_eq!(post_attach_action(false), POST_APPLY_CLIENT_SIZE);

    let mirrors = vec![
        MirrorRecord::new("dead", "win-a", None),
        MirrorRecord::new("live", "win-a", Some("ws-live".into())),
    ];
    let kept = purge_dead_mirrors(&mirrors);
    assert_eq!(kept.len(), 1);
    assert_eq!(existing_mirror_window(&kept, &["win-a".into()]).as_deref(), Some("win-a"));
    for (source, result) in golden()["close_sources"].as_object().unwrap() {
        assert_eq!(host_close_policy(source), result.as_str().unwrap());
    }
}

#[test]
fn restore_record_json_and_atomic_files_match_python() {
    let record = RestoreRecord::new(
        "0cb0c5e4a7465217",
        "/tmp/herdr.sock",
        vec!["sess-1".into()],
        "contextual",
        Some("win-active".into()),
    );
    assert_eq!(record.to_value(), golden()["restore_payload"]);
    assert!(RestoreRecord::from_value(&json!({"mode": "replay_tree"})).is_none());
    assert!(RestoreRecord::from_value(&json!({
        "endpoint_hash": false,
        "socket_path": "/tmp/herdr.sock",
        "session_ids": ["sess-1"],
        "target_kind": "contextual"
    })).is_none());

    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("nested/restore.json");
    write_restore(&path, &record).unwrap();
    assert_eq!(read_restore(&path).unwrap(), Some(record));
    assert!(!path.with_extension("json.tmp").exists());
    assert_eq!(read_restore(&temp.path().join("missing.json")).unwrap(), None);
    std::fs::write(&path, b"not json").unwrap();
    assert_eq!(read_restore(&path).unwrap(), None);
}

#[test]
fn restore_planning_always_reattaches_and_reseeds() {
    let record = RestoreRecord::new("x", "/tmp/herdr.sock", vec!["sess-1".into()], "explicit", Some("win-stale".into()));
    let plan = plan_restore(&record, true, true, &[session()], &["win-active".into()], Some("win-active"));
    assert_eq!(plan.outcome, "mirrored");
    assert_eq!(plan.post_attach.as_deref(), Some(POST_RESEED));
    assert_eq!(plan.reason.as_deref(), Some("restore_reattach"));
    assert!(!plan.create_window);

    let dedicated = RestoreRecord::new("x", "/tmp/herdr.sock", vec!["sess-1".into()], "dedicated_new_window", None);
    assert!(plan_restore(&dedicated, true, true, &[session()], &["win-active".into()], None).create_window);
}

#[test]
fn controller_attach_reuse_move_restore_and_detach_are_safe() {
    let mut ctl = LifecycleController::default();
    let contextual = AttachWindowTarget::new("contextual", None);
    let first = ctl.attach("/tmp/herdr.sock", &[session()], &contextual, false);
    assert_eq!(first["outcome"], "mirrored");
    assert_eq!(first["post_attach"], POST_APPLY_CLIENT_SIZE);
    let second = ctl.attach("/tmp/herdr.sock", &[session()], &contextual, false);
    assert_eq!(second["outcome"], "reused");

    assert!(ctl.registry.begin_attach(&endpoint_hash("/tmp/herdr.sock")));
    let locked = ctl.attach("/tmp/herdr.sock", &[session()], &contextual, false);
    assert_eq!(locked["outcome"], "already_attaching");
    ctl.registry.end_attach(&endpoint_hash("/tmp/herdr.sock"));

    let moved = ctl.attach("/tmp/herdr.sock", &[session()], &AttachWindowTarget::new("dedicated_new_window", None), true);
    assert!(moved["window_id"].as_str().unwrap().starts_with("win-new-"));
    assert_eq!(ctl.active_window_id.as_deref(), moved["window_id"].as_str());

    ctl.mirrors.clear();
    ctl.connections.clear();
    let restored = ctl.restore(&[session()]);
    assert_eq!(restored["mode"], "reattach");
    assert_eq!(restored["post_attach"], POST_RESEED);

    let detached = ctl.detach("sess-1", TEARDOWN_EXPLICIT_DETACH);
    assert_eq!(detached["server_stopped"], false);
    let ended = ctl.detach("sess-1", TEARDOWN_SESSION_ENDED);
    assert_eq!(ended["reason"], TEARDOWN_SESSION_ENDED);
    let closed = ctl.close_host("last_workspace_tab");
    assert_eq!(closed["server_stopped"], false);
    assert!(!ctl.server_stopped);
}

#[test]
fn dispatch_state_and_grid_payloads_preserve_json_contract() {
    assert_eq!(dispatch("remote.herdr.sessions", &json!({"socket": "/tmp/herdr.sock"}), false).code.as_deref(), Some("disabled"));
    assert_eq!(dispatch("remote.tmux.attach", &json!({"socket": "/tmp/herdr.sock"}), true).code.as_deref(), Some("unknown_method"));
    assert_eq!(dispatch("remote.herdr.attach", &json!({"socket": "/tmp/herdr.sock"}), true).code.as_deref(), Some("invalid_params"));
    assert_eq!(dispatch("remote.herdr.attach", &json!({"socket": "/tmp/herdr.sock"}), true).to_value(), json!({"ok": false, "code": "invalid_params"}));
    let accepted = dispatch("remote.herdr.attach", &json!({"socket": "/tmp/herdr.sock", "session": "main", "activate": true}), true);
    assert!(accepted.ok);
    assert!(accepted.activate);
    assert_eq!(accepted.session.as_deref(), Some("main"));
    assert_eq!(dispatch("remote.herdr.window", &json!({"socket_path": "/tmp/herdr.sock"}), true).target.unwrap().kind, "dedicated_new_window");

    assert_eq!(session_payload(&session()), golden()["session_payload"]);
    let mut ctl = LifecycleController::default();
    ctl.attach("/tmp/herdr.sock", &[session()], &AttachWindowTarget::new("contextual", None), false);
    note_output(ctl.connections.get_mut("sess-1").unwrap(), "w2:p1", 12);
    let state = ctl.state("sess-1");
    assert_eq!(state["attached"], true);
    assert_eq!(state["total_output_bytes"], 12);
    assert_eq!(state["pane_output_bytes"]["w2:p1"], 12);

    assert!(grid_match(80, 24, 80, 24, true, true));
    assert!(!grid_match(80, 24, 79, 24, true, false));
    assert!(grid_match(80, 24, 80, 30, true, false));
    let payload = pane_grid_payload(
        "w2:t1",
        &[json!({
            "pane_id": "w2:p1", "assigned_cols": "80", "assigned_rows": 24,
            "rendered_cols": 80, "rendered_rows": 24,
            "exact_cols": true, "exact_rows": true, "has_panel": 0
        })],
        0,
        false,
        80,
        24,
        Some((80, 24)),
        true,
    );
    assert_eq!(payload["panes"][0]["match"], true);
    assert_eq!(payload["panes"][0]["assigned"]["cols"], "80");
    assert_eq!(payload["panes"][0]["has_panel"], false);
    assert_eq!(payload["pushed"]["cols"], 80);
}
