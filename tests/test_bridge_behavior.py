#!/usr/bin/env python3
"""Behavior tests for the stdlib-only cmux/herdr bridge."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from bridge import cmux_herdr_bridge as bridge  # noqa: E402


def completed(args=(), returncode=0, stdout="", stderr=""):
    return subprocess.CompletedProcess(args, returncode, stdout, stderr)


class JsonAndCommandTests(unittest.TestCase):
    def test_parse_json_accepts_object_array_and_leading_noise(self):
        self.assertEqual(bridge._parse_json_payload('{"ok": true}'), {"ok": True})
        self.assertEqual(bridge._parse_json_payload("[1, 2]"), [1, 2])
        self.assertEqual(
            bridge._parse_json_payload('diagnostic output\n{"result": {"panes": []}}\n'),
            {"result": {"panes": []}},
        )

    def test_parse_json_rejects_empty_and_invalid_payloads(self):
        with self.assertRaisesRegex(bridge.BridgeError, "empty JSON response"):
            bridge._parse_json_payload("  \n")
        with self.assertRaises(json.JSONDecodeError):
            bridge._parse_json_payload("not json")

    @mock.patch.object(bridge, "run_cmd")
    @mock.patch.object(bridge, "which", return_value="/mock/herdr")
    def test_herdr_json_invokes_cli_and_parses_result(self, _which, run_cmd):
        run_cmd.return_value = completed(stdout='{"result": {"tabs": []}}')

        result = bridge.herdr_json(["tab", "list"], timeout=2.5)

        self.assertEqual(result, {"result": {"tabs": []}})
        run_cmd.assert_called_once_with(["herdr", "tab", "list"], timeout=2.5)

    @mock.patch.object(bridge, "which", return_value=None)
    def test_missing_herdr_is_a_user_facing_error(self, _which):
        with self.assertRaisesRegex(bridge.BridgeError, "herdr not found on PATH"):
            bridge.herdr_json(["pane", "list"])

    @mock.patch.object(bridge, "which", return_value=None)
    def test_missing_cmux_is_a_user_facing_error(self, _which):
        with self.assertRaisesRegex(bridge.BridgeError, "cmux not found on PATH"):
            bridge.cmux_cmd(["list-status"])

    @mock.patch.object(bridge, "run_cmd")
    @mock.patch.object(bridge, "which", return_value="/mock/herdr")
    def test_herdr_command_failure_includes_stderr(self, _which, run_cmd):
        run_cmd.return_value = completed(returncode=7, stderr="socket refused\n")
        with self.assertRaisesRegex(bridge.BridgeError, "socket refused"):
            bridge.herdr_json(["pane", "list"])

    @mock.patch.object(bridge, "run_cmd")
    @mock.patch.object(bridge, "which", return_value="/mock/cmux")
    def test_cmux_command_passes_workspace(self, _which, run_cmd):
        run_cmd.return_value = completed()

        bridge.cmux_cmd(["list-status"], workspace="workspace:9", timeout=4)

        run_cmd.assert_called_once_with(
            ["cmux", "list-status", "--workspace", "workspace:9"], timeout=4
        )


class SnapshotParsingTests(unittest.TestCase):
    @mock.patch.object(bridge, "herdr_json")
    def test_fetch_panes_parses_nested_result_and_skips_missing_ids(self, herdr_json):
        herdr_json.return_value = {
            "result": {
                "panes": [
                    {
                        "pane_id": "w1:p2",
                        "tab_id": "w1:t1",
                        "workspace_id": "w1",
                        "agent": "pi",
                        "agent_status": "working",
                        "focused": True,
                    },
                    {"tab_id": "w1:t1", "agent": "ignored"},
                ]
            }
        }

        panes = bridge.fetch_panes()

        self.assertEqual(len(panes), 1)
        self.assertEqual(panes[0].pane_id, "w1:p2")
        self.assertEqual(panes[0].agent, "pi")
        self.assertTrue(panes[0].focused)

    @mock.patch.object(bridge, "fetch_panes")
    @mock.patch.object(bridge, "herdr_json", side_effect=bridge.BridgeError("unsupported"))
    def test_fetch_agents_falls_back_to_agent_panes(self, _herdr_json, fetch_panes):
        agent = bridge.Pane("p1", "t1", "w1", agent="claude")
        plain = bridge.Pane("p2", "t1", "w1")
        fetch_panes.return_value = [agent, plain]

        self.assertEqual(bridge.fetch_agents(), [agent])


class SyncTests(unittest.TestCase):
    @mock.patch.object(bridge, "cmux_cmd")
    @mock.patch.object(bridge, "list_cmux_herdr_keys")
    def test_sync_sets_agent_status_and_clears_only_stale_herdr_keys(
        self, list_keys, cmux_cmd
    ):
        pane = bridge.Pane(
            pane_id="w1:p1",
            tab_id="w1:t1",
            workspace_id="w1",
            agent="pi",
            agent_status="working",
            label="Builder",
        )
        snap = bridge.Snapshot(panes=[pane], tabs=[], workspaces=[])
        list_keys.return_value = [pane.status_key, "herdr:stale"]
        cmux_cmd.return_value = completed()

        summary = bridge.sync_to_cmux(
            snap, workspace="workspace:3", set_progress=False, log=False
        )

        self.assertEqual(summary["applied"], [pane.status_key])
        self.assertEqual(summary["stale_cleared"], ["herdr:stale"])
        calls = cmux_cmd.call_args_list
        self.assertTrue(any(c.args[0][0] == "set-status" for c in calls))
        self.assertIn(
            mock.call(["clear-status", "herdr:stale"], workspace="workspace:3"),
            calls,
        )
        self.assertNotIn(
            mock.call(["clear-status", pane.status_key], workspace="workspace:3"),
            calls,
        )

    @mock.patch.object(bridge, "cmux_cmd")
    @mock.patch.object(bridge, "list_cmux_herdr_keys", return_value=["herdr:old"])
    def test_sync_retains_stale_status_when_cleanup_disabled(self, _keys, cmux_cmd):
        cmux_cmd.return_value = completed()
        snap = bridge.Snapshot(panes=[], tabs=[], workspaces=[])

        summary = bridge.sync_to_cmux(
            snap,
            workspace="workspace:1",
            clear_stale=False,
            set_progress=False,
            log=False,
        )

        self.assertEqual(summary["stale_cleared"], [])
        cmux_cmd.assert_not_called()

    def test_sync_without_resolvable_workspace_fails_gracefully(self):
        snap = bridge.Snapshot(panes=[], tabs=[], workspaces=[])
        with mock.patch.object(bridge, "resolve_cmux_workspace", return_value=None):
            with self.assertRaisesRegex(bridge.BridgeError, "could not resolve"):
                bridge.sync_to_cmux(snap, log=False)


class AvailabilityTests(unittest.TestCase):
    def test_missing_tools_and_socket_report_unavailable_without_raising(self):
        with mock.patch.dict(os.environ, {}, clear=True), mock.patch.object(
            bridge, "which", return_value=None
        ):
            self.assertFalse(bridge.herdr_available())
            status = bridge.dual_status()

        self.assertFalse(status["herdr"]["available"])
        self.assertFalse(status["cmux"]["available"])
        self.assertFalse(status["nested"])


class InstallScriptTests(unittest.TestCase):
    def test_install_script_syntax_and_temp_home_install(self):
        script = ROOT / "scripts" / "install.sh"
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
            sidebar = Path(home) / ".config" / "cmux" / "sidebars" / "herdr.swift"
            self.assertTrue(target.exists())
            self.assertTrue(sidebar.exists())
            self.assertIn("Done.", install.stdout)


class ParentWorkspaceBindingTests(unittest.TestCase):
    def test_plain_unknown_shell_is_not_an_agent(self):
        pane = bridge.Pane(
            pane_id="p-shell",
            tab_id="t1",
            workspace_id="w1",
            agent=None,
            agent_status="unknown",
        )
        snap = bridge.Snapshot(panes=[pane], tabs=[], workspaces=[])
        self.assertFalse(pane.has_agent)
        self.assertEqual(snap.agent_panes(), [])

    def test_persisted_parent_wins_after_outer_focus_changes(self):
        with tempfile.TemporaryDirectory() as tmp, mock.patch.dict(
            os.environ,
            {
                "XDG_STATE_HOME": tmp,
                "HERDR_SOCKET_PATH": "/tmp/herdr.sock",
                "HERDR_WORKSPACE_ID": "w1",
                "CMUX_SURFACE_ID": "surface-uuid",
            },
            clear=False,
        ), mock.patch.object(bridge, "which", return_value="/mock/cmux"), mock.patch.object(
            bridge, "_workspace_is_valid", return_value=True
        ), mock.patch.object(
            bridge, "_workspace_from_identify", side_effect=["workspace:7", "workspace:99"]
        ) as identify:
            self.assertEqual(bridge.resolve_cmux_workspace(), "workspace:7")
            self.assertEqual(bridge.resolve_cmux_workspace(), "workspace:7")
            identify.assert_called_once_with("surface-uuid")

    def test_invalid_binding_is_replaced_atomically(self):
        with tempfile.TemporaryDirectory() as tmp, mock.patch.dict(
            os.environ,
            {
                "XDG_STATE_HOME": tmp,
                "HERDR_SOCKET_PATH": "/tmp/herdr.sock",
                "HERDR_WORKSPACE_ID": "w1",
                "CMUX_SURFACE_ID": "surface-uuid",
            },
            clear=False,
        ), mock.patch.object(bridge, "which", return_value="/mock/cmux"), mock.patch.object(
            bridge, "_workspace_is_valid", side_effect=lambda ws: ws != "workspace:old"
        ), mock.patch.object(
            bridge, "_workspace_from_identify", return_value="workspace:new"
        ):
            bridge._save_parent_binding("workspace:old")
            self.assertEqual(bridge.resolve_cmux_workspace(), "workspace:new")
            self.assertEqual(bridge._load_parent_binding(), "workspace:new")

    def test_missing_fingerprint_refuses_auto_resolve(self):
        with tempfile.TemporaryDirectory() as tmp, mock.patch.dict(
            os.environ,
            {
                "XDG_STATE_HOME": tmp,
                "HERDR_SOCKET_PATH": "/tmp/herdr.sock",
                "HERDR_WORKSPACE_ID": "w1",
            },
            clear=True,
        ), mock.patch.object(bridge, "which", return_value="/mock/cmux"), mock.patch.object(
            bridge, "_workspace_from_identify"
        ) as identify:
            with self.assertRaisesRegex(bridge.BridgeError, "incomplete host fingerprint"):
                bridge.resolve_cmux_workspace()
            identify.assert_not_called()

    def test_two_hosts_keep_distinct_parent_bindings(self):
        """Two fake outer surfaces sharing a Herdr socket must not collide."""
        with tempfile.TemporaryDirectory() as tmp:
            state = Path(tmp)
            host_a = {
                "XDG_STATE_HOME": tmp,
                "HERDR_SOCKET_PATH": "/tmp/shared-herdr.sock",
                "HERDR_WORKSPACE_ID": "w1",
                "CMUX_SURFACE_ID": "surface-host-a",
                "HERDR_SERVER_PID": "1001",
            }
            host_b = {
                "XDG_STATE_HOME": tmp,
                "HERDR_SOCKET_PATH": "/tmp/shared-herdr.sock",
                "HERDR_WORKSPACE_ID": "w1",
                "CMUX_SURFACE_ID": "surface-host-b",
                "HERDR_SERVER_PID": "1001",
            }

            with mock.patch.dict(os.environ, host_a, clear=True), mock.patch.object(
                bridge, "which", return_value="/mock/cmux"
            ), mock.patch.object(
                bridge, "_workspace_is_valid", return_value=True
            ), mock.patch.object(
                bridge, "_workspace_from_identify", return_value="workspace:A"
            ):
                self.assertEqual(bridge.resolve_cmux_workspace(), "workspace:A")
                key_a = bridge._parent_key()
                path_a = Path(bridge._binding_path())

            with mock.patch.dict(os.environ, host_b, clear=True), mock.patch.object(
                bridge, "which", return_value="/mock/cmux"
            ), mock.patch.object(
                bridge, "_workspace_is_valid", return_value=True
            ), mock.patch.object(
                bridge, "_workspace_from_identify", return_value="workspace:B"
            ):
                self.assertEqual(bridge.resolve_cmux_workspace(), "workspace:B")
                key_b = bridge._parent_key()
                path_b = Path(bridge._binding_path())

            self.assertNotEqual(key_a, key_b)
            self.assertNotEqual(path_a, path_b)
            self.assertTrue(path_a.exists())
            self.assertTrue(path_b.exists())
            self.assertEqual(json.loads(path_a.read_text())["workspace_ref"], "workspace:A")
            self.assertEqual(json.loads(path_b.read_text())["workspace_ref"], "workspace:B")

            # Invoking env selects its own binding; the other host file stays intact.
            with mock.patch.dict(os.environ, host_a, clear=True), mock.patch.object(
                bridge, "which", return_value="/mock/cmux"
            ), mock.patch.object(
                bridge, "_workspace_is_valid", return_value=True
            ), mock.patch.object(
                bridge, "_workspace_from_identify"
            ) as identify:
                self.assertEqual(bridge.resolve_cmux_workspace(), "workspace:A")
                identify.assert_not_called()

            parents = sorted(p.name for p in state.joinpath("cmux-herdr").glob("parent-*.json"))
            self.assertEqual(len(parents), 2)


class AssociationSyncTests(unittest.TestCase):
    def test_sync_writes_association_cache(self):
        pane = bridge.Pane(
            pane_id="w2:p34",
            tab_id="w2:t17",
            workspace_id="w2",
            agent="pi",
            agent_status="working",
            agent_session_path="/tmp/sess.jsonl",
            agent_session_kind="path",
        )
        snap = bridge.Snapshot(panes=[pane], tabs=[], workspaces=[])

        def fake_cmux(args, workspace=None):
            class R:
                returncode = 0
                stdout = ""
                stderr = ""

            return R()

        with tempfile.TemporaryDirectory() as tmp, mock.patch.dict(
            os.environ,
            {
                "XDG_STATE_HOME": tmp,
                "HERDR_SOCKET_PATH": "/tmp/herdr.sock",
                "HERDR_WORKSPACE_ID": "w2",
                "CMUX_SURFACE_ID": "surface-uuid",
                "CMUX_WORKSPACE_ID": "workspace:7",
            },
            clear=False,
        ), mock.patch.object(bridge, "cmux_cmd", side_effect=fake_cmux), mock.patch.object(
            bridge, "fetch_snapshot", return_value=snap
        ), mock.patch.object(
            bridge, "list_cmux_herdr_keys", return_value=[]
        ):
            summary = bridge.sync_to_cmux(snapshot=snap, workspace="workspace:7", log=False)
            self.assertIn("associations", summary)
            self.assertEqual(summary["associations"]["pane_count"], 1)
            assoc_path = Path(summary["associations"]["path"])
            self.assertTrue(assoc_path.exists())
            data = json.loads(assoc_path.read_text())
            self.assertEqual(data["panes"]["w2:p34"]["status_key"], "herdr:w2:p34")
            self.assertEqual(data["cmux_workspace"], "workspace:7")
            self.assertEqual(data["cmux_surface_id"], "surface-uuid")
            self.assertIn("host_fingerprint_key", data)

    def test_two_hosts_keep_distinct_association_files(self):
        pane = bridge.Pane(
            pane_id="w2:p1",
            tab_id="w2:t1",
            workspace_id="w2",
            agent="pi",
            agent_status="idle",
        )
        snap = bridge.Snapshot(panes=[pane], tabs=[], workspaces=[])

        with tempfile.TemporaryDirectory() as tmp:
            env_a = {
                "XDG_STATE_HOME": tmp,
                "HERDR_SOCKET_PATH": "/tmp/shared-herdr.sock",
                "HERDR_WORKSPACE_ID": "w2",
                "CMUX_SURFACE_ID": "surface-host-a",
            }
            env_b = {
                "XDG_STATE_HOME": tmp,
                "HERDR_SOCKET_PATH": "/tmp/shared-herdr.sock",
                "HERDR_WORKSPACE_ID": "w2",
                "CMUX_SURFACE_ID": "surface-host-b",
            }
            with mock.patch.dict(os.environ, env_a, clear=True):
                path_a = Path(
                    bridge.update_association_map(snap, cmux_workspace="workspace:A")["path"]
                )
            with mock.patch.dict(os.environ, env_b, clear=True):
                path_b = Path(
                    bridge.update_association_map(snap, cmux_workspace="workspace:B")["path"]
                )
            self.assertNotEqual(path_a, path_b)
            self.assertTrue(path_a.exists())
            self.assertTrue(path_b.exists())
            self.assertEqual(json.loads(path_a.read_text())["cmux_workspace"], "workspace:A")
            self.assertEqual(json.loads(path_b.read_text())["cmux_workspace"], "workspace:B")


if __name__ == "__main__":
    unittest.main()

