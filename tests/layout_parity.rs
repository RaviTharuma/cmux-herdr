//! Golden parity: Rust layout parser + split planner must match the Python
//! bridge across a captured battery.
//!
//! Golden committed at `tests/layout_golden.json`; regenerate from the Python
//! bridge if the layout module changes (both sides move together).
use std::path::PathBuf;

use serde_json::{json, Value};

#[path = "../src/layout.rs"]
mod layout;

use layout::{LayoutNode, LayoutRect};

fn golden() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/layout_golden.json");
    let raw = std::fs::read_to_string(&path).expect("read golden");
    serde_json::from_str(&raw).expect("parse golden")
}

/// Serialize a `LayoutNode` into the shape the Python generator emits.
fn node_json(n: Option<&LayoutNode>) -> Value {
    match n {
        None => Value::Null,
        Some(n) => json!({
            "kind": n.kind,
            "pane_id": n.pane_id,
            "rect": [n.rect.x, n.rect.y, n.rect.width, n.rect.height],
            "signature": n.structure_signature(),
            "pane_ids_in_order": n.pane_ids_in_order(),
            "first_child_ratio": n.first_child_ratio(),
            "children": n.children.iter().map(|c| node_json(Some(c))).collect::<Vec<_>>(),
        }),
    }
}

#[test]
fn parse_layout_matches_python() {
    let g = golden();
    for entry in g["parse"].as_array().unwrap() {
        let node = layout::parse_layout(&entry["raw"]);
        assert_eq!(node_json(node.as_ref()), entry["node"], "parse for {}", entry["raw"]);
    }
}

#[test]
fn rect_from_mapping_matches_python() {
    let g = golden();
    for entry in g["rect"].as_array().unwrap() {
        let rect = layout::rect_from_mapping(&entry["raw"]);
        let got = match rect {
            None => Value::Null,
            Some(r) => json!([r.x, r.y, r.width, r.height]),
        };
        assert_eq!(got, entry["rect"], "rect for {}", entry["raw"]);
    }
}

#[test]
fn pane_is_zoomed_matches_python() {
    let g = golden();
    for entry in g["zoom"].as_array().unwrap() {
        let got = layout::pane_is_zoomed(&entry["raw"]);
        assert_eq!(json!(got), entry["zoomed"], "zoom for {}", entry["raw"]);
    }
}

#[test]
fn split_specs_matches_python() {
    let g = golden();
    for entry in g["split_specs"].as_array().unwrap() {
        let node = layout::parse_layout(&entry["raw"]).unwrap();
        let specs: Vec<Value> = layout::split_specs(&node)
            .iter()
            .map(|s| {
                json!({
                    "pane_id": s.pane_id,
                    "split_from_pane_id": s.split_from_pane_id,
                    "direction": s.direction,
                    "ratio": s.ratio,
                })
            })
            .collect();
        assert_eq!(json!(specs), entry["specs"], "specs for {}", entry["raw"]);
    }
}

#[test]
fn tree_from_rects_matches_python() {
    let g = golden();
    for entry in g["bsp"].as_array().unwrap() {
        let items: Vec<(String, LayoutRect)> = entry["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|it| {
                let pid = it[0].as_str().unwrap().to_string();
                let r = &it[1];
                let rect = LayoutRect {
                    x: r[0].as_i64().unwrap(),
                    y: r[1].as_i64().unwrap(),
                    width: r[2].as_i64().unwrap(),
                    height: r[3].as_i64().unwrap(),
                };
                (pid, rect)
            })
            .collect();
        let node = layout::tree_from_rects(&items);
        assert_eq!(node_json(node.as_ref()), entry["node"], "bsp for {}", entry["items"]);
    }
}

#[test]
fn layouts_by_tab_id_matches_python() {
    let g = golden();
    for entry in g["by_tab"].as_array().unwrap() {
        let map = layout::layouts_by_tab_id(&entry["raw"]);
        let mut got = serde_json::Map::new();
        for (k, v) in &map {
            got.insert(k.clone(), node_json(Some(v)));
        }
        assert_eq!(Value::Object(got), entry["tabs"], "by_tab for {}", entry["raw"]);
    }
}

#[test]
fn pane_rects_from_dicts_matches_python() {
    let g = golden();
    // The generator feeds only dicts, exercising the dict branch.
    let cases = json!([
        {"pane_id": "a", "width": 10, "height": 5, "x": 0, "y": 0},
        {"pane_id": "b", "geometry": {"cols": 8, "rows": 4}},
        {"pane_id": "", "width": 5, "height": 5},
        {"width": 5, "height": 5}
    ]);
    let panes = cases.as_array().unwrap();
    let got: Vec<Value> = layout::pane_rects_from_dicts(panes)
        .iter()
        .map(|(pid, r)| json!([pid, [r.x, r.y, r.width, r.height]]))
        .collect();
    assert_eq!(json!(got), g["pane_rects"]);
}
