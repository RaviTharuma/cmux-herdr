#!/usr/bin/env python3
"""Public-tree hygiene: license, version, and no personal session dumps."""

from __future__ import annotations

import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / "bin" / "cmux-herdr"


class OssHygieneTests(unittest.TestCase):
    """Guards that keep the public GitHub tree safe to clone."""

    def test_license_is_mit(self) -> None:
        text = (ROOT / "LICENSE").read_text(encoding="utf-8")
        self.assertIn("MIT License", text)
        self.assertIn("Copyright (c) 2026 Ravi Tharuma", text)

    def test_version_file_is_semver(self) -> None:
        version = (ROOT / "VERSION").read_text(encoding="utf-8").strip()
        self.assertRegex(version, r"^\d+\.\d+\.\d+$")

    def test_cli_version_matches_version_file(self) -> None:
        version = (ROOT / "VERSION").read_text(encoding="utf-8").strip()
        proc = subprocess.run(
            [str(CLI), "--version"],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertEqual(proc.stdout.strip(), f"cmux-herdr {version}")

    def test_live_env_snapshot_is_absent(self) -> None:
        self.assertFalse(
            (ROOT / "docs" / "live-env-snapshot.txt").exists(),
            "personal cmux/herdr dumps must not be in the tree",
        )

    def test_auto_squash_merge_workflow_is_absent(self) -> None:
        self.assertFalse(
            (ROOT / ".github" / "workflows" / "auto-squash-merge.yml").exists(),
            "branch-name auto-merge is unsafe on a public repository",
        )

    def test_community_files_exist(self) -> None:
        for rel in (
            "CODE_OF_CONDUCT.md",
            "CONTRIBUTING.md",
            "SECURITY.md",
            "README.md",
            ".github/workflows/ci.yml",
            ".github/PULL_REQUEST_TEMPLATE.md",
            "docs/ARCHITECTURE.md",
            "docs/de/README.md",
            "cmux-plugin.toml",
        ):
            path = ROOT / rel
            self.assertTrue(path.is_file(), f"missing {rel}")
            self.assertGreater(path.stat().st_size, 64)

    def test_readme_does_not_advertise_stale_v010_target(self) -> None:
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        self.assertNotIn("Current stopgap release target: **v0.1.0**", readme)
        self.assertIn("MIT", readme)

    def test_readme_presents_as_released_plugin(self) -> None:
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        self.assertIn("cmux-herdr", readme)
        self.assertIn("A cmux plugin for Herdr", readme)
        self.assertIn("## Features", readme)
        self.assertIn("## Install", readme)
        self.assertIn("cmux sidebar plugin install", readme)
        self.assertIn("## Quick start", readme)
        self.assertNotIn("user plugin, no upstream PR", readme)
        self.assertNotIn("## Two-path strategy", readme)
        self.assertNotIn("## Why", readme.split("## Features")[0])

    def test_german_overview_names_the_plugin(self) -> None:
        text = (ROOT / "docs" / "de" / "README.md").read_text(encoding="utf-8")
        self.assertIn("Ein cmux-Plugin für Herdr", text)
        self.assertIn("cmux-herdr", text)


if __name__ == "__main__":
    unittest.main()
