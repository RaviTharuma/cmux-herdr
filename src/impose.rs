//! Host-agnostic Bonsplit impose planning.
//!
//! Behavioral port of `bridge/cmux_herdr_impose.py`: builds the right-associated
//! divider tree, chooses targeted tree actions, computes exact extents when
//! metrics are available, and translates divider drags back to cell counts.

use std::collections::HashSet;

use crate::layout::{split_specs, LayoutNode, LayoutRect, SplitSpec};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImposeSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImposeMetrics {
    pub cell_width: f64,
    pub cell_height: f64,
    pub divider_thickness: f64,
    pub tab_bar_height: f64,
    pub surface_pad_width: f64,
    pub surface_pad_height: f64,
    pub minimum_pane_extent: f64,
}

impl Default for ImposeMetrics {
    fn default() -> Self {
        Self {
            cell_width: 0.0,
            cell_height: 0.0,
            divider_thickness: 0.0,
            tab_bar_height: 0.0,
            surface_pad_width: 0.0,
            surface_pad_height: 0.0,
            minimum_pane_extent: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DividerLeaf {
    pub pane_id: String,
    pub outer: Option<ImposeSize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DividerSplit {
    pub orientation: String,
    pub fraction: f64,
    pub first_extent: Option<f64>,
    pub first: Box<DividerNode>,
    pub second: Box<DividerNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DividerNode {
    Leaf(DividerLeaf),
    Split(DividerSplit),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LeafExpansion {
    pub existing_pane_id: String,
    pub new_pane_id: String,
    pub orientation: String,
    pub insert_first: bool,
    pub fraction: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TreeAction {
    pub kind: String,
    pub expansion: Option<LeafExpansion>,
    pub removed_pane_id: Option<String>,
}

impl TreeAction {
    fn simple(kind: &str) -> Self {
        Self {
            kind: kind.to_string(),
            expansion: None,
            removed_pane_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DividerDragHold {
    pub split_key: String,
    pub axis: String,
    pub target_cells: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImposePlan {
    pub tree_action: TreeAction,
    pub divider_tree: DividerNode,
    pub focus_pane_id: Option<String>,
    pub title: String,
    pub held_split_key: Option<String>,
    pub fractions: Vec<f64>,
}

/// Minimal adapter used by [`plan_from_reconcile`] to avoid coupling impose
/// planning to the engine module's concrete reconcile result.
pub trait ReconcileResultLike {
    fn rendered_layout(&self) -> &LayoutNode;
    fn focus_pane_id(&self) -> Option<&str>;
}

fn py_min(first: f64, second: f64) -> f64 {
    if second < first { second } else { first }
}

fn py_max(first: f64, second: f64) -> f64 {
    if second > first { second } else { first }
}

fn divider_fraction_totals(first_span: i128, rest_span: i128) -> f64 {
    let first = first_span.max(0);
    let rest = rest_span.max(0);
    let denominator = (first + rest + 1).max(1);
    clamp_ratio(first as f64 / denominator as f64)
}

/// Keep a divider fraction inside the range cmux/Bonsplit accepts.
pub fn clamp_ratio(value: f64) -> f64 {
    py_min(0.95, py_max(0.05, value))
}

/// Tmux fraction: first / (first + rest + one divider cell).
pub fn divider_fraction(first_span: i64, rest_spans: &[i64]) -> f64 {
    let first = i128::from(first_span);
    let rest = rest_spans
        .iter()
        .map(|span| i128::from(*span).max(0))
        .sum();
    divider_fraction_totals(first, rest)
}

/// Choose an exact-fit plan parent without ever exceeding the banked region.
pub fn region_bounded_plan_parent(
    render: Option<ImposeSize>,
    region: Option<ImposeSize>,
) -> Option<ImposeSize> {
    let parent = render.or(region)?;
    match region {
        None => Some(parent),
        Some(region) => Some(ImposeSize {
            width: py_min(parent.width, region.width),
            height: py_min(parent.height, region.height),
        }),
    }
}

/// Compare split nesting and pane ids while ignoring geometry.
pub fn same_shape_and_pane_ids(lhs: &LayoutNode, rhs: &LayoutNode) -> bool {
    if lhs.kind != rhs.kind {
        return false;
    }
    if lhs.kind == "pane" {
        return lhs.pane_id == rhs.pane_id;
    }
    lhs.children.len() == rhs.children.len()
        && lhs
            .children
            .iter()
            .zip(&rhs.children)
            .all(|(left, right)| same_shape_and_pane_ids(left, right))
}

fn _span(node: &LayoutNode, horizontal: bool) -> i64 {
    if horizontal {
        node.rect.width
    } else {
        node.rect.height
    }
}

fn _two_leaf_split(node: &LayoutNode) -> Option<(String, Vec<String>, f64)> {
    if node.kind == "pane" || node.children.len() != 2 {
        return None;
    }
    let pane_ids: Option<Vec<String>> = node
        .children
        .iter()
        .map(|child| {
            if child.kind != "pane" {
                return None;
            }
            child.pane_id.as_ref().filter(|id| !id.is_empty()).cloned()
        })
        .collect();
    let pane_ids = pane_ids?;
    let orientation = if node.kind == "horizontal" {
        "horizontal"
    } else {
        "vertical"
    };
    let horizontal = orientation == "horizontal";
    let fraction = divider_fraction(
        _span(&node.children[0], horizontal),
        &[_span(&node.children[1], horizontal)],
    );
    Some((orientation.to_string(), pane_ids, fraction))
}

/// Find a targeted one-leaf expansion.
pub fn leaf_expansion(
    old_node: &LayoutNode,
    new_node: &LayoutNode,
    added_pane_id: &str,
) -> Option<LeafExpansion> {
    if old_node.kind == "pane" {
        let old_pane_id = old_node.pane_id.as_deref().filter(|id| !id.is_empty())?;
        let (orientation, pane_ids, fraction) = _two_leaf_split(new_node)?;
        if pane_ids.iter().any(|id| id == old_pane_id)
            && pane_ids.iter().any(|id| id == added_pane_id)
        {
            return Some(LeafExpansion {
                existing_pane_id: old_pane_id.to_string(),
                new_pane_id: added_pane_id.to_string(),
                orientation,
                insert_first: pane_ids[0] == added_pane_id,
                fraction,
            });
        }
        return None;
    }

    if matches!(old_node.kind.as_str(), "horizontal" | "vertical")
        && old_node.kind == new_node.kind
        && old_node.children.len() == new_node.children.len()
    {
        for (old_child, new_child) in old_node.children.iter().zip(&new_node.children) {
            if let Some(expansion) = leaf_expansion(old_child, new_child, added_pane_id) {
                return Some(expansion);
            }
        }
    }
    None
}

/// Decide whether the host should rebuild, keep, expand, or remove a leaf.
pub fn tree_action(
    previous_rendered: Option<&LayoutNode>,
    rendered: &LayoutNode,
) -> TreeAction {
    let Some(previous) = previous_rendered else {
        return TreeAction::simple("rebuild");
    };
    if same_shape_and_pane_ids(previous, rendered) {
        return TreeAction::simple("keep");
    }

    let old_ids = previous.pane_ids_in_order();
    let new_ids = rendered.pane_ids_in_order();
    let old_set: HashSet<&str> = old_ids.iter().map(String::as_str).collect();
    let new_set: HashSet<&str> = new_ids.iter().map(String::as_str).collect();
    let added: Vec<&str> = new_set.difference(&old_set).copied().collect();
    let removed: Vec<&str> = old_set.difference(&new_set).copied().collect();

    if new_set.len() == old_set.len() + 1 && added.len() == 1 {
        if let Some(expansion) = leaf_expansion(previous, rendered, added[0]) {
            return TreeAction {
                kind: "expand_leaf".to_string(),
                expansion: Some(expansion),
                removed_pane_id: None,
            };
        }
    }
    if old_set.len() == new_set.len() + 1 && removed.len() == 1 {
        return TreeAction {
            kind: "remove_leaf".to_string(),
            expansion: None,
            removed_pane_id: Some(removed[0].to_string()),
        };
    }
    TreeAction::simple("rebuild")
}

fn _first_extent(
    first_span: i64,
    rest_span: i128,
    parent_extent: f64,
    metrics: &ImposeMetrics,
    horizontal: bool,
) -> (f64, f64) {
    let available = parent_extent - metrics.divider_thickness;
    if available <= 0.0 {
        return (0.0, divider_fraction_totals(i128::from(first_span), rest_span));
    }
    let cell = if horizontal {
        metrics.cell_width
    } else {
        metrics.cell_height
    };
    let pad = if horizontal {
        metrics.surface_pad_width
    } else {
        metrics.surface_pad_height
    };
    let first_ideal = first_span as f64 * cell + pad;
    let rest_ideal = rest_span as f64 * cell + pad;
    let total_ideal = first_ideal + rest_ideal;
    if total_ideal <= 0.0 {
        let fraction = divider_fraction_totals(i128::from(first_span), rest_span);
        return (available * fraction, fraction);
    }

    let mut raw = available * (first_ideal / total_ideal);
    let floor = metrics.minimum_pane_extent;
    if floor > 0.0 && available > 2.0 * floor {
        raw = py_min(available - floor, py_max(floor, raw));
    }
    raw = py_min(available, py_max(0.0, raw));
    let fraction = clamp_ratio(raw / available);
    (raw, fraction)
}

/// Build the right-associated binary view of an n-ary layout.
pub fn binary_tree(
    node: &LayoutNode,
    metrics: Option<&ImposeMetrics>,
    parent: Option<ImposeSize>,
) -> DividerNode {
    if node.kind == "pane" {
        return DividerNode::Leaf(DividerLeaf {
            pane_id: node.pane_id.clone().unwrap_or_default(),
            outer: parent,
        });
    }

    let horizontal = node.kind == "horizontal";
    let orientation = if horizontal {
        "horizontal"
    } else {
        "vertical"
    };
    if node.children.is_empty() {
        return DividerNode::Leaf(DividerLeaf {
            pane_id: String::new(),
            outer: parent,
        });
    }
    if node.children.len() == 1 {
        return binary_tree(&node.children[0], metrics, parent);
    }

    let first = &node.children[0];
    let rest = &node.children[1..];
    let first_span = _span(first, horizontal);
    let rest_spans: Vec<i64> = rest.iter().map(|child| _span(child, horizontal)).collect();
    let rest_span: i128 = rest_spans.iter().map(|span| i128::from(*span)).sum();

    let mut first_size = None;
    let mut second_size = None;
    let mut first_extent = None;
    let fraction;
    if let (Some(parent), Some(metrics)) = (parent, metrics) {
        let parent_extent = if horizontal {
            parent.width
        } else {
            parent.height
        };
        let (extent, computed_fraction) =
            _first_extent(first_span, rest_span, parent_extent, metrics, horizontal);
        first_extent = Some(extent);
        fraction = computed_fraction;
        if horizontal {
            first_size = Some(ImposeSize {
                width: extent,
                height: parent.height,
            });
            second_size = Some(ImposeSize {
                width: py_max(0.0, parent.width - extent - metrics.divider_thickness),
                height: parent.height,
            });
        } else {
            first_size = Some(ImposeSize {
                width: parent.width,
                height: extent,
            });
            second_size = Some(ImposeSize {
                width: parent.width,
                height: py_max(0.0, parent.height - extent - metrics.divider_thickness),
            });
        }
    } else {
        fraction = divider_fraction(first_span, &rest_spans);
    }

    let rest_node = if rest.len() == 1 {
        rest[0].clone()
    } else {
        _combine(rest, horizontal)
    };
    DividerNode::Split(DividerSplit {
        orientation: orientation.to_string(),
        fraction,
        first_extent,
        first: Box::new(binary_tree(first, metrics, first_size)),
        second: Box::new(binary_tree(&rest_node, metrics, second_size)),
    })
}

fn _combine(children: &[LayoutNode], horizontal: bool) -> LayoutNode {
    if children.len() == 1 {
        return children[0].clone();
    }
    let min_x = children.iter().map(|child| child.rect.x).min().unwrap();
    let min_y = children.iter().map(|child| child.rect.y).min().unwrap();
    let right = children
        .iter()
        .map(|child| child.rect.x + child.rect.width)
        .max()
        .unwrap();
    let bottom = children
        .iter()
        .map(|child| child.rect.y + child.rect.height)
        .max()
        .unwrap();
    LayoutNode {
        kind: if horizontal { "horizontal" } else { "vertical" }.to_string(),
        pane_id: None,
        children: children.to_vec(),
        rect: LayoutRect {
            x: min_x,
            y: min_y,
            width: (right - min_x).max(1),
            height: (bottom - min_y).max(1),
        },
    }
}

/// Return divider fractions in depth-first order.
pub fn collect_fractions(node: &DividerNode) -> Vec<f64> {
    match node {
        DividerNode::Leaf(_) => Vec::new(),
        DividerNode::Split(split) => {
            let mut fractions = vec![split.fraction];
            fractions.extend(collect_fractions(&split.first));
            fractions.extend(collect_fractions(&split.second));
            fractions
        }
    }
}

/// Start a divider drag hold.
pub fn begin_divider_drag(
    split_key: &str,
    axis: &str,
    assigned_cells: i64,
) -> DividerDragHold {
    DividerDragHold {
        split_key: split_key.to_string(),
        axis: axis.to_string(),
        target_cells: assigned_cells.max(1),
    }
}

/// Clear a hold once its assignment lands or its split disappears.
pub fn resolve_divider_hold(
    hold: Option<DividerDragHold>,
    assigned_cells: Option<i64>,
    split_still_exists: bool,
) -> Option<DividerDragHold> {
    let hold = hold?;
    if !split_still_exists || assigned_cells.is_none() || assigned_cells == Some(hold.target_cells) {
        None
    } else {
        Some(hold)
    }
}

/// Convert a settled divider extent to cells and report whether a send is needed.
pub fn end_divider_drag(
    dragged_extent: f64,
    axis_span: f64,
    total_cells: i64,
    assigned_cells: i64,
) -> (i64, bool) {
    let cells = if axis_span <= 0.0 || total_cells < 1 {
        1
    } else {
        let fraction = clamp_ratio(dragged_extent / axis_span);
        let rounded = (fraction * total_cells as f64).round_ties_even() as i64;
        (total_cells - 1).max(1).min(rounded.max(1))
    };
    (cells, cells != assigned_cells)
}

/// Build one host impose plan for a rendered layout tree.
#[allow(clippy::too_many_arguments)]
pub fn plan_impose(
    rendered: &LayoutNode,
    previous_rendered: Option<&LayoutNode>,
    focus_pane_id: Option<&str>,
    title: &str,
    metrics: Option<&ImposeMetrics>,
    render_size: Option<ImposeSize>,
    region_size: Option<ImposeSize>,
    hold: Option<&DividerDragHold>,
) -> ImposePlan {
    let parent = region_bounded_plan_parent(render_size, region_size);
    let divider_tree = binary_tree(rendered, metrics, parent);
    let fractions = collect_fractions(&divider_tree);
    ImposePlan {
        tree_action: tree_action(previous_rendered, rendered),
        divider_tree,
        focus_pane_id: focus_pane_id.map(str::to_string),
        title: title.to_string(),
        held_split_key: hold.map(|value| value.split_key.clone()),
        fractions,
    }
}

/// Overlay impose fractions onto the sequential split plan.
pub fn specs_with_impose_fractions(node: &LayoutNode) -> Vec<SplitSpec> {
    let specs = split_specs(node);
    let fractions = collect_fractions(&binary_tree(node, None, None));
    specs
        .into_iter()
        .enumerate()
        .map(|(index, spec)| SplitSpec {
            pane_id: spec.pane_id,
            split_from_pane_id: spec.split_from_pane_id,
            direction: spec.direction,
            ratio: fractions.get(index).copied().or(spec.ratio),
        })
        .collect()
}

/// Build an impose plan from an engine reconcile result.
#[allow(clippy::too_many_arguments)]
pub fn plan_from_reconcile<R: ReconcileResultLike>(
    result: &R,
    previous_rendered: Option<&LayoutNode>,
    title: &str,
    metrics: Option<&ImposeMetrics>,
    render_size: Option<ImposeSize>,
    region_size: Option<ImposeSize>,
    hold: Option<&DividerDragHold>,
) -> ImposePlan {
    plan_impose(
        result.rendered_layout(),
        previous_rendered,
        result.focus_pane_id(),
        title,
        metrics,
        render_size,
        region_size,
        hold,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::parse_layout;
    use serde_json::json;

    fn parsed(raw: serde_json::Value) -> LayoutNode {
        parse_layout(&raw).unwrap()
    }

    fn horizontal() -> LayoutNode {
        parsed(json!({
            "width": 200, "height": 50, "x": 0, "y": 0,
            "horizontal": [
                {"width": 100, "height": 50, "x": 0, "y": 0, "pane": "a"},
                {"width": 99, "height": 50, "x": 101, "y": 0, "pane": "b"}
            ]
        }))
    }

    #[test]
    fn divider_fraction_includes_separator_and_clamps() {
        assert_eq!(divider_fraction(100, &[99]), 0.5);
        assert_eq!(divider_fraction(1, &[1000]), 0.05);
        assert_eq!(divider_fraction(1000, &[1]), 0.95);
    }

    #[test]
    fn plan_parent_never_exceeds_region() {
        let parent = region_bounded_plan_parent(
            Some(ImposeSize { width: 900.0, height: 500.0 }),
            Some(ImposeSize { width: 800.0, height: 400.0 }),
        ).unwrap();
        assert_eq!(parent, ImposeSize { width: 800.0, height: 400.0 });
    }

    #[test]
    fn geometry_only_change_keeps_tree() {
        let node = horizontal();
        let mut wider = node.clone();
        wider.rect.width = 400;
        assert!(same_shape_and_pane_ids(&node, &wider));
        assert_eq!(tree_action(Some(&node), &wider).kind, "keep");
    }

    #[test]
    fn leaf_addition_and_removal_are_targeted() {
        let split = horizontal();
        let leaf = parsed(json!({"pane": "a", "width": 200, "height": 50}));
        let expansion = tree_action(Some(&leaf), &split);
        assert_eq!(expansion.kind, "expand_leaf");
        assert_eq!(expansion.expansion.unwrap().new_pane_id, "b");
        let removal = tree_action(Some(&split), &leaf);
        assert_eq!(removal.kind, "remove_leaf");
        assert_eq!(removal.removed_pane_id.as_deref(), Some("b"));
    }

    #[test]
    fn metrics_keep_extent_inside_parent() {
        let tree = binary_tree(
            &horizontal(),
            Some(&ImposeMetrics {
                cell_width: 8.0,
                cell_height: 16.0,
                divider_thickness: 4.0,
                ..ImposeMetrics::default()
            }),
            Some(ImposeSize { width: 800.0, height: 400.0 }),
        );
        let DividerNode::Split(split) = tree else { panic!("expected split") };
        assert!(split.first_extent.unwrap() > 0.0);
        assert!(split.first_extent.unwrap() <= 800.0);
    }

    #[test]
    fn divider_hold_clears_only_after_target_or_disappearance() {
        let hold = begin_divider_drag("split", "horizontal", 40);
        assert_eq!(
            resolve_divider_hold(Some(hold.clone()), Some(50), true),
            Some(hold.clone())
        );
        assert_eq!(resolve_divider_hold(Some(hold.clone()), Some(40), true), None);
        assert_eq!(resolve_divider_hold(Some(hold), Some(50), false), None);
    }
}
