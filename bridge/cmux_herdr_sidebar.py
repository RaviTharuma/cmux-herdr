#!/usr/bin/env python3
"""Sidebar TUI hosted by the official cmux plugin manager.

This is an ordinary terminal program. cmux runs it in a sidebar PTY and
forwards keys while focused. The host Ghostty/cmux theme is inherited:
this module does not set a custom color palette.

Live rows come from the mux control socket (``CMUX_TUI_SOCKET``, legacy
``CMUX_MUX_SOCKET``) via documented JSON-lines commands ``identify`` and
``list-workspaces``. No Herdr APIs are invented here, and no fake team
is synthesized when the socket is down.
"""

from __future__ import annotations

import json
import os
import select
import signal
import socket
import sys
import time
from typing import Any, Dict, List, Optional, Tuple

SOCKET_ENV_KEYS = ("CMUX_TUI_SOCKET", "CMUX_MUX_SOCKET")
REFRESH_SECONDS = 2.0
MAX_LINE_BYTES = 512 * 1024


class MuxSocketError(RuntimeError):
    """Control-socket transport or protocol failure."""


def socket_path_from_env(environ: Optional[Dict[str, str]] = None) -> Optional[str]:
    """Return the first set mux socket path from the documented env vars."""
    env = environ if environ is not None else os.environ
    for key in SOCKET_ENV_KEYS:
        value = (env.get(key) or "").strip()
        if value:
            return value
    return None


def workspaces_from_tree(payload: Any) -> List[Dict[str, Any]]:
    """Extract workspace rows from a ``list-workspaces`` result.

    Args:
        payload: JSON object returned in the mux ``data`` field.

    Returns:
        A list of ``{id, name, active, key}`` dicts. Missing optional
        fields are omitted rather than invented.
    """
    if not isinstance(payload, dict):
        return []
    raw = payload.get("workspaces")
    if not isinstance(raw, list):
        return []
    rows: List[Dict[str, Any]] = []
    for item in raw:
        if not isinstance(item, dict):
            continue
        workspace_id = item.get("id")
        name = item.get("name")
        if workspace_id is None or not isinstance(name, str):
            continue
        row: Dict[str, Any] = {
            "id": workspace_id,
            "name": name,
            "active": bool(item.get("active")),
        }
        key = item.get("key")
        if isinstance(key, str) and key:
            row["key"] = key
        rows.append(row)
    return rows


def render_sidebar(
    rows: List[Dict[str, Any]],
    selected: int,
    *,
    cols: int,
    rows_h: int,
    connected: bool,
    message: str = "",
) -> str:
    """Render the sidebar using default terminal attributes only.

    Bold and reverse video inherit the host Ghostty/cmux palette. No
    custom RGB or green theme is applied.

    Args:
        rows: Workspace rows from the mux socket.
        selected: Highlighted index.
        cols: Terminal width in cells.
        rows_h: Terminal height in cells.
        connected: Whether the mux socket is currently usable.
        message: Optional status line (reconnect / error).

    Returns:
        A string using ANSI cursor-home, not a trailing reset-to-green.
    """
    width = max(16, int(cols or 24))
    height = max(6, int(rows_h or 12))
    lines: List[str] = []
    title = "cmux-herdr"
    lines.append(_clip(f"\x1b[1m{title}\x1b[0m", width))
    if connected:
        lines.append(_clip("workspaces", width))
    else:
        lines.append(_clip("waiting for mux socket", width))
    lines.append(_clip("", width))

    if not connected:
        detail = message or "set CMUX_TUI_SOCKET (legacy CMUX_MUX_SOCKET)"
        for part in _wrap(detail, width):
            lines.append(_clip(part, width))
        lines.append(_clip("", width))
        lines.append(_clip("retrying…", width))
    elif not rows:
        lines.append(_clip("no workspaces in this session", width))
    else:
        selected = max(0, min(selected, len(rows) - 1))
        for index, row in enumerate(rows):
            marker = ">" if index == selected else " "
            active = "*" if row.get("active") else " "
            label = f"{marker}{active} {row['name']}"
            body = _clip(label, width)
            if index == selected:
                lines.append(f"\x1b[7m{body}\x1b[0m")
            else:
                lines.append(body)

    while len(lines) < height - 1:
        lines.append(_clip("", width))
    footer = f"{len(rows)} live" if connected else "offline"
    lines.append(_clip(footer, width))
    body = "\n".join(lines[:height])
    return f"\x1b[H\x1b[J{body}"


def _clip(text: str, width: int) -> str:
    """Pad or truncate a visible line to ``width`` cells."""
    visible = _strip_ansi(text)
    if len(visible) > width:
        visible = visible[: max(0, width - 1)] + "…"
        return visible
    return visible + (" " * (width - len(visible)))


def _strip_ansi(text: str) -> str:
    """Remove the few SGR sequences this renderer emits."""
    return text.replace("\x1b[1m", "").replace("\x1b[0m", "").replace("\x1b[7m", "")


def _wrap(text: str, width: int) -> List[str]:
    """Wrap ``text`` to ``width`` without hyphenation."""
    if width <= 1:
        return [text]
    words = text.split()
    if not words:
        return [""]
    lines: List[str] = []
    current = words[0]
    for word in words[1:]:
        trial = f"{current} {word}"
        if len(trial) <= width:
            current = trial
        else:
            lines.append(current)
            current = word
    lines.append(current)
    return lines


class MuxClient:
    """JSON-lines client for the documented mux control socket."""

    def __init__(self, path: str, timeout: float = 2.0) -> None:
        """Open ``path`` as a Unix stream socket.

        Args:
            path: Filesystem path from ``CMUX_TUI_SOCKET``.
            timeout: Per-request timeout in seconds.

        Raises:
            MuxSocketError: when the socket cannot be opened.
        """
        self.path = path
        self.timeout = timeout
        self._next_id = 1
        try:
            self._sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            self._sock.settimeout(timeout)
            self._sock.connect(path)
        except OSError as exc:
            raise MuxSocketError(f"connect failed: {exc}") from exc
        self._buf = bytearray()

    def close(self) -> None:
        """Close the underlying socket."""
        try:
            self._sock.close()
        except OSError:
            pass

    def call(self, cmd: str, **fields: Any) -> Any:
        """Send one documented mux command and return its ``data``.

        Args:
            cmd: Command name, for example ``list-workspaces``.
            **fields: Extra request fields from the command contract.

        Returns:
            The JSON ``data`` payload.

        Raises:
            MuxSocketError: on transport or ``ok:false`` responses.
        """
        request_id = self._next_id
        self._next_id += 1
        payload: Dict[str, Any] = {"id": request_id, "cmd": cmd}
        payload.update(fields)
        raw = (json.dumps(payload, separators=(",", ":")) + "\n").encode("utf-8")
        try:
            self._sock.sendall(raw)
            reply = self._read_json()
        except OSError as exc:
            raise MuxSocketError(f"{cmd} failed: {exc}") from exc
        if not isinstance(reply, dict):
            raise MuxSocketError(f"{cmd} returned a non-object")
        if reply.get("id") != request_id:
            raise MuxSocketError(f"{cmd} reply id mismatch")
        if not reply.get("ok"):
            raise MuxSocketError(f"{cmd} error: {reply.get('error') or reply}")
        return reply.get("data")

    def list_workspaces(self) -> List[Dict[str, Any]]:
        """Return live workspaces from ``list-workspaces``."""
        self.call("identify")
        return workspaces_from_tree(self.call("list-workspaces"))

    def select_workspace(self, index: int) -> None:
        """Select a workspace by zero-based index."""
        self.call("select-workspace", index=int(index))

    def _read_json(self) -> Any:
        deadline = time.monotonic() + self.timeout
        while True:
            newline = self._buf.find(b"\n")
            if newline >= 0:
                line = bytes(self._buf[:newline])
                del self._buf[: newline + 1]
                if not line:
                    continue
                if len(line) > MAX_LINE_BYTES:
                    raise MuxSocketError("reply exceeds size limit")
                try:
                    return json.loads(line.decode("utf-8"))
                except json.JSONDecodeError as exc:
                    raise MuxSocketError(f"invalid JSON: {exc}") from exc
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise MuxSocketError("timed out waiting for reply")
            self._sock.settimeout(remaining)
            chunk = self._sock.recv(4096)
            if not chunk:
                raise MuxSocketError("socket closed")
            self._buf.extend(chunk)
            if len(self._buf) > MAX_LINE_BYTES:
                raise MuxSocketError("reply exceeds size limit")


def _terminal_size() -> Tuple[int, int]:
    try:
        size = os.get_terminal_size()
        return size.columns, size.lines
    except OSError:
        return 28, 20


def run_sidebar(
    environ: Optional[Dict[str, str]] = None,
    *,
    stdin=None,
    stdout=None,
    now=time.monotonic,
    sleep=time.sleep,
    client_factory=MuxClient,
    once: bool = False,
) -> int:
    """Run the sidebar event loop.

    Args:
        environ: Environment mapping; defaults to ``os.environ``.
        stdin: Input stream (defaults to ``sys.stdin``).
        stdout: Output stream (defaults to ``sys.stdout``).
        now: Clock function, injectable for tests.
        sleep: Sleep function, injectable for tests.
        client_factory: Mux client constructor, injectable for tests.
        once: When true, draw one frame and return (tests / doctor).

    Returns:
        Process exit code. ``Esc`` never exits; ``Ctrl-C`` does.
    """
    env = environ if environ is not None else os.environ
    stdin = sys.stdin if stdin is None else stdin
    stdout = sys.stdout if stdout is None else stdout
    selected = 0
    rows: List[Dict[str, Any]] = []
    connected = False
    message = ""
    last_refresh = 0.0
    resized = {"flag": False}

    def _on_winch(_signum, _frame) -> None:
        resized["flag"] = True

    try:
        signal.signal(signal.SIGWINCH, _on_winch)
    except (OSError, ValueError, AttributeError):
        pass

    def draw() -> None:
        cols, rows_h = _terminal_size()
        frame = render_sidebar(
            rows,
            selected,
            cols=cols,
            rows_h=rows_h,
            connected=connected,
            message=message,
        )
        stdout.write(frame)
        stdout.flush()

    def refresh() -> None:
        nonlocal rows, connected, message, last_refresh, selected
        last_refresh = now()
        path = socket_path_from_env(env)
        if not path:
            connected = False
            rows = []
            message = "CMUX_TUI_SOCKET is unset"
            return
        client = None
        try:
            client = client_factory(path)
            fetched = client.list_workspaces()
            rows = fetched
            connected = True
            message = ""
            if rows:
                selected = max(0, min(selected, len(rows) - 1))
        except (MuxSocketError, OSError) as exc:
            connected = False
            rows = []
            message = str(exc)
        finally:
            if client is not None:
                client.close()

    refresh()
    draw()
    if once:
        return 0

    fd = None
    try:
        fd = stdin.fileno()
    except (AttributeError, OSError, ValueError):
        fd = None

    while True:
        if resized["flag"]:
            resized["flag"] = False
            draw()
        if now() - last_refresh >= REFRESH_SECONDS:
            refresh()
            draw()
        timeout = max(0.05, REFRESH_SECONDS - (now() - last_refresh))
        ready = []
        if fd is not None:
            try:
                ready, _, _ = select.select([fd], [], [], timeout)
            except (OSError, ValueError):
                sleep(timeout)
                continue
        else:
            sleep(timeout)
            continue
        if not ready:
            continue
        data = os.read(fd, 32)
        if not data:
            continue
        if data in (b"\x03",):  # Ctrl-C
            return 0
        if data in (b"\x1b", b"\x1b\x1b"):
            # cmux owns the focus escape chord; do not exit.
            continue
        if data in (b"\x1b[A", b"k"):
            if rows:
                selected = (selected - 1) % len(rows)
                draw()
            continue
        if data in (b"\x1b[B", b"j"):
            if rows:
                selected = (selected + 1) % len(rows)
                draw()
            continue
        if data in (b"\r", b"\n"):
            path = socket_path_from_env(env)
            if path and rows:
                client = None
                try:
                    client = client_factory(path)
                    client.select_workspace(selected)
                except (MuxSocketError, OSError) as exc:
                    message = str(exc)
                    connected = False
                finally:
                    if client is not None:
                        client.close()
                refresh()
                draw()
            continue


def main(argv: Optional[List[str]] = None) -> int:
    """CLI entry for the sidebar TUI."""
    args = list(sys.argv[1:] if argv is None else argv)
    if args in (["--help"], ["-h"]):
        sys.stdout.write(
            "cmux-herdr-sidebar — cmux sidebar plugin TUI\n"
            "Hosted by: cmux sidebar plugin use cmux-herdr\n"
            "Socket: CMUX_TUI_SOCKET (legacy CMUX_MUX_SOCKET)\n"
            "Keys: j/k or arrows move, enter selects, Ctrl-C exits.\n"
            "Esc is ignored (cmux owns the sidebar escape chord).\n"
        )
        return 0
    if args == ["--once"]:
        return run_sidebar(once=True)
    try:
        return run_sidebar()
    except KeyboardInterrupt:
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
