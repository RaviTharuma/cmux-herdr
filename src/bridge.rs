//! Herdr topology fetch: subprocess + socket orchestration and payload parsing.
//!
//! Port of the topology layer of `bridge/cmux_herdr_bridge.py` (`which`,
//! `run_cmd`, `_parse_json_payload`, `herdr_json`, `cmux_cmd`,
//! `herdr_available`, `cmux_available`, `fetch_panes/tabs/workspaces/agents`,
//! `fetch_layouts_raw`, `fetch_snapshot`, `fetch_snapshot_via_socket`).
//!
//! The pure payload→model parsers (`panes_from_list`, `tabs_from_list`,
//! `workspaces_from_list`, `agents_from_list`) are factored out so they can be
//! golden-tested against Python without spawning subprocesses.

use std::process::Command;
use std::time::Duration;

use serde_json::Value;

use crate::api::{ApiError, HerdrApi};
use crate::model::{
    pane_from_raw, snapshot_from_session_payload, tab_from_raw, workspace_from_raw, Pane, Snapshot,
    Tab, Workspace,
};

/// User-facing bridge failure (`BridgeError`).
#[derive(Debug, Clone, PartialEq)]
pub struct BridgeError(pub String);

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for BridgeError {}

impl BridgeError {
    pub fn new(msg: impl Into<String>) -> Self {
        BridgeError(msg.into())
    }
}

type Result<T> = std::result::Result<T, BridgeError>;

/// Default subprocess timeout Python passes to `run_cmd` (`timeout=15.0`).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);

/// `shutil.which(cmd)` — resolve an executable on `PATH`.
pub fn which(cmd: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(cmd);
        if is_executable(&candidate) {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
        Ok(meta) => meta.is_file() && meta.permissions().mode() & 0o111 != 0,
        Err(_) => false,
    }
}

/// Result of a subprocess run (`subprocess.CompletedProcess` subset).
pub struct CmdOutput {
    pub returncode: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Run a command, capturing output (`run_cmd`). `env` entries are merged onto
/// the inherited environment. Stdout/stderr are drained concurrently so a
/// noisy child cannot deadlock on a full pipe while the timeout is polled.
pub fn run_cmd(
    args: &[&str],
    timeout: Duration,
    env: Option<&[(&str, &str)]>,
) -> std::io::Result<CmdOutput> {
    use std::io::Read;
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;

    let (program, rest) = args
        .split_first()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing program"))?;
    let mut cmd = Command::new(program);
    cmd.args(rest)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    if let Some(env) = env {
        for (k, v) in env {
            cmd.env(k, v);
        }
    }
    let mut child = cmd.spawn()?;
    let process_group =
        rustix::process::Pid::from_raw(child.id() as i32).expect("child process id is positive");
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    let stdout_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).map(|_| bytes)
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).map(|_| bytes)
    });

    let deadline = std::time::Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            let _ =
                rustix::process::kill_process_group(process_group, rustix::process::Signal::Kill);
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("command timed out after {:.3}s", timeout.as_secs_f64()),
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| std::io::Error::other("stdout reader panicked"))??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| std::io::Error::other("stderr reader panicked"))??;
    Ok(CmdOutput {
        returncode: status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

/// Parse a JSON payload from CLI stdout (`_parse_json_payload`).
///
/// Tolerates trailing noise: on a whole-string parse failure, retry the first
/// line that starts with `{` or `[`.
pub fn parse_json_payload(stdout: &str) -> Result<Value> {
    let text = stdout.trim();
    if text.is_empty() {
        return Err(BridgeError::new("empty JSON response"));
    }
    if let Ok(v) = serde_json::from_str::<Value>(text) {
        return Ok(v);
    }
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('{') || line.starts_with('[') {
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                return Ok(v);
            }
            // Python lets the last json.loads raise; mirror by returning err.
            return Err(BridgeError::new("invalid JSON response"));
        }
    }
    Err(BridgeError::new("invalid JSON response"))
}

/// Invoke `herdr` with `args`, returning parsed JSON (`herdr_json`).
pub fn herdr_json(args: &[&str]) -> Result<Value> {
    if which("herdr").is_none() {
        return Err(BridgeError::new("herdr not found on PATH"));
    }
    let mut full = vec!["herdr"];
    full.extend_from_slice(args);
    let proc = run_cmd(&full, DEFAULT_TIMEOUT, None)
        .map_err(|e| BridgeError::new(format!("herdr {}: {e}", args.join(" "))))?;
    if proc.returncode != 0 {
        let err = {
            let s = proc.stderr.trim();
            if !s.is_empty() {
                s.to_string()
            } else {
                let o = proc.stdout.trim();
                if !o.is_empty() {
                    o.to_string()
                } else {
                    proc.returncode.to_string()
                }
            }
        };
        return Err(BridgeError::new(format!(
            "herdr {} failed: {err}",
            args.join(" ")
        )));
    }
    parse_json_payload(&proc.stdout)
}

/// Run `cmux` with `args`, appending `--workspace` when absent (`cmux_cmd`).
pub fn cmux_cmd(args: &[&str], workspace: Option<&str>) -> Result<CmdOutput> {
    if which("cmux").is_none() {
        return Err(BridgeError::new("cmux not found on PATH"));
    }
    let mut full: Vec<String> = std::iter::once("cmux".to_string())
        .chain(args.iter().map(|s| s.to_string()))
        .collect();
    if let Some(ws) = workspace {
        if !ws.is_empty() && !full.iter().any(|a| a == "--workspace") {
            full.push("--workspace".into());
            full.push(ws.to_string());
        }
    }
    let refs: Vec<&str> = full.iter().map(String::as_str).collect();
    run_cmd(&refs, DEFAULT_TIMEOUT, None).map_err(|e| BridgeError::new(format!("cmux failed: {e}")))
}

/// True when `herdr` is on PATH and reachable (`herdr_available`).
pub fn herdr_available() -> bool {
    if which("herdr").is_none() {
        return false;
    }
    if let Some(sock) = std::env::var("HERDR_SOCKET_PATH")
        .ok()
        .filter(|s| !s.is_empty())
    {
        if std::path::Path::new(&sock).exists() {
            return true;
        }
    }
    match run_cmd(&["herdr", "status"], Duration::from_secs(5), None) {
        Ok(proc) => proc.returncode == 0,
        Err(_) => std::env::var("HERDR_ENV")
            .map(|v| !v.is_empty())
            .unwrap_or(false),
    }
}

/// True when `cmux` is on PATH and reachable (`cmux_available`).
pub fn cmux_available() -> bool {
    if which("cmux").is_none() {
        return false;
    }
    if let Some(sock) = std::env::var("CMUX_SOCKET_PATH")
        .ok()
        .filter(|s| !s.is_empty())
    {
        if std::path::Path::new(&sock).exists() {
            return true;
        }
    }
    match run_cmd(
        &["cmux", "identify", "--json"],
        Duration::from_secs(5),
        None,
    ) {
        Ok(proc) => proc.returncode == 0,
        Err(_) => false,
    }
}

// --- pure payload parsers ----------------------------------------------------

/// `(data.get("result") or {}).get(key) or []` → the raw list.
fn result_list<'a>(data: &'a Value, key: &str) -> &'a [Value] {
    let result = data.get("result").filter(|v| !v.is_null());
    let inner = result.and_then(|r| r.get(key)).filter(|v| !v.is_null());
    match inner {
        Some(Value::Array(a)) => a.as_slice(),
        _ => &[],
    }
}

/// Parse a `pane list` payload into panes (`fetch_panes` body). Drops entries
/// without a truthy `pane_id`.
pub fn panes_from_list(data: &Value) -> Vec<Pane> {
    result_list(data, "panes")
        .iter()
        .filter(|p| truthy(p.get("pane_id")))
        .map(pane_from_raw)
        .collect()
}

/// Parse a `tab list` payload into tabs (`fetch_tabs` body).
pub fn tabs_from_list(data: &Value) -> Vec<Tab> {
    result_list(data, "tabs").iter().map(tab_from_raw).collect()
}

/// Parse a `workspace list` payload into workspaces (`fetch_workspaces` body).
///
/// Handles the three shapes Python tolerates: `result.workspaces`,
/// `result.workspace_list`, or a bare `result` list.
pub fn workspaces_from_list(data: &Value) -> Vec<Workspace> {
    let result = data.get("result").filter(|v| !v.is_null());
    let raw: &[Value] = match result {
        Some(Value::Array(a)) => a.as_slice(),
        Some(obj @ Value::Object(_)) => {
            let ws = obj.get("workspaces").filter(|v| !v.is_null());
            let ws = ws.or_else(|| obj.get("workspace_list").filter(|v| !v.is_null()));
            match ws {
                Some(Value::Array(a)) => a.as_slice(),
                _ => &[],
            }
        }
        _ => &[],
    };
    raw.iter().map(workspace_from_raw).collect()
}

/// Parse an `agent list` payload into panes (`fetch_agents` primary path).
/// Returns `None` when the payload has no agents (caller falls back).
pub fn agents_from_list(data: &Value) -> Option<Vec<Pane>> {
    let agents = result_list(data, "agents");
    if agents.is_empty() {
        return None;
    }
    Some(
        agents
            .iter()
            .filter(|a| truthy(a.get("pane_id")))
            .map(pane_from_raw)
            .collect(),
    )
}

// --- fetch orchestration -----------------------------------------------------

/// Fetch panes over the CLI (`fetch_panes`).
pub fn fetch_panes(workspace_id: Option<&str>) -> Result<Vec<Pane>> {
    let mut args = vec!["pane", "list"];
    if let Some(ws) = workspace_id.filter(|w| !w.is_empty()) {
        args.push("--workspace");
        args.push(ws);
    }
    Ok(panes_from_list(&herdr_json(&args)?))
}

/// Fetch tabs over the CLI (`fetch_tabs`).
pub fn fetch_tabs() -> Result<Vec<Tab>> {
    Ok(tabs_from_list(&herdr_json(&["tab", "list"])?))
}

/// Fetch workspaces over the CLI (`fetch_workspaces`).
pub fn fetch_workspaces() -> Result<Vec<Workspace>> {
    Ok(workspaces_from_list(&herdr_json(&["workspace", "list"])?))
}

/// Fetch agents (pane-shaped), falling back to panes declaring an agent
/// (`fetch_agents`).
pub fn fetch_agents() -> Result<Vec<Pane>> {
    if let Ok(data) = herdr_json(&["agent", "list"]) {
        if let Some(agents) = agents_from_list(&data) {
            return Ok(agents);
        }
    }
    Ok(fetch_panes(None)?
        .into_iter()
        .filter(|p| p.agent.is_some())
        .collect())
}

/// Best-effort Herdr layout payload (`fetch_layouts_raw`). Prefers
/// `api snapshot`, then `pane layout --current`.
pub fn fetch_layouts_raw() -> Value {
    let attempts: [&[&str]; 2] = [&["api", "snapshot"], &["pane", "layout", "--current"]];
    for args in attempts {
        if let Ok(data) = herdr_json(args) {
            if truthy(Some(&data)) {
                return data;
            }
        }
    }
    Value::Object(serde_json::Map::new())
}

/// Fetch topology, preferring a live socket snapshot (`fetch_snapshot`).
///
/// `api` is the process-wide [`HerdrApi`] used for the socket-first path.
pub fn fetch_snapshot(api: &mut HerdrApi) -> Result<Snapshot> {
    if let Some(snap) = fetch_snapshot_via_socket(api) {
        return Ok(snap);
    }
    let panes = fetch_panes(None)?;
    let tabs = fetch_tabs().unwrap_or_default();
    let workspaces = fetch_workspaces().unwrap_or_default();
    let layouts = fetch_layouts_raw();
    Ok(Snapshot {
        panes,
        tabs,
        workspaces,
        layouts,
    })
}

/// Return a snapshot from `session.snapshot` over the socket, or `None`
/// (`fetch_snapshot_via_socket`). Never raises for transport failures.
pub fn fetch_snapshot_via_socket(api: &mut HerdrApi) -> Option<Snapshot> {
    match api.call("session.snapshot", Value::Object(Default::default()), true) {
        Ok(outcome) => snapshot_from_session_payload(&outcome.result),
        Err(_) => None,
    }
}

/// Build the process-wide topology `HerdrApi`, opening the socket best-effort
/// (`_herdr_api`). Transport errors on `open` are swallowed (CLI fallback
/// still works).
pub fn herdr_api<'r>() -> HerdrApi<'r> {
    let socket_path = std::env::var("HERDR_SOCKET_PATH")
        .ok()
        .filter(|s| !s.is_empty());
    let mut api = HerdrApi::new(socket_path, DEFAULT_TIMEOUT);
    let _: std::result::Result<(), ApiError> = api.open();
    api
}

/// Socket-first RPC returning the bare `result`, wrapping `ApiError`
/// (`herdr_rpc`).
pub fn herdr_rpc(api: &mut HerdrApi, method: &str, params: Value) -> Result<Value> {
    let params = if params.is_object() {
        params
    } else {
        Value::Object(Default::default())
    };
    match api.call(method, params, false) {
        Ok(outcome) => Ok(outcome.result),
        Err(e) => Err(BridgeError::new(e.to_string())),
    }
}

/// Python truthiness of an optional JSON value.
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_json_payload_tolerates_trailing_noise() {
        assert_eq!(parse_json_payload("{\"a\":1}").unwrap(), json!({"a": 1}));
        // whole-string fails, first {-line wins
        let out = parse_json_payload("warn: hi\n{\"ok\":true}\nmore").unwrap();
        assert_eq!(out, json!({"ok": true}));
        // array line
        assert_eq!(parse_json_payload("noise\n[1,2]").unwrap(), json!([1, 2]));
    }

    #[test]
    fn parse_json_payload_empty_errors() {
        assert_eq!(
            parse_json_payload("   ").unwrap_err(),
            BridgeError::new("empty JSON response")
        );
    }

    #[test]
    fn run_cmd_enforces_timeout() {
        let err = run_cmd(&["sh", "-c", "sleep 1"], Duration::from_millis(20), None)
            .err()
            .expect("command should time out");
        assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
    }

    #[test]
    fn panes_from_list_drops_idless() {
        let data = json!({"result": {"panes": [
            {"pane_id": "p1", "tab_id": "t1"},
            {"pane_id": "", "tab_id": "t2"},
            {"tab_id": "t3"},
        ]}});
        let panes = panes_from_list(&data);
        assert_eq!(panes.len(), 1);
        assert_eq!(panes[0].pane_id, "p1");
    }

    #[test]
    fn panes_from_list_handles_missing_result() {
        assert!(panes_from_list(&json!({})).is_empty());
        assert!(panes_from_list(&json!({"result": null})).is_empty());
        assert!(panes_from_list(&json!({"result": {"panes": null}})).is_empty());
    }

    #[test]
    fn tabs_from_list_parses() {
        let data = json!({"result": {"tabs": [
            {"tab_id": "t1", "workspace_id": "w1", "label": "L", "pane_count": 3, "focused": true}
        ]}});
        let tabs = tabs_from_list(&data);
        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs[0].tab_id, "t1");
        assert_eq!(tabs[0].pane_count, 3);
        assert!(tabs[0].focused);
    }

    #[test]
    fn workspaces_from_list_three_shapes() {
        // result.workspaces
        let a = json!({"result": {"workspaces": [{"workspace_id": "w1"}]}});
        assert_eq!(workspaces_from_list(&a).len(), 1);
        // result.workspace_list
        let b = json!({"result": {"workspace_list": [{"workspace_id": "w2"}]}});
        assert_eq!(workspaces_from_list(&b).len(), 1);
        // bare result list
        let c = json!({"result": [{"workspace_id": "w3"}, {"workspace_id": "w4"}]});
        assert_eq!(workspaces_from_list(&c).len(), 2);
    }

    #[test]
    fn agents_from_list_none_when_empty() {
        assert!(agents_from_list(&json!({"result": {"agents": []}})).is_none());
        assert!(agents_from_list(&json!({})).is_none());
        let some = agents_from_list(&json!({"result": {"agents": [{"pane_id": "p1"}]}}));
        assert_eq!(some.unwrap().len(), 1);
    }
}
