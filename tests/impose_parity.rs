//! Golden parity: Rust impose planning must match the Python bridge across a
//! captured battery of tree, metrics, drag, and reconcile cases.
use std::path::PathBuf;

use serde_json::{json, Value};

#[path = "../src/layout.rs"]
mod layout;
#[path = "../src/impose.rs"]
mod impose;

use impose::{
    DividerDragHold, DividerNode, ImposeMetrics, ImposePlan, ImposeSize, LeafExpansion,
    ReconcileResultLike, TreeAction,
};
use layout::{parse_layout, LayoutNode, SplitSpec};

fn golden() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/impose_golden.json");
    let raw = std::fs::read_to_string(&path).expect("read golden");
    serde_json::from_str(&raw).expect("parse golden")
}

fn f64_from(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| {
        value
            .get("__f64_bits")
            .and_then(Value::as_u64)
            .map(f64::from_bits)
    })
}

fn f64_json(value: f64) -> Value {
    if value.is_finite() {
        json!(value)
    } else {
        json!({"__f64_bits": value.to_bits()})
    }
}

fn optional_f64_json(value: Option<f64>) -> Value {
    value.map_or(Value::Null, f64_json)
}

fn size_json(value: Option<ImposeSize>) -> Value {
    value.map_or(Value::Null, |s| json!({
        "width": f64_json(s.width),
        "height": f64_json(s.height),
    }))
}

fn size_from(value: &Value) -> Option<ImposeSize> {
    if value.is_null() {
        return None;
    }
    Some(ImposeSize {
        width: f64_from(&value["width"]).unwrap(),
        height: f64_from(&value["height"]).unwrap(),
    })
}

fn metrics_from(value: &Value) -> Option<ImposeMetrics> {
    let object = value.as_object()?;
    let number = |key: &str| object.get(key).and_then(f64_from).unwrap_or(0.0);
    Some(ImposeMetrics {
        cell_width: number("cell_width"),
        cell_height: number("cell_height"),
        divider_thickness: number("divider_thickness"),
        tab_bar_height: number("tab_bar_height"),
        surface_pad_width: number("surface_pad_width"),
        surface_pad_height: number("surface_pad_height"),
        minimum_pane_extent: number("minimum_pane_extent"),
    })
}

fn hold_from(value: &Value) -> Option<DividerDragHold> {
    if value.is_null() {
        return None;
    }
    Some(DividerDragHold {
        split_key: value["split_key"].as_str().unwrap().to_string(),
        axis: value["axis"].as_str().unwrap().to_string(),
        target_cells: value["target_cells"].as_i64().unwrap(),
    })
}

fn hold_json(value: Option<&DividerDragHold>) -> Value {
    value.map_or(Value::Null, |hold| {
        json!({
            "split_key": hold.split_key,
            "axis": hold.axis,
            "target_cells": hold.target_cells,
        })
    })
}

fn expansion_json(value: Option<&LeafExpansion>) -> Value {
    value.map_or(Value::Null, |expansion| {
        json!({
            "existing_pane_id": expansion.existing_pane_id,
            "new_pane_id": expansion.new_pane_id,
            "orientation": expansion.orientation,
            "insert_first": expansion.insert_first,
            "fraction": f64_json(expansion.fraction),
        })
    })
}

fn action_json(action: &TreeAction) -> Value {
    json!({
        "kind": action.kind,
        "expansion": expansion_json(action.expansion.as_ref()),
        "removed_pane_id": action.removed_pane_id,
    })
}

fn divider_json(node: &DividerNode) -> Value {
    match node {
        DividerNode::Leaf(leaf) => json!({
            "pane_id": leaf.pane_id,
            "outer": size_json(leaf.outer),
        }),
        DividerNode::Split(split) => json!({
            "orientation": split.orientation,
            "fraction": f64_json(split.fraction),
            "first_extent": optional_f64_json(split.first_extent),
            "first": divider_json(&split.first),
            "second": divider_json(&split.second),
        }),
    }
}

fn spec_json(spec: &SplitSpec) -> Value {
    json!({
        "pane_id": spec.pane_id,
        "split_from_pane_id": spec.split_from_pane_id,
        "direction": spec.direction,
        "ratio": spec.ratio.map(f64_json).unwrap_or(Value::Null),
    })
}

fn plan_json(plan: &ImposePlan) -> Value {
    json!({
        "tree_action": action_json(&plan.tree_action),
        "divider_tree": divider_json(&plan.divider_tree),
        "focus_pane_id": plan.focus_pane_id,
        "title": plan.title,
        "held_split_key": plan.held_split_key,
        "fractions": plan.fractions.iter().copied().map(f64_json).collect::<Vec<_>>(),
    })
}

fn parsed(value: &Value) -> LayoutNode {
    parse_layout(value).expect("golden layout parses")
}

struct FakeReconcile {
    rendered_layout: LayoutNode,
    focus_pane_id: Option<String>,
}

impl ReconcileResultLike for FakeReconcile {
    fn rendered_layout(&self) -> &LayoutNode {
        &self.rendered_layout
    }

    fn focus_pane_id(&self) -> Option<&str> {
        self.focus_pane_id.as_deref()
    }
}

fn exact_floats(value: Value) -> Value {
    match value {
        Value::Number(number) if number.is_f64() => {
            json!({"__f64_bits": number.as_f64().unwrap().to_bits()})
        }
        Value::Array(items) => Value::Array(items.into_iter().map(exact_floats).collect()),
        Value::Object(items) => Value::Object(
            items
                .into_iter()
                .map(|(key, value)| (key, exact_floats(value)))
                .collect(),
        ),
        other => other,
    }
}

#[test]
fn impose_matches_python_golden() {
    let golden = golden();
    let cases = golden["cases"].as_array().unwrap();
    assert_eq!(golden["case_count"].as_u64().unwrap() as usize, cases.len());
    assert!(cases.len() >= 15, "golden battery must remain non-trivial");

    for case in cases {
        let inputs = &case["inputs"];
        let got = match case["op"].as_str().unwrap() {
            "clamp_ratio" => f64_json(impose::clamp_ratio(f64_from(&inputs["value"]).unwrap())),
            "divider_fraction" => {
                let rest: Vec<i64> = inputs["rest_spans"]
                    .as_array().unwrap().iter().map(|v| v.as_i64().unwrap()).collect();
                f64_json(impose::divider_fraction(inputs["first_span"].as_i64().unwrap(), &rest))
            }
            "region_bounded_plan_parent" => size_json(impose::region_bounded_plan_parent(
                size_from(&inputs["render"]), size_from(&inputs["region"]),
            )),
            "same_shape_and_pane_ids" => json!(impose::same_shape_and_pane_ids(
                &parsed(&inputs["lhs"]), &parsed(&inputs["rhs"]),
            )),
            "leaf_expansion" => expansion_json(impose::leaf_expansion(
                &parsed(&inputs["old"]),
                &parsed(&inputs["new"]),
                inputs["added_pane_id"].as_str().unwrap(),
            ).as_ref()),
            "tree_action" => {
                let previous = (!inputs["previous"].is_null()).then(|| parsed(&inputs["previous"]));
                action_json(&impose::tree_action(previous.as_ref(), &parsed(&inputs["rendered"])))
            }
            "binary_tree" => divider_json(&impose::binary_tree(
                &parsed(&inputs["node"]),
                metrics_from(&inputs["metrics"]).as_ref(),
                size_from(&inputs["parent"]),
            )),
            "collect_fractions" => {
                let tree = impose::binary_tree(&parsed(&inputs["node"]), None, None);
                Value::Array(impose::collect_fractions(&tree).into_iter().map(f64_json).collect())
            }
            "begin_divider_drag" => hold_json(Some(&impose::begin_divider_drag(
                inputs["split_key"].as_str().unwrap(),
                inputs["axis"].as_str().unwrap(),
                inputs["assigned_cells"].as_i64().unwrap(),
            ))),
            "resolve_divider_hold" => {
                let hold = hold_from(&inputs["hold"]);
                let assigned = inputs["assigned_cells"].as_i64();
                hold_json(impose::resolve_divider_hold(
                    hold, assigned, inputs["split_still_exists"].as_bool().unwrap(),
                ).as_ref())
            }
            "end_divider_drag" => {
                let (cells, should_send) = impose::end_divider_drag(
                    f64_from(&inputs["dragged_extent"]).unwrap(),
                    f64_from(&inputs["axis_span"]).unwrap(),
                    inputs["total_cells"].as_i64().unwrap(),
                    inputs["assigned_cells"].as_i64().unwrap(),
                );
                json!([cells, should_send])
            }
            "plan_impose" => {
                let rendered = parsed(&inputs["rendered"]);
                let previous = (!inputs["previous"].is_null()).then(|| parsed(&inputs["previous"]));
                let metrics = metrics_from(&inputs["metrics"]);
                let hold = hold_from(&inputs["hold"]);
                plan_json(&impose::plan_impose(
                    &rendered,
                    previous.as_ref(),
                    inputs["focus_pane_id"].as_str(),
                    inputs["title"].as_str().unwrap(),
                    metrics.as_ref(),
                    size_from(&inputs["render_size"]),
                    size_from(&inputs["region_size"]),
                    hold.as_ref(),
                ))
            }
            "specs_with_impose_fractions" => json!(impose::specs_with_impose_fractions(
                &parsed(&inputs["node"]),
            ).iter().map(spec_json).collect::<Vec<_>>()),
            "plan_from_reconcile" => {
                let result = FakeReconcile {
                    rendered_layout: parsed(&inputs["rendered"]),
                    focus_pane_id: inputs["focus_pane_id"].as_str().map(str::to_string),
                };
                let previous = (!inputs["previous"].is_null()).then(|| parsed(&inputs["previous"]));
                let metrics = metrics_from(&inputs["metrics"]);
                let hold = hold_from(&inputs["hold"]);
                plan_json(&impose::plan_from_reconcile(
                    &result,
                    previous.as_ref(),
                    inputs["title"].as_str().unwrap(),
                    metrics.as_ref(),
                    size_from(&inputs["render_size"]),
                    size_from(&inputs["region_size"]),
                    hold.as_ref(),
                ))
            }
            op => panic!("unknown golden operation {op}"),
        };
        assert_eq!(
            exact_floats(got),
            case["output"],
            "case {}",
            case["name"]
        );
    }
}
