#!/usr/bin/env python3
"""Tests for scripts/changelog_notes.py."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "changelog_notes.py"


def _load_module():
    spec = importlib.util.spec_from_file_location("changelog_notes", SCRIPT)
    assert spec is not None and spec.loader is not None
    mod = importlib.util.module_from_spec(spec)
    sys.modules["changelog_notes"] = mod
    spec.loader.exec_module(mod)
    return mod


notes = _load_module()


class ChangelogNotesTests(unittest.TestCase):
    """Tag → CHANGELOG section extraction."""

    SAMPLE = (
        "# Changelog\n\n"
        "## [Unreleased]\n\n"
        "## [0.3.4] - 2026-08-19\n\n"
        "### Added\n\n"
        "- MIT license\n\n"
        "## [0.3.3] - 2026-08-19\n\n"
        "old\n"
    )

    def test_tag_to_version(self) -> None:
        self.assertEqual(notes.tag_to_version("v0.3.4"), "0.3.4")
        self.assertEqual(notes.tag_to_version("0.3.4"), "0.3.4")
        with self.assertRaises(ValueError):
            notes.tag_to_version("latest")

    def test_section_stops_at_next_heading(self) -> None:
        body = notes.changelog_section(self.SAMPLE, "0.3.4")
        self.assertIn("MIT license", body)
        self.assertNotIn("0.3.3", body)
        self.assertTrue(body.startswith("## [0.3.4]"))

    def test_release_notes_install_is_plugin_manager_only(self) -> None:
        text = notes.release_notes("v0.3.4", self.SAMPLE)
        self.assertIn("cmux sidebar plugin install", text)
        self.assertIn("cmux sidebar plugin use cmux-herdr", text)
        self.assertIn("cmux sidebar plugin update cmux-herdr", text)
        self.assertIn("chmod +x", text)
        for demoted in (
            "cp sidebars/herdr",
            "~/.config/cmux/sidebars",
            "cmux sidebar open herdr",
            "cmux sidebar select herdr",
            "cmux sidebar validate herdr",
        ):
            self.assertNotIn(demoted, text)

    def test_missing_section_fails(self) -> None:
        with self.assertRaises(ValueError):
            notes.changelog_section(self.SAMPLE, "9.9.9")

    def test_real_changelog_has_current_version(self) -> None:
        version = (ROOT / "VERSION").read_text(encoding="utf-8").strip()
        text = notes.release_notes(f"v{version}")
        self.assertIn(f"## [{version}]", text)


if __name__ == "__main__":
    unittest.main()
