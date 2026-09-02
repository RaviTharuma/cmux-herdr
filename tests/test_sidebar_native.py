#!/usr/bin/env python3
"""Contract tests for the native Herdr sidebar.

The sidebar cannot be mounted on this Linux VM (cmux is macOS). These tests
lock the product rules: Herdr is the name, cmux is the chrome, live workspaces
only, no iframe/bridge/CLI cheat-sheet.
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SIDEBAR_JS = ROOT / "sidebars" / "herdr.js"
SIDEBAR_SWIFT = ROOT / "sidebars" / "herdr.swift"

FORBIDDEN_UX = (
    "bridge",
    "iframe",
    "Dual hierarchy",
    "dual hierarchy",
    "Status pills update after",
    "cmux-herdr sync",
    "cmux-herdr watch",
    "cmux-herdr status",
    "cmux-herdr tree",
    "cmux-herdr mirror",
    "cmux-herdr agents",
    "Inner agent mux",
    "you are looking at",
    "boxed-in",
)


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


class NativeSidebarContractTests(unittest.TestCase):
    """Keep both sidebar runtimes on native cmux chrome with Herdr named."""

    def setUp(self) -> None:
        self.js = _read(SIDEBAR_JS)
        self.swift = _read(SIDEBAR_SWIFT)

    def test_js_is_the_product_sidebar_and_swift_is_fallback(self) -> None:
        self.assertTrue(SIDEBAR_JS.is_file())
        self.assertTrue(SIDEBAR_SWIFT.is_file())
        self.assertGreater(len(self.js), 200)
        self.assertGreater(len(self.swift), 200)
        self.assertIn("wins over herdr.swift", self.js)
        self.assertIsNone(re.search(r"^\s*struct\s+\w+", self.swift, flags=re.M))
        self.assertNotIn("@State", self.swift)
        self.assertIn("ScrollView", self.swift)
        self.assertIn("sidebar(", self.js)

    def test_keeps_herdr_name_visible(self) -> None:
        self.assertRegex(self.js, r'Text\("Herdr"\)')
        self.assertRegex(self.swift, r'Text\("Herdr"\)')

    def test_does_not_advertise_cli_or_iframe_bridge_ux(self) -> None:
        for source in (self.js, self.swift):
            labels = re.findall(r'(?:Text|Button)\(\s*"([^"]+)"', source)
            joined = "\n".join(labels)
            for phrase in FORBIDDEN_UX:
                self.assertNotIn(
                    phrase.lower(),
                    joined.lower(),
                    f"sidebar still advertises {phrase!r} in UI copy",
                )
            self.assertNotRegex(source, r'Text\("cmux-herdr')
            self.assertNotIn('Text("Commands")', source)

    def test_binds_live_workspaces_not_hardcoded_rows(self) -> None:
        for source in (self.js, self.swift):
            self.assertIn("workspace.select", source)
            self.assertIn("workspace.reorder", source)
            self.assertIn("Reorderable", source)
            self.assertIn("contextMenu", source)
            self.assertIn("surface.focus", source)
            self.assertNotIn("Alice", source)
            self.assertNotIn("Bob", source)
            self.assertNotIn("Charlie", source)
            self.assertNotRegex(
                source,
                r'Text\("(Engineering|Design|Marketing|Team Members)"\)',
            )

    def test_does_not_invent_a_team_section(self) -> None:
        for source in (self.js, self.swift):
            lowered = source.lower()
            self.assertNotIn("team roster", lowered)
            self.assertNotIn("teammates", lowered)
            self.assertNotRegex(source, r'Text\("Team"\)')

    def test_uses_ghostty_cmux_theme_tokens_for_chrome(self) -> None:
        for source in (self.js, self.swift):
            for token in ('"accent"', '"primary"', '"secondary"', '"tertiary"'):
                self.assertIn(token, source, f"missing theme token {token}")
            self.assertNotIn("#0A84FF", source)
            self.assertNotIn("#FF9F0A", source)
            self.assertNotIn("#8E8E93", source)
            self.assertNotIn("#1F2430", source)
            self.assertNotIn("#D9D7CE", source)

    def test_status_chips_are_native_labels_not_herdr_key_dumps(self) -> None:
        for source in (self.js, self.swift):
            self.assertIn("statusLabel", source)
            self.assertIn("statuses", source)
            self.assertIn("tabs", source)
        self.assertNotIn('s.key.hasPrefix("herdr:")', self.swift)
        self.assertIn('indexOf(":")', self.js)
        self.assertIn('!s.key.contains(":")', self.swift)

    def test_caps_live_lists(self) -> None:
        for source in (self.js, self.swift):
            self.assertIn("40", source)
            self.assertIn("12", source)
            self.assertIn("6", source)


if __name__ == "__main__":
    unittest.main()
