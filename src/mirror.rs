//! Herdr topology projection and idempotent cmux mirror planning.
//!
//! Behavioral port of the pure planner surface in
//! `bridge/cmux_herdr_mirror.py`. Host-side cmux execution remains outside this
//! module; this module owns desired projection, reconcile actions, JSON parsing,
//! and human-readable plan formatting.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::env;

use serde_json::{json, Map, Value};

use crate::impose::specs_with_impose_fractions;
use crate::layout::{layouts_by_tab_id, pane_is_zoomed, pane_rects_from_dicts, parse_layout, tree_from_rects, LayoutNode, SplitSpec};
use crate::model::{Pane, Snapshot, Tab};

pub const ATTACH_ENV: &str = "CMUX_HERDR_ATTACH_PANE";
pub const SIZE_AUTHORITY_ENV: &str = "CMUX_HERDR_SIZE_AUTHORITY";
pub const MIRROR_KEY_PREFIX: &str = "herdr-mirror:";
pub const DEFAULT_ATTACH_INTERVAL: f64 = 0.25;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorError(pub String);

impl std::fmt::Display for MirrorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for MirrorError {}

/// One Herdr pane that should exist as a cmux surface (`DesiredMirror`).
#[derive(Debug, Clone, PartialEq)]
pub struct DesiredMirror {
    pub pane_id: String,
    pub tab_id: String,
    pub workspace_id: String,
    pub title: String,
    pub role: String,
    pub split_direction: String,
    pub agent: Option<String>,
    pub agent_status: String,
    pub split_ratio: Option<f64>,
    pub split_from_pane_id: Option<String>,
    pub tab_number: Option<Value>,
    pub tab_index: Option<i64>,
    pub focused: bool,
    pub zoomed: bool,
    pub visible: bool,
}

impl DesiredMirror {
    pub fn key(&self) -> String {
        mirror_key_for_pane(&self.pane_id)
    }
}

/// One mirror reconcile step (`MirrorAction`).
#[derive(Debug, Clone, PartialEq)]
pub struct MirrorAction {
    pub op: String,
    pub pane_id: String,
    pub title: String,
    pub tab_id: String,
    pub role: String,
    pub split_direction: String,
    pub key: String,
    pub surface_id: Option<String>,
    pub split_from_surface_id: Option<String>,
    pub split_from_pane_id: Option<String>,
    pub ratio: Option<f64>,
    pub tab_index: Option<i64>,
    pub reason: String,
}

/// Ordered mirror reconcile plan (`MirrorPlan`).
#[derive(Debug, Clone, PartialEq)]
pub struct MirrorPlan {
    pub actions: Vec<MirrorAction>,
    pub scope: String,
    pub desired_count: usize,
}

impl Default for MirrorPlan {
    fn default() -> Self {
        Self {
            actions: Vec::new(),
            scope: "current-tab".to_string(),
            desired_count: 0,
        }
    }
}

impl MirrorPlan {
    fn matching(&self, operation: &str) -> Vec<&MirrorAction> {
        self.actions.iter().filter(|action| action.op == operation).collect()
    }

    pub fn creates(&self) -> Vec<&MirrorAction> {
        self.actions
            .iter()
            .filter(|action| matches!(action.op.as_str(), "create_tab" | "create_split"))
            .collect()
    }

    pub fn renames(&self) -> Vec<&MirrorAction> {
        self.matching("rename")
    }

    pub fn prunes(&self) -> Vec<&MirrorAction> {
        self.matching("prune")
    }

    pub fn keeps(&self) -> Vec<&MirrorAction> {
        self.matching("keep")
    }

    pub fn ratio_updates(&self) -> Vec<&MirrorAction> {
        self.matching("set_ratio")
    }

    pub fn moves(&self) -> Vec<&MirrorAction> {
        self.matching("move_tab")
    }

    pub fn focuses(&self) -> Vec<&MirrorAction> {
        self.matching("focus")
    }
}

pub fn mirror_key_for_pane(pane_id: &str) -> String {
    format!("{MIRROR_KEY_PREFIX}{pane_id}")
}

pub fn is_attach_process() -> bool {
    env::var_os(ATTACH_ENV).is_some_and(|value| !value.is_empty())
}

fn tab_label(tab: Option<&Tab>) -> Option<String> {
    let tab = tab?;
    if let Some(raw) = tab.raw.get("label") {
        return truthy(Some(raw)).then(|| python_str(Some(raw)));
    }
    tab.label.as_deref().filter(|value| !value.is_empty()).map(str::to_string)
}

fn pane_title(pane: &Pane, tab: Option<&Tab>, role: &str) -> String {
    if role == "tab-root" {
        if let Some(label) = tab_label(tab) {
            return label;
        }
    }
    let name = pane.display_name();
    if !name.is_empty() && name != pane.pane_id {
        return name;
    }
    if let Some(agent) = pane.agent.as_deref().filter(|value| !value.is_empty()) {
        return format!("{agent}@{}", pane.pane_id);
    }
    if let Some(label) = tab_label(tab) {
        return label;
    }
    pane.pane_id.clone()
}

fn split_direction_for_index(index: usize) -> &'static str {
    if index % 2 == 1 { "right" } else { "down" }
}

fn tab_layout_node(snapshot: &Snapshot, tab_id: &str, members: &[&Pane]) -> Option<LayoutNode> {
    if let Some(node) = layouts_by_tab_id(&snapshot.layouts).remove(tab_id) {
        return Some(node);
    }
    let raw_panes: Vec<Value> = members.iter().map(|pane| pane.raw.clone()).collect();
    let rects = pane_rects_from_dicts(&raw_panes);
    match rects.len() {
        0 => None,
        1 => {
            let (pane_id, rect) = &rects[0];
            parse_layout(&json!({
                "pane_id": pane_id,
                "x": rect.x,
                "y": rect.y,
                "width": rect.width,
                "height": rect.height,
            }))
        }
        _ => tree_from_rects(&rects),
    }
}

fn truncate_chars(value: String, limit: usize) -> String {
    value.chars().take(limit).collect()
}

fn tab_number_for_sort(tab: &Tab) -> i64 {
    match tab.raw.get("number") {
        Some(Value::Bool(value)) => i64::from(*value),
        Some(Value::Number(value)) if value.is_i64() || value.is_u64() => {
            value.as_i64().or_else(|| value.as_u64().and_then(|v| i64::try_from(v).ok())).unwrap_or(1_000_000_000)
        }
        _ => tab.number.unwrap_or(1_000_000_000),
    }
}

/// Build the desired cmux projection from a Herdr snapshot (`desired_mirrors`).
pub fn desired_mirrors(
    snapshot: &Snapshot,
    scope: &str,
    current_tab_id: Option<&str>,
    current_workspace_id: Option<&str>,
    use_layout: bool,
) -> Result<Vec<DesiredMirror>, MirrorError> {
    if !matches!(scope, "current-tab" | "workspace" | "all") {
        return Err(MirrorError("scope must be current-tab, workspace, or all".to_string()));
    }

    let tabs_by_id: HashMap<&str, &Tab> = snapshot
        .tabs
        .iter()
        .filter(|tab| !tab.tab_id.is_empty())
        .map(|tab| (tab.tab_id.as_str(), tab))
        .collect();
    let mut panes: Vec<&Pane> = snapshot.panes.iter().filter(|pane| !pane.pane_id.is_empty()).collect();

    if scope == "current-tab" {
        let environment = env::var("HERDR_TAB_ID").ok();
        let tab_id = current_tab_id
            .filter(|value| !value.is_empty())
            .or_else(|| environment.as_deref().filter(|value| !value.is_empty()))
            .ok_or_else(|| MirrorError(
                "scope current-tab needs HERDR_TAB_ID or --tab (or pass --all / --herdr-workspace)".to_string(),
            ))?;
        panes.retain(|pane| pane.tab_id == tab_id);
    } else if scope == "workspace" {
        let environment = env::var("HERDR_WORKSPACE_ID").ok();
        let workspace_id = current_workspace_id
            .filter(|value| !value.is_empty())
            .or_else(|| environment.as_deref().filter(|value| !value.is_empty()))
            .ok_or_else(|| MirrorError(
                "scope workspace needs HERDR_WORKSPACE_ID or --herdr-workspace".to_string(),
            ))?;
        panes.retain(|pane| pane.workspace_id == workspace_id);
    }

    let mut grouped: HashMap<String, Vec<&Pane>> = HashMap::new();
    for pane in panes {
        let tab_id = if pane.tab_id.is_empty() { &pane.pane_id } else { &pane.tab_id };
        grouped.entry(tab_id.clone()).or_default().push(pane);
    }

    let mut ordered_tab_ids: Vec<String> = grouped.keys().cloned().collect();
    ordered_tab_ids.sort_by(|left, right| {
        match (tabs_by_id.get(left.as_str()), tabs_by_id.get(right.as_str())) {
            (Some(left_tab), Some(right_tab)) => tab_number_for_sort(left_tab)
                .cmp(&tab_number_for_sort(right_tab))
                .then_with(|| left.cmp(right)),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => left.cmp(right),
        }
    });

    let mut desired = Vec::new();
    for (tab_index, tab_id) in ordered_tab_ids.iter().enumerate() {
        let mut members = grouped.remove(tab_id).unwrap_or_default();
        let tab = tabs_by_id.get(tab_id.as_str()).copied();
        let mut spec_by_id: HashMap<String, SplitSpec> = HashMap::new();
        let mut order = Vec::new();
        if use_layout {
            if let Some(node) = tab_layout_node(snapshot, tab_id, &members) {
                order = node.pane_ids_in_order();
                spec_by_id = specs_with_impose_fractions(&node)
                    .into_iter()
                    .map(|spec| (spec.pane_id.clone(), spec))
                    .collect();
            }
        }
        let order_index: HashMap<&str, usize> = order
            .iter()
            .enumerate()
            .map(|(index, pane_id)| (pane_id.as_str(), index))
            .collect();
        members.sort_by(|left, right| {
            match (order_index.get(left.pane_id.as_str()), order_index.get(right.pane_id.as_str())) {
                (Some(left_index), Some(right_index)) => left_index.cmp(right_index).then_with(|| left.pane_id.cmp(&right.pane_id)),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => (!left.focused).cmp(&!right.focused).then_with(|| left.pane_id.cmp(&right.pane_id)),
            }
        });

        let zoomed_id = members
            .iter()
            .find(|pane| pane.focused && pane_is_zoomed(&pane.raw))
            .or_else(|| members.iter().find(|pane| pane_is_zoomed(&pane.raw)))
            .map(|pane| pane.pane_id.as_str());

        for (index, pane) in members.into_iter().enumerate() {
            let spec = spec_by_id.get(&pane.pane_id);
            let role = if index == 0 { "tab-root" } else { "split" };
            desired.push(DesiredMirror {
                pane_id: pane.pane_id.clone(),
                tab_id: tab_id.clone(),
                workspace_id: pane.workspace_id.clone(),
                title: truncate_chars(pane_title(pane, tab, role), 80),
                role: role.to_string(),
                split_direction: spec
                    .map(|value| value.direction.clone())
                    .unwrap_or_else(|| split_direction_for_index(index).to_string()),
                agent: pane.agent.clone(),
                agent_status: if pane.agent_status.is_empty() { "unknown".to_string() } else { pane.agent_status.clone() },
                split_ratio: spec.and_then(|value| value.ratio),
                split_from_pane_id: spec.map(|value| value.split_from_pane_id.clone()),
                tab_number: tab.and_then(|value| {
                    value.raw.get("number").cloned().filter(|raw| !raw.is_null())
                        .or_else(|| value.number.map(|number| json!(number)))
                }),
                tab_index: Some(tab_index as i64),
                focused: pane.focused,
                zoomed: zoomed_id == Some(pane.pane_id.as_str()),
                visible: zoomed_id.is_none() || zoomed_id == Some(pane.pane_id.as_str()),
            });
        }
    }
    Ok(desired)
}

fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(value)) => *value,
        Some(Value::Number(value)) => value.as_f64().is_some_and(|value| value != 0.0),
        Some(Value::String(value)) => !value.is_empty(),
        Some(Value::Array(value)) => !value.is_empty(),
        Some(Value::Object(value)) => !value.is_empty(),
    }
}

fn python_repr_string(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') { '"' } else { '\'' };
    let mut output = String::with_capacity(text.len() + 2);
    output.push(quote);
    for character in text.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            value if value == quote => {
                output.push('\\');
                output.push(value);
            }
            value if value.is_control() => {
                let code = value as u32;
                if code <= 0xff {
                    output.push_str(&format!("\\x{code:02x}"));
                } else if code <= 0xffff {
                    output.push_str(&format!("\\u{code:04x}"));
                } else {
                    output.push_str(&format!("\\U{code:08x}"));
                }
            }
            value => output.push(value),
        }
    }
    output.push(quote);
    output
}

fn python_repr(value: &Value) -> String {
    match value {
        Value::Null => "None".to_string(),
        Value::Bool(true) => "True".to_string(),
        Value::Bool(false) => "False".to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => python_repr_string(value),
        Value::Array(values) => format!(
            "[{}]",
            values.iter().map(python_repr).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(values) => format!(
            "{{{}}}",
            values
                .iter()
                .map(|(key, value)| format!("{}: {}", python_repr_string(key), python_repr(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn python_str(value: Option<&Value>) -> String {
    match value {
        None => "None".to_string(),
        Some(Value::String(value)) => value.clone(),
        Some(value) => python_repr(value),
    }
}

fn value_or_default_string(value: Option<&Value>, default: &str) -> String {
    if truthy(value) { python_str(value) } else { default.to_string() }
}

fn string_set(value: Option<&Value>) -> HashSet<String> {
    value
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default()
}

fn base_action(item: &DesiredMirror, op: &str, surface: Option<String>, reason: &str) -> MirrorAction {
    MirrorAction {
        op: op.to_string(),
        pane_id: item.pane_id.clone(),
        title: item.title.clone(),
        tab_id: item.tab_id.clone(),
        role: item.role.clone(),
        split_direction: item.split_direction.clone(),
        key: item.key(),
        surface_id: surface,
        split_from_surface_id: None,
        split_from_pane_id: item.split_from_pane_id.clone(),
        ratio: item.split_ratio,
        tab_index: item.tab_index,
        reason: reason.to_string(),
    }
}

fn python_numeric_equals(value: Option<&Value>, expected: f64) -> bool {
    match value {
        Some(Value::Bool(value)) => (if *value { 1.0 } else { 0.0 }) == expected,
        Some(Value::Number(value)) => value.as_f64() == Some(expected),
        _ => false,
    }
}

fn python_int(value: Option<&Value>) -> Option<i64> {
    match value {
        Some(Value::Bool(value)) => Some(i64::from(*value)),
        Some(Value::Number(value)) if value.is_i64() || value.is_u64() => {
            value.as_i64().or_else(|| value.as_u64().and_then(|raw| i64::try_from(raw).ok()))
        }
        _ => None,
    }
}

/// Diff desired panes against the persisted mirror map (`plan_mirror`).
#[allow(clippy::too_many_arguments)]
pub fn plan_mirror(
    desired: &[DesiredMirror],
    existing: &Value,
    live_surface_ids: Option<&HashSet<String>>,
    prune: bool,
    sync_focus: bool,
    sync_order: bool,
    sync_ratios: bool,
    engine: Option<&Value>,
) -> MirrorPlan {
    let existing_mirrors = existing.as_object();
    let desired_ids: HashSet<&str> = desired.iter().map(|item| item.pane_id.as_str()).collect();
    let mut tab_root_surface: HashMap<String, String> = HashMap::new();
    let mut actions = Vec::new();
    let engine_hints = engine.and_then(Value::as_object).filter(|value| !value.is_empty());
    let protected = string_set(engine_hints.and_then(|value| value.get("protected_pane_ids")));
    let engine_closed = string_set(engine_hints.and_then(|value| value.get("closed_pane_ids")));
    let engine_created = engine_hints.map(|value| string_set(value.get("created_pane_ids")));

    let mapped_surface = |pane_id: &str| -> Option<String> {
        let entry = existing_mirrors?.get(pane_id)?.as_object()?;
        let surface = entry.get("cmux_surface_id")?.as_str()?;
        if surface.is_empty() || live_surface_ids.is_some_and(|live| !live.contains(surface)) {
            None
        } else {
            Some(surface.to_string())
        }
    };

    for item in desired {
        let surface = mapped_surface(&item.pane_id);
        let entry = existing_mirrors.and_then(|values| values.get(&item.pane_id)).and_then(Value::as_object);
        let prior_title = value_or_default_string(entry.and_then(|value| value.get("title")), "");
        let title_locked = truthy(entry.and_then(|value| value.get("title_lock")));
        if item.role == "tab-root" {
            if let Some(value) = surface.as_ref() {
                tab_root_surface.insert(item.tab_id.clone(), value.clone());
            }
        }
        if let Some(surface) = surface {
            if prior_title != "" && prior_title != item.title && !title_locked {
                actions.push(base_action(item, "rename", Some(surface), "title changed"));
            } else {
                actions.push(base_action(item, "keep", Some(surface), ""));
            }
            continue;
        }

        if let Some(created) = engine_created.as_ref() {
            if !created.contains(&item.pane_id) && !protected.contains(&item.pane_id) {
                continue;
            }
        }
        if item.role == "tab-root" {
            actions.push(base_action(item, "create_tab", None, "missing tab-root surface"));
        } else {
            let mut action = base_action(item, "create_split", None, "missing split surface");
            action.split_from_surface_id = tab_root_surface.get(&item.tab_id).cloned();
            actions.push(action);
        }
    }

    if prune {
        let mut pane_ids: Vec<&String> = existing_mirrors
            .map(|values| values.keys().collect())
            .unwrap_or_default();
        pane_ids.sort();
        for pane_id in pane_ids {
            if desired_ids.contains(pane_id.as_str()) || protected.contains(pane_id) {
                continue;
            }
            let Some(entry) = existing_mirrors.and_then(|values| values.get(pane_id)).and_then(Value::as_object) else {
                continue;
            };
            if engine_hints.is_some() && !engine_closed.contains(pane_id) {
                continue;
            }
            actions.push(MirrorAction {
                op: "prune".to_string(),
                pane_id: pane_id.clone(),
                title: value_or_default_string(entry.get("title"), pane_id),
                tab_id: value_or_default_string(entry.get("tab_id"), ""),
                role: value_or_default_string(entry.get("role"), "split"),
                split_direction: "right".to_string(),
                key: value_or_default_string(entry.get("key"), &mirror_key_for_pane(pane_id)),
                surface_id: entry.get("cmux_surface_id").and_then(Value::as_str).map(str::to_string),
                split_from_surface_id: None,
                split_from_pane_id: None,
                ratio: None,
                tab_index: None,
                reason: "herdr pane gone".to_string(),
            });
        }
    }

    if sync_ratios {
        for item in desired {
            let Some(ratio) = item.split_ratio.filter(|_| item.role == "split") else { continue };
            let Some(surface) = mapped_surface(&item.pane_id) else { continue };
            let prior = existing_mirrors
                .and_then(|values| values.get(&item.pane_id))
                .and_then(Value::as_object)
                .and_then(|value| value.get("split_ratio"));
            if !python_numeric_equals(prior, ratio) {
                actions.push(base_action(item, "set_ratio", Some(surface), "layout ratio"));
            }
        }
    }

    if sync_order {
        for item in desired {
            let Some(tab_index) = item.tab_index.filter(|_| item.role == "tab-root") else { continue };
            let surface = mapped_surface(&item.pane_id);
            let prior = existing_mirrors
                .and_then(|values| values.get(&item.pane_id))
                .and_then(Value::as_object)
                .and_then(|value| python_int(value.get("tab_index")));
            if surface.is_some() && prior == Some(tab_index) {
                continue;
            }
            actions.push(base_action(item, "move_tab", surface, "herdr tab order"));
        }
    }

    if sync_focus {
        let focused = desired.iter().find(|item| item.focused).or_else(|| desired.iter().find(|item| item.zoomed));
        let prior_focused = existing_mirrors.and_then(|values| {
            values.iter().find_map(|(pane_id, entry)| {
                truthy(entry.as_object().and_then(|value| value.get("focused"))).then(|| pane_id.as_str())
            })
        });
        if let Some(item) = focused.filter(|item| prior_focused != Some(item.pane_id.as_str())) {
            actions.push(base_action(item, "focus", mapped_surface(&item.pane_id), "herdr focused pane"));
        }
    }

    MirrorPlan {
        actions,
        scope: "current-tab".to_string(),
        desired_count: desired.len(),
    }
}

/// Parse cmux CLI JSON, tolerating a leading `OK` or other non-JSON lines.
pub fn parse_cmux_json(stdout: &str) -> Result<Option<Value>, serde_json::Error> {
    let text = stdout.trim();
    if text.is_empty() {
        return Ok(None);
    }
    match serde_json::from_str(text) {
        Ok(value) => Ok(Some(value)),
        Err(_) => {
            for line in text.lines().map(str::trim) {
                if line.starts_with('{') || line.starts_with('[') {
                    return serde_json::from_str(line).map(Some);
                }
            }
            Ok(None)
        }
    }
}

fn object_or_empty(value: Option<&Value>) -> Option<&Map<String, Value>> {
    value.and_then(Value::as_object)
}

fn sequence_len(value: Option<&Value>) -> usize {
    if !truthy(value) {
        return 0;
    }
    match value.unwrap() {
        Value::Array(values) => values.len(),
        Value::Object(values) => values.len(),
        Value::String(value) => value.chars().count(),
        _ => 0,
    }
}

fn string_sequence(value: Option<&Value>) -> Vec<String> {
    if !truthy(value) {
        return Vec::new();
    }
    value
        .and_then(Value::as_array)
        .map(|values| values.iter().map(|value| value.as_str().unwrap_or("").to_string()).collect())
        .unwrap_or_default()
}

/// Render the human-readable mirror reconcile summary (`format_mirror_plan`).
pub fn format_mirror_plan(result: &Value) -> String {
    let result = result.as_object();
    let get = |key: &str| result.and_then(|value| value.get(key));
    if truthy(get("native_live")) || get("skipped_reason").and_then(Value::as_str) == Some("native_live") {
        let writer = value_or_default_string(get("writer"), "native");
        return format!(
            "herdr → cmux mirror  SKIPPED (native attachment live; writer={writer})  set CMUX_HERDR_FORCE_PLUGIN=1 to force plugin projection"
        );
    }

    let plan = object_or_empty(get("plan"));
    let plan_get = |key: &str| plan.and_then(|value| value.get(key));
    let scope = python_str(get("scope"));
    let desired_count = get("desired_count")
        .map(|value| python_str(Some(value)))
        .unwrap_or_else(|| "0".to_string());
    let workspace = value_or_default_string(get("workspace"), "-");
    let mut first = format!("herdr → cmux mirror  scope={scope}  desired={desired_count}  cmux_ws={workspace}");
    if truthy(plan_get("dry_run")) {
        first.push_str("  DRY-RUN");
    }
    let created = string_sequence(plan_get("created"));
    let renamed = string_sequence(plan_get("renamed"));
    let pruned = string_sequence(plan_get("pruned"));
    let mut lines = vec![
        first,
        format!("  created {}: {}", created.len(), created.join(", ")),
        format!("  renamed {}: {}", renamed.len(), renamed.join(", ")),
        format!("  kept    {}", sequence_len(plan_get("kept"))),
        format!("  pruned  {}: {}", pruned.len(), pruned.join(", ")),
        format!("  ratios  {}", sequence_len(plan_get("ratios"))),
        format!("  moved   {}", sequence_len(plan_get("moved"))),
        format!("  focused {}", sequence_len(plan_get("focused"))),
    ];
    if truthy(get("tmux_parity")) {
        lines[0].push_str("  tmux-parity");
    }
    if let Some(live) = get("live").and_then(Value::as_object).filter(|value| !value.is_empty()) {
        lines.push(format!(
            "  live    panels={} tabs={}",
            sequence_len(live.get("make_panel")),
            sequence_len(live.get("tabs")),
        ));
    }
    if let Some(engine) = get("engine").and_then(Value::as_object) {
        if engine.get("structure_changed_tabs").is_some_and(|value| !value.is_null()) {
            let changed = engine.get("structure_changed_tabs").filter(|value| truthy(Some(value)));
            lines.push(format!(
                "  engine  structure_changed_tabs={} order_changed={}",
                changed.map_or_else(|| "[]".to_string(), |value| python_str(Some(value))),
                python_str(engine.get("order_changed")),
            ));
        }
    }
    let errors = plan_get("errors").and_then(Value::as_array).cloned().unwrap_or_default();
    if !errors.is_empty() {
        lines.push(format!("  errors  {}", errors.len()));
        for error in errors.iter().take(12) {
            lines.push(format!("    {}", python_str(Some(error))));
        }
    }
    let actions = plan_get("actions").and_then(Value::as_array).cloned().unwrap_or_default();
    if !actions.is_empty() {
        lines.push("  actions:".to_string());
        for action in actions.iter().take(40) {
            let action = action.as_object();
            let op = python_str(action.and_then(|value| value.get("op")));
            let pane_id = python_str(action.and_then(|value| value.get("pane_id")));
            let title = value_or_default_string(action.and_then(|value| value.get("title")), "");
            let reason = value_or_default_string(action.and_then(|value| value.get("reason")), "");
            lines.push(format!("    {op:<12} {pane_id}  {title}  {reason}").trim_end().to_string());
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::snapshot_from_session_payload;

    #[test]
    fn plan_views_preserve_action_order() {
        let plan = MirrorPlan {
            actions: vec![
                MirrorAction {
                    op: "create_tab".into(), pane_id: "p1".into(), title: "one".into(), tab_id: "t".into(),
                    role: "tab-root".into(), split_direction: "right".into(), key: "k1".into(), surface_id: None,
                    split_from_surface_id: None, split_from_pane_id: None, ratio: None, tab_index: None, reason: String::new(),
                },
                MirrorAction {
                    op: "create_split".into(), pane_id: "p2".into(), title: "two".into(), tab_id: "t".into(),
                    role: "split".into(), split_direction: "right".into(), key: "k2".into(), surface_id: None,
                    split_from_surface_id: None, split_from_pane_id: None, ratio: None, tab_index: None, reason: String::new(),
                },
            ],
            ..MirrorPlan::default()
        };
        assert_eq!(plan.creates().iter().map(|action| action.pane_id.as_str()).collect::<Vec<_>>(), vec!["p1", "p2"]);
    }

    #[test]
    fn desired_titles_truncate_by_unicode_scalar() {
        let title = "é".repeat(81);
        let snapshot = snapshot_from_session_payload(&json!({
            "panes": [{"pane_id": "p", "tab_id": "t", "workspace_id": "w"}],
            "tabs": [{"tab_id": "t", "workspace_id": "w", "label": title}],
        })).unwrap();
        let desired = desired_mirrors(&snapshot, "all", None, None, false).unwrap();
        assert_eq!(desired[0].title.chars().count(), 80);
        assert!(desired[0].title.chars().all(|character| character == 'é'));
    }

    #[test]
    fn prune_order_is_sorted_not_map_insertion_order() {
        let plan = plan_mirror(
            &[],
            &json!({"z": {"cmux_surface_id": "sz"}, "a": {"cmux_surface_id": "sa"}}),
            None,
            true,
            false,
            false,
            false,
            None,
        );
        assert_eq!(plan.prunes().iter().map(|action| action.pane_id.as_str()).collect::<Vec<_>>(), vec!["a", "z"]);
    }

    #[test]
    fn malformed_json_candidate_propagates_decode_error() {
        assert!(parse_cmux_json("OK\n{broken").is_err());
        assert_eq!(parse_cmux_json("OK\nnoise").unwrap(), None);
    }
}
