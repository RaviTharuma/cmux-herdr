#!/usr/bin/env python3
"""Plugin-manager contract: manifest, optional build, executable [run]."""

from __future__ import annotations

import os
import re
import shutil
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "cmux-plugin.toml"
SIDEBAR = ROOT / "bin" / "cmux-herdr-sidebar"
README = ROOT / "README.md"
NAME_RE = re.compile(r"^[a-z0-9_-]+$")


def _parse_simple_toml(text: str) -> dict:
    """Parse the tiny manifest subset this repo ships (stdlib only)."""
    tables: dict = {}
    current = None
    for raw in text.splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        if line.startswith("[") and line.endswith("]"):
            current = line[1:-1].strip()
            tables.setdefault(current, {})
            continue
        if current is None or "=" not in line:
            raise ValueError(f"invalid toml line: {raw}")
        key, value = [part.strip() for part in line.split("=", 1)]
        tables[current][key] = _parse_toml_value(value)
    return tables


def _parse_toml_value(value: str):
    if value.startswith("[") and value.endswith("]"):
        inner = value[1:-1].strip()
        if not inner:
            return []
        return [_parse_toml_value(part.strip()) for part in inner.split(",")]
    if len(value) >= 2 and value[0] == value[-1] and value[0] in {'"', "'"}:
        return value[1:-1]
    return value


def _validate_manifest(tables: dict) -> None:
    """Mirror manaflow-ai/cmux plugin_manager.rs validate_manifest."""
    plugin = tables.get("plugin") or {}
    run = tables.get("run") or {}
    name = plugin.get("name")
    if not isinstance(name, str) or not NAME_RE.fullmatch(name):
        raise ValueError("plugin name must match [a-z0-9-_]+")
    if plugin.get("kind") != "sidebar":
        raise ValueError('plugin.kind must be "sidebar"')
    command = run.get("command")
    if not command or not str(command[0]).strip():
        raise ValueError("run.command must not be empty")
    build = tables.get("build")
    if build is not None:
        build_cmd = build.get("command")
        if not build_cmd or not str(build_cmd[0]).strip():
            raise ValueError("build.command must not be empty when present")


class PluginManifestTests(unittest.TestCase):
    """Honest cmux-plugin.toml that the official manager will accept."""

    def setUp(self) -> None:
        self.tables = _parse_simple_toml(MANIFEST.read_text(encoding="utf-8"))

    def test_manifest_matches_plugin_manager_rules(self) -> None:
        _validate_manifest(self.tables)
        plugin = self.tables["plugin"]
        self.assertEqual(plugin["name"], "cmux-herdr")
        self.assertEqual(plugin["kind"], "sidebar")
        version = (ROOT / "VERSION").read_text(encoding="utf-8").strip()
        self.assertEqual(plugin["version"], version)
        self.assertTrue(plugin.get("description"))

    def test_run_command_is_repo_relative_python_wrapper(self) -> None:
        command = self.tables["run"]["command"]
        self.assertEqual(command[0], "bin/cmux-herdr-sidebar")
        self.assertTrue(SIDEBAR.is_file())
        self.assertTrue(
            os.access(SIDEBAR, os.X_OK),
            "plugin-manager verify_executable requires +x",
        )
        shebang = SIDEBAR.read_text(encoding="utf-8").splitlines()[0]
        self.assertTrue(shebang.startswith("#!/usr/bin/env python3"))

    def test_build_is_honest_chmod_not_cargo(self) -> None:
        build = self.tables.get("build")
        self.assertIsNotNone(build)
        command = build["command"]
        self.assertEqual(command[0], "chmod")
        joined = " ".join(command)
        self.assertNotIn("cargo", joined)
        self.assertNotIn("target/release", joined)
        self.assertIn("bin/cmux-herdr-sidebar", joined)


class PluginManagerInstallSimulationTests(unittest.TestCase):
    """Clone → validate → optional build → executable exists."""

    def test_clone_build_and_run_executable(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp) / "src.git"
            dest = Path(tmp) / "clone"
            subprocess.run(["git", "init", str(src)], check=True, capture_output=True)
            subprocess.run(
                ["git", "-C", str(src), "config", "user.email", "lab@example.test"],
                check=True,
                capture_output=True,
            )
            subprocess.run(
                ["git", "-C", str(src), "config", "user.name", "Lab"],
                check=True,
                capture_output=True,
            )
            for rel in (
                "cmux-plugin.toml",
                "bin/cmux-herdr-sidebar",
                "bridge/cmux_herdr_sidebar.py",
                "VERSION",
            ):
                target = src / rel
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(ROOT / rel, target)
            subprocess.run(["git", "-C", str(src), "add", "."], check=True, capture_output=True)
            subprocess.run(
                ["git", "-C", str(src), "commit", "-m", "plugin fixture"],
                check=True,
                capture_output=True,
            )
            subprocess.run(
                ["git", "clone", "--depth", "1", str(src), str(dest)],
                check=True,
                capture_output=True,
            )

            tables = _parse_simple_toml((dest / "cmux-plugin.toml").read_text(encoding="utf-8"))
            _validate_manifest(tables)
            build = tables.get("build")
            if build:
                completed = subprocess.run(
                    build["command"],
                    cwd=dest,
                    capture_output=True,
                    text=True,
                    check=False,
                )
                self.assertEqual(completed.returncode, 0, completed.stderr)

            run_rel = tables["run"]["command"][0]
            run_path = (dest / run_rel).resolve()
            self.assertTrue(run_path.is_file())
            mode = run_path.stat().st_mode
            self.assertTrue(mode & stat.S_IXUSR)
            help_run = subprocess.run(
                [str(run_path), "--help"],
                cwd=dest,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(help_run.returncode, 0, help_run.stderr)
            self.assertIn("cmux sidebar plugin use cmux-herdr", help_run.stdout)


class OfficialInstallCopyTests(unittest.TestCase):
    """README documents the plugin manager as the only product install path."""

    PLUGIN_MANAGER = (
        "cmux sidebar plugin install https://github.com/RaviTharuma/cmux-herdr.git",
        "cmux sidebar plugin use cmux-herdr",
        "cmux sidebar plugin update cmux-herdr",
        "cmux sidebar plugin remove cmux-herdr",
    )
    DEMOTED = (
        "cp sidebars/herdr.js ~/.config/cmux/sidebars/herdr.js",
        "cp sidebars/herdr.swift ~/.config/cmux/sidebars/herdr.swift",
        "mkdir -p ~/.config/cmux/sidebars",
        "cmux sidebar open herdr",
        "cmux sidebar select herdr",
        "cmux sidebar validate herdr",
    )

    def test_readme_install_is_plugin_manager_only(self) -> None:
        text = README.read_text(encoding="utf-8")
        install_section = text.split("## Install", 1)[1].split("## Features", 1)[0]
        for line in self.PLUGIN_MANAGER:
            self.assertIn(line, install_section)
        self.assertNotIn("./scripts/install.sh", install_section)
        self.assertNotIn("git clone --branch", install_section)
        hero = text.split("## Install", 1)[0]
        self.assertNotIn("docs/screenshot", hero)
        self.assertNotIn("<img src=\"docs/", hero)
        self.assertIn("github/v/release/RaviTharuma/cmux-herdr", text)

    def test_readme_never_advertises_legacy_sidebar_copy(self) -> None:
        text = README.read_text(encoding="utf-8")
        for line in self.DEMOTED:
            self.assertNotIn(
                line,
                text,
                f"README still advertises the demoted path {line!r}",
            )

    def test_readme_labels_legacy_sidebar_files_as_not_the_product(self) -> None:
        text = README.read_text(encoding="utf-8")
        legacy = text.split("### Legacy sidebar files", 1)[1].split("## ", 1)[0]
        self.assertIn("not the product", legacy)
        self.assertIn("sidebars/herdr.js", legacy)
        self.assertIn("sidebars/herdr.swift", legacy)

    def test_readme_features_lead_with_plugin_manager_install(self) -> None:
        text = README.read_text(encoding="utf-8")
        features = text.split("## Features", 1)[1].split("## ", 1)[0]
        self.assertIn("Plugin-manager install", features)
        self.assertNotIn("docs/screenshot", features)

    def test_readme_omits_generated_screenshots(self) -> None:
        text = README.read_text(encoding="utf-8")
        self.assertNotIn("docs/screenshot.png", text)
        self.assertNotIn("docs/screenshot-pills.png", text)
        self.assertNotIn("docs/screenshot-mirror.png", text)
        self.assertNotIn("<img src=\"docs/", text)
        for rel in (
            "docs/screenshot.png",
            "docs/screenshot-pills.png",
            "docs/screenshot-mirror.png",
        ):
            self.assertFalse((ROOT / rel).exists(), f"{rel} must not ship")


if __name__ == "__main__":
    unittest.main()
