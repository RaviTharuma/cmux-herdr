//! Herdr topology projection and idempotent cmux mirror planning.
//!
//! Behavioral port of `bridge/cmux_herdr_mirror.py`: desired projection,
//! reconcile planning, cmux execution, state persistence, attach-pane I/O, and
//! live host coordination.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};

use crate::api::extract_read_text;
use crate::bridge::{self, CmdOutput};
use crate::control::encode_named_key;
use crate::engine::{self, HerdrWindow, WindowMirrorState};
use crate::handoff;
use crate::impose::specs_with_impose_fractions;
use crate::layout::{layouts_by_tab_id, pane_is_zoomed, pane_rects_from_dicts, parse_layout, tree_from_rects, LayoutNode, SplitSpec};
use crate::model::{Pane, Snapshot, Tab};
use crate::socket::EventSession;
use crate::state::{self, SystemEnv};

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

/// Engine reconciliation details consumed by mirror planning and apply.
#[derive(Debug, Clone)]
pub struct EngineReconcile {
    pub created_pane_ids: Vec<String>,
    pub closed_pane_ids: Vec<String>,
    pub protected_pane_ids: Vec<String>,
    pub structure_changed_tabs: Vec<String>,
    pub ordered_tab_ids: Vec<String>,
    pub order_changed: bool,
    pub states: HashMap<String, WindowMirrorState>,
    pub windows: Vec<HerdrWindow>,
}

impl EngineReconcile {
    pub fn planner_hints(&self) -> Value {
        json!({
            "created_pane_ids": self.created_pane_ids,
            "closed_pane_ids": self.closed_pane_ids,
            "protected_pane_ids": self.protected_pane_ids,
            "structure_changed_tabs": self.structure_changed_tabs,
            "ordered_tab_ids": self.ordered_tab_ids,
            "order_changed": self.order_changed,
        })
    }
}

/// Convert a desired projection into engine windows (`build_herdr_windows`).
pub fn build_herdr_windows(snapshot: &Snapshot, desired: &[DesiredMirror]) -> Vec<HerdrWindow> {
    let mut by_tab: HashMap<&str, Vec<&DesiredMirror>> = HashMap::new();
    for item in desired {
        by_tab.entry(&item.tab_id).or_default().push(item);
    }
    let panes_by_id: HashMap<&str, &Pane> = snapshot
        .panes
        .iter()
        .filter(|pane| !pane.pane_id.is_empty())
        .map(|pane| (pane.pane_id.as_str(), pane))
        .collect();
    let mut windows = Vec::new();
    for (tab_id, items) in by_tab {
        let members: Vec<&Pane> = items
            .iter()
            .filter_map(|item| panes_by_id.get(item.pane_id.as_str()).copied())
            .collect();
        let node = tab_layout_node(snapshot, tab_id, &members).or_else(|| {
            let ordered: Vec<&str> = items.iter().map(|item| item.pane_id.as_str()).collect();
            match ordered.as_slice() {
                [] => None,
                [pane_id] => parse_layout(&json!({"pane_id": pane_id})),
                _ => parse_layout(&json!({
                    "horizontal": ordered.iter().map(|pane_id| json!({"pane_id": pane_id})).collect::<Vec<_>>()
                })),
            }
        });
        let Some(node) = node else { continue };
        let root = items
            .iter()
            .find(|item| item.role == "tab-root")
            .copied()
            .unwrap_or(items[0]);
        let zoomed = items.iter().find(|item| item.zoomed).copied();
        let active = items
            .iter()
            .find(|item| item.focused)
            .map(|item| item.pane_id.clone())
            .or_else(|| zoomed.map(|item| item.pane_id.clone()))
            .unwrap_or_else(|| root.pane_id.clone());
        let visible = zoomed.and_then(|item| parse_layout(&json!({"pane_id": item.pane_id})));
        windows.push(HerdrWindow::new(
            tab_id,
            &root.title,
            root.tab_index.unwrap_or(0),
            node,
            visible,
            zoomed.is_some(),
            Some(active),
        ));
    }
    windows
}

fn window_state_from_mirrors(
    tab_id: &str,
    window: &HerdrWindow,
    existing: &Value,
) -> Option<WindowMirrorState> {
    let mut surfaces = HashMap::new();
    let mut version = 0;
    for (pane_id, entry) in existing.as_object().into_iter().flatten() {
        let Some(entry) = entry.as_object() else { continue };
        if value_or_default_string(entry.get("tab_id"), "") != tab_id {
            continue;
        }
        if let Some(surface) = entry.get("cmux_surface_id").and_then(Value::as_str).filter(|value| !value.is_empty()) {
            surfaces.insert(pane_id.clone(), surface.to_string());
        }
        if let Some(raw_version) = python_int(entry.get("layout_structure_version")) {
            version = version.max(raw_version);
        }
    }
    if surfaces.is_empty() && version == 0 {
        return None;
    }
    Some(WindowMirrorState {
        tab_id: tab_id.to_string(),
        title: window.title.clone(),
        layout: window.layout.clone(),
        visible_layout: window.visible_layout.clone(),
        zoomed: window.zoomed,
        active_pane_id: window.active_pane_id.clone(),
        pane_ids: if surfaces.is_empty() {
            window.base_pane_ids()
        } else {
            surfaces.keys().cloned().collect()
        },
        layout_structure_version: version,
        surface_id_by_pane_id: surfaces,
    })
}

/// Run the pure engine for the desired projection (`reconcile_engine_for_desired`).
pub fn reconcile_engine_for_desired(
    snapshot: &Snapshot,
    desired: &[DesiredMirror],
    existing: &Value,
) -> EngineReconcile {
    let windows = build_herdr_windows(snapshot, desired);
    let mut created = Vec::new();
    let mut closed = Vec::new();
    let mut protected = HashSet::new();
    let mut changed = HashSet::new();
    let mut states = HashMap::new();
    for window in &windows {
        protected.extend(window.base_pane_ids());
        let previous = window_state_from_mirrors(&window.tab_id, window, existing);
        let (state, result) = engine::apply_window(window, previous.as_ref());
        created.extend(result.created_pane_ids);
        closed.extend(result.closed_pane_ids);
        if result.structure_changed {
            changed.insert(window.tab_id.clone());
        }
        states.insert(window.tab_id.clone(), state);
    }
    let mut previous_tabs: Vec<String> = existing
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(_, entry)| entry.as_object())
        .filter_map(|entry| entry.get("tab_id"))
        .filter(|value| truthy(Some(value)))
        .map(|value| python_str(Some(value)))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    previous_tabs.sort();
    let session = engine::reconcile_session(&windows, &previous_tabs);
    let closed_tabs: HashSet<&str> = session.closed_tab_ids.iter().map(String::as_str).collect();
    for (pane_id, entry) in existing.as_object().into_iter().flatten() {
        let Some(entry) = entry.as_object() else { continue };
        let tab_id = value_or_default_string(entry.get("tab_id"), "");
        if closed_tabs.contains(tab_id.as_str()) {
            closed.push(pane_id.clone());
        }
    }
    closed.sort();
    closed.dedup();
    let mut protected_pane_ids: Vec<String> = protected.into_iter().collect();
    protected_pane_ids.sort();
    let mut structure_changed_tabs: Vec<String> = changed.into_iter().collect();
    structure_changed_tabs.sort();
    EngineReconcile {
        created_pane_ids: created,
        closed_pane_ids: closed,
        protected_pane_ids,
        structure_changed_tabs,
        ordered_tab_ids: session.ordered_tab_ids,
        order_changed: session.order_changed,
        states,
        windows,
    }
}

/// Injectable cmux command seam used by the executor and focused tests.
pub trait CmuxRunner {
    fn run(&mut self, args: &[String], workspace: Option<&str>) -> Result<CmdOutput, MirrorError>;
}

pub struct SystemCmuxRunner;

impl CmuxRunner for SystemCmuxRunner {
    fn run(&mut self, args: &[String], workspace: Option<&str>) -> Result<CmdOutput, MirrorError> {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        bridge::cmux_cmd(&refs, workspace).map_err(|error| MirrorError(error.to_string()))
    }
}

fn command_error(args: &[String], output: &CmdOutput) -> MirrorError {
    let detail = if !output.stderr.trim().is_empty() {
        output.stderr.trim().to_string()
    } else if !output.stdout.trim().is_empty() {
        output.stdout.trim().to_string()
    } else {
        output.returncode.to_string()
    };
    MirrorError(format!("cmux {} failed: {detail}", args.join(" ")))
}

pub fn cmux_json_with<R: CmuxRunner>(
    runner: &mut R,
    args: &[String],
    workspace: Option<&str>,
) -> Result<Value, MirrorError> {
    let mut with_json = args.to_vec();
    if !with_json.iter().any(|arg| arg == "--json") {
        with_json.push("--json".to_string());
    }
    let output = runner.run(&with_json, workspace)?;
    if output.returncode == 0 {
        return match parse_cmux_json(&output.stdout).map_err(|error| MirrorError(error.to_string()))? {
            Some(parsed) => Ok(parsed),
            None => Ok(json!({"ok": true, "stdout": output.stdout.trim()})),
        };
    }
    let output = runner.run(args, workspace)?;
    if output.returncode != 0 {
        return Err(command_error(args, &output));
    }
    match parse_cmux_json(&output.stdout).map_err(|error| MirrorError(error.to_string()))? {
        Some(parsed) => Ok(parsed),
        None => Ok(json!({"ok": true, "stdout": output.stdout.trim()})),
    }
}

pub fn cmux_json(args: &[String], workspace: Option<&str>) -> Result<Value, MirrorError> {
    cmux_json_with(&mut SystemCmuxRunner, args, workspace)
}

fn extract_cmux_id(payload: &Value, keys: &[&str]) -> Option<String> {
    if let Some(text) = payload.as_str().map(str::trim).filter(|text| !text.is_empty()) {
        return Some(text.to_string());
    }
    let object = payload.as_object()?;
    for key in keys {
        if let Some(text) = object.get(*key).and_then(Value::as_str).map(str::trim).filter(|text| !text.is_empty()) {
            return Some(text.to_string());
        }
    }
    for key in ["result", "payload", "surface", "pane", "terminal"] {
        if let Some(found) = object.get(key).and_then(|value| extract_cmux_id(value, keys)) {
            return Some(found);
        }
    }
    None
}

fn create_terminal_with<R: CmuxRunner>(
    runner: &mut R,
    key: &str,
    name: &str,
    command: &str,
    workspace: Option<&str>,
    pane: Option<&str>,
) -> Result<Value, MirrorError> {
    let mut attempts = Vec::new();
    if let Some(pane) = pane {
        attempts.push(vec!["create-terminal", "--key", key, "--name", name, "--command", command, "--pane", pane]);
    }
    attempts.extend([
        vec!["create-terminal", "--key", key, "--name", name, "--command", command],
        vec!["run", "--key", key, "--name", name, "--command", command],
        vec!["run", "--name", name, "--command", command],
    ]);
    let mut errors = Vec::new();
    for attempt in attempts {
        let args: Vec<String> = attempt.into_iter().map(str::to_string).collect();
        match cmux_json_with(runner, &args, workspace) {
            Ok(payload) => return Ok(json!({
                "cmux_surface_id": extract_cmux_id(&payload, &["surface_id", "surface_ref", "id", "pane_id", "terminal_id"]),
                "cmux_pane_id": extract_cmux_id(&payload, &["pane_id", "pane_ref", "pane"]),
                "payload": payload,
                "args": args,
            })),
            Err(error) => errors.push(error.to_string()),
        }
    }
    let tail = errors.iter().rev().take(3).cloned().collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>();
    Err(MirrorError(format!("could not create cmux terminal for mirror key {key}: {}", tail.join(" | "))))
}

pub fn create_terminal(
    key: &str,
    name: &str,
    command: &str,
    workspace: Option<&str>,
    pane: Option<&str>,
) -> Result<Value, MirrorError> {
    create_terminal_with(&mut SystemCmuxRunner, key, name, command, workspace, pane)
}

fn split_pane_with<R: CmuxRunner>(
    runner: &mut R,
    from_surface: &str,
    direction: &str,
    workspace: Option<&str>,
) -> Result<Value, MirrorError> {
    let dir = if direction == "right" { "right" } else { "down" };
    let attempts = if direction == "right" {
        vec![
            vec!["split", "--pane", from_surface, "--dir", dir],
            vec!["split", from_surface, dir],
            vec!["new-pane-right", "--pane", from_surface],
        ]
    } else {
        vec![
            vec!["split", "--pane", from_surface, "--dir", dir],
            vec!["split", from_surface, dir],
            vec!["new-pane", "--pane", from_surface],
        ]
    };
    let mut errors = Vec::new();
    for attempt in attempts {
        let args: Vec<String> = attempt.into_iter().map(str::to_string).collect();
        match cmux_json_with(runner, &args, workspace) {
            Ok(payload) => return Ok(json!({
                "cmux_surface_id": extract_cmux_id(&payload, &["surface_id", "surface_ref", "id"]),
                "cmux_pane_id": extract_cmux_id(&payload, &["pane_id", "pane_ref", "id"]),
                "payload": payload,
                "args": args,
            })),
            Err(error) => errors.push(error.to_string()),
        }
    }
    let tail = errors.iter().rev().take(3).cloned().collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>();
    Err(MirrorError(format!("could not split cmux surface {from_surface}: {}", tail.join(" | "))))
}

pub fn split_pane(from_surface: &str, direction: &str, workspace: Option<&str>) -> Result<Value, MirrorError> {
    split_pane_with(&mut SystemCmuxRunner, from_surface, direction, workspace)
}

fn attempt_cmux_commands<R: CmuxRunner>(
    runner: &mut R,
    attempts: Vec<Vec<String>>,
    workspace: Option<&str>,
) -> Result<(), MirrorError> {
    let mut last = None;
    for args in attempts {
        match cmux_json_with(runner, &args, workspace) {
            Ok(_) => return Ok(()),
            Err(error) => last = Some(error),
        }
    }
    Err(last.unwrap_or_else(|| MirrorError("no cmux command attempts".to_string())))
}

fn rename_surface_with<R: CmuxRunner>(runner: &mut R, surface: &str, title: &str, workspace: Option<&str>) -> Result<(), MirrorError> {
    attempt_cmux_commands(runner, vec![
        vec!["rename-surface".into(), surface.into(), title.into()],
        vec!["rename-surface".into(), "--surface".into(), surface.into(), "--name".into(), title.into()],
    ], workspace)
}

pub fn rename_surface(surface: &str, title: &str, workspace: Option<&str>) -> Result<(), MirrorError> {
    rename_surface_with(&mut SystemCmuxRunner, surface, title, workspace)
}

fn close_surface_with<R: CmuxRunner>(runner: &mut R, surface: &str, workspace: Option<&str>) -> Result<(), MirrorError> {
    attempt_cmux_commands(runner, vec![
        vec!["close-surface".into(), surface.into()],
        vec!["close-surface".into(), "--surface".into(), surface.into()],
        vec!["close-terminal".into(), surface.into()],
    ], workspace)
}

pub fn close_surface(surface: &str, workspace: Option<&str>) -> Result<(), MirrorError> {
    close_surface_with(&mut SystemCmuxRunner, surface, workspace)
}

fn set_split_ratio_with<R: CmuxRunner>(runner: &mut R, surface: &str, ratio: f64, workspace: Option<&str>) -> Result<(), MirrorError> {
    let ratio = format!("{:.4}", ratio.clamp(0.05, 0.95));
    attempt_cmux_commands(runner, vec![
        vec!["set-ratio".into(), "--pane".into(), surface.into(), "--ratio".into(), ratio.clone()],
        vec!["set-split-ratio".into(), "--pane".into(), surface.into(), "--ratio".into(), ratio.clone()],
        vec!["set-ratio".into(), surface.into(), ratio.clone()],
        vec!["apply-layout".into(), "--pane".into(), surface.into(), "--ratio".into(), ratio],
    ], workspace)
}

pub fn set_split_ratio(surface: &str, ratio: f64, workspace: Option<&str>) -> Result<(), MirrorError> {
    set_split_ratio_with(&mut SystemCmuxRunner, surface, ratio, workspace)
}

fn move_tab_with<R: CmuxRunner>(runner: &mut R, surface: &str, index: i64, workspace: Option<&str>) -> Result<(), MirrorError> {
    let index = index.max(0).to_string();
    attempt_cmux_commands(runner, vec![
        vec!["move-tab".into(), "--surface".into(), surface.into(), "--index".into(), index.clone()],
        vec!["move-tab".into(), surface.into(), index.clone()],
        vec!["move-tab".into(), "--pane".into(), surface.into(), "--to".into(), index],
    ], workspace)
}

pub fn move_tab(surface: &str, index: i64, workspace: Option<&str>) -> Result<(), MirrorError> {
    move_tab_with(&mut SystemCmuxRunner, surface, index, workspace)
}

fn focus_surface_with<R: CmuxRunner>(runner: &mut R, surface: &str, workspace: Option<&str>) -> Result<(), MirrorError> {
    attempt_cmux_commands(runner, vec![
        vec!["focus-surface".into(), surface.into()],
        vec!["select-pane".into(), surface.into()],
        vec!["focus".into(), "--surface".into(), surface.into()],
        vec!["focus-pane".into(), surface.into()],
    ], workspace)
}

pub fn focus_surface(surface: &str, workspace: Option<&str>) -> Result<(), MirrorError> {
    focus_surface_with(&mut SystemCmuxRunner, surface, workspace)
}

fn collect_ids(node: &Value, found: &mut HashSet<String>) {
    match node {
        Value::Object(object) => {
            for (key, value) in object {
                if matches!(key.as_str(), "surface_id" | "surface_ref" | "id" | "terminal_id" | "pane_id") {
                    if let Some(text) = value.as_str().filter(|text| !text.is_empty()) {
                        found.insert(text.to_string());
                    }
                } else {
                    collect_ids(value, found);
                }
            }
        }
        Value::Array(values) => values.iter().for_each(|value| collect_ids(value, found)),
        _ => {}
    }
}

pub fn list_live_surface_ids_with<R: CmuxRunner>(runner: &mut R, workspace: Option<&str>) -> Option<HashSet<String>> {
    for args in [vec!["tree".to_string()], vec!["list-terminals".to_string()], vec!["ids".to_string(), "--kind".to_string(), "surface".to_string()]] {
        if let Ok(payload) = cmux_json_with(runner, &args, workspace) {
            let mut found = HashSet::new();
            collect_ids(&payload, &mut found);
            if !found.is_empty() {
                return Some(found);
            }
        }
    }
    None
}

pub fn list_live_surface_ids(workspace: Option<&str>) -> Option<HashSet<String>> {
    list_live_surface_ids_with(&mut SystemCmuxRunner, workspace)
}

pub fn load_mirrors() -> Value {
    let fingerprint = state::collect_host_fingerprint(&SystemEnv);
    state::load_association_map(&SystemEnv, &fingerprint)
        .get("mirrors")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}))
}

pub fn save_mirrors(mirrors: &Value, workspace: Option<&str>) -> Result<(), MirrorError> {
    let fingerprint = state::collect_host_fingerprint(&SystemEnv);
    let mut association = state::load_association_map(&SystemEnv, &fingerprint);
    association["mirrors"] = mirrors.as_object().cloned().map(Value::Object).unwrap_or_else(|| json!({}));
    if let Some(workspace) = workspace.filter(|value| !value.is_empty()) {
        association["cmux_workspace"] = json!(workspace);
    }
    state::save_association_map(&SystemEnv, &association, &fingerprint).map_err(|error| MirrorError(error.to_string()))
}

pub fn size_authority_path() -> PathBuf {
    let fingerprint = state::collect_host_fingerprint(&SystemEnv);
    state::state_dir(&SystemEnv).join(format!("size-authority-{}", state::parent_key(&fingerprint)))
}

pub fn write_size_authority(pane_id: Option<&str>) -> Result<(), MirrorError> {
    let path = size_authority_path();
    if pane_id.is_none_or(|value| value.is_empty()) {
        return match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(MirrorError(error.to_string())),
        };
    }
    let directory = state::state_dir(&SystemEnv);
    fs::create_dir_all(&directory).map_err(|error| MirrorError(error.to_string()))?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, format!("{}\n", pane_id.unwrap().trim())).map_err(|error| MirrorError(error.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600)).map_err(|error| MirrorError(error.to_string()))?;
    }
    fs::rename(temporary, path).map_err(|error| MirrorError(error.to_string()))
}

pub fn read_size_authority() -> Option<String> {
    fs::read_to_string(size_authority_path()).ok().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

pub fn may_claim_client_size(pane_id: &str) -> bool {
    let fingerprint = state::collect_host_fingerprint(&SystemEnv);
    let decision = handoff::resolve_writer(&state::parent_key(&fingerprint), None, None);
    if decision.native_live && !handoff::env_truthy(handoff::FORCE_PLUGIN_ENV) {
        return false;
    }
    let environment = env::var(SIZE_AUTHORITY_ENV).unwrap_or_default();
    if !environment.trim().is_empty() {
        return environment.trim() == pane_id;
    }
    if let Some(authority) = read_size_authority() {
        if authority == "native" || authority.starts_with("native:") {
            return false;
        }
        return authority == pane_id;
    }
    true
}

fn attach_argv(pane_id: &str) -> Vec<String> {
    let executable = bridge::which("cmux-herdr")
        .or_else(|| std::env::current_exe().ok().map(|path| path.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "cmux-herdr".to_string());
    vec![executable, "attach-pane".to_string(), pane_id.to_string()]
}

fn action_json(action: &MirrorAction) -> Value {
    json!({
        "op": action.op,
        "pane_id": action.pane_id,
        "title": action.title,
        "tab_id": action.tab_id,
        "role": action.role,
        "reason": action.reason,
        "split_direction": action.split_direction,
        "ratio": action.ratio,
        "tab_index": action.tab_index,
    })
}

fn now_seconds() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|duration| duration.as_secs_f64()).unwrap_or(0.0)
}

#[allow(clippy::too_many_arguments)]
pub fn apply_mirror_plan_with<R: CmuxRunner>(
    plan: &MirrorPlan,
    existing: &Value,
    workspace: Option<&str>,
    dry_run: bool,
    log: bool,
    mut engine_states: Option<&mut HashMap<String, WindowMirrorState>>,
    runner: &mut R,
    persist: bool,
) -> Result<Value, MirrorError> {
    let mut mirrors = Map::new();
    for (pane_id, entry) in existing.as_object().into_iter().flatten() {
        if let Some(entry) = entry.as_object() {
            mirrors.insert(pane_id.clone(), Value::Object(entry.clone()));
        }
    }
    let mut created = Vec::new();
    let mut renamed = Vec::new();
    let mut pruned = Vec::new();
    let mut kept = Vec::new();
    let mut ratios = Vec::new();
    let mut moved = Vec::new();
    let mut focused = Vec::new();
    let mut errors = Vec::new();
    let mut tab_root_surface = HashMap::new();
    for entry in mirrors.values().filter_map(Value::as_object) {
        if entry.get("role").and_then(Value::as_str) == Some("tab-root") {
            if let Some(surface) = entry.get("cmux_surface_id").and_then(Value::as_str).filter(|value| !value.is_empty()) {
                tab_root_surface.insert(value_or_default_string(entry.get("tab_id"), ""), surface.to_string());
            }
        }
    }

    for action in &plan.actions {
        if action.op == "keep" {
            kept.push(action.pane_id.clone());
            if action.role == "tab-root" {
                if let Some(surface) = &action.surface_id {
                    tab_root_surface.insert(action.tab_id.clone(), surface.clone());
                }
            }
            continue;
        }
        if dry_run {
            continue;
        }
        let outcome: Result<(), MirrorError> = (|| {
            match action.op.as_str() {
                "rename" => {
                    if let Some(surface) = action.surface_id.as_deref() {
                        rename_surface_with(runner, surface, &action.title, workspace)?;
                        mirrors.entry(action.pane_id.clone()).or_insert_with(|| json!({}))["title"] = json!(action.title);
                        renamed.push(action.pane_id.clone());
                    }
                }
                "prune" => {
                    if let Some(surface) = action.surface_id.as_deref() {
                        close_surface_with(runner, surface, workspace)?;
                        mirrors.remove(&action.pane_id);
                        pruned.push(action.pane_id.clone());
                    }
                }
                "set_ratio" => {
                    if let (Some(surface), Some(ratio)) = (action.surface_id.as_deref(), action.ratio) {
                        set_split_ratio_with(runner, surface, ratio, workspace)?;
                        mirrors.entry(action.pane_id.clone()).or_insert_with(|| json!({}))["split_ratio"] = json!(ratio);
                        ratios.push(action.pane_id.clone());
                    }
                }
                "move_tab" => {
                    if let Some(index) = action.tab_index {
                        let surface = action.surface_id.clone().or_else(|| mirrors.get(&action.pane_id).and_then(|entry| entry.get("cmux_surface_id")).and_then(Value::as_str).map(str::to_string));
                        if let Some(surface) = surface {
                            move_tab_with(runner, &surface, index, workspace)?;
                            mirrors.entry(action.pane_id.clone()).or_insert_with(|| json!({}))["tab_index"] = json!(index);
                            moved.push(action.pane_id.clone());
                        }
                    }
                }
                "focus" => {
                    let surface = action.surface_id.clone().or_else(|| mirrors.get(&action.pane_id).and_then(|entry| entry.get("cmux_surface_id")).and_then(Value::as_str).map(str::to_string));
                    if let Some(surface) = surface {
                        focus_surface_with(runner, &surface, workspace)?;
                    }
                    for (pane_id, entry) in &mut mirrors {
                        if let Some(entry) = entry.as_object_mut() {
                            entry.insert("focused".into(), json!(pane_id == &action.pane_id));
                        }
                    }
                    focused.push(action.pane_id.clone());
                }
                "create_tab" | "create_split" => {
                    let command = attach_argv(&action.pane_id).join(" ");
                    let mut created_info = if action.op == "create_split" {
                        let split_from = action.split_from_pane_id.as_ref()
                            .and_then(|pane_id| mirrors.get(pane_id))
                            .and_then(|entry| entry.get("cmux_surface_id"))
                            .and_then(Value::as_str)
                            .map(str::to_string)
                            .or_else(|| action.split_from_surface_id.clone())
                            .or_else(|| tab_root_surface.get(&action.tab_id).cloned());
                        let Some(split_from) = split_from else {
                            return Err(MirrorError(format!("no split-from surface (refusing orphan-tab fallback)")));
                        };
                        let split = split_pane_with(runner, &split_from, &action.split_direction, workspace)
                            .map_err(|error| MirrorError(format!("{error} (refusing orphan-tab fallback)")))?;
                        let pane = split.get("cmux_pane_id").or_else(|| split.get("cmux_surface_id")).and_then(Value::as_str);
                        let mut created = create_terminal_with(runner, &action.key, &action.title, &command, workspace, pane)?;
                        if created.get("cmux_surface_id").is_none_or(Value::is_null) {
                            created["cmux_surface_id"] = split.get("cmux_surface_id").cloned().unwrap_or(Value::Null);
                        }
                        if created.get("cmux_pane_id").is_none_or(Value::is_null) {
                            created["cmux_pane_id"] = split.get("cmux_pane_id").cloned().unwrap_or(Value::Null);
                        }
                        created
                    } else {
                        create_terminal_with(runner, &action.key, &action.title, &command, workspace, None)?
                    };
                    let surface = created_info.get_mut("cmux_surface_id").and_then(|value| value.as_str()).map(str::to_string);
                    if action.role == "tab-root" {
                        if let Some(surface) = surface.as_deref() {
                            tab_root_surface.insert(action.tab_id.clone(), surface.to_string());
                            let _ = rename_surface_with(runner, surface, &action.title, workspace);
                        }
                    }
                    mirrors.insert(action.pane_id.clone(), json!({
                        "pane_id": action.pane_id,
                        "tab_id": action.tab_id,
                        "role": action.role,
                        "title": action.title,
                        "key": action.key,
                        "cmux_surface_id": surface,
                        "cmux_pane_id": created_info.get("cmux_pane_id").cloned().unwrap_or(Value::Null),
                        "split_direction": action.split_direction,
                        "split_ratio": action.ratio,
                        "split_from_pane_id": action.split_from_pane_id,
                        "tab_index": action.tab_index,
                        "focused": false,
                        "updated_at": now_seconds(),
                    }));
                    if let Some(states) = engine_states.as_deref_mut() {
                        if let Some(state) = states.get_mut(&action.tab_id) {
                            mirrors.get_mut(&action.pane_id).unwrap()["layout_structure_version"] = json!(state.layout_structure_version);
                            if let Some(surface) = surface.as_deref() {
                                state.surface_id_by_pane_id.insert(action.pane_id.clone(), surface.to_string());
                            }
                        }
                    }
                    if action.op == "create_split" {
                        if let (Some(surface), Some(ratio)) = (surface.as_deref(), action.ratio) {
                            match set_split_ratio_with(runner, surface, ratio, workspace) {
                                Ok(()) => ratios.push(action.pane_id.clone()),
                                Err(error) => errors.push(format!("set_ratio {}: {error}", action.pane_id)),
                            }
                        }
                    }
                    created.push(action.pane_id.clone());
                }
                _ => {}
            }
            Ok(())
        })();
        if let Err(error) = outcome {
            errors.push(format!("{} {}: {error}", action.op, action.pane_id));
        }
    }

    if !dry_run {
        if let Some(states) = engine_states.as_deref_mut() {
            for (pane_id, entry) in &mut mirrors {
                let Some(entry) = entry.as_object_mut() else { continue };
                let tab_id = value_or_default_string(entry.get("tab_id"), "");
                let Some(state) = states.get_mut(&tab_id) else { continue };
                entry.insert("layout_structure_version".into(), json!(state.layout_structure_version));
                if let Some(surface) = entry.get("cmux_surface_id").and_then(Value::as_str).filter(|value| !value.is_empty()) {
                    state.surface_id_by_pane_id.insert(pane_id.clone(), surface.to_string());
                }
            }
        }
        if persist {
            save_mirrors(&Value::Object(mirrors.clone()), workspace)?;
        }
        if log {
            let summary = format!(
                "herdr mirror: created={} renamed={} kept={} pruned={} ratios={} moved={} focused={} errors={}",
                created.len(), renamed.len(), kept.len(), pruned.len(), ratios.len(), moved.len(), focused.len(), errors.len()
            );
            let _ = runner.run(&["log".to_string(), summary], workspace);
        }
    }
    Ok(json!({
        "created": created,
        "renamed": renamed,
        "kept": kept,
        "pruned": pruned,
        "ratios": ratios,
        "moved": moved,
        "focused": focused,
        "errors": errors,
        "dry_run": dry_run,
        "mirrors": mirrors,
        "actions": plan.actions.iter().map(action_json).collect::<Vec<_>>(),
    }))
}

pub fn apply_mirror_plan(
    plan: &MirrorPlan,
    existing: &Value,
    workspace: Option<&str>,
    dry_run: bool,
    log: bool,
    engine_states: Option<&mut HashMap<String, WindowMirrorState>>,
) -> Result<Value, MirrorError> {
    apply_mirror_plan_with(plan, existing, workspace, dry_run, log, engine_states, &mut SystemCmuxRunner, true)
}

fn fingerprint_json() -> Value {
    let fingerprint = state::collect_host_fingerprint(&SystemEnv);
    json!({
        "cmux_surface_id": fingerprint.cmux_surface_id,
        "herdr_socket_path": fingerprint.herdr_socket_path,
        "herdr_server_pid": fingerprint.herdr_server_pid,
        "herdr_workspace_id": fingerprint.herdr_workspace_id,
    })
}

pub fn resolve_cmux_workspace(explicit: Option<&str>) -> Option<String> {
    if let Some(workspace) = explicit.filter(|value| !value.is_empty()) {
        return Some(workspace.to_string());
    }
    if let Some(workspace) = env::var("CMUX_WORKSPACE_ID").ok().filter(|value| !value.is_empty()) {
        return Some(workspace);
    }
    let fingerprint = state::collect_host_fingerprint(&SystemEnv);
    let association = state::load_association_map(&SystemEnv, &fingerprint);
    if let Some(workspace) = association.get("cmux_workspace").and_then(Value::as_str).filter(|value| !value.is_empty()) {
        return Some(workspace.to_string());
    }
    for args in [vec!["identify".to_string()], vec!["focused".to_string()]] {
        if let Ok(payload) = cmux_json(&args, None) {
            if let Some(workspace) = extract_cmux_id(&payload, &["workspace_id", "workspace_ref", "workspace"]) {
                return Some(workspace);
            }
        }
    }
    None
}

fn focused_surface_with<R: CmuxRunner>(runner: &mut R, workspace: Option<&str>) -> Option<String> {
    for args in [
        vec!["identify".to_string(), "--json".to_string()],
        vec!["focused".to_string(), "--json".to_string()],
        vec!["identify".to_string()],
    ] {
        if let Ok(payload) = cmux_json_with(runner, &args, workspace) {
            if let Some(surface) = extract_cmux_id(&payload, &["surface_id", "surface_ref", "focused_surface_id", "id"]) {
                return Some(surface);
            }
        }
    }
    None
}

pub fn sync_focus_from_cmux(mirrors: &mut Value, workspace: Option<&str>) -> Result<(), MirrorError> {
    let Some(surface) = focused_surface_with(&mut SystemCmuxRunner, workspace) else { return Ok(()) };
    let Some(entries) = mirrors.as_object_mut() else { return Ok(()) };
    let selected = entries.iter().find_map(|(pane_id, entry)| {
        (entry.get("cmux_surface_id").and_then(Value::as_str) == Some(surface.as_str()))
            .then(|| (pane_id.clone(), truthy(entry.get("focused"))))
    });
    let Some((pane_id, already_focused)) = selected else { return Ok(()) };
    if already_focused {
        return Ok(());
    }
    let mut api = bridge::herdr_api();
    bridge::herdr_rpc(&mut api, "agent.focus", json!({"target": pane_id, "pane_id": pane_id}))
        .map_err(|error| MirrorError(error.to_string()))?;
    for (other_id, entry) in entries.iter_mut() {
        if let Some(entry) = entry.as_object_mut() {
            entry.insert("focused".into(), json!(other_id == &pane_id));
        }
    }
    save_mirrors(mirrors, workspace)
}

fn status_keys_with<R: CmuxRunner>(runner: &mut R, workspace: &str) -> Vec<String> {
    let Ok(output) = runner.run(&["list-status".to_string()], Some(workspace)) else { return Vec::new() };
    if output.returncode != 0 {
        return Vec::new();
    }
    output.stdout.lines().filter_map(|line| {
        line.split_once('=').map(|(key, _)| key.trim()).filter(|key| key.starts_with("herdr:")).map(str::to_string)
    }).collect()
}

fn sync_status_to_cmux(snapshot: &Snapshot, workspace: &str, log: bool) -> Value {
    let tabs: HashMap<String, Tab> = snapshot.tabs.iter().map(|tab| (tab.tab_id.clone(), tab.clone())).collect();
    let fingerprint = state::collect_host_fingerprint(&SystemEnv);
    let prior_state = state::load_association_map(&SystemEnv, &fingerprint);
    let previous = prior_state.get("panes").unwrap_or(&Value::Null);
    let mut panes: Vec<&Pane> = snapshot.panes.iter().filter(|pane| pane.agent.as_ref().is_some_and(|agent| !agent.is_empty())).collect();
    if panes.is_empty() {
        panes = snapshot.panes.iter().filter(|pane| matches!(pane.agent_status.as_str(), "working" | "idle" | "done" | "blocked")).collect();
    }
    let mut applied = Vec::new();
    let mut errors = Vec::new();
    let mut desired = HashSet::new();
    let mut write_meta = Map::new();
    let mut runner = SystemCmuxRunner;
    for pane in &panes {
        let key = pane.status_key();
        desired.insert(key.clone());
        let prior = state::prior_for_pane(pane, previous);
        let payload = crate::status::status_write_payload(pane, Some(&tabs), Some(prior));
        let args = vec![
            "set-status".to_string(), key.clone(), payload["value"].as_str().unwrap_or("").to_string(),
            "--icon".to_string(), payload["icon"].as_str().unwrap_or("").to_string(),
            "--color".to_string(), payload["color"].as_str().unwrap_or("").to_string(),
            "--priority".to_string(), payload["priority"].as_i64().unwrap_or(0).to_string(),
        ];
        match runner.run(&args, Some(workspace)) {
            Ok(output) if output.returncode == 0 => {
                applied.push(key);
                write_meta.insert(pane.pane_id.clone(), payload);
            }
            Ok(output) => errors.push(command_error(&args, &output).to_string()),
            Err(error) => errors.push(error.to_string()),
        }
    }
    let mut stale = Vec::new();
    for key in status_keys_with(&mut runner, workspace) {
        if desired.contains(&key) {
            continue;
        }
        if runner.run(&["clear-status".to_string(), key.clone()], Some(workspace)).is_ok_and(|output| output.returncode == 0) {
            stale.push(key);
        }
    }
    let associations = state::update_association_map(
        &SystemEnv,
        snapshot,
        Some(workspace),
        Some(&Value::Object(write_meta)),
    ).unwrap_or(Value::Null);
    let summary = format!("herdr sync: {} panes → cmux ws={workspace}", applied.len());
    if log {
        let _ = runner.run(&["log".to_string(), summary.clone()], Some(workspace));
    }
    json!({
        "workspace": workspace,
        "applied": applied,
        "skipped_unchanged": [],
        "stale_cleared": stale,
        "errors": errors,
        "summary": summary,
        "pane_count": snapshot.panes.len(),
        "agent_count": panes.len(),
        "associations": associations,
        "host_fingerprint_key": state::parent_key(&fingerprint),
        "writer": "plugin",
        "native_live": false,
    })
}

fn live_report(windows: &[HerdrWindow]) -> Result<Value, MirrorError> {
    let host = crate::live::apply_live_windows(windows, None, true).map_err(MirrorError)?;
    let tabs: Vec<String> = host.windows.keys().cloned().collect();
    let make_panel: Vec<String> = host.windows.values().flat_map(|window| window.surfaces.keys().cloned()).collect();
    Ok(json!({
        "tabs": tabs,
        "make_panel": make_panel,
        "pane_surfaces": host.pane_surfaces(),
        "pane_grids": host.pane_grids(),
        "defaults_open": host.defaults_open,
        "ops": host.log,
    }))
}

#[allow(clippy::too_many_arguments)]
pub fn mirror_to_cmux_with_snapshot(
    snapshot: &Snapshot,
    scope: &str,
    workspace: Option<&str>,
    herdr_workspace: Option<&str>,
    tab: Option<&str>,
    prune: bool,
    sync_status: bool,
    use_layout: bool,
    sync_focus: bool,
    sync_order: bool,
    sync_ratios: bool,
    tmux_parity: bool,
    dry_run: bool,
    log: bool,
) -> Result<Value, MirrorError> {
    if is_attach_process() {
        return Err(MirrorError(format!(
            "refusing to nest mirror inside attach-pane ({ATTACH_ENV}={})",
            env::var(ATTACH_ENV).unwrap_or_default()
        )));
    }
    let fingerprint = state::collect_host_fingerprint(&SystemEnv);
    let writer = handoff::writer_status(&state::parent_key(&fingerprint));
    if writer["native_live"].as_bool() == Some(true) {
        let mirrors = load_mirrors();
        return Ok(json!({
            "scope": scope,
            "workspace": workspace,
            "desired_count": 0,
            "tmux_parity": tmux_parity,
            "sync_focus": sync_focus,
            "sync_order": sync_order,
            "sync_ratios": sync_ratios,
            "plan": {
                "created": [], "renamed": [], "kept": [], "pruned": [], "ratios": [], "moved": [], "focused": [],
                "errors": [], "dry_run": dry_run, "mirrors": mirrors, "actions": []
            },
            "status_sync": Value::Null,
            "host_fingerprint": fingerprint_json(),
            "writer": writer["writer"],
            "native_live": true,
            "skipped_reason": "native_live",
        }));
    }
    let (scope, prune, use_layout, sync_focus, sync_order, sync_ratios) = if tmux_parity {
        ("all", true, true, true, true, true)
    } else {
        (scope, prune, use_layout, sync_focus, sync_order, sync_ratios)
    };
    let desired = desired_mirrors(snapshot, scope, tab, herdr_workspace, use_layout)?;
    let resolved_workspace = if dry_run { workspace.map(str::to_string) } else { resolve_cmux_workspace(workspace) };
    if !dry_run && resolved_workspace.is_none() {
        return Err(MirrorError("could not resolve cmux workspace for mirror".to_string()));
    }
    let existing = load_mirrors();
    let mut engine = reconcile_engine_for_desired(snapshot, &desired, &existing);
    let hints = engine.planner_hints();
    let live_ids = if dry_run { None } else { list_live_surface_ids(resolved_workspace.as_deref()) };
    let mut plan = plan_mirror(
        &desired, &existing, live_ids.as_ref(), prune, sync_focus, sync_order, sync_ratios, Some(&hints),
    );
    plan.scope = scope.to_string();
    let mut applied = apply_mirror_plan(
        &plan, &existing, resolved_workspace.as_deref(), dry_run, log, Some(&mut engine.states),
    )?;
    if sync_focus && !dry_run {
        let _ = sync_focus_from_cmux(&mut applied["mirrors"], resolved_workspace.as_deref()).map_err(|error| {
            if let Some(errors) = applied["errors"].as_array_mut() {
                errors.push(json!(format!("focus reverse: {error}")));
            }
            error
        });
        let focused = applied["mirrors"].as_object().and_then(|mirrors| mirrors.iter().find_map(|(pane_id, entry)| truthy(entry.get("focused")).then(|| pane_id.clone())))
            .or_else(|| desired.iter().find(|item| item.focused).or_else(|| desired.first()).map(|item| item.pane_id.clone()));
        write_size_authority(focused.as_deref())?;
    }
    let live = if tmux_parity { live_report(&engine.windows)? } else { Value::Null };
    let status_summary = if sync_status && !dry_run {
        resolved_workspace.as_deref().map(|workspace| sync_status_to_cmux(snapshot, workspace, log)).unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    Ok(json!({
        "scope": scope,
        "workspace": resolved_workspace,
        "desired_count": desired.len(),
        "tmux_parity": tmux_parity,
        "sync_focus": sync_focus,
        "sync_order": sync_order,
        "sync_ratios": sync_ratios,
        "plan": applied,
        "status_sync": status_summary,
        "host_fingerprint": fingerprint_json(),
        "writer": writer["writer"],
        "native_live": false,
        "live": live,
        "engine": {
            "created_pane_ids": engine.created_pane_ids,
            "closed_pane_ids": engine.closed_pane_ids,
            "structure_changed_tabs": engine.structure_changed_tabs,
            "order_changed": engine.order_changed,
        },
    }))
}

#[allow(clippy::too_many_arguments)]
pub fn mirror_to_cmux(
    scope: &str,
    workspace: Option<&str>,
    herdr_workspace: Option<&str>,
    tab: Option<&str>,
    prune: bool,
    sync_status: bool,
    use_layout: bool,
    sync_focus: bool,
    sync_order: bool,
    sync_ratios: bool,
    tmux_parity: bool,
    dry_run: bool,
    log: bool,
) -> Result<Value, MirrorError> {
    let mut api = bridge::herdr_api();
    let snapshot = bridge::fetch_snapshot(&mut api).map_err(|error| MirrorError(error.to_string()))?;
    mirror_to_cmux_with_snapshot(
        &snapshot, scope, workspace, herdr_workspace, tab, prune, sync_status, use_layout,
        sync_focus, sync_order, sync_ratios, tmux_parity, dry_run, log,
    )
}

pub fn send_pane_text(pane_id: &str, text: &str) -> Result<(), MirrorError> {
    if text.is_empty() {
        return Ok(());
    }
    let mut api = bridge::herdr_api();
    if bridge::herdr_rpc(&mut api, "pane.send_text", json!({"pane_id": pane_id, "text": text})).is_ok() {
        return Ok(());
    }
    if bridge::which("herdr").is_none() {
        return Err(MirrorError("herdr not found on PATH".to_string()));
    }
    let output = bridge::run_cmd(&["herdr", "pane", "send-text", pane_id, text], Duration::from_secs(5), None)
        .map_err(|error| MirrorError(error.to_string()))?;
    if output.returncode == 0 {
        return Ok(());
    }
    let detail = if !output.stderr.trim().is_empty() {
        output.stderr.trim().to_string()
    } else if !output.stdout.trim().is_empty() {
        output.stdout.trim().to_string()
    } else {
        output.returncode.to_string()
    };
    Err(MirrorError(if detail.is_empty() { format!("herdr pane send-text failed for {pane_id}") } else { detail }))
}

pub fn send_pane_named_key(pane_id: &str, name: &str) -> Result<Value, MirrorError> {
    let item = encode_named_key(pane_id, name).ok_or_else(|| MirrorError(format!("unknown key name: {name}")))?;
    if let Some(key) = item.key.as_deref() {
        let mut api = bridge::herdr_api();
        if bridge::herdr_rpc(&mut api, "pane.send_keys", json!({"pane_id": pane_id, "keys": key, "key": key})).is_ok() {
            return Ok(json!({"pane_id": pane_id, "key": key, "via": "send_keys"}));
        }
    }
    if let Some(csi) = item.csi {
        let text: String = csi.into_iter().map(char::from).collect();
        send_pane_text(pane_id, &text)?;
        return Ok(json!({"pane_id": pane_id, "key": item.key, "via": "csi"}));
    }
    Err(MirrorError(format!("could not send key {name} to {pane_id}")))
}

pub fn read_pane_text(pane_id: &str, lines: i64, ansi: bool) -> Result<String, MirrorError> {
    let mut params = json!({"pane_id": pane_id, "source": "recent", "lines": lines});
    if ansi {
        params["ansi"] = json!(true);
    }
    let mut api = bridge::herdr_api();
    if let Ok(payload) = bridge::herdr_rpc(&mut api, "pane.read", params.clone()) {
        let text = extract_read_text(&payload);
        if !text.is_empty() {
            return Ok(text);
        }
    }
    if ansi {
        params.as_object_mut().unwrap().remove("ansi");
        params["source"] = json!("recent-unwrapped");
        let mut api = bridge::herdr_api();
        if let Ok(payload) = bridge::herdr_rpc(&mut api, "pane.read", params) {
            let text = extract_read_text(&payload);
            if !text.is_empty() {
                return Ok(text);
            }
        }
    }
    let lines_text = lines.to_string();
    let mut attempts = Vec::new();
    if ansi {
        attempts.push(vec!["pane", "read", pane_id, "--source", "recent-unwrapped", "--lines", &lines_text, "--ansi"]);
        attempts.push(vec!["pane", "read", pane_id, "--source", "recent", "--lines", &lines_text, "--raw"]);
    }
    attempts.push(vec!["pane", "read", pane_id, "--source", "recent-unwrapped", "--lines", &lines_text]);
    let mut last_error = None;
    for args in attempts {
        match bridge::herdr_json(&args) {
            Ok(payload) => {
                let text = extract_read_text(&payload);
                return if text.is_empty() {
                    serde_json::to_string_pretty(&payload).map_err(|error| MirrorError(error.to_string()))
                } else {
                    Ok(text)
                };
            }
            Err(error) => last_error = Some(error.to_string()),
        }
    }
    let output = bridge::run_cmd(
        &["herdr", "pane", "read", pane_id, "--source", "recent-unwrapped", "--lines", &lines_text],
        Duration::from_secs(8),
        None,
    ).map_err(|error| MirrorError(error.to_string()))?;
    if output.returncode != 0 {
        let detail = if !output.stderr.trim().is_empty() {
            output.stderr.trim().to_string()
        } else if !output.stdout.trim().is_empty() {
            output.stdout.trim().to_string()
        } else {
            last_error.unwrap_or_else(|| format!("pane read failed for {pane_id}"))
        };
        return Err(MirrorError(detail));
    }
    Ok(output.stdout)
}

/// Herdr 0.8 exposes edge resize, not a one-shot PTY claim-size RPC.
pub fn resize_herdr_pane(pane_id: &str, cols: i64, rows: i64) {
    if pane_id.is_empty() || cols <= 0 || rows <= 0 {
        return;
    }
}

pub fn resize_pane_from_tty(pane_id: &str) -> Result<Option<Value>, MirrorError> {
    if !may_claim_client_size(pane_id) {
        return Ok(None);
    }
    let output = bridge::run_cmd(&["stty", "size"], Duration::from_secs(2), None)
        .map_err(|error| MirrorError(error.to_string()))?;
    if output.returncode != 0 {
        return Ok(None);
    }
    let dimensions: Vec<i64> = output.stdout.split_whitespace().filter_map(|value| value.parse().ok()).collect();
    if dimensions.len() != 2 {
        return Ok(None);
    }
    let rows = dimensions[0];
    let cols = dimensions[1];
    resize_herdr_pane(pane_id, cols, rows);
    Ok(Some(json!({"pane_id": pane_id, "cols": cols, "rows": rows})))
}

#[cfg(unix)]
fn set_stdin_nonblocking(enabled: bool, previous: &mut Option<i32>) {
    unsafe extern "C" {
        fn fcntl(fd: i32, command: i32, ...) -> i32;
    }
    const F_GETFL: i32 = 3;
    const F_SETFL: i32 = 4;
    const O_NONBLOCK: i32 = 0o4000;
    let fd = io::stdin().as_raw_fd();
    unsafe {
        if enabled {
            let flags = fcntl(fd, F_GETFL);
            if flags >= 0 {
                *previous = Some(flags);
                let _ = fcntl(fd, F_SETFL, flags | O_NONBLOCK);
            }
        } else if let Some(flags) = previous.take() {
            let _ = fcntl(fd, F_SETFL, flags);
        }
    }
}

#[cfg(not(unix))]
fn set_stdin_nonblocking(_enabled: bool, _previous: &mut Option<i32>) {}

fn drain_stdin_to_pane(pane_id: &str) {
    use std::io::IsTerminal;
    if !io::stdin().is_terminal() {
        return;
    }
    let mut buffer = [0_u8; 64];
    match io::stdin().read(&mut buffer) {
        Ok(0) => {}
        Ok(count) => {
            let text = String::from_utf8_lossy(&buffer[..count]);
            let _ = send_pane_text(pane_id, &text);
        }
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
        Err(_) => {}
    }
}

struct TtyModeGuard {
    saved: Option<String>,
}

impl TtyModeGuard {
    fn enter(enabled: bool) -> Self {
        use std::io::IsTerminal;
        if !enabled || !io::stdin().is_terminal() {
            return Self { saved: None };
        }
        let saved = bridge::run_cmd(&["stty", "-g"], Duration::from_secs(2), None)
            .ok()
            .filter(|output| output.returncode == 0)
            .map(|output| output.stdout.trim().to_string())
            .filter(|value| !value.is_empty());
        if saved.is_some() {
            let _ = bridge::run_cmd(&["stty", "cbreak", "-echo"], Duration::from_secs(2), None);
        }
        Self { saved }
    }
}

impl Drop for TtyModeGuard {
    fn drop(&mut self) {
        if let Some(saved) = self.saved.as_deref() {
            let _ = bridge::run_cmd(&["stty", saved], Duration::from_secs(2), None);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn attach_pane_loop_with<W, F, S>(
    pane_id: &str,
    interval: f64,
    _lines: i64,
    send_input: bool,
    raw_tty: bool,
    follow_resize: bool,
    _ansi: bool,
    max_iterations: Option<usize>,
    mut read_once: F,
    mut sleeper: S,
    stdout: &mut W,
) -> Result<i32, MirrorError>
where
    W: Write,
    F: FnMut() -> Result<String, MirrorError>,
    S: FnMut(Duration),
{
    use std::io::IsTerminal;
    env::set_var(ATTACH_ENV, pane_id);
    let stdin_is_tty = io::stdin().is_terminal();
    let _tty = TtyModeGuard::enter(raw_tty && send_input);
    let mut old_flags = None;
    if send_input && stdin_is_tty {
        set_stdin_nonblocking(true, &mut old_flags);
    }
    if follow_resize {
        let _ = resize_pane_from_tty(pane_id);
    }
    let header = format!("cmux-herdr attach-pane {pane_id}  (Ctrl-C to detach this viewer; Herdr pane stays alive)\n");
    let mut last: Option<String> = None;
    let mut iteration = 0;
    let result = loop {
        iteration += 1;
        let text = match read_once() {
            Ok(text) => text,
            Err(error) => {
                writeln!(stdout, "\ncmux-herdr: pane {pane_id} gone ({error})").map_err(|error| MirrorError(error.to_string()))?;
                stdout.flush().map_err(|error| MirrorError(error.to_string()))?;
                break Ok(1);
            }
        };
        let (chunk, full_redraw) = engine::output_delta(last.as_deref(), &text);
        if last.is_none() || full_redraw {
            write!(stdout, "\x1b[H\x1b[2J{header}{text}").map_err(|error| MirrorError(error.to_string()))?;
            if !text.ends_with('\n') {
                writeln!(stdout).map_err(|error| MirrorError(error.to_string()))?;
            }
            stdout.flush().map_err(|error| MirrorError(error.to_string()))?;
        } else if !chunk.is_empty() {
            write!(stdout, "{chunk}").map_err(|error| MirrorError(error.to_string()))?;
            stdout.flush().map_err(|error| MirrorError(error.to_string()))?;
        }
        last = Some(text);
        if send_input {
            drain_stdin_to_pane(pane_id);
        }
        if max_iterations.is_some_and(|limit| iteration >= limit) {
            break Ok(0);
        }
        sleeper(Duration::from_secs_f64(interval.max(0.05)));
    };
    if send_input && stdin_is_tty {
        set_stdin_nonblocking(false, &mut old_flags);
    }
    result
}

pub fn attach_pane_loop(
    pane_id: &str,
    interval: f64,
    lines: i64,
    send_input: bool,
    raw_tty: bool,
    follow_resize: bool,
    ansi: bool,
) -> Result<i32, MirrorError> {
    let mut output = io::stdout();
    attach_pane_loop_with(
        pane_id, interval, lines, send_input, raw_tty, follow_resize, ansi, None,
        || read_pane_text(pane_id, lines, ansi),
        std::thread::sleep,
        &mut output,
    )
}

pub fn wait_herdr_event(timeout: f64) -> bool {
    let timeout = Duration::from_secs_f64(timeout.max(0.0));
    let Some(mut session) = EventSession::try_open(None, Duration::from_secs_f64(timeout.as_secs_f64().max(0.1))) else {
        std::thread::sleep(Duration::from_secs_f64(timeout.as_secs_f64().max(0.05)));
        return false;
    };
    let result = session.wait(timeout).is_some();
    session.close();
    result
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
    #[derive(Default)]
    struct FakeCmuxRunner {
        replies: std::collections::VecDeque<Result<crate::bridge::CmdOutput, MirrorError>>,
        calls: Vec<Vec<String>>,
    }

    impl CmuxRunner for FakeCmuxRunner {
        fn run(
            &mut self,
            args: &[String],
            _workspace: Option<&str>,
        ) -> Result<crate::bridge::CmdOutput, MirrorError> {
            self.calls.push(args.to_vec());
            self.replies.pop_front().expect("fake cmux reply")
        }
    }

    fn command_output(code: i32, stdout: &str, stderr: &str) -> crate::bridge::CmdOutput {
        crate::bridge::CmdOutput {
            returncode: code,
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
        }
    }

    #[test]
    fn cmux_json_retries_without_json_flag() {
        let mut runner = FakeCmuxRunner::default();
        runner.replies.push_back(Ok(command_output(2, "", "unknown flag --json")));
        runner.replies.push_back(Ok(command_output(0, "OK\n{\"surface_id\":\"s1\"}\n", "")));
        let result = cmux_json_with(&mut runner, &["tree".to_string()], Some("workspace:1")).unwrap();
        assert_eq!(result["surface_id"], "s1");
        assert_eq!(runner.calls, vec![vec!["tree".to_string(), "--json".to_string()], vec!["tree".to_string()]]);
    }

    #[test]
    fn cmux_json_propagates_malformed_json_candidate() {
        let mut runner = FakeCmuxRunner::default();
        runner.replies.push_back(Ok(command_output(0, "OK\n{broken", "")));
        assert!(cmux_json_with(&mut runner, &["tree".to_string()], None).is_err());
    }

    #[test]
    fn live_surface_collection_ignores_empty_identifiers() {
        let mut found = HashSet::new();
        collect_ids(&json!({"pane_id":"", "nested":[{"surface_id":""}]}), &mut found);
        assert!(found.is_empty());
    }

    #[test]
    fn apply_split_failure_refuses_orphan_tab_fallback() {
        let desired = DesiredMirror {
            pane_id: "p2".into(), tab_id: "t".into(), workspace_id: "w".into(), title: "Child".into(),
            role: "split".into(), split_direction: "right".into(), agent: None, agent_status: "unknown".into(),
            split_ratio: None, split_from_pane_id: Some("p1".into()), tab_number: None, tab_index: Some(0),
            focused: false, zoomed: false, visible: true,
        };
        let plan = plan_mirror(&[desired], &json!({"p1": {"pane_id":"p1", "tab_id":"t", "role":"tab-root", "cmux_surface_id":"s1"}}), None, false, false, false, false, None);
        let mut runner = FakeCmuxRunner::default();
        for _ in 0..6 {
            runner.replies.push_back(Ok(command_output(1, "", "split denied")));
        }
        let result = apply_mirror_plan_with(
            &plan,
            &json!({"p1": {"pane_id":"p1", "tab_id":"t", "role":"tab-root", "cmux_surface_id":"s1"}}),
            Some("workspace:1"), false, false, None, &mut runner, false,
        ).unwrap();
        assert_eq!(result["created"], json!([]));
        assert!(result["errors"][0].as_str().unwrap().contains("refusing orphan-tab fallback"));
        assert!(runner.calls.iter().all(|args| !args.iter().any(|arg| arg == "create-terminal")));
    }

    #[test]
    fn engine_reconcile_protects_zoom_hidden_base_panes() {
        let snapshot = snapshot_from_session_payload(&json!({
            "panes": [
                {"pane_id":"p1", "tab_id":"t", "workspace_id":"w", "focused":true, "zoomed":true},
                {"pane_id":"p2", "tab_id":"t", "workspace_id":"w"}
            ],
            "tabs": [{"tab_id":"t", "workspace_id":"w", "label":"Agents", "number":1}],
            "layouts": {"t": {"horizontal":[{"pane_id":"p1"},{"pane_id":"p2"}]}}
        })).unwrap();
        let desired = desired_mirrors(&snapshot, "all", None, None, true).unwrap();
        let result = reconcile_engine_for_desired(&snapshot, &desired, &json!({}));
        assert_eq!(result.protected_pane_ids, vec!["p1", "p2"]);
        assert_eq!(result.created_pane_ids, vec!["p1", "p2"]);
    }
}
