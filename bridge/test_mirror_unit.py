#!/usr/bin/env python3
"""Unit tests for the userspace Herdr → cmux tab/pane mirror planner."""

from __future__ import annotations

import io
import os
import sys
import unittest
from pathlib import Path
from unittest import mock

_ROOT = Path(__file__).resolve().parents[1]
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from bridge.cmux_herdr_bridge import BridgeError, Pane, Snapshot, Tab
from bridge.cmux_herdr_mirror import (
    ATTACH_ENV,
    DesiredMirror,
    attach_pane_loop,
    desired_mirrors,
    format_mirror_plan,
    is_attach_process,
    mirror_key_for_pane,
    parse_cmux_json,
    plan_mirror,
    send_pane_text,
)


def _pane(pane_id: str, tab_id: str, **kwargs) -> Pane:
    kwargs.setdefault("workspace_id", "w2")
    return Pane(pane_id=pane_id, tab_id=tab_id, **kwargs)


def _snap(panes, tabs=None) -> Snapshot:
    return Snapshot(panes=list(panes), tabs=list(tabs or []), workspaces=[])


class DesiredMirrorTests(unittest.TestCase):
    def test_current_tab_only_includes_that_tab(self):
        snap = _snap(
            [
                _pane("w2:p1", "w2:t1", label="A", focused=True),
                _pane("w2:p2", "w2:t1", label="B"),
                _pane("w2:p9", "w2:t9", label="Other"),
            ],
            [Tab(tab_id="w2:t1", workspace_id="w2", label="Agents", number=1)],
        )
        desired = desired_mirrors(snap, scope="current-tab", current_tab_id="w2:t1")
        self.assertEqual([d.pane_id for d in desired], ["w2:p1", "w2:p2"])
        self.assertEqual(desired[0].role, "tab-root")
        self.assertEqual(desired[0].title, "Agents")
        self.assertEqual(desired[1].role, "split")
        self.assertEqual(desired[1].split_direction, "right")

    def test_all_includes_every_pane_grouped_by_tab_number(self):
        snap = _snap(
            [
                _pane("w2:p2", "w2:t2"),
                _pane("w2:p1", "w2:t1"),
            ],
            [
                Tab(tab_id="w2:t2", workspace_id="w2", label="Second", number=2),
                Tab(tab_id="w2:t1", workspace_id="w2", label="First", number=1),
            ],
        )
        desired = desired_mirrors(snap, scope="all")
        self.assertEqual([d.tab_id for d in desired], ["w2:t1", "w2:t2"])

    def test_current_tab_requires_tab_id(self):
        with mock.patch.dict(os.environ, {}, clear=True):
            with self.assertRaisesRegex(BridgeError, "HERDR_TAB_ID"):
                desired_mirrors(_snap([]), scope="current-tab")

    def test_workspace_scope_filters(self):
        snap = _snap(
            [
                _pane("w2:p1", "w2:t1", workspace_id="w2"),
                _pane("w3:p1", "w3:t1", workspace_id="w3"),
            ]
        )
        desired = desired_mirrors(snap, scope="workspace", current_workspace_id="w2")
        self.assertEqual([d.pane_id for d in desired], ["w2:p1"])


class PlanMirrorTests(unittest.TestCase):
    def test_empty_existing_creates_tab_then_split(self):
        desired = [
            DesiredMirror(
                pane_id="w2:p1",
                tab_id="w2:t1",
                workspace_id="w2",
                title="Agents",
                role="tab-root",
                split_direction="right",
            ),
            DesiredMirror(
                pane_id="w2:p2",
                tab_id="w2:t1",
                workspace_id="w2",
                title="Reviewer",
                role="split",
                split_direction="right",
            ),
        ]
        plan = plan_mirror(desired, {})
        self.assertEqual([a.op for a in plan.actions], ["create_tab", "create_split"])
        self.assertEqual(plan.creates[1].key, "herdr-mirror:w2:p2")

    def test_idempotent_when_live_surfaces_match(self):
        desired = [
            DesiredMirror(
                pane_id="w2:p1",
                tab_id="w2:t1",
                workspace_id="w2",
                title="Agents",
                role="tab-root",
                split_direction="right",
            )
        ]
        existing = {
            "w2:p1": {
                "cmux_surface_id": "surface-1",
                "title": "Agents",
                "role": "tab-root",
                "tab_id": "w2:t1",
            }
        }
        plan = plan_mirror(desired, existing, live_surface_ids={"surface-1"})
        self.assertEqual([a.op for a in plan.actions], ["keep"])
        again = plan_mirror(desired, existing, live_surface_ids={"surface-1"})
        self.assertEqual([a.op for a in again.actions], ["keep"])

    def test_rename_when_title_changes(self):
        desired = [
            DesiredMirror(
                pane_id="w2:p1",
                tab_id="w2:t1",
                workspace_id="w2",
                title="New name",
                role="tab-root",
                split_direction="right",
            )
        ]
        existing = {
            "w2:p1": {
                "cmux_surface_id": "surface-1",
                "title": "Old name",
                "role": "tab-root",
            }
        }
        plan = plan_mirror(desired, existing, live_surface_ids={"surface-1"})
        self.assertEqual(plan.actions[0].op, "rename")
        self.assertEqual(plan.actions[0].title, "New name")

    def test_recreate_when_mapped_surface_is_dead(self):
        desired = [
            DesiredMirror(
                pane_id="w2:p1",
                tab_id="w2:t1",
                workspace_id="w2",
                title="Agents",
                role="tab-root",
                split_direction="right",
            )
        ]
        existing = {"w2:p1": {"cmux_surface_id": "dead", "title": "Agents"}}
        plan = plan_mirror(desired, existing, live_surface_ids=set())
        self.assertEqual(plan.actions[0].op, "create_tab")

    def test_prune_only_when_requested(self):
        desired = [
            DesiredMirror(
                pane_id="w2:p1",
                tab_id="w2:t1",
                workspace_id="w2",
                title="Agents",
                role="tab-root",
                split_direction="right",
            )
        ]
        existing = {
            "w2:p1": {"cmux_surface_id": "s1", "title": "Agents", "role": "tab-root"},
            "w2:p-gone": {"cmux_surface_id": "s-old", "title": "Gone", "role": "split"},
        }
        without = plan_mirror(desired, existing, live_surface_ids={"s1", "s-old"})
        self.assertEqual([a.op for a in without.actions], ["keep"])
        with_prune = plan_mirror(
            desired, existing, live_surface_ids={"s1", "s-old"}, prune=True
        )
        ops = [a.op for a in with_prune.actions]
        self.assertIn("prune", ops)
        self.assertEqual(with_prune.prunes[0].pane_id, "w2:p-gone")


class ParseAndAttachTests(unittest.TestCase):
    def test_parse_cmux_json_skips_ok_prefix(self):
        payload = parse_cmux_json('OK\n{"surface_id": "abc"}\n')
        self.assertEqual(payload["surface_id"], "abc")

    def test_mirror_key(self):
        self.assertEqual(mirror_key_for_pane("w2:p34"), "herdr-mirror:w2:p34")

    def test_attach_env_blocks_nested_mirror(self):
        with mock.patch.dict(os.environ, {ATTACH_ENV: "w2:p1"}):
            self.assertTrue(is_attach_process())
        with mock.patch.dict(os.environ, {}, clear=True):
            self.assertFalse(is_attach_process())

    def test_attach_loop_redraws_on_change_and_stops(self):
        reads = ["hello", "hello", "world"]
        buf = io.StringIO()

        def read_once() -> str:
            return reads.pop(0) if reads else "world"

        with mock.patch.dict(os.environ, {}, clear=False):
            os.environ.pop(ATTACH_ENV, None)
            code = attach_pane_loop(
                "w2:p1",
                interval=0.0,
                send_input=False,
                stdout=buf,
                sleeper=lambda _s: None,
                max_iterations=3,
                read_once=read_once,
            )
            self.assertEqual(code, 0)
            self.assertEqual(os.environ.get(ATTACH_ENV), "w2:p1")
        self.assertIn("hello", buf.getvalue())
        self.assertIn("world", buf.getvalue())

    def test_format_plan_mentions_scope(self):
        text = format_mirror_plan(
            {
                "scope": "current-tab",
                "desired_count": 2,
                "workspace": "workspace:9",
                "plan": {
                    "created": ["w2:p1"],
                    "renamed": [],
                    "kept": ["w2:p2"],
                    "pruned": [],
                    "errors": [],
                    "dry_run": True,
                    "actions": [
                        {
                            "op": "create_tab",
                            "pane_id": "w2:p1",
                            "title": "Agents",
                            "reason": "missing",
                        }
                    ],
                },
            }
        )
        self.assertIn("DRY-RUN", text)
        self.assertIn("create_tab", text)
        self.assertIn("w2:p1", text)

    def test_send_pane_text_tries_flag_then_stdin(self):
        from bridge import cmux_herdr_mirror as mirror

        fail = mock.Mock(returncode=1, stderr="nope", stdout="")
        ok = mock.Mock(returncode=0, stderr="", stdout="")
        with mock.patch.object(mirror, "which", return_value="/mock/herdr"), mock.patch.object(
            mirror, "run_cmd", return_value=fail
        ), mock.patch.object(mirror.subprocess, "run", return_value=ok) as sp_run:
            send_pane_text("w2:p1", "x")
            sp_run.assert_called_once()
            self.assertEqual(sp_run.call_args.kwargs.get("input"), "x")


if __name__ == "__main__":
    unittest.main()
