#!/usr/bin/env python3
"""Unit tests for pane I/O isolation + focus projection (tmux routeOutput)."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

_ROOT = Path(__file__).resolve().parents[1]
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from bridge.cmux_herdr_io import (
    PaneIORouter,
    TitleEscapeFilter,
    project_focus,
    route_input,
    route_output,
)


class TitleEscapeFilterTests(unittest.TestCase):
    def test_passthrough_without_escape(self) -> None:
        filt = TitleEscapeFilter()
        self.assertEqual(filt.filter(b"hello"), b"hello")

    def test_strips_screen_title_sequence(self) -> None:
        filt = TitleEscapeFilter()
        raw = b'echo "ej"\r\n\x1bkecho\x1b\\ej'
        self.assertEqual(filt.filter(raw), b'echo "ej"\r\nej')

    def test_split_across_chunks(self) -> None:
        filt = TitleEscapeFilter()
        self.assertEqual(filt.filter(b"ab\x1bkti"), b"ab")
        self.assertEqual(filt.filter(b"tle\x1b\\cd"), b"cd")

    def test_held_esc_emitted_when_not_title(self) -> None:
        filt = TitleEscapeFilter()
        self.assertEqual(filt.filter(b"\x1b[0m"), b"\x1b[0m")


class RouteOutputTests(unittest.TestCase):
    def test_unknown_pane_is_noop(self) -> None:
        router = PaneIORouter()
        self.assertIsNone(route_output(router, "w2:p1", b"hello"))
        self.assertEqual(router.buffers, {})

    def test_never_writes_across_panes(self) -> None:
        router = PaneIORouter()
        router.bind("w2:p1", "s1")
        router.bind("w2:p2", "s2")
        write = route_output(router, "w2:p1", b"alpha")
        self.assertIsNotNone(write)
        assert write is not None
        self.assertEqual(write.surface_id, "s1")
        self.assertEqual(router.buffer_for("w2:p1"), b"alpha")
        self.assertEqual(router.buffer_for("w2:p2"), b"")

    def test_empty_chunk_is_noop(self) -> None:
        router = PaneIORouter()
        router.bind("w2:p1", "s1")
        self.assertIsNone(route_output(router, "w2:p1", b""))
        self.assertEqual(router.buffer_for("w2:p1"), b"")

    def test_unbind_stops_writes(self) -> None:
        router = PaneIORouter()
        router.bind("w2:p1", "s1")
        router.unbind("w2:p1")
        self.assertIsNone(route_output(router, "w2:p1", b"late"))
        self.assertEqual(router.buffer_for("w2:p1"), b"")

    def test_title_bytes_never_reach_surface(self) -> None:
        router = PaneIORouter()
        router.bind("w2:p1", "s1")
        route_output(router, "w2:p1", b"hi\x1bktitle\x1b\\there")
        self.assertEqual(router.buffer_for("w2:p1"), b"hithere")

    def test_text_delta_incremental_then_redraw(self) -> None:
        router = PaneIORouter()
        router.bind("w2:p1", "s1")
        first = router.route_output_text("w2:p1", "hello")
        self.assertIsNotNone(first)
        assert first is not None
        self.assertTrue(first.full_redraw)
        second = router.route_output_text("w2:p1", "hello\nworld")
        self.assertIsNotNone(second)
        assert second is not None
        self.assertFalse(second.full_redraw)
        self.assertEqual(second.data, b"\nworld")
        self.assertEqual(router.buffer_for("w2:p1"), b"hello\nworld")
        third = router.route_output_text("w2:p1", "goodbye")
        self.assertIsNotNone(third)
        assert third is not None
        self.assertTrue(third.full_redraw)
        self.assertEqual(router.buffer_for("w2:p1"), b"goodbye")

    def test_unchanged_snapshot_is_noop(self) -> None:
        router = PaneIORouter()
        router.bind("w2:p1", "s1")
        router.route_output_text("w2:p1", "same")
        self.assertIsNone(router.route_output_text("w2:p1", "same"))


class RouteInputTests(unittest.TestCase):
    def test_unknown_pane_is_noop(self) -> None:
        router = PaneIORouter()
        self.assertIsNone(route_input(router, "w2:p1", b"x"))

    def test_only_bound_pane_receives_keys(self) -> None:
        router = PaneIORouter()
        router.bind("w2:p1", "s1")
        router.bind("w2:p2", "s2")
        send = route_input(router, "w2:p2", b"xy")
        self.assertIsNotNone(send)
        assert send is not None
        self.assertEqual(send.pane_id, "w2:p2")
        self.assertEqual(send.data, b"xy")
        self.assertNotIn("in:w2:p1", "".join(router.log))

    def test_focus_input_requires_active_pane(self) -> None:
        router = PaneIORouter()
        router.bind("w2:p1", "s1")
        self.assertIsNone(router.route_input_to_focus(b"x"))
        router.user_focus("w2:p1")
        send = router.route_input_to_focus(b"x")
        self.assertIsNotNone(send)
        assert send is not None
        self.assertEqual(send.pane_id, "w2:p1")


class FocusProjectionTests(unittest.TestCase):
    def test_provider_never_sends(self) -> None:
        router = PaneIORouter()
        router.bind("w2:p1", "s1")
        result = project_focus(router, "w2:p1", from_provider=True)
        self.assertFalse(result.send_to_provider)
        self.assertEqual(result.source, "provider")
        self.assertEqual(router.active_pane_id, "w2:p1")

    def test_user_sends_once_until_echo(self) -> None:
        router = PaneIORouter()
        router.bind("w2:p1", "s1")
        router.set_live_panes(["w2:p1", "w2:p2"])
        first = project_focus(router, "w2:p2", from_provider=False)
        self.assertTrue(first.send_to_provider)
        second = project_focus(router, "w2:p2", from_provider=False)
        self.assertFalse(second.send_to_provider)
        echo = project_focus(router, "w2:p2", from_provider=True)
        self.assertFalse(echo.send_to_provider)
        self.assertIsNone(router.pending_user_focus)
        third = project_focus(router, "w2:p2", from_provider=False)
        self.assertTrue(third.send_to_provider)

    def test_user_unknown_pane_is_noop(self) -> None:
        router = PaneIORouter()
        result = project_focus(router, "w2:missing", from_provider=False)
        self.assertIsNone(result.pane_id)
        self.assertFalse(result.send_to_provider)
        self.assertIsNone(router.active_pane_id)

    def test_provider_unknown_pane_still_projects(self) -> None:
        router = PaneIORouter()
        result = project_focus(router, "w2:pending", from_provider=True)
        self.assertEqual(result.pane_id, "w2:pending")
        self.assertFalse(result.send_to_provider)
        self.assertEqual(router.active_pane_id, "w2:pending")


class CwdRoutingTests(unittest.TestCase):
    def test_background_cd_does_not_hijack_tab(self) -> None:
        router = PaneIORouter()
        router.bind("w2:p1", "s1")
        router.bind("w2:p2", "s2")
        router.note_remote_active("w2:p1")
        background = router.route_cwd("w2:p2", "/tmp/other", "w2:t1")
        self.assertIsNotNone(background)
        assert background is not None
        self.assertFalse(background.apply_to_tab)
        active = router.route_cwd("w2:p1", "/tmp/here", "w2:t1")
        self.assertIsNotNone(active)
        assert active is not None
        self.assertTrue(active.apply_to_tab)

    def test_empty_path_is_noop(self) -> None:
        router = PaneIORouter()
        self.assertIsNone(router.route_cwd("w2:p1", "  ", "w2:t1"))


if __name__ == "__main__":
    unittest.main()
