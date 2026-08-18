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

FAKE_HERDR_FULL = r'''#!/usr/bin/env python3
import json
import sys

argv = sys.argv[1:]
if argv[:1] == ["--version"]:
    print("herdr 0.8.0")
    raise SystemExit(0)
if argv[:1] == ["status"]:
    print(json.dumps({"status": "ok"}))
    raise SystemExit(0)

command = argv[:2] if len(argv) >= 2 else argv
if command == ["pane", "list"]:
    result = {
        "panes": [
            {
                "pane_id": "p1",
                "tab_id": "t1",
                "workspace_id": "w1",
                "agent": "pi",
                "agent_status": "idle",
            }
        ]
    }
elif command == ["tab", "list"]:
    result = {"tabs": [{"tab_id": "t1", "workspace_id": "w1", "label": "Tests"}]}
elif command == ["workspace", "list"]:
    result = {"workspaces": [{"workspace_id": "w1", "label": "Demo"}]}
elif command == ["pane", "read"]:
    print("pane-output-line")
    raise SystemExit(0)
elif command == ["agent", "read"]:
    print("agent-output-line")
    raise SystemExit(0)
elif command == ["agent", "focus"]:
    print(json.dumps({"result": {"focused": argv[2] if len(argv) > 2 else ""}}))
    raise SystemExit(0)
elif command == ["workspace", "focus"]:
    print(json.dumps({"result": {"workspace_id": argv[2] if len(argv) > 2 else ""}}))
    raise SystemExit(0)
elif command == ["pane", "get"]:
    print(json.dumps({"result": {"pane_id": argv[2] if len(argv) > 2 else "p1"}}))
    raise SystemExit(0)
elif command == ["pane", "zoom"]:
    print("zoom should not be used as focus", file=sys.stderr)
    raise SystemExit(9)
elif command == ["pane", "send"] or command == ["pane", "send-keys"]:
    print(json.dumps({"result": {"sent": True}}))
    raise SystemExit(0)
elif command == ["tab", "create"]:
    result = {"tab_id": "t2", "label": "logs"}
elif command == ["pane", "close"]:
    result = {"closed": argv[2] if len(argv) > 2 else ""}
elif command == ["agent", "prompt"]:
    result = {"prompted": True}
elif command == ["agent", "wait"]:
    result = {"until": "done"}
else:
    print("unexpected command: " + " ".join(argv), file=sys.stderr)
    raise SystemExit(9)
print(json.dumps({"result": result}))
'''


def write_executable(path: Path, content: str) -> None:
    path.write_text(textwrap.dedent(content), encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR)


class CliBehaviorTests(unittest.TestCase):
    def run_cli(self, *args: str, path: str | None = None, env_extra: dict | None = None):
        env = os.environ.copy()
        if path is not None:
            env["PATH"] = path
        if env_extra:
            env.update(env_extra)
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
        self.assertIn("doctor", result.stdout)
        self.assertIn("read-pane", result.stdout)
        self.assertIn("read-agent", result.stdout)
        self.assertIn("focus-workspace", result.stdout)
        self.assertIn("focus-agent", result.stdout)
        self.assertIn("mirror", result.stdout)
        self.assertIn("attach-pane", result.stdout)
        self.assertIn("send-key", result.stdout)
        self.assertIn("observe", result.stdout)
        self.assertIn("attach", result.stdout)
        self.assertIn("detach", result.stdout)
        self.assertIn("restore", result.stdout)
        self.assertIn("lock-title", result.stdout)
        self.assertIn("unlock-title", result.stdout)
        self.assertIn("api", result.stdout)
        self.assertIn("new-tab", result.stdout)
        self.assertIn("close-pane", result.stdout)
        self.assertIn("zoom-pane", result.stdout)
        self.assertIn("send", result.stdout)
        self.assertIn("agent-prompt", result.stdout)
        self.assertIn("agent-wait", result.stdout)
        self.assertIn("layout", result.stdout)

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
if sys.argv[1:2] == ["status"]:
    print(json.dumps({"status": "ok"}))
    raise SystemExit(0)
elif command == ["pane", "list"]:
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
if sys.argv[1:2] == ["status"]:
    print(json.dumps({"status": "ok"}))
    raise SystemExit(0)
elif command == ["pane", "list"]:
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

    def test_doctor_fails_when_herdr_missing(self):
        result = self.run_cli(
            "doctor",
            path=os.fspath(Path(sys.executable).parent),
            env_extra={"HERDR_ENV": "", "CMUX_SURFACE_ID": "", "HERDR_SOCKET_PATH": ""},
        )
        self.assertEqual(result.returncode, 1)
        self.assertIn("herdr_cli", result.stdout)
        self.assertIn("FAIL", result.stdout)
        self.assertNotIn("Traceback", result.stderr)

    def test_doctor_ok_with_fake_herdr_and_fingerprint(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            fake_bin = root / "bin"
            fake_bin.mkdir()
            write_executable(fake_bin / "herdr", FAKE_HERDR_FULL)
            sock = root / "herdr.sock"
            sock.write_text("", encoding="utf-8")
            state = root / "state"
            env_extra = {
                "HOME": str(root),
                "XDG_STATE_HOME": str(state),
                "HERDR_ENV": "1",
                "HERDR_SOCKET_PATH": str(sock),
                "CMUX_SURFACE_ID": "surface-cli",
                "HERDR_WORKSPACE_ID": "w1",
            }
            result = self.run_cli(
                "doctor",
                "--json",
                path=f"{fake_bin}{os.pathsep}{os.environ.get('PATH', '')}",
                env_extra=env_extra,
            )
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        self.assertIn("[ok  ] herdr_cli", result.stdout)
        self.assertIn("host_fingerprint", result.stdout)
        # JSON blob is appended; ensure ok=true appears
        self.assertIn('"ok": true', result.stdout)

    def test_doctor_fails_nested_incomplete_fingerprint(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_bin = Path(tmp) / "bin"
            fake_bin.mkdir()
            write_executable(fake_bin / "herdr", FAKE_HERDR_FULL)
            result = self.run_cli(
                "doctor",
                path=f"{fake_bin}{os.pathsep}{os.environ.get('PATH', '')}",
                env_extra={
                    "HOME": tmp,
                    "XDG_STATE_HOME": str(Path(tmp) / "state"),
                    "HERDR_ENV": "1",
                    "HERDR_SOCKET_PATH": "",
                    "CMUX_SURFACE_ID": "",
                },
            )
        self.assertEqual(result.returncode, 1)
        self.assertIn("FAIL", result.stdout)
        self.assertIn("incomplete host fingerprint", result.stdout)

    def test_read_pane_and_read_agent_wrappers(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_bin = Path(tmp) / "bin"
            fake_bin.mkdir()
            write_executable(fake_bin / "herdr", FAKE_HERDR_FULL)
            path = f"{fake_bin}{os.pathsep}{os.environ.get('PATH', '')}"
            pane = self.run_cli(
                "read-pane",
                "p1",
                "--source",
                "recent",
                "--lines",
                "20",
                path=path,
            )
            agent = self.run_cli(
                "read-agent",
                "p1",
                "--source",
                "visible",
                path=path,
            )
        self.assertEqual(pane.returncode, 0, pane.stderr)
        self.assertIn("pane-output-line", pane.stdout)
        self.assertEqual(agent.returncode, 0, agent.stderr)
        self.assertIn("agent-output-line", agent.stdout)

    def test_focus_workspace_agent_and_pane(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_bin = Path(tmp) / "bin"
            fake_bin.mkdir()
            write_executable(fake_bin / "herdr", FAKE_HERDR_FULL)
            path = f"{fake_bin}{os.pathsep}{os.environ.get('PATH', '')}"
            ws = self.run_cli("focus-workspace", "w1", path=path)
            agent = self.run_cli("focus-agent", "p1", path=path)
            pane = self.run_cli("focus-pane", "p1", path=path)
        self.assertEqual(ws.returncode, 0, ws.stderr)
        self.assertIn("focused workspace w1", ws.stdout)
        self.assertEqual(agent.returncode, 0, agent.stderr)
        self.assertIn("focused agent p1", agent.stdout)
        self.assertEqual(pane.returncode, 0, pane.stderr)
        self.assertIn("focused pane p1", pane.stdout)

    def test_focus_pane_reports_error_without_zoom_success(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_bin = Path(tmp) / "bin"
            fake_bin.mkdir()
            write_executable(
                fake_bin / "herdr",
                r'''#!/usr/bin/env python3
import json
import sys
argv = sys.argv[1:]
if argv[:1] == ["status"]:
    print(json.dumps({"status": "ok"}))
    raise SystemExit(0)
if argv[:2] == ["agent", "focus"]:
    print("focus refused", file=sys.stderr)
    raise SystemExit(3)
if argv[:2] == ["pane", "get"]:
    print(json.dumps({"result": {"pane_id": argv[2]}}))
    raise SystemExit(0)
if argv[:2] == ["pane", "zoom"]:
    # Zoom succeeding must NOT be treated as focus success.
    raise SystemExit(0)
print("unexpected", argv, file=sys.stderr)
raise SystemExit(9)
''',
            )
            result = self.run_cli(
                "focus-pane",
                "p1",
                path=f"{fake_bin}{os.pathsep}{os.environ.get('PATH', '')}",
            )
        self.assertEqual(result.returncode, 1)
        self.assertIn("focus refused", result.stderr)
        self.assertNotIn("focused pane", result.stdout)

    def test_mirror_dry_run_with_mocked_herdr(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_bin = Path(tmp) / "bin"
            fake_bin.mkdir()
            write_executable(fake_bin / "herdr", FAKE_HERDR_FULL)
            result = self.run_cli(
                "mirror",
                "--dry-run",
                "--tab",
                "t1",
                "--no-status",
                "--no-log",
                "--json",
                path=f"{fake_bin}{os.pathsep}{os.environ.get('PATH', '')}",
            )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("DRY-RUN", result.stdout)
        self.assertIn("create_tab", result.stdout)
        idx = result.stdout.find("{")
        self.assertGreaterEqual(idx, 0)
        payload = json.loads(result.stdout[idx:])
        self.assertEqual(payload["scope"], "current-tab")
        self.assertGreaterEqual(payload["desired_count"], 1)

    def test_mirror_tmux_parity_dry_run_is_full_session(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_bin = Path(tmp) / "bin"
            fake_bin.mkdir()
            write_executable(fake_bin / "herdr", FAKE_HERDR_FULL)
            result = self.run_cli(
                "mirror",
                "--tmux-parity",
                "--dry-run",
                "--no-status",
                "--no-log",
                "--json",
                path=f"{fake_bin}{os.pathsep}{os.environ.get('PATH', '')}",
            )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("tmux-parity", result.stdout)
        idx = result.stdout.find("{")
        payload = json.loads(result.stdout[idx:])
        self.assertEqual(payload["scope"], "all")
        self.assertTrue(payload["tmux_parity"])
        self.assertTrue(payload["sync_focus"])
        self.assertTrue(payload["sync_order"])
        self.assertTrue(payload["sync_ratios"])

    def _lifecycle_env(self, tmp: str, fake_bin: Path) -> tuple[str, dict]:
        sock = Path(tmp) / "herdr.sock"
        sock.write_text("", encoding="utf-8")
        env = {
            "HOME": tmp,
            "XDG_STATE_HOME": str(Path(tmp) / "state"),
            "HERDR_ENV": "1",
            "HERDR_SOCKET_PATH": str(sock),
            "HERDR_TAB_ID": "t1",
            "HERDR_WORKSPACE_ID": "w1",
            "CMUX_SURFACE_ID": "surface-cli",
        }
        return f"{fake_bin}{os.pathsep}{os.environ.get('PATH', '')}", env

    def test_send_key_named_key(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_bin = Path(tmp) / "bin"
            fake_bin.mkdir()
            write_executable(fake_bin / "herdr", FAKE_HERDR_FULL)
            path, env = self._lifecycle_env(tmp, fake_bin)
            result = self.run_cli("send-key", "p1", "C-Up", path=path, env_extra=env)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("C-Up", result.stdout)
        self.assertIn("p1", result.stdout)

    def test_send_key_unknown_fails_closed(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_bin = Path(tmp) / "bin"
            fake_bin.mkdir()
            write_executable(fake_bin / "herdr", FAKE_HERDR_FULL)
            path, env = self._lifecycle_env(tmp, fake_bin)
            result = self.run_cli("send-key", "p1", "NotAKey", path=path, env_extra=env)
        self.assertEqual(result.returncode, 1)
        self.assertIn("unknown key name", result.stderr)
        self.assertNotIn("Traceback", result.stderr)

    def test_attach_observe_detach_never_stops_server(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_bin = Path(tmp) / "bin"
            fake_bin.mkdir()
            write_executable(fake_bin / "herdr", FAKE_HERDR_FULL)
            path, env = self._lifecycle_env(tmp, fake_bin)
            attached = self.run_cli("attach", "--json", path=path, env_extra=env)
            observed = self.run_cli(
                "observe",
                "--method",
                "pane_surfaces",
                "--json",
                path=path,
                env_extra=env,
            )
            detached = self.run_cli("detach", "--json", path=path, env_extra=env)
        self.assertEqual(attached.returncode, 0, attached.stderr + attached.stdout)
        attach_payload = json.loads(attached.stdout)
        self.assertTrue(attach_payload["ok"])
        self.assertFalse(attach_payload["server_stopped"])
        self.assertTrue(attach_payload.get("restore_path"))
        self.assertEqual(observed.returncode, 0, observed.stderr + observed.stdout)
        observe_payload = json.loads(observed.stdout)
        self.assertTrue(observe_payload["ok"])
        self.assertEqual(detached.returncode, 0, detached.stderr + detached.stdout)
        detach_payload = json.loads(detached.stdout)
        self.assertTrue(detach_payload["ok"])
        self.assertFalse(detach_payload["server_stopped"])

    def test_attach_yields_when_native_live(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_bin = Path(tmp) / "bin"
            fake_bin.mkdir()
            write_executable(fake_bin / "herdr", FAKE_HERDR_FULL)
            path, env = self._lifecycle_env(tmp, fake_bin)
            env["CMUX_HERDR_NATIVE_LIVE"] = "1"
            attached = self.run_cli("attach", "--json", path=path, env_extra=env)
            observed = self.run_cli(
                "observe",
                "--method",
                "pane_surfaces",
                "--json",
                path=path,
                env_extra=env,
            )
            restored = self.run_cli("restore", "--json", path=path, env_extra=env)
            detached = self.run_cli("detach", "--json", path=path, env_extra=env)
        self.assertEqual(attached.returncode, 0, attached.stderr + attached.stdout)
        attach_payload = json.loads(attached.stdout)
        self.assertTrue(attach_payload["ok"])
        self.assertEqual(attach_payload["outcome"], "native_owns")
        self.assertFalse(attach_payload["server_stopped"])
        self.assertIsNone(attach_payload.get("restore_path"))
        observe_payload = json.loads(observed.stdout)
        self.assertEqual(observe_payload["outcome"], "native_owns")
        self.assertEqual(observe_payload["panes"], [])
        restore_payload = json.loads(restored.stdout)
        self.assertEqual(restore_payload["outcome"], "native_owns")
        self.assertEqual(restore_payload["mode"], "reattach")
        detach_payload = json.loads(detached.stdout)
        self.assertEqual(detach_payload["outcome"], "native_owns")
        self.assertFalse(detach_payload["detached"])
        self.assertFalse(detach_payload["server_stopped"])

    def test_restore_without_persist_fails_closed(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_bin = Path(tmp) / "bin"
            fake_bin.mkdir()
            write_executable(fake_bin / "herdr", FAKE_HERDR_FULL)
            path, env = self._lifecycle_env(tmp, fake_bin)
            result = self.run_cli("restore", "--json", path=path, env_extra=env)
        self.assertEqual(result.returncode, 2, result.stderr + result.stdout)
        payload = json.loads(result.stdout)
        self.assertFalse(payload["ok"])
        self.assertEqual(payload["outcome"], "no_persist")

    def test_api_lists_methods_and_refuses_server_stop(self):
        listed = self.run_cli("api", "--list")
        self.assertEqual(listed.returncode, 0, listed.stderr)
        self.assertIn("pane.close", listed.stdout)
        self.assertIn("agent.prompt", listed.stdout)
        self.assertNotIn("server.stop", listed.stdout)
        refused = self.run_cli("api", "server.stop")
        self.assertEqual(refused.returncode, 1)
        self.assertIn("server.stop", refused.stderr)
        self.assertNotIn("Traceback", refused.stderr)
        graphics = self.run_cli("api", "pane.graphics.set")
        self.assertEqual(graphics.returncode, 1)
        self.assertIn("pane.graphics.set", graphics.stderr)

    def test_send_and_new_tab_fall_back_to_herdr_cli(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_bin = Path(tmp) / "bin"
            fake_bin.mkdir()
            write_executable(fake_bin / "herdr", FAKE_HERDR_FULL)
            path, env = self._lifecycle_env(tmp, fake_bin)
            sent = self.run_cli(
                "send", "p1", "hello", "world", path=path, env_extra=env
            )
            created = self.run_cli(
                "new-tab", "--label", "logs", "--json", path=path, env_extra=env
            )
        self.assertEqual(sent.returncode, 0, sent.stderr)
        self.assertIn("p1", sent.stdout)
        self.assertEqual(created.returncode, 0, created.stderr)
        payload = json.loads(created.stdout)
        self.assertTrue(payload["ok"])
        self.assertEqual(payload["via"], "cli")
        self.assertEqual(payload["method"], "tab.create")

    def test_close_pane_requires_force_when_busy(self):
        fake = r'''#!/usr/bin/env python3
import json, sys
argv = sys.argv[1:]
command = argv[:2] if len(argv) >= 2 else argv
if argv[:1] == ["status"]:
    print(json.dumps({"status": "ok"}))
    raise SystemExit(0)
if command == ["pane", "get"]:
    print(json.dumps({"result": {"pane_id": argv[2], "agent_status": "working"}}))
    raise SystemExit(0)
if command == ["pane", "close"]:
    print("should not close busy pane without --force", file=sys.stderr)
    raise SystemExit(9)
print("unexpected " + " ".join(argv), file=sys.stderr)
raise SystemExit(9)
'''
        with tempfile.TemporaryDirectory() as tmp:
            fake_bin = Path(tmp) / "bin"
            fake_bin.mkdir()
            write_executable(fake_bin / "herdr", fake)
            path, env = self._lifecycle_env(tmp, fake_bin)
            result = self.run_cli("close-pane", "w1:p1", path=path, env_extra=env)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("busy", result.stderr)
        self.assertIn("--force", result.stderr)


if __name__ == "__main__":
    unittest.main()
