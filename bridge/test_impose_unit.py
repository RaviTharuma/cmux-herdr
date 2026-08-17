#!/usr/bin/env python3
"""Unit tests for the Bonsplit impose planner (tmux imposeDividerPlan twin)."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

_ROOT = Path(__file__).resolve().parents[1]
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from bridge.cmux_herdr_engine import HerdrWindow, apply_window, impose_after_apply
from bridge.cmux_herdr_impose import (
    DividerSplit,
    ImposeMetrics,
    ImposeSize,
    begin_divider_drag,
    binary_tree,
    divider_fraction,
    end_divider_drag,
    plan_impose,
    region_bounded_plan_parent,
    resolve_divider_hold,
    same_shape_and_pane_ids,
    specs_with_impose_fractions,
    tree_action,
)
from bridge.cmux_herdr_layout import LayoutNode, LayoutRect, parse_layout


HORIZONTAL_JSON = {
    "width": 200,
    "height": 50,
    "x": 0,
    "y": 0,
    "horizontal": [
        {"width": 100, "height": 50, "x": 0, "y": 0, "pane": "w2:p1"},
        {"width": 99, "height": 50, "x": 101, "y": 0, "pane": "w2:p2"},
    ],
}


def _leaf(pane_id: str, width: int = 80, height: int = 24) -> LayoutNode:
    return LayoutNode(
        kind="pane",
        pane_id=pane_id,
        rect=LayoutRect(0, 0, width, height),
    )


class DividerFractionTests(unittest.TestCase):
    def test_tmux_plus_one_divider_cell(self) -> None:
        # first=100, rest=99 → 100 / (100 + 99 + 1) = 0.5
        self.assertAlmostEqual(divider_fraction(100, [99]), 0.5)
        self.assertGreater(divider_fraction(100, [50]), 0.5)
        self.assertLess(divider_fraction(50, [100]), 0.5)

    def test_clamps_extreme_spans(self) -> None:
        self.assertEqual(divider_fraction(1, [1000]), 0.05)
        self.assertEqual(divider_fraction(1000, [1]), 0.95)


class RegionBoundTests(unittest.TestCase):
    def test_plan_parent_never_exceeds_region(self) -> None:
        parent = region_bounded_plan_parent(
            ImposeSize(900, 500),
            ImposeSize(800, 400),
        )
        assert parent is not None
        self.assertEqual(parent.width, 800)
        self.assertEqual(parent.height, 400)

    def test_render_used_when_inside_region(self) -> None:
        parent = region_bounded_plan_parent(
            ImposeSize(640, 320),
            ImposeSize(800, 400),
        )
        assert parent is not None
        self.assertEqual(parent.width, 640)
        self.assertEqual(parent.height, 320)

    def test_region_only_when_no_render(self) -> None:
        parent = region_bounded_plan_parent(None, ImposeSize(100, 50))
        assert parent is not None
        self.assertEqual(parent.width, 100)


class TreeActionTests(unittest.TestCase):
    def test_first_pass_rebuilds(self) -> None:
        node = parse_layout(HORIZONTAL_JSON)
        assert node is not None
        self.assertEqual(tree_action(None, node).kind, "rebuild")

    def test_geometry_only_keeps_tree(self) -> None:
        node = parse_layout(HORIZONTAL_JSON)
        assert node is not None
        wider = parse_layout(
            {
                "width": 400,
                "height": 50,
                "x": 0,
                "y": 0,
                "horizontal": [
                    {"width": 200, "height": 50, "x": 0, "y": 0, "pane": "w2:p1"},
                    {"width": 199, "height": 50, "x": 201, "y": 0, "pane": "w2:p2"},
                ],
            }
        )
        assert wider is not None
        self.assertTrue(same_shape_and_pane_ids(node, wider))
        self.assertEqual(tree_action(node, wider).kind, "keep")

    def test_leaf_expansion(self) -> None:
        old = _leaf("w2:p1", width=200, height=50)
        new = parse_layout(HORIZONTAL_JSON)
        assert new is not None
        action = tree_action(old, new)
        self.assertEqual(action.kind, "expand_leaf")
        assert action.expansion is not None
        self.assertEqual(action.expansion.existing_pane_id, "w2:p1")
        self.assertEqual(action.expansion.new_pane_id, "w2:p2")
        self.assertEqual(action.expansion.orientation, "horizontal")
        self.assertFalse(action.expansion.insert_first)

    def test_leaf_removal(self) -> None:
        old = parse_layout(HORIZONTAL_JSON)
        assert old is not None
        action = tree_action(old, _leaf("w2:p1"))
        self.assertEqual(action.kind, "remove_leaf")
        self.assertEqual(action.removed_pane_id, "w2:p2")


class BinaryTreeTests(unittest.TestCase):
    def test_right_associated_ternary(self) -> None:
        node = parse_layout(
            {
                "width": 300,
                "height": 24,
                "x": 0,
                "y": 0,
                "horizontal": [
                    {"width": 100, "height": 24, "x": 0, "y": 0, "pane": "a"},
                    {"width": 100, "height": 24, "x": 100, "y": 0, "pane": "b"},
                    {"width": 99, "height": 24, "x": 201, "y": 0, "pane": "c"},
                ],
            }
        )
        assert node is not None
        tree = binary_tree(node)
        self.assertIsInstance(tree, DividerSplit)
        assert isinstance(tree, DividerSplit)
        self.assertEqual(tree.orientation, "horizontal")
        self.assertIsInstance(tree.second, DividerSplit)
        assert isinstance(tree.second, DividerSplit)
        self.assertEqual(tree.second.first.pane_id, "b")  # type: ignore[union-attr]
        self.assertEqual(tree.second.second.pane_id, "c")  # type: ignore[union-attr]

    def test_metrics_produce_first_extent_inside_parent(self) -> None:
        node = parse_layout(HORIZONTAL_JSON)
        assert node is not None
        metrics = ImposeMetrics(cell_width=8, cell_height=16, divider_thickness=4)
        parent = ImposeSize(800, 400)
        tree = binary_tree(node, metrics=metrics, parent=parent)
        assert isinstance(tree, DividerSplit)
        self.assertIsNotNone(tree.first_extent)
        assert tree.first_extent is not None
        self.assertLessEqual(tree.first_extent, parent.width)
        self.assertGreater(tree.first_extent, 0)


class DragSessionTests(unittest.TestCase):
    def test_hold_clears_when_reply_assigns_target(self) -> None:
        hold = begin_divider_drag("split-1", "horizontal", assigned_cells=40)
        hold = resolve_divider_hold(hold, assigned_cells=50, split_still_exists=True)
        self.assertIsNotNone(hold)
        hold = resolve_divider_hold(hold, assigned_cells=40, split_still_exists=True)
        self.assertIsNone(hold)

    def test_hold_clears_when_split_vanishes(self) -> None:
        hold = begin_divider_drag("split-1", "horizontal", assigned_cells=40)
        self.assertIsNone(
            resolve_divider_hold(hold, assigned_cells=40, split_still_exists=False)
        )

    def test_drag_end_skips_noop_send(self) -> None:
        cells, should_send = end_divider_drag(
            dragged_extent=400,
            axis_span=800,
            total_cells=100,
            assigned_cells=50,
        )
        self.assertEqual(cells, 50)
        self.assertFalse(should_send)

    def test_drag_end_sends_when_cells_change(self) -> None:
        cells, should_send = end_divider_drag(
            dragged_extent=200,
            axis_span=800,
            total_cells=100,
            assigned_cells=50,
        )
        self.assertEqual(cells, 25)
        self.assertTrue(should_send)


class EngineBridgeTests(unittest.TestCase):
    def test_impose_after_first_apply_rebuilds(self) -> None:
        node = parse_layout(HORIZONTAL_JSON)
        assert node is not None
        window = HerdrWindow(
            tab_id="w2:t1",
            title="Build",
            order_index=0,
            layout=node,
            active_pane_id="w2:p2",
        )
        _, result = apply_window(window, None)
        plan = impose_after_apply(result)
        self.assertEqual(plan.tree_action.kind, "rebuild")
        self.assertEqual(plan.focus_pane_id, "w2:p2")
        self.assertEqual(len(plan.fractions), 1)
        self.assertAlmostEqual(plan.fractions[0], 0.5)

    def test_specs_overlay_tmux_fraction(self) -> None:
        node = parse_layout(HORIZONTAL_JSON)
        assert node is not None
        specs = specs_with_impose_fractions(node)
        self.assertEqual(len(specs), 1)
        self.assertAlmostEqual(specs[0].ratio or 0, 0.5)

    def test_plan_impose_records_hold_key(self) -> None:
        node = parse_layout(HORIZONTAL_JSON)
        assert node is not None
        hold = begin_divider_drag("split-a", "horizontal", 50)
        plan = plan_impose(node, hold=hold)
        self.assertEqual(plan.held_split_key, "split-a")


if __name__ == "__main__":
    unittest.main()
