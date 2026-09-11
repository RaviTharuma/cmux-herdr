//! Nest Herdr workspaces as ordinary cmux workspaces under a host group.
//!
//! Uses cmux `workspace-group` plus `new-workspace` / `workspace create` so
//! Herdr workspaces show in the left sidebar as normal workspaces/machines
//! nav entries (ssh-tmux fashion: one remote session → one sidebar workspace).
//! Never invents Herdr-only left chrome.

use serde_json::{json, Map, Value};

use crate::bridge::{self, CmdOutput};
use crate::model::{Snapshot, Workspace};
use crate::state::{self, Fingerprint, HostEnv};

const BINDINGS_KEY: &str = "workspace_bindings";
const GROUP_ID_KEY: &str = "workspace_group_id";
const GROUP_NAME_KEY: &str = "workspace_group_name";
const GROUP_NAME: &str = "Herdr";

/// Outcome of one nest reconcile against cmux.
#[derive(Debug, Clone, PartialEq)]
pub struct NestReport {
    pub ok: bool,
    pub skipped_reason: Option<String>,
    pub group_id: Option<String>,
    pub created: Vec<String>,
    pub reused: Vec<String>,
    pub renamed: Vec<String>,
    pub pruned: Vec<String>,
    pub bindings: Map<String, Value>,
    pub errors: Vec<String>,
}

impl NestReport {
    pub(crate) fn skipped(reason: impl Into<String>) -> Self {
        Self {
            ok: true,
            skipped_reason: Some(reason.into()),
            group_id: None,
            created: Vec::new(),
            reused: Vec::new(),
            renamed: Vec::new(),
            pruned: Vec::new(),
            bindings: Map::new(),
            errors: Vec::new(),
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "ok": self.ok,
            "skipped_reason": self.skipped_reason,
            "group_id": self.group_id,
            "created": self.created,
            "reused": self.reused,
            "renamed": self.renamed,
            "pruned": self.pruned,
            "bindings": self.bindings,
            "errors": self.errors,
        })
    }
}

/// Which cmux verbs are available for nesting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestCapability {
    pub workspace_group: bool,
    pub new_workspace: bool,
    pub workspace_create: bool,
}

impl NestCapability {
    pub fn supported(self) -> bool {
        self.workspace_group && (self.new_workspace || self.workspace_create)
    }
}

fn help_mentions(output: &CmdOutput, needle: &str) -> bool {
    output.returncode == 0
        && (output.stdout.to_lowercase().contains(needle)
            || output.stderr.to_lowercase().contains(needle))
}

/// Probe cmux CLI for workspace-group / workspace-create support (fail closed).
pub fn probe_nest_capability() -> NestCapability {
    let workspace_group = bridge::cmux_cmd(&["workspace-group", "--help"], None)
        .ok()
        .is_some_and(|out| help_mentions(&out, "create") || help_mentions(&out, "usage"));
    let new_workspace = bridge::cmux_cmd(&["new-workspace", "--help"], None)
        .ok()
        .is_some_and(|out| help_mentions(&out, "usage") || help_mentions(&out, "--name"));
    let workspace_create = bridge::cmux_cmd(&["workspace", "create", "--help"], None)
        .ok()
        .is_some_and(|out| help_mentions(&out, "usage") || help_mentions(&out, "--name"));
    NestCapability {
        workspace_group,
        new_workspace,
        workspace_create,
    }
}

fn display_name(workspace: &Workspace) -> String {
    workspace
        .label
        .as_deref()
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .unwrap_or(workspace.workspace_id.as_str())
        .to_string()
}

fn extract_id(payload: &Value, object_key: &str, id_keys: &[&str]) -> Option<String> {
    if let Some(obj) = payload.get(object_key) {
        for key in id_keys {
            if let Some(id) = obj.get(*key).and_then(Value::as_str) {
                let id = id.trim();
                if !id.is_empty() {
                    return Some(id.to_string());
                }
            }
        }
    }
    for key in id_keys {
        if let Some(id) = payload.get(*key).and_then(Value::as_str) {
            let id = id.trim();
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
    }
    None
}

fn parse_ok_id(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        let line = line.trim();
        let Some(rest) = line
            .strip_prefix("OK ")
            .or_else(|| line.strip_prefix("ok "))
        else {
            continue;
        };
        let id = rest.split_whitespace().next().unwrap_or("").trim();
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }
    None
}

fn cmux_json(args: &[&str]) -> Result<Value, String> {
    let mut full = Vec::with_capacity(args.len() + 1);
    full.push("--json");
    full.extend_from_slice(args);
    let output = bridge::cmux_cmd(&full, None)
        .map_err(|error| format!("cmux {}: {error}", args.join(" ")))?;
    if output.returncode != 0 {
        return Err(format!(
            "cmux {} failed (exit {}): {}",
            args.join(" "),
            output.returncode,
            output.stderr.trim()
        ));
    }
    match bridge::parse_json_payload(&output.stdout) {
        Ok(value) => Ok(value),
        Err(_) => Ok(json!({"_raw": output.stdout})),
    }
}

fn cmux_ok_id(args: &[&str], object_key: &str, id_keys: &[&str]) -> Result<String, String> {
    let payload = cmux_json(args)?;
    if let Some(id) = extract_id(&payload, object_key, id_keys) {
        return Ok(id);
    }
    if let Some(Value::String(raw)) = payload.get("_raw") {
        if let Some(id) = parse_ok_id(raw) {
            return Ok(id);
        }
    }
    Err(format!("cmux {} returned no id: {payload}", args.join(" ")))
}

fn ensure_group(
    host_workspace: &str,
    fingerprint_key: &str,
    prior_group: Option<&str>,
) -> Result<String, String> {
    if let Some(group_id) = prior_group.map(str::trim).filter(|id| !id.is_empty()) {
        return Ok(group_id.to_string());
    }
    let idempotency = format!("cmux-herdr-nest-{fingerprint_key}");
    cmux_ok_id(
        &[
            "workspace-group",
            "create",
            "--name",
            GROUP_NAME,
            "--from",
            host_workspace,
            "--idempotency-key",
            &idempotency,
            "--external-id",
            &idempotency,
        ],
        "group",
        &["id", "group_id", "ref", "workspace_group_id"],
    )
}

fn create_child_workspace(
    capability: NestCapability,
    title: &str,
    group_id: &str,
) -> Result<String, String> {
    if capability.workspace_create {
        return cmux_ok_id(
            &[
                "workspace",
                "create",
                "--name",
                title,
                "--group",
                group_id,
                "--focus",
                "false",
            ],
            "workspace",
            &["id", "workspace_id", "workspace_ref", "ref"],
        );
    }
    cmux_ok_id(
        &[
            "new-workspace",
            "--name",
            title,
            "--group",
            group_id,
            "--focus",
            "false",
        ],
        "workspace",
        &["id", "workspace_id", "workspace_ref", "ref"],
    )
}

fn rename_child_workspace(cmux_workspace_id: &str, title: &str) -> Result<(), String> {
    let modern = bridge::cmux_cmd(
        &["workspace", "rename", cmux_workspace_id, "--name", title],
        None,
    );
    if let Ok(output) = modern {
        if output.returncode == 0 {
            return Ok(());
        }
    }
    let legacy = bridge::cmux_cmd(
        &["rename-workspace", cmux_workspace_id, "--name", title],
        None,
    )
    .map_err(|error| error.to_string())?;
    if legacy.returncode == 0 {
        Ok(())
    } else {
        Err(format!(
            "rename workspace {cmux_workspace_id} failed: {}",
            legacy.stderr.trim()
        ))
    }
}

fn add_to_group(group_id: &str, cmux_workspace_id: &str) -> Result<(), String> {
    let output = bridge::cmux_cmd(
        &[
            "workspace-group",
            "add",
            "--group",
            group_id,
            "--workspace",
            cmux_workspace_id,
        ],
        None,
    )
    .map_err(|error| error.to_string())?;
    if output.returncode == 0 {
        return Ok(());
    }
    let combined = format!("{} {}", output.stdout, output.stderr).to_lowercase();
    if combined.contains("already") || combined.contains("member") {
        Ok(())
    } else {
        Err(format!(
            "workspace-group add failed: {}",
            output.stderr.trim()
        ))
    }
}

fn close_child_workspace(cmux_workspace_id: &str) -> Result<(), String> {
    let modern = bridge::cmux_cmd(&["workspace", "close", cmux_workspace_id], None);
    if let Ok(output) = modern {
        if output.returncode == 0 {
            return Ok(());
        }
    }
    let legacy = bridge::cmux_cmd(&["close-workspace", cmux_workspace_id], None)
        .map_err(|error| error.to_string())?;
    if legacy.returncode == 0 {
        Ok(())
    } else {
        Err(format!(
            "close workspace {cmux_workspace_id} failed: {}",
            legacy.stderr.trim()
        ))
    }
}

fn binding_record(herdr_id: &str, cmux_id: &str, title: &str, now: f64) -> Value {
    json!({
        "herdr_workspace_id": herdr_id,
        "cmux_workspace_id": cmux_id,
        "title": title,
        "updated_at": now,
    })
}

/// Reconcile Herdr workspaces into cmux left-nav workspace-group members.
///
/// `host_workspace` is the outer cmux workspace that hosts Herdr (machine face /
/// group anchor). Children are ordinary cmux workspaces — not foreign chrome.
pub fn reconcile_nested_workspaces(
    env: &dyn HostEnv,
    fingerprint: &Fingerprint,
    host_workspace: &str,
    snapshot: &Snapshot,
    prune: bool,
) -> NestReport {
    let host_workspace = host_workspace.trim();
    if host_workspace.is_empty() {
        return NestReport::skipped("missing host workspace");
    }
    let capability = probe_nest_capability();
    if !capability.supported() {
        return NestReport::skipped(
            "cmux workspace-group + new-workspace/workspace create unavailable; left-nav nesting skipped (fail closed)",
        );
    }

    let fingerprint_key = state::parent_key(fingerprint);
    let mut associations = state::load_association_map(env, fingerprint);
    let prior_group = associations
        .get(GROUP_ID_KEY)
        .and_then(Value::as_str)
        .map(str::to_string);
    let mut prior_bindings = associations
        .get(BINDINGS_KEY)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let group_id = match ensure_group(host_workspace, &fingerprint_key, prior_group.as_deref()) {
        Ok(id) => id,
        Err(error) => {
            let mut report = NestReport::skipped(error);
            report.ok = false;
            return report;
        }
    };

    let mut report = NestReport {
        ok: true,
        skipped_reason: None,
        group_id: Some(group_id.clone()),
        created: Vec::new(),
        reused: Vec::new(),
        renamed: Vec::new(),
        pruned: Vec::new(),
        bindings: Map::new(),
        errors: Vec::new(),
    };

    let now = env.now();
    let mut seen = std::collections::BTreeSet::new();

    for workspace in &snapshot.workspaces {
        let herdr_id = workspace.workspace_id.trim();
        if herdr_id.is_empty() {
            continue;
        }
        seen.insert(herdr_id.to_string());
        let title = display_name(workspace);
        let prior = prior_bindings.get(herdr_id).cloned();
        let prior_cmux = prior
            .as_ref()
            .and_then(|value| value.get("cmux_workspace_id"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_string);
        let prior_title = prior
            .as_ref()
            .and_then(|value| value.get("title"))
            .and_then(Value::as_str)
            .unwrap_or("");

        let cmux_id = if let Some(existing) = prior_cmux {
            if prior_title != title {
                if let Err(error) = rename_child_workspace(&existing, &title) {
                    report.errors.push(error);
                    report.ok = false;
                } else {
                    report.renamed.push(herdr_id.to_string());
                }
            }
            if let Err(error) = add_to_group(&group_id, &existing) {
                report.errors.push(error);
                report.ok = false;
            }
            report.reused.push(herdr_id.to_string());
            existing
        } else {
            match create_child_workspace(capability, &title, &group_id) {
                Ok(id) => {
                    report.created.push(herdr_id.to_string());
                    id
                }
                Err(error) => {
                    report.errors.push(error);
                    report.ok = false;
                    continue;
                }
            }
        };

        prior_bindings.insert(
            herdr_id.to_string(),
            binding_record(herdr_id, &cmux_id, &title, now),
        );
    }

    if prune {
        let stale: Vec<String> = prior_bindings
            .keys()
            .filter(|key| !seen.contains(key.as_str()))
            .cloned()
            .collect();
        for herdr_id in stale {
            if let Some(cmux_id) = prior_bindings
                .get(&herdr_id)
                .and_then(|value| value.get("cmux_workspace_id"))
                .and_then(Value::as_str)
            {
                if let Err(error) = close_child_workspace(cmux_id) {
                    report.errors.push(error);
                    report.ok = false;
                    continue;
                }
            }
            prior_bindings.remove(&herdr_id);
            report.pruned.push(herdr_id);
        }
    }

    report.bindings = prior_bindings.clone();

    if let Some(object) = associations.as_object_mut() {
        object.insert(GROUP_ID_KEY.into(), json!(group_id));
        object.insert(GROUP_NAME_KEY.into(), json!(GROUP_NAME));
        object.insert(BINDINGS_KEY.into(), Value::Object(prior_bindings));
        object.insert("cmux_workspace".into(), json!(host_workspace));
    }
    if let Err(error) = state::save_association_map(env, &associations, fingerprint) {
        report.errors.push(format!("persist associations: {error}"));
        report.ok = false;
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_prefers_label() {
        let labeled = Workspace {
            workspace_id: "ws-a".into(),
            label: Some("Alpha".into()),
            number: None,
            agent_status: "idle".into(),
            focused: false,
            pane_count: 1,
            tab_count: 1,
            raw: json!({}),
        };
        assert_eq!(display_name(&labeled), "Alpha");
        let bare = Workspace {
            label: None,
            ..labeled
        };
        assert_eq!(display_name(&bare), "ws-a");
    }

    #[test]
    fn extract_id_reads_nested_and_top_level() {
        let nested = json!({"group":{"id":"g1"}});
        assert_eq!(
            extract_id(&nested, "group", &["id", "group_id"]).as_deref(),
            Some("g1")
        );
        let top = json!({"workspace_id":"w9"});
        assert_eq!(
            extract_id(&top, "workspace", &["workspace_id", "id"]).as_deref(),
            Some("w9")
        );
    }

    #[test]
    fn parse_ok_id_reads_cli_line() {
        assert_eq!(
            parse_ok_id("OK workspace:abc\n").as_deref(),
            Some("workspace:abc")
        );
        assert_eq!(parse_ok_id("noise\n").as_deref(), None);
    }

    #[test]
    fn nest_report_json_includes_skip_reason() {
        let report = NestReport::skipped("no api");
        assert_eq!(report.to_json()["skipped_reason"], "no api");
        assert_eq!(report.to_json()["ok"], true);
    }
}
