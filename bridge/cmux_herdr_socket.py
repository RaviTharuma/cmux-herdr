#!/usr/bin/env python3
"""Persistent NDJSON Unix-socket client for Herdr protocol 17.

Wire shape matches native ``HerdrNestedTopologyClient``:

``{"id": "...", "method": "...", "params": {...}}``

Never shells out to the ``herdr`` CLI. ``watch --tmux-parity`` holds one
``events.subscribe`` session instead of reconnecting every tick.
"""

from __future__ import annotations

import json
import os
import socket
import threading
from typing import Any, Dict, List, Optional

# Same subscription set as HerdrProtocol17Compatibility.defaultSubscriptions.
DEFAULT_SUBSCRIPTIONS: List[Dict[str, str]] = [
    {"type": "workspace.created"},
    {"type": "workspace.updated"},
    {"type": "workspace.metadata_updated"},
    {"type": "workspace.renamed"},
    {"type": "workspace.moved"},
    {"type": "workspace.reordered"},
    {"type": "workspace.closed"},
    {"type": "workspace.focused"},
    {"type": "tab.created"},
    {"type": "tab.closed"},
    {"type": "tab.focused"},
    {"type": "tab.renamed"},
    {"type": "tab.moved"},
    {"type": "pane.created"},
    {"type": "pane.closed"},
    {"type": "pane.updated"},
    {"type": "pane.focused"},
    {"type": "pane.moved"},
    {"type": "pane.exited"},
    {"type": "pane.agent_detected"},
    {"type": "pane.agent_status_changed"},
    {"type": "pane.resized"},
    {"type": "layout.updated"},
    {"type": "layout.changed"},
]

MAX_LINE_BYTES = 512 * 1024


class HerdrSocketError(RuntimeError):
    """Transport or protocol failure on the Herdr Unix socket."""


def socket_path_from_env() -> Optional[str]:
    """Return ``HERDR_SOCKET_PATH`` when it exists on disk."""
    path = os.environ.get("HERDR_SOCKET_PATH")
    if not path or not os.path.exists(path):
        return None
    return path


def assert_socket_secure(path: str) -> None:
    """Reject unsafe Herdr sockets (symlink, wrong owner, loose mode).

    Raises:
        HerdrSocketError: when the path fails local security checks.
    """
    try:
        st = os.lstat(path)
    except OSError as exc:
        raise HerdrSocketError(f"socket stat failed: {exc}") from exc
    import stat as stat_mod

    if stat_mod.S_ISLNK(st.st_mode):
        raise HerdrSocketError("refusing symlink Herdr socket path")
    if not stat_mod.S_ISSOCK(st.st_mode):
        raise HerdrSocketError("Herdr path is not a Unix socket")
    mode = st.st_mode & 0o777
    if mode & 0o077:
        raise HerdrSocketError(
            f"Herdr socket mode {oct(mode)} is too open (want 0600 or tighter)"
        )
    if hasattr(os, "getuid") and st.st_uid != os.getuid():
        raise HerdrSocketError(
            f"Herdr socket uid {st.st_uid} != current uid {os.getuid()}"
        )


class HerdrSocketClient:
    """One connected NDJSON session (request/response + optional subscribe)."""

    def __init__(self, path: str, *, timeout: float = 5.0) -> None:
        """Create a client for ``path``. Call ``connect`` before requests."""
        self.path = path
        self.timeout = timeout
        self._sock: Optional[socket.socket] = None
        self._buf = b""
        self._next = 0
        self._lock = threading.Lock()

    def connect(self) -> None:
        """Open the Unix stream. Raises ``HerdrSocketError`` on failure."""
        self.close()
        assert_socket_secure(self.path)
        sock = None
        try:
            sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            sock.settimeout(self.timeout)
            sock.connect(self.path)
        except OSError as exc:
            if sock is not None:
                try:
                    sock.close()
                except OSError:
                    pass
            raise HerdrSocketError(f"connect failed: {exc}") from exc
        self._sock = sock
        self._buf = b""

    def close(self) -> None:
        """Close the socket if open. Safe to call twice."""
        sock = self._sock
        self._sock = None
        self._buf = b""
        if sock is None:
            return
        try:
            sock.close()
        except OSError:
            return

    def __enter__(self) -> "HerdrSocketClient":
        """Connect and return self."""
        self.connect()
        return self

    def __exit__(self, *_exc: object) -> None:
        """Close on context exit."""
        self.close()

    def request(self, method: str, params: Optional[Dict[str, Any]] = None) -> Any:
        """Send one RPC and return the ``result`` object.

        Raises:
            HerdrSocketError: on transport, timeout, or ``error`` payload.
        """
        req_id = self._allocate_id()
        payload = {"id": req_id, "method": method, "params": params or {}}
        self._send(payload)
        line = self._read_line(timeout=self.timeout, required=True)
        try:
            obj = json.loads(line)
        except json.JSONDecodeError as exc:
            raise HerdrSocketError(f"malformed JSON: {exc}") from exc
        if not isinstance(obj, dict):
            raise HerdrSocketError("response is not an object")
        error = obj.get("error")
        if isinstance(error, dict):
            message = str(error.get("message") or error.get("code") or error)
            raise HerdrSocketError(message)
        return obj.get("result")

    def ping(self) -> Any:
        """Handshake ``ping`` → pong result."""
        return self.request("ping", {})

    def snapshot(self) -> Any:
        """Full ``session.snapshot`` result."""
        return self.request("session.snapshot", {})

    def subscribe(
        self, subscriptions: Optional[List[Dict[str, str]]] = None
    ) -> Any:
        """Start ``events.subscribe``. Later events are read via ``read_event``."""
        return self.request(
            "events.subscribe",
            {"subscriptions": list(subscriptions or DEFAULT_SUBSCRIPTIONS)},
        )

    def read_event(self, *, timeout: float) -> Optional[Dict[str, Any]]:
        """Read one NDJSON event line, or ``None`` on timeout/close."""
        line = self._read_line(timeout=timeout, required=False)
        if line is None:
            return None
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            return None
        return obj if isinstance(obj, dict) else None

    def _allocate_id(self) -> str:
        """Return the next request id (``plugin-N``)."""
        with self._lock:
            self._next += 1
            return f"plugin-{self._next}"

    def _send(self, payload: Dict[str, Any]) -> None:
        """Write one NDJSON request line."""
        if self._sock is None:
            raise HerdrSocketError("not connected")
        data = (json.dumps(payload, separators=(",", ":")) + "\n").encode("utf-8")
        if len(data) > MAX_LINE_BYTES:
            raise HerdrSocketError("oversized request")
        try:
            self._sock.sendall(data)
        except OSError as exc:
            raise HerdrSocketError(f"send failed: {exc}") from exc

    def _read_line(self, *, timeout: float, required: bool) -> Optional[str]:
        """Read until newline. ``required`` raises; otherwise returns ``None``."""
        if self._sock is None:
            if required:
                raise HerdrSocketError("not connected")
            return None
        self._sock.settimeout(max(0.05, timeout))
        while b"\n" not in self._buf:
            try:
                chunk = self._sock.recv(min(64 * 1024, MAX_LINE_BYTES))
            except socket.timeout:
                if required:
                    raise HerdrSocketError("timeout waiting for response")
                return None
            except OSError as exc:
                if required:
                    raise HerdrSocketError(f"recv failed: {exc}") from exc
                return None
            if not chunk:
                if required:
                    raise HerdrSocketError("socket closed")
                return None
            self._buf += chunk
            if len(self._buf) > MAX_LINE_BYTES:
                raise HerdrSocketError("oversized line")
        line, self._buf = self._buf.split(b"\n", 1)
        return line.decode("utf-8", errors="replace")


class HerdrEventSession:
    """Long-lived ``events.subscribe`` used by ``watch --tmux-parity``."""

    def __init__(self, client: HerdrSocketClient) -> None:
        """Wrap an already-subscribed client."""
        self.client = client

    @classmethod
    def try_open(
        cls,
        path: Optional[str] = None,
        *,
        timeout: float = 5.0,
    ) -> Optional["HerdrEventSession"]:
        """Connect and subscribe, or return ``None`` when the socket is unusable."""
        resolved = path or socket_path_from_env()
        if not resolved or not os.path.exists(resolved):
            return None
        client = HerdrSocketClient(resolved, timeout=timeout)
        try:
            client.connect()
            result = client.subscribe()
            if isinstance(result, dict):
                kind = result.get("type")
                if kind not in (None, "subscription_started", "ok"):
                    client.close()
                    return None
            return cls(client)
        except (OSError, HerdrSocketError, ValueError):
            client.close()
            return None

    def wait(self, *, timeout: float = 3.0) -> Optional[Dict[str, Any]]:
        """Block until one event arrives. Returns the event dict, or None."""
        try:
            event = self.client.read_event(timeout=timeout)
        except HerdrSocketError:
            return None
        return event

    def alive(self) -> bool:
        """True while the subscribe socket is still open."""
        return self.client._sock is not None

    def close(self) -> None:
        """Tear down the subscribe socket."""
        self.client.close()
