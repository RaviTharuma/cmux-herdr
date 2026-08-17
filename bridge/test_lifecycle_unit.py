#!/usr/bin/env python3
"""Unit tests for cmux-tmux attach/detach/restore mapped onto Herdr."""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

_ROOT = Path(__file__).resolve().parents[1]
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from bridge.cmux_herdr_lifecycle import (
    POST_APPLY_CLIENT_SIZE,
    POST_RESEED,
    SETTING_KEY,
    SOCKET_METHODS,
    TEARDOWN_EXPLICIT_DETACH,
    TEARDOWN_SESSION_ENDED,
    AttachRegistry,
    AttachWindowTarget,
    ConnectionRecord,
    DiscoveredSession,
    LifecycleController,
    MirrorRecord,
    RestoreRecord,
    connection_action,
    decode_beta,
    dispatch,
    endpoint_hash,
    existing_mirror_window,
    grid_match,
    host_close_policy,
    may_cache_connection,
    note_output,
    pane_grid_payload,
    plan_attach,
    plan_restore,
    post_attach_action,
    purge_dead_mirrors,
    read_restore,
    session_payload,
    validate_session_name,
    validate_socket_path,
    window_target_from_params,
    write_restore,
)

SOCKET = "/tmp/herdr.sock"
SESSION = DiscoveredSession("sess-1", "main", window_count=2)


def _plan(target, **kwargs):
    defaults = dict(
        enabled=True,
        app_ready=True,
        already_attaching=False,
        existing_mirror_window_id=None,
        active_window_id="win-active",
        live_windows=("win-active", "win-other"),
        sessions=(SESSION,),
    )
    defaults.update(kwargs)
    return plan_attach(target, **defaults)


class SocketValidationTests(unittest.TestCase):
    def test_accepts_absolute_path(self) -> None:
        self.assertEqual(validate_socket_path("  /tmp/herdr.sock  "), SOCKET)

    def test_rejects_relative_dash_and_control(self) -> None:
        self.assertIsNone(validate_socket_path("tmp/herdr.sock"))
        self.assertIsNone(validate_socket_path("-oProxyCommand=x"))
        self.assertIsNone(validate_socket_path("/tmp/herdr\x1bsock"))
        self.assertIsNone(validate_socket_path(""))
        self.assertIsNone(validate_session_name("  "))
        self.assertEqual(validate_session_name("  main  "), "main")

    def test_endpoint_hash_is_stable_and_short(self) -> None:
        self.assertEqual(endpoint_hash(SOCKET), endpoint_hash(SOCKET))
        self.assertEqual(len(endpoint_hash(SOCKET)), 16)
        self.assertNotEqual(endpoint_hash(SOCKET), endpoint_hash("/tmp/other.sock"))


class BetaFlagTests(unittest.TestCase):
    def test_catalog_key_matches_tmux_shape(self) -> None:
        self.assertEqual(SETTING_KEY, "betaFeatures.remoteHerdrMirror")

    def test_decode_defaults_and_strings(self) -> None:
        self.assertFalse(decode_beta(None))
        self.assertTrue(decode_beta("on"))
        self.assertFalse(decode_beta("off"))
        self.assertTrue(decode_beta(1))
        self.assertFalse(decode_beta("maybe", default=False))


class WindowTargetTests(unittest.TestCase):
    def test_existing_mirror_affinity_beats_explicit(self) -> None:
        target = AttachWindowTarget(kind="explicit", window_id="win-other")
        resolved = target.resolve("win-active", "win-other", lambda wid: True)
        self.assertEqual(resolved, "win-active")

    def test_explicit_dead_window_fails_closed(self) -> None:
        target = AttachWindowTarget(kind="explicit", window_id="win-dead")
        self.assertIsNone(target.resolve(None, "win-active", lambda wid: wid == "win-active"))

    def test_unresolved_explicit_never_falls_back(self) -> None:
        target = AttachWindowTarget(kind="unresolved_explicit")
        self.assertIsNone(target.resolve(None, "win-active", lambda wid: True))

    def test_contextual_falls_back_to_active(self) -> None:
        target = AttachWindowTarget(kind="contextual", window_id="win-dead")
        resolved = target.resolve(
            None, "win-active", lambda wid: wid == "win-active"
        )
        self.assertEqual(resolved, "win-active")

    def test_dedicated_does_not_resolve(self) -> None:
        target = AttachWindowTarget(kind="dedicated_new_window")
        self.assertIsNone(target.resolve(None, "win-active", lambda wid: True))

    def test_params_null_window_id_is_unresolved(self) -> None:
        target = window_target_from_params({"window_id": None})
        self.assertEqual(target.kind, "unresolved_explicit")
        dedicated = window_target_from_params({"window_id": "x"}, dedicated=True)
        self.assertEqual(dedicated.kind, "dedicated_new_window")


class AttachPlanTests(unittest.TestCase):
    def test_preflight_rejects_before_discovery(self) -> None:
        plan = _plan(
            AttachWindowTarget(kind="unresolved_explicit"),
            sessions=None,
        )
        self.assertEqual(plan.outcome, "invalid_target")

    def test_disabled_and_unready_and_reentrant(self) -> None:
        target = AttachWindowTarget(kind="contextual")
        self.assertEqual(_plan(target, enabled=False).outcome, "disabled")
        self.assertEqual(_plan(target, app_ready=False).outcome, "unreachable")
        self.assertEqual(_plan(target, already_attaching=True).outcome, "already_attaching")

    def test_empty_discovery_does_not_create_chrome(self) -> None:
        plan = _plan(
            AttachWindowTarget(kind="dedicated_new_window"),
            sessions=(),
        )
        self.assertEqual(plan.outcome, "no_sessions")
        self.assertFalse(plan.create_window)

    def test_dedicated_creates_only_after_sessions(self) -> None:
        plan = _plan(AttachWindowTarget(kind="dedicated_new_window"))
        self.assertEqual(plan.outcome, "mirrored")
        self.assertTrue(plan.create_window)
        self.assertTrue(plan.discard_window_on_fail)
        self.assertEqual(plan.sessions_to_mirror, ("sess-1",))
        self.assertEqual(plan.post_attach, POST_APPLY_CLIENT_SIZE)

    def test_failed_empty_discards_dedicated_window(self) -> None:
        plan = _plan(
            AttachWindowTarget(kind="dedicated_new_window"),
            mirrored_workspace_ids=(),
        )
        self.assertEqual(plan.outcome, "failed_empty")
        self.assertTrue(plan.discard_window_on_fail)

    def test_reuse_live_connection(self) -> None:
        plan = _plan(
            AttachWindowTarget(kind="contextual"),
            live_session_ids=("sess-1",),
            mirrors=(MirrorRecord("sess-1", "win-active", "ws-1"),),
        )
        self.assertEqual(plan.outcome, "reused")
        self.assertEqual(plan.sessions_to_reuse, ("sess-1",))
        self.assertEqual(plan.sessions_to_mirror, ())

    def test_dead_mirror_is_purged_and_remirrored(self) -> None:
        plan = _plan(
            AttachWindowTarget(kind="contextual"),
            mirrors=(MirrorRecord("sess-1", "win-active", None),),
            live_session_ids=("sess-1",),
        )
        self.assertEqual(plan.purge_session_ids, ("sess-1",))
        self.assertEqual(plan.sessions_to_mirror, ("sess-1",))
        self.assertEqual(plan.outcome, "mirrored")


class ConnectionTests(unittest.TestCase):
    def test_reuse_replace_start(self) -> None:
        self.assertEqual(connection_action(None), "start")
        live = ConnectionRecord("s", started=True)
        self.assertEqual(connection_action(live), "reuse")
        dead = ConnectionRecord("s", started=True, exited=True)
        self.assertEqual(connection_action(dead), "replace")

    def test_never_cache_unstarted(self) -> None:
        self.assertFalse(may_cache_connection(ConnectionRecord("s", started=False)))
        self.assertFalse(
            may_cache_connection(ConnectionRecord("s", started=True, exited=True))
        )
        self.assertTrue(may_cache_connection(ConnectionRecord("s", started=True)))

    def test_post_attach_reseed_vs_size(self) -> None:
        self.assertEqual(post_attach_action(replaced_dead=True), POST_RESEED)
        self.assertEqual(post_attach_action(replaced_dead=False), POST_APPLY_CLIENT_SIZE)


class RegistryAndPurgeTests(unittest.TestCase):
    def test_reentrant_guard(self) -> None:
        registry = AttachRegistry()
        self.assertTrue(registry.begin_attach("abc"))
        self.assertFalse(registry.begin_attach("abc"))
        registry.end_attach("abc")
        self.assertTrue(registry.begin_attach("abc"))

    def test_purge_dead_unblocks_reattach(self) -> None:
        kept = purge_dead_mirrors(
            [
                MirrorRecord("dead", "win-a", None),
                MirrorRecord("live", "win-a", "ws-live"),
            ]
        )
        self.assertEqual([item.session_id for item in kept], ["live"])
        self.assertEqual(
            existing_mirror_window(kept, ("win-a",)),
            "win-a",
        )


class DetachPolicyTests(unittest.TestCase):
    def test_every_host_close_detaches_never_kills(self) -> None:
        for source in (
            "last_workspace_tab",
            "window_quit",
            "app_terminate",
            "explicit_detach",
            "host_tab",
        ):
            self.assertEqual(host_close_policy(source), "detach")
        self.assertEqual(host_close_policy("unknown"), "noop")

    def test_controller_detach_leaves_server(self) -> None:
        ctl = LifecycleController()
        result = ctl.attach(
            SOCKET,
            (SESSION,),
            AttachWindowTarget(kind="contextual"),
        )
        self.assertTrue(result["ok"])
        detached = ctl.detach("sess-1", reason=TEARDOWN_EXPLICIT_DETACH)
        self.assertTrue(detached["ok"])
        self.assertFalse(detached["server_stopped"])
        self.assertFalse(ctl.state("sess-1")["attached"])
        ended = ctl.detach("sess-1", reason=TEARDOWN_SESSION_ENDED)
        self.assertEqual(ended["reason"], TEARDOWN_SESSION_ENDED)

    def test_last_tab_close_detaches_all(self) -> None:
        ctl = LifecycleController()
        ctl.attach(SOCKET, (SESSION,), AttachWindowTarget(kind="contextual"))
        closed = ctl.close_host("last_workspace_tab")
        self.assertEqual(closed["outcome"], "detach")
        self.assertFalse(closed["server_stopped"])
        self.assertEqual(ctl.mirrors, {})


class ControllerAttachTests(unittest.TestCase):
    def test_attach_reuses_then_rejects_reentrant(self) -> None:
        ctl = LifecycleController()
        first = ctl.attach(
            SOCKET, (SESSION,), AttachWindowTarget(kind="contextual")
        )
        self.assertEqual(first["outcome"], "mirrored")
        self.assertEqual(first["post_attach"], POST_APPLY_CLIENT_SIZE)
        second = ctl.attach(
            SOCKET, (SESSION,), AttachWindowTarget(kind="contextual")
        )
        self.assertEqual(second["outcome"], "reused")
        ctl.registry.begin_attach(endpoint_hash(SOCKET))
        locked = ctl.attach(
            SOCKET, (SESSION,), AttachWindowTarget(kind="contextual")
        )
        self.assertEqual(locked["outcome"], "already_attaching")

    def test_one_endpoint_stays_in_one_window(self) -> None:
        ctl = LifecycleController()
        ctl.attach(SOCKET, (SESSION,), AttachWindowTarget(kind="contextual"))
        other = ctl.attach(
            SOCKET,
            (SESSION, DiscoveredSession("sess-2", "two")),
            AttachWindowTarget(kind="explicit", window_id="win-other"),
        )
        self.assertTrue(other["ok"])
        windows = {record.window_id for record in ctl.mirrors.values()}
        self.assertEqual(windows, {"win-active"})

    def test_dedicated_moves_existing_mirrors(self) -> None:
        ctl = LifecycleController()
        ctl.attach(SOCKET, (SESSION,), AttachWindowTarget(kind="contextual"))
        moved = ctl.attach(
            SOCKET,
            (SESSION,),
            AttachWindowTarget(kind="dedicated_new_window"),
        )
        self.assertTrue(moved["ok"])
        self.assertTrue(str(moved["window_id"]).startswith("win-new-"))
        self.assertEqual(ctl.mirrors["sess-1"].window_id, moved["window_id"])

    def test_activate_selects_window(self) -> None:
        ctl = LifecycleController()
        result = ctl.attach(
            SOCKET,
            (SESSION,),
            AttachWindowTarget(kind="dedicated_new_window"),
            activate=True,
        )
        self.assertEqual(ctl.active_window_id, result["window_id"])

    def test_disabled_controller_rejects(self) -> None:
        ctl = LifecycleController(enabled=False)
        result = ctl.attach(
            SOCKET, (SESSION,), AttachWindowTarget(kind="contextual")
        )
        self.assertEqual(result["outcome"], "disabled")


class RestoreTests(unittest.TestCase):
    def test_restore_reattaches_not_replay_tree(self) -> None:
        record = RestoreRecord(
            endpoint_hash=endpoint_hash(SOCKET),
            socket_path=SOCKET,
            session_ids=("sess-1",),
            target_kind="explicit",
            window_id="win-stale",
        )
        plan = plan_restore(
            record,
            enabled=True,
            app_ready=True,
            sessions=(SESSION,),
            live_windows=("win-active",),
            active_window_id="win-active",
        )
        self.assertEqual(plan.outcome, "mirrored")
        self.assertEqual(plan.post_attach, POST_RESEED)
        self.assertEqual(plan.reason, "restore_reattach")
        self.assertNotEqual(plan.reason, "replay_tree")

    def test_restore_keeps_dedicated_intent(self) -> None:
        record = RestoreRecord(
            endpoint_hash="x",
            socket_path=SOCKET,
            session_ids=("sess-1",),
            target_kind="dedicated_new_window",
        )
        plan = plan_restore(
            record,
            enabled=True,
            app_ready=True,
            sessions=(SESSION,),
            live_windows=("win-active",),
        )
        self.assertTrue(plan.create_window)
        self.assertEqual(plan.post_attach, POST_RESEED)

    def test_restore_disabled_keeps_persist(self) -> None:
        ctl = LifecycleController()
        ctl.attach(SOCKET, (SESSION,), AttachWindowTarget(kind="contextual"))
        persist = ctl.persist
        ctl.enabled = False
        result = ctl.restore((SESSION,))
        self.assertEqual(result["outcome"], "disabled")
        self.assertEqual(ctl.persist, persist)

    def test_controller_restore_after_clear_mirrors(self) -> None:
        ctl = LifecycleController()
        ctl.attach(SOCKET, (SESSION,), AttachWindowTarget(kind="contextual"))
        ctl.mirrors.clear()
        ctl.connections.clear()
        result = ctl.restore((SESSION,))
        self.assertTrue(result["ok"])
        self.assertEqual(result["post_attach"], POST_RESEED)
        self.assertEqual(result["mode"], "reattach")
        self.assertIn("sess-1", ctl.mirrors)

    def test_persist_round_trip_rejects_replay_tree(self) -> None:
        record = RestoreRecord(
            endpoint_hash="abc",
            socket_path=SOCKET,
            session_ids=("sess-1",),
            target_kind="contextual",
        )
        self.assertEqual(record.to_dict()["mode"], "reattach")
        self.assertIsNone(RestoreRecord.from_dict({"mode": "replay_tree"}))
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "restore.json"
            write_restore(path, record)
            loaded = read_restore(path)
            self.assertIsNotNone(loaded)
            assert loaded is not None
            self.assertEqual(loaded.session_ids, ("sess-1",))
            self.assertIsNone(read_restore(path.with_name("missing.json")))


class ObservabilityTests(unittest.TestCase):
    def test_socket_methods_twin_tmux(self) -> None:
        self.assertIn("remote.herdr.attach", SOCKET_METHODS)
        self.assertIn("remote.herdr.pane_grids", SOCKET_METHODS)
        self.assertEqual(len(SOCKET_METHODS), 8)

    def test_dispatch_gates_and_validates(self) -> None:
        self.assertEqual(
            dispatch("remote.herdr.sessions", {"socket": SOCKET}, enabled=False)["code"],
            "disabled",
        )
        self.assertEqual(
            dispatch("remote.tmux.attach", {"socket": SOCKET}, enabled=True)["code"],
            "unknown_method",
        )
        self.assertEqual(
            dispatch("remote.herdr.attach", {"socket": SOCKET}, enabled=True)["code"],
            "invalid_params",
        )
        ok = dispatch(
            "remote.herdr.attach",
            {"socket": SOCKET, "session": "main", "activate": True},
            enabled=True,
        )
        self.assertTrue(ok["ok"])
        self.assertTrue(ok["activate"])
        self.assertEqual(ok["session"], "main")
        window = dispatch(
            "remote.herdr.window",
            {"socket_path": SOCKET},
            enabled=True,
        )
        self.assertEqual(window["target"].kind, "dedicated_new_window")

    def test_session_and_state_payloads(self) -> None:
        self.assertEqual(
            session_payload(SESSION),
            {"id": "sess-1", "name": "main", "windows": 2, "attached": False},
        )
        ctl = LifecycleController()
        ctl.attach(SOCKET, (SESSION,), AttachWindowTarget(kind="contextual"))
        note_output(ctl.connections["sess-1"], "w2:p1", 12)
        state = ctl.state("sess-1")
        self.assertTrue(state["attached"])
        self.assertEqual(state["total_output_bytes"], 12)
        self.assertEqual(state["pane_output_bytes"]["w2:p1"], 12)

    def test_pane_grids_match_contract(self) -> None:
        self.assertTrue(
            grid_match(80, 24, 80, 24, exact_cols=True, exact_rows=True)
        )
        self.assertFalse(
            grid_match(80, 24, 79, 24, exact_cols=True, exact_rows=False)
        )
        self.assertTrue(
            grid_match(80, 24, 80, 30, exact_cols=True, exact_rows=False)
        )
        payload = pane_grid_payload(
            "w2:t1",
            [
                {
                    "pane_id": "w2:p1",
                    "assigned_cols": 80,
                    "assigned_rows": 24,
                    "rendered_cols": 80,
                    "rendered_rows": 24,
                    "exact_cols": True,
                    "exact_rows": True,
                }
            ],
            base_cols=80,
            base_rows=24,
            pushed=(80, 24),
        )
        self.assertTrue(payload["panes"][0]["match"])
        self.assertEqual(payload["pushed"]["cols"], 80)


if __name__ == "__main__":
    unittest.main()
