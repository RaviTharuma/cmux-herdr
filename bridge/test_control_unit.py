#!/usr/bin/env python3
"""Unit tests for cmux-tmux mutation depth mapped onto Herdr."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

_ROOT = Path(__file__).resolve().parents[1]
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from bridge.cmux_herdr_control import (
    FocusController,
    InputForwarder,
    PaneSeedQueue,
    ProviderInput,
    adjacent_pane,
    apply_session_title,
    close_intent,
    encode_named_key,
    pane_surface_entries,
    request_split,
    tab_activity,
)
from bridge.cmux_herdr_layout import parse_layout


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

CROSSED = {
    "width": 200,
    "height": 50,
    "x": 0,
    "y": 0,
    "vertical": [
        {
            "width": 200,
            "height": 24,
            "x": 0,
            "y": 0,
            "horizontal": [
                {"width": 100, "height": 24, "x": 0, "y": 0, "pane": "a"},
                {"width": 99, "height": 24, "x": 101, "y": 0, "pane": "b"},
            ],
        },
        {"width": 200, "height": 25, "x": 0, "y": 25, "pane": "c"},
    ],
}


class NamedKeyTests(unittest.TestCase):
    def test_arrow_has_key_name_and_csi(self) -> None:
        item = encode_named_key("w2:p1", "Up")
        self.assertIsNotNone(item)
        assert item is not None
        self.assertEqual(item.kind, "key")
        self.assertEqual(item.key, "Up")
        self.assertEqual(item.csi, b"\x1b[A")

    def test_ctrl_up_uses_xterm_modifier(self) -> None:
        item = encode_named_key("w2:p1", "C-Up")
        self.assertIsNotNone(item)
        assert item is not None
        self.assertEqual(item.key, "C-Up")
        self.assertEqual(item.csi, b"\x1b[1;5A")

    def test_unknown_key_is_none(self) -> None:
        self.assertIsNone(encode_named_key("w2:p1", "NotAKey"))
        self.assertIsNone(encode_named_key("w2:p1", ""))


class InputForwarderTests(unittest.TestCase):
    def test_overflow_does_not_enqueue(self) -> None:
        fwd = InputForwarder(maximum_pending_bytes=4)
        first = fwd.enqueue(ProviderInput(pane_id="p1", kind="text", text="abcd"))
        self.assertEqual(first, "enqueued")
        second = fwd.enqueue(ProviderInput(pane_id="p1", kind="text", text="x"))
        self.assertEqual(second, "overflow")
        self.assertEqual(len(fwd.queue), 1)

    def test_deactivate_drops_queue(self) -> None:
        fwd = InputForwarder()
        fwd.enqueue(ProviderInput(pane_id="p1", kind="text", text="x"))
        fwd.deactivate()
        self.assertEqual(fwd.enqueue(ProviderInput(pane_id="p1", kind="text", text="y")), "inactive")
        self.assertEqual(fwd.drain(), [])


class FocusRollbackTests(unittest.TestCase):
    def test_reject_restores_previous(self) -> None:
        ctl = FocusController(live_pane_ids=["p1", "p2"], active_pane_id="p1")
        sent = ctl.user_select("p2")
        self.assertTrue(sent.send_to_provider)
        self.assertEqual(ctl.active_pane_id, "p2")
        rolled = ctl.command_rejected(sent.request_id or "")
        self.assertTrue(rolled.rolled_back)
        self.assertEqual(ctl.active_pane_id, "p1")

    def test_provider_confirm_clears_pending(self) -> None:
        ctl = FocusController(live_pane_ids=["p1", "p2"], active_pane_id="p1")
        ctl.user_select("p2")
        ctl.provider_confirms("p2")
        self.assertIsNone(ctl.pending)
        self.assertEqual(ctl.active_pane_id, "p2")

    def test_unknown_pane_is_noop(self) -> None:
        ctl = FocusController(live_pane_ids=["p1"])
        result = ctl.user_select("missing")
        self.assertFalse(result.send_to_provider)
        self.assertIsNone(ctl.active_pane_id)


class AdjacentPaneTests(unittest.TestCase):
    def test_horizontal_neighbors(self) -> None:
        node = parse_layout(HORIZONTAL)
        assert node is not None
        self.assertEqual(adjacent_pane(node, "w2:p1", "right"), "w2:p2")
        self.assertEqual(adjacent_pane(node, "w2:p2", "left"), "w2:p1")
        self.assertIsNone(adjacent_pane(node, "w2:p1", "left"))
        self.assertIsNone(adjacent_pane(node, "w2:p1", "up"))

    def test_crossed_tree(self) -> None:
        node = parse_layout(CROSSED)
        assert node is not None
        self.assertEqual(adjacent_pane(node, "a", "right"), "b")
        self.assertEqual(adjacent_pane(node, "a", "down"), "c")
        self.assertEqual(adjacent_pane(node, "c", "up"), "a")


class SeedQueueTests(unittest.TestCase):
    def test_holds_until_grid_matches(self) -> None:
        q = PaneSeedQueue()
        self.assertEqual(q.queue("p1", b"seed", target_grid=(80, 24)), "queued")
        self.assertIsNone(q.note_ready("p1", 40, 12))
        self.assertEqual(q.note_ready("p1", 80, 24), b"seed")
        self.assertIsNone(q.note_ready("p1", 80, 24))

    def test_overflow_defers_full(self) -> None:
        q = PaneSeedQueue(maximum_bytes=4)
        self.assertEqual(q.queue("p1", b"12345"), "overflow")
        self.assertIn("p1", q.deferred_full)
        self.assertNotIn("p1", q.pending)


class ActivityAndCloseTests(unittest.TestCase):
    def test_working_agent_is_busy(self) -> None:
        activity = tab_activity(
            {"p1": "working", "p2": "idle"}, agents={"p1": "claude"}
        )
        self.assertTrue(activity.has_active_command)
        self.assertEqual(activity.active_command_name, "claude")
        self.assertTrue(activity.needs_close_confirmation)

    def test_idle_tab_is_quiet(self) -> None:
        activity = tab_activity({"p1": "idle"})
        self.assertFalse(activity.has_active_command)
        self.assertFalse(activity.needs_close_confirmation)

    def test_host_close_detaches(self) -> None:
        intent = close_intent("host_tab", pane_id="p1", agent_status="working")
        self.assertEqual(intent.action, "detach")

    def test_user_close_busy_confirms(self) -> None:
        intent = close_intent("user_pane", pane_id="p1", agent_status="working")
        self.assertEqual(intent.action, "confirm_then_close_pane")
        idle = close_intent("user_pane", pane_id="p1", agent_status="idle")
        self.assertEqual(idle.action, "close_pane")


class SessionTitleAndSplitTests(unittest.TestCase):
    def test_inbound_rename_strips_controls(self) -> None:
        title = apply_session_title("  Build\x1b[0m  ", current="old")
        self.assertEqual(title, "Build")
        self.assertIsNone(apply_session_title("Build", current="Build"))
        self.assertIsNone(
            apply_session_title("Build", propagate_to_provider=True)
        )

    def test_user_split(self) -> None:
        split = request_split("w2:p1", vertical=True, insert_first=True)
        self.assertIsNotNone(split)
        assert split is not None
        self.assertEqual(split.orientation, "vertical")
        self.assertTrue(split.focus_created)
        self.assertIsNone(request_split("", vertical=False))

    def test_pane_surface_entries_sorted(self) -> None:
        rows = pane_surface_entries(
            [("t2", "p2", "s2", True), ("t1", "p1", "s1", False)]
        )
        self.assertEqual(rows[0]["tab_id"], "t1")
        self.assertFalse(rows[0]["on_screen"])


if __name__ == "__main__":
    unittest.main()
