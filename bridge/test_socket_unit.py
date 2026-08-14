#!/usr/bin/env python3
"""Unit tests for the persistent Herdr NDJSON socket client."""

from __future__ import annotations

import json
import os
import socket
import sys
import tempfile
import threading
import unittest
from pathlib import Path
from typing import Any, Callable, Dict, List

_ROOT = Path(__file__).resolve().parents[1]
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from bridge.cmux_herdr_socket import (
    HerdrEventSession,
    HerdrSocketClient,
    HerdrSocketError,
)


class FakeHerdrUnixServer:
    """Minimal protocol-17 NDJSON server for tests."""

    def __init__(
        self,
        handler: Callable[[Dict[str, Any]], List[Dict[str, Any]]],
    ) -> None:
        """Bind a temp Unix socket and serve ``handler`` on a daemon thread."""
        self.handler = handler
        self.accepts = 0
        fd, self.path = tempfile.mkstemp(suffix=".sock")
        os.close(fd)
        os.unlink(self.path)
        self._sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self._sock.bind(self.path)
        os.chmod(self.path, 0o600)
        self._sock.listen(8)
        self._sock.settimeout(0.2)
        self._running = True
        self._thread = threading.Thread(target=self._loop, daemon=True)
        self._thread.start()

    def shutdown(self) -> None:
        """Stop accepting and remove the socket file."""
        self._running = False
        try:
            self._sock.close()
        except OSError:
            pass
        try:
            os.unlink(self.path)
        except OSError:
            pass

    def _loop(self) -> None:
        while self._running:
            try:
                conn, _ = self._sock.accept()
            except socket.timeout:
                continue
            except OSError:
                return
            self.accepts += 1
            threading.Thread(target=self._serve, args=(conn,), daemon=True).start()

    def _serve(self, conn: socket.socket) -> None:
        buf = b""
        try:
            conn.settimeout(2.0)
            while self._running:
                try:
                    chunk = conn.recv(65536)
                except socket.timeout:
                    continue
                except OSError:
                    return
                if not chunk:
                    return
                buf += chunk
                while b"\n" in buf:
                    raw, buf = buf.split(b"\n", 1)
                    if not raw.strip():
                        continue
                    try:
                        request = json.loads(raw.decode("utf-8"))
                    except (json.JSONDecodeError, UnicodeDecodeError):
                        continue
                    if not isinstance(request, dict):
                        continue
                    for response in self.handler(request):
                        conn.sendall((json.dumps(response) + "\n").encode("utf-8"))
        finally:
            try:
                conn.close()
            except OSError:
                return


class HerdrSocketClientTests(unittest.TestCase):
    def test_ping_and_snapshot_round_trip(self) -> None:
        def handler(request: Dict[str, Any]) -> List[Dict[str, Any]]:
            req_id = request.get("id")
            method = request.get("method")
            if method == "ping":
                return [
                    {
                        "id": req_id,
                        "result": {"type": "pong", "version": "0.8.0", "protocol": 17},
                    }
                ]
            if method == "session.snapshot":
                return [
                    {
                        "id": req_id,
                        "result": {
                            "type": "session_snapshot",
                            "snapshot": {"tabs": [{"tab_id": "w1:t1"}]},
                        },
                    }
                ]
            return [{"id": req_id, "error": {"code": "nope", "message": method}}]

        server = FakeHerdrUnixServer(handler)
        self.addCleanup(server.shutdown)
        with HerdrSocketClient(server.path, timeout=2.0) as client:
            pong = client.ping()
            self.assertEqual(pong["type"], "pong")
            self.assertEqual(pong["protocol"], 17)
            snap = client.snapshot()
            self.assertEqual(snap["snapshot"]["tabs"][0]["tab_id"], "w1:t1")

    def test_error_payload_raises(self) -> None:
        def handler(request: Dict[str, Any]) -> List[Dict[str, Any]]:
            return [
                {
                    "id": request.get("id"),
                    "error": {"code": "not_found", "message": "missing pane"},
                }
            ]

        server = FakeHerdrUnixServer(handler)
        self.addCleanup(server.shutdown)
        with HerdrSocketClient(server.path, timeout=2.0) as client:
            with self.assertRaises(HerdrSocketError) as ctx:
                client.request("pane.read", {"pane_id": "nope"})
            self.assertIn("missing pane", str(ctx.exception))

    def test_event_session_reuses_one_connection(self) -> None:
        def handler(request: Dict[str, Any]) -> List[Dict[str, Any]]:
            req_id = request.get("id")
            if request.get("method") != "events.subscribe":
                return [{"id": req_id, "error": {"code": "nope", "message": "unexpected"}}]
            params = request.get("params") or {}
            self.assertIn("subscriptions", params)
            return [
                {"id": req_id, "result": {"type": "subscription_started"}},
                {
                    "event": "pane.focused",
                    "data": {"type": "pane.focused", "pane_id": "w1:p1"},
                },
            ]

        server = FakeHerdrUnixServer(handler)
        self.addCleanup(server.shutdown)
        session = HerdrEventSession.try_open(server.path, timeout=2.0)
        self.assertIsNotNone(session)
        assert session is not None
        try:
            event = session.wait(timeout=1.0)
            self.assertIsInstance(event, dict)
            self.assertIsNone(session.wait(timeout=0.15))
        finally:
            session.close()
        self.assertEqual(server.accepts, 1)

    def test_try_open_returns_none_without_socket(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            missing = str(Path(tmp) / "nope.sock")
            self.assertIsNone(HerdrEventSession.try_open(missing, timeout=0.2))


if __name__ == "__main__":
    unittest.main()
