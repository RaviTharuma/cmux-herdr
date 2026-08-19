#!/usr/bin/env python3
"""Plugin ↔ native writer lease and shared restore."""

from __future__ import annotations

import json
import os
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

from bridge.cmux_herdr_handoff import (
    FORCE_PLUGIN_ENV,
    NATIVE_LIVE_ENV,
    NATIVE_STATE_ENV,
    OUTCOME_NATIVE_OWNS,
    OWNER_NATIVE,
    OWNER_PLUGIN,
    WriterLease,
    claim_native_writer,
    claim_plugin_writer,
    clear_shared_restore,
    heartbeat_native_writer,
    heartbeat_plugin_writer,
    observe_foreign,
    parse_lease_text,
    pid_alive,
    read_shared_restore,
    release_native_writer,
    release_plugin_writer,
    resolve_writer,
    write_lease,
    write_shared_restore,
    writer_status,
)
from bridge.cmux_herdr_live import attach_live, restore_live
from bridge.cmux_herdr_lifecycle import DiscoveredSession
from bridge.test_live_unit import _window


class HandoffLeaseTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.home = Path(self.tmp.name)
        self.state = self.home / "state"
        self.native = self.home / "native-state"
        self.native.mkdir()
        self.env = {
            "XDG_STATE_HOME": str(self.state),
            "HOME": str(self.home),
            NATIVE_STATE_ENV: str(self.native),
            "CMUX_SURFACE_ID": "surface-1",
            "HERDR_SOCKET_PATH": "/tmp/herdr.sock",
            "HERDR_WORKSPACE_ID": "w1",
        }
        self.env.pop(NATIVE_LIVE_ENV, None)
        self.patch = mock.patch.dict(os.environ, self.env, clear=False)
        self.patch.start()
        os.environ.pop(NATIVE_LIVE_ENV, None)
        os.environ.pop(FORCE_PLUGIN_ENV, None)

    def tearDown(self) -> None:
        self.patch.stop()
        self.tmp.cleanup()

    def test_legacy_one_file_is_native_while_fresh(self) -> None:
        path = self.state / "cmux-herdr" / "native-live-host"
        path.parent.mkdir(parents=True)
        path.write_text("1\n", encoding="utf-8")
        lease = parse_lease_text("1\n", path=path, fallback_owner=OWNER_NATIVE)
        self.assertIsNotNone(lease)
        assert lease is not None
        self.assertEqual(lease.owner, OWNER_NATIVE)
        self.assertTrue(lease.is_fresh())

    def test_stale_mtime_legacy_marker_does_not_block_plugin(self) -> None:
        path = self.state / "cmux-herdr" / "native-live-fp"
        path.parent.mkdir(parents=True)
        path.write_text("live\n", encoding="utf-8")
        old = time.time() - 3_600
        os.utime(path, (old, old))
        decision = resolve_writer("fp")
        self.assertFalse(decision.native_live)
        self.assertTrue(decision.lease_stale)
        self.assertEqual(decision.writer, OWNER_PLUGIN)

    def test_dead_pid_is_immediately_stale(self) -> None:
        claim_native_writer("fp", pid=999_999_999)
        decision = resolve_writer("fp")
        self.assertFalse(decision.native_live)
        self.assertTrue(decision.lease_stale)

    def test_env_native_live_wins_until_force_plugin(self) -> None:
        with mock.patch.dict(os.environ, {NATIVE_LIVE_ENV: "1"}, clear=False):
            decision = resolve_writer("fp")
            self.assertTrue(decision.native_live)
            self.assertEqual(decision.writer, OWNER_NATIVE)
        with mock.patch.dict(
            os.environ,
            {NATIVE_LIVE_ENV: "1", FORCE_PLUGIN_ENV: "1"},
            clear=False,
        ):
            decision = resolve_writer("fp")
            self.assertFalse(decision.native_live)
            self.assertEqual(decision.writer, "plugin-forced")

    def test_plugin_claim_writes_json_native_can_read(self) -> None:
        lease = claim_plugin_writer("fp", socket_path="/tmp/herdr.sock")
        self.assertIsNotNone(lease)
        status = writer_status("fp")
        self.assertEqual(status["writer"], OWNER_PLUGIN)
        self.assertTrue(status["plugin_live"])
        self.assertFalse(status["native_live"])
        native_copy = self.native / "plugin-live-fp"
        self.assertTrue(native_copy.is_file())
        payload = json.loads(native_copy.read_text(encoding="utf-8"))
        self.assertEqual(payload["owner"], OWNER_PLUGIN)
        self.assertEqual(payload["schema"], 1)

    def test_native_yields_when_other_plugin_is_fresh(self) -> None:
        if not pid_alive(1):
            self.skipTest("pid 1 is not visible")
        write_lease(OWNER_PLUGIN, "fp", pid=1)
        self.assertIsNone(claim_native_writer("fp"))
        decision = resolve_writer("fp")
        self.assertTrue(decision.plugin_live)

    def test_plugin_yields_when_native_lease_is_fresh(self) -> None:
        claim_native_writer("fp", pid=os.getpid())
        self.assertIsNone(claim_plugin_writer("fp"))
        decision = resolve_writer("fp")
        self.assertTrue(decision.yields)
        self.assertEqual(decision.outcome, OUTCOME_NATIVE_OWNS)

    def test_heartbeat_keeps_plugin_fresh(self) -> None:
        claim_plugin_writer("fp")
        first = resolve_writer("fp").lease
        assert first is not None
        time.sleep(0.01)
        beat = heartbeat_plugin_writer("fp")
        self.assertIsNotNone(beat)
        assert beat is not None
        self.assertGreaterEqual(beat.heartbeat_ms, first.heartbeat_ms)

    def test_heartbeat_keeps_native_fresh(self) -> None:
        claim_native_writer("fp", pid=os.getpid())
        first = resolve_writer("fp").lease
        assert first is not None
        time.sleep(0.01)
        beat = heartbeat_native_writer("fp", pid=os.getpid())
        self.assertIsNotNone(beat)
        assert beat is not None
        self.assertGreaterEqual(beat.heartbeat_ms, first.heartbeat_ms)
        self.assertTrue(resolve_writer("fp").native_live)

    def test_release_plugin_does_not_drop_native(self) -> None:
        claim_native_writer("fp", pid=os.getpid())
        release_plugin_writer("fp")
        self.assertTrue(resolve_writer("fp").native_live)

    def test_shared_restore_rejects_replay_tree(self) -> None:
        with self.assertRaises(ValueError):
            write_shared_restore("abc", {"mode": "replay_tree", "socket_path": "/tmp/x"})
        hashed = "deadbeefdeadbeef"
        (self.state / "cmux-herdr").mkdir(parents=True, exist_ok=True)
        bad = self.state / "cmux-herdr" / f"restore-{hashed}.json"
        bad.write_text(
            json.dumps({"mode": "replay_tree", "socket_path": "/tmp/herdr.sock"}),
            encoding="utf-8",
        )
        self.assertIsNone(read_shared_restore(hashed))
        path = write_shared_restore(
            hashed,
            {
                "endpoint_hash": hashed,
                "socket_path": "/tmp/herdr.sock",
                "session_ids": ["main"],
                "target_kind": "contextual",
            },
        )
        payload = read_shared_restore(hashed)
        self.assertIsNotNone(payload)
        assert payload is not None
        self.assertEqual(payload["mode"], "reattach")
        self.assertTrue(clear_shared_restore(hashed))
        self.assertFalse(Path(path).is_file())

    def test_observe_foreign_does_not_invent_grids(self) -> None:
        claim_native_writer("fp", pid=os.getpid())
        decision = resolve_writer("fp")
        body = observe_foreign(decision, "remote.herdr.pane_surfaces")
        self.assertEqual(body["outcome"], OUTCOME_NATIVE_OWNS)
        self.assertEqual(body["panes"], [])
        self.assertFalse(body["server_stopped"])

    def test_attach_live_yields_to_native(self) -> None:
        with mock.patch.dict(os.environ, {NATIVE_LIVE_ENV: "1"}, clear=False):
            host, payload = attach_live(
                [_window()],
                [DiscoveredSession("sess-1", "main")],
                socket_path="/tmp/herdr.sock",
            )
        self.assertIsNone(host)
        self.assertTrue(payload["ok"])
        self.assertEqual(payload["outcome"], OUTCOME_NATIVE_OWNS)
        self.assertFalse(payload["server_stopped"])

    def test_restore_live_yields_to_native(self) -> None:
        with mock.patch.dict(os.environ, {NATIVE_LIVE_ENV: "1"}, clear=False):
            host, payload = restore_live(
                [_window()],
                [DiscoveredSession("sess-1", "main")],
                socket_path="/tmp/herdr.sock",
            )
        self.assertIsNone(host)
        self.assertEqual(payload["outcome"], OUTCOME_NATIVE_OWNS)
        self.assertEqual(payload["mode"], "reattach")

    def test_stale_native_lets_plugin_attach(self) -> None:
        path = self.state / "cmux-herdr" / "native-live"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("1\n", encoding="utf-8")
        old = time.time() - 3_600
        os.utime(path, (old, old))
        host, payload = attach_live(
            [_window()],
            [DiscoveredSession("sess-1", "main")],
            socket_path="/tmp/herdr.sock",
        )
        self.assertIsNotNone(host)
        self.assertTrue(payload["ok"])
        self.assertEqual(payload["writer"], OWNER_PLUGIN)
        assert host is not None
        host.detach()
        from bridge.cmux_herdr_bridge import _parent_key

        release_plugin_writer(_parent_key())


class WriterLeaseFreshnessTests(unittest.TestCase):
    def test_alive_pid_without_heartbeat_goes_stale(self) -> None:
        lease = WriterLease(
            owner=OWNER_NATIVE,
            pid=os.getpid(),
            heartbeat_ms=now_offset(-120_000),
            fingerprint="fp",
        )
        self.assertFalse(lease.is_fresh(ttl=45_000))

    def test_our_pid_with_recent_heartbeat_is_fresh(self) -> None:
        lease = WriterLease(
            owner=OWNER_PLUGIN,
            pid=os.getpid(),
            heartbeat_ms=now_offset(0),
            fingerprint="fp",
        )
        self.assertTrue(lease.is_fresh())


def now_offset(delta_ms: int) -> int:
    """Heartbeat timestamp relative to now."""
    return int(time.time() * 1000) + delta_ms


if __name__ == "__main__":
    unittest.main()
