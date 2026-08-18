#!/usr/bin/env python3
"""Socket-first Herdr control surface for the cmux-herdr plugin.

Official methods: https://herdr.dev/docs/socket-api/

Calls the Unix-socket RPC first (same wire as native
``HerdrNestedTopologyClient``), then falls back to documented ``herdr``
CLI wrappers when the socket is down. Mutations talk to **Herdr** even
when native owns the cmux projection — the handoff lease only gates
mirroring, not the inner mux.

Never wraps ``server.stop``, ``pane.graphics.*``, or ``plugin.*``.
Host close still detaches; it does not stop Herdr.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass
from typing import Any, Callable, Dict, List, Optional, Sequence, Tuple

try:
    from .cmux_herdr_socket import (
        HerdrSocketClient,
        HerdrSocketError,
        socket_path_from_env,
    )
except ImportError:  # pragma: no cover - script-style import
    from cmux_herdr_socket import (  # type: ignore
        HerdrSocketClient,
        HerdrSocketError,
        socket_path_from_env,
    )


class ApiError(RuntimeError):
    """User-facing Herdr API failure (allowlist, transport, or CLI)."""


# Destructive / experimental surfaces the plugin must not expose.
FORBIDDEN_METHODS = frozenset({"server.stop"})
FORBIDDEN_PREFIXES: Tuple[str, ...] = ("pane.graphics.", "plugin.")

# Published protocol-17 methods the plugin may call. Keep in lockstep with
# herdr.dev/docs/socket-api — add here when Herdr ships a new *safe* verb.
ALLOWED_METHODS = frozenset(
    {
        "ping",
        "server.reload_config",
        "server.agent_manifests",
        "server.reload_agent_manifests",
        "notification.show",
        "client.window_title.set",
        "client.window_title.clear",
        "session.snapshot",
        "workspace.create",
        "workspace.list",
        "workspace.get",
        "workspace.focus",
        "workspace.rename",
        "workspace.move",
        "workspace.move_block",
        "workspace.report_metadata",
        "workspace.close",
        "worktree.list",
        "worktree.create",
        "worktree.open",
        "worktree.remove",
        "tab.create",
        "tab.list",
        "tab.get",
        "tab.focus",
        "tab.rename",
        "tab.move",
        "tab.close",
        "pane.split",
        "pane.swap",
        "pane.move",
        "pane.zoom",
        "pane.layout",
        "pane.process_info",
        "pane.neighbor",
        "pane.edges",
        "pane.focus_direction",
        "pane.resize",
        "pane.list",
        "pane.current",
        "pane.get",
        "pane.rename",
        "pane.send_text",
        "pane.send_keys",
        "pane.send_input",
        "pane.read",
        "pane.report_agent",
        "pane.report_agent_session",
        "pane.report_metadata",
        "pane.clear_agent_authority",
        "pane.release_agent",
        "pane.close",
        "pane.wait_for_output",
        "popup.close",
        "layout.export",
        "layout.apply",
        "layout.set_split_ratio",
        "agent.list",
        "agent.get",
        "agent.read",
        "agent.explain",
        "agent.send_keys",
        "agent.prompt",
        "agent.wait",
        "agent.rename",
        "agent.focus",
        "agent.start",
        "agent.view.set",
        "agent.view.clear",
        "events.subscribe",
        "events.wait",
        "integration.install",
        "integration.uninstall",
    }
)


def assert_method_allowed(method: str) -> str:
    """Return ``method`` if it is a published, non-forbidden RPC.

    Raises:
        ApiError: when the method is empty, forbidden, or not allowlisted.
    """
    name = (method or "").strip()
    if not name:
        raise ApiError("missing Herdr method")
    if name in FORBIDDEN_METHODS or name.startswith(FORBIDDEN_PREFIXES):
        raise ApiError(f"refusing {name}: not part of the plugin control surface")
    if name not in ALLOWED_METHODS:
        raise ApiError(f"unknown or unsupported Herdr method: {name}")
    return name


def extract_read_text(payload: Any) -> str:
    """Pull pane/agent text out of a socket result or CLI blob."""
    if payload is None:
        return ""
    if isinstance(payload, str):
        return payload
    if isinstance(payload, list):
        return "\n".join(str(item) for item in payload)
    if not isinstance(payload, dict):
        return str(payload)
    for key in ("text", "content", "output", "data"):
        value = payload.get(key)
        if isinstance(value, str):
            return value
    lines = payload.get("lines")
    if isinstance(lines, list):
        return "\n".join(str(item) for item in lines)
    for key in ("result", "pane", "agent"):
        nested = payload.get(key)
        if nested is not None and nested is not payload:
            text = extract_read_text(nested)
            if text:
                return text
    return ""


def extract_agent_status(payload: Any) -> Optional[str]:
    """Return ``agent_status`` from a pane/agent get/list payload."""
    if not isinstance(payload, dict):
        return None
    for key in ("agent_status", "status", "state"):
        value = payload.get(key)
        if isinstance(value, str) and value:
            return value
    pane = payload.get("pane")
    if isinstance(pane, dict):
        return extract_agent_status(pane)
    agent = payload.get("agent")
    if isinstance(agent, dict):
        return extract_agent_status(agent)
    result = payload.get("result")
    if isinstance(result, dict) and result is not payload:
        return extract_agent_status(result)
    return None


def build_cli_argv(method: str, params: Optional[Dict[str, Any]] = None) -> Optional[List[str]]:
    """Map a socket method onto a documented ``herdr`` CLI argv (no binary).

    Returns None when there is no safe CLI equivalent (socket-only).
    """
    params = dict(params or {})
    pane = _str(params.get("pane_id") or params.get("target"))
    tab = _str(params.get("tab_id"))
    workspace = _str(params.get("workspace_id"))
    label = _str(params.get("label"))
    direction = _str(params.get("direction"))
    text = _str(params.get("text") or params.get("prompt"))

    if method == "ping":
        return ["status"]
    if method == "session.snapshot":
        return ["api", "snapshot"]
    if method == "workspace.list":
        return ["workspace", "list"]
    if method == "workspace.get" and workspace:
        return ["workspace", "get", workspace]
    if method == "workspace.focus" and workspace:
        return ["workspace", "focus", workspace]
    if method == "workspace.close" and workspace:
        return ["workspace", "close", workspace]
    if method == "workspace.rename" and workspace and label:
        return ["workspace", "rename", workspace, label]
    if method == "workspace.create":
        argv = ["workspace", "create"]
        cwd = _str(params.get("cwd"))
        if cwd:
            argv.extend(["--cwd", cwd])
        if label:
            argv.extend(["--label", label])
        return argv
    if method == "tab.list":
        return ["tab", "list"]
    if method == "tab.get" and tab:
        return ["tab", "get", tab]
    if method == "tab.focus" and tab:
        return ["tab", "focus", tab]
    if method == "tab.close" and tab:
        return ["tab", "close", tab]
    if method == "tab.rename" and tab and label:
        return ["tab", "rename", tab, label]
    if method == "tab.create":
        argv = ["tab", "create"]
        if label:
            argv.extend(["--label", label])
        if workspace:
            argv.extend(["--workspace", workspace])
        return argv
    if method == "pane.list":
        return ["pane", "list"]
    if method == "pane.get" and pane:
        return ["pane", "get", pane]
    if method == "pane.current":
        argv = ["pane", "current"]
        caller = _str(params.get("caller_pane_id"))
        if caller:
            argv.append(caller)
        return argv
    if method == "pane.close" and pane:
        argv = ["pane", "close", pane]
        if params.get("force"):
            argv.append("--force")
        return argv
    if method == "pane.zoom":
        argv = ["pane", "zoom"]
        if pane:
            argv.append(pane)
        mode = _str(params.get("mode"))
        if mode in {"on", "off"}:
            argv.append(f"--{mode}")
        elif not pane:
            argv.append("--current")
        return argv
    if method == "pane.resize":
        argv = ["pane", "resize"]
        if pane:
            argv.append(pane)
        else:
            argv.append("--current")
        if direction:
            argv.extend(["--direction", direction])
        amount = params.get("amount")
        if amount is not None:
            argv.extend(["--amount", str(amount)])
        cols, rows = params.get("cols"), params.get("rows")
        if cols is not None and rows is not None:
            argv.extend(["--cols", str(cols), "--rows", str(rows)])
        return argv
    if method == "pane.swap":
        argv = ["pane", "swap"]
        source = _str(params.get("source_pane_id") or pane)
        target = _str(params.get("target_pane_id"))
        if source and target:
            argv.extend([source, target])
            return argv
        if source:
            argv.append(source)
        if direction:
            argv.extend(["--direction", direction])
        elif not source:
            argv.append("--current")
        return argv
    if method == "pane.neighbor":
        argv = ["pane", "neighbor"]
        if pane:
            argv.append(pane)
        if direction:
            argv.extend(["--direction", direction])
        if not pane:
            argv.append("--current")
        return argv
    if method == "pane.layout":
        argv = ["pane", "layout"]
        if pane:
            argv.append(pane)
        elif tab:
            argv.extend(["--tab", tab])
        else:
            argv.append("--current")
        return argv
    if method == "pane.split":
        argv = ["pane", "split"]
        if pane:
            argv.append(pane)
        else:
            argv.append("--current")
        argv.extend(["--direction", direction or "right"])
        ratio = params.get("ratio")
        if ratio is not None:
            argv.extend(["--ratio", str(ratio)])
        return argv
    if method == "pane.focus_direction" and direction:
        return ["pane", "focus", "--direction", direction]
    if method == "pane.send_text" and pane and text:
        return ["pane", "send", pane, "--text", text]
    if method == "pane.send_keys" and pane:
        keys = _str(params.get("keys") or params.get("key") or text)
        if not keys:
            return None
        return ["pane", "send-keys", pane, keys]
    if method == "pane.read" and pane:
        argv = ["pane", "read", pane]
        source = _str(params.get("source")) or "recent"
        argv.extend(["--source", source])
        lines = params.get("lines")
        if lines is not None:
            argv.extend(["--lines", str(lines)])
        return argv
    if method == "layout.export":
        argv = ["pane", "layout"]
        if tab:
            argv.extend(["--tab", tab])
        elif pane:
            argv.append(pane)
        else:
            argv.append("--current")
        return argv
    if method == "agent.list":
        return ["agent", "list"]
    if method == "agent.get" and pane:
        return ["agent", "get", pane]
    if method == "agent.focus" and pane:
        return ["agent", "focus", pane]
    if method == "agent.read" and pane:
        argv = ["agent", "read", pane]
        source = _str(params.get("source"))
        if source:
            argv.extend(["--source", source])
        lines = params.get("lines")
        if lines is not None:
            argv.extend(["--lines", str(lines)])
        return argv
    if method == "agent.prompt" and pane and text:
        argv = ["agent", "prompt", pane, text]
        wait = params.get("wait")
        until = _str(params.get("until"))
        if isinstance(wait, dict):
            until = until or _str(wait.get("until"))
        if until:
            argv.extend(["--until", until])
        return argv
    if method == "agent.wait" and pane:
        argv = ["agent", "wait", pane]
        until = _str(params.get("until")) or "done"
        argv.extend(["--until", until])
        timeout_ms = params.get("timeout_ms")
        if timeout_ms is not None:
            argv.extend(["--timeout-ms", str(timeout_ms)])
        return argv
    if method == "agent.start" and pane:
        argv = ["agent", "start", pane]
        agent = _str(params.get("agent"))
        if agent:
            argv.extend(["--agent", agent])
        return argv
    if method == "agent.send_keys" and pane:
        keys = _str(params.get("keys") or params.get("key") or text)
        if not keys:
            return None
        return ["agent", "send-keys", pane, keys]
    if method == "agent.rename" and pane and label:
        return ["agent", "rename", pane, label]
    if method == "notification.show":
        title = _str(params.get("title"))
        if not title:
            return None
        argv = ["notification", "show", title]
        body = _str(params.get("body"))
        if body:
            argv.extend(["--body", body])
        return argv
    return None


def _str(value: Any) -> str:
    """Return a stripped string, or empty when the value is missing."""
    if value is None:
        return ""
    return str(value).strip()


@dataclass
class ApiResult:
    """One allowlisted Herdr RPC outcome."""

    ok: bool
    method: str
    via: str
    result: Any = None
    error: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        """JSON-ready payload for the CLI."""
        payload: Dict[str, Any] = {
            "ok": self.ok,
            "method": self.method,
            "via": self.via,
            "result": self.result,
        }
        if self.error:
            payload["error"] = self.error
        return payload


CliRunner = Callable[[Sequence[str]], Any]


class HerdrApi:
    """Allowlisted Herdr RPC: socket first, documented CLI fallback."""

    def __init__(
        self,
        *,
        socket_path: Optional[str] = None,
        client: Optional[HerdrSocketClient] = None,
        cli_runner: Optional[CliRunner] = None,
        timeout: float = 8.0,
    ) -> None:
        """Create a caller. ``client`` is used as-is when provided (tests)."""
        self.socket_path = socket_path
        self.client = client
        self.cli_runner = cli_runner
        self.timeout = timeout

    def call(
        self,
        method: str,
        params: Optional[Dict[str, Any]] = None,
        *,
        socket_only: bool = False,
    ) -> ApiResult:
        """Invoke ``method`` with ``params``.

        Raises:
            ApiError: forbidden/unknown method, or both transports failed.
        """
        name = assert_method_allowed(method)
        payload = dict(params or {})
        socket_error: Optional[str] = None
        if self.client is not None:
            try:
                result = self.client.request(name, payload)
                return ApiResult(ok=True, method=name, via="socket", result=result)
            except HerdrSocketError as exc:
                socket_error = str(exc)
        else:
            try:
                result = self._socket_request(name, payload)
                return ApiResult(ok=True, method=name, via="socket", result=result)
            except (ApiError, HerdrSocketError, OSError) as exc:
                socket_error = str(exc)
        if socket_only:
            raise ApiError(socket_error or f"{name} socket request failed")
        argv = build_cli_argv(name, payload)
        if argv is None:
            raise ApiError(
                socket_error or f"{name} has no CLI fallback (socket required)"
            )
        try:
            result = self._cli_request(argv)
            return ApiResult(ok=True, method=name, via="cli", result=result)
        except Exception as exc:  # noqa: BLE001 — surface either transport
            detail = socket_error or str(exc)
            raise ApiError(f"{name} failed: {detail}") from exc

    def _socket_request(self, method: str, params: Dict[str, Any]) -> Any:
        """Open a short-lived socket, call once, close. Never reuse subscribe."""
        path = self.socket_path or socket_path_from_env()
        if not path:
            path = os.environ.get("HERDR_SOCKET_PATH") or ""
        if not path or not os.path.exists(path):
            raise ApiError("Herdr socket not available")
        with HerdrSocketClient(path, timeout=self.timeout) as client:
            return client.request(method, params)

    def _cli_request(self, argv: Sequence[str]) -> Any:
        """Run ``herdr <argv>`` via the injected runner or the bridge helper."""
        if self.cli_runner is not None:
            return self.cli_runner(argv)
        try:
            from .cmux_herdr_bridge import BridgeError, herdr_json, run_cmd, which
        except ImportError:
            from cmux_herdr_bridge import BridgeError, herdr_json, run_cmd, which
        if not which("herdr"):
            raise ApiError("herdr not found on PATH")
        try:
            return herdr_json(list(argv))
        except BridgeError:
            proc = run_cmd(["herdr", *argv], timeout=self.timeout)
            if proc.returncode != 0:
                err = (proc.stderr or proc.stdout or str(proc.returncode)).strip()
                raise ApiError(err or "herdr CLI failed")
            out = (proc.stdout or "").strip()
            if out.startswith("{") or out.startswith("["):
                try:
                    return json.loads(out)
                except json.JSONDecodeError:
                    return out
            return out
