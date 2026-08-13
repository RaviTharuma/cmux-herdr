#!/usr/bin/env python3
"""Unit tests for the Python twin of RemoteHerdrWindowMirror."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

_ROOT = Path(__file__).resolve().parents[1]
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from bridge.cmux_herdr_engine import (
    HerdrWindow,
    apply_window,
    bind_surface,
    client_grid,
    output_delta,
    reconcile_session,
    resize_cells,
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


def _window(
    layout: LayoutNode,
    *,
    tab_id: str = "w2:t1",
    title: str = "Build",
    order_index: int = 0,
    zoomed: bool = False,
    visible: LayoutNode | None = None,
    active: str | None = None,
) -> HerdrWindow:
    return HerdrWindow(
        tab_id=tab_id,
        title=title,
        order_index=order_index,
        layout=layout,
        visible_layout=visible,
        zoomed=zoomed,
        active_pane_id=active,
    )


class WindowMirrorTests(unittest.TestCase):
    def test_first_apply_creates_all_panes(self) -> None:
        node = parse_layout(HORIZONTAL_JSON)
        assert node is not None
        state, result = apply_window(_window(node, active="w2:p2"), None)
        self.assertEqual(result.created_pane_ids, ["w2:p1", "w2:p2"])
        self.assertEqual(result.closed_pane_ids, [])
        self.assertTrue(result.structure_changed)
        self.assertEqual(result.focus_pane_id, "w2:p2")
        self.assertEqual(len(result.split_specs), 1)
        self.assertEqual(result.split_specs[0].pane_id, "w2:p2")
        self.assertEqual(state.layout_structure_version, 0)
        bind_surface(state, "w2:p1", "s1")
        bind_surface(state, "w2:p2", "s2")
        self.assertEqual(state.surface_id_by_pane_id["w2:p1"], "s1")

    def test_gone_pane_closes_surface(self) -> None:
        node = parse_layout(HORIZONTAL_JSON)
        assert node is not None
        state, _ = apply_window(_window(node), None)
        bind_surface(state, "w2:p1", "s1")
        bind_surface(state, "w2:p2", "s2")
        single = _leaf("w2:p1")
        state2, result = apply_window(_window(single), state)
        self.assertEqual(result.closed_pane_ids, ["w2:p2"])
        self.assertEqual(result.kept_pane_ids, ["w2:p1"])
        self.assertNotIn("w2:p2", state2.surface_id_by_pane_id)
        self.assertEqual(state2.layout_structure_version, 1)

    def test_zoom_keeps_hidden_pane_ids(self) -> None:
        node = parse_layout(HORIZONTAL_JSON)
        assert node is not None
        state, _ = apply_window(_window(node), None)
        zoomed = _window(
            node,
            zoomed=True,
            visible=_leaf("w2:p2", width=200, height=50),
            active="w2:p2",
        )
        state2, result = apply_window(zoomed, state)
        self.assertEqual(result.created_pane_ids, [])
        self.assertEqual(result.closed_pane_ids, [])
        self.assertEqual(state2.pane_ids, ["w2:p1", "w2:p2"])
        self.assertEqual(result.rendered_layout.pane_ids_in_order, ["w2:p2"])
        self.assertFalse(result.structure_changed)
        self.assertEqual(state2.layout_structure_version, state.layout_structure_version)

    def test_geometry_only_does_not_bump_structure_version(self) -> None:
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
        state, _ = apply_window(_window(node), None)
        state2, result = apply_window(_window(wider), state)
        self.assertFalse(result.structure_changed)
        self.assertEqual(state2.layout_structure_version, state.layout_structure_version)
        self.assertEqual(result.created_pane_ids, [])
        self.assertEqual(result.closed_pane_ids, [])


class SessionMirrorTests(unittest.TestCase):
    def test_order_follows_tab_numbers(self) -> None:
        windows = [
            _window(_leaf("p-b"), tab_id="t2", order_index=2),
            _window(_leaf("p-a"), tab_id="t1", order_index=1),
        ]
        result = reconcile_session(windows, [])
        self.assertEqual(result.ordered_tab_ids, ["t1", "t2"])
        self.assertEqual(result.created_tab_ids, ["t1", "t2"])
        self.assertFalse(result.order_changed)

    def test_reorder_sets_order_changed(self) -> None:
        windows = [
            _window(_leaf("p-a"), tab_id="t1", order_index=0),
            _window(_leaf("p-b"), tab_id="t2", order_index=1),
        ]
        result = reconcile_session(windows, ["t2", "t1"])
        self.assertTrue(result.order_changed)
        self.assertEqual(result.kept_tab_ids, ["t1", "t2"])
        self.assertEqual(result.created_tab_ids, [])


class SizingAndOutputTests(unittest.TestCase):
    def test_client_grid_ignores_pane_frames(self) -> None:
        grid = client_grid(800, 400, 8, 16)
        self.assertEqual(grid, (100, 25))
        with_chrome = client_grid(800, 400, 8, 16, chrome_width=16, chrome_height=32)
        self.assertEqual(with_chrome, (98, 23))
        self.assertIsNone(client_grid(10, 10, 0, 16))

    def test_resize_cells_clamps(self) -> None:
        self.assertEqual(resize_cells(400, 800, 100), 50)
        self.assertEqual(resize_cells(0, 800, 100), 5)
        self.assertEqual(resize_cells(800, 800, 100), 95)

    def test_output_delta_incremental_and_redraw(self) -> None:
        chunk, full = output_delta(None, "hello")
        self.assertEqual(chunk, "hello")
        self.assertTrue(full)
        chunk, full = output_delta("hello", "hello\nworld")
        self.assertEqual(chunk, "\nworld")
        self.assertFalse(full)
        chunk, full = output_delta("hello", "hello")
        self.assertEqual(chunk, "")
        self.assertFalse(full)
        chunk, full = output_delta("hello", "goodbye")
        self.assertEqual(chunk, "goodbye")
        self.assertTrue(full)


if __name__ == "__main__":
    unittest.main()
