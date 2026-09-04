//! Ordered host-apply verbs for one impose and reconcile pass.
//!
//! Behavioral port of `bridge/cmux_herdr_host.py`. Panel lifecycle follows the
//! base tree, while tree mutations and divider imposition follow the visible
//! tree. Ordering is load-bearing: create, close, tree, dividers, focus.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::engine::ReconcileResult;
use crate::impose::{DividerNode, ImposePlan};

/// One host verb. Its position in an action list is load-bearing.
#[derive(Debug, Clone, PartialEq)]
pub struct HostAction {
    pub op: String,
    pub pane_id: Option<String>,
    pub split_from_pane_id: Option<String>,
    pub orientation: Option<String>,
    pub fraction: Option<f64>,
    pub first_extent: Option<f64>,
    pub insert_first: bool,
    pub surface_id: Option<String>,
    pub skip_split_key: Option<String>,
    pub split_key: Option<String>,
}

impl HostAction {
    pub fn new(op: impl Into<String>) -> Self {
        Self {
            op: op.into(),
            pane_id: None,
            split_from_pane_id: None,
            orientation: None,
            fraction: None,
            first_extent: None,
            insert_first: false,
            surface_id: None,
            skip_split_key: None,
            split_key: None,
        }
    }

    pub fn with_pane_id(mut self, pane_id: impl Into<String>) -> Self {
        self.pane_id = Some(pane_id.into());
        self
    }

    pub fn with_surface_id(mut self, surface_id: impl Into<String>) -> Self {
        self.surface_id = Some(surface_id.into());
        self
    }
}

/// Walk the binary divider tree depth-first, omitting the held drag split.
pub fn divider_impose_actions(
    node: &DividerNode,
    held_split_key: Option<&str>,
    key_prefix: &str,
) -> Vec<HostAction> {
    fn walk(
        node: &DividerNode,
        key: &str,
        held_split_key: Option<&str>,
        actions: &mut Vec<HostAction>,
    ) {
        let DividerNode::Split(split) = node else {
            return;
        };
        if held_split_key != Some(key) {
            let mut action = HostAction::new("impose_divider");
            action.orientation = Some(split.orientation.clone());
            action.fraction = Some(split.fraction);
            action.first_extent = split.first_extent;
            action.split_key = Some(key.to_string());
            actions.push(action);
        }
        walk(&split.first, &format!("{key}.0"), held_split_key, actions);
        walk(&split.second, &format!("{key}.1"), held_split_key, actions);
    }

    let mut actions = Vec::new();
    walk(node, key_prefix, held_split_key, &mut actions);
    actions
}

/// Linearize one reconcile and impose pass into ordered host verbs.
pub fn host_actions(result: &ReconcileResult, plan: &ImposePlan) -> Vec<HostAction> {
    let mut actions = Vec::new();
    actions.extend(
        result
            .created_pane_ids
            .iter()
            .cloned()
            .map(|pane_id| HostAction::new("create_panel").with_pane_id(pane_id)),
    );
    actions.extend(
        result
            .closed_pane_ids
            .iter()
            .cloned()
            .map(|pane_id| HostAction::new("close_panel").with_pane_id(pane_id)),
    );

    match plan.tree_action.kind.as_str() {
        "rebuild" => actions.push(HostAction::new("rebuild_tree")),
        "keep" => actions.push(HostAction::new("keep_tree")),
        "expand_leaf" => {
            if let Some(expansion) = &plan.tree_action.expansion {
                let mut action =
                    HostAction::new("expand_leaf").with_pane_id(expansion.new_pane_id.clone());
                action.split_from_pane_id = Some(expansion.existing_pane_id.clone());
                action.orientation = Some(expansion.orientation.clone());
                action.fraction = Some(expansion.fraction);
                action.insert_first = expansion.insert_first;
                actions.push(action);
            } else {
                actions.push(HostAction::new("rebuild_tree"));
            }
        }
        "remove_leaf" => {
            let mut action = HostAction::new("remove_leaf");
            action.pane_id = plan.tree_action.removed_pane_id.clone();
            actions.push(action);
        }
        _ => actions.push(HostAction::new("rebuild_tree")),
    }

    actions.extend(divider_impose_actions(
        &plan.divider_tree,
        plan.held_split_key.as_deref(),
        "s",
    ));
    if let Some(pane_id) = &plan.focus_pane_id {
        actions.push(HostAction::new("focus").with_pane_id(pane_id.clone()));
    }
    actions
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostError(pub String);

impl fmt::Display for HostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for HostError {}

/// In-memory Bonsplit stand-in that validates action ordering and preconditions.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FakeBonsplitHost {
    pub panels: BTreeSet<String>,
    pub surfaces: BTreeMap<String, String>,
    pub focus: Option<String>,
    pub last_tree_op: Option<String>,
    pub imposed: Vec<HostAction>,
    pub log: Vec<String>,
}

impl FakeBonsplitHost {
    /// Apply every verb in order. Unknown or invalid verbs fail closed.
    pub fn apply(&mut self, actions: &[HostAction]) -> Result<(), HostError> {
        for action in actions {
            self.apply_one(action)?;
        }
        Ok(())
    }

    fn apply_one(&mut self, action: &HostAction) -> Result<(), HostError> {
        match action.op.as_str() {
            "create_panel" => {
                let pane_id = required(&action.pane_id, "create_panel requires pane_id")?;
                self.panels.insert(pane_id.to_string());
                self.log.push(format!("create:{pane_id}"));
            }
            "close_panel" => {
                if let Some(pane_id) = action.pane_id.as_ref().filter(|id| !id.is_empty()) {
                    self.panels.remove(pane_id);
                    self.surfaces.remove(pane_id);
                    self.log.push(format!("close:{pane_id}"));
                }
            }
            "bind_surface" => {
                if let (Some(pane_id), Some(surface_id)) = (
                    action.pane_id.as_ref().filter(|id| !id.is_empty()),
                    action.surface_id.as_ref().filter(|id| !id.is_empty()),
                ) {
                    if !self.panels.contains(pane_id) {
                        return Err(HostError(format!(
                            "bind_surface before create_panel: {pane_id}"
                        )));
                    }
                    self.surfaces.insert(pane_id.clone(), surface_id.clone());
                }
            }
            "rebuild_tree" | "keep_tree" | "expand_leaf" | "remove_leaf" => {
                let missing = required_panes(action)
                    .into_iter()
                    .filter(|pane_id| !self.panels.contains(*pane_id))
                    .cloned()
                    .collect::<Vec<_>>();
                if !missing.is_empty() {
                    return Err(HostError(format!(
                        "{} missing panels {:?}",
                        action.op, missing
                    )));
                }
                self.last_tree_op = Some(action.op.clone());
                self.log.push(action.op.clone());
            }
            "impose_divider" => {
                self.imposed.push(action.clone());
                self.log.push(format!(
                    "impose:{}",
                    action.split_key.as_deref().unwrap_or("None")
                ));
            }
            "focus" => {
                if let Some(pane_id) = action.pane_id.as_ref().filter(|id| !id.is_empty()) {
                    if !self.panels.contains(pane_id) {
                        return Err(HostError(format!("focus missing panel {pane_id}")));
                    }
                }
                self.focus = action.pane_id.clone();
                self.log.push(format!(
                    "focus:{}",
                    action.pane_id.as_deref().unwrap_or("None")
                ));
            }
            op => return Err(HostError(format!("unknown host op {op}"))),
        }
        Ok(())
    }
}

fn required<'a>(value: &'a Option<String>, message: &str) -> Result<&'a str, HostError> {
    value
        .as_deref()
        .filter(|item| !item.is_empty())
        .ok_or_else(|| HostError(message.to_string()))
}

fn required_panes(action: &HostAction) -> Vec<&String> {
    if action.op != "expand_leaf" {
        return Vec::new();
    }
    action
        .split_from_pane_id
        .iter()
        .chain(action.pane_id.iter())
        .filter(|pane_id| !pane_id.is_empty())
        .collect()
}
