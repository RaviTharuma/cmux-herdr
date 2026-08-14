#!/usr/bin/env python3
"""Unit tests for pure bridge helpers (no herdr/cmux required)."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

_ROOT = Path(__file__).resolve().parents[1]
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from bridge.cmux_herdr_bridge import (
    FORCE_PLUGIN_ENV,
    NATIVE_LIVE_ENV,
    STATUS_PREFIX,
    BridgeError,
    Pane,
    Snapshot,
    Tab,
    association_key_for_pane,
    collect_host_fingerprint,
    fingerprint_missing_fields,
    format_associations,
    map_status_to_style,
    native_attachment_is_live,
    require_host_fingerprint,
    reset_native_skip_log,
    resolve_association_parents,
    set_title_lock,
    should_write_status_pill,
    status_value_for_pane,
    status_write_payload,
    update_association_map,
    writer_status,
    _load_association_map,
    _pane_from_raw,
    _parent_key,
    _prior_for_pane,
)


class MapStatusTests(unittest.TestCase):
    def test_working(self):
        icon, color, prio = map_status_to_style("working")
        self.assertEqual(icon, "hammer")
        self.assertTrue(color.startswith("#"))
        self.assertGreaterEqual(prio, 50)

    def test_unknown_default(self):
        icon, color, prio = map_status_to_style("nope")
        self.assertEqual(map_status_to_style(None)[0], "questionmark.circle")
        self.assertEqual(icon, "circle")

    def test_case_insensitive(self):
        self.assertEqual(map_status_to_style("WORKING")[0], "hammer")
        self.assertEqual(map_status_to_style("Done")[0], "checkmark.circle")


class PaneTests(unittest.TestCase):
    def test_status_key(self):
        p = Pane(pane_id="w2:p34", tab_id="w2:t17", workspace_id="w2")
        self.assertEqual(p.status_key, f"{STATUS_PREFIX}w2:p34")

    def test_display_name_prefers_label(self):
        p = Pane(
            pane_id="w2:p1",
            tab_id="w2:t1",
            workspace_id="w2",
            label="Orchestrator",
            terminal_title="long title",
        )
        self.assertEqual(p.display_name, "Orchestrator")

    def test_status_value_includes_agent(self):
        p = Pane(
            pane_id="w2:p1",
            tab_id="w2:t1",
            workspace_id="w2",
            agent="pi",
            agent_status="working",
            label="Bot",
        )
        val = status_value_for_pane(p)
        self.assertIn("pi/working", val)
        self.assertIn("Bot", val)

    def test_pane_from_raw_reads_agent_from_agent_session(self):
        """Herdr 0.8 nests agent under agent_session when top-level agent is absent."""
        pane = _pane_from_raw(
            {
                "pane_id": "w2:p9",
                "tab_id": "w2:t3",
                "workspace_id": "w2",
                "agent_status": "working",
                "agent_session": {
                    "agent": "claude",
                    "kind": "path",
                    "value": "/tmp/session-claude.jsonl",
                },
            }
        )
        self.assertEqual(pane.agent, "claude")
        self.assertEqual(pane.agent_session_path, "/tmp/session-claude.jsonl")
        self.assertEqual(pane.agent_session_kind, "path")

    def test_pane_from_raw_prefers_top_level_agent(self):
        pane = _pane_from_raw(
            {
                "pane_id": "w2:p9",
                "tab_id": "w2:t3",
                "workspace_id": "w2",
                "agent": "pi",
                "agent_session": {"agent": "claude", "kind": "id", "value": "sess-1"},
            }
        )
        self.assertEqual(pane.agent, "pi")


class HostFingerprintTests(unittest.TestCase):
    def test_parent_keys_differ_for_two_surfaces(self):
        import os
        from unittest import mock

        shared = {
            "HERDR_SOCKET_PATH": "/tmp/shared.sock",
            "HERDR_WORKSPACE_ID": "w1",
            "HERDR_SERVER_PID": "42",
        }
        with mock.patch.dict(
            os.environ, {**shared, "CMUX_SURFACE_ID": "surface-a"}, clear=False
        ):
            key_a = _parent_key()
            fp_a = collect_host_fingerprint()
        with mock.patch.dict(
            os.environ, {**shared, "CMUX_SURFACE_ID": "surface-b"}, clear=False
        ):
            key_b = _parent_key()
            fp_b = collect_host_fingerprint()
        self.assertNotEqual(key_a, key_b)
        self.assertEqual(fp_a["herdr_server_pid"], 42)
        self.assertEqual(fp_b["cmux_surface_id"], "surface-b")
        self.assertEqual(fingerprint_missing_fields(fp_a), [])

    def test_require_host_fingerprint_lists_missing_fields(self):
        import os
        from unittest import mock

        with mock.patch.dict(os.environ, {"HERDR_SOCKET_PATH": "/tmp/h.sock"}, clear=True):
            self.assertEqual(fingerprint_missing_fields(), ["CMUX_SURFACE_ID"])
            with self.assertRaisesRegex(BridgeError, "CMUX_SURFACE_ID"):
                require_host_fingerprint()


class AssociationMapTests(unittest.TestCase):
    def test_update_association_map_tracks_and_prunes(self):
        import os
        import tempfile
        from unittest import mock

        pane1 = Pane(
            pane_id="w2:p1",
            tab_id="w2:t1",
            workspace_id="w2",
            agent="pi",
            agent_status="working",
            agent_session_path="/tmp/session-a.jsonl",
            agent_session_kind="path",
            revision=3,
        )
        pane2 = Pane(
            pane_id="w2:p2",
            tab_id="w2:t1",
            workspace_id="w2",
            agent="pi",
            agent_status="idle",
        )
        with tempfile.TemporaryDirectory() as tmp, mock.patch.dict(
            os.environ,
            {
                "XDG_STATE_HOME": tmp,
                "HERDR_SOCKET_PATH": "/tmp/herdr.sock",
                "HERDR_WORKSPACE_ID": "w2",
                "CMUX_SURFACE_ID": "surface-1",
            },
            clear=False,
        ):
            first = update_association_map(
                Snapshot(panes=[pane1, pane2], tabs=[], workspaces=[]),
                cmux_workspace="workspace:7",
            )
            self.assertEqual(first["pane_count"], 2)
            self.assertEqual(first["pruned"], [])
            state = _load_association_map()
            self.assertEqual(state["cmux_workspace"], "workspace:7")
            self.assertIn("w2:p1", state["panes"])
            self.assertEqual(state["panes"]["w2:p1"]["status_key"], "herdr:w2:p1")
            self.assertEqual(
                state["panes"]["w2:p1"]["agent_session_path"],
                "/tmp/session-a.jsonl",
            )

            second = update_association_map(
                Snapshot(panes=[pane1], tabs=[], workspaces=[]),
                cmux_workspace="workspace:7",
            )
            self.assertEqual(second["pane_count"], 1)
            self.assertEqual(second["pruned"], ["w2:p2"])
            rendered = format_associations()
            self.assertIn("associations: 1 panes, 0 mirrored surfaces", rendered)
            self.assertIn("w2:p1", rendered)
            self.assertNotIn("w2:p2  ", rendered)
            rec = _load_association_map()["panes"]["w2:p1"]
            self.assertEqual(rec["association_key"], "w2:p1")
            self.assertEqual(rec["parent_tab_id"], "w2:t1")
            self.assertTrue(rec["heuristic_satisfied"])
            self.assertFalse(rec["title_lock"])


class AssociationKeyTests(unittest.TestCase):
    def test_association_key_includes_session_id(self):
        pane = Pane(
            pane_id="w2:p1",
            tab_id="w2:t1",
            workspace_id="w2",
            agent_session_id="sess-9",
        )
        self.assertEqual(association_key_for_pane(pane), "w2:p1:sess-9")

    def test_association_key_falls_back_to_pane_id(self):
        pane = Pane(pane_id="w2:p1", tab_id="w2:t1", workspace_id="w2")
        self.assertEqual(association_key_for_pane(pane), "w2:p1")


class HeuristicOnceTests(unittest.TestCase):
    def test_keeps_parent_when_snapshot_parentage_flickers(self):
        first = Pane(pane_id="w2:p1", tab_id="w2:t1", workspace_id="w2")
        parents = resolve_association_parents(first, {})
        self.assertEqual(parents["parent_tab_id"], "w2:t1")
        self.assertTrue(parents["heuristic_satisfied"])
        self.assertFalse(parents["used_heuristic"])

        flicker = Pane(pane_id="w2:p1", tab_id="", workspace_id="")
        again = resolve_association_parents(flicker, parents)
        self.assertEqual(again["parent_tab_id"], "w2:t1")
        self.assertEqual(again["parent_workspace_id"], "w2")
        self.assertTrue(again["heuristic_satisfied"])
        self.assertFalse(again["used_heuristic"])

    def test_env_heuristic_runs_once_for_invoking_pane(self):
        import os
        from unittest import mock

        pane = Pane(pane_id="w2:p1", tab_id="", workspace_id="w2")
        env = {
            "HERDR_PANE_ID": "w2:p1",
            "HERDR_TAB_ID": "w2:t9",
            "HERDR_WORKSPACE_ID": "w2",
        }
        with mock.patch.dict(os.environ, env, clear=False):
            first = resolve_association_parents(pane, {})
            self.assertEqual(first["parent_tab_id"], "w2:t9")
            self.assertTrue(first["used_heuristic"])
            self.assertTrue(first["heuristic_satisfied"])

        with mock.patch.dict(
            os.environ,
            {
                "HERDR_PANE_ID": "w2:p1",
                "HERDR_TAB_ID": "w2:t99",
                "HERDR_WORKSPACE_ID": "w2",
            },
            clear=False,
        ):
            second = resolve_association_parents(pane, first)
        self.assertEqual(second["parent_tab_id"], "w2:t9")
        self.assertFalse(second["used_heuristic"])

    def test_provider_tab_id_updates_after_satisfied(self):
        first = Pane(pane_id="w2:p1", tab_id="w2:t1", workspace_id="w2")
        parents = resolve_association_parents(first, {})
        moved = Pane(pane_id="w2:p1", tab_id="w2:t2", workspace_id="w2")
        again = resolve_association_parents(moved, parents)
        self.assertEqual(again["parent_tab_id"], "w2:t2")
        self.assertFalse(again["used_heuristic"])

    def test_session_id_change_drops_prior_locks(self):
        pane_a = Pane(
            pane_id="w2:p1",
            tab_id="w2:t1",
            workspace_id="w2",
            agent_session_id="sess-a",
        )
        pane_b = Pane(
            pane_id="w2:p1",
            tab_id="w2:t1",
            workspace_id="w2",
            agent_session_id="sess-b",
        )
        previous = {
            "w2:p1": {
                "agent_session_id": "sess-a",
                "title_lock": True,
                "locked_title": "Old",
                "heuristic_satisfied": True,
            }
        }
        self.assertEqual(_prior_for_pane(pane_a, previous)["locked_title"], "Old")
        self.assertEqual(_prior_for_pane(pane_b, previous), {})


class TitleLockTests(unittest.TestCase):
    def test_locked_title_used_in_status_value(self):
        pane = Pane(
            pane_id="w2:p1",
            tab_id="w2:t1",
            workspace_id="w2",
            agent="pi",
            agent_status="working",
            label="Other",
        )
        val = status_value_for_pane(pane, locked_title="Orchestrator")
        self.assertIn("Orchestrator", val)
        self.assertNotIn("Other", val)

    def test_parent_tab_id_used_for_tab_label(self):
        pane = Pane(
            pane_id="w2:p1",
            tab_id="",
            workspace_id="w2",
            agent="pi",
            agent_status="idle",
            label="Bot",
        )
        tabs = {"w2:t1": Tab(tab_id="w2:t1", workspace_id="w2", label="Review")}
        val = status_value_for_pane(pane, tabs, parent_tab_id="w2:t1")
        self.assertIn("Review", val)

    def test_diff_before_write_skips_identical_payload(self):
        payload = {
            "value": "pi/working · Bot",
            "icon": "hammer",
            "color": "#ff9500",
            "priority": 80,
        }
        prior = {
            "last_status_value": "pi/working · Bot",
            "last_icon": "hammer",
            "last_color": "#ff9500",
            "last_priority": 80,
        }
        self.assertFalse(should_write_status_pill(payload, prior))
        prior["last_status_value"] = "pi/idle · Bot"
        self.assertTrue(should_write_status_pill(payload, prior))

    def test_title_lock_payload_ignores_label_change(self):
        pane = Pane(
            pane_id="w2:p1",
            tab_id="w2:t1",
            workspace_id="w2",
            agent="pi",
            agent_status="working",
            label="NewName",
        )
        prior = {
            "title_lock": True,
            "locked_title": "Orchestrator",
            "heuristic_satisfied": True,
            "parent_tab_id": "w2:t1",
        }
        payload = status_write_payload(pane, {}, prior)
        self.assertIn("Orchestrator", payload["value"])
        self.assertNotIn("NewName", payload["value"])


class SingleWriterTests(unittest.TestCase):
    def setUp(self):
        reset_native_skip_log()

    def test_env_marks_native_live(self):
        import os
        from unittest import mock

        with mock.patch.dict(os.environ, {NATIVE_LIVE_ENV: "1"}, clear=False):
            self.assertTrue(native_attachment_is_live())
            self.assertEqual(writer_status()["writer"], "native")

    def test_force_plugin_overrides_native_live(self):
        import os
        from unittest import mock

        with mock.patch.dict(
            os.environ,
            {NATIVE_LIVE_ENV: "1", FORCE_PLUGIN_ENV: "1"},
            clear=False,
        ):
            self.assertFalse(native_attachment_is_live())
            self.assertEqual(writer_status()["writer"], "plugin-forced")

    def test_marker_file_marks_native_live(self):
        import os
        import tempfile
        from unittest import mock

        with tempfile.TemporaryDirectory() as tmp, mock.patch.dict(
            os.environ,
            {
                "XDG_STATE_HOME": tmp,
                "CMUX_SURFACE_ID": "surface-1",
                "HERDR_SOCKET_PATH": "/tmp/herdr.sock",
            },
            clear=False,
        ):
            from bridge.cmux_herdr_bridge import native_live_marker_path

            path = native_live_marker_path()
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, "w", encoding="utf-8") as handle:
                handle.write("live\n")
            self.assertTrue(native_attachment_is_live())

    def test_set_title_lock_round_trip(self):
        import os
        import tempfile
        from unittest import mock

        with tempfile.TemporaryDirectory() as tmp, mock.patch.dict(
            os.environ,
            {
                "XDG_STATE_HOME": tmp,
                "CMUX_SURFACE_ID": "surface-1",
                "HERDR_SOCKET_PATH": "/tmp/herdr.sock",
            },
            clear=False,
        ):
            locked = set_title_lock("w2:p1", locked=True, title="Orchestrator")
            self.assertTrue(locked["title_lock"])
            self.assertEqual(locked["locked_title"], "Orchestrator")
            state = _load_association_map()
            self.assertTrue(state["panes"]["w2:p1"]["title_lock"])
            unlocked = set_title_lock("w2:p1", locked=False)
            self.assertFalse(unlocked["title_lock"])
            self.assertNotIn("locked_title", unlocked)


if __name__ == "__main__":
    unittest.main()

