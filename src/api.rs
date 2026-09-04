//! Socket-first Herdr control surface for the cmux-herdr plugin.
//!
//! Official methods: <https://herdr.dev/docs/socket-api/>
//!
//! Calls the Unix-socket RPC first (same wire as native
//! `HerdrNestedTopologyClient`), then falls back to documented `herdr` CLI
//! wrappers when the socket is down. Mutations talk to Herdr even when native
//! owns the cmux projection — the handoff lease only gates mirroring.
//!
//! Never wraps `server.stop`, `pane.graphics.*`, or `plugin.*`.
//!
//! Ported from `bridge/cmux_herdr_api.py`. Behavior-preserving except for the
//! retry policy in [`HerdrApi::call`], which follows the oracle parity ruling:
//! only explicitly replay-safe reads are retried after an ambiguous transport
//! failure, and a mutation whose request bytes may have hit the wire is never
//! resent or CLI-fallback'd (returns an indeterminate-outcome error instead).
//! Python retried every method once and could double-apply a mutation
//! (`cmux_herdr_api.py:596-609`).

use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

use serde_json::{json, Value};

use crate::socket::{socket_path_from_env, ErrorStage, SocketClient, SocketError};

/// User-facing Herdr API failure (allowlist, transport, or CLI) (`ApiError`).
#[derive(Debug)]
pub struct ApiError(pub String);

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ApiError {}

impl ApiError {
    fn new(msg: impl Into<String>) -> Self {
        ApiError(msg.into())
    }
}

type Result<T, E = ApiError> = std::result::Result<T, E>;

/// Destructive / experimental surfaces the plugin must not expose
/// (`FORBIDDEN_METHODS`).
const FORBIDDEN_METHODS: &[&str] = &["server.stop"];

/// Prefixes the plugin refuses to wrap (`FORBIDDEN_PREFIXES`).
const FORBIDDEN_PREFIXES: &[&str] = &["pane.graphics.", "plugin."];

/// Published protocol-17 methods the plugin may call (`ALLOWED_METHODS`). Keep
/// in lockstep with herdr.dev/docs/socket-api — add here when Herdr ships a new
/// *safe* verb. Order irrelevant (membership test only).
const ALLOWED_METHODS: &[&str] = &[
    "ping",
    "server.reload_config",
    "server.agent_manifests",
    "server.reload_agent_manifests",
    "notification.show",
    "client.window_title.set",
    "client.window_title.clear",
    "session.snapshot",
    "workspace.create",
    "workspace.list",
    "workspace.get",
    "workspace.focus",
    "workspace.rename",
    "workspace.move",
    "workspace.move_block",
    "workspace.report_metadata",
    "workspace.close",
    "worktree.list",
    "worktree.create",
    "worktree.open",
    "worktree.remove",
    "tab.create",
    "tab.list",
    "tab.get",
    "tab.focus",
    "tab.rename",
    "tab.move",
    "tab.close",
    "pane.split",
    "pane.swap",
    "pane.move",
    "pane.zoom",
    "pane.layout",
    "pane.process_info",
    "pane.neighbor",
    "pane.edges",
    "pane.focus_direction",
    "pane.resize",
    "pane.list",
    "pane.current",
    "pane.get",
    "pane.rename",
    "pane.send_text",
    "pane.send_keys",
    "pane.send_input",
    "pane.read",
    "pane.report_agent",
    "pane.report_agent_session",
    "pane.report_metadata",
    "pane.clear_agent_authority",
    "pane.release_agent",
    "pane.close",
    "pane.wait_for_output",
    "popup.close",
    "layout.export",
    "layout.apply",
    "layout.set_split_ratio",
    "agent.list",
    "agent.get",
    "agent.read",
    "agent.explain",
    "agent.send_keys",
    "agent.prompt",
    "agent.wait",
    "agent.rename",
    "agent.focus",
    "agent.start",
    "agent.view.set",
    "agent.view.clear",
    "events.subscribe",
    "events.wait",
    "integration.install",
    "integration.uninstall",
];

/// Methods that are safe to replay after an ambiguous transport/protocol
/// failure (oracle decision_2). Default classification is non-replayable;
/// membership here is the *only* thing that authorizes a reconnect+retry.
/// Never infer replay safety from a name prefix.
const REPLAY_SAFE_READS: &[&str] = &[
    "ping",
    "server.agent_manifests",
    "session.snapshot",
    "workspace.list",
    "workspace.get",
    "worktree.list",
    "tab.list",
    "tab.get",
    "pane.list",
    "pane.current",
    "pane.get",
    "pane.read",
    "pane.process_info",
    "pane.neighbor",
    "pane.edges",
    "pane.layout",
    "layout.export",
    "agent.list",
    "agent.get",
    "agent.read",
];

/// Return `method` if it is a published, non-forbidden RPC
/// (`assert_method_allowed`). Errors when empty, forbidden, or not allowlisted.
pub fn assert_method_allowed(method: &str) -> Result<String> {
    let name = method.trim();
    if name.is_empty() {
        return Err(ApiError::new("missing Herdr method"));
    }
    if FORBIDDEN_METHODS.contains(&name) || FORBIDDEN_PREFIXES.iter().any(|p| name.starts_with(p)) {
        return Err(ApiError::new(format!(
            "refusing {name}: not part of the plugin control surface"
        )));
    }
    if !ALLOWED_METHODS.contains(&name) {
        return Err(ApiError::new(format!(
            "unknown or unsupported Herdr method: {name}"
        )));
    }
    Ok(name.to_string())
}

fn is_replay_safe(method: &str) -> bool {
    REPLAY_SAFE_READS.contains(&method)
}

/// Pull pane/agent text out of a socket result or CLI blob (`extract_read_text`).
pub fn extract_read_text(payload: &Value) -> String {
    match payload {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(value_to_plain)
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(map) => {
            for key in ["text", "content", "output", "data"] {
                if let Some(Value::String(s)) = map.get(key) {
                    return s.clone();
                }
            }
            if let Some(Value::Array(lines)) = map.get("lines") {
                return lines
                    .iter()
                    .map(value_to_plain)
                    .collect::<Vec<_>>()
                    .join("\n");
            }
            for key in ["result", "pane", "agent"] {
                if let Some(nested) = map.get(key) {
                    if !std::ptr::eq(nested, payload) {
                        let text = extract_read_text(nested);
                        if !text.is_empty() {
                            return text;
                        }
                    }
                }
            }
            String::new()
        }
        other => other.to_string(),
    }
}

/// Return `agent_status` from a pane/agent get/list payload
/// (`extract_agent_status`).
pub fn extract_agent_status(payload: &Value) -> Option<String> {
    let map = payload.as_object()?;
    for key in ["agent_status", "status", "state"] {
        if let Some(Value::String(s)) = map.get(key) {
            if !s.is_empty() {
                return Some(s.clone());
            }
        }
    }
    for key in ["pane", "agent", "result"] {
        if let Some(nested) = map.get(key) {
            if nested.is_object() && !std::ptr::eq(nested, payload) {
                if let Some(found) = extract_agent_status(nested) {
                    return Some(found);
                }
            }
        }
    }
    None
}

/// Stringify a JSON scalar the way Python's `str(item)` renders it for the
/// text joins in [`extract_read_text`]: strings verbatim, everything else via
/// its JSON form (numbers/bools without quotes).
fn value_to_plain(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Return a trimmed string, or empty when the value is missing (`_str`).
/// Numbers/bools render via their JSON form to mirror Python `str(value)`.
fn s(params: &Value, key: &str) -> String {
    str_of(params.get(key))
}

fn str_of(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.trim().to_string(),
        Some(other) => other.to_string().trim().to_string(),
    }
}

/// First non-empty of two params (mirrors `params.get(a) or params.get(b)`).
fn s_or(params: &Value, a: &str, b: &str) -> String {
    let first = s(params, a);
    if !first.is_empty() {
        first
    } else {
        s(params, b)
    }
}

/// Render a numeric/JSON param as Python's `str(x)` would for argv (used for
/// `--ratio`, `--amount`, `--index`, `--lines`). Integers drop the `.0`.
fn num_str(v: &Value) -> String {
    match v {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => {
            // Python str(True) == "True"; only used where Herdr expects it.
            if *b {
                "True".into()
            } else {
                "False".into()
            }
        }
        other => other.to_string(),
    }
}

/// `--pane ID` or `--current` (Herdr CLI, not a positional id) (`_cli_pane_target`).
fn cli_pane_target(pane: &str) -> Vec<String> {
    if !pane.is_empty() {
        vec!["--pane".into(), pane.into()]
    } else {
        vec!["--current".into()]
    }
}

/// Map socket `timeout_ms` onto documented `--timeout` (ms) (`_cli_timeout`).
fn cli_timeout(params: &Value) -> Vec<String> {
    let mut timeout = params.get("timeout_ms").cloned();
    if timeout.is_none() {
        if let Some(wait) = params.get("wait").and_then(Value::as_object) {
            timeout = wait.get("timeout_ms").cloned();
        }
    }
    match timeout {
        Some(Value::Null) | None => vec![],
        Some(v) => vec!["--timeout".into(), num_str(&v)],
    }
}

/// Map a socket method onto a documented `herdr` CLI argv (no binary).
///
/// Ported verbatim from `build_cli_argv` (`cmux_herdr_api.py:205-548`). Returns
/// `None` when there is no safe CLI equivalent (socket-only) or required params
/// are missing. Never invents verbs or flags.
pub fn build_cli_argv(method: &str, params: &Value) -> Option<Vec<String>> {
    let empty = json!({});
    let params = if params.is_object() { params } else { &empty };
    let pane = s_or(params, "pane_id", "target");
    let tab = s(params, "tab_id");
    let workspace = s(params, "workspace_id");
    let label = s(params, "label");
    let direction = s(params, "direction");
    let text = s_or(params, "text", "prompt");

    let v = |parts: &[&str]| Some(parts.iter().map(|p| p.to_string()).collect::<Vec<_>>());

    match method {
        "ping" => return v(&["status"]),
        "session.snapshot" => return v(&["api", "snapshot"]),
        "workspace.list" => return v(&["workspace", "list"]),
        _ => {}
    }
    if method == "workspace.get" && !workspace.is_empty() {
        return v(&["workspace", "get", &workspace]);
    }
    if method == "workspace.focus" && !workspace.is_empty() {
        return v(&["workspace", "focus", &workspace]);
    }
    if method == "workspace.close" && !workspace.is_empty() {
        return v(&["workspace", "close", &workspace]);
    }
    if method == "workspace.rename" && !workspace.is_empty() && !label.is_empty() {
        return v(&["workspace", "rename", &workspace, &label]);
    }
    if method == "workspace.create" {
        let mut argv = vec!["workspace".into(), "create".into()];
        let cwd = s(params, "cwd");
        if !cwd.is_empty() {
            argv.extend(["--cwd".into(), cwd]);
        }
        if !label.is_empty() {
            argv.extend(["--label".into(), label]);
        }
        return Some(argv);
    }
    if method == "tab.list" {
        return v(&["tab", "list"]);
    }
    if method == "tab.get" && !tab.is_empty() {
        return v(&["tab", "get", &tab]);
    }
    if method == "tab.focus" && !tab.is_empty() {
        return v(&["tab", "focus", &tab]);
    }
    if method == "tab.close" && !tab.is_empty() {
        return v(&["tab", "close", &tab]);
    }
    if method == "tab.rename" && !tab.is_empty() && !label.is_empty() {
        return v(&["tab", "rename", &tab, &label]);
    }
    if method == "tab.create" {
        // Python order: --label then --workspace.
        let mut argv = vec!["tab".into(), "create".into()];
        if !label.is_empty() {
            argv.extend(["--label".into(), label]);
        }
        if !workspace.is_empty() {
            argv.extend(["--workspace".into(), workspace]);
        }
        return Some(argv);
    }
    if method == "pane.list" {
        return v(&["pane", "list"]);
    }
    if method == "pane.current" {
        // ["pane","current"] with no --current flag; appends --pane <caller>
        // when caller_pane_id is present (documented quirk).
        let mut argv = vec!["pane".into(), "current".into()];
        let caller = s(params, "caller_pane_id");
        if !caller.is_empty() {
            argv.extend(["--pane".into(), caller]);
        }
        return Some(argv);
    }
    if method == "pane.get" && !pane.is_empty() {
        return v(&["pane", "get", &pane]);
    }
    if method == "pane.focus_direction" && !direction.is_empty() {
        let mut argv = vec![
            "pane".into(),
            "focus".into(),
            "--direction".into(),
            direction.clone(),
        ];
        if !pane.is_empty() {
            argv.extend(["--pane".into(), pane.clone()]);
        }
        return Some(argv);
    }
    if method == "pane.send_text" && !pane.is_empty() && !text.is_empty() {
        return Some(vec!["pane".into(), "send-text".into(), pane, text]);
    }
    if method == "pane.send_keys" && !pane.is_empty() {
        let keys = keys_param(params, &text);
        if keys.is_empty() {
            return None;
        }
        return Some(vec!["pane".into(), "send-keys".into(), pane, keys]);
    }
    if method == "pane.read" && !pane.is_empty() {
        let mut argv = vec!["pane".into(), "read".into(), pane];
        let source = {
            let src = s(params, "source");
            if src.is_empty() {
                "recent".into()
            } else {
                src
            }
        };
        argv.extend(["--source".into(), source]);
        if let Some(lines) = non_null(params.get("lines")) {
            argv.extend(["--lines".into(), num_str(lines)]);
        }
        if truthy(params.get("ansi")) {
            argv.push("--ansi".into());
        }
        return Some(argv);
    }
    if method == "layout.export" {
        // CLI wrapper is `pane layout`. Tab-id export is socket-only.
        if !tab.is_empty() && pane.is_empty() {
            return None;
        }
        let mut argv = vec!["pane".into(), "layout".into()];
        argv.extend(cli_pane_target(&pane));
        return Some(argv);
    }
    if method == "pane.move" && !pane.is_empty() {
        return build_pane_move(params, &pane);
    }
    if method == "pane.close" && !pane.is_empty() {
        return Some(vec!["pane".into(), "close".into(), pane]);
    }
    if method == "pane.zoom" {
        let mut argv = vec!["pane".into(), "zoom".into()];
        if !pane.is_empty() {
            argv.push(pane.clone());
        }
        let mode = s(params, "mode");
        if matches!(mode.as_str(), "on" | "off" | "toggle") {
            argv.push(format!("--{mode}"));
        } else if pane.is_empty() {
            argv.push("--current".into());
        }
        return Some(argv);
    }
    if method == "pane.resize" {
        if direction.is_empty() {
            return None;
        }
        let mut argv = vec![
            "pane".into(),
            "resize".into(),
            "--direction".into(),
            direction.clone(),
        ];
        argv.extend(cli_pane_target(&pane));
        if let Some(amount) = non_null(params.get("amount")) {
            argv.extend(["--amount".into(), num_str(amount)]);
        }
        return Some(argv);
    }
    if method == "pane.swap" {
        let source = {
            let sp = s(params, "source_pane_id");
            if sp.is_empty() {
                pane.clone()
            } else {
                sp
            }
        };
        let target = s(params, "target_pane_id");
        if !source.is_empty() && !target.is_empty() {
            return Some(vec![
                "pane".into(),
                "swap".into(),
                "--source-pane".into(),
                source,
                "--target-pane".into(),
                target,
            ]);
        }
        if direction.is_empty() {
            return None;
        }
        let mut argv = vec![
            "pane".into(),
            "swap".into(),
            "--direction".into(),
            direction.clone(),
        ];
        argv.extend(cli_pane_target(&source));
        return Some(argv);
    }
    if method == "pane.neighbor" {
        if direction.is_empty() {
            return None;
        }
        let mut argv = vec![
            "pane".into(),
            "neighbor".into(),
            "--direction".into(),
            direction.clone(),
        ];
        argv.extend(cli_pane_target(&pane));
        return Some(argv);
    }
    if method == "pane.edges" {
        let mut argv = vec!["pane".into(), "edges".into()];
        argv.extend(cli_pane_target(&pane));
        return Some(argv);
    }
    if method == "pane.layout" {
        let mut argv = vec!["pane".into(), "layout".into()];
        argv.extend(cli_pane_target(&pane));
        return Some(argv);
    }
    if method == "pane.split" {
        return build_pane_split(params, &pane, &direction);
    }
    if method == "pane.process_info" {
        let mut argv = vec!["pane".into(), "process-info".into()];
        argv.extend(cli_pane_target(&pane));
        return Some(argv);
    }
    if method == "pane.release_agent" && !pane.is_empty() {
        return Some(vec!["pane".into(), "release-agent".into(), pane]);
    }
    if method == "pane.clear_agent_authority" && !pane.is_empty() {
        return Some(vec!["pane".into(), "clear-agent-authority".into(), pane]);
    }
    if method == "pane.rename" && !pane.is_empty() && !label.is_empty() {
        return Some(vec!["pane".into(), "rename".into(), pane, label]);
    }
    if method == "pane.wait_for_output" && !pane.is_empty() {
        let mat = {
            let m = s(params, "pattern");
            if !m.is_empty() {
                m
            } else {
                let m = s(params, "needle");
                if !m.is_empty() {
                    m
                } else {
                    s(params, "match")
                }
            }
        };
        let regex = s(params, "regex");
        if mat.is_empty() && regex.is_empty() {
            return None;
        }
        let mut argv = vec!["pane".into(), "wait-output".into(), pane];
        if !regex.is_empty() {
            argv.extend(["--regex".into(), regex]);
        } else {
            argv.extend(["--match".into(), mat]);
        }
        argv.extend(cli_timeout(params));
        return Some(argv);
    }
    if method == "agent.list" {
        return v(&["agent", "list"]);
    }
    if method == "agent.get" && !pane.is_empty() {
        return Some(vec!["agent".into(), "get".into(), pane]);
    }
    if method == "agent.focus" && !pane.is_empty() {
        return Some(vec!["agent".into(), "focus".into(), pane]);
    }
    if method == "agent.read" && !pane.is_empty() {
        let mut argv = vec!["agent".into(), "read".into(), pane];
        let source = s(params, "source");
        if !source.is_empty() {
            argv.extend(["--source".into(), source]);
        }
        if let Some(lines) = non_null(params.get("lines")) {
            argv.extend(["--lines".into(), num_str(lines)]);
        }
        return Some(argv);
    }
    if method == "agent.prompt" && !pane.is_empty() && !text.is_empty() {
        let mut argv = vec!["agent".into(), "prompt".into(), pane, text];
        let wait = params.get("wait");
        let mut until = s(params, "until");
        if let Some(w) = wait.and_then(Value::as_object) {
            if until.is_empty() {
                until = str_of(w.get("until"));
            }
        }
        let wants_wait =
            truthy(wait) || !until.is_empty() || matches!(wait, Some(Value::Bool(true)));
        if wants_wait {
            argv.push("--wait".into());
        }
        if !until.is_empty() {
            argv.extend(["--until".into(), until]);
        }
        argv.extend(cli_timeout(params));
        return Some(argv);
    }
    if method == "agent.wait" && !pane.is_empty() {
        let mut argv = vec!["agent".into(), "wait".into(), pane];
        let until = s(params, "until");
        if !until.is_empty() {
            argv.extend(["--until".into(), until]);
        }
        argv.extend(cli_timeout(params));
        return Some(argv);
    }
    if method == "agent.start" {
        let name = s(params, "name");
        let kind = s_or(params, "kind", "agent");
        if name.is_empty() || kind.is_empty() || pane.is_empty() {
            return None;
        }
        let mut argv = vec![
            "agent".into(),
            "start".into(),
            name,
            "--kind".into(),
            kind,
            "--pane".into(),
            pane,
        ];
        argv.extend(cli_timeout(params));
        return Some(argv);
    }
    if method == "agent.send_keys" && !pane.is_empty() {
        let keys = keys_param(params, &text);
        if keys.is_empty() {
            return None;
        }
        return Some(vec!["agent".into(), "send-keys".into(), pane, keys]);
    }
    if method == "agent.rename" && !pane.is_empty() && !label.is_empty() {
        return Some(vec!["agent".into(), "rename".into(), pane, label]);
    }
    if method == "agent.explain" && !pane.is_empty() {
        return Some(vec!["agent".into(), "explain".into(), pane]);
    }
    if method == "agent.view.set" && !pane.is_empty() {
        let view = s(params, "view");
        if view.is_empty() {
            return None;
        }
        return Some(vec![
            "agent".into(),
            "view".into(),
            "set".into(),
            pane,
            view,
        ]);
    }
    if method == "agent.view.clear" && !pane.is_empty() {
        return Some(vec!["agent".into(), "view".into(), "clear".into(), pane]);
    }
    if method == "client.window_title.set" {
        let title = s(params, "title");
        if title.is_empty() {
            return None;
        }
        return Some(vec![
            "client".into(),
            "window-title".into(),
            "set".into(),
            title,
        ]);
    }
    if method == "client.window_title.clear" {
        return v(&["client", "window-title", "clear"]);
    }
    if method == "layout.apply" {
        // Layout trees are JSON; prefer socket. No stable CLI tree passthrough.
        return None;
    }
    if method == "server.agent_manifests" {
        return v(&["server", "agent-manifests"]);
    }
    if method == "server.reload_agent_manifests" {
        return v(&["server", "reload-agent-manifests"]);
    }
    if method == "worktree.list" {
        return v(&["worktree", "list"]);
    }
    if method == "worktree.create" {
        let mut argv = vec!["worktree".into(), "create".into()];
        let path = s(params, "path");
        let name = s(params, "name");
        if !path.is_empty() {
            argv.extend(["--path".into(), path]);
        }
        if !name.is_empty() {
            argv.extend(["--name".into(), name]);
        }
        return Some(argv);
    }
    if method == "worktree.open" {
        let target = s_or(params, "id", "path");
        if target.is_empty() {
            return None;
        }
        return Some(vec!["worktree".into(), "open".into(), target]);
    }
    if method == "worktree.remove" {
        let target = s_or(params, "id", "path");
        if target.is_empty() {
            return None;
        }
        return Some(vec!["worktree".into(), "remove".into(), target]);
    }
    if method == "workspace.move" && !workspace.is_empty() {
        let mut argv = vec!["workspace".into(), "move".into(), workspace];
        if let Some(index) = non_null(params.get("index")) {
            argv.extend(["--index".into(), num_str(index)]);
        }
        return Some(argv);
    }
    if method == "workspace.move_block" && !workspace.is_empty() {
        let block = s(params, "block");
        if block.is_empty() {
            return None;
        }
        let mut argv = vec!["workspace".into(), "move-block".into(), workspace, block];
        if let Some(index) = non_null(params.get("index")) {
            argv.extend(["--index".into(), num_str(index)]);
        }
        return Some(argv);
    }
    if method == "notification.show" {
        let title = s(params, "title");
        if title.is_empty() {
            return None;
        }
        let mut argv = vec!["notification".into(), "show".into(), title];
        let body = s(params, "body");
        if !body.is_empty() {
            argv.extend(["--body".into(), body]);
        }
        return Some(argv);
    }
    None
}

/// `params.get("keys") or params.get("key") or text`, stripped.
fn keys_param(params: &Value, text: &str) -> String {
    let k = s(params, "keys");
    if !k.is_empty() {
        return k;
    }
    let k = s(params, "key");
    if !k.is_empty() {
        return k;
    }
    text.to_string()
}

/// `pane.move` argv builder (`cmux_herdr_api.py:305-356`).
fn build_pane_move(params: &Value, pane: &str) -> Option<Vec<String>> {
    let dest = params.get("destination").and_then(Value::as_object)?;
    let dtype = str_of(dest.get("type"));
    let mut argv = vec!["pane".into(), "move".into(), pane.into()];
    match dtype.as_str() {
        "tab" => {
            let dest_tab = str_of(dest.get("tab_id"));
            let split = {
                let sp = str_of(dest.get("split"));
                if sp.is_empty() {
                    "right".into()
                } else {
                    sp
                }
            };
            if dest_tab.is_empty() {
                return None;
            }
            argv.extend(["--tab".into(), dest_tab, "--split".into(), split]);
            let target = str_of(dest.get("target_pane_id"));
            if !target.is_empty() {
                argv.extend(["--target-pane".into(), target]);
            }
            if let Some(ratio) = non_null(dest.get("ratio")) {
                argv.extend(["--ratio".into(), num_str(ratio)]);
            }
        }
        "new_tab" => {
            argv.push("--new-tab".into());
            let dest_ws = str_of(dest.get("workspace_id"));
            if !dest_ws.is_empty() {
                argv.extend(["--workspace".into(), dest_ws]);
            }
            let dest_label = str_of(dest.get("label"));
            if !dest_label.is_empty() {
                argv.extend(["--label".into(), dest_label]);
            }
        }
        "new_workspace" => {
            argv.push("--new-workspace".into());
            let dest_label = str_of(dest.get("label"));
            if !dest_label.is_empty() {
                argv.extend(["--label".into(), dest_label]);
            }
            let tab_label = str_of(dest.get("tab_label"));
            if !tab_label.is_empty() {
                argv.extend(["--tab-label".into(), tab_label]);
            }
        }
        _ => return None,
    }
    match params.get("focus") {
        Some(Value::Bool(false)) => argv.push("--no-focus".into()),
        Some(other) if truthy(Some(other)) => argv.push("--focus".into()),
        _ => {}
    }
    Some(argv)
}
/// `pane.split` argv builder (`cmux_herdr_api.py:326-336`).
fn build_pane_split(params: &Value, pane: &str, direction: &str) -> Option<Vec<String>> {
    let mut argv = vec!["pane".into(), "split".into()];
    if !pane.is_empty() {
        argv.push(pane.into());
    } else {
        argv.push("--current".into());
    }
    let dir = if direction.is_empty() {
        "right"
    } else {
        direction
    };
    argv.extend(["--direction".into(), dir.into()]);
    if let Some(ratio) = non_null(params.get("ratio")) {
        argv.extend(["--ratio".into(), num_str(ratio)]);
    }
    Some(argv)
}

fn non_null(v: Option<&Value>) -> Option<&Value> {
    match v {
        Some(Value::Null) | None => None,
        some => some,
    }
}

/// Python truthiness for the JSON values we branch on (`if params.get(k)`):
/// null/false/0/""/[]/{} are falsy.
fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::Bool(true)) => true,
        Some(Value::Number(n)) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
    }
}

/// One allowlisted Herdr RPC outcome (`ApiResult`).
#[derive(Debug, Clone)]
pub struct ApiOutcome {
    pub ok: bool,
    pub method: String,
    pub via: &'static str,
    pub result: Value,
    pub error: Option<String>,
}

impl ApiOutcome {
    /// JSON-ready payload for the CLI (`ApiResult.to_dict`).
    pub fn to_json(&self) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert("ok".into(), Value::Bool(self.ok));
        obj.insert("method".into(), Value::String(self.method.clone()));
        obj.insert("via".into(), Value::String(self.via.to_string()));
        obj.insert("result".into(), self.result.clone());
        if let Some(err) = &self.error {
            obj.insert("error".into(), Value::String(err.clone()));
        }
        Value::Object(obj)
    }
}

/// Runs a `herdr <argv>` CLI fallback. Returns parsed JSON stdout or a plain
/// string, mirroring `_cli_request`. Injected in tests.
pub trait CliRunner {
    fn run(&self, argv: &[String]) -> Result<Value>;
}

/// Allowlisted Herdr RPC: socket first, documented CLI fallback (`HerdrApi`).
///
/// [`open`](Self::open) holds one RPC socket (native
/// `HerdrNestedTopologyClient`). Never reuse an `events.subscribe` stream for
/// requests.
pub struct HerdrApi<'r> {
    socket_path: Option<String>,
    client: Option<SocketClient>,
    /// Whether `client` is owned by this instance (openable/closable). An
    /// injected client (tests) is used as-is and never reconnected.
    owned_client: bool,
    cli_runner: Option<&'r dyn CliRunner>,
    timeout: Duration,
}

impl<'r> HerdrApi<'r> {
    /// Create a caller with a resolved-later socket path (default 8s timeout).
    pub fn new(socket_path: Option<String>, timeout: Duration) -> Self {
        HerdrApi {
            socket_path,
            client: None,
            owned_client: false,
            cli_runner: None,
            timeout,
        }
    }

    /// Inject a pre-connected client (tests). Used as-is, never reconnected.
    pub fn with_client(mut self, client: SocketClient) -> Self {
        self.client = Some(client);
        self.owned_client = false;
        self
    }

    /// Set a CLI fallback runner.
    pub fn with_cli_runner(mut self, runner: &'r dyn CliRunner) -> Self {
        self.cli_runner = Some(runner);
        self
    }

    /// Connect a persistent RPC socket. No-op when a client is injected
    /// (`open`).
    pub fn open(&mut self) -> Result<()> {
        if self.client.is_some() {
            if self.client.as_ref().map(|c| c.connected()).unwrap_or(false) {
                return Ok(());
            }
            if !self.owned_client {
                return Ok(());
            }
            if let Some(c) = self.client.as_mut() {
                c.close();
            }
            self.client = None;
            self.owned_client = false;
        }
        let path = self
            .resolved_socket_path()
            .ok_or_else(|| ApiError::new("Herdr socket not available"))?;
        let mut client = SocketClient::new(path, self.timeout);
        client.connect().map_err(|e| ApiError::new(e.message))?;
        self.client = Some(client);
        self.owned_client = true;
        Ok(())
    }

    /// Close an owned persistent socket. Injected clients are left alone
    /// (`close`).
    pub fn close(&mut self) {
        if !self.owned_client {
            return;
        }
        if let Some(mut c) = self.client.take() {
            c.close();
        }
        self.owned_client = false;
    }

    /// Invoke `method` with `params` (`call`).
    ///
    /// Socket first. On an ambiguous transport/protocol failure the socket is
    /// reconnected and retried **once**, but only for replay-safe reads
    /// (oracle decision_2); a mutation whose bytes may have hit the wire is not
    /// resent and not CLI-fallback'd — it returns an indeterminate error. A
    /// remote RPC error is authoritative (no retry, no fallback). CLI fallback
    /// runs only when no socket byte was written and `socket_only` is false.
    pub fn call(&mut self, method: &str, params: Value, socket_only: bool) -> Result<ApiOutcome> {
        let name = assert_method_allowed(method)?;
        let payload = if params.is_object() {
            params
        } else {
            json!({})
        };
        let replay_safe = is_replay_safe(&name);

        let first = self.request_socket(&name, &payload);
        let socket_error = match first {
            Ok(result) => {
                return Ok(ApiOutcome {
                    ok: true,
                    method: name,
                    via: "socket",
                    result,
                    error: None,
                })
            }
            Err(e) => e,
        };

        // Authoritative remote error: never retry or fall back.
        if socket_error.stage == ErrorStage::Remote {
            return Err(ApiError::new(socket_error.message));
        }

        // If request bytes may have reached the wire and the method is not
        // replay-safe, the outcome is indeterminate. Do not resend, do not CLI.
        let bytes_maybe_sent = socket_error.stage == ErrorStage::AfterSend;
        let mut socket_error_msg = socket_error.message;

        if bytes_maybe_sent && !replay_safe {
            return Err(ApiError::new(format!(
                "{name} outcome indeterminate: request may have been delivered; not retrying a non-idempotent method ({socket_error_msg})"
            )));
        }

        // Retry once on an owned client for replay-safe reads (or any
        // BeforeSend failure). Injected clients are never reconnected.
        if self.owned_client && (replay_safe || !bytes_maybe_sent) {
            self.close();
            match self.open() {
                Ok(()) => match self.request_socket(&name, &payload) {
                    Ok(result) => {
                        return Ok(ApiOutcome {
                            ok: true,
                            method: name,
                            via: "socket",
                            result,
                            error: None,
                        })
                    }
                    Err(e) => {
                        if e.stage == ErrorStage::Remote {
                            return Err(ApiError::new(e.message));
                        }
                        if e.stage == ErrorStage::AfterSend && !replay_safe {
                            return Err(ApiError::new(format!(
                                "{name} outcome indeterminate on retry ({})",
                                e.message
                            )));
                        }
                        socket_error_msg = e.message;
                    }
                },
                Err(e) => socket_error_msg = e.0,
            }
        }

        if socket_only {
            return Err(ApiError::new(if socket_error_msg.is_empty() {
                format!("{name} socket request failed")
            } else {
                socket_error_msg
            }));
        }

        // CLI fallback is only safe when no socket byte was written.
        if bytes_maybe_sent {
            return Err(ApiError::new(format!(
                "{name} outcome indeterminate: request may have been delivered; no CLI fallback ({socket_error_msg})"
            )));
        }

        let argv = build_cli_argv(&name, &payload).ok_or_else(|| {
            ApiError::new(if socket_error_msg.is_empty() {
                format!("{name} has no CLI fallback (socket required)")
            } else {
                socket_error_msg.clone()
            })
        })?;

        match self.cli_request(&argv) {
            Ok(result) => Ok(ApiOutcome {
                ok: true,
                method: name,
                via: "cli",
                result,
                error: None,
            }),
            Err(e) => {
                let detail = if e.0.is_empty() {
                    socket_error_msg
                } else {
                    e.0
                };
                Err(ApiError::new(format!("{name} failed: {detail}")))
            }
        }
    }

    /// Prefer an explicit path, then `HERDR_SOCKET_PATH` if it exists
    /// (`_resolved_socket_path`).
    fn resolved_socket_path(&self) -> Option<String> {
        let mut path = self.socket_path.clone().or_else(socket_path_from_env);
        if path.is_none() {
            path = std::env::var("HERDR_SOCKET_PATH")
                .ok()
                .filter(|p| !p.is_empty());
        }
        let path = path?;
        if path.is_empty() || !Path::new(&path).exists() {
            return None;
        }
        Some(path)
    }

    /// One RPC on the persistent client, or a one-shot connection
    /// (`_request_socket` / `_socket_request`).
    fn request_socket(&mut self, method: &str, params: &Value) -> Result<Value, SocketError> {
        if let Some(client) = self.client.as_mut() {
            return client.request(method, params.clone());
        }
        let path = self
            .resolved_socket_path()
            .ok_or_else(|| SocketError::before_send("Herdr socket not available"))?;
        let mut client = SocketClient::new(path, self.timeout);
        client.connect()?;
        let out = client.request(method, params.clone());
        client.close();
        out
    }

    /// Run `herdr <argv>` once via the injected runner (`_cli_request`). Without
    /// a runner there is no CLI fallback in the Rust build — the plugin never
    /// shells out on its own; a spawn-based runner is wired at the CLI layer.
    fn cli_request(&self, argv: &[String]) -> Result<Value> {
        match self.cli_runner {
            Some(runner) => runner.run(argv),
            None => Err(ApiError::new("herdr CLI fallback not configured")),
        }
    }
}

/// Set of allowlisted methods, for tests / introspection.
pub fn allowed_methods() -> BTreeSet<&'static str> {
    ALLOWED_METHODS.iter().copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forbidden_and_prefixes_rejected() {
        assert!(assert_method_allowed("server.stop").is_err());
        assert!(assert_method_allowed("pane.graphics.frame").is_err());
        assert!(assert_method_allowed("plugin.install_anything").is_err());
        assert!(assert_method_allowed("").is_err());
        assert!(assert_method_allowed("does.not.exist").is_err());
    }

    #[test]
    fn allowlist_membership() {
        assert_eq!(assert_method_allowed(" ping ").unwrap(), "ping");
        assert!(assert_method_allowed("pane.split").is_ok());
        assert_eq!(allowed_methods().len(), ALLOWED_METHODS.len());
    }

    #[test]
    fn argv_pane_current_has_no_flag() {
        // Documented quirk: pane.current → ["pane","current"], no --current.
        assert_eq!(
            build_cli_argv("pane.current", &json!({})),
            Some(vec!["pane".to_string(), "current".to_string()])
        );
    }

    #[test]
    fn argv_layout_export_tab_only_is_socket_only() {
        assert_eq!(
            build_cli_argv("layout.export", &json!({"tab_id": "t1"})),
            None
        );
        assert_eq!(
            build_cli_argv("layout.export", &json!({})),
            Some(vec![
                "pane".to_string(),
                "layout".to_string(),
                "--current".to_string()
            ])
        );
    }

    #[test]
    fn argv_pane_read_defaults_recent() {
        assert_eq!(
            build_cli_argv("pane.read", &json!({"pane_id": "p1"})),
            Some(vec![
                "pane".into(),
                "read".into(),
                "p1".into(),
                "--source".into(),
                "recent".into()
            ])
        );
    }

    #[test]
    fn argv_tab_move_is_socket_only() {
        // Documented quirk: tab.move has no CLI wrapper.
        assert_eq!(build_cli_argv("tab.move", &json!({"tab_id": "t1"})), None);
    }

    #[test]
    fn argv_pane_close_drops_force() {
        // Documented quirk: CLI fallback drops the `force` flag.
        assert_eq!(
            build_cli_argv("pane.close", &json!({"pane_id": "p1", "force": true})),
            Some(vec!["pane".into(), "close".into(), "p1".into()])
        );
    }

    #[test]
    fn replay_safe_classification() {
        assert!(is_replay_safe("session.snapshot"));
        assert!(is_replay_safe("pane.read"));
        assert!(!is_replay_safe("pane.split"));
        assert!(!is_replay_safe("pane.close"));
        assert!(!is_replay_safe("workspace.create"));
    }

    #[test]
    fn extract_read_text_prefers_keys() {
        assert_eq!(extract_read_text(&json!({"text": "hi"})), "hi");
        assert_eq!(extract_read_text(&json!({"lines": ["a", "b"]})), "a\nb");
        assert_eq!(extract_read_text(&json!({"result": {"content": "x"}})), "x");
        assert_eq!(extract_read_text(&json!(["a", 1])), "a\n1");
    }

    #[test]
    fn extract_agent_status_walks_nested() {
        assert_eq!(
            extract_agent_status(&json!({"pane": {"agent_status": "working"}})),
            Some("working".into())
        );
        assert_eq!(extract_agent_status(&json!({})), None);
    }
}
