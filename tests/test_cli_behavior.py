#!/usr/bin/env python3
"""CLI tests using temporary mocked herdr/cmux executables."""

from __future__ import annotations

import json
import os
import stat
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / "bin" / "cmux-herdr"


def write_executable(path: Path, content: str) -> None:
    path.write_text(textwrap.dedent(content), encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR)


class CliBehaviorTests(unittest.TestCase):
    def run_cli(self, *args: str, path: str | None = None):
        env = os.environ.copy()
        if path is not None:
            env["PATH"] = path
        return subprocess.run(
            [os.fspath(CLI), *args],
            cwd=ROOT,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )

    def test_help_lists_core_commands(self):
        result = self.run_cli("--help")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("status", result.stdout)
        self.assertIn("sync", result.stdout)
        self.assertIn("json-dump", result.stdout)

    def test_unknown_command_is_argparse_error(self):
        result = self.run_cli("does-not-exist")
        self.assertEqual(result.returncode, 2)
        self.assertIn("invalid choice", result.stderr)

    def test_json_dump_with_mocked_herdr(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_bin = Path(tmp)
            write_executable(
                fake_bin / "herdr",
                r'''#!/usr/bin/env python3
import json
import sys
command = sys.argv[1:3]
if command == ["pane", "list"]:
    result = {"panes": [{"pane_id": "p1", "tab_id": "t1", "workspace_id": "w1", "agent": "pi", "agent_status": "idle"}]}
elif command == ["tab", "list"]:
    result = {"tabs": [{"tab_id": "t1", "workspace_id": "w1", "label": "Tests"}]}
elif command == ["workspace", "list"]:
    result = {"workspaces": [{"workspace_id": "w1", "label": "Demo"}]}
else:
    print("unexpected command", file=sys.stderr)
    raise SystemExit(9)
print(json.dumps({"result": result}))
''',
            )
            result = self.run_cli(
                "json-dump", path=f"{fake_bin}{os.pathsep}{os.environ.get('PATH', '')}"
            )

        self.assertEqual(result.returncode, 0, result.stderr)
        payload = json.loads(result.stdout)
        self.assertEqual(payload["panes"][0]["pane_id"], "p1")
        self.assertEqual(payload["tabs"][0]["label"], "Tests")
        self.assertEqual(payload["workspaces"][0]["workspace_id"], "w1")

    def test_sync_with_mocked_tools_clears_stale_status(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            fake_bin = root / "bin"
            fake_bin.mkdir()
            log = root / "cmux.log"
            write_executable(
                fake_bin / "herdr",
                r'''#!/usr/bin/env python3
import json
import sys
command = sys.argv[1:3]
if command == ["pane", "list"]:
    result = {"panes": [{"pane_id": "p1", "tab_id": "t1", "workspace_id": "w1", "agent": "pi", "agent_status": "working", "label": "Bot"}]}
elif command == ["tab", "list"]:
    result = {"tabs": []}
elif command == ["workspace", "list"]:
    result = {"workspaces": []}
else:
    raise SystemExit(8)
print(json.dumps({"result": result}))
''',
            )
            write_executable(
                fake_bin / "cmux",
                r'''#!/usr/bin/env python3
import os
import sys
with open(os.environ["FAKE_CMUX_LOG"], "a", encoding="utf-8") as handle:
    handle.write(" ".join(sys.argv[1:]) + "\n")
if sys.argv[1:2] == ["list-status"]:
    print("herdr:p1=current")
    print("herdr:stale=old")
    print("unrelated=keep")
''',
            )
            env_path = f"{fake_bin}{os.pathsep}{os.environ.get('PATH', '')}"
            env = os.environ.copy()
            env["PATH"] = env_path
            env["FAKE_CMUX_LOG"] = str(log)
            result = subprocess.run(
                [os.fspath(CLI), "sync", "--workspace", "workspace:1", "--no-progress"],
                cwd=ROOT,
                env=env,
                capture_output=True,
                text=True,
                check=False,
            )
            calls = log.read_text(encoding="utf-8")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("herdr sync: 1 panes", result.stdout)
        self.assertIn("set-status herdr:p1", calls)
        self.assertIn("clear-status herdr:stale", calls)
        self.assertNotIn("clear-status herdr:p1", calls)
        self.assertNotIn("clear-status unrelated", calls)

    def test_missing_herdr_reports_clean_error(self):
        # Keep the interpreter discoverable for the CLI's /usr/bin/env shebang,
        # while providing no directory that could contain herdr.
        result = self.run_cli("tree", path=os.fspath(Path(sys.executable).parent))
        self.assertEqual(result.returncode, 1)
        self.assertIn("herdr not available", result.stderr)
        self.assertNotIn("Traceback", result.stderr)


if __name__ == "__main__":
    unittest.main()
