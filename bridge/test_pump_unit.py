#!/usr/bin/env python3
"""Live pump: topology resync, isolated output, focus, agent_status."""

from __future__ import annotations

import unittest

from bridge.cmux_herdr_live import LiveApplyHost, apply_live_windows
from bridge.cmux_herdr_pump import (
    KIND_FOCUS,
    KIND_OUTPUT,
    KIND_STATUS,
    KIND_TOPOLOGY,
    LivePump,
    MemoryTransport,
    classify_event,
    event_type,
    unwrap_event,
)
from bridge.test_live_unit import _window


class EventShapeTests(unittest.TestCase):
    def test_unwraps_event_envelope(self) -> None:
        raw = {
            "event": "pane.focused",
            "data": {"type": "pane.focused", "pane_id": "w2:p1"},
        }
        self.assertEqual(event_type(raw), "pane.focused")
        self.assertEqual(unwrap_event(raw)["pane_id"], "w2:p1")
        self.assertEqual(classify_event(raw), KIND_FOCUS)

    def test_classifies_topology_and_output(self) -> None:
        self.assertEqual(
            classify_event({"type": "tab.created", "tab_id": "w2:t2"}),
            KIND_TOPOLOGY,
        )
        self.assertEqual(
            classify_event({"type": "pane.updated", "pane_id": "w2:p1"}),
            KIND_OUTPUT,
        )
        self.assertEqual(
            classify_event({"type": "pane.agent_status_changed", "pane_id": "w2:p1"}),
            KIND_STATUS,
        )


class PumpApplyTests(unittest.TestCase):
    def _host(self) -> LiveApplyHost:
        return apply_live_windows([_window()])

    def test_output_stays_on_one_pane(self) -> None:
        host = self._host()
        transport = MemoryTransport(
            reads={"w2:p1": "alpha", "w2:p2": "bravo"}
        )
        pump = LivePump(transport=transport)
        result = pump.handle_event(
            {"type": "pane.updated", "pane_id": "w2:p1"}, host
        )
        self.assertEqual(result.kind, KIND_OUTPUT)
        self.assertTrue(result.routed_output)
        self.assertIn(b"alpha", host.windows["w2:t1"].surfaces["w2:p1"].buffer)
        self.assertNotIn(b"alpha", host.windows["w2:t1"].surfaces["w2:p2"].buffer)
        self.assertEqual(transport.read_calls, ["w2:p1"])

    def test_unknown_pane_output_is_noop(self) -> None:
        host = self._host()
        transport = MemoryTransport(reads={"missing": "nope"})
        pump = LivePump(transport=transport)
        result = pump.handle_event(
            {"type": "pane.updated", "pane_id": "missing"}, host
        )
        self.assertFalse(result.routed_output)
        for surface in host.windows["w2:t1"].surfaces.values():
            self.assertNotIn(b"nope", surface.buffer)

    def test_topology_event_rebuilds_session(self) -> None:
        host = self._host()
        built = {"count": 0}

        def builder():
            built["count"] += 1
            return [_window()]

        pump = LivePump(transport=MemoryTransport(), windows_builder=builder)
        result = pump.handle_event({"type": "pane.created", "pane_id": "w2:p3"}, host)
        self.assertTrue(result.resync)
        self.assertEqual(result.kind, KIND_TOPOLOGY)
        self.assertEqual(built["count"], 1)
        self.assertEqual(host.windows["w2:t1"].surfaces["w2:p1"].first_responder, False)

    def test_focus_does_not_steal_first_responder(self) -> None:
        host = self._host()
        pump = LivePump(transport=MemoryTransport())
        result = pump.handle_event(
            {"event": "pane.focused", "data": {"pane_id": "w2:p2"}},
            host,
        )
        self.assertTrue(result.focused)
        mirror = host.windows["w2:t1"]
        self.assertEqual(mirror.focus.active_pane_id, "w2:p2")
        self.assertFalse(mirror.surfaces["w2:p2"].first_responder)
        self.assertFalse(mirror.is_applying_focus)

    def test_agent_status_feeds_busy_close_and_activity(self) -> None:
        host = self._host()
        transport = MemoryTransport(
            panes={"w2:p1": {"agent_status": "working", "cwd": "/repo"}}
        )
        pump = LivePump(transport=transport)
        result = pump.handle_event(
            {
                "type": "pane.agent_status_changed",
                "pane_id": "w2:p1",
                "agent_status": "working",
                "agent": "codex",
            },
            host,
        )
        self.assertTrue(result.status_updated)
        self.assertEqual(host.agent_statuses["w2:p1"], "working")
        self.assertEqual(host.agent_names["w2:p1"], "codex")
        intent = host.close_user_pane("w2:p1")
        self.assertEqual(intent.action, "confirm_then_close_pane")
        activity = host.activity()
        self.assertTrue(activity.needs_close_confirmation)

    def test_poll_paints_every_live_pane_in_isolation(self) -> None:
        host = self._host()
        transport = MemoryTransport(
            reads={"w2:p1": "one", "w2:p2": "two"}
        )
        pump = LivePump(transport=transport)
        result = pump.poll(host)
        self.assertTrue(result.routed_output)
        self.assertIn(b"one", host.windows["w2:t1"].surfaces["w2:p1"].buffer)
        self.assertIn(b"two", host.windows["w2:t1"].surfaces["w2:p2"].buffer)
        self.assertNotIn(b"two", host.windows["w2:t1"].surfaces["w2:p1"].buffer)
        self.assertCountEqual(transport.read_calls, ["w2:p1", "w2:p2"])

    def test_no_host_is_a_noop(self) -> None:
        pump = LivePump(transport=MemoryTransport(reads={"w2:p1": "x"}))
        result = pump.handle_event({"type": "pane.updated", "pane_id": "w2:p1"}, None)
        self.assertEqual(result.log, "no_host")

    def test_host_route_helpers_match_window_isolation(self) -> None:
        host = self._host()
        self.assertTrue(host.route_read_snapshot("w2:p1", "seed-a"))
        self.assertFalse(host.route_read_snapshot("missing", "seed-a"))
        self.assertIn(b"seed-a", host.windows["w2:t1"].surfaces["w2:p1"].buffer)
        self.assertNotIn(b"seed-a", host.windows["w2:t1"].surfaces["w2:p2"].buffer)
        self.assertCountEqual(host.live_pane_ids(), ["w2:p1", "w2:p2"])

    def test_tab_focus_without_pane_id(self) -> None:
        host = self._host()
        pump = LivePump(transport=MemoryTransport())
        result = pump.handle_event({"type": "tab.focused", "tab_id": "w2:t1"}, host)
        self.assertTrue(result.focused)
        self.assertEqual(result.log, "tab_focus")
        self.assertEqual(host.session_host.focus, "w2:t1")

    def test_workspace_focus_without_pane_id(self) -> None:
        host = self._host()
        pump = LivePump(transport=MemoryTransport())
        result = pump.handle_event(
            {"type": "workspace.focused", "workspace_id": "w2"}, host
        )
        self.assertTrue(result.focused)
        self.assertEqual(result.log, "workspace_focus")
        self.assertFalse(result.resync)
        self.assertEqual(host.focused_workspace_id, "w2")

    def test_flush_input_sends_queued_named_key(self) -> None:
        host = self._host()
        self.assertEqual(host.windows["w2:t1"].send_named_key("w2:p1", "Up"), "enqueued")
        transport = MemoryTransport()
        pump = LivePump(transport=transport)
        flushed = pump.flush_input(host)
        self.assertEqual(flushed, 1)
        self.assertEqual(transport.sent[0][0], "key")
        self.assertEqual(transport.sent[0][1], "w2:p1")
        self.assertIn("Up", transport.sent[0][2])

    def test_paint_read_seeds_then_applies_delta(self) -> None:
        host = self._host()
        self.assertTrue(host.paint_read("w2:p1", "hello"))
        self.assertTrue(host.paint_read("w2:p1", "hello!"))
        buf = host.windows["w2:t1"].surfaces["w2:p1"].buffer
        self.assertIn(b"hello", buf)
        self.assertIn(b"!", buf)
        self.assertNotIn(b"hello", host.windows["w2:t1"].surfaces["w2:p2"].buffer)


if __name__ == "__main__":
    unittest.main()
