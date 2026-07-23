#!/usr/bin/env python3
"""Unit tests for pure bridge helpers (no herdr/cmux required)."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

_ROOT = Path(__file__).resolve().parents[1]
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from bridge.cmux_herdr_bridge import (

    STATUS_PREFIX,
    Pane,
    Snapshot,
    format_associations,
    map_status_to_style,
    status_value_for_pane,
    update_association_map,
    _load_association_map,
)


class MapStatusTests(unittest.TestCase):
    def test_working(self):
        icon, color, prio = map_status_to_style("working")
        self.assertEqual(icon, "hammer")
        self.assertTrue(color.startswith("#"))
        self.assertGreaterEqual(prio, 50)

    def test_unknown_default(self):
        icon, color, prio = map_status_to_style("nope")
        self.assertEqual(map_status_to_style(None)[0], "questionmark.circle")
        self.assertEqual(icon, "circle")

    def test_case_insensitive(self):
        self.assertEqual(map_status_to_style("WORKING")[0], "hammer")
        self.assertEqual(map_status_to_style("Done")[0], "checkmark.circle")


class PaneTests(unittest.TestCase):
    def test_status_key(self):
        p = Pane(pane_id="w2:p34", tab_id="w2:t17", workspace_id="w2")
        self.assertEqual(p.status_key, f"{STATUS_PREFIX}w2:p34")

    def test_display_name_prefers_label(self):
        p = Pane(
            pane_id="w2:p1",
            tab_id="w2:t1",
            workspace_id="w2",
            label="Orchestrator",
            terminal_title="long title",
        )
        self.assertEqual(p.display_name, "Orchestrator")

    def test_status_value_includes_agent(self):
        p = Pane(
            pane_id="w2:p1",
            tab_id="w2:t1",
            workspace_id="w2",
            agent="pi",
            agent_status="working",
            label="Bot",
        )
        val = status_value_for_pane(p)
        self.assertIn("pi/working", val)
        self.assertIn("Bot", val)


if __name__ == "__main__":
    unittest.main()


class AssociationMapTests(unittest.TestCase):
    def test_update_association_map_tracks_and_prunes(self):
        import os
        import tempfile
        from unittest import mock

        pane1 = Pane(
            pane_id="w2:p1",
            tab_id="w2:t1",
            workspace_id="w2",
            agent="pi",
            agent_status="working",
            agent_session_path="/tmp/session-a.jsonl",
            agent_session_kind="path",
            revision=3,
        )
        pane2 = Pane(
            pane_id="w2:p2",
            tab_id="w2:t1",
            workspace_id="w2",
            agent="pi",
            agent_status="idle",
        )
        with tempfile.TemporaryDirectory() as tmp, mock.patch.dict(
            os.environ,
            {
                "XDG_STATE_HOME": tmp,
                "HERDR_SOCKET_PATH": "/tmp/herdr.sock",
                "HERDR_WORKSPACE_ID": "w2",
                "CMUX_SURFACE_ID": "surface-1",
            },
            clear=False,
        ):
            first = update_association_map(
                Snapshot(panes=[pane1, pane2], tabs=[], workspaces=[]),
                cmux_workspace="workspace:7",
            )
            self.assertEqual(first["pane_count"], 2)
            self.assertEqual(first["pruned"], [])
            state = _load_association_map()
            self.assertEqual(state["cmux_workspace"], "workspace:7")
            self.assertIn("w2:p1", state["panes"])
            self.assertEqual(state["panes"]["w2:p1"]["status_key"], "herdr:w2:p1")
            self.assertEqual(
                state["panes"]["w2:p1"]["agent_session_path"],
                "/tmp/session-a.jsonl",
            )

            second = update_association_map(
                Snapshot(panes=[pane1], tabs=[], workspaces=[]),
                cmux_workspace="workspace:7",
            )
            self.assertEqual(second["pane_count"], 1)
            self.assertEqual(second["pruned"], ["w2:p2"])
            rendered = format_associations()
            self.assertIn("associations: 1 panes", rendered)
            self.assertIn("w2:p1", rendered)
            self.assertNotIn("w2:p2  ", rendered)

