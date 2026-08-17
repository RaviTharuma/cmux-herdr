#!/usr/bin/env python3
"""Live apply machine: makePanel, output, drag, focus, size, attach, restore."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

_ROOT = Path(__file__).resolve().parents[1]
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from bridge.cmux_herdr_engine import HerdrWindow
from bridge.cmux_herdr_layout import parse_layout
from bridge.cmux_herdr_lifecycle import DiscoveredSession
from bridge.cmux_herdr_live import LiveApplyHost, LiveWindowMirror, apply_live_windows

HORIZONTAL = {
    "width": 200,
    "height": 50,
    "x": 0,
    "y": 0,
    "horizontal": [
        {"width": 100, "height": 50, "x": 0, "y": 0, "pane": "w2:p1"},
        {"width": 99, "height": 50, "x": 101, "y": 0, "pane": "w2:p2"},
    ],
}


def _window(**kwargs) -> HerdrWindow:
    node = parse_layout(kwargs.get("layout") or HORIZONTAL)
    assert node is not None
    return HerdrWindow(
        tab_id=kwargs.get("tab_id", "w2:t1"),
        title=kwargs.get("title", "Build"),
        order_index=0,
        layout=node,
        visible_layout=kwargs.get("visible"),
        zoomed=kwargs.get("zoomed", False),
        active_pane_id=kwargs.get("active", "w2:p1"),
    )


class MakePanelAndOutputTests(unittest.TestCase):
    def test_make_panel_before_rebuild_and_output_stays_isolated(self) -> None:
        host = apply_live_windows([_window()])
        mirror = host.windows["w2:t1"]
        self.assertIn("w2:p1", mirror.surfaces)
        self.assertIn("w2:p2", mirror.surfaces)
        self.assertIn("create:w2:p1", mirror.bonsplit.log)
        self.assertIn("rebuild_tree", mirror.bonsplit.log)
        self.assertTrue(mirror.route_output("w2:p1", b"hello"))
        self.assertIn(b"hello", mirror.surfaces["w2:p1"].buffer)
        self.assertNotIn(b"hello", mirror.surfaces["w2:p2"].buffer)
        self.assertFalse(mirror.route_output("w2:unknown", b"nope"))

    def test_title_escape_is_stripped_before_ghostty(self) -> None:
        host = apply_live_windows([_window()])
        mirror = host.windows["w2:t1"]
        mirror.route_output("w2:p1", b"ab\x1bkTitle\x1b\\cd")
        self.assertEqual(mirror.surfaces["w2:p1"].buffer, b"abcd")

    def test_zoom_keeps_hidden_panel(self) -> None:
        host = LiveApplyHost()
        host.apply_session([_window()])
        zoomed = parse_layout({"pane": "w2:p1", "width": 200, "height": 50})
        host.apply_session([_window(zoomed=True, visible=zoomed, active="w2:p1")])
        mirror = host.windows["w2:t1"]
        self.assertIn("w2:p2", mirror.surfaces)
        self.assertTrue(mirror.surfaces["w2:p2"].live)


class InputAndFocusTests(unittest.TestCase):
    def test_named_key_and_text_only_reach_bound_pane(self) -> None:
        host = apply_live_windows([_window()])
        mirror = host.windows["w2:t1"]
        self.assertEqual(mirror.send_named_key("w2:p1", "C-Up"), "enqueued")
        self.assertEqual(mirror.send_named_key("w2:p1", "NotAKey"), "unknown")
        self.assertEqual(mirror.send_named_key("missing", "Up"), "inactive")
        self.assertEqual(mirror.send_text("w2:p2", "ls\n"), "enqueued")
        keys = [item.pane_id for item in mirror.input.drain()]
        self.assertEqual(keys, ["w2:p1", "w2:p2"])

    def test_provider_focus_does_not_steal_first_responder(self) -> None:
        host = apply_live_windows([_window()])
        mirror = host.windows["w2:t1"]
        mirror.user_focus("w2:p1")
        self.assertTrue(mirror.surfaces["w2:p1"].first_responder)
        mirror._apply_provider_focus("w2:p2")
        self.assertFalse(mirror.is_applying_focus)
        self.assertFalse(mirror.surfaces["w2:p2"].first_responder)
        self.assertTrue(mirror.surfaces["w2:p1"].first_responder)

    def test_adjacent_focus_sends_user_select(self) -> None:
        host = apply_live_windows([_window()])
        mirror = host.windows["w2:t1"]
        mirror.user_focus("w2:p1")
        neighbor = mirror.navigate_focus("right")
        self.assertEqual(neighbor, "w2:p2")
        self.assertTrue(mirror.surfaces["w2:p2"].first_responder)

    def test_user_split_maps_to_pane_split(self) -> None:
        host = apply_live_windows([_window()])
        split = host.windows["w2:t1"].user_split("w2:p1", "down")
        self.assertIsNotNone(split)
        assert split is not None
        self.assertEqual(split.orientation, "vertical")


class SizingAndDragTests(unittest.TestCase):
    def test_client_size_ignores_pane_frames(self) -> None:
        host = apply_live_windows([_window()])
        mirror = host.windows["w2:t1"]
        mirror.container_width = 160
        mirror.container_height = 48
        grid = mirror.update_client_size()
        self.assertEqual(grid, (20, 3))
        self.assertEqual(mirror.surfaces["w2:p1"].cols, 20)
        mirror.surfaces["w2:p1"].cols = 999
        again = mirror.update_client_size()
        self.assertEqual(again, (20, 3))
        self.assertEqual(mirror.last_client_grid, (20, 3))

    def test_hidden_tab_does_not_claim_size(self) -> None:
        host = apply_live_windows([_window()])
        mirror = host.windows["w2:t1"]
        mirror.is_visible_for_sizing = False
        self.assertIsNone(mirror.update_client_size())

    def test_divider_drag_sends_only_when_cells_change(self) -> None:
        host = apply_live_windows([_window()])
        mirror = host.windows["w2:t1"]
        mirror.begin_drag("s", "horizontal", 100)
        cells, send = mirror.end_drag(
            dragged_extent=50, axis_span=200, total_cells=200, assigned_cells=100
        )
        self.assertTrue(send)
        self.assertEqual(cells, 50)
        noop_cells, noop_send = mirror.end_drag(
            dragged_extent=100, axis_span=200, total_cells=200, assigned_cells=100
        )
        self.assertFalse(noop_send)
        self.assertEqual(noop_cells, 100)


class AttachDetachRestoreTests(unittest.TestCase):
    def test_attach_then_host_close_never_stops_server(self) -> None:
        host = LiveApplyHost()
        host.apply_session([_window()])
        attached = host.attach([DiscoveredSession("sess-1", "main")])
        self.assertTrue(attached["ok"])
        closed = host.detach()
        self.assertEqual(closed["outcome"], "detach")
        self.assertFalse(closed["server_stopped"])
        self.assertEqual(host.windows, {})
        self.assertTrue(host.windows.get("w2:t1") is None or True)

    def test_restore_reattaches_and_reseeds(self) -> None:
        host = LiveApplyHost()
        host.apply_session([_window()])
        host.attach([DiscoveredSession("sess-1", "main")])
        host.windows["w2:t1"].route_output("w2:p1", b"seed-me")
        host.windows.clear()
        result = host.restore(
            [DiscoveredSession("sess-1", "main")],
            [_window()],
        )
        self.assertTrue(result["ok"])
        self.assertEqual(result["mode"], "reattach")
        self.assertEqual(result["post_attach"], "reseed")
        self.assertIn("w2:t1", host.windows)

    def test_busy_close_and_activity(self) -> None:
        host = apply_live_windows([_window()])
        host.agent_statuses = {"w2:p1": "working"}
        host.agent_names = {"w2:p1": "coder"}
        intent = host.close_user_pane("w2:p1")
        self.assertEqual(intent.action, "confirm_then_close_pane")
        activity = host.activity()
        self.assertTrue(activity.has_active_command)
        self.assertEqual(activity.active_command_name, "coder")

    def test_observability_twins(self) -> None:
        host = apply_live_windows([_window()])
        surfaces = host.observe(
            "remote.herdr.pane_surfaces",
            {"socket": "/tmp/herdr.sock", "session": "main"},
        )
        self.assertTrue(surfaces["ok"])
        self.assertEqual(len(surfaces["panes"]), 2)
        grids = host.observe(
            "remote.herdr.pane_grids",
            {"socket": "/tmp/herdr.sock", "session": "main"},
        )
        self.assertTrue(grids["windows"][0]["panes"][0]["has_panel"])
        disabled = LiveApplyHost(enabled=False).observe(
            "remote.herdr.state",
            {"socket": "/tmp/herdr.sock", "session": "main"},
        )
        self.assertEqual(disabled["code"], "disabled")


class SeedAndTeardownTests(unittest.TestCase):
    def test_seed_waits_for_grid_then_paints(self) -> None:
        mirror = LiveWindowMirror(tab_id="t", title="t")
        mirror.make_panel("p")
        mirror.surfaces["p"].resize_grid(40, 12)
        self.assertIsNone(mirror.seed_pane("p", b"hello", 80, 24))
        flushed = mirror.seed.note_ready("p", 80, 24)
        self.assertEqual(flushed, b"hello")

    def test_teardown_drops_input_and_first_responder(self) -> None:
        host = apply_live_windows([_window()])
        mirror = host.windows["w2:t1"]
        mirror.user_focus("w2:p1")
        mirror.send_text("w2:p1", "x")
        mirror.teardown()
        self.assertTrue(mirror.is_torn_down)
        self.assertEqual(mirror.send_text("w2:p1", "y"), "inactive")
        self.assertFalse(mirror.surfaces["w2:p1"].first_responder)


if __name__ == "__main__":
    unittest.main()
