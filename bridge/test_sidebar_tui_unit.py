#!/usr/bin/env python3
"""Unit tests for the plugin-manager sidebar TUI."""

from __future__ import annotations

import io
import json
import socket
import tempfile
import threading
import unittest
from pathlib import Path
from typing import Any, Dict, List

from bridge.cmux_herdr_sidebar import (
    MuxClient,
    MuxSocketError,
    render_sidebar,
    run_sidebar,
    socket_path_from_env,
    workspaces_from_tree,
)

LAB_TREE = {
    "workspace_revision": 1,
    "workspaces": [
        {
            "id": 4,
            "key": "6ba7b810-9dad-41d1-80b4-00c04fd430c8",
            "name": "lab-west",
            "active": True,
            "screens": [],
        },
        {
            "id": 7,
            "name": "sandbox",
            "active": False,
            "screens": [],
        },
    ],
}


class WorkspaceParseTests(unittest.TestCase):
    def test_reads_live_workspaces_without_inventing_team(self) -> None:
        rows = workspaces_from_tree(LAB_TREE)
        self.assertEqual([row["name"] for row in rows], ["lab-west", "sandbox"])
        self.assertTrue(rows[0]["active"])
        self.assertFalse(rows[1]["active"])
        self.assertNotIn("team", json.dumps(rows))

    def test_empty_or_malformed_tree_is_empty(self) -> None:
        self.assertEqual(workspaces_from_tree(None), [])
        self.assertEqual(workspaces_from_tree({"workspaces": "nope"}), [])
        self.assertEqual(workspaces_from_tree({"workspaces": [{"name": "x"}]}), [])

    def test_socket_env_prefers_documented_names(self) -> None:
        self.assertEqual(
            socket_path_from_env({"CMUX_TUI_SOCKET": "/tmp/lab.sock"}),
            "/tmp/lab.sock",
        )
        self.assertEqual(
            socket_path_from_env({"CMUX_MUX_SOCKET": "/tmp/legacy.sock"}),
            "/tmp/legacy.sock",
        )
        self.assertIsNone(socket_path_from_env({}))


class RenderTests(unittest.TestCase):
    def test_inherit_host_theme_no_custom_green(self) -> None:
        rows = workspaces_from_tree(LAB_TREE)
        frame = render_sidebar(rows, 0, cols=28, rows_h=12, connected=True)
        self.assertIn("lab-west", frame)
        self.assertIn("sandbox", frame)
        self.assertIn("\x1b[7m", frame)
        self.assertNotIn("\x1b[32m", frame)
        self.assertNotIn("#00", frame.lower())
        self.assertNotIn("team", frame.lower())

    def test_reconnect_when_socket_missing(self) -> None:
        frame = render_sidebar(
            [],
            0,
            cols=32,
            rows_h=10,
            connected=False,
            message="CMUX_TUI_SOCKET is unset",
        )
        self.assertIn("waiting for mux socket", frame)
        self.assertIn("retrying", frame)
        self.assertNotIn("Acme", frame)
        self.assertNotIn("Engineering", frame)


class FakeClient:
    def __init__(self, path: str) -> None:
        self.path = path
        self.closed = False
        self.selected = None

    def list_workspaces(self) -> List[Dict[str, Any]]:
        return workspaces_from_tree(LAB_TREE)

    def select_workspace(self, index: int) -> None:
        self.selected = index

    def close(self) -> None:
        self.closed = True


class RunOnceTests(unittest.TestCase):
    def test_once_draws_live_rows_from_socket(self) -> None:
        out = io.StringIO()
        clients: List[FakeClient] = []

        def factory(path: str) -> FakeClient:
            client = FakeClient(path)
            clients.append(client)
            return client

        code = run_sidebar(
            {"CMUX_TUI_SOCKET": "/tmp/lab-cmux.sock"},
            stdout=out,
            client_factory=factory,
            once=True,
        )
        self.assertEqual(code, 0)
        self.assertIn("lab-west", out.getvalue())
        self.assertTrue(clients[0].closed)

    def test_once_does_not_invent_rows_without_socket(self) -> None:
        out = io.StringIO()
        code = run_sidebar({}, stdout=out, once=True)
        self.assertEqual(code, 0)
        self.assertIn("waiting for mux socket", out.getvalue())
        self.assertNotIn("lab-west", out.getvalue())


class MuxClientTests(unittest.TestCase):
    def test_json_lines_identify_and_list_workspaces(self) -> None:
        tmp = tempfile.mkdtemp()
        path = str(Path(tmp) / "mux.sock")
        server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        server.bind(path)
        server.listen(1)
        errors: List[BaseException] = []

        def serve() -> None:
            conn, _ = server.accept()
            try:
                buf = b""
                for expected_cmd, reply in (
                    (
                        "identify",
                        {"id": 1, "ok": True, "data": {"app": "cmux-tui", "protocol": 12}},
                    ),
                    ("list-workspaces", {"id": 2, "ok": True, "data": LAB_TREE}),
                ):
                    while b"\n" not in buf:
                        chunk = conn.recv(4096)
                        if not chunk:
                            raise MuxSocketError("client closed")
                        buf += chunk
                    line, buf = buf.split(b"\n", 1)
                    request = json.loads(line.decode("utf-8"))
                    if request["cmd"] != expected_cmd:
                        raise AssertionError(request)
                    conn.sendall((json.dumps(reply) + "\n").encode("utf-8"))
            except BaseException as exc:  # noqa: BLE001 — test helper
                errors.append(exc)
            finally:
                conn.close()
                server.close()

        thread = threading.Thread(target=serve)
        thread.start()
        try:
            client = MuxClient(path, timeout=2.0)
            rows = client.list_workspaces()
            client.close()
        finally:
            thread.join(timeout=2.0)
        self.assertFalse(errors)
        self.assertEqual([row["name"] for row in rows], ["lab-west", "sandbox"])


if __name__ == "__main__":
    unittest.main()
