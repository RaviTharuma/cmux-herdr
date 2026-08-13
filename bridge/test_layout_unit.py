#!/usr/bin/env python3
"""Unit tests for the Herdr layout planner (tmux RemoteTmuxLayoutNode analogue)."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

_ROOT = Path(__file__).resolve().parents[1]
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from bridge.cmux_herdr_layout import (
    LayoutNode,
    LayoutRect,
    layouts_by_tab_id,
    parse_layout,
    split_specs,
    tree_from_rects,
)


class ParseLayoutTests(unittest.TestCase):
    def test_tmux_json_horizontal_split(self):
        node = parse_layout(
            {
                "width": 200,
                "height": 50,
                "x": 0,
                "y": 0,
                "horizontal": [
                    {"width": 100, "height": 50, "x": 0, "y": 0, "pane": "w2:p1"},
                    {"width": 99, "height": 50, "x": 101, "y": 0, "pane": "w2:p2"},
                ],
            }
        )
        self.assertIsNotNone(node)
        assert node is not None
        self.assertEqual(node.kind, "horizontal")
        self.assertEqual(node.pane_ids_in_order, ["w2:p1", "w2:p2"])
        specs = split_specs(node)
        self.assertEqual(len(specs), 1)
        self.assertEqual(specs[0].pane_id, "w2:p2")
        self.assertEqual(specs[0].split_from_pane_id, "w2:p1")
        self.assertEqual(specs[0].direction, "right")
        self.assertAlmostEqual(specs[0].ratio or 0, 0.5, delta=0.05)

    def test_binary_split_vertical(self):
        node = parse_layout(
            {
                "type": "split",
                "direction": "down",
                "first": {"type": "pane", "pane_id": "a", "width": 80, "height": 12},
                "second": {"type": "pane", "pane_id": "b", "width": 80, "height": 12},
            }
        )
        self.assertIsNotNone(node)
        assert node is not None
        self.assertEqual(node.kind, "vertical")
        specs = split_specs(node)
        self.assertEqual(specs[0].direction, "down")
        self.assertEqual(specs[0].pane_id, "b")

    def test_nested_split_create_order(self):
        # H[ V[A, C], B ]  → DFS A, C, B
        node = parse_layout(
            {
                "kind": "hsplit",
                "children": [
                    {
                        "kind": "vsplit",
                        "children": [
                            {
                                "pane_id": "A",
                                "x": 0,
                                "y": 0,
                                "width": 40,
                                "height": 12,
                            },
                            {
                                "pane_id": "C",
                                "x": 0,
                                "y": 13,
                                "width": 40,
                                "height": 12,
                            },
                        ],
                    },
                    {
                        "pane_id": "B",
                        "x": 41,
                        "y": 0,
                        "width": 40,
                        "height": 25,
                    },
                ],
            }
        )
        assert node is not None
        self.assertEqual(node.pane_ids_in_order, ["A", "C", "B"])
        specs = split_specs(node)
        by_id = {s.pane_id: s for s in specs}
        self.assertEqual(by_id["C"].direction, "down")
        self.assertEqual(by_id["C"].split_from_pane_id, "A")
        self.assertEqual(by_id["B"].direction, "right")
        self.assertEqual(by_id["B"].split_from_pane_id, "A")

    def test_layouts_by_tab_id_from_map_and_list(self):
        mapped = layouts_by_tab_id(
            {"w2:t1": {"type": "pane", "pane_id": "w2:p1", "width": 80, "height": 24}}
        )
        self.assertEqual(mapped["w2:t1"].pane_ids_in_order, ["w2:p1"])
        listed = layouts_by_tab_id(
            [
                {
                    "tab_id": "w2:t2",
                    "layout": {"pane_id": "w2:p9", "width": 10, "height": 10},
                }
            ]
        )
        self.assertEqual(listed["w2:t2"].pane_ids_in_order, ["w2:p9"])


class TreeFromRectsTests(unittest.TestCase):
    def test_side_by_side_becomes_horizontal(self):
        node = tree_from_rects(
            [
                ("p1", LayoutRect(0, 0, 40, 24)),
                ("p2", LayoutRect(41, 0, 40, 24)),
            ]
        )
        assert node is not None
        self.assertEqual(node.kind, "horizontal")
        self.assertEqual(node.pane_ids_in_order, ["p1", "p2"])
        self.assertEqual(split_specs(node)[0].direction, "right")

    def test_stacked_becomes_vertical(self):
        node = tree_from_rects(
            [
                ("p1", LayoutRect(0, 0, 80, 12)),
                ("p2", LayoutRect(0, 13, 80, 12)),
            ]
        )
        assert node is not None
        self.assertEqual(node.kind, "vertical")
        self.assertEqual(split_specs(node)[0].direction, "down")

    def test_single_leaf(self):
        node = tree_from_rects([("p1", LayoutRect(0, 0, 80, 24))])
        assert node is not None
        self.assertEqual(node.kind, "pane")
        self.assertEqual(split_specs(node), [])

    def test_structure_signature_ignores_geometry(self):
        a = LayoutNode(
            kind="horizontal",
            children=[
                LayoutNode(kind="pane", pane_id="p1", rect=LayoutRect(0, 0, 10, 10)),
                LayoutNode(kind="pane", pane_id="p2", rect=LayoutRect(11, 0, 10, 10)),
            ],
        )
        b = LayoutNode(
            kind="horizontal",
            children=[
                LayoutNode(kind="pane", pane_id="p1", rect=LayoutRect(0, 0, 50, 20)),
                LayoutNode(kind="pane", pane_id="p2", rect=LayoutRect(51, 0, 50, 20)),
            ],
        )
        self.assertEqual(a.structure_signature(), b.structure_signature())


if __name__ == "__main__":
    unittest.main()
