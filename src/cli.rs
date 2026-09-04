//! Complete clap parser and command dispatch for `cmux-herdr`.

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    LazyLock,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::{json, Map, Value};

use crate::api::{self, ApiError, ApiOutcome, CliRunner, HerdrApi};
use crate::bridge::{self, BridgeError};
use crate::state::{self, SystemEnv};

static VERSION: LazyLock<String> = LazyLock::new(crate::version::read_version);
const API_TIMEOUT: Duration = Duration::from_secs(8);

static WATCH_STOP: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
struct ErrorDeduplicator {
    last: String,
}
impl ErrorDeduplicator {
    fn success(&mut self) {
        self.last.clear();
    }
    fn report(&mut self, text: String) {
        if self.last != text {
            eprintln!("{text}");
            self.last = text;
        }
    }
}

#[cfg(unix)]
extern "C" fn stop_watch(_: i32) {
    WATCH_STOP.store(true, Ordering::Relaxed);
}

#[cfg(unix)]
fn install_watch_signals() {
    unsafe extern "C" {
        fn signal(signal: i32, handler: extern "C" fn(i32)) -> usize;
    }
    unsafe {
        signal(2, stop_watch);
        signal(15, stop_watch);
    }
}

#[cfg(not(unix))]
fn install_watch_signals() {}

struct ProcessCliRunner;

impl CliRunner for ProcessCliRunner {
    fn run(&self, argv: &[String]) -> Result<Value, ApiError> {
        let herdr =
            bridge::which("herdr").ok_or_else(|| ApiError("herdr not found on PATH".into()))?;
        let mut owned = Vec::with_capacity(argv.len() + 1);
        owned.push(herdr);
        owned.extend(argv.iter().cloned());
        let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
        let proc = bridge::run_cmd(&refs, API_TIMEOUT, None)
            .map_err(|error| ApiError(format!("failed to run herdr: {error}")))?;
        if proc.returncode != 0 {
            let detail = if !proc.stderr.trim().is_empty() {
                proc.stderr.trim()
            } else if !proc.stdout.trim().is_empty() {
                proc.stdout.trim()
            } else {
                return Err(ApiError(proc.returncode.to_string()));
            };
            return Err(ApiError(detail.to_string()));
        }
        let text = proc.stdout.trim();
        if text.is_empty() {
            return Ok(json!({"ok": true}));
        }
        if text.starts_with('{') || text.starts_with('[') {
            if let Ok(value) = serde_json::from_str(text) {
                return Ok(value);
            }
        }
        Ok(Value::String(text.to_string()))
    }
}

fn json_flag() -> Arg {
    Arg::new("json").long("json").action(ArgAction::SetTrue)
}
fn str_arg(name: &'static str) -> Arg {
    Arg::new(name)
}
fn opt(name: &'static str) -> Arg {
    Arg::new(name).long(name)
}
fn bool_opt(name: &'static str) -> Arg {
    opt(name).action(ArgAction::SetTrue)
}
fn direction_arg(name: &'static str) -> Arg {
    Arg::new(name).value_parser(["right", "left", "up", "down"])
}
fn timeout_args(command: Command) -> Command {
    command
        .arg(opt("timeout").value_parser(clap::value_parser!(i64)))
        .arg(
            opt("timeout-ms")
                .value_parser(clap::value_parser!(i64))
                .hide(true),
        )
}
fn read_args(command: Command, raw: bool) -> Command {
    let command = command
        .arg(opt("source").value_parser(["visible", "recent", "recent-unwrapped", "detection"]))
        .arg(opt("lines").value_parser(clap::value_parser!(i64)))
        .arg(opt("format").value_parser(["text", "ansi"]))
        .arg(bool_opt("ansi"));
    if raw {
        command.arg(bool_opt("raw"))
    } else {
        command
    }
}
fn sync_args(command: Command) -> Command {
    command
        .arg(opt("workspace"))
        .arg(bool_opt("no-clear-stale"))
        .arg(bool_opt("no-progress"))
        .arg(bool_opt("no-log"))
        .arg(json_flag())
}
fn mirror_args(command: Command) -> Command {
    command
        .arg(bool_opt("all"))
        .arg(opt("herdr-workspace"))
        .arg(opt("tab"))
        .arg(bool_opt("prune"))
        .arg(bool_opt("dry-run"))
        .arg(bool_opt("no-status"))
        .arg(bool_opt("tmux-parity"))
        .arg(bool_opt("focus"))
        .arg(bool_opt("order"))
        .arg(bool_opt("ratios"))
        .arg(bool_opt("no-layout"))
}
fn lifecycle_args(command: Command) -> Command {
    command
        .arg(opt("socket"))
        .arg(opt("session"))
        .arg(json_flag())
}

/// Build the complete command parser. Every Python `cmd_*` handler has one
/// identically named subcommand; `sidebar` is the native launcher entrypoint.
pub fn build_parser() -> Command {
    let status = Command::new("status")
        .about("Show dual cmux+herdr context")
        .arg(json_flag());
    let doctor = Command::new("doctor")
        .about("Diagnose plugin install (herdr, socket, fingerprint, LaunchAgent, sidebar)")
        .arg(json_flag());
    let lease = Command::new("lease")
        .about("Show plugin↔native writer lease for this host fingerprint")
        .arg(json_flag());
    let tree = Command::new("tree")
        .about("Pretty-print herdr workspaces/tabs/panes/agents")
        .arg(json_flag());
    let sync =
        sync_args(Command::new("sync").about("One-shot mirror herdr agents → cmux set-status"));
    let watch = mirror_args(sync_args(
        Command::new("watch").about("Live Herdr session as real cmux tabs/panes (default)"),
    ))
    .arg(
        Arg::new("interval")
            .short('n')
            .long("interval")
            .default_value("3.0")
            .value_parser(clap::value_parser!(f64)),
    )
    .arg(bool_opt("once"))
    .arg(bool_opt("mirror"))
    .arg(bool_opt("events"))
    .arg(
        Arg::new("pills-only")
            .long("pills-only")
            .action(ArgAction::SetTrue),
    );
    let mirror = mirror_args(sync_args(Command::new("mirror").about(
        "Project Herdr tabs/panes into real cmux tabs/splits (userspace ssh-tmux analogue)",
    )));
    let attach_pane = Command::new("attach-pane")
        .about("Follow one Herdr pane in this terminal (used by mirror surfaces)")
        .arg(str_arg("pane_id").required(true))
        .arg(
            Arg::new("interval")
                .short('n')
                .long("interval")
                .default_value("0.25")
                .value_parser(clap::value_parser!(f64)),
        )
        .arg(
            opt("lines")
                .default_value("200")
                .value_parser(clap::value_parser!(i64)),
        )
        .arg(bool_opt("read-only"))
        .arg(bool_opt("no-raw-tty"))
        .arg(bool_opt("no-resize"))
        .arg(bool_opt("no-ansi"));
    let focus_tab = Command::new("focus-tab")
        .about("Focus herdr tab by id or label")
        .arg(str_arg("target").required(true));
    let focus_pane = Command::new("focus-pane")
        .about("Focus herdr pane by id (via herdr agent focus; no zoom fallback)")
        .arg(str_arg("pane_id").required(true));
    let focus_workspace = Command::new("focus-workspace")
        .about("Focus herdr workspace by id")
        .arg(str_arg("workspace_id").required(true));
    let focus_agent = Command::new("focus-agent")
        .about("Focus herdr agent by pane id or label")
        .arg(str_arg("target").required(true));
    let read_pane = read_args(
        Command::new("read-pane")
            .about("Read herdr pane output (thin wrapper over herdr pane read)")
            .arg(str_arg("pane_id").required(true)),
        true,
    );
    let read_agent = read_args(
        Command::new("read-agent")
            .about("Read herdr agent output (thin wrapper over herdr agent read)")
            .arg(str_arg("target").required(true)),
        false,
    );
    let split = Command::new("split").about("Split current herdr pane").arg(
        opt("direction")
            .default_value("right")
            .value_parser(["right", "down"]),
    );
    let agents = Command::new("agents")
        .about("List herdr agents compactly")
        .arg(json_flag());
    let associations = Command::new("associations")
        .about("Show hybrid pane/session association cache (parent map + status keys)")
        .arg(json_flag());
    let lock_title = Command::new("lock-title")
        .about("Lock a pane display name (native-title lock; sync will not overwrite it)")
        .arg(str_arg("pane_id").required(true))
        .arg(opt("title"));
    let unlock_title = Command::new("unlock-title")
        .about("Clear the native-title lock so sync may update the display name")
        .arg(str_arg("pane_id").required(true));
    let clear = Command::new("clear")
        .about("Clear all cmux statuses with herdr: prefix")
        .arg(opt("workspace"));
    let json_dump = Command::new("json-dump").about("Raw snapshot for debugging");
    let send_key = Command::new("send-key")
        .about("Send a tmux-style named key (C-Up, F5, PPage) to one Herdr pane")
        .arg(str_arg("pane_id").required(true))
        .arg(str_arg("key").required(true));
    let observe = Command::new("observe")
        .about("remote.herdr.* observability (pane_surfaces / pane_grids / state)")
        .arg(opt("method").default_value("remote.herdr.pane_surfaces"))
        .arg(opt("socket"))
        .arg(opt("session").default_value("main"))
        .arg(json_flag());
    let attach = lifecycle_args(
        Command::new("attach")
            .about("Attach the live apply host (tmux remote.tmux.attach analogue)"),
    )
    .arg(bool_opt("no-activate"));
    let detach = lifecycle_args(
        Command::new("detach").about("Detach every live mirror; never stops the Herdr server"),
    );
    let restore = lifecycle_args(
        Command::new("restore").about("Reattach after restart (never replay a stale tree)"),
    );
    let api = Command::new("api")
        .about("Allowlisted Herdr socket RPC (CLI fallback; never server.stop)")
        .arg(str_arg("method"))
        .arg(opt("params"))
        .arg(bool_opt("list"))
        .arg(json_flag());
    let new_tab = Command::new("new-tab")
        .about("Create a Herdr tab (tab.create)")
        .arg(opt("label"))
        .arg(opt("workspace"))
        .arg(json_flag());
    let close_tab = Command::new("close-tab")
        .about("Close a Herdr tab (never stops Herdr)")
        .arg(str_arg("tab_id").required(true))
        .arg(json_flag());
    let rename_tab = Command::new("rename-tab")
        .about("Rename a Herdr tab")
        .arg(str_arg("tab_id").required(true))
        .arg(str_arg("label").required(true))
        .arg(json_flag());
    let new_workspace = Command::new("new-workspace")
        .about("Create a Herdr workspace")
        .arg(opt("label"))
        .arg(opt("cwd"))
        .arg(json_flag());
    let close_workspace = Command::new("close-workspace")
        .about("Close a Herdr workspace (never server.stop)")
        .arg(str_arg("workspace_id").required(true))
        .arg(json_flag());
    let rename_workspace = Command::new("rename-workspace")
        .about("Rename a Herdr workspace")
        .arg(str_arg("workspace_id").required(true))
        .arg(str_arg("label").required(true))
        .arg(json_flag());
    let close_pane = Command::new("close-pane")
        .about("Close a Herdr pane (busy panes require --force)")
        .arg(str_arg("pane_id").required(true))
        .arg(bool_opt("force"))
        .arg(json_flag());
    let zoom_pane = Command::new("zoom-pane")
        .about("Zoom or unzoom a Herdr pane")
        .arg(str_arg("pane_id"))
        .arg(
            opt("mode")
                .default_value("toggle")
                .value_parser(["toggle", "on", "off"]),
        )
        .arg(json_flag());
    let resize_pane = Command::new("resize-pane")
        .about("Resize a Herdr pane")
        .arg(str_arg("pane_id"))
        .arg(direction_arg("direction").long("direction").required(true))
        .arg(
            opt("amount")
                .default_value("0.1")
                .value_parser(clap::value_parser!(f64)),
        )
        .arg(json_flag());
    let swap_pane = Command::new("swap-pane")
        .about("Swap panes in the same Herdr tab")
        .arg(str_arg("pane_id"))
        .arg(direction_arg("direction").long("direction"))
        .arg(opt("target"))
        .arg(json_flag());
    let send = Command::new("send")
        .about("Send text to a Herdr pane (pane.send_text)")
        .arg(str_arg("pane_id").required(true))
        .arg(str_arg("text").required(true).num_args(1..))
        .arg(json_flag());
    let neighbor = Command::new("neighbor")
        .about("Look up a neighboring Herdr pane")
        .arg(str_arg("pane_id"))
        .arg(
            direction_arg("direction")
                .long("direction")
                .default_value("right"),
        )
        .arg(json_flag());
    let layout = Command::new("layout")
        .about("Export a Herdr tab layout tree")
        .arg(opt("tab"))
        .arg(opt("pane"))
        .arg(json_flag());
    let set_ratio = Command::new("set-ratio")
        .about("Set a tab split ratio (layout.set_split_ratio)")
        .arg(opt("tab"))
        .arg(
            opt("ratio")
                .required(true)
                .value_parser(clap::value_parser!(f64)),
        )
        .arg(opt("path"))
        .arg(json_flag());
    let move_pane = Command::new("move-pane")
        .about("Move a running pane to another tab")
        .arg(str_arg("pane_id").required(true))
        .arg(opt("tab"))
        .arg(bool_opt("new-tab"))
        .arg(opt("label"))
        .arg(opt("workspace"))
        .arg(
            opt("split")
                .default_value("right")
                .value_parser(["right", "down"]),
        )
        .arg(opt("target"))
        .arg(bool_opt("no-focus"))
        .arg(json_flag());
    let focus_dir = Command::new("focus-dir")
        .about("Focus a neighboring pane (pane.focus_direction)")
        .arg(direction_arg("direction").required(true))
        .arg(json_flag());
    let move_tab = Command::new("move-tab")
        .about("Move a Herdr tab (tab.move)")
        .arg(str_arg("tab_id").required(true))
        .arg(opt("index").value_parser(clap::value_parser!(i64)))
        .arg(opt("workspace"))
        .arg(json_flag());
    let rename_pane = Command::new("rename-pane")
        .about("Rename a Herdr pane (pane.rename)")
        .arg(str_arg("pane_id").required(true))
        .arg(str_arg("label").required(true))
        .arg(json_flag());
    let rename_agent = Command::new("rename-agent")
        .about("Rename a Herdr agent (agent.rename)")
        .arg(str_arg("target").required(true))
        .arg(str_arg("label").required(true))
        .arg(json_flag());
    let start_agent = timeout_args(
        Command::new("start-agent")
            .about("Start a named agent in an existing shell pane (agent.start)")
            .arg(str_arg("name").required(true))
            .arg(opt("kind"))
            .arg(opt("agent"))
            .arg(opt("pane").required(true)),
    )
    .arg(json_flag());
    let notify = Command::new("notify")
        .about("Show a Herdr notification (notification.show)")
        .arg(str_arg("title").required(true))
        .arg(opt("body"))
        .arg(json_flag());
    let wait_output = timeout_args(
        Command::new("wait-output")
            .about("Wait for pane output (pane.wait_for_output)")
            .arg(str_arg("pane_id").required(true))
            .arg(str_arg("pattern"))
            .arg(opt("match"))
            .arg(opt("regex")),
    )
    .arg(json_flag());
    let agent_prompt = timeout_args(
        Command::new("agent-prompt")
            .about("Submit an agent prompt (optional --wait / --until)")
            .arg(str_arg("target").required(true))
            .arg(str_arg("prompt").required(true).num_args(1..))
            .arg(bool_opt("wait"))
            .arg(opt("until")),
    )
    .arg(json_flag());
    let agent_wait = timeout_args(
        Command::new("agent-wait")
            .about("Wait for semantic agent state")
            .arg(str_arg("target").required(true))
            .arg(opt("until")),
    )
    .arg(json_flag());
    let agent_explain = Command::new("agent-explain")
        .about("Explain agent/pane state (agent.explain; Herdr-only)")
        .arg(str_arg("target").required(true))
        .arg(json_flag());
    let agent_view = Command::new("agent-view")
        .about("Set/clear agent view (agent.view.*; Herdr-only)")
        .arg(str_arg("target").required(true))
        .arg(str_arg("view"))
        .arg(bool_opt("clear"))
        .arg(json_flag());
    let process_info = Command::new("process-info")
        .about("Show pane process info (pane.process_info; Herdr-only)")
        .arg(str_arg("pane_id").required(true))
        .arg(json_flag());
    let release_agent = Command::new("release-agent")
        .about("Release agent on a pane (pane.release_agent; Herdr-only)")
        .arg(str_arg("pane_id").required(true))
        .arg(json_flag());
    let clear_agent_authority = Command::new("clear-agent-authority")
        .about("Clear pane agent authority (pane.clear_agent_authority)")
        .arg(str_arg("pane_id").required(true))
        .arg(json_flag());
    let window_title = Command::new("window-title")
        .about("Set/clear Herdr client window title (client.window_title.*)")
        .arg(str_arg("title"))
        .arg(bool_opt("clear"))
        .arg(json_flag());
    let layout_apply = Command::new("layout-apply")
        .about("Apply a layout tree JSON (layout.apply)")
        .arg(opt("tree").required(true))
        .arg(opt("tab"))
        .arg(json_flag());
    let manifests = Command::new("manifests")
        .about("List/reload Herdr agent manifests (server.agent_manifests)")
        .arg(bool_opt("reload"))
        .arg(json_flag());
    let worktree = Command::new("worktree")
        .about("Herdr worktree list/create/open/remove (no tmux analogue)")
        .arg(
            str_arg("action")
                .required(true)
                .value_parser(["list", "create", "open", "remove"]),
        )
        .arg(str_arg("target"))
        .arg(opt("path"))
        .arg(opt("name"))
        .arg(json_flag());
    let workspace_move = Command::new("workspace-move")
        .about("Move a workspace or block (workspace.move / move_block)")
        .arg(str_arg("workspace_id").required(true))
        .arg(opt("index").value_parser(clap::value_parser!(i64)))
        .arg(opt("block"))
        .arg(json_flag());
    let sidebar = Command::new("sidebar")
        .about("Run the native cmux sidebar plugin TUI")
        .arg(bool_opt("once"));
    let update_install = Command::new("install")
        .arg(opt("manifest-url").required(true))
        .arg(opt("channel").default_value("preview"))
        .arg(opt("herdr"))
        .arg(json_flag());
    let update_uninstall = Command::new("uninstall").arg(opt("herdr")).arg(json_flag());
    let update_run = Command::new("run").arg(opt("herdr")).arg(json_flag());
    let update_status = Command::new("status").arg(opt("herdr")).arg(json_flag());
    let update_service = Command::new("update-service")
        .about("Manage the transactional Herdr auto-update service")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommands([update_install, update_uninstall, update_run, update_status]);

    Command::new("cmux-herdr")
        .about("cmux plugin for Herdr — cmux is the official UI, with status pills, tab/pane mirroring, and inner-mux control.")
        .version(VERSION.as_str()).subcommand_required(true).arg_required_else_help(true)
        .subcommands([status, doctor, lease, tree, sync, watch, mirror, attach_pane,
            focus_tab, focus_pane, focus_workspace, focus_agent, read_pane, read_agent,
            split, agents, associations, lock_title, unlock_title, clear, json_dump,
            send_key, observe, attach, detach, restore, api, new_tab, close_tab,
            rename_tab, new_workspace, close_workspace, rename_workspace, close_pane,
            zoom_pane, resize_pane, swap_pane, send, neighbor, layout, set_ratio,
            move_pane, focus_dir, move_tab, rename_pane, rename_agent, start_agent,
            notify, wait_output, agent_prompt, agent_wait, agent_explain, agent_view,
            process_info, release_agent, clear_agent_authority, window_title,
            layout_apply, manifests, worktree, workspace_move, sidebar, update_service])
}

fn s<'a>(m: &'a ArgMatches, key: &str) -> Option<&'a str> {
    m.get_one::<String>(key).map(String::as_str)
}
fn b(m: &ArgMatches, key: &str) -> bool {
    m.get_flag(key)
}
fn strings(m: &ArgMatches, key: &str) -> Vec<String> {
    m.get_many::<String>(key)
        .map(|v| v.cloned().collect())
        .unwrap_or_default()
}
fn timeout_ms(m: &ArgMatches) -> Option<i64> {
    m.get_one::<i64>("timeout")
        .copied()
        .or_else(|| m.get_one::<i64>("timeout-ms").copied())
}
fn insert_str(map: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|v| !v.is_empty()) {
        map.insert(key.into(), Value::String(value.into()));
    }
}
fn pretty(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("JSON serialization")
    );
}
fn die(message: impl std::fmt::Display) -> i32 {
    eprintln!("cmux-herdr: {message}");
    1
}
fn ensure_herdr() -> Result<(), i32> {
    if bridge::herdr_available() {
        Ok(())
    } else {
        Err(die("herdr not available (HERDR_ENV unset and socket/CLI unhealthy). Run inside a herdr pane or start herdr."))
    }
}
fn call_api(method: &str, params: Value) -> Result<ApiOutcome, ApiError> {
    let runner = ProcessCliRunner;
    let mut api = HerdrApi::new(None, API_TIMEOUT).with_cli_runner(&runner);
    api.call(method, params, false)
}
fn print_api(outcome: ApiOutcome, json_output: bool, line: Option<String>) -> i32 {
    if let Some(line) = line.as_deref().filter(|_| !json_output) {
        println!("{line}");
    }
    if json_output || line.is_none() {
        pretty(&outcome.to_json());
    }
    if outcome.ok {
        0
    } else {
        2
    }
}
fn run_api(m: &ArgMatches, method: &str, params: Value, line: Option<String>) -> i32 {
    match call_api(method, params) {
        Ok(outcome) => print_api(outcome, b(m, "json"), line),
        Err(error) => die(error),
    }
}

fn generic_dispatch(name: &str, m: &ArgMatches) -> Option<i32> {
    let mut p = Map::new();
    let (method, line): (&str, Option<String>) = match name {
        "new-tab" => {
            insert_str(&mut p, "label", s(m, "label"));
            insert_str(&mut p, "workspace_id", s(m, "workspace"));
            ("tab.create", Some("created tab".into()))
        }
        "close-tab" => {
            let id = s(m, "tab_id").unwrap();
            insert_str(&mut p, "tab_id", Some(id));
            ("tab.close", Some(format!("closed tab {id}")))
        }
        "rename-tab" => {
            let id = s(m, "tab_id").unwrap();
            insert_str(&mut p, "tab_id", Some(id));
            insert_str(&mut p, "label", s(m, "label"));
            ("tab.rename", Some(format!("renamed tab {id}")))
        }
        "new-workspace" => {
            insert_str(&mut p, "label", s(m, "label"));
            insert_str(&mut p, "cwd", s(m, "cwd"));
            ("workspace.create", Some("created workspace".into()))
        }
        "close-workspace" => {
            let id = s(m, "workspace_id").unwrap();
            insert_str(&mut p, "workspace_id", Some(id));
            ("workspace.close", Some(format!("closed workspace {id}")))
        }
        "rename-workspace" => {
            let id = s(m, "workspace_id").unwrap();
            insert_str(&mut p, "workspace_id", Some(id));
            insert_str(&mut p, "label", s(m, "label"));
            ("workspace.rename", Some(format!("renamed workspace {id}")))
        }
        "zoom-pane" => {
            insert_str(&mut p, "pane_id", s(m, "pane_id"));
            insert_str(&mut p, "mode", s(m, "mode"));
            ("pane.zoom", Some(format!("zoom {}", s(m, "mode").unwrap())))
        }
        "resize-pane" => {
            insert_str(&mut p, "pane_id", s(m, "pane_id"));
            insert_str(&mut p, "direction", s(m, "direction"));
            p.insert("amount".into(), json!(*m.get_one::<f64>("amount").unwrap()));
            ("pane.resize", Some("resized pane".into()))
        }
        "swap-pane" => {
            insert_str(&mut p, "pane_id", s(m, "pane_id"));
            if let Some(target) = s(m, "target") {
                insert_str(&mut p, "source_pane_id", s(m, "pane_id"));
                insert_str(&mut p, "target_pane_id", Some(target));
            }
            insert_str(&mut p, "direction", s(m, "direction"));
            ("pane.swap", Some("swapped pane".into()))
        }
        "send" => {
            let id = s(m, "pane_id").unwrap();
            insert_str(&mut p, "pane_id", Some(id));
            p.insert("text".into(), Value::String(strings(m, "text").join(" ")));
            ("pane.send_text", Some(format!("sent text to {id}")))
        }
        "neighbor" => {
            insert_str(&mut p, "pane_id", s(m, "pane_id"));
            insert_str(&mut p, "direction", s(m, "direction"));
            ("pane.neighbor", None)
        }
        "layout" => {
            insert_str(&mut p, "tab_id", s(m, "tab"));
            insert_str(&mut p, "pane_id", s(m, "pane"));
            ("layout.export", None)
        }
        "focus-dir" => {
            let d = s(m, "direction").unwrap();
            insert_str(&mut p, "direction", Some(d));
            ("pane.focus_direction", Some(format!("focus {d}")))
        }
        "move-tab" => {
            let id = s(m, "tab_id").unwrap();
            insert_str(&mut p, "tab_id", Some(id));
            if let Some(i) = m.get_one::<i64>("index") {
                p.insert("index".into(), json!(i));
            }
            insert_str(&mut p, "workspace_id", s(m, "workspace"));
            ("tab.move", Some(format!("moved tab {id}")))
        }
        "rename-pane" => {
            let id = s(m, "pane_id").unwrap();
            insert_str(&mut p, "pane_id", Some(id));
            insert_str(&mut p, "label", s(m, "label"));
            ("pane.rename", Some(format!("renamed pane {id}")))
        }
        "rename-agent" => {
            let target = s(m, "target").unwrap();
            insert_str(&mut p, "pane_id", Some(target));
            insert_str(&mut p, "target", Some(target));
            insert_str(&mut p, "label", s(m, "label"));
            ("agent.rename", Some(format!("renamed agent {target}")))
        }
        "notify" => {
            insert_str(&mut p, "title", s(m, "title"));
            insert_str(&mut p, "body", s(m, "body"));
            ("notification.show", Some("notified".into()))
        }
        "agent-explain" => {
            let target = s(m, "target").unwrap();
            insert_str(&mut p, "target", Some(target));
            insert_str(&mut p, "pane_id", Some(target));
            ("agent.explain", Some(format!("explain {target}")))
        }
        "process-info" => {
            let id = s(m, "pane_id").unwrap();
            insert_str(&mut p, "pane_id", Some(id));
            ("pane.process_info", Some(format!("process {id}")))
        }
        "release-agent" => {
            let id = s(m, "pane_id").unwrap();
            insert_str(&mut p, "pane_id", Some(id));
            ("pane.release_agent", Some(format!("released agent {id}")))
        }
        "clear-agent-authority" => {
            let id = s(m, "pane_id").unwrap();
            insert_str(&mut p, "pane_id", Some(id));
            (
                "pane.clear_agent_authority",
                Some(format!("cleared authority {id}")),
            )
        }
        _ => return None,
    };
    Some(run_api(m, method, Value::Object(p), line))
}

fn dispatch(name: &str, m: &ArgMatches) -> i32 {
    if let Some(code) = generic_dispatch(name, m) {
        return code;
    }
    match name {
        "api" => cmd_api(m),
        "set-ratio" => cmd_set_ratio(m),
        "move-pane" => cmd_move_pane(m),
        "start-agent" => cmd_start_agent(m),
        "wait-output" => cmd_wait_output(m),
        "agent-prompt" => cmd_agent_prompt(m),
        "agent-wait" => cmd_agent_wait(m),
        "agent-view" => cmd_agent_view(m),
        "window-title" => cmd_window_title(m),
        "layout-apply" => cmd_layout_apply(m),
        "manifests" => cmd_manifests(m),
        "worktree" => cmd_worktree(m),
        "workspace-move" => cmd_workspace_move(m),
        "close-pane" => cmd_close_pane(m),
        "focus-tab" => cmd_focus_tab(m),
        "focus-pane" => cmd_focus_target(m, "pane"),
        "focus-workspace" => cmd_focus_target(m, "workspace"),
        "focus-agent" => cmd_focus_target(m, "agent"),
        "read-pane" => cmd_read(m, false),
        "read-agent" => cmd_read(m, true),
        "split" => cmd_split(m),
        "agents" => cmd_agents(m),
        "tree" => cmd_tree(m),
        "json-dump" => cmd_json_dump(),
        "associations" => cmd_associations(m),
        "lock-title" => cmd_title_lock(m, true),
        "unlock-title" => cmd_title_lock(m, false),
        "clear" => cmd_clear(m),
        "sync" => cmd_sync(m),
        "status" => cmd_status(m),
        "doctor" => cmd_doctor(m),
        "lease" => cmd_lease(m),
        "send-key" => cmd_send_key(m),
        "mirror" => cmd_mirror(m),
        "watch" => cmd_watch(m),
        "attach-pane" => cmd_attach_pane(m),
        "attach" | "detach" | "restore" | "observe" => cmd_live(name, m),
        "sidebar" => {
            let args = if b(m, "once") {
                vec!["--once".into()]
            } else {
                vec![]
            };
            crate::sidebar::main(&args)
        }
        "update-service" => cmd_update_service(m),
        _ => unreachable!("clap produced an unregistered subcommand: {name}"),
    }
}

/// Parse and dispatch process arguments, returning the exact process exit code.
pub fn run() -> i32 {
    run_from(std::env::args_os())
}

pub fn run_from<I, T>(args: I) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match build_parser().try_get_matches_from(args) {
        Ok(matches) => {
            let (name, sub) = matches.subcommand().expect("required subcommand");
            dispatch(name, sub)
        }
        Err(error) => {
            let code = if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                0
            } else {
                2
            };
            let _ = error.print();
            code
        }
    }
}

fn cmd_api(m: &ArgMatches) -> i32 {
    if b(m, "list") {
        for method in api::allowed_methods() {
            println!("{method}");
        }
        return 0;
    }
    let Some(method) = s(m, "method") else {
        return die("api: pass a method or --list");
    };
    let params = match s(m, "params") {
        None => json!({}),
        Some(raw) => match serde_json::from_str::<Value>(raw) {
            Ok(Value::Null) => json!({}),
            Ok(v @ Value::Object(_)) => v,
            Ok(_) => return die("--params must be a JSON object"),
            Err(e) => return die(format!("invalid --params JSON: {e}")),
        },
    };
    run_api(m, method, params, None)
}
fn cmd_set_ratio(m: &ArgMatches) -> i32 {
    let path = match s(m, "path") {
        None => json!([]),
        Some(raw) => match serde_json::from_str::<Value>(raw) {
            Ok(v @ Value::Array(_)) => v,
            Ok(_) => return die("--path must be a JSON array of hop indexes"),
            Err(e) => return die(format!("invalid --path JSON: {e}")),
        },
    };
    let mut p = json!({"ratio":*m.get_one::<f64>("ratio").unwrap(),"path":path});
    if let Some(tab) = s(m, "tab") {
        p["tab_id"] = json!(tab)
    }
    run_api(
        m,
        "layout.set_split_ratio",
        p,
        Some("set split ratio".into()),
    )
}
fn cmd_move_pane(m: &ArgMatches) -> i32 {
    let id = s(m, "pane_id").unwrap();
    let dest = if b(m, "new-tab") {
        let mut d = json!({"type":"new_tab","label":s(m,"label").unwrap_or("moved")});
        if let Some(ws) = s(m, "workspace") {
            d["workspace_id"] = json!(ws)
        }
        d
    } else if let Some(tab) = s(m, "tab") {
        let mut d = json!({"type":"tab","tab_id":tab,"split":s(m,"split").unwrap()});
        if let Some(t) = s(m, "target") {
            d["target_pane_id"] = json!(t)
        }
        d
    } else {
        return die("move-pane: pass --tab or --new-tab");
    };
    run_api(
        m,
        "pane.move",
        json!({"pane_id":id,"destination":dest,"focus":!b(m,"no-focus")}),
        Some(format!("moved {id}")),
    )
}
fn cmd_start_agent(m: &ArgMatches) -> i32 {
    let Some(kind) = s(m, "kind").or_else(|| s(m, "agent")) else {
        return die("start-agent: --kind is required (Herdr agent kind, e.g. codex)");
    };
    let mut p = json!({"name":s(m,"name").unwrap(),"kind":kind,"pane_id":s(m,"pane").unwrap(),"target":s(m,"pane").unwrap()});
    if let Some(t) = timeout_ms(m) {
        p["timeout_ms"] = json!(t)
    }
    run_api(
        m,
        "agent.start",
        p,
        Some(format!(
            "started {} on {}",
            s(m, "name").unwrap(),
            s(m, "pane").unwrap()
        )),
    )
}
fn cmd_wait_output(m: &ArgMatches) -> i32 {
    let literal = s(m, "match").or_else(|| s(m, "pattern"));
    let regex = s(m, "regex");
    if literal.is_none() && regex.is_none() {
        return die("wait-output: pass --match TEXT or --regex PATTERN");
    };
    let mut p = json!({"pane_id":s(m,"pane_id").unwrap()});
    if let Some(r) = regex {
        p["regex"] = json!(r)
    } else {
        p["pattern"] = json!(literal.unwrap());
        p["match"] = json!(literal.unwrap())
    }
    if let Some(t) = timeout_ms(m) {
        p["timeout_ms"] = json!(t)
    }
    run_api(
        m,
        "pane.wait_for_output",
        p,
        Some(format!("wait output {}", s(m, "pane_id").unwrap())),
    )
}
fn cmd_agent_prompt(m: &ArgMatches) -> i32 {
    let target = s(m, "target").unwrap();
    let mut p = json!({"target":target,"pane_id":target,"prompt":strings(m,"prompt").join(" ")});
    let t = timeout_ms(m);
    if b(m, "wait") || s(m, "until").is_some() {
        let mut w = Map::new();
        if let Some(u) = s(m, "until") {
            w.insert("until".into(), json!(u));
            p["until"] = json!(u)
        }
        if let Some(ms) = t {
            w.insert("timeout_ms".into(), json!(ms));
        }
        p["wait"] = if w.is_empty() {
            Value::Bool(true)
        } else {
            Value::Object(w)
        }
    }
    if let Some(ms) = t {
        p["timeout_ms"] = json!(ms)
    }
    run_api(m, "agent.prompt", p, Some(format!("prompted {target}")))
}
fn cmd_agent_wait(m: &ArgMatches) -> i32 {
    let target = s(m, "target").unwrap();
    let mut p = json!({"target":target,"pane_id":target});
    if let Some(u) = s(m, "until") {
        p["until"] = json!(u)
    }
    if let Some(t) = timeout_ms(m) {
        p["timeout_ms"] = json!(t)
    }
    run_api(
        m,
        "agent.wait",
        p,
        Some(format!(
            "wait {} {target}",
            s(m, "until").unwrap_or("settled")
        )),
    )
}
fn cmd_agent_view(m: &ArgMatches) -> i32 {
    let target = s(m, "target").unwrap();
    let mut p = json!({"target":target,"pane_id":target});
    if b(m, "clear") {
        return run_api(
            m,
            "agent.view.clear",
            p,
            Some(format!("cleared view {target}")),
        );
    }
    let Some(view) = s(m, "view") else {
        return die("agent-view: pass VIEW or --clear");
    };
    p["view"] = json!(view);
    run_api(
        m,
        "agent.view.set",
        p,
        Some(format!("view {target}={view}")),
    )
}
fn cmd_window_title(m: &ArgMatches) -> i32 {
    if b(m, "clear") {
        return run_api(
            m,
            "client.window_title.clear",
            json!({}),
            Some("cleared window title".into()),
        );
    }
    let Some(title) = s(m, "title") else {
        return die("window-title: pass TITLE or --clear");
    };
    run_api(
        m,
        "client.window_title.set",
        json!({"title":title}),
        Some("set window title".into()),
    )
}
fn cmd_layout_apply(m: &ArgMatches) -> i32 {
    let tree = match serde_json::from_str::<Value>(s(m, "tree").unwrap()) {
        Ok(v) => v,
        Err(e) => return die(format!("invalid --tree JSON: {e}")),
    };
    let mut p = json!({"layout":tree,"tree":tree});
    if let Some(tab) = s(m, "tab") {
        p["tab_id"] = json!(tab)
    }
    run_api(m, "layout.apply", p, Some("applied layout".into()))
}
fn cmd_manifests(m: &ArgMatches) -> i32 {
    if b(m, "reload") {
        run_api(
            m,
            "server.reload_agent_manifests",
            json!({}),
            Some("reloaded agent manifests".into()),
        )
    } else {
        run_api(
            m,
            "server.agent_manifests",
            json!({}),
            Some("agent manifests".into()),
        )
    }
}
fn cmd_worktree(m: &ArgMatches) -> i32 {
    let action = s(m, "action").unwrap();
    let (method, p, line) = match action {
        "list" => ("worktree.list", json!({}), "worktrees".into()),
        "create" => {
            let mut p = json!({});
            if let Some(v) = s(m, "path") {
                p["path"] = json!(v)
            }
            if let Some(v) = s(m, "name") {
                p["name"] = json!(v)
            }
            ("worktree.create", p, "created worktree".into())
        }
        "open" | "remove" => {
            let Some(t) = s(m, "target") else {
                return die(format!("worktree {action}: pass TARGET id/path"));
            };
            (
                if action == "open" {
                    "worktree.open"
                } else {
                    "worktree.remove"
                },
                json!({"id":t,"path":t}),
                format!(
                    "{} worktree {t}",
                    if action == "open" {
                        "opened"
                    } else {
                        "removed"
                    }
                ),
            )
        }
        _ => unreachable!(),
    };
    run_api(m, method, p, Some(line))
}
fn cmd_workspace_move(m: &ArgMatches) -> i32 {
    let id = s(m, "workspace_id").unwrap();
    let (method, mut p, line) = if let Some(block) = s(m, "block") {
        (
            "workspace.move_block",
            json!({"workspace_id":id,"block":block}),
            format!("moved block {id}"),
        )
    } else {
        (
            "workspace.move",
            json!({"workspace_id":id}),
            format!("moved workspace {id}"),
        )
    };
    if let Some(i) = m.get_one::<i64>("index") {
        p["index"] = json!(i)
    }
    run_api(m, method, p, Some(line))
}

fn cmd_close_pane(m: &ArgMatches) -> i32 {
    if let Err(code) = ensure_herdr() {
        return code;
    }
    let pane_id = s(m, "pane_id").unwrap();
    if !b(m, "force") {
        match call_api("pane.get", json!({"pane_id": pane_id})) {
            Ok(outcome) => {
                if let Some(status) = api::extract_agent_status(&outcome.result) {
                    let intent =
                        crate::control::close_intent("user_pane", Some(pane_id), Some(&status));
                    if intent.action == "confirm_then_close_pane" {
                        return die(format!(
                            "pane {pane_id} is busy (agent_status={status}); pass --force to close"
                        ));
                    }
                }
            }
            Err(error) => return die(error),
        }
    }
    run_api(
        m,
        "pane.close",
        json!({"pane_id": pane_id}),
        Some(format!("closed pane {pane_id}")),
    )
}

fn cmd_focus_tab(_m: &ArgMatches) -> i32 {
    if let Err(code) = ensure_herdr() {
        return code;
    }
    let target = s(_m, "target").unwrap();
    let tabs = match bridge::fetch_tabs() {
        Ok(value) => value,
        Err(error) => return die(error),
    };
    let Some(tab) = tabs
        .iter()
        .find(|tab| tab.tab_id == target || tab.label.as_deref() == Some(target))
    else {
        return die(format!("tab not found: {target}"));
    };
    let tab_id = tab.tab_id.clone();
    match call_api("tab.focus", json!({"tab_id": tab_id})) {
        Ok(_) => {
            println!("focused tab {tab_id}");
            0
        }
        Err(error) => die(error),
    }
}

fn cmd_focus_target(m: &ArgMatches, kind: &str) -> i32 {
    if let Err(code) = ensure_herdr() {
        return code;
    }
    let (target, method, params) = match kind {
        "workspace" => {
            let value = s(m, "workspace_id").unwrap();
            (value, "workspace.focus", json!({"workspace_id":value}))
        }
        "pane" => {
            let value = s(m, "pane_id").unwrap();
            (
                value,
                "agent.focus",
                json!({"target":value,"pane_id":value}),
            )
        }
        _ => {
            let value = s(m, "target").unwrap();
            (
                value,
                "agent.focus",
                json!({"target":value,"pane_id":value}),
            )
        }
    };
    match call_api(method, params) {
        Ok(_) => {
            println!("focused {kind} {target}");
            0
        }
        Err(error) => die(error),
    }
}

fn cmd_read(m: &ArgMatches, agent: bool) -> i32 {
    if let Err(code) = ensure_herdr() {
        return code;
    }
    let target = s(m, if agent { "target" } else { "pane_id" }).unwrap();
    let mut params = json!({"pane_id":target,"target":target});
    for key in ["source", "format"] {
        if let Some(value) = s(m, key) {
            params[key] = json!(value);
        }
    }
    if let Some(lines) = m.get_one::<i64>("lines") {
        params["lines"] = json!(lines);
    }
    if b(m, "ansi") {
        params["ansi"] = json!(true);
    }
    if !agent && b(m, "raw") {
        params["raw"] = json!(true);
    }
    let method = if agent { "agent.read" } else { "pane.read" };
    match call_api(method, params) {
        Ok(outcome) => {
            let text = api::extract_read_text(&outcome.result);
            print!("{text}");
            if !text.ends_with('\n') {
                println!();
            }
            0
        }
        Err(error) => die(error),
    }
}

fn cmd_split(m: &ArgMatches) -> i32 {
    if let Err(code) = ensure_herdr() {
        return code;
    }
    match call_api("pane.split", json!({"direction":s(m,"direction").unwrap()})) {
        Ok(outcome) => {
            if outcome.result.is_object() || outcome.result.is_array() {
                pretty(&outcome.result)
            } else {
                println!("{}", plain(&outcome.result))
            }
            0
        }
        Err(error) => die(error),
    }
}

fn plain(value: &Value) -> String {
    match value {
        Value::String(v) => v.clone(),
        Value::Null => "None".into(),
        Value::Bool(v) => {
            if *v {
                "True".into()
            } else {
                "False".into()
            }
        }
        _ => value.to_string(),
    }
}

fn fetch_snapshot() -> Result<crate::model::Snapshot, BridgeError> {
    let mut api = bridge::herdr_api();
    bridge::fetch_snapshot(&mut api)
}

fn cmd_agents(m: &ArgMatches) -> i32 {
    if let Err(code) = ensure_herdr() {
        return code;
    }
    let agents = match bridge::fetch_agents() {
        Ok(v) => v,
        Err(e) => return die(e),
    };
    if b(m, "json") {
        pretty(&Value::Array(agents.into_iter().map(|a| a.raw).collect()));
        return 0;
    }
    if agents.is_empty() {
        println!("(no agents)");
        return 0;
    }
    for a in agents {
        let mark = if a.focused { "▶" } else { " " };
        let label = a.label.as_deref().unwrap_or("");
        let tail = if label.is_empty() {
            a.terminal_title
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(40)
                .collect()
        } else {
            label.to_string()
        };
        println!(
            "{mark} {:10}  {:8}  {:10}  {:8}  {}",
            a.pane_id,
            a.tab_id,
            a.agent.as_deref().unwrap_or("-"),
            a.agent_status,
            tail
        );
    }
    0
}

fn cmd_tree(m: &ArgMatches) -> i32 {
    if let Err(code) = ensure_herdr() {
        return code;
    }
    let snap = match fetch_snapshot() {
        Ok(v) => v,
        Err(e) => return die(e),
    };
    if b(m, "json") {
        pretty(
            &json!({"workspaces":snap.workspaces.iter().map(|w|w.raw.clone()).collect::<Vec<_>>(),"tabs":snap.tabs.iter().map(|t|t.raw.clone()).collect::<Vec<_>>(),"panes":snap.panes.iter().map(|p|p.raw.clone()).collect::<Vec<_>>() }),
        );
    } else {
        println!("{}", format_tree(&snap));
    }
    0
}

fn format_tree(snap: &crate::model::Snapshot) -> String {
    let mut panes_by_tab: HashMap<&str, Vec<&crate::model::Pane>> = HashMap::new();
    for pane in &snap.panes {
        panes_by_tab.entry(&pane.tab_id).or_default().push(pane);
    }
    let mut tabs_by_ws: HashMap<&str, Vec<&crate::model::Tab>> = HashMap::new();
    for tab in &snap.tabs {
        tabs_by_ws.entry(&tab.workspace_id).or_default().push(tab);
    }
    let mut lines = Vec::new();
    for ws in &snap.workspaces {
        let mark = if ws.focused { "●" } else { "○" };
        let tabs = tabs_by_ws
            .get(ws.workspace_id.as_str())
            .cloned()
            .unwrap_or_default();
        lines.push(format!(
            "{mark} workspace {}  {}  [{}] tabs={} panes={}",
            ws.workspace_id,
            ws.label.as_deref().unwrap_or(&ws.workspace_id),
            ws.agent_status,
            if ws.tab_count != 0 {
                ws.tab_count
            } else {
                tabs.len() as i64
            },
            ws.pane_count
        ));
        let mut tabs = tabs;
        tabs.sort_by_key(|tab| {
            (
                tab.number.is_none(),
                tab.number.unwrap_or(0),
                tab.tab_id.clone(),
            )
        });
        let mut seen = HashSet::new();
        for tab in tabs {
            if !seen.insert(&tab.tab_id) {
                continue;
            }
            let panes = panes_by_tab
                .get(tab.tab_id.as_str())
                .cloned()
                .unwrap_or_default();
            let mark = if tab.focused { "▶" } else { "·" };
            lines.push(format!(
                "  {mark} tab {}  {}  [{}] panes={}",
                tab.tab_id,
                tab.label.as_deref().unwrap_or(&tab.tab_id),
                tab.agent_status,
                if tab.pane_count != 0 {
                    tab.pane_count
                } else {
                    panes.len() as i64
                }
            ));
            let mut panes = panes;
            panes.sort_by_key(|pane| (!pane.focused, pane.pane_id.clone()));
            for pane in panes {
                let mark = if pane.focused { "▶" } else { "·" };
                lines.push(format!(
                    "    {mark} pane {}  {}  [{}]  {}",
                    pane.pane_id,
                    pane.label.as_deref().unwrap_or(&pane.pane_id),
                    pane.agent_status,
                    pane.agent.as_deref().unwrap_or("-")
                ));
            }
        }
    }
    if lines.is_empty() {
        lines.push("(no herdr workspaces/panes)".into())
    }
    lines.join("\n")
}

fn cmd_json_dump() -> i32 {
    if let Err(code) = ensure_herdr() {
        return code;
    }
    let snap = match fetch_snapshot() {
        Ok(v) => v,
        Err(e) => return die(e),
    };
    let fetched_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    let mut env = Map::new();
    for key in [
        "HERDR_ENV",
        "HERDR_PANE_ID",
        "HERDR_TAB_ID",
        "HERDR_WORKSPACE_ID",
        "HERDR_SOCKET_PATH",
        "CMUX_SURFACE_ID",
        "CMUX_TAB_ID",
        "CMUX_WORKSPACE_ID",
        "CMUX_SOCKET_PATH",
    ] {
        env.insert(
            key.into(),
            std::env::var(key).map(Value::String).unwrap_or(Value::Null),
        );
    }
    pretty(
        &json!({"fetched_at":fetched_at,"env":env,"cmux_resolved_workspace":resolve_workspace(None),"workspaces":snap.workspaces.iter().map(|w|w.raw.clone()).collect::<Vec<_>>(),"tabs":snap.tabs.iter().map(|t|t.raw.clone()).collect::<Vec<_>>(),"panes":snap.panes.iter().map(|p|p.raw.clone()).collect::<Vec<_>>() }),
    );
    0
}

fn cmd_associations(m: &ArgMatches) -> i32 {
    let fp = state::collect_host_fingerprint(&SystemEnv);
    let data = state::load_association_map(&SystemEnv, &fp);
    if b(m, "json") {
        pretty(&data)
    } else {
        println!("{}", state::format_associations(&SystemEnv, Some(&data)))
    }
    0
}
fn cmd_title_lock(m: &ArgMatches, locked: bool) -> i32 {
    let id = s(m, "pane_id").unwrap();
    match state::set_title_lock(&SystemEnv, id, locked, s(m, "title")) {
        Ok(entry) => {
            if locked {
                println!(
                    "locked title for {}: {}",
                    entry.get("pane_id").map(plain).unwrap_or_default(),
                    entry
                        .get("locked_title")
                        .filter(|v| !v.is_null())
                        .map(plain)
                        .unwrap_or_else(|| "-".into())
                )
            } else {
                println!(
                    "unlocked title for {}",
                    entry.get("pane_id").map(plain).unwrap_or_default()
                )
            }
            0
        }
        Err(e) => die(e),
    }
}

fn resolve_workspace(explicit: Option<&str>) -> Option<String> {
    crate::mirror::resolve_cmux_workspace(explicit)
}
fn list_status_keys(workspace: &str) -> Vec<String> {
    bridge::cmux_cmd(&["list-status"], Some(workspace))
        .ok()
        .filter(|p| p.returncode == 0)
        .map(|p| {
            p.stdout
                .lines()
                .filter_map(|line| {
                    line.split_once('=')
                        .map(|x| x.0.trim())
                        .filter(|k| k.starts_with("herdr:"))
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}
fn cmd_clear(m: &ArgMatches) -> i32 {
    let Some(ws) = resolve_workspace(s(m, "workspace")) else {
        return die("no cmux workspace resolved; is cmux running?");
    };
    let mut cleared = Vec::new();
    for key in list_status_keys(&ws) {
        if bridge::cmux_cmd(&["clear-status", &key], Some(&ws)).is_ok_and(|p| p.returncode == 0) {
            cleared.push(key)
        }
    }
    if cleared.is_empty() {
        println!("no herdr:* status keys to clear")
    } else {
        println!("cleared {} status keys:", cleared.len());
        for k in cleared {
            println!("  {k}")
        }
    }
    0
}

fn cmd_sync(m: &ArgMatches) -> i32 {
    if let Err(code) = ensure_herdr() {
        return code;
    }
    let snap = match fetch_snapshot() {
        Ok(value) => value,
        Err(error) => return die(error),
    };
    let Some(workspace) = resolve_workspace(s(m, "workspace")) else {
        return die("could not resolve cmux workspace for status sync (need CMUX_SURFACE_ID + HERDR_SOCKET_PATH, or pass --workspace)");
    };
    let fingerprint = state::collect_host_fingerprint(&SystemEnv);
    let fingerprint_key = state::parent_key(&fingerprint);
    let writer = crate::handoff::writer_status(&fingerprint_key);
    if writer["native_live"].as_bool().unwrap_or(false) {
        let associations = state::update_association_map(&SystemEnv, &snap, Some(&workspace), None)
            .unwrap_or(Value::Null);
        let summary=format!("herdr sync: skipped (native attachment live) ws={workspace} fingerprint={fingerprint_key}");
        if !b(m, "no-log") {
            let _ = bridge::cmux_cmd(&["log", &summary], Some(&workspace));
        }
        let result = json!({"workspace":workspace,"applied":[],"skipped_unchanged":[],"stale_cleared":[],"counts":{"working":0,"idle":0,"done":0,"blocked":0,"unknown":0,"other":0},"progress":null,"errors":[],"summary":summary,"pane_count":snap.panes.len(),"agent_count":0,"associations":associations,"host_fingerprint_key":fingerprint_key,"writer":writer["writer"],"native_live":true,"skipped_reason":"native_live"});
        println!("{}", result["summary"].as_str().unwrap());
        if b(m, "json") {
            pretty(&result)
        }
        return 0;
    }
    let tabs: HashMap<String, crate::model::Tab> = snap
        .tabs
        .iter()
        .map(|tab| (tab.tab_id.clone(), tab.clone()))
        .collect();
    let prior_state = state::load_association_map(&SystemEnv, &fingerprint);
    let previous = prior_state.get("panes").unwrap_or(&Value::Null);
    let mut panes: Vec<_> = snap
        .panes
        .iter()
        .filter(|pane| pane.agent.as_ref().is_some_and(|agent| !agent.is_empty()))
        .collect();
    if panes.is_empty() {
        panes = snap
            .panes
            .iter()
            .filter(|pane| {
                matches!(
                    pane.agent_status.as_str(),
                    "working" | "idle" | "done" | "blocked"
                )
            })
            .collect()
    }
    let mut counts = Map::new();
    for key in ["working", "idle", "done", "blocked", "unknown", "other"] {
        counts.insert(key.into(), json!(0));
    }
    let mut applied = Vec::new();
    let mut skipped = Vec::new();
    let mut errors = Vec::new();
    let mut desired = HashSet::new();
    let mut write_meta = Map::new();
    for pane in &panes {
        let key = pane.status_key();
        desired.insert(key.clone());
        let status = if counts.contains_key(&pane.agent_status.to_lowercase()) {
            pane.agent_status.to_lowercase()
        } else {
            "other".into()
        };
        counts[&status] = json!(counts[&status].as_i64().unwrap_or(0) + 1);
        let prior = state::prior_for_pane(pane, previous);
        let payload = crate::status::status_write_payload(pane, Some(&tabs), Some(prior));
        if !crate::status::should_write_status_pill(&payload, Some(prior)) {
            skipped.push(key);
            continue;
        }
        let value = payload["value"].as_str().unwrap_or("");
        let icon = payload["icon"].as_str().unwrap_or("");
        let color = payload["color"].as_str().unwrap_or("");
        let priority = payload["priority"].as_i64().unwrap_or(0).to_string();
        match bridge::cmux_cmd(
            &[
                "set-status",
                &key,
                value,
                "--icon",
                icon,
                "--color",
                color,
                "--priority",
                &priority,
            ],
            Some(&workspace),
        ) {
            Ok(proc) if proc.returncode == 0 => {
                applied.push(key);
                write_meta.insert(pane.pane_id.clone(),json!({"last_status_value":value,"last_icon":icon,"last_color":color,"last_priority":payload["priority"]}));
            }
            Ok(proc) => errors.push(if proc.stderr.trim().is_empty() {
                proc.stdout.trim().into()
            } else {
                proc.stderr.trim().into()
            }),
            Err(error) => errors.push(error.to_string()),
        }
    }
    let mut stale = Vec::new();
    if !b(m, "no-clear-stale") {
        for key in list_status_keys(&workspace) {
            if !desired.contains(&key)
                && bridge::cmux_cmd(&["clear-status", &key], Some(&workspace))
                    .is_ok_and(|proc| proc.returncode == 0)
            {
                stale.push(key)
            }
        }
    }
    let active = counts["working"].as_i64().unwrap_or(0);
    let total = active
        + counts["idle"].as_i64().unwrap_or(0)
        + counts["done"].as_i64().unwrap_or(0)
        + counts["blocked"].as_i64().unwrap_or(0);
    let progress = if !b(m, "no-progress") && total > 0 {
        let value = ((active as f64 / total as f64) * 1000.0).round() / 1000.0;
        let mut text = value.to_string();
        if !text.contains('.') {
            text.push_str(".0")
        }
        let label = format!("herdr {active}/{total} working");
        let _ = bridge::cmux_cmd(
            &["set-progress", &text, "--label", &label],
            Some(&workspace),
        );
        Some(value)
    } else {
        None
    };
    let unchanged = if skipped.is_empty() {
        String::new()
    } else {
        format!(" unchanged={}", skipped.len())
    };
    let summary = format!(
        "herdr sync: {} panes → cmux ws={} (working={} idle={} done={} blocked={} unknown={}{}{})",
        applied.len(),
        workspace,
        counts["working"],
        counts["idle"],
        counts["done"],
        counts["blocked"],
        counts["unknown"],
        unchanged,
        ""
    );
    if !b(m, "no-log") {
        let _ = bridge::cmux_cmd(&["log", &summary], Some(&workspace));
    }
    let associations = state::update_association_map(
        &SystemEnv,
        &snap,
        Some(&workspace),
        Some(&Value::Object(write_meta)),
    )
    .unwrap_or(Value::Null);
    let result = json!({"workspace":workspace,"applied":applied,"skipped_unchanged":skipped,"stale_cleared":stale,"counts":counts,"progress":progress,"errors":errors,"summary":summary,"pane_count":snap.panes.len(),"agent_count":panes.len(),"associations":associations,"host_fingerprint_key":fingerprint_key,"writer":"plugin","native_live":false});
    println!("{}", result["summary"].as_str().unwrap());
    if !result["skipped_unchanged"].as_array().unwrap().is_empty() {
        println!(
            "unchanged: {} pills",
            result["skipped_unchanged"].as_array().unwrap().len()
        )
    }
    if !stale.is_empty() {
        println!("cleared stale: {}", stale.join(", "))
    }
    if b(m, "json") {
        pretty(&result)
    }
    if errors.is_empty() {
        0
    } else {
        for error in errors {
            eprintln!("error: {error}")
        }
        2
    }
}

fn cmd_status(m: &ArgMatches) -> i32 {
    let fingerprint = state::collect_host_fingerprint(&SystemEnv);
    let key = state::parent_key(&fingerprint);
    let available = bridge::herdr_available();
    let mut herdr = json!({"available":available,"cli":bridge::which("herdr"),"env":std::env::var("HERDR_ENV").ok(),"workspace_id":std::env::var("HERDR_WORKSPACE_ID").ok(),"tab_id":std::env::var("HERDR_TAB_ID").ok(),"pane_id":std::env::var("HERDR_PANE_ID").ok(),"socket_path":std::env::var("HERDR_SOCKET_PATH").ok(),"socket_exists":std::env::var("HERDR_SOCKET_PATH").ok().is_some_and(|path|std::path::Path::new(&path).exists()),"server_pid":fingerprint.herdr_server_pid});
    if available {
        match call_api("session.snapshot", json!({})) {
            Ok(outcome) => {
                herdr["api"] = json!({"ok":outcome.ok,"via":outcome.via});
                if let Some(snap) = crate::model::snapshot_from_session_payload(&outcome.result) {
                    herdr["workspace_count"] = json!(snap.workspaces.len());
                    herdr["tab_count"] = json!(snap.tabs.len());
                    herdr["pane_count"] = json!(snap.panes.len());
                    herdr["agent_count"] = json!(snap.agent_panes().len());
                    let mut counts = Map::new();
                    for pane in snap.agent_panes() {
                        let status = pane.agent_status.to_lowercase();
                        counts[&status] =
                            json!(counts.get(&status).and_then(Value::as_i64).unwrap_or(0) + 1)
                    }
                    herdr["status_counts"] = Value::Object(counts)
                }
            }
            Err(error) => herdr["api"] = json!({"ok":false,"error":error.to_string()}),
        }
    }
    let cmux = json!({"available":bridge::cmux_available(),"cli":bridge::which("cmux"),"workspace_id":std::env::var("CMUX_WORKSPACE_ID").ok(),"tab_id":std::env::var("CMUX_TAB_ID").ok(),"surface_id":std::env::var("CMUX_SURFACE_ID").ok(),"socket_path":std::env::var("CMUX_SOCKET_PATH").ok(),"socket_exists":std::env::var("CMUX_SOCKET_PATH").ok().is_some_and(|path|std::path::Path::new(&path).exists()),"resolved_workspace":resolve_workspace(None)});
    let writer = crate::handoff::writer_status(&key);
    let missing = state::fingerprint_missing_fields(&fingerprint);
    let payload = json!({"nested":std::env::var_os("HERDR_ENV").is_some(),"herdr":herdr,"cmux":cmux,"host_fingerprint":{"cmux_surface_id":fingerprint.cmux_surface_id,"herdr_socket_path":fingerprint.herdr_socket_path,"herdr_server_pid":fingerprint.herdr_server_pid,"herdr_workspace_id":fingerprint.herdr_workspace_id},"host_fingerprint_key":key,"host_fingerprint_missing":missing,"writer":writer});
    println!(
        "cmux-herdr status\n─────────────────\nnested context : {}",
        if payload["nested"].as_bool().unwrap_or(false) {
            "yes"
        } else {
            "no"
        }
    );
    println!("herdr:\n  available    : {}\n  cli          : {}\n  env          : {}\n  workspace    : {}\n  tab          : {}\n  pane         : {}\n  socket       : {} (exists={})\n  server pid   : {}",plain(&herdr["available"]),plain(&herdr["cli"]),plain(&herdr["env"]),plain(&herdr["workspace_id"]),plain(&herdr["tab_id"]),plain(&herdr["pane_id"]),plain(&herdr["socket_path"]),plain(&herdr["socket_exists"]),herdr["server_pid"].as_i64().map(|value|value.to_string()).unwrap_or_else(||"-".into()));
    if herdr["api"]["ok"].as_bool() == Some(true) {
        println!("  api          : ok via {}", plain(&herdr["api"]["via"]))
    } else if !herdr["api"]["error"].is_null() {
        println!("  api          : {}", plain(&herdr["api"]["error"]))
    }
    if !herdr["pane_count"].is_null() {
        println!(
            "  topology     : workspaces={} tabs={} panes={} agents={}",
            plain(&herdr["workspace_count"]),
            plain(&herdr["tab_count"]),
            plain(&herdr["pane_count"]),
            plain(&herdr["agent_count"])
        );
        println!("  statuses     : {}", plain(&herdr["status_counts"]))
    }
    println!("cmux:\n  available    : {}\n  cli          : {}\n  env workspace: {}\n  env tab      : {}\n  env surface  : {}\n  socket       : {} (exists={})\n  resolved ws  : {}",plain(&cmux["available"]),plain(&cmux["cli"]),plain(&cmux["workspace_id"]),plain(&cmux["tab_id"]),plain(&cmux["surface_id"]),plain(&cmux["socket_path"]),plain(&cmux["socket_exists"]),plain(&cmux["resolved_workspace"]));
    println!(
        "host fingerprint:\n  key          : {}\n  missing      : {}",
        if key.is_empty() { "-" } else { &key },
        if missing.is_empty() {
            "(none)".into()
        } else {
            missing.join(", ")
        }
    );
    println!(
        "writer:\n  path         : {}\n  native live  : {}\n  plugin live  : {}",
        writer["writer"].as_str().unwrap_or("plugin"),
        plain(&writer["native_live"]),
        plain(&writer["plugin_live"])
    );
    if writer["lease_stale"].as_bool() == Some(true) {
        println!("  lease        : stale (other path may resume)")
    }
    if writer["force_plugin"].as_bool() == Some(true) {
        println!("  force plugin : yes")
    }
    if b(m, "json") {
        pretty(&payload)
    }
    0
}

fn cmd_doctor(m: &ArgMatches) -> i32 {
    let fp = state::collect_host_fingerprint(&SystemEnv);
    let missing = state::fingerprint_missing_fields(&fp);
    let nested = std::env::var_os("HERDR_ENV").is_some();
    let checks = vec![
        json!({"name":"herdr_cli","ok":bridge::which("herdr").is_some(),"hard":true,"detail":bridge::which("herdr").unwrap_or_else(||"not found on PATH".into())}),
        json!({"name":"herdr_health","ok":bridge::herdr_available(),"hard":true,"detail":if bridge::herdr_available(){"available"}else{"unavailable"}}),
        json!({"name":"host_fingerprint","ok":!nested||missing.is_empty(),"hard":nested,"detail":if missing.is_empty(){"complete".into()}else{format!("incomplete host fingerprint (missing {})",missing.join(", "))}}),
    ];
    let hard: Vec<String> = checks
        .iter()
        .filter(|c| c["hard"] == json!(true) && c["ok"] == json!(false))
        .map(|c| c["detail"].as_str().unwrap_or("").to_string())
        .collect();
    let report = json!({"ok":hard.is_empty(),"checks":checks,"hard_failures":hard});
    println!("cmux-herdr doctor\n─────────────────");
    for c in checks {
        let mark = if c["ok"] == json!(true) {
            "ok"
        } else if c["hard"] == json!(true) {
            "FAIL"
        } else {
            "warn"
        };
        println!(
            "[{mark:4}] {}: {}",
            c["name"].as_str().unwrap(),
            c["detail"].as_str().unwrap()
        )
    }
    if b(m, "json") {
        pretty(&report)
    }
    if report["ok"] == json!(true) {
        0
    } else {
        1
    }
}

fn cmd_lease(m: &ArgMatches) -> i32 {
    let fingerprint = state::collect_host_fingerprint(&SystemEnv);
    let key = state::parent_key(&fingerprint);
    let decision = crate::handoff::resolve_writer(&key, None, None);
    let size_env = std::env::var(crate::mirror::SIZE_AUTHORITY_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty());
    let size_authority = size_env.clone().or_else(crate::mirror::read_size_authority);
    let mut payload = json!({"fingerprint":key,"writer":decision.writer,"outcome":decision.outcome(),"native_live":decision.native_live,"plugin_live":decision.plugin_live,"lease_stale":decision.lease_stale,"force_plugin":decision.force_plugin,"env_native_live":decision.env_native_live,"size_authority":size_authority,"size_authority_path":crate::mirror::size_authority_path(),"size_authority_env":size_env});
    if let Some(lease) = decision.lease {
        payload["lease"] = lease.to_dict()
    }
    if b(m, "json") {
        pretty(&payload)
    } else {
        println!("fingerprint : {}\nwriter      : {}\noutcome     : {}\nnative_live : {}\nplugin_live : {}\nlease_stale : {}\nsize_auth   : {}",key,payload["writer"].as_str().unwrap(),payload["outcome"].as_str().unwrap(),plain(&payload["native_live"]),plain(&payload["plugin_live"]),plain(&payload["lease_stale"]),payload["size_authority"].as_str().unwrap_or("(none)"));
        if decision.force_plugin {
            println!("force_plugin: True")
        }
    }
    0
}

fn cmd_send_key(m: &ArgMatches) -> i32 {
    if let Err(code) = ensure_herdr() {
        return code;
    }
    let pane = s(m, "pane_id").unwrap();
    let name = s(m, "key").unwrap();
    let Some(input) = crate::control::encode_named_key(pane, name) else {
        return die(format!("unknown key name: {name}"));
    };
    if let Some(key) = input.key.as_deref() {
        if call_api(
            "pane.send_keys",
            json!({"pane_id":pane,"keys":key,"key":key}),
        )
        .is_ok()
        {
            println!("sent {key} to {pane} via send_keys");
            return 0;
        }
    }
    if let Some(csi) = input.csi {
        let text: String = csi.into_iter().map(char::from).collect();
        match call_api("pane.send_text", json!({"pane_id":pane,"text":text})) {
            Ok(_) => {
                println!(
                    "sent {} to {pane} via csi",
                    input.key.as_deref().unwrap_or("None")
                );
                0
            }
            Err(e) => die(e),
        }
    } else {
        die(format!("could not send key {name} to {pane}"))
    }
}

fn tmux_parity(m: &ArgMatches) -> bool {
    b(m, "tmux-parity")
        || m.try_get_one::<bool>("pills-only")
            .ok()
            .flatten()
            .is_some_and(|pills| !*pills)
}
fn mirror_scope(m: &ArgMatches) -> &'static str {
    if tmux_parity(m) || b(m, "all") {
        "all"
    } else if s(m, "herdr-workspace").is_some() {
        "workspace"
    } else {
        "current-tab"
    }
}
fn cmd_mirror(m: &ArgMatches) -> i32 {
    if crate::mirror::is_attach_process() {
        return die("refusing to nest mirror inside attach-pane");
    }
    if let Err(code) = ensure_herdr() {
        return code;
    }
    let tmux = tmux_parity(m);
    let result = match crate::mirror::mirror_to_cmux(
        mirror_scope(m),
        s(m, "workspace"),
        s(m, "herdr-workspace"),
        s(m, "tab"),
        b(m, "prune") || tmux,
        !b(m, "no-status") && !b(m, "no-progress"),
        !b(m, "no-layout"),
        b(m, "focus") || tmux,
        b(m, "order") || tmux,
        b(m, "ratios") || tmux,
        tmux,
        b(m, "dry-run"),
        !b(m, "no-log"),
    ) {
        Ok(value) => value,
        Err(error) => return die(error),
    };
    println!("{}", crate::mirror::format_mirror_plan(&result));
    if b(m, "json") {
        pretty(&result)
    }
    if result["plan"]["errors"]
        .as_array()
        .is_some_and(|errors| !errors.is_empty())
    {
        2
    } else {
        0
    }
}

fn cmd_watch(m: &ArgMatches) -> i32 {
    if b(m, "once") {
        return if b(m, "pills-only") {
            cmd_sync(m)
        } else {
            cmd_mirror(m)
        };
    }
    if let Err(code) = ensure_herdr() {
        return code;
    }
    let interval = (*m.get_one::<f64>("interval").unwrap()).max(0.5);
    WATCH_STOP.store(false, Ordering::Relaxed);
    install_watch_signals();
    eprintln!(
        "cmux-herdr watch: {} every {}s (workspace={}); Ctrl-C to stop",
        if b(m, "pills-only") {
            "status"
        } else {
            "tmux-parity"
        },
        interval,
        s(m, "workspace").unwrap_or("auto")
    );
    let mut errors = ErrorDeduplicator::default();
    while !WATCH_STOP.load(Ordering::Relaxed) {
        let code = if b(m, "pills-only") {
            cmd_sync(m)
        } else {
            cmd_mirror(m)
        };
        if code == 0 {
            errors.success()
        } else {
            errors.report(format!("cmux-herdr: watch iteration failed ({code})"))
        }
        if tmux_parity(m) || b(m, "events") {
            let _ = crate::mirror::wait_herdr_event(interval);
        } else {
            let deadline = std::time::Instant::now() + Duration::from_secs_f64(interval);
            while !WATCH_STOP.load(Ordering::Relaxed) && std::time::Instant::now() < deadline {
                std::thread::sleep(
                    Duration::from_millis(250)
                        .min(deadline.saturating_duration_since(std::time::Instant::now())),
                )
            }
        }
    }
    eprintln!("\ncmux-herdr watch: stopping…");
    0
}

fn cmd_attach_pane(m: &ArgMatches) -> i32 {
    if let Err(code) = ensure_herdr() {
        return code;
    }
    match crate::mirror::attach_pane_loop(
        s(m, "pane_id").unwrap(),
        (*m.get_one::<f64>("interval").unwrap()).max(0.05),
        *m.get_one::<i64>("lines").unwrap(),
        !b(m, "read-only"),
        !b(m, "no-raw-tty"),
        !b(m, "no-resize"),
        !b(m, "no-ansi"),
    ) {
        Ok(code) => code,
        Err(error) => die(error),
    }
}

fn live_inputs(
    m: &ArgMatches,
) -> Result<
    (
        crate::model::Snapshot,
        Vec<crate::engine::HerdrWindow>,
        String,
    ),
    i32,
> {
    let snap = fetch_snapshot().map_err(die)?;
    let desired = crate::mirror::desired_mirrors(&snap, "all", None, None, true).map_err(die)?;
    let windows = crate::mirror::build_herdr_windows(&snap, &desired);
    let socket = crate::live::resolve_socket_path(s(m, "socket"))
        .ok_or_else(|| die("need an absolute Herdr socket (--socket or HERDR_SOCKET_PATH)"))?;
    Ok((snap, windows, socket))
}

fn cmd_live(name: &str, m: &ArgMatches) -> i32 {
    if let Err(code) = ensure_herdr() {
        return code;
    }
    let (snap, windows, socket) = match live_inputs(m) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let payload = match name {
        "attach" => {
            let mut sessions = crate::live::sessions_from_snapshot(&snap);
            if let Some(wanted) = s(m, "session") {
                let matched: Vec<_> = sessions
                    .iter()
                    .filter(|item| item.session_id == wanted || item.name == wanted)
                    .cloned()
                    .collect();
                if !matched.is_empty() {
                    sessions = matched
                }
            }
            let (_, payload) =
                crate::live::attach_live(&windows, &sessions, &socket, !b(m, "no-activate"), true);
            payload
        }
        "detach" => crate::live::detach_live(&windows, &socket),
        "restore" => {
            let sessions = crate::live::sessions_from_snapshot(&snap);
            let (_, payload) = crate::live::restore_live(&windows, &sessions, &socket);
            payload
        }
        "observe" => {
            let raw = s(m, "method").unwrap();
            let method = if raw.starts_with("remote.herdr.") {
                raw.to_string()
            } else {
                format!("remote.herdr.{raw}")
            };
            let (_, payload) = crate::live::observe_live(
                &windows,
                &socket,
                &method,
                s(m, "session").unwrap_or("main"),
            );
            payload
        }
        _ => unreachable!(),
    };
    if b(m, "json") {
        pretty(&payload)
    } else {
        match name {
            "attach" => {
                if payload["writer"] == json!("native")
                    || payload["outcome"] == json!("native_owns")
                {
                    println!("native owns this host; plugin apply not started")
                } else {
                    println!(
                        "attached {} tabs={} server_stopped={}",
                        plain(&payload["outcome"]),
                        payload["apply"]["tabs"].as_array().map_or(0, Vec::len),
                        plain(&payload["server_stopped"])
                    );
                    if let Some(path) = payload["restore_path"].as_str() {
                        println!("restore: {path}")
                    }
                }
            }
            "detach" => {
                if payload["outcome"] == json!("native_owns") {
                    println!("native owns this host; plugin did not detach native")
                } else {
                    println!(
                        "detached (server_stopped={}; Herdr session left running)",
                        plain(&payload["server_stopped"])
                    )
                }
            }
            "restore" => {
                if payload["outcome"] == json!("native_owns") {
                    println!("native owns this host; restore stays on the native path")
                } else {
                    println!(
                        "restore {} server_stopped={}",
                        payload
                            .get("mode")
                            .or_else(|| payload.get("outcome"))
                            .map(plain)
                            .unwrap_or_default(),
                        plain(&payload["server_stopped"])
                    )
                }
            }
            _ => println!("{}", payload),
        }
    }
    if payload["ok"].as_bool().unwrap_or(false) {
        0
    } else {
        2
    }
}

#[cfg(target_os = "macos")]
fn update_manager() -> crate::update::ServiceManager {
    crate::update::ServiceManager::Launchd {
        domain: format!("gui/{}", rustix::process::getuid().as_raw()),
    }
}
#[cfg(not(target_os = "macos"))]
fn update_manager() -> crate::update::ServiceManager {
    crate::update::ServiceManager::Systemd
}
fn file_change(value: crate::update::FileChange) -> &'static str {
    match value {
        crate::update::FileChange::Changed => "changed",
        crate::update::FileChange::Unchanged => "unchanged",
    }
}
fn update_paths(
    m: &ArgMatches,
    manager: &crate::update::ServiceManager,
) -> Result<crate::update::ServicePaths, i32> {
    crate::update::ServicePaths::discover(manager, s(m, "herdr").map(Path::new)).map_err(die)
}
fn cmd_update_service(m: &ArgMatches) -> i32 {
    let (action, sub) = m.subcommand().expect("required update-service action");
    let manager = update_manager();
    let paths = match update_paths(sub, &manager) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let result = match action {
        "install" => {
            let request = crate::update::InstallRequest::new(
                manager,
                paths,
                s(sub, "channel").unwrap().to_string(),
                s(sub, "manifest-url").unwrap().to_string(),
            );
            match crate::update::install_service(&request, &crate::update::RealCommandRunner) {
                Ok(out) => {
                    json!({"ok":true,"action":"install","config":file_change(out.config),"runtime_binary":out.runtime_binary,"definitions":out.definitions,"uninstall_command":out.uninstall_command})
                }
                Err(e) => return die(e),
            }
        }
        "uninstall" => match crate::update::uninstall_service(
            &manager,
            &paths,
            &crate::update::RealCommandRunner,
        ) {
            Ok(out) => {
                json!({"ok":true,"action":"uninstall","config":file_change(out.config),"removed":out.removed,"command_warnings":out.command_warnings})
            }
            Err(e) => return die(e),
        },
        "run" => {
            let request = crate::update::UpdateRequest::new(
                paths.herdr_binary.clone(),
                paths.state_root.clone(),
            );
            match crate::update::run_update(
                &request,
                &crate::update::RealCommandRunner,
                &crate::update::SystemLiveness,
            ) {
                Ok(crate::update::UpdateOutcome::Busy { pid }) => {
                    json!({"ok":true,"action":"run","outcome":"busy","pid":pid})
                }
                Ok(crate::update::UpdateOutcome::Current { version, digest }) => {
                    json!({"ok":true,"action":"run","outcome":"current","version":version,"sha256":digest})
                }
                Ok(crate::update::UpdateOutcome::Updated {
                    from_version,
                    to_version,
                    digest,
                    backup,
                }) => {
                    json!({"ok":true,"action":"run","outcome":"updated","from_version":from_version,"to_version":to_version,"sha256":digest,"backup":backup})
                }
                Err(e) => return die(e),
            }
        }
        "status" => {
            let definitions = match manager {
                crate::update::ServiceManager::Launchd { .. } => vec![paths.launchd_plist()],
                crate::update::ServiceManager::Systemd => {
                    vec![paths.systemd_service(), paths.systemd_timer()]
                }
            };
            let runtime = paths.runtime_binary();
            json!({"ok":true,"action":"status","installed":runtime.exists()&&definitions.iter().all(|p|p.exists()),"runtime_binary":runtime,"runtime_exists":runtime.exists(),"definitions":definitions.iter().map(|p|json!({"path":p,"exists":p.exists()})).collect::<Vec<_>>(),"config_path":paths.config_path,"state_root":paths.state_root})
        }
        _ => unreachable!(),
    };
    if b(sub, "json") {
        pretty(&result)
    } else {
        println!(
            "update-service {action}: {}",
            if result["ok"].as_bool().unwrap_or(false) {
                result
                    .get("outcome")
                    .and_then(Value::as_str)
                    .unwrap_or("ok")
            } else {
                "failed"
            }
        )
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parser_registers_every_python_handler_once() {
        let fixture: Value =
            serde_json::from_str(include_str!("../tests/cli_golden.json")).unwrap();
        let expected: Vec<String> = fixture["python_commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert_eq!(expected.len(), 61);
        let extras = ["sidebar", "update-service"];
        let mut actual: Vec<String> = build_parser()
            .get_subcommands()
            .map(|c| c.get_name().to_string())
            .filter(|n| !extras.contains(&n.as_str()))
            .collect();
        actual.sort();
        assert_eq!(actual, expected);
        for extra in extras {
            assert!(build_parser().find_subcommand(extra).is_some());
        }
    }
    #[test]
    fn parser_preserves_watch_defaults_and_choices() {
        let watch = build_parser()
            .try_get_matches_from(["cmux-herdr", "watch"])
            .unwrap();
        let sub = watch.subcommand_matches("watch").unwrap();
        assert!(!b(sub, "pills-only"));
        assert!(tmux_parity(sub));
        assert_eq!(*sub.get_one::<f64>("interval").unwrap(), 3.0);
        let pills = build_parser()
            .try_get_matches_from(["cmux-herdr", "watch", "--pills-only"])
            .unwrap();
        assert!(!tmux_parity(pills.subcommand_matches("watch").unwrap()));
        assert!(build_parser()
            .try_get_matches_from(["cmux-herdr", "resize-pane", "--direction", "diagonal"])
            .is_err());
    }
    #[test]
    fn error_deduplicator_resets_after_success() {
        let mut errors = ErrorDeduplicator::default();
        errors.report("same failure".into());
        errors.report("same failure".into());
        assert_eq!(errors.last, "same failure");
        errors.success();
        assert!(errors.last.is_empty());
        errors.report("same failure".into());
        assert_eq!(errors.last, "same failure");
    }
}
