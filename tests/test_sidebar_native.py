#!/usr/bin/env python3
"""Contract tests for the interpreted Herdr sidebar.

The sidebar cannot be mounted on this Linux VM (cmux is macOS). These tests
lock the native-UI rules: live cmux workspaces only, Ghostty/cmux theme
tokens, and no invented team roster.
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SIDEBAR = ROOT / "sidebars" / "herdr.swift"


def _sidebar_source() -> str:
    return SIDEBAR.read_text(encoding="utf-8")


class NativeSidebarContractTests(unittest.TestCase):
    """Keep the custom sidebar on live cmux data and host theming."""

    def setUp(self) -> None:
        self.source = _sidebar_source()

    def test_sidebar_file_exists_and_is_a_view_expression(self) -> None:
        self.assertTrue(SIDEBAR.is_file())
        self.assertGreater(len(self.source), 200)
        self.assertIsNone(re.search(r"^\s*struct\s+\w+", self.source, flags=re.M))
        self.assertNotIn("@State", self.source)
        self.assertIn("ScrollView", self.source)

    def test_binds_live_workspaces_not_hardcoded_rows(self) -> None:
        self.assertIn("workspaces", self.source)
        self.assertIn("workspace.select", self.source)
        self.assertIn("workspace.reorder", self.source)
        self.assertIn("Reorderable", self.source)
        self.assertIn("No live cmux workspaces", self.source)
        self.assertNotIn("Alice", self.source)
        self.assertNotIn("Bob", self.source)
        self.assertNotIn("Charlie", self.source)
        self.assertNotRegex(
            self.source,
            r'Text\("(Engineering|Design|Marketing|Team Members)"\)',
        )

    def test_does_not_invent_a_team_section(self) -> None:
        lowered = self.source.lower()
        self.assertNotIn("team roster", lowered)
        self.assertNotIn("teammates", lowered)
        self.assertNotRegex(self.source, r'Text\("Team"\)')

    def test_uses_ghostty_cmux_theme_tokens_for_chrome(self) -> None:
        for token in ('"accent"', '"primary"', '"secondary"', '"tertiary"'):
            self.assertIn(token, self.source, f"missing theme token {token}")
        self.assertNotIn("#0A84FF", self.source)
        self.assertNotIn("#FF9F0A", self.source)
        self.assertNotIn("#8E8E93", self.source)
        self.assertNotIn("#1F2430", self.source)
        self.assertNotIn("#D9D7CE", self.source)

    def test_renders_live_statuses_agents_and_tabs(self) -> None:
        self.assertIn("w.statuses", self.source)
        self.assertIn("w.agents", self.source)
        self.assertIn("w.tabs", self.source)
        self.assertIn("w.color", self.source)
        self.assertIn("w.branch", self.source)
        self.assertIn("w.progress", self.source)
        self.assertIn("surface.focus", self.source)
        self.assertIn("clock.time", self.source)
        self.assertIn("selectedTitle", self.source)
        self.assertIn("herdr:", self.source)

    def test_caps_live_lists(self) -> None:
        self.assertIn("prefix(40)", self.source)
        self.assertIn("prefix(12)", self.source)
        self.assertIn("prefix(8)", self.source)
        self.assertIn("prefix(6)", self.source)


if __name__ == "__main__":
    unittest.main()
