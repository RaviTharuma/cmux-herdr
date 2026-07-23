#!/usr/bin/env python3
"""Unit tests for pure bridge helpers (no herdr/cmux required)."""

from __future__ import annotations

import unittest

from cmux_herdr_bridge import (
    STATUS_PREFIX,
    Pane,
    map_status_to_style,
    status_value_for_pane,
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
