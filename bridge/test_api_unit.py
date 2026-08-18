#!/usr/bin/env python3
"""Allowlisted Herdr socket API: refuse server.stop, socket then CLI."""

from __future__ import annotations

import os
import unittest
from typing import Any, Dict, List, Sequence

from bridge.cmux_herdr_api import (
    ALLOWED_METHODS,
    FORBIDDEN_METHODS,
    ApiError,
    ApiResult,
    HerdrApi,
    assert_method_allowed,
    build_cli_argv,
    extract_agent_status,
    extract_read_text,
)
from bridge.cmux_herdr_socket import HerdrSocketError


class _FakeClient:
    def __init__(self, result: Any = None, error: str = "") -> None:
        self.result = result
        self.error = error
        self.calls: List[tuple] = []
        self._open = True

    @property
    def connected(self) -> bool:
        return self._open

    def close(self) -> None:
        self._open = False

    def request(self, method: str, params: Dict[str, Any]) -> Any:
        self.calls.append((method, params))
        if self.error:
            raise HerdrSocketError(self.error)
        return self.result


class AllowlistTests(unittest.TestCase):
    def test_server_stop_is_forbidden(self) -> None:
        with self.assertRaises(ApiError) as ctx:
            assert_method_allowed("server.stop")
        self.assertIn("server.stop", str(ctx.exception))
        self.assertIn("server.stop", FORBIDDEN_METHODS)
        self.assertNotIn("server.stop", ALLOWED_METHODS)

    def test_graphics_and_plugin_are_forbidden(self) -> None:
        for method in (
            "pane.graphics.set",
            "pane.graphics.stream",
            "plugin.link",
            "plugin.action.invoke",
        ):
            with self.assertRaises(ApiError, msg=method):
                assert_method_allowed(method)

    def test_published_control_methods_are_allowed(self) -> None:
        for method in (
            "pane.close",
            "pane.zoom",
            "pane.resize",
            "pane.send_text",
            "tab.create",
            "tab.close",
            "workspace.create",
            "agent.prompt",
            "agent.wait",
            "layout.export",
            "session.snapshot",
        ):
            self.assertEqual(assert_method_allowed(method), method)

    def test_unknown_method_is_rejected(self) -> None:
        with self.assertRaises(ApiError):
            assert_method_allowed("not.a.method")


class ExtractorTests(unittest.TestCase):
    def test_extract_read_text_from_nested_result(self) -> None:
        self.assertEqual(
            extract_read_text({"result": {"text": "hello\nworld"}}),
            "hello\nworld",
        )
        self.assertEqual(extract_read_text({"lines": ["a", "b"]}), "a\nb")
        self.assertEqual(extract_read_text("plain"), "plain")

    def test_extract_agent_status(self) -> None:
        self.assertEqual(
            extract_agent_status({"pane": {"agent_status": "working"}}),
            "working",
        )
        self.assertIsNone(extract_agent_status({"pane_id": "w1:p1"}))


class CliArgvTests(unittest.TestCase):
    def test_high_value_verbs_map_to_herdr_cli(self) -> None:
        self.assertEqual(
            build_cli_argv("tab.create", {"label": "logs"}),
            ["tab", "create", "--label", "logs"],
        )
        self.assertEqual(
            build_cli_argv("pane.close", {"pane_id": "w1:p1", "force": True}),
            ["pane", "close", "w1:p1", "--force"],
        )
        self.assertEqual(
            build_cli_argv(
                "pane.resize",
                {"pane_id": "w1:p1", "direction": "right", "amount": 0.1},
            ),
            [
                "pane",
                "resize",
                "w1:p1",
                "--direction",
                "right",
                "--amount",
                "0.1",
            ],
        )
        self.assertEqual(
            build_cli_argv("agent.wait", {"pane_id": "w1:p1", "until": "done"}),
            ["agent", "wait", "w1:p1", "--until", "done"],
        )
        self.assertIsNone(build_cli_argv("layout.set_split_ratio", {"ratio": 0.5}))


class HerdrApiCallTests(unittest.TestCase):
    def test_socket_success_does_not_touch_cli(self) -> None:
        client = _FakeClient(result={"type": "pane_close", "changed": True})
        calls: List[Sequence[str]] = []

        def cli(argv: Sequence[str]) -> Any:
            calls.append(argv)
            raise AssertionError("CLI must not run")

        api = HerdrApi(client=client, cli_runner=cli)  # type: ignore[arg-type]
        result = api.call("pane.close", {"pane_id": "w1:p1"})
        self.assertTrue(result.ok)
        self.assertEqual(result.via, "socket")
        self.assertEqual(client.calls, [("pane.close", {"pane_id": "w1:p1"})])
        self.assertEqual(calls, [])

    def test_cli_fallback_when_socket_fails(self) -> None:
        client = _FakeClient(error="connect failed")

        def cli(argv: Sequence[str]) -> Any:
            self.assertEqual(argv, ["pane", "close", "w1:p1"])
            return {"type": "pane_close"}

        api = HerdrApi(client=client, cli_runner=cli)  # type: ignore[arg-type]
        result = api.call("pane.close", {"pane_id": "w1:p1"})
        self.assertEqual(result.via, "cli")
        self.assertEqual(result.result["type"], "pane_close")

    def test_forbidden_never_reaches_socket(self) -> None:
        client = _FakeClient(result={"nope": True})
        api = HerdrApi(client=client)  # type: ignore[arg-type]
        with self.assertRaises(ApiError):
            api.call("server.stop", {})
        self.assertEqual(client.calls, [])

    def test_socket_only_does_not_fall_back(self) -> None:
        client = _FakeClient(error="down")
        api = HerdrApi(client=client, cli_runner=lambda argv: {"ok": True})  # type: ignore[arg-type]
        with self.assertRaises(ApiError):
            api.call("pane.close", {"pane_id": "w1:p1"}, socket_only=True)

    def test_result_to_dict(self) -> None:
        payload = ApiResult(
            ok=False, method="pane.close", via="socket", error="missing"
        ).to_dict()
        self.assertEqual(payload["error"], "missing")
        self.assertFalse(payload["ok"])

    def test_persistent_session_reuses_one_connection(self) -> None:
        from bridge.test_socket_unit import FakeHerdrUnixServer

        def handler(request: Dict[str, Any]) -> List[Dict[str, Any]]:
            return [{"id": request.get("id"), "result": {"type": "pong"}}]

        server = FakeHerdrUnixServer(handler)
        self.addCleanup(server.shutdown)
        with HerdrApi(socket_path=server.path, timeout=2.0) as api:
            first = api.call("ping")
            second = api.call("ping")
        self.assertTrue(first.ok)
        self.assertTrue(second.ok)
        self.assertEqual(first.via, "socket")
        self.assertEqual(server.accepts, 1)

    def test_injected_client_is_not_closed_by_api_close(self) -> None:
        client = _FakeClient(result={"type": "pong"})
        api = HerdrApi(client=client)  # type: ignore[arg-type]
        api.close()
        self.assertTrue(client.connected)
        result = api.call("ping")
        self.assertEqual(result.via, "socket")

    def test_cli_fallback_invokes_herdr_once(self) -> None:
        from types import SimpleNamespace
        from unittest import mock

        fake = SimpleNamespace(returncode=1, stderr="agent focus denied", stdout="")
        with mock.patch.dict(
            os.environ, {"HERDR_SOCKET_PATH": "/no/such/herdr.sock"}, clear=False
        ), mock.patch(
            "bridge.cmux_herdr_bridge.which", return_value="/mock/herdr"
        ), mock.patch(
            "bridge.cmux_herdr_bridge.run_cmd", return_value=fake
        ) as run_cmd:
            api = HerdrApi()
            with self.assertRaises(ApiError) as ctx:
                api.call("agent.focus", {"pane_id": "w2:p1"})
            self.assertIn("agent focus denied", str(ctx.exception))
            self.assertEqual(run_cmd.call_count, 1)

    def test_tab_move_and_pane_rename_map_to_cli(self) -> None:
        self.assertEqual(
            build_cli_argv("tab.move", {"tab_id": "w2:t1", "index": 2}),
            ["tab", "move", "w2:t1", "--index", "2"],
        )
        self.assertEqual(
            build_cli_argv("pane.rename", {"pane_id": "w2:p1", "label": "logs"}),
            ["pane", "rename", "w2:p1", "logs"],
        )


if __name__ == "__main__":
    unittest.main()
