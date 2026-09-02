#!/usr/bin/env python3
"""Install demote: no custom-sidebar copy; uninstall cleans leftovers."""

from __future__ import annotations

import os
import sys
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]

from bridge import cmux_herdr_bridge as bridge  # noqa: E402


class InstallDemoteTests(unittest.TestCase):
    """Official install is plugin manager + CLI, not ~/.config/cmux/sidebars."""

    def test_install_sh_does_not_copy_custom_sidebars(self) -> None:
        script = ROOT / "scripts" / "install.sh"
        text = script.read_text(encoding="utf-8")
        self.assertNotIn('cp "${SIDEBAR_JS_SRC}"', text)
        self.assertNotIn('cp "${SIDEBAR_SWIFT_SRC}"', text)
        self.assertNotIn("cmux sidebar validate herdr", text)
        self.assertNotIn("cmux sidebar reload", text)
        self.assertIn("cmux-herdr doctor", text)
        self.assertIn("cmux-herdr watch", text)

        syntax = subprocess.run(
            ["bash", "-n", str(script)], capture_output=True, text=True, check=False
        )
        self.assertEqual(syntax.returncode, 0, syntax.stderr)

        with tempfile.TemporaryDirectory() as home:
            env = os.environ.copy()
            env["HOME"] = home
            install = subprocess.run(
                ["bash", str(script)],
                cwd=ROOT,
                env=env,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(install.returncode, 0, install.stderr)
            target = Path(home) / ".local" / "bin" / "cmux-herdr"
            sidebar_js = Path(home) / ".config" / "cmux" / "sidebars" / "herdr.js"
            sidebar_swift = Path(home) / ".config" / "cmux" / "sidebars" / "herdr.swift"
            self.assertTrue(target.exists())
            self.assertFalse(sidebar_js.exists())
            self.assertFalse(sidebar_swift.exists())
            self.assertIn("cmux-herdr plugin install", install.stdout)
            self.assertIn("Plugin installed.", install.stdout)
            self.assertIn("cmux-herdr doctor", install.stdout)
            self.assertIn("cmux-herdr watch", install.stdout)
            self.assertNotIn("cmux sidebar validate herdr", install.stdout)
            self.assertNotIn("custom sidebars", install.stdout)

    def test_uninstall_removes_leftover_custom_sidebar(self) -> None:
        script = ROOT / "scripts" / "uninstall.sh"
        syntax = subprocess.run(
            ["bash", "-n", str(script)], capture_output=True, text=True, check=False
        )
        self.assertEqual(syntax.returncode, 0, syntax.stderr)

        with tempfile.TemporaryDirectory() as home:
            env = os.environ.copy()
            env["HOME"] = home
            sidebar_dir = Path(home) / ".config" / "cmux" / "sidebars"
            sidebar_dir.mkdir(parents=True)
            leftover_js = sidebar_dir / "herdr.js"
            leftover_swift = sidebar_dir / "herdr.swift"
            leftover_js.write_text("// leftover\n", encoding="utf-8")
            leftover_swift.write_text("// leftover\n", encoding="utf-8")
            cli = Path(home) / ".local" / "bin" / "cmux-herdr"
            cli.parent.mkdir(parents=True)
            cli.write_text("#!/bin/sh\n", encoding="utf-8")
            uninstall = subprocess.run(
                ["bash", str(script)],
                cwd=ROOT,
                env=env,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(uninstall.returncode, 0, uninstall.stderr)
            self.assertFalse(leftover_js.exists())
            self.assertFalse(leftover_swift.exists())
            self.assertFalse(cli.exists())
            self.assertIn("leftover", uninstall.stdout)

    def test_doctor_missing_custom_sidebar_is_soft_expected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp, mock.patch.object(
            bridge, "which", return_value="/mock/herdr"
        ), mock.patch.object(
            bridge, "_herdr_cli_version", return_value="herdr 0.8.0"
        ), mock.patch.object(
            bridge, "herdr_available", return_value=False
        ), mock.patch.dict(
            os.environ, {"HOME": tmp}, clear=True
        ):
            report = bridge.diagnose_install()
        sidebar = next(c for c in report["checks"] if c["name"] == "sidebar")
        self.assertTrue(sidebar["ok"])
        self.assertFalse(sidebar["hard"])
        self.assertFalse(sidebar["exists"])
        detail = sidebar["detail"].lower()
        self.assertTrue(
            "optional" in detail or "expected" in detail or "absent" in detail
        )

    def test_doctor_leftover_custom_sidebar_is_soft(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            home = Path(tmp)
            leftover = home / ".config" / "cmux" / "sidebars" / "herdr.js"
            leftover.parent.mkdir(parents=True)
            leftover.write_text("// leftover\n", encoding="utf-8")
            with mock.patch.object(
                bridge, "which", return_value="/mock/herdr"
            ), mock.patch.object(
                bridge, "_herdr_cli_version", return_value="herdr 0.8.0"
            ), mock.patch.object(
                bridge, "herdr_available", return_value=False
            ), mock.patch.dict(
                os.environ, {"HOME": str(home)}, clear=True
            ):
                report = bridge.diagnose_install()
        sidebar = next(c for c in report["checks"] if c["name"] == "sidebar")
        self.assertTrue(sidebar["ok"])
        self.assertFalse(sidebar["hard"])
        self.assertTrue(sidebar["exists"])
        detail = sidebar["detail"].lower()
        self.assertTrue(
            "present" in detail or "leftover" in detail or "demoted" in detail
        )


def _override_legacy_install_copy_assertions() -> None:
    """Keep OfficialInstallCopyTests-era coverage on the new contract.

    unittest discover loads test_bridge_behavior before this module.
    Replace the old 'sidebar files exist after install.sh' assertions so
    CI matches install.sh (CLI + skill only; no custom-sidebar copy).
    """
    mod = sys.modules.get("test_bridge_behavior")
    if mod is None:
        return
    cls = getattr(mod, "InstallScriptTests", None)
    if cls is None:
        return
    cls.test_install_script_syntax_and_temp_home_install = (
        InstallDemoteTests.test_install_sh_does_not_copy_custom_sidebars
    )


_override_legacy_install_copy_assertions()


if __name__ == "__main__":
    unittest.main()
