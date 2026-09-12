//! Right-rail projection for cmux-native surfaces.
//!
//! Moshi Desktop deep-integrates Herdr via a loopback gateway (workspaces,
//! agents, focus, transcripts). cmux already owns a closed right-rail enum
//! (`files|find|vault|sessions|feed|dock|cloud` — see manaflow-ai/cmux#11707).
//! This module projects Herdr agents into that chrome without a second Agents
//! list and without Moshi Chat View / web gateway chrome.
//!
//! Dedup key is the same `herdr:<pane_id>` used by status pills so
//! `sessions` / coding-agent rows and pills never double-count.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Map, Value};

use crate::model::{Pane, Snapshot, STATUS_PREFIX};
use crate::state::{self, Fingerprint, HostEnv};
use crate::status::{map_status_to_style, status_value_for_pane};

/// Schema version for `rail-<fingerprint>.json`.
pub const RAIL_SCHEMA_VERSION: i64 = 1;

/// Closed-enum right-rail modes this projection feeds (never invents new ones).
pub const RAIL_MODES: &[&str] = &["sessions", "feed", "dock"];

/// One deduped agent row for the right rail (`sessions` / Agents-adjacent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RailAgent {
    pub status_key: String,
    pub pane_id: String,
    pub tab_id: String,
    pub workspace_id: String,
    pub agent: String,
    pub agent_status: String,
    pub display_name: String,
    pub value: String,
    pub icon: String,
    pub priority: i64,
    pub focused: bool,
}

impl RailAgent {
    /// Serialize one agent row for JSON consumers.
    pub fn to_json(&self) -> Value {
        json!({
            "status_key": self.status_key,
            "pane_id": self.pane_id,
            "tab_id": self.tab_id,
            "workspace_id": self.workspace_id,
            "agent": self.agent,
            "agent_status": self.agent_status,
            "display_name": self.display_name,
            "value": self.value,
            "icon": self.icon,
            "priority": self.priority,
            "focused": self.focused,
            // Same identity pill consumers already use — no parallel keyspace.
            "dedupe_with": ["cmux.set-status", "sessions"],
        })
    }
}

/// Collect agent panes the same way `sync` does (named agent first; else
/// known statuses), then dedupe by `status_key`.
pub fn agent_panes_for_rail(snapshot: &Snapshot) -> Vec<&Pane> {
    let named: Vec<&Pane> = snapshot
        .panes
        .iter()
        .filter(|pane| pane.agent.as_ref().is_some_and(|agent| !agent.is_empty()))
        .collect();
    let mut panes = if named.is_empty() {
        snapshot
            .panes
            .iter()
            .filter(|pane| {
                matches!(
                    pane.agent_status.as_str(),
                    "working" | "idle" | "done" | "blocked"
                )
            })
            .collect::<Vec<_>>()
    } else {
        named
    };
    // Stable order: workspace, tab, pane — then dedupe keeps first.
    panes.sort_by(|a, b| {
        (
            a.workspace_id.as_str(),
            a.tab_id.as_str(),
            a.pane_id.as_str(),
        )
            .cmp(&(
                b.workspace_id.as_str(),
                b.tab_id.as_str(),
                b.pane_id.as_str(),
            ))
    });
    dedupe_panes_by_status_key(panes)
}

/// Keep the first pane per `herdr:<pane_id>` (Moshi Jump To identity → one row).
pub fn dedupe_panes_by_status_key(panes: Vec<&Pane>) -> Vec<&Pane> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::with_capacity(panes.len());
    for pane in panes {
        let key = pane.status_key();
        if seen.insert(key) {
            out.push(pane);
        }
    }
    out
}

/// Build [`RailAgent`] rows from a snapshot (deduped).
pub fn rail_agents_from_snapshot(snapshot: &Snapshot) -> Vec<RailAgent> {
    agent_panes_for_rail(snapshot)
        .into_iter()
        .map(|pane| {
            let style = map_status_to_style(Some(&pane.agent_status));
            let display_name = pane.display_name();
            let value = status_value_for_pane(pane, None, None, None);
            RailAgent {
                status_key: pane.status_key(),
                pane_id: pane.pane_id.clone(),
                tab_id: pane.tab_id.clone(),
                workspace_id: pane.workspace_id.clone(),
                agent: pane
                    .agent
                    .clone()
                    .filter(|a| !a.is_empty())
                    .unwrap_or_else(|| "agent".into()),
                agent_status: pane.agent_status.to_lowercase(),
                display_name,
                value,
                icon: style.icon.to_string(),
                priority: style.priority,
                focused: pane.focused,
            }
        })
        .collect()
}

/// Dock quick actions: CLI verbs only (no foreign Dock UI chrome).
pub fn dock_actions(agents: &[RailAgent]) -> Vec<Value> {
    let mut actions = vec![
        json!({
            "id": "focus-agent",
            "label": "Focus agent",
            "cli": "cmux-herdr focus-agent <pane_id>",
            "mode": "dock",
        }),
        json!({
            "id": "attach-pane",
            "label": "Attach pane viewer",
            "cli": "cmux-herdr attach-pane --pane <pane_id>",
            "mode": "dock",
        }),
        json!({
            "id": "sessions",
            "label": "List sessions",
            "cli": "cmux-herdr sessions --json",
            "mode": "sessions",
        }),
    ];
    if let Some(focused) = agents.iter().find(|agent| agent.focused) {
        actions.push(json!({
            "id": "focus-focused",
            "label": format!("Focus {}", focused.display_name),
            "cli": format!("cmux-herdr focus-agent {}", focused.pane_id),
            "pane_id": focused.pane_id,
            "mode": "dock",
        }));
    }
    actions
}

/// Feed events for status transitions that deserve attention (blocked/done).
/// Soft-consumed by `cmux right-sidebar set feed` when host supports it.
pub fn feed_events_from_transition(previous: Option<&Value>, agents: &[RailAgent]) -> Vec<Value> {
    let prior_status = previous
        .and_then(|value| value.get("agents"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let key = row.get("status_key")?.as_str()?;
                    let status = row.get("agent_status")?.as_str()?;
                    Some((key.to_string(), status.to_string()))
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();

    let mut events = Vec::new();
    for agent in agents {
        let prev = prior_status.get(&agent.status_key).map(String::as_str);
        let now = agent.agent_status.as_str();
        if prev == Some(now) {
            continue;
        }
        if !matches!(now, "blocked" | "done") {
            continue;
        }
        events.push(json!({
            "kind": format!("herdr.agent.{now}"),
            "status_key": agent.status_key,
            "pane_id": agent.pane_id,
            "agent": agent.agent,
            "from": prev,
            "to": now,
            "title": format!("{} {}", agent.agent, now),
            "body": agent.value,
            "mode": "feed",
            "priority": agent.priority,
        }));
    }
    events
}

/// Full right-rail snapshot payload (`method: cmux.herdr.rail`).
pub fn build_rail_payload(
    snapshot: &Snapshot,
    fingerprint: &Fingerprint,
    socket_path: &str,
    previous: Option<&Value>,
    cmux_workspace: Option<&str>,
) -> Value {
    let agents = rail_agents_from_snapshot(snapshot);
    let feed_events = feed_events_from_transition(previous, &agents);
    let dock = dock_actions(&agents);
    let sessions = crate::live::sessions_list_payload(snapshot, socket_path);
    let agent_json: Vec<Value> = agents.iter().map(RailAgent::to_json).collect();
    let mut counts = Map::new();
    for key in ["working", "idle", "done", "blocked", "unknown", "other"] {
        counts.insert(key.into(), json!(0));
    }
    for agent in &agents {
        let bucket = if counts.contains_key(agent.agent_status.as_str()) {
            agent.agent_status.as_str()
        } else {
            "other"
        };
        counts[bucket] = json!(counts[bucket].as_i64().unwrap_or(0) + 1);
    }

    json!({
        "ok": true,
        "method": "cmux.herdr.rail",
        "schema_version": RAIL_SCHEMA_VERSION,
        "status_prefix": STATUS_PREFIX,
        "host_fingerprint_key": state::parent_key(fingerprint),
        "cmux_workspace": cmux_workspace,
        "socket": socket_path,
        // Closed-enum modes only — never invent RightSidebarMode.herdr.
        "modes": RAIL_MODES,
        "guidance": {
            "sessions": "cmux right-sidebar set sessions  # Agents-adjacent; rows match herdr:<pane_id> pills",
            "feed": "cmux right-sidebar set feed       # optional; consume feed_events on blocked/done",
            "dock": "cmux right-sidebar set dock       # quick actions → CLI verbs only",
            "vault": "leave alone (not Herdr)",
            "avoid": [
                "cmux sidebar open herdr",
                "cmux sidebar select herdr",
                "cmux right-sidebar set custom herdr",
                "Moshi Chat View / web gateway chrome",
            ],
        },
        "agents": agent_json,
        "agent_count": agents.len(),
        "counts": counts,
        "sessions": sessions.get("sessions").cloned().unwrap_or_else(|| json!([])),
        "sessions_method": sessions.get("method").cloned().unwrap_or(json!("remote.herdr.sessions")),
        "feed_events": feed_events,
        "dock_actions": dock,
        "dedupe": {
            "key": "status_key",
            "format": "herdr:<pane_id>",
            "also_written_by": ["sync", "watch"],
        },
    })
}

/// Path for `rail-<fingerprint>.json`.
pub fn rail_path(env: &dyn HostEnv, fp: &Fingerprint) -> std::path::PathBuf {
    state::state_dir(env).join(format!("rail-{}.json", state::parent_key(fp)))
}

/// Load a previously written rail snapshot, if present and schema-valid.
pub fn load_rail_snapshot(env: &dyn HostEnv, fp: &Fingerprint) -> Option<Value> {
    let path = rail_path(env, fp);
    let text = env.read_file(path.to_str()?)?;
    let data: Value = serde_json::from_str(&text).ok()?;
    if data.get("schema_version") == Some(&json!(RAIL_SCHEMA_VERSION))
        && data.get("method") == Some(&json!("cmux.herdr.rail"))
    {
        Some(data)
    } else {
        None
    }
}

/// Persist a rail snapshot atomically (0600), same fashion as associations.
pub fn save_rail_snapshot(
    env: &dyn HostEnv,
    fp: &Fingerprint,
    payload: &Value,
) -> std::io::Result<()> {
    let dir = state::state_dir(env);
    let mut body = payload.as_object().cloned().unwrap_or_default();
    body.insert("schema_version".into(), json!(RAIL_SCHEMA_VERSION));
    body.insert("updated_at".into(), json!(env.now()));
    body.insert(
        "host_fingerprint_key".into(),
        Value::String(state::parent_key(fp)),
    );
    let encoded = serde_json::to_string_pretty(&Value::Object(body))
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    atomic_write_rail(&dir, &rail_path(env, fp), &format!("{encoded}\n"))
}

fn atomic_write_rail(
    dir: &std::path::Path,
    target: &std::path::Path,
    body: &str,
) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .or_else(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                Ok(())
            } else {
                Err(error)
            }
        })?;
    let mut tmp = tempfile::Builder::new()
        .prefix(".rail-")
        .suffix(".tmp")
        .tempfile_in(dir)?;
    tmp.write_all(body.as_bytes())?;
    tmp.flush()?;
    std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o600))?;
    tmp.persist(target).map_err(|error| error.error)?;
    Ok(())
}

/// Build + persist rail snapshot for sync/watch. Soft-fails I/O (returns payload).
pub fn project_and_persist(
    env: &dyn HostEnv,
    snapshot: &Snapshot,
    fingerprint: &Fingerprint,
    socket_path: &str,
    cmux_workspace: Option<&str>,
) -> Value {
    let previous = load_rail_snapshot(env, fingerprint);
    let payload = build_rail_payload(
        snapshot,
        fingerprint,
        socket_path,
        previous.as_ref(),
        cmux_workspace,
    );
    let _ = save_rail_snapshot(env, fingerprint, &payload);
    payload
}

/// Best-effort: push feed_events through `cmux log` (native chrome). Never
/// invents a feed tab; never fails the caller hard.
pub fn soft_publish_feed_events(workspace: Option<&str>, payload: &Value) {
    let Some(events) = payload.get("feed_events").and_then(Value::as_array) else {
        return;
    };
    for event in events {
        let title = event
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("herdr");
        let body = event.get("body").and_then(Value::as_str).unwrap_or("");
        let line = if body.is_empty() {
            format!("herdr feed: {title}")
        } else {
            format!("herdr feed: {title} — {body}")
        };
        let _ = crate::bridge::cmux_cmd(&["log", &line], workspace);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::pane_from_raw;

    fn pane(id: &str, agent: &str, status: &str) -> Pane {
        pane_from_raw(&json!({
            "pane_id": id,
            "tab_id": "t1",
            "workspace_id": "w1",
            "agent": agent,
            "agent_status": status,
            "terminal_title": format!("{agent}-title"),
        }))
    }

    #[test]
    fn dedupe_keeps_first_status_key_only() {
        let a = pane("p1", "bot", "working");
        let b = pane("p1", "bot", "idle"); // same pane_id → same status_key
        let c = pane("p2", "codex", "blocked");
        let panes = vec![&a, &b, &c];
        let deduped = dedupe_panes_by_status_key(panes);
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].pane_id, "p1");
        assert_eq!(deduped[0].agent_status, "working");
        assert_eq!(deduped[1].pane_id, "p2");
    }

    #[test]
    fn rail_payload_uses_herdr_status_keys_and_closed_modes() {
        let snapshot = Snapshot {
            panes: vec![pane("p1", "pi", "working"), pane("p2", "codex", "done")],
            tabs: vec![],
            workspaces: vec![],
            layouts: json!({}),
        };
        let fp = Fingerprint {
            cmux_surface_id: Some("surface-1".into()),
            herdr_socket_path: Some("/tmp/herdr.sock".into()),
            herdr_server_pid: None,
            herdr_workspace_id: Some("w1".into()),
        };
        let payload = build_rail_payload(&snapshot, &fp, "/tmp/herdr.sock", None, Some("ws:1"));
        assert_eq!(payload["method"], "cmux.herdr.rail");
        assert_eq!(payload["schema_version"], RAIL_SCHEMA_VERSION);
        assert_eq!(payload["modes"], json!(["sessions", "feed", "dock"]));
        let agents = payload["agents"].as_array().unwrap();
        assert_eq!(agents.len(), 2);
        assert_eq!(agents[0]["status_key"], "herdr:p1");
        assert_eq!(agents[1]["status_key"], "herdr:p2");
        assert_eq!(payload["dedupe"]["format"], "herdr:<pane_id>");
        assert!(payload["dock_actions"].as_array().unwrap().len() >= 3);
    }

    #[test]
    fn feed_events_only_on_blocked_or_done_transitions() {
        let agents = vec![
            RailAgent {
                status_key: "herdr:p1".into(),
                pane_id: "p1".into(),
                tab_id: "t1".into(),
                workspace_id: "w1".into(),
                agent: "pi".into(),
                agent_status: "blocked".into(),
                display_name: "pi".into(),
                value: "pi/blocked".into(),
                icon: "exclamationmark.triangle".into(),
                priority: 90,
                focused: false,
            },
            RailAgent {
                status_key: "herdr:p2".into(),
                pane_id: "p2".into(),
                tab_id: "t1".into(),
                workspace_id: "w1".into(),
                agent: "codex".into(),
                agent_status: "working".into(),
                display_name: "codex".into(),
                value: "codex/working".into(),
                icon: "hammer".into(),
                priority: 80,
                focused: false,
            },
        ];
        let previous = json!({
            "agents": [
                {"status_key": "herdr:p1", "agent_status": "working"},
                {"status_key": "herdr:p2", "agent_status": "working"},
            ]
        });
        let events = feed_events_from_transition(Some(&previous), &agents);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["kind"], "herdr.agent.blocked");
        assert_eq!(events[0]["mode"], "feed");
    }

    /// Isolated HostEnv so parallel tests do not thrash process XDG_STATE_HOME.
    struct RailTestEnv {
        xdg: std::path::PathBuf,
        files: std::cell::RefCell<std::collections::HashMap<String, String>>,
    }

    impl HostEnv for RailTestEnv {
        fn var(&self, name: &str) -> Option<String> {
            if name == "XDG_STATE_HOME" {
                return Some(self.xdg.to_string_lossy().into_owned());
            }
            None
        }
        fn now(&self) -> f64 {
            1_700_000_000.0
        }
        fn read_file(&self, path: &str) -> Option<String> {
            if let Some(text) = self.files.borrow().get(path).cloned() {
                return Some(text);
            }
            std::fs::read_to_string(path).ok()
        }
    }

    #[test]
    fn rail_snapshot_roundtrip_under_xdg() {
        let tmp = tempfile::tempdir().unwrap();
        let env = RailTestEnv {
            xdg: tmp.path().join("xdg"),
            files: std::cell::RefCell::new(std::collections::HashMap::new()),
        };
        std::fs::create_dir_all(&env.xdg).unwrap();
        let snapshot = Snapshot {
            panes: vec![
                pane("p1", "pi", "working"),
                pane("p1", "pi", "idle"),
                pane("p3", "bot", "done"),
            ],
            tabs: vec![],
            workspaces: vec![],
            layouts: json!({}),
        };
        let fp = Fingerprint {
            cmux_surface_id: Some("surface-rail".into()),
            herdr_socket_path: Some("/tmp/herdr-rail.sock".into()),
            herdr_server_pid: None,
            herdr_workspace_id: None,
        };
        let payload = project_and_persist(&env, &snapshot, &fp, "/tmp/herdr-rail.sock", None);
        assert_eq!(payload["agent_count"], 2);
        assert_eq!(payload["agents"][0]["status_key"], "herdr:p1");
        let loaded = load_rail_snapshot(&env, &fp).expect("persisted rail");
        assert_eq!(loaded["method"], "cmux.herdr.rail");
        assert_eq!(loaded["schema_version"], RAIL_SCHEMA_VERSION);
        assert_eq!(loaded["agents"].as_array().unwrap().len(), 2);
        let path = rail_path(&env, &fp);
        assert!(path.exists());
        assert!(path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("rail-"));
    }
}
