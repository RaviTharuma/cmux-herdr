#!/usr/bin/env python3
"""Unit tests for the host-apply verb list (tmux AppKit seam)."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

_ROOT = Path(__file__).resolve().parents[1]
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from bridge.cmux_herdr_engine import HerdrWindow, apply_window, impose_after_apply
from bridge.cmux_herdr_host import FakeBonsplitHost, host_actions
from bridge.cmux_herdr_impose import begin_divider_drag, plan_from_reconcile
from bridge.cmux_herdr_layout import parse_layout


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


def _window(layout, **kwargs):
    return HerdrWindow(
        tab_id=kwargs.get("tab_id", "w2:t1"),
        title=kwargs.get("title", "Build"),
        order_index=0,
        layout=layout,
        visible_layout=kwargs.get("visible"),
        zoomed=kwargs.get("zoomed", False),
        active_pane_id=kwargs.get("active"),
    )


class HostActionOrderTests(unittest.TestCase):
    def test_first_apply_creates_panels_before_rebuild(self) -> None:
        node = parse_layout(HORIZONTAL_JSON)
        assert node is not None
        _, result = apply_window(_window(node, active="w2:p2"), None)
        plan = impose_after_apply(result)
        actions = host_actions(result, plan)
        ops = [item.op for item in actions]
        self.assertEqual(ops[:3], ["create_panel", "create_panel", "rebuild_tree"])
        self.assertEqual(actions[0].pane_id, "w2:p1")
        self.assertEqual(actions[1].pane_id, "w2:p2")
        self.assertIn("impose_divider", ops)
        self.assertEqual(ops[-1], "focus")
        self.assertEqual(actions[-1].pane_id, "w2:p2")

    def test_fake_host_rejects_focus_without_panel(self) -> None:
        host = FakeBonsplitHost()
        from bridge.cmux_herdr_host import HostAction

        with self.assertRaises(ValueError):
            host.apply([HostAction(op="focus", pane_id="w2:p1")])

    def test_fake_host_applies_full_first_pass(self) -> None:
        node = parse_layout(HORIZONTAL_JSON)
        assert node is not None
        _, result = apply_window(_window(node, active="w2:p2"), None)
        plan = impose_after_apply(result)
        host = FakeBonsplitHost()
        host.apply(host_actions(result, plan))
        self.assertEqual(host.panels, {"w2:p1", "w2:p2"})
        self.assertEqual(host.last_tree_op, "rebuild_tree")
        self.assertEqual(host.focus, "w2:p2")
        self.assertEqual(len(host.imposed), 1)
        self.assertAlmostEqual(host.imposed[0].fraction or 0, 0.5)
        self.assertEqual(host.log[0], "create:w2:p1")
        self.assertEqual(host.log[1], "create:w2:p2")
        self.assertEqual(host.log[2], "rebuild_tree")

    def test_geometry_only_keeps_tree_and_reimposes(self) -> None:
        node = parse_layout(HORIZONTAL_JSON)
        assert node is not None
        state, first = apply_window(_window(node), None)
        host = FakeBonsplitHost()
        host.apply(host_actions(first, impose_after_apply(first)))
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
        _, second = apply_window(_window(wider), state)
        plan = impose_after_apply(second, previous_rendered=node)
        actions = host_actions(second, plan)
        self.assertEqual([item.op for item in actions if item.op.startswith("create")], [])
        self.assertIn("keep_tree", [item.op for item in actions])
        host.apply(actions)
        self.assertEqual(host.last_tree_op, "keep_tree")
        self.assertEqual(host.panels, {"w2:p1", "w2:p2"})

    def test_leaf_expansion_and_removal_round_trip(self) -> None:
        leaf = parse_layout({"width": 200, "height": 50, "x": 0, "y": 0, "pane": "w2:p1"})
        assert leaf is not None
        state, first = apply_window(_window(leaf), None)
        host = FakeBonsplitHost()
        host.apply(host_actions(first, impose_after_apply(first)))
        split = parse_layout(HORIZONTAL_JSON)
        assert split is not None
        state, expanded = apply_window(_window(split), state)
        plan = impose_after_apply(expanded, previous_rendered=leaf)
        actions = host_actions(expanded, plan)
        self.assertEqual(actions[0].op, "create_panel")
        self.assertEqual(actions[0].pane_id, "w2:p2")
        self.assertEqual(actions[1].op, "expand_leaf")
        self.assertEqual(actions[1].split_from_pane_id, "w2:p1")
        host.apply(actions)
        self.assertEqual(host.panels, {"w2:p1", "w2:p2"})
        self.assertEqual(host.last_tree_op, "expand_leaf")

        state, removed = apply_window(_window(leaf), state)
        plan = impose_after_apply(removed, previous_rendered=split)
        actions = host_actions(removed, plan)
        self.assertEqual(actions[0].op, "close_panel")
        self.assertEqual(actions[0].pane_id, "w2:p2")
        self.assertEqual(actions[1].op, "remove_leaf")
        host.apply(actions)
        self.assertEqual(host.panels, {"w2:p1"})
        self.assertEqual(host.last_tree_op, "remove_leaf")

    def test_held_split_is_skipped_during_impose(self) -> None:
        node = parse_layout(HORIZONTAL_JSON)
        assert node is not None
        _, result = apply_window(_window(node), None)
        hold = begin_divider_drag("s", "horizontal", 50)
        plan = plan_from_reconcile(result, hold=hold)
        actions = host_actions(result, plan)
        impose_ops = [item for item in actions if item.op == "impose_divider"]
        self.assertEqual(impose_ops, [])

    def test_zoom_keeps_hidden_panel(self) -> None:
        node = parse_layout(HORIZONTAL_JSON)
        assert node is not None
        state, first = apply_window(_window(node), None)
        host = FakeBonsplitHost()
        host.apply(host_actions(first, impose_after_apply(first)))
        zoomed_leaf = parse_layout(
            {"width": 200, "height": 50, "x": 0, "y": 0, "pane": "w2:p2"}
        )
        assert zoomed_leaf is not None
        _, zoomed = apply_window(
            _window(node, zoomed=True, visible=zoomed_leaf, active="w2:p2"),
            state,
        )
        host.apply(host_actions(zoomed, impose_after_apply(zoomed, previous_rendered=node)))
        self.assertEqual(host.panels, {"w2:p1", "w2:p2"})
        self.assertNotIn("close:w2:p1", host.log)


if __name__ == "__main__":
    unittest.main()
