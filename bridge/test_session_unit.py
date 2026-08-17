#!/usr/bin/env python3
"""Unit tests for session-tab host verbs (tmux rebuildTopology)."""

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
    impose_after_apply,
    reconcile_session,
    session_host_actions,
)
from bridge.cmux_herdr_host import FakeBonsplitHost, host_actions
from bridge.cmux_herdr_io import PaneIORouter
from bridge.cmux_herdr_layout import LayoutNode, LayoutRect, parse_layout
from bridge.cmux_herdr_session import FakeSessionHost, session_actions


def _leaf(pane_id: str) -> LayoutNode:
    return LayoutNode(kind="pane", pane_id=pane_id, rect=LayoutRect(0, 0, 80, 24))


def _window(
    pane_id: str,
    *,
    tab_id: str,
    order_index: int,
    title: str = "",
) -> HerdrWindow:
    return HerdrWindow(
        tab_id=tab_id,
        title=title or tab_id,
        order_index=order_index,
        layout=_leaf(pane_id),
        active_pane_id=pane_id,
    )


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


class SessionActionOrderTests(unittest.TestCase):
    def test_first_apply_creates_in_herdr_order_then_closes_defaults(self) -> None:
        windows = [
            _window("p-b", tab_id="t2", order_index=2, title="Two"),
            _window("p-a", tab_id="t1", order_index=1, title="One"),
        ]
        session = reconcile_session(windows, [])
        actions = session_host_actions(
            session,
            titles={"t1": "One", "t2": "Two"},
            defaults_open=True,
            focus_tab_id="t1",
        )
        ops = [item.op for item in actions]
        self.assertEqual(ops, ["create_tab", "create_tab", "close_default_tabs", "focus_tab"])
        self.assertEqual(actions[0].tab_id, "t1")
        self.assertEqual(actions[1].tab_id, "t2")
        self.assertEqual(actions[0].title, "One")
        host = FakeSessionHost()
        host.apply(actions)
        self.assertEqual(host.tabs, ["t1", "t2"])
        self.assertTrue(host.defaults_closed)
        self.assertEqual(host.focus, "t1")
        self.assertEqual(host.log[0], "create:t1")
        self.assertEqual(host.log[1], "create:t2")

    def test_close_gone_tabs_after_create(self) -> None:
        windows = [_window("p-a", tab_id="t1", order_index=0)]
        session = reconcile_session(windows, ["t1", "t-gone"])
        actions = session_actions(session)
        self.assertEqual([item.op for item in actions], ["close_tab"])
        self.assertEqual(actions[0].tab_id, "t-gone")
        host = FakeSessionHost()
        host.tabs = ["t1", "t-gone"]
        host.mirrors = {"t1": True, "t-gone": True}
        host.apply(actions)
        self.assertEqual(host.tabs, ["t1"])
        self.assertNotIn("t-gone", host.mirrors)

    def test_reorder_when_order_changed(self) -> None:
        windows = [
            _window("p-a", tab_id="t1", order_index=0),
            _window("p-b", tab_id="t2", order_index=1),
        ]
        session = reconcile_session(windows, ["t2", "t1"])
        self.assertTrue(session.order_changed)
        actions = session_actions(session)
        self.assertEqual([item.op for item in actions], ["reorder_tabs"])
        self.assertEqual(actions[0].ordered_tab_ids, ("t1", "t2"))
        host = FakeSessionHost()
        host.tabs = ["t2", "t1"]
        host.apply(actions)
        self.assertEqual(host.tabs, ["t1", "t2"])

    def test_no_reorder_for_single_tab(self) -> None:
        windows = [_window("p-a", tab_id="t1", order_index=0)]
        session = reconcile_session(windows, ["t1"])
        self.assertFalse(session.order_changed)
        self.assertEqual(session_actions(session), [])

    def test_rename_kept_tab(self) -> None:
        windows = [_window("p-a", tab_id="t1", order_index=0, title="Now")]
        session = reconcile_session(windows, ["t1"])
        actions = session_actions(
            session, titles={"t1": "Now"}, previous_titles={"t1": "Was"}
        )
        self.assertEqual([item.op for item in actions], ["rename_tab"])
        host = FakeSessionHost()
        host.tabs = ["t1"]
        host.titles = {"t1": "Was"}
        host.apply(actions)
        self.assertEqual(host.titles["t1"], "Now")

    def test_idempotent_second_pass(self) -> None:
        windows = [
            _window("p-a", tab_id="t1", order_index=0, title="One"),
            _window("p-b", tab_id="t2", order_index=1, title="Two"),
        ]
        first = reconcile_session(windows, [])
        host = FakeSessionHost()
        host.apply(
            session_actions(
                first, titles={"t1": "One", "t2": "Two"}, defaults_open=True
            )
        )
        second = reconcile_session(windows, host.tabs)
        actions = session_actions(
            second,
            titles={"t1": "One", "t2": "Two"},
            previous_titles={"t1": "One", "t2": "Two"},
            defaults_open=False,
        )
        self.assertEqual(actions, [])

    def test_focus_missing_tab_fails_closed(self) -> None:
        host = FakeSessionHost()
        from bridge.cmux_herdr_session import SessionAction

        with self.assertRaises(ValueError):
            host.apply([SessionAction(op="focus_tab", tab_id="missing")])

    def test_unknown_op_fails_closed(self) -> None:
        host = FakeSessionHost()
        from bridge.cmux_herdr_session import SessionAction

        with self.assertRaises(ValueError):
            host.apply([SessionAction(op="explode")])


class SessionPlusWindowIsolationTests(unittest.TestCase):
    def test_two_tabs_never_cross_io(self) -> None:
        """Full tmux-depth seam: session tabs + window apply + isolated I/O."""
        node = parse_layout(HORIZONTAL_JSON)
        assert node is not None
        windows = [
            HerdrWindow(
                tab_id="t1",
                title="Build",
                order_index=0,
                layout=node,
                active_pane_id="w2:p1",
            ),
            _window("w3:p1", tab_id="t2", order_index=1, title="Logs"),
        ]
        session = reconcile_session(windows, [])
        tabs = FakeSessionHost()
        tabs.apply(session_actions(session, titles={"t1": "Build", "t2": "Logs"}))
        self.assertEqual(tabs.tabs, ["t1", "t2"])

        hosts = {}
        router = PaneIORouter()
        for window in windows:
            _, result = apply_window(window, None)
            host = FakeBonsplitHost()
            host.apply(host_actions(result, impose_after_apply(result)))
            hosts[window.tab_id] = host
            for pane_id in result.created_pane_ids:
                router.bind(pane_id, f"s-{pane_id}")
            if result.focus_pane_id:
                router.note_remote_active(result.focus_pane_id)

        self.assertEqual(hosts["t1"].panels, {"w2:p1", "w2:p2"})
        self.assertEqual(hosts["t2"].panels, {"w3:p1"})

        router.route_output("w2:p1", b"build-out")
        router.route_output("w3:p1", b"log-out")
        self.assertEqual(router.buffer_for("w2:p1"), b"build-out")
        self.assertEqual(router.buffer_for("w2:p2"), b"")
        self.assertEqual(router.buffer_for("w3:p1"), b"log-out")

        send = router.route_input("w2:p2", b"typed")
        self.assertIsNotNone(send)
        assert send is not None
        self.assertEqual(send.pane_id, "w2:p2")
        self.assertNotIn("in:w3:p1", "".join(router.log))


if __name__ == "__main__":
    unittest.main()
