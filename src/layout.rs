//! Herdr layout trees → cmux split plan.
//!
//! Port of `bridge/cmux_herdr_layout.py`. Parses Herdr/tmux-shaped layout
//! payloads into a [`LayoutNode`] tree, reconstructs a binary tree from pane
//! rectangles (BSP) when only geometry is present, and emits a sequential
//! [`SplitSpec`] plan. Pure: depends only on `serde_json`.

use std::collections::HashMap;

use serde_json::Value;

/// Cell (or pixel) rectangle. Units are whatever Herdr reported (`LayoutRect`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutRect {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

impl Default for LayoutRect {
    fn default() -> Self {
        LayoutRect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        }
    }
}

impl LayoutRect {
    /// Bounding rectangle of `self` and `other` (`union`).
    pub fn union(&self, other: &LayoutRect) -> LayoutRect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = (self.x + self.width).max(other.x + other.width);
        let bottom = (self.y + self.height).max(other.y + other.height);
        LayoutRect {
            x,
            y,
            width: (right - x).max(1),
            height: (bottom - y).max(1),
        }
    }

    /// Fraction of this rect occupied by `child` along `axis`
    /// (`first_child_ratio`). Clamped to `[0.05, 0.95]`.
    pub fn first_child_ratio(&self, child: &LayoutRect, axis: &str) -> f64 {
        let (span, part) = if axis == "horizontal" {
            (self.width, child.width)
        } else {
            (self.height, child.height)
        };
        if span <= 0 {
            return 0.5;
        }
        (part as f64 / span as f64).clamp(0.05, 0.95)
    }
}

/// One node in a Herdr tab's pane tree (`LayoutNode`). `kind` is `pane`,
/// `horizontal`, or `vertical`.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutNode {
    pub kind: String,
    pub pane_id: Option<String>,
    pub children: Vec<LayoutNode>,
    pub rect: LayoutRect,
}

impl LayoutNode {
    fn leaf(pane_id: Option<String>, rect: LayoutRect) -> Self {
        LayoutNode {
            kind: "pane".into(),
            pane_id,
            children: Vec::new(),
            rect,
        }
    }

    /// Depth-first left→right leaf ids (`pane_ids_in_order`).
    pub fn pane_ids_in_order(&self) -> Vec<String> {
        if self.kind == "pane" {
            return self.pane_id.iter().cloned().collect();
        }
        let mut out = Vec::new();
        for child in &self.children {
            out.extend(child.pane_ids_in_order());
        }
        out
    }

    /// Divider fraction for the first child of a split, or `None` for a leaf
    /// (`first_child_ratio`).
    pub fn first_child_ratio(&self) -> Option<f64> {
        if self.kind == "pane" || self.children.len() < 2 {
            return None;
        }
        let axis = if self.kind == "horizontal" {
            "horizontal"
        } else {
            "vertical"
        };
        Some(self.rect.first_child_ratio(&self.children[0].rect, axis))
    }

    /// Stable fingerprint of split nesting + pane set (`structure_signature`).
    pub fn structure_signature(&self) -> String {
        if self.kind == "pane" {
            return format!("p:{}", self.pane_id.as_deref().unwrap_or(""));
        }
        let inner = self
            .children
            .iter()
            .map(|c| c.structure_signature())
            .collect::<Vec<_>>()
            .join(",");
        let first = self.kind.chars().next().unwrap_or_default();
        format!("{first}:{inner}")
    }
}

/// One sequential cmux split needed to realize a layout tree (`SplitSpec`).
#[derive(Debug, Clone, PartialEq)]
pub struct SplitSpec {
    pub pane_id: String,
    pub split_from_pane_id: String,
    pub direction: String,
    pub ratio: Option<f64>,
}

// --- primitive coercions -----------------------------------------------------

/// `_as_int` — coerce to int; bools → default, floats truncate toward zero,
/// non-numeric strings → default.
fn as_int(value: Option<&Value>, default: i64) -> i64 {
    match value {
        Some(Value::Bool(_)) => default,
        Some(Value::Number(n)) => n
            .as_i64()
            .or_else(|| n.as_u64().map(|u| u as i64))
            .or_else(|| n.as_f64().map(|f| f as i64))
            .unwrap_or(default),
        Some(Value::String(s)) if !s.trim().is_empty() => {
            s.trim().parse::<f64>().map(|f| f as i64).unwrap_or(default)
        }
        _ => default,
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

/// Python `str(value)` for the value shapes reachable here.
fn value_to_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => if *b { "True" } else { "False" }.into(),
        Value::Null => "None".into(),
        other => other.to_string(),
    }
}

/// Python `a or b or ...` over object keys: first truthy value, else the last
/// evaluated operand (which may be absent → `None`).
fn py_or<'a>(raw: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    let mut last = None;
    for k in keys {
        let v = raw.get(*k);
        last = v;
        if truthy(v) {
            return v;
        }
    }
    last
}

/// Best-effort rectangle from a pane dict, geometry object, or layout node
/// (`rect_from_mapping`).
pub fn rect_from_mapping(raw: &Value) -> Option<LayoutRect> {
    if !raw.is_object() {
        return None;
    }
    let geom = raw.get("geometry").filter(|v| v.is_object());
    let rect = raw.get("rect").filter(|v| v.is_object());
    // src = geom or rect or raw (empty dict is falsy).
    let src = [geom, rect]
        .into_iter()
        .flatten()
        .find(|v| truthy(Some(v)))
        .unwrap_or(raw);

    let width = as_int(
        py_or(src, &["width", "cols", "columns", "pane_width", "w"]),
        0,
    );
    let height = as_int(py_or(src, &["height", "rows", "pane_height", "h"]), 0);
    let x = as_int(py_or(src, &["x", "left", "pane_left", "col"]), 0);
    let y = as_int(py_or(src, &["y", "top", "pane_top", "row"]), 0);
    if width <= 0 && height <= 0 {
        return None;
    }
    Some(LayoutRect {
        x,
        y,
        width: width.max(1),
        height: height.max(1),
    })
}

/// True when Herdr reports this pane as the zoomed (visible) leaf
/// (`pane_is_zoomed`).
pub fn pane_is_zoomed(raw: &Value) -> bool {
    if !raw.is_object() {
        return false;
    }
    if truthy(raw.get("zoomed")) || truthy(raw.get("is_zoomed")) || truthy(raw.get("zoom")) {
        return true;
    }
    let flags = raw.get("flags").map(value_to_str).unwrap_or_default();
    flags.contains('Z')
}

// --- parsing -----------------------------------------------------------------

fn pane_id_from(raw: &Value) -> Option<String> {
    for key in ["pane_id", "pane", "id"] {
        let value = raw.get(key);
        match value {
            Some(Value::String(_)) | Some(Value::Number(_)) => {
                let text = value_to_str(value.unwrap());
                let text = text.trim();
                if text.is_empty() {
                    continue;
                }
                if key == "id" && matches!(text, "horizontal" | "vertical" | "split") {
                    continue;
                }
                return Some(text.to_string());
            }
            _ => continue,
        }
    }
    None
}

fn looks_like_split(raw: &Value, kind: &str) -> bool {
    if matches!(
        kind,
        "split"
            | "hsplit"
            | "vsplit"
            | "horizontal"
            | "vertical"
            | "row"
            | "column"
            | "cols"
            | "rows"
    ) {
        return true;
    }
    [
        "children",
        "first",
        "second",
        "horizontal",
        "vertical",
        "nodes",
    ]
    .iter()
    .any(|k| raw.get(*k).is_some())
}

fn normalize_direction(text: &str) -> Option<String> {
    let text = text.to_lowercase();
    let text = text.trim();
    if matches!(
        text,
        "horizontal" | "hsplit" | "h" | "row" | "cols" | "right" | "left" | "x"
    ) {
        return Some("horizontal".into());
    }
    if matches!(
        text,
        "vertical" | "vsplit" | "v" | "column" | "rows" | "down" | "up" | "y"
    ) {
        return Some("vertical".into());
    }
    if text == "split" {
        return Some("horizontal".into());
    }
    None
}

fn from_tmux_named(raw: &Value, axis: &str) -> LayoutNode {
    let kids: Vec<LayoutNode> = raw
        .get(axis)
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(parse_layout).collect())
        .unwrap_or_default();
    nary_split(axis, kids, Some(raw))
}

fn nary_split(direction: &str, children: Vec<LayoutNode>, raw: Option<&Value>) -> LayoutNode {
    if children.len() == 1 {
        return children.into_iter().next().unwrap();
    }
    let mut rect = raw.and_then(rect_from_mapping);
    if rect.is_none() && !children.is_empty() {
        let mut bound = children[0].rect;
        for child in &children[1..] {
            bound = bound.union(&child.rect);
        }
        rect = Some(bound);
    }
    let kind = if matches!(direction, "horizontal" | "vertical") {
        direction
    } else {
        "horizontal"
    };
    LayoutNode {
        kind: kind.into(),
        pane_id: None,
        children,
        rect: rect.unwrap_or_default(),
    }
}

/// Parse a Herdr/tmux-shaped layout payload into a [`LayoutNode`]
/// (`parse_layout`).
pub fn parse_layout(raw: &Value) -> Option<LayoutNode> {
    match raw {
        Value::Null => None,
        Value::Array(arr) => {
            if arr.is_empty() {
                return None;
            }
            if arr.len() == 1 {
                return parse_layout(&arr[0]);
            }
            let children: Vec<LayoutNode> = arr.iter().filter_map(parse_layout).collect();
            if children.is_empty() {
                return None;
            }
            Some(nary_split("vertical", children, None))
        }
        Value::Object(_) => parse_layout_object(raw),
        _ => None,
    }
}

fn parse_layout_object(raw: &Value) -> Option<LayoutNode> {
    for wrap in ["layout", "root", "tree", "node"] {
        if let Some(nested) = raw.get(wrap) {
            if let Some(node) = parse_layout(nested) {
                return Some(node);
            }
        }
    }

    let pane_id = pane_id_from(raw);
    let kind = py_or(raw, &["kind", "type", "orientation"])
        .map(value_to_str)
        .unwrap_or_default()
        .to_lowercase();

    if matches!(raw.get("horizontal"), Some(Value::Array(_))) {
        return Some(from_tmux_named(raw, "horizontal"));
    }
    if matches!(raw.get("vertical"), Some(Value::Array(_))) {
        return Some(from_tmux_named(raw, "vertical"));
    }

    if matches!(kind.as_str(), "pane" | "leaf")
        || (pane_id.is_some() && !looks_like_split(raw, &kind))
    {
        let rect = rect_from_mapping(raw).unwrap_or_default();
        return Some(LayoutNode::leaf(pane_id, rect));
    }

    let dir_input: String =
        match py_or(raw, &["direction", "dir", "axis"]).filter(|v| truthy(Some(v))) {
            Some(v) => value_to_str(v),
            None => {
                if kind != "split" {
                    kind.clone()
                } else {
                    String::new()
                }
            }
        };
    let direction = normalize_direction(&dir_input);

    let children_raw = py_or(raw, &["children", "nodes", "panes"]);
    let first = py_or(raw, &["first", "a", "left", "top"]);
    let second = py_or(raw, &["second", "b", "right", "bottom"]);

    if first.is_some() || second.is_some() {
        let kids: Vec<LayoutNode> = [first, second]
            .into_iter()
            .flatten()
            .filter_map(parse_layout)
            .collect();
        if !kids.is_empty() {
            return Some(nary_split(
                direction.as_deref().unwrap_or("horizontal"),
                kids,
                Some(raw),
            ));
        }
    }

    if let Some(Value::Array(arr)) = children_raw {
        if !arr.is_empty() {
            let kids: Vec<LayoutNode> = arr.iter().filter_map(parse_layout).collect();
            if !kids.is_empty() {
                return Some(nary_split(
                    direction.as_deref().unwrap_or("horizontal"),
                    kids,
                    Some(raw),
                ));
            }
        }
    }

    if pane_id.is_some() {
        let rect = rect_from_mapping(raw).unwrap_or_default();
        return Some(LayoutNode::leaf(pane_id, rect));
    }
    None
}

/// Index parsed layout trees by Herdr `tab_id` (`layouts_by_tab_id`).
pub fn layouts_by_tab_id(raw: &Value) -> HashMap<String, LayoutNode> {
    let mut out: HashMap<String, LayoutNode> = HashMap::new();
    match raw {
        Value::Object(map) => {
            let nested = py_or(raw, &["layouts", "result"]);
            if let Some(nested) = nested {
                if nested.is_object() || nested.is_array() {
                    out.extend(layouts_by_tab_id(nested));
                    if !out.is_empty() {
                        return out;
                    }
                }
            }
            if raw.get("tab_id").is_some()
                && (raw.get("layout").is_some()
                    || raw.get("tree").is_some()
                    || raw.get("root").is_some())
            {
                let node = parse_layout(raw);
                let tab_id = raw
                    .get("tab_id")
                    .filter(|v| truthy(Some(v)))
                    .map(value_to_str)
                    .unwrap_or_default();
                if let (Some(node), false) = (node, tab_id.is_empty()) {
                    out.insert(tab_id, node);
                }
                return out;
            }
            const SKIP: [&str; 7] = [
                "type",
                "workspaces",
                "tabs",
                "panes",
                "agents",
                "focused",
                "result",
            ];
            for (key, value) in map {
                if SKIP.contains(&key.as_str()) {
                    continue;
                }
                if let Some(node) = parse_layout(value) {
                    out.insert(key.clone(), node);
                }
            }
            if let Some(Value::Array(_)) = raw.get("tabs") {
                out.extend(layouts_by_tab_id(&raw["tabs"]));
            }
            out
        }
        Value::Array(arr) => {
            for item in arr {
                if !item.is_object() {
                    continue;
                }
                if truthy(item.get("tab_id")) {
                    if let Some(node) = parse_layout(item) {
                        out.insert(value_to_str(&item["tab_id"]), node);
                    }
                } else {
                    out.extend(layouts_by_tab_id(item));
                }
            }
            out
        }
        _ => out,
    }
}

// --- BSP reconstruction ------------------------------------------------------

/// Reconstruct a binary split tree from pane rectangles (`tree_from_rects`).
pub fn tree_from_rects(items: &[(String, LayoutRect)]) -> Option<LayoutNode> {
    let cleaned: Vec<(String, LayoutRect)> = items
        .iter()
        .filter(|(pid, _)| !pid.is_empty())
        .cloned()
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    Some(bsp(cleaned))
}

fn bsp(items: Vec<(String, LayoutRect)>) -> LayoutNode {
    if items.len() == 1 {
        let (pane_id, rect) = items.into_iter().next().unwrap();
        return LayoutNode::leaf(Some(pane_id), rect);
    }
    let mut bound = items[0].1;
    for (_, rect) in &items[1..] {
        bound = bound.union(rect);
    }
    let partitioned = partition_axis(&items, "x").or_else(|| partition_axis(&items, "y"));
    match partitioned {
        None => {
            // Degraded: stack remaining panes top-to-bottom in id order.
            let mut items = items;
            items.sort_by(|a, b| a.0.cmp(&b.0));
            let kids = items
                .into_iter()
                .map(|(pid, rect)| LayoutNode::leaf(Some(pid), rect))
                .collect();
            LayoutNode {
                kind: "vertical".into(),
                pane_id: None,
                children: kids,
                rect: bound,
            }
        }
        Some((left, right, axis)) => {
            let kind = if axis == "x" {
                "horizontal"
            } else {
                "vertical"
            };
            LayoutNode {
                kind: kind.into(),
                pane_id: None,
                children: vec![bsp(left), bsp(right)],
                rect: bound,
            }
        }
    }
}

type Partition = (
    Vec<(String, LayoutRect)>,
    Vec<(String, LayoutRect)>,
    &'static str,
);

fn partition_axis(items: &[(String, LayoutRect)], axis: &'static str) -> Option<Partition> {
    let start = |r: &LayoutRect| if axis == "x" { r.x } else { r.y };
    let end = |r: &LayoutRect| {
        if axis == "x" {
            r.x + r.width
        } else {
            r.y + r.height
        }
    };
    let mut ordered = items.to_vec();
    ordered.sort_by(|a, b| (start(&a.1), &a.0).cmp(&(start(&b.1), &b.0)));
    for cut in 1..ordered.len() {
        let left = &ordered[..cut];
        let right = &ordered[cut..];
        let left_end = left.iter().map(|(_, r)| end(r)).max().unwrap();
        let right_start = right.iter().map(|(_, r)| start(r)).min().unwrap();
        // Allow a 1-cell separator like tmux pane borders.
        if left_end <= right_start + 1 {
            return Some((left.to_vec(), right.to_vec(), axis));
        }
    }
    None
}

// --- split plan --------------------------------------------------------------

/// Sequential splits to create every non-root leaf under `root`
/// (`split_specs`).
pub fn split_specs(root: &LayoutNode) -> Vec<SplitSpec> {
    let mut specs = Vec::new();
    walk_splits(root, &mut specs);
    specs
}

fn walk_splits(node: &LayoutNode, specs: &mut Vec<SplitSpec>) {
    if node.kind == "pane" || node.children.len() < 2 {
        for child in &node.children {
            walk_splits(child, specs);
        }
        return;
    }
    let direction = if node.kind == "horizontal" {
        "right"
    } else {
        "down"
    };
    let ratio = node.first_child_ratio();
    let first_leaves: Vec<Vec<String>> = node
        .children
        .iter()
        .map(|c| c.pane_ids_in_order())
        .collect();
    let mut anchor = first_leaves[0].first().cloned().unwrap_or_default();
    for (index, child) in node.children.iter().enumerate() {
        if index == 0 {
            walk_splits(child, specs);
            continue;
        }
        let leaves = &first_leaves[index];
        if leaves.is_empty() || anchor.is_empty() {
            walk_splits(child, specs);
            continue;
        }
        specs.push(SplitSpec {
            pane_id: leaves[0].clone(),
            split_from_pane_id: anchor.clone(),
            direction: direction.into(),
            ratio: if index == 1 {
                ratio
            } else {
                remaining_ratio(node, index)
            },
        });
        walk_splits(child, specs);
        anchor = leaves[0].clone();
    }
}

fn remaining_ratio(node: &LayoutNode, index: usize) -> Option<f64> {
    let rest = &node.children[index - 1..];
    if rest.len() < 2 {
        return None;
    }
    let mut bound = rest[0].rect;
    for child in &rest[1..] {
        bound = bound.union(&child.rect);
    }
    let axis = if node.kind == "horizontal" {
        "horizontal"
    } else {
        "vertical"
    };
    Some(bound.first_child_ratio(&rest[0].rect, axis))
}

/// Pull `(pane_id, rect)` pairs from raw pane dicts
/// (`pane_rects_from_objects`, dict branch).
pub fn pane_rects_from_dicts(panes: &[Value]) -> Vec<(String, LayoutRect)> {
    let mut out = Vec::new();
    for pane in panes {
        let pane_id = pane
            .get("pane_id")
            .filter(|v| truthy(Some(v)))
            .map(value_to_str)
            .unwrap_or_default();
        if pane_id.is_empty() {
            continue;
        }
        if let Some(rect) = rect_from_mapping(pane) {
            out.push((pane_id, rect));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_tmux_named_split() {
        let raw = json!({
            "width": 100, "height": 40,
            "horizontal": [
                {"pane_id": "a", "width": 60, "height": 40, "x": 0, "y": 0},
                {"pane_id": "b", "width": 40, "height": 40, "x": 60, "y": 0}
            ]
        });
        let node = parse_layout(&raw).unwrap();
        assert_eq!(node.kind, "horizontal");
        assert_eq!(node.pane_ids_in_order(), vec!["a", "b"]);
        let r = node.first_child_ratio().unwrap();
        assert!((r - 0.6).abs() < 1e-9);
    }

    #[test]
    fn parse_binary_split_first_second() {
        let raw = json!({
            "type": "split", "direction": "vertical",
            "first": {"pane_id": "top"}, "second": {"pane_id": "bot"}
        });
        let node = parse_layout(&raw).unwrap();
        assert_eq!(node.kind, "vertical");
        assert_eq!(node.pane_ids_in_order(), vec!["top", "bot"]);
    }

    #[test]
    fn split_specs_sequential() {
        let raw = json!({
            "horizontal": [
                {"pane_id": "a", "width": 50, "height": 40, "x": 0, "y": 0},
                {"pane_id": "b", "width": 50, "height": 40, "x": 50, "y": 0}
            ]
        });
        let node = parse_layout(&raw).unwrap();
        let specs = split_specs(&node);
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].pane_id, "b");
        assert_eq!(specs[0].split_from_pane_id, "a");
        assert_eq!(specs[0].direction, "right");
    }

    #[test]
    fn bsp_from_rects() {
        let items = vec![
            (
                "a".to_string(),
                LayoutRect {
                    x: 0,
                    y: 0,
                    width: 50,
                    height: 40,
                },
            ),
            (
                "b".to_string(),
                LayoutRect {
                    x: 50,
                    y: 0,
                    width: 50,
                    height: 40,
                },
            ),
        ];
        let node = tree_from_rects(&items).unwrap();
        assert_eq!(node.kind, "horizontal");
        assert_eq!(node.pane_ids_in_order(), vec!["a", "b"]);
    }

    #[test]
    fn structure_signature_stable() {
        let raw = json!({"horizontal": [{"pane_id": "a"}, {"pane_id": "b"}]});
        let node = parse_layout(&raw).unwrap();
        assert_eq!(node.structure_signature(), "h:p:a,p:b");
    }
}
