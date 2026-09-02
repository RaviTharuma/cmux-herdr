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
            sidebar_js = Path(home) / ".config" / "cmux" / "sidebars" / "herdr.js"
            sidebar_swift = Path(home) / ".config" / "cmux" / "sidebars" / "herdr.swift"
            self.assertTrue(target.exists())
            self.assertTrue(sidebar_js.exists())
            self.assertTrue(sidebar_swift.exists())
            self.assertIn("cmux-herdr plugin install", install.stdout)
            self.assertIn("Plugin installed.", install.stdout)

    def test_sidebar_install_path_prefers_js_over_swift(self):
        with tempfile.TemporaryDirectory() as home:
            sidebars = Path(home) / ".config" / "cmux" / "sidebars"
            sidebars.mkdir(parents=True)
            js = sidebars / "herdr.js"
            swift = sidebars / "herdr.swift"
            with mock.patch.dict(os.environ, {"HOME": home}, clear=False):
                self.assertTrue(bridge.sidebar_install_path().endswith("herdr.js"))
                swift.write_text("// fallback\n", encoding="utf-8")
                self.assertTrue(bridge.sidebar_install_path().endswith("herdr.swift"))
                js.write_text("// product\n", encoding="utf-8")
                self.assertTrue(bridge.sidebar_install_path().endswith("herdr.js"))

    def test_watch_service_install_does_not_replace_loaded_real_home_agent(self):
        script = ROOT / "scripts" / "install-watch-service.sh"
        syntax = subprocess.run(
            ["bash", "-n", str(script)], capture_output=True, text=True, check=False
        )
        self.assertEqual(syntax.returncode, 0, syntax.stderr)

        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            home = tmp_path / "home"
            fake_bin = tmp_path / "bin"
            home.mkdir()
            fake_bin.mkdir()
            cli = home / ".local" / "bin" / "cmux-herdr"
            cli.parent.mkdir(parents=True)
            cli.write_text("#!/bin/sh\n", encoding="utf-8")
            cli.chmod(0o755)
            launchctl_log = tmp_path / "launchctl.log"
            launchctl = fake_bin / "launchctl"
            launchctl.write_text(
                "#!/bin/sh\n"
                f"printf '%s\\n' \"$*\" >> {launchctl_log!s}\n"
                "case \"$1\" in\n"
                "  print) exit 0 ;;\n"
                "  bootstrap) exit 0 ;;\n"
                "esac\n"
                "exit 0\n",
                encoding="utf-8",
            )
            launchctl.chmod(0o755)
            env = os.environ.copy()
            env["HOME"] = str(home)
            env["PATH"] = f"{fake_bin}:{env['PATH']}"
            install = subprocess.run(
                ["bash", str(script)],
                cwd=ROOT,
                env=env,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(install.returncode, 0, install.stderr)
            calls = launchctl_log.read_text(encoding="utf-8")
            self.assertIn(
                f"bootout gui/{os.getuid()} {home}/Library/LaunchAgents/"
                "com.cmux-herdr.watch.plist",
                calls,
            )
            self.assertNotIn(
                f"bootout gui/{os.getuid()}/com.cmux-herdr.watch",
                calls,
            )


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

    def test_sync_skips_when_native_attachment_live(self):
        pane = bridge.Pane(
            pane_id="w2:p34",
            tab_id="w2:t17",
            workspace_id="w2",
            agent="pi",
            agent_status="working",
        )
        snap = bridge.Snapshot(panes=[pane], tabs=[], workspaces=[])
        calls = []

        def fake_cmux(args, workspace=None):
            calls.append(list(args))

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
                "CMUX_HERDR_NATIVE_LIVE": "1",
            },
            clear=False,
        ), mock.patch.object(bridge, "cmux_cmd", side_effect=fake_cmux), mock.patch.object(
            bridge, "fetch_snapshot", return_value=snap
        ), mock.patch.object(
            bridge, "list_cmux_herdr_keys", return_value=[]
        ):
            bridge.reset_native_skip_log()
            summary = bridge.sync_to_cmux(snapshot=snap, workspace="workspace:7", log=False)
            self.assertTrue(summary["native_live"])
            self.assertEqual(summary["skipped_reason"], "native_live")
            self.assertEqual(summary["applied"], [])
            self.assertEqual(summary["writer"], "native")
            self.assertFalse(any(c and c[0] == "set-status" for c in calls))
            self.assertEqual(summary["associations"]["pane_count"], 1)

    def test_sync_force_plugin_writes_when_native_live(self):
        pane = bridge.Pane(
            pane_id="w2:p34",
            tab_id="w2:t17",
            workspace_id="w2",
            agent="pi",
            agent_status="working",
        )
        snap = bridge.Snapshot(panes=[pane], tabs=[], workspaces=[])
        calls = []

        def fake_cmux(args, workspace=None):
            calls.append(list(args))

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
                "CMUX_HERDR_NATIVE_LIVE": "1",
                "CMUX_HERDR_FORCE_PLUGIN": "1",
            },
            clear=False,
        ), mock.patch.object(bridge, "cmux_cmd", side_effect=fake_cmux), mock.patch.object(
            bridge, "list_cmux_herdr_keys", return_value=[]
        ):
            bridge.reset_native_skip_log()
            summary = bridge.sync_to_cmux(snapshot=snap, workspace="workspace:7", log=False)
            self.assertFalse(summary["native_live"])
            self.assertEqual(summary["writer"], "plugin-forced")
            self.assertEqual(summary["applied"], ["herdr:w2:p34"])
            self.assertTrue(any(c and c[0] == "set-status" for c in calls))

    def test_sync_skips_identical_second_write(self):
        pane = bridge.Pane(
            pane_id="w2:p34",
            tab_id="w2:t17",
            workspace_id="w2",
            agent="pi",
            agent_status="working",
            label="Bot",
        )
        snap = bridge.Snapshot(panes=[pane], tabs=[], workspaces=[])
        calls = []

        def fake_cmux(args, workspace=None):
            calls.append(list(args))

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
            },
            clear=False,
        ), mock.patch.object(bridge, "cmux_cmd", side_effect=fake_cmux), mock.patch.object(
            bridge, "list_cmux_herdr_keys", return_value=[]
        ):
            first = bridge.sync_to_cmux(snapshot=snap, workspace="workspace:7", log=False)
            self.assertEqual(first["applied"], ["herdr:w2:p34"])
            set_status_first = [c for c in calls if c and c[0] == "set-status"]
            self.assertEqual(len(set_status_first), 1)
            calls.clear()
            second = bridge.sync_to_cmux(snapshot=snap, workspace="workspace:7", log=False)
            self.assertEqual(second["applied"], [])
            self.assertEqual(second["skipped_unchanged"], ["herdr:w2:p34"])
            self.assertFalse(any(c and c[0] == "set-status" for c in calls))

    def test_sync_title_lock_keeps_locked_display_name(self):
        pane = bridge.Pane(
            pane_id="w2:p34",
            tab_id="w2:t17",
            workspace_id="w2",
            agent="pi",
            agent_status="working",
            label="NewName",
        )
        snap = bridge.Snapshot(panes=[pane], tabs=[], workspaces=[])
        calls = []

        def fake_cmux(args, workspace=None):
            calls.append(list(args))

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
            },
            clear=False,
        ), mock.patch.object(bridge, "cmux_cmd", side_effect=fake_cmux), mock.patch.object(
            bridge, "list_cmux_herdr_keys", return_value=[]
        ):
            bridge.set_title_lock("w2:p34", locked=True, title="Orchestrator")
            summary = bridge.sync_to_cmux(snapshot=snap, workspace="workspace:7", log=False)
            self.assertEqual(summary["applied"], ["herdr:w2:p34"])
            set_calls = [c for c in calls if c and c[0] == "set-status"]
            self.assertEqual(len(set_calls), 1)
            self.assertIn("Orchestrator", set_calls[0][2])
            self.assertNotIn("NewName", set_calls[0][2])
            data = json.loads(Path(summary["associations"]["path"]).read_text())
            self.assertTrue(data["panes"]["w2:p34"]["title_lock"])
            self.assertEqual(data["panes"]["w2:p34"]["locked_title"], "Orchestrator")


class FocusAndReadTests(unittest.TestCase):
    def setUp(self):
        bridge.reset_herdr_rpc()
        self._env = mock.patch.dict(
            os.environ,
            {"HERDR_SOCKET_PATH": "/no/such/herdr.sock"},
            clear=False,
        )
        self._env.start()

    def tearDown(self):
        self._env.stop()
        bridge.reset_herdr_rpc()

    @mock.patch.object(bridge, "run_cmd")
    @mock.patch.object(bridge, "which", return_value="/mock/herdr")
    def test_focus_pane_does_not_use_zoom_fallback(self, _which, run_cmd):
        run_cmd.side_effect = [
            completed(returncode=1, stderr="agent focus denied"),
            completed(stdout='{"result": {"pane_id": "w2:p1"}}'),
        ]
        with self.assertRaisesRegex(bridge.BridgeError, "agent focus denied"):
            bridge.focus_pane("w2:p1")
        calls = [c.args[0] for c in run_cmd.call_args_list]
        self.assertEqual(calls[0], ["herdr", "agent", "focus", "w2:p1"])
        self.assertEqual(calls[1][:3], ["herdr", "pane", "get"])
        self.assertFalse(any("zoom" in c for c in calls))

    @mock.patch.object(bridge, "run_cmd")
    @mock.patch.object(bridge, "which", return_value="/mock/herdr")
    def test_focus_workspace_and_agent(self, _which, run_cmd):
        run_cmd.return_value = completed(returncode=0)
        self.assertEqual(bridge.focus_workspace("w2"), "w2")
        run_cmd.assert_called_with(
            ["herdr", "workspace", "focus", "w2"], timeout=8.0
        )
        self.assertEqual(bridge.focus_agent("reviewer"), "reviewer")
        run_cmd.assert_called_with(
            ["herdr", "agent", "focus", "reviewer"], timeout=8.0
        )

    @mock.patch.object(bridge, "run_cmd")
    @mock.patch.object(bridge, "which", return_value="/mock/herdr")
    def test_read_pane_and_agent_pass_flags(self, _which, run_cmd):
        run_cmd.return_value = completed(returncode=0, stdout="hello\n")
        proc = bridge.read_pane(
            "w2:p1",
            source="recent-unwrapped",
            lines=40,
            format="text",
            ansi=True,
            raw=True,
        )
        self.assertEqual(proc.stdout, "hello\n")
        run_cmd.assert_called_with(
            [
                "herdr",
                "pane",
                "read",
                "w2:p1",
                "--source",
                "recent-unwrapped",
                "--lines",
                "40",
                "--format",
                "text",
                "--ansi",
                "--raw",
            ]
        )
        bridge.read_agent("bot", source="visible", lines=10)
        run_cmd.assert_called_with(
            [
                "herdr",
                "agent",
                "read",
                "bot",
                "--source",
                "visible",
                "--lines",
                "10",
            ]
        )


class DoctorTests(unittest.TestCase):
    def test_doctor_hard_fails_when_herdr_missing(self):
        with mock.patch.object(bridge, "which", return_value=None), mock.patch.dict(
            os.environ, {}, clear=True
        ):
            report = bridge.diagnose_install()
        self.assertFalse(report["ok"])
        self.assertTrue(
            any("herdr not found" in item for item in report["hard_failures"])
        )
        names = [c["name"] for c in report["checks"]]
        self.assertIn("herdr_cli", names)
        self.assertIn("dry_sync", names)

    def test_doctor_hard_fails_incomplete_fingerprint_when_nested(self):
        with tempfile.TemporaryDirectory() as tmp, mock.patch.object(
            bridge, "which", side_effect=lambda cmd: "/mock/herdr" if cmd == "herdr" else None
        ), mock.patch.object(
            bridge, "_herdr_cli_version", return_value="herdr 0.8.0"
        ), mock.patch.object(
            bridge, "herdr_available", return_value=False
        ), mock.patch.dict(
            os.environ,
            {
                "HOME": tmp,
                "XDG_STATE_HOME": str(Path(tmp) / "state"),
                "HERDR_ENV": "1",
                # Intentionally omit CMUX_SURFACE_ID / HERDR_SOCKET_PATH
            },
            clear=True,
        ):
            report = bridge.diagnose_install()
        self.assertFalse(report["ok"])
        self.assertTrue(any("incomplete host fingerprint" in f for f in report["hard_failures"]))
        fp_check = next(c for c in report["checks"] if c["name"] == "host_fingerprint")
        self.assertFalse(fp_check["ok"])
        self.assertTrue(fp_check["hard"])
        self.assertIn("CMUX_SURFACE_ID", fp_check["missing"])
        self.assertIn("HERDR_SOCKET_PATH", fp_check["missing"])

    def test_doctor_ok_with_complete_fingerprint_and_binding(self):
        with tempfile.TemporaryDirectory() as tmp:
            home = Path(tmp)
            state = home / "state"
            sock = home / "herdr.sock"
            sock.write_text("", encoding="utf-8")
            os.chmod(sock, 0o600)
            sidebar = home / ".config" / "cmux" / "sidebars" / "herdr.swift"
            sidebar.parent.mkdir(parents=True)
            sidebar.write_text("// sidebar\n", encoding="utf-8")
            env = {
                "HOME": str(home),
                "XDG_STATE_HOME": str(state),
                "HERDR_ENV": "1",
                "HERDR_SOCKET_PATH": str(sock),
                "CMUX_SURFACE_ID": "surface-doctor",
                "HERDR_WORKSPACE_ID": "w2",
            }
            with mock.patch.dict(os.environ, env, clear=True), mock.patch.object(
                bridge, "which", side_effect=lambda cmd: "/mock/herdr" if cmd == "herdr" else None
            ), mock.patch.object(
                bridge, "_herdr_cli_version", return_value="herdr 0.8.0"
            ), mock.patch.object(
                bridge, "herdr_available", return_value=True
            ), mock.patch.object(
                bridge,
                "fetch_snapshot",
                return_value=bridge.Snapshot(
                    panes=[
                        bridge.Pane(
                            pane_id="w2:p1",
                            tab_id="w2:t1",
                            workspace_id="w2",
                            agent="pi",
                            agent_status="working",
                        )
                    ],
                    tabs=[],
                    workspaces=[],
                ),
            ), mock.patch.object(
                bridge, "resolve_cmux_workspace", return_value="workspace:9"
            ), mock.patch.object(
                bridge,
                "_launchagent_status",
                return_value={
                    "label": bridge.LAUNCH_AGENT_LABEL,
                    "checked": False,
                    "skipped": True,
                    "reason": "not macOS",
                    "loaded": None,
                },
            ):
                bridge._save_parent_binding("workspace:9")
                report = bridge.diagnose_install()

            self.assertTrue(report["ok"], report.get("hard_failures"))
            by_name = {c["name"]: c for c in report["checks"]}
            self.assertTrue(by_name["herdr_cli"]["ok"])
            self.assertTrue(by_name["herdr_socket"]["ok"])
            self.assertIn("mode=", by_name["herdr_socket"]["detail"])
            self.assertTrue(by_name["host_fingerprint"]["ok"])
            self.assertIn("matching parent binding=yes", by_name["state_binding"]["detail"])
            self.assertIn("skipped", by_name["launch_agent"]["detail"])
            self.assertTrue(by_name["sidebar"]["exists"])
            self.assertFalse(by_name["dry_sync"]["dry_sync"]["skipped"])
            self.assertEqual(by_name["dry_sync"]["dry_sync"]["agent_count"], 1)

    def test_doctor_marks_stale_socket_api_unhealthy(self):
        with tempfile.TemporaryDirectory() as tmp:
            sock = Path(tmp) / "stale.sock"
            sock.write_text("", encoding="utf-8")
            os.chmod(sock, 0o600)
            env = {
                "HOME": tmp,
                "XDG_STATE_HOME": str(Path(tmp) / "state"),
                "HERDR_SOCKET_PATH": str(sock),
            }
            with mock.patch.dict(os.environ, env, clear=True), mock.patch.object(
                bridge, "which", side_effect=lambda cmd: "/mock/herdr" if cmd == "herdr" else None
            ), mock.patch.object(
                bridge, "_herdr_cli_version", return_value="herdr 0.8.0"
            ), mock.patch.object(
                bridge, "herdr_available", return_value=False
            ):
                report = bridge.diagnose_install()

        by_name = {check["name"]: check for check in report["checks"]}
        self.assertTrue(by_name["herdr_socket"]["ok"])
        self.assertFalse(by_name["herdr_api"]["ok"])
        self.assertIn("ping failed", by_name["herdr_api"]["detail"])

    def test_doctor_does_not_invent_fingerprint_hosts(self):
        with mock.patch.object(bridge, "which", return_value="/mock/herdr"), mock.patch.object(
            bridge, "_herdr_cli_version", return_value="herdr 0.8.0"
        ), mock.patch.object(bridge, "herdr_available", return_value=False), mock.patch.dict(
            os.environ, {"HERDR_ENV": "1"}, clear=True
        ):
            report = bridge.diagnose_install()
        fp = report["host_fingerprint"]
        self.assertIsNone(fp.get("cmux_surface_id"))
        self.assertIsNone(fp.get("herdr_socket_path"))


if __name__ == "__main__":
    unittest.main()
