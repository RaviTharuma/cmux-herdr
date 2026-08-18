#!/usr/bin/env python3
"""cmux-herdr bridge: mirror herdr agent/pane state into cmux sidebar status APIs.

Works without any cmux upstream PR. Invokes `herdr` and `cmux` CLIs.
Stdlib only.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Sequence, Tuple

try:
    from .cmux_herdr_handoff import (
        FORCE_PLUGIN_ENV,
        NATIVE_LIVE_ENV,
        claim_native_writer,
        claim_plugin_writer,
        heartbeat_plugin_writer,
        release_native_writer,
        release_plugin_writer,
        resolve_writer,
    )
    from .cmux_herdr_handoff import writer_status as handoff_writer_status
except ImportError:  # pragma: no cover - script-style import
    from cmux_herdr_handoff import (
        FORCE_PLUGIN_ENV,
        NATIVE_LIVE_ENV,
        claim_native_writer,
        claim_plugin_writer,
        heartbeat_plugin_writer,
        release_native_writer,
        release_plugin_writer,
        resolve_writer,
    )
    from cmux_herdr_handoff import writer_status as handoff_writer_status

STATUS_PREFIX = "herdr:"

# Single-writer / title-lock contract (shared with native via handoff).
LOCK_TITLES_ENV = "CMUX_HERDR_LOCK_TITLES"

# agent_status -> (icon, color, priority)
STATUS_STYLE: Dict[str, Tuple[str, str, int]] = {
    "working": ("hammer", "#ff9500", 80),
    "idle": ("pause.circle", "#8e8e93", 40),
    "done": ("checkmark.circle", "#34c759", 30),
    "blocked": ("exclamationmark.triangle", "#ff3b30", 90),
    "unknown": ("questionmark.circle", "#8e8e93", 10),
}

DEFAULT_STYLE = ("circle", "#8e8e93", 10)


class BridgeError(RuntimeError):
    """User-facing bridge failure."""


@dataclass
class Pane:
    pane_id: str
    tab_id: str
    workspace_id: str
    agent: Optional[str] = None
    agent_status: str = "unknown"
    label: Optional[str] = None
    cwd: Optional[str] = None
    focused: bool = False
    terminal_title: Optional[str] = None
    agent_session_path: Optional[str] = None
    agent_session_id: Optional[str] = None
    agent_session_kind: Optional[str] = None
    revision: Optional[int] = None
    raw: Dict[str, Any] = field(default_factory=dict)

    @property
    def display_name(self) -> str:
        if self.label:
            return self.label
        title = (self.terminal_title or "").strip()
        if title:
            # Keep short for status pills
            return title[:40] + ("…" if len(title) > 40 else "")
        if self.agent:
            return f"{self.agent}@{self.pane_id}"
        return self.pane_id

    @property
    def status_key(self) -> str:
        # Compact key; pane_ids are unique within a herdr session
        return f"{STATUS_PREFIX}{self.pane_id}"

    @property
    def has_agent(self) -> bool:
        return bool(self.agent) or self.agent_status not in ("", None, "unknown")


@dataclass
class Tab:
    tab_id: str
    workspace_id: str
    label: Optional[str] = None
    number: Optional[int] = None
    agent_status: str = "unknown"
    focused: bool = False
    pane_count: int = 0
    raw: Dict[str, Any] = field(default_factory=dict)


@dataclass
class Workspace:
    workspace_id: str
    label: Optional[str] = None
    number: Optional[int] = None
    agent_status: str = "unknown"
    focused: bool = False
    pane_count: int = 0
    tab_count: int = 0
    raw: Dict[str, Any] = field(default_factory=dict)


@dataclass
class Snapshot:
    panes: List[Pane]
    tabs: List[Tab]
    workspaces: List[Workspace]
    layouts: Any = None
    fetched_at: float = field(default_factory=time.time)

    def agent_panes(self) -> List[Pane]:
        return [p for p in self.panes if p.has_agent]


def which(cmd: str) -> Optional[str]:
    return shutil.which(cmd)


def run_cmd(
    args: Sequence[str],
    *,
    timeout: float = 15.0,
    check: bool = False,
    env: Optional[Dict[str, str]] = None,
) -> subprocess.CompletedProcess:
    merged = os.environ.copy()
    if env:
        merged.update(env)
    return subprocess.run(
        list(args),
        capture_output=True,
        text=True,
        timeout=timeout,
        check=check,
        env=merged,
    )


def _parse_json_payload(stdout: str) -> Any:
    text = (stdout or "").strip()
    if not text:
        raise BridgeError("empty JSON response")
    # herdr returns a single JSON object; tolerate trailing noise
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        # Try first JSON object line
        for line in text.splitlines():
            line = line.strip()
            if line.startswith("{") or line.startswith("["):
                return json.loads(line)
        raise


def herdr_json(args: Sequence[str], *, timeout: float = 15.0) -> Any:
    if not which("herdr"):
        raise BridgeError("herdr not found on PATH")
    proc = run_cmd(["herdr", *args], timeout=timeout)
    if proc.returncode != 0:
        err = (proc.stderr or proc.stdout or "").strip()
        raise BridgeError(f"herdr {' '.join(args)} failed: {err or proc.returncode}")
    return _parse_json_payload(proc.stdout)


_RPC_API = None


def reset_herdr_rpc() -> None:
    """Drop the process-wide RPC session (tests / socket path change)."""
    global _RPC_API
    api = _RPC_API
    _RPC_API = None
    closer = getattr(api, "close", None)
    if callable(closer):
        closer()


def _herdr_api():
    """Reuse one ``HerdrApi`` (native ``HerdrNestedTopologyClient``)."""
    global _RPC_API
    if _RPC_API is not None:
        return _RPC_API
    try:
        from .cmux_herdr_api import ApiError, HerdrApi
        from .cmux_herdr_socket import HerdrSocketError
    except ImportError:
        from cmux_herdr_api import ApiError, HerdrApi
        from cmux_herdr_socket import HerdrSocketError

    api = HerdrApi()
    try:
        api.open()
    except (ApiError, HerdrSocketError, OSError):
        pass
    _RPC_API = api
    return api


def herdr_rpc(method: str, params: Optional[Dict[str, Any]] = None) -> Any:
    """Socket-first Herdr RPC. Wraps ``ApiError`` as ``BridgeError``.

    Prefer this for control mutations so the plugin uses the same wire as
    native. Falls back to the documented CLI inside ``HerdrApi``.
    Reuses one RPC socket for the process when the Unix path is live.
    """
    try:
        from .cmux_herdr_api import ApiError
    except ImportError:
        from cmux_herdr_api import ApiError

    try:
        result = _herdr_api().call(method, params or {})
    except ApiError as exc:
        raise BridgeError(str(exc)) from exc
    return result.result


def cmux_cmd(
    args: Sequence[str],
    *,
    timeout: float = 15.0,
    workspace: Optional[str] = None,
) -> subprocess.CompletedProcess:
    if not which("cmux"):
        raise BridgeError("cmux not found on PATH")
    full = ["cmux", *args]
    if workspace and "--workspace" not in full:
        full.extend(["--workspace", workspace])
    return run_cmd(full, timeout=timeout)


def herdr_available() -> bool:
    if not which("herdr"):
        return False
    sock = os.environ.get("HERDR_SOCKET_PATH")
    if sock and os.path.exists(sock):
        return True
    # Probe lightly
    try:
        proc = run_cmd(["herdr", "status"], timeout=5.0)
        return proc.returncode == 0
    except Exception:
        return bool(os.environ.get("HERDR_ENV"))


def cmux_available() -> bool:
    if not which("cmux"):
        return False
    sock = os.environ.get("CMUX_SOCKET_PATH")
    if sock and os.path.exists(sock):
        return True
    try:
        proc = run_cmd(["cmux", "identify", "--json"], timeout=5.0)
        return proc.returncode == 0
    except Exception:
        return False


def map_status_to_style(status: Optional[str]) -> Tuple[str, str, int]:
    """Return (icon, color, priority) for a herdr agent_status."""
    key = (status or "unknown").lower().strip()
    return STATUS_STYLE.get(key, DEFAULT_STYLE)


def status_value_for_pane(
    pane: Pane,
    tabs_by_id: Optional[Dict[str, "Tab"]] = None,
    *,
    locked_title: Optional[str] = None,
    parent_tab_id: Optional[str] = None,
) -> str:
    """Build a compact, human-readable cmux status-pill value.

    When *locked_title* is set (native-title lock), that display name is used
    instead of ``pane.display_name``. *parent_tab_id* prefers the persisted
    parent map over a flickering snapshot ``tab_id``.
    """
    agent = pane.agent or "agent"
    status = (pane.agent_status or "unknown").lower()
    parts = [f"{agent}/{status}"]
    tab_id = parent_tab_id or pane.tab_id
    tab = (tabs_by_id or {}).get(tab_id)
    if tab and tab.label:
        parts.append(str(tab.label))
    name = locked_title if locked_title else pane.display_name
    if name and name not in parts:
        parts.append(name)
    return " · ".join(parts)


def _pane_from_raw(raw: Dict[str, Any]) -> Pane:
    session = raw.get("agent_session") if isinstance(raw.get("agent_session"), dict) else {}
    session_path = None
    session_id = None
    session_kind = None
    if session:
        kind = session.get("kind")
        value = session.get("value")
        session_kind = str(kind) if kind else None
        if kind == "path" and isinstance(value, str):
            session_path = value
        elif kind == "id" and isinstance(value, str):
            session_id = value
        elif isinstance(value, str) and value.endswith(".jsonl"):
            session_path = value
        elif isinstance(value, str):
            session_id = value
    revision = raw.get("revision")
    # Herdr 0.8 nests the agent name under agent_session.agent; prefer top-level.
    agent = raw.get("agent") or session.get("agent")
    if isinstance(agent, str):
        agent = agent.strip() or None
    else:
        agent = None
    return Pane(
        pane_id=str(raw.get("pane_id") or ""),
        tab_id=str(raw.get("tab_id") or ""),
        workspace_id=str(raw.get("workspace_id") or ""),
        agent=agent,
        agent_status=str(raw.get("agent_status") or "unknown"),
        label=raw.get("label"),
        cwd=raw.get("cwd") or raw.get("foreground_cwd"),
        focused=bool(raw.get("focused")),
        terminal_title=raw.get("terminal_title_stripped") or raw.get("terminal_title"),
        agent_session_path=session_path,
        agent_session_id=session_id,
        agent_session_kind=session_kind,
        revision=int(revision) if isinstance(revision, int) else None,
        raw=raw,
    )


def fetch_panes(workspace_id: Optional[str] = None) -> List[Pane]:
    args: List[str] = ["pane", "list"]
    if workspace_id:
        args.extend(["--workspace", workspace_id])
    data = herdr_json(args)
    panes_raw = (data.get("result") or {}).get("panes") or []
    return [_pane_from_raw(p) for p in panes_raw if p.get("pane_id")]


def fetch_tabs() -> List[Tab]:
    data = herdr_json(["tab", "list"])
    tabs_raw = (data.get("result") or {}).get("tabs") or []
    out: List[Tab] = []
    for t in tabs_raw:
        out.append(
            Tab(
                tab_id=str(t.get("tab_id") or ""),
                workspace_id=str(t.get("workspace_id") or ""),
                label=t.get("label"),
                number=t.get("number"),
                agent_status=str(t.get("agent_status") or "unknown"),
                focused=bool(t.get("focused")),
                pane_count=int(t.get("pane_count") or 0),
                raw=t,
            )
        )
    return out


def fetch_workspaces() -> List[Workspace]:
    data = herdr_json(["workspace", "list"])
    result = data.get("result") or {}
    # shape: {type, workspaces: [...]}
    ws_raw = result.get("workspaces") or result.get("workspace_list") or []
    if isinstance(result, list):
        ws_raw = result
    out: List[Workspace] = []
    for w in ws_raw:
        out.append(
            Workspace(
                workspace_id=str(w.get("workspace_id") or ""),
                label=w.get("label"),
                number=w.get("number"),
                agent_status=str(w.get("agent_status") or "unknown"),
                focused=bool(w.get("focused")),
                pane_count=int(w.get("pane_count") or 0),
                tab_count=int(w.get("tab_count") or 0),
                raw=w,
            )
        )
    return out


def fetch_agents() -> List[Pane]:
    """Agent list is pane-shaped; reuse Pane model."""
    try:
        data = herdr_json(["agent", "list"])
        agents_raw = (data.get("result") or {}).get("agents") or []
        if agents_raw:
            return [_pane_from_raw(a) for a in agents_raw if a.get("pane_id")]
    except BridgeError:
        pass
    # Fallback: panes that declare an agent
    return [p for p in fetch_panes() if p.agent]


def fetch_layouts_raw() -> Any:
    """Best-effort Herdr layout payload (tab trees or session.snapshot).

    Returns the raw JSON object so the mirror can parse multiple shapes.
    Missing CLI verbs are not an error — older Herdr builds simply have no
    layout command.
    """
    for args in (
        ["layout", "list"],
        ["tab", "layout"],
        ["session", "snapshot"],
    ):
        try:
            data = herdr_json(args)
        except BridgeError:
            continue
        if data:
            return data
    return {}


def fetch_snapshot() -> Snapshot:
    """Fetch Herdr topology, preferring a live Unix-socket snapshot.

    Socket-first avoids CLI fan-out every watch tick (tmux-parity). Falls
    back to ``herdr pane|tab|workspace|layout`` CLI verbs when the socket
    is missing or returns an unusable payload.
    """
    sock_snap = fetch_snapshot_via_socket()
    if sock_snap is not None:
        return sock_snap
    panes = fetch_panes()
    try:
        tabs = fetch_tabs()
    except BridgeError:
        tabs = []
    try:
        workspaces = fetch_workspaces()
    except BridgeError:
        workspaces = []
    try:
        layouts = fetch_layouts_raw()
    except BridgeError:
        layouts = {}
    return Snapshot(panes=panes, tabs=tabs, workspaces=workspaces, layouts=layouts)


def fetch_snapshot_via_socket() -> Optional[Snapshot]:
    """Return a Snapshot from ``session.snapshot`` over the Herdr socket.

    Reuses the process-wide ``HerdrApi`` session (native nested client).
    Returns None when the socket is unavailable or the payload cannot be
    parsed into panes. Never raises for transport failures.
    """
    try:
        from .cmux_herdr_api import ApiError
        from .cmux_herdr_socket import HerdrSocketError
    except ImportError:
        from cmux_herdr_api import ApiError
        from cmux_herdr_socket import HerdrSocketError
    try:
        payload = _herdr_api().call("session.snapshot", {}, socket_only=True).result
    except (ApiError, HerdrSocketError, OSError, ValueError, TypeError):
        return None
    return _snapshot_from_session_payload(payload)


def _snapshot_from_session_payload(result: Any) -> Optional[Snapshot]:
    """Parse a ``session.snapshot`` result into a Snapshot, or None."""
    if not isinstance(result, dict):
        return None
    if isinstance(result.get("snapshot"), dict):
        result = result["snapshot"]
    panes_raw = result.get("panes") or []
    tabs_raw = result.get("tabs") or []
    workspaces_raw = result.get("workspaces") or result.get("workspace_list") or []
    if not isinstance(panes_raw, list):
        return None
    panes = [
        _pane_from_raw(p)
        for p in panes_raw
        if isinstance(p, dict) and p.get("pane_id")
    ]
    tabs: List[Tab] = []
    if isinstance(tabs_raw, list):
        for t in tabs_raw:
            if not isinstance(t, dict):
                continue
            tabs.append(
                Tab(
                    tab_id=str(t.get("tab_id") or ""),
                    workspace_id=str(t.get("workspace_id") or ""),
                    label=t.get("label"),
                    number=t.get("number"),
                    agent_status=str(t.get("agent_status") or "unknown"),
                    focused=bool(t.get("focused")),
                    pane_count=int(t.get("pane_count") or 0),
                    raw=t,
                )
            )
    workspaces: List[Workspace] = []
    if isinstance(workspaces_raw, list):
        for w in workspaces_raw:
            if not isinstance(w, dict):
                continue
            workspaces.append(
                Workspace(
                    workspace_id=str(w.get("workspace_id") or ""),
                    label=w.get("label"),
                    number=w.get("number"),
                    agent_status=str(w.get("agent_status") or "unknown"),
                    focused=bool(w.get("focused")),
                    pane_count=int(w.get("pane_count") or 0),
                    tab_count=int(w.get("tab_count") or 0),
                    raw=w,
                )
            )
    layouts = result.get("layouts") or result
    return Snapshot(
        panes=panes,
        tabs=tabs,
        workspaces=workspaces,
        layouts=layouts,
    )


def _state_dir() -> str:
    """Return the user-scoped state directory, honoring XDG_STATE_HOME."""
    root = os.environ.get("XDG_STATE_HOME") or os.path.expanduser("~/.local/state")
    return os.path.join(root, "cmux-herdr")


def _env_truthy(name: str) -> bool:
    """Return True when *name* is a conventional truthy env flag."""
    raw = (os.environ.get(name) or "").strip().lower()
    return raw in ("1", "true", "yes", "on")


def association_key_for_pane(pane: Pane) -> str:
    """Return the contract association key ``pane_id:session_id`` (or pane_id)."""
    sid = (pane.agent_session_id or "").strip()
    if sid:
        return f"{pane.pane_id}:{sid}"
    return pane.pane_id


def native_live_marker_path(fp: Optional[Dict[str, Any]] = None) -> str:
    """Per-fingerprint marker native cmux writes while attachment is live."""
    return os.path.join(_state_dir(), f"native-live-{_parent_key(fp)}")


def plugin_force_writer() -> bool:
    """Escape hatch: plugin may write even when native attachment is detected."""
    return _env_truthy(FORCE_PLUGIN_ENV)


def auto_lock_titles() -> bool:
    """When set, lock each pane display name after the first successful write."""
    return _env_truthy(LOCK_TITLES_ENV)


def native_attachment_is_live(fp: Optional[Dict[str, Any]] = None) -> bool:
    """True when a *fresh* native lease owns this host.

    Detection (any of):
    - ``CMUX_HERDR_NATIVE_LIVE`` is truthy
    - a fresh JSON lease on ``native-live-<key>`` / ``writer-<key>.json``
    - a fresh legacy ``native-live`` marker (mtime within the lease TTL)

    A dead pid or expired heartbeat does **not** count — the plugin may
    resume. ``CMUX_HERDR_FORCE_PLUGIN=1`` always returns False.
    """
    return resolve_writer(_parent_key(fp)).native_live


def write_native_live_marker(fp: Optional[Dict[str, Any]] = None) -> str:
    """Native AppKit / dogfood helper claims the single-writer lock."""
    key = _parent_key(fp)
    lease = claim_native_writer(key)
    if lease is None:
        return native_live_marker_path(fp)
    return lease.path or native_live_marker_path(fp)


def clear_native_live_marker(fp: Optional[Dict[str, Any]] = None) -> None:
    """Drop the native lease on detach / process exit. Plugin files stay."""
    release_native_writer(_parent_key(fp))


def writer_status(fp: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
    """Describe which path is allowed to project ``herdr:*`` pills."""
    return handoff_writer_status(_parent_key(fp))


_native_skip_logged = False


def reset_native_skip_log() -> None:
    """Reset the once-only native-skip log (tests / process reuse)."""
    global _native_skip_logged
    _native_skip_logged = False


def _log_native_skip_once(log: bool, fp: Optional[Dict[str, Any]] = None) -> None:
    """Log the single-writer handoff once per process."""
    global _native_skip_logged
    if _native_skip_logged:
        return
    _native_skip_logged = True
    msg = (
        "cmux-herdr: native attachment is live; skipping herdr:* status writes "
        f"(fingerprint={_parent_key(fp)}). Set {FORCE_PLUGIN_ENV}=1 to force "
        "plugin writes."
    )
    if log:
        print(msg, file=sys.stderr)


def _herdr_server_pid() -> Optional[int]:
    """Return Herdr server pid when cheaply available (env or sidecar pid file)."""
    raw = os.environ.get("HERDR_SERVER_PID")
    if raw and raw.isdigit():
        return int(raw)
    sock = os.environ.get("HERDR_SOCKET_PATH")
    if not sock:
        return None
    candidates = (
        f"{sock}.pid",
        os.path.join(os.path.dirname(sock), "herdr.pid"),
    )
    for path in candidates:
        try:
            with open(path, "r", encoding="utf-8") as handle:
                text = handle.read().strip()
            if text.isdigit():
                return int(text)
        except OSError:
            continue
    return None


def collect_host_fingerprint() -> Dict[str, Any]:
    """Collect the invoking-environment host fingerprint fields.

    Fingerprint pieces (best → required for auto-bind):
    - CMUX_SURFACE_ID — outer cmux surface / host identity
    - HERDR_SOCKET_PATH — Herdr provider endpoint
    - herdr_server_pid — optional; HERDR_SERVER_PID or socket sidecar pid file
    - HERDR_WORKSPACE_ID — scopes associations within one Herdr host
    """
    return {
        "cmux_surface_id": os.environ.get("CMUX_SURFACE_ID") or None,
        "herdr_socket_path": os.environ.get("HERDR_SOCKET_PATH") or None,
        "herdr_server_pid": _herdr_server_pid(),
        "herdr_workspace_id": os.environ.get("HERDR_WORKSPACE_ID") or None,
    }


def fingerprint_missing_fields(fp: Optional[Dict[str, Any]] = None) -> List[str]:
    """Return required fingerprint env names that are unset."""
    data = fp if fp is not None else collect_host_fingerprint()
    missing: List[str] = []
    if not data.get("cmux_surface_id"):
        missing.append("CMUX_SURFACE_ID")
    if not data.get("herdr_socket_path"):
        missing.append("HERDR_SOCKET_PATH")
    return missing


def require_host_fingerprint() -> Dict[str, Any]:
    """Return a complete host fingerprint or raise a clear BridgeError.

    Auto-resolve must never fall back to the currently focused cmux workspace when
    fingerprint pieces are missing — that silently writes pills to a random host.
    Callers with an explicit ``--workspace`` override bypass this gate.
    """
    fp = collect_host_fingerprint()
    missing = fingerprint_missing_fields(fp)
    if missing:
        raise BridgeError(
            "incomplete host fingerprint (missing "
            + ", ".join(missing)
            + "); refusing to auto-select a parent binding "
            "(would risk writing herdr:* status pills to the wrong outer workspace). "
            "Run sync/watch from a cmux-hosted Herdr pane so CMUX_SURFACE_ID and "
            "HERDR_SOCKET_PATH are set, or pass --workspace explicitly."
        )
    return fp


def _parent_key(fp: Optional[Dict[str, Any]] = None) -> str:
    """Stable filename key for one concurrent parent binding / association file.

    Multiple hosts keep distinct ``parent-<key>.json`` / ``associations-<key>.json``
    files under the state directory. Key material is hashed so long socket paths
    cannot truncate away the surface id.
    """
    data = fp if fp is not None else collect_host_fingerprint()
    surface = data.get("cmux_surface_id") or "_nosurface_"
    socket_path = data.get("herdr_socket_path") or "default"
    herdr_ws = data.get("herdr_workspace_id") or "default"
    pid = data.get("herdr_server_pid")
    pid_part = str(pid) if isinstance(pid, int) else "nopid"
    material = f"{surface}|{socket_path}|{herdr_ws}|{pid_part}"
    digest = hashlib.sha256(material.encode("utf-8")).hexdigest()[:16]
    surface_slug = re.sub(r"[^A-Za-z0-9_.-]+", "_", str(surface))[:48] or "host"
    return f"{surface_slug}-{digest}"


def _binding_path(fp: Optional[Dict[str, Any]] = None) -> str:
    return os.path.join(_state_dir(), f"parent-{_parent_key(fp)}.json")


def _binding_matches_fingerprint(data: Dict[str, Any], fp: Dict[str, Any]) -> bool:
    """True when a persisted binding belongs to the invoking fingerprint."""
    if data.get("cmux_surface_id") != fp.get("cmux_surface_id"):
        return False
    if data.get("herdr_socket_path") != fp.get("herdr_socket_path"):
        return False
    stored_ws = data.get("herdr_workspace_id")
    current_ws = fp.get("herdr_workspace_id")
    if stored_ws and current_ws and stored_ws != current_ws:
        return False
    stored_pid = data.get("herdr_server_pid")
    current_pid = fp.get("herdr_server_pid")
    # When both sides know a pid and they disagree, the Herdr server restarted
    # under the same socket path — drop the stale lock and re-resolve.
    if isinstance(stored_pid, int) and isinstance(current_pid, int) and stored_pid != current_pid:
        return False
    return True


def _load_parent_binding(fp: Optional[Dict[str, Any]] = None) -> Optional[str]:
    data_fp = fp if fp is not None else collect_host_fingerprint()
    try:
        with open(_binding_path(data_fp), "r", encoding="utf-8") as handle:
            data = json.load(handle)
        if not isinstance(data, dict):
            return None
        if not _binding_matches_fingerprint(data, data_fp):
            return None
        workspace = data.get("workspace_ref")
        return workspace if isinstance(workspace, str) and workspace else None
    except (OSError, ValueError, TypeError):
        return None


_binding_lock = threading.Lock()


def _save_parent_binding(workspace: str, fp: Optional[Dict[str, Any]] = None) -> None:
    data_fp = fp if fp is not None else collect_host_fingerprint()
    directory = _state_dir()
    os.makedirs(directory, mode=0o700, exist_ok=True)
    payload = {
        "workspace_ref": workspace,
        "cmux_surface_id": data_fp.get("cmux_surface_id"),
        "herdr_socket_path": data_fp.get("herdr_socket_path"),
        "herdr_workspace_id": data_fp.get("herdr_workspace_id"),
        "herdr_server_pid": data_fp.get("herdr_server_pid"),
        "host_fingerprint_key": _parent_key(data_fp),
        "updated_at": time.time(),
    }
    target = _binding_path(data_fp)
    with _binding_lock:
        fd, temporary = tempfile.mkstemp(prefix=".parent-", suffix=".tmp", dir=directory)
        try:
            with os.fdopen(fd, "w", encoding="utf-8") as handle:
                json.dump(payload, handle)
                handle.write("\n")
            os.chmod(temporary, 0o600)
            os.replace(temporary, target)
        finally:
            try:
                os.unlink(temporary)
            except FileNotFoundError:
                pass


def _workspace_is_valid(workspace: str) -> bool:
    try:
        proc = run_cmd(["cmux", "list-status", "--workspace", workspace], timeout=4.0)
        return proc.returncode == 0
    except (OSError, subprocess.SubprocessError):
        return False


def _workspace_from_identify(surface: Optional[str] = None) -> Optional[str]:
    args = ["cmux", "identify"]
    if surface:
        args.extend(["--surface", surface])
    args.append("--json")
    try:
        proc = run_cmd(args, timeout=5.0)
        if proc.returncode != 0 or not proc.stdout:
            return None
        data = _parse_json_payload(proc.stdout)
        if not isinstance(data, dict):
            return None
        for section in (data.get("caller"), data.get("focused")):
            if isinstance(section, dict):
                value = section.get("workspace_ref") or section.get("workspace_id")
                if isinstance(value, str) and value:
                    return value
    except (OSError, ValueError, BridgeError, subprocess.SubprocessError):
        pass
    return None


def _association_path(fp: Optional[Dict[str, Any]] = None) -> str:
    return os.path.join(_state_dir(), f"associations-{_parent_key(fp)}.json")


def _load_association_map(fp: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
    """Load the hybrid pane/session association cache (best-effort)."""
    data_fp = fp if fp is not None else collect_host_fingerprint()
    try:
        with open(_association_path(data_fp), "r", encoding="utf-8") as handle:
            data = json.load(handle)
        if isinstance(data, dict) and data.get("version") == 1 and isinstance(data.get("panes"), dict):
            if not isinstance(data.get("mirrors"), dict):
                data["mirrors"] = {}
            return data
    except (OSError, ValueError, TypeError):
        pass
    return {
        "version": 1,
        "panes": {},
        "mirrors": {},
        "cmux_workspace": None,
        "herdr_socket_path": data_fp.get("herdr_socket_path"),
        "herdr_workspace_id": data_fp.get("herdr_workspace_id"),
        "cmux_surface_id": data_fp.get("cmux_surface_id"),
        "herdr_server_pid": data_fp.get("herdr_server_pid"),
        "host_fingerprint_key": _parent_key(data_fp),
        "updated_at": None,
    }


def _save_association_map(state: Dict[str, Any], fp: Optional[Dict[str, Any]] = None) -> None:
    data_fp = fp if fp is not None else collect_host_fingerprint()
    directory = _state_dir()
    os.makedirs(directory, mode=0o700, exist_ok=True)
    state = dict(state)
    state["version"] = 1
    state["updated_at"] = time.time()
    state["herdr_socket_path"] = data_fp.get("herdr_socket_path")
    state["herdr_workspace_id"] = data_fp.get("herdr_workspace_id")
    state["cmux_surface_id"] = data_fp.get("cmux_surface_id")
    state["herdr_server_pid"] = data_fp.get("herdr_server_pid")
    state["host_fingerprint_key"] = _parent_key(data_fp)
    target = _association_path(data_fp)
    with _binding_lock:
        fd, temporary = tempfile.mkstemp(prefix=".assoc-", suffix=".tmp", dir=directory)
        try:
            with os.fdopen(fd, "w", encoding="utf-8") as handle:
                json.dump(state, handle, indent=2, sort_keys=True)
                handle.write("\n")
            os.chmod(temporary, 0o600)
            os.replace(temporary, target)
        finally:
            try:
                os.unlink(temporary)
            except FileNotFoundError:
                pass


def _nonempty_id(value: Any) -> Optional[str]:
    """Return a stripped non-empty string id, or None."""
    if isinstance(value, str):
        text = value.strip()
        if text:
            return text
    return None


def _prior_for_pane(pane: Pane, previous: Dict[str, Any]) -> Dict[str, Any]:
    """Return the prior association record, or {} when the session instance changed.

    Locks and heuristic-satisfied flags must not reuse across ``session_id``
    identities for the same ``pane_id``.
    """
    prior = previous.get(pane.pane_id)
    if not isinstance(prior, dict):
        return {}
    prior_sid = prior.get("agent_session_id")
    new_sid = pane.agent_session_id
    if prior_sid and new_sid and prior_sid != new_sid:
        return {}
    return prior


def _infer_parent_once(
    pane: Pane,
    snapshot: Optional[Snapshot] = None,
) -> Tuple[Optional[str], Optional[str]]:
    """Prompt-time parent heuristic. Must not run after heuristic_satisfied.

    1. If this pane is the invoking Herdr pane, seed from ``HERDR_TAB_ID`` /
       ``HERDR_WORKSPACE_ID``.
    2. Else if the snapshot has exactly one tab in the pane's workspace, use it.
    Never invent a parent from titles or cwd matching.
    """
    env_pane = os.environ.get("HERDR_PANE_ID")
    if env_pane and env_pane == pane.pane_id:
        tab = _nonempty_id(os.environ.get("HERDR_TAB_ID"))
        ws = _nonempty_id(os.environ.get("HERDR_WORKSPACE_ID")) or _nonempty_id(
            pane.workspace_id
        )
        if tab or ws:
            return tab, ws
    if snapshot is not None:
        ws = _nonempty_id(pane.workspace_id)
        tabs = [
            t
            for t in snapshot.tabs
            if t.tab_id and (not ws or t.workspace_id == ws)
        ]
        if len(tabs) == 1:
            return tabs[0].tab_id, tabs[0].workspace_id or ws
    return None, None


def resolve_association_parents(
    pane: Pane,
    prior: Optional[Dict[str, Any]] = None,
    *,
    snapshot: Optional[Snapshot] = None,
) -> Dict[str, Any]:
    """Two-pass parent resolution for one pane.

    Pass A uses authoritative snapshot ``tab_id`` / ``workspace_id``.
    Pass B (heuristic, once) uses env / sole-tab inference only when the
    provider has not yet emitted parentage and the record is not already
    satisfied. After the first successful association, later empty snapshots
    keep the recorded parent map instead of re-inferring.
    """
    prior = prior if isinstance(prior, dict) else {}
    satisfied = bool(prior.get("heuristic_satisfied"))
    prior_tab = _nonempty_id(prior.get("parent_tab_id") or prior.get("tab_id"))
    prior_ws = _nonempty_id(prior.get("parent_workspace_id") or prior.get("workspace_id"))

    snap_tab = _nonempty_id(pane.tab_id)
    snap_ws = _nonempty_id(pane.workspace_id)
    tab_id = snap_tab
    workspace_id = snap_ws
    used_heuristic = False

    if not tab_id or not workspace_id:
        if satisfied and (prior_tab or prior_ws):
            tab_id = tab_id or prior_tab
            workspace_id = workspace_id or prior_ws
        else:
            inferred_tab, inferred_ws = _infer_parent_once(pane, snapshot)
            if inferred_tab or inferred_ws:
                used_heuristic = True
                tab_id = tab_id or inferred_tab
                workspace_id = workspace_id or inferred_ws

    now_satisfied = bool(satisfied or (tab_id and workspace_id) or used_heuristic)
    return {
        "parent_tab_id": tab_id,
        "parent_workspace_id": workspace_id,
        "heuristic_satisfied": now_satisfied,
        "used_heuristic": used_heuristic,
    }


def locked_display_name(pane: Pane, prior: Optional[Dict[str, Any]] = None) -> Optional[str]:
    """Return the locked title when the association record has a title lock."""
    prior = prior if isinstance(prior, dict) else {}
    if not prior.get("title_lock"):
        return None
    locked = prior.get("locked_title")
    if isinstance(locked, str) and locked.strip():
        return locked.strip()
    return None


def status_write_payload(
    pane: Pane,
    tabs_by_id: Optional[Dict[str, Tab]] = None,
    prior: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    """Build the set-status payload, honoring title lock and parent map."""
    prior = prior if isinstance(prior, dict) else {}
    locked = locked_display_name(pane, prior)
    parent_tab = prior.get("parent_tab_id") if prior.get("heuristic_satisfied") else None
    if not isinstance(parent_tab, str) or not parent_tab.strip():
        parent_tab = None
    icon, color, priority = map_status_to_style(pane.agent_status)
    value = status_value_for_pane(
        pane,
        tabs_by_id,
        locked_title=locked,
        parent_tab_id=parent_tab,
    )
    return {
        "value": value,
        "icon": icon,
        "color": color,
        "priority": priority,
        "title_lock": bool(prior.get("title_lock")),
        "locked_title": locked,
    }


def should_write_status_pill(
    payload: Dict[str, Any],
    prior: Optional[Dict[str, Any]] = None,
) -> bool:
    """Return False when the last written pill is identical (diff-before-write)."""
    if not isinstance(prior, dict):
        return True
    return not (
        prior.get("last_status_value") == payload.get("value")
        and prior.get("last_icon") == payload.get("icon")
        and prior.get("last_color") == payload.get("color")
        and prior.get("last_priority") == payload.get("priority")
    )


def _association_record(
    pane: Pane,
    prior: Dict[str, Any],
    parents: Dict[str, Any],
    write_meta: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    """Build one association cache record, preserving locks and last write."""
    meta = write_meta if isinstance(write_meta, dict) else {}
    title_lock = bool(meta.get("title_lock") if "title_lock" in meta else prior.get("title_lock"))
    locked_title = meta.get("locked_title")
    if title_lock and not locked_title:
        locked_title = prior.get("locked_title") or pane.display_name
    if not title_lock:
        locked_title = None
    return {
        "pane_id": pane.pane_id,
        "association_key": association_key_for_pane(pane),
        "tab_id": pane.tab_id,
        "workspace_id": pane.workspace_id,
        "parent_tab_id": parents.get("parent_tab_id") or pane.tab_id,
        "parent_workspace_id": parents.get("parent_workspace_id") or pane.workspace_id,
        "heuristic_satisfied": bool(parents.get("heuristic_satisfied")),
        "title_lock": title_lock,
        "locked_title": locked_title,
        "last_status_value": meta.get("last_status_value", prior.get("last_status_value")),
        "last_icon": meta.get("last_icon", prior.get("last_icon")),
        "last_color": meta.get("last_color", prior.get("last_color")),
        "last_priority": meta.get("last_priority", prior.get("last_priority")),
        "agent": pane.agent,
        "agent_status": pane.agent_status,
        "status_key": pane.status_key,
        "label": pane.label,
        "cwd": pane.cwd,
        "focused": pane.focused,
        "agent_session_path": pane.agent_session_path,
        "agent_session_id": pane.agent_session_id,
        "agent_session_kind": pane.agent_session_kind,
        "revision": pane.revision,
        "first_seen_at": prior.get("first_seen_at") or time.time(),
        "last_seen_at": time.time(),
    }


def update_association_map(
    snapshot: Snapshot,
    *,
    cmux_workspace: Optional[str] = None,
    write_meta: Optional[Dict[str, Dict[str, Any]]] = None,
) -> Dict[str, Any]:
    """Rewrite the hybrid association cache from a live Herdr snapshot.

    Production data pattern:
    - parent binding (outer cmux workspace) is locked separately per host fingerprint
    - this map tracks inner pane_id → status_key/session/status for restore + pruning
    - parent map + title-lock + heuristic-satisfied are explicit fields
    - treated as cache only; never authoritative restore state for native cmux
    """
    fp = collect_host_fingerprint()
    state = _load_association_map(fp)
    previous = state.get("panes") if isinstance(state.get("panes"), dict) else {}
    meta_by_pane = write_meta if isinstance(write_meta, dict) else {}
    live: Dict[str, Any] = {}
    for pane in snapshot.panes:
        if not pane.pane_id:
            continue
        prior = _prior_for_pane(pane, previous)
        meta = meta_by_pane.get(pane.pane_id)
        if not isinstance(meta, dict):
            meta = {}
        parents = resolve_association_parents(pane, prior, snapshot=snapshot)
        if meta.get("parent_tab_id") or meta.get("heuristic_satisfied") is not None:
            parents = {
                "parent_tab_id": meta.get("parent_tab_id", parents["parent_tab_id"]),
                "parent_workspace_id": meta.get(
                    "parent_workspace_id", parents["parent_workspace_id"]
                ),
                "heuristic_satisfied": bool(
                    meta.get("heuristic_satisfied", parents["heuristic_satisfied"])
                ),
            }
        live[pane.pane_id] = _association_record(pane, prior, parents, meta)
    pruned = sorted(set(previous) - set(live))
    state["panes"] = live
    if not isinstance(state.get("mirrors"), dict):
        state["mirrors"] = {}
    state["cmux_workspace"] = cmux_workspace or state.get("cmux_workspace")
    state["pruned_pane_ids"] = pruned
    _save_association_map(state, fp)
    return {
        "path": _association_path(fp),
        "pane_count": len(live),
        "pruned": pruned,
        "cmux_workspace": state.get("cmux_workspace"),
        "host_fingerprint_key": _parent_key(fp),
    }


def set_title_lock(
    pane_id: str,
    *,
    locked: bool,
    title: Optional[str] = None,
) -> Dict[str, Any]:
    """Set or clear the native-title lock on one association record.

    Creates a stub record when the pane is not yet in the cache. Idempotent.
    """
    if not pane_id or not str(pane_id).strip():
        raise BridgeError("pane_id is required to lock or unlock a title")
    pane_id = str(pane_id).strip()
    fp = collect_host_fingerprint()
    state = _load_association_map(fp)
    panes = state.get("panes") if isinstance(state.get("panes"), dict) else {}
    entry = panes.get(pane_id) if isinstance(panes.get(pane_id), dict) else {}
    if not entry:
        entry = {
            "pane_id": pane_id,
            "status_key": f"{STATUS_PREFIX}{pane_id}",
            "association_key": pane_id,
        }
    if locked:
        entry["title_lock"] = True
        if title and str(title).strip():
            entry["locked_title"] = str(title).strip()
        elif not entry.get("locked_title"):
            entry["locked_title"] = entry.get("label") or pane_id
    else:
        entry["title_lock"] = False
        entry.pop("locked_title", None)
    panes[pane_id] = entry
    state["panes"] = panes
    _save_association_map(state, fp)
    return entry


def format_associations(state: Optional[Dict[str, Any]] = None) -> str:
    data = state or _load_association_map()
    panes = data.get("panes") if isinstance(data.get("panes"), dict) else {}
    mirrors = data.get("mirrors") if isinstance(data.get("mirrors"), dict) else {}
    lines = [
        f"associations: {len(panes)} panes, {len(mirrors)} mirrored surfaces",
        f"  cmux_workspace={data.get('cmux_workspace') or '-'}",
        f"  herdr_workspace={data.get('herdr_workspace_id') or '-'}",
        f"  surface={data.get('cmux_surface_id') or '-'}",
        f"  herdr_pid={data.get('herdr_server_pid') or '-'}",
        f"  fingerprint={data.get('host_fingerprint_key') or _parent_key()}",
        f"  file={_association_path()}",
    ]
    for pane_id in sorted(panes):
        entry = panes[pane_id] if isinstance(panes[pane_id], dict) else {}
        session = entry.get("agent_session_path") or entry.get("agent_session_id") or "-"
        if isinstance(session, str) and len(session) > 60:
            session = "…" + session[-57:]
        lock = "Y" if entry.get("title_lock") else "n"
        heur = "Y" if entry.get("heuristic_satisfied") else "n"
        parent = entry.get("parent_tab_id") or entry.get("tab_id") or "-"
        lines.append(
            f"  {pane_id:10}  {(entry.get('agent_status') or '?'):8}  "
            f"{(entry.get('agent') or '-'):8}  {entry.get('status_key') or '-'}  "
            f"parent={parent}  lock={lock}  heur={heur}  session={session}"
        )
    if mirrors:
        lines.append("mirrors:")
        for pane_id in sorted(mirrors):
            entry = mirrors[pane_id] if isinstance(mirrors[pane_id], dict) else {}
            lines.append(
                f"  {pane_id:10}  {(entry.get('role') or '?'):8}  "
                f"{entry.get('cmux_surface_id') or '-'}  {entry.get('title') or '-'}"
            )
    return "\n".join(lines)


def resolve_cmux_workspace() -> Optional[str]:
    """Resolve and lock the outer cmux workspace for this host fingerprint.

    Requires CMUX_SURFACE_ID + HERDR_SOCKET_PATH. Never probes the bare focused
    workspace when fingerprint pieces are missing (that would bind a random host).
    """
    if not which("cmux"):
        return None

    fp = require_host_fingerprint()

    bound = _load_parent_binding(fp)
    if bound and _workspace_is_valid(bound):
        return bound

    surface = fp.get("cmux_surface_id")
    resolved = _workspace_from_identify(surface if isinstance(surface, str) else None)
    if not resolved:
        inherited_workspace = os.environ.get("CMUX_WORKSPACE_ID")
        if inherited_workspace and _workspace_is_valid(inherited_workspace):
            resolved = inherited_workspace

    if resolved and _workspace_is_valid(resolved):
        _save_parent_binding(resolved, fp)
        return resolved
    return None

def list_cmux_herdr_keys(workspace: Optional[str] = None) -> List[str]:
    ws = workspace or resolve_cmux_workspace()
    if not ws:
        return []
    proc = cmux_cmd(["list-status"], workspace=ws)
    if proc.returncode != 0:
        return []
    keys: List[str] = []
    for line in (proc.stdout or "").splitlines():
        line = line.strip()
        if not line or "=" not in line:
            continue
        key = line.split("=", 1)[0].strip()
        if key.startswith(STATUS_PREFIX):
            keys.append(key)
    return keys


def clear_cmux_herdr_statuses(workspace: Optional[str] = None) -> List[str]:
    ws = workspace or resolve_cmux_workspace()
    if not ws:
        raise BridgeError("no cmux workspace resolved; is cmux running?")
    cleared: List[str] = []
    for key in list_cmux_herdr_keys(ws):
        proc = cmux_cmd(["clear-status", key], workspace=ws)
        if proc.returncode == 0:
            cleared.append(key)
    return cleared


def sync_to_cmux(
    snapshot: Optional[Snapshot] = None,
    *,
    workspace: Optional[str] = None,
    clear_stale: bool = True,
    set_progress: bool = True,
    log: bool = True,
) -> Dict[str, Any]:
    """Mirror herdr agent panes into cmux set-status pills.

    Returns a summary dict.
    """
    snap = snapshot or fetch_snapshot()
    fingerprint_warning: Optional[str] = None
    if workspace:
        missing = fingerprint_missing_fields()
        if missing:
            fingerprint_warning = (
                "incomplete host fingerprint (missing "
                + ", ".join(missing)
                + "); using --workspace override without a stable host key "
                "(association files may collide across outer surfaces)"
            )
            if log:
                print(f"cmux-herdr: {fingerprint_warning}", file=sys.stderr)
        ws = workspace
    else:
        ws = resolve_cmux_workspace()
    if not ws:
        raise BridgeError(
            "could not resolve cmux workspace for status sync "
            "(need CMUX_SURFACE_ID + HERDR_SOCKET_PATH, or pass --workspace)"
        )

    writer = writer_status()
    fp = collect_host_fingerprint()
    if writer["native_live"]:
        association = update_association_map(snap, cmux_workspace=ws)
        _log_native_skip_once(log, fp)
        summary_line = (
            f"herdr sync: skipped (native attachment live) ws={ws} "
            f"fingerprint={_parent_key(fp)}"
        )
        if log:
            try:
                cmux_cmd(["log", summary_line], workspace=ws)
            except Exception:
                pass
        result = {
            "workspace": ws,
            "applied": [],
            "skipped_unchanged": [],
            "stale_cleared": [],
            "counts": {
                "working": 0,
                "idle": 0,
                "done": 0,
                "blocked": 0,
                "unknown": 0,
                "other": 0,
            },
            "progress": None,
            "errors": [],
            "summary": summary_line,
            "pane_count": len(snap.panes),
            "agent_count": 0,
            "associations": association,
            "host_fingerprint": fp,
            "host_fingerprint_key": _parent_key(fp),
            "writer": writer["writer"],
            "native_live": True,
            "skipped_reason": "native_live",
        }
        if fingerprint_warning:
            result["fingerprint_warning"] = fingerprint_warning
        return result

    tabs_by_id = {t.tab_id: t for t in snap.tabs}
    # Prefer panes that look like agents; fall back to all non-unknown
    panes = [p for p in snap.panes if p.agent]
    if not panes:
        panes = [p for p in snap.panes if p.agent_status in ("working", "idle", "done", "blocked")]

    prior_state = _load_association_map(fp)
    previous = prior_state.get("panes") if isinstance(prior_state.get("panes"), dict) else {}
    desired_keys = set()
    counts = {"working": 0, "idle": 0, "done": 0, "blocked": 0, "unknown": 0, "other": 0}
    applied: List[str] = []
    skipped_unchanged: List[str] = []
    errors: List[str] = []
    write_meta: Dict[str, Dict[str, Any]] = {}
    lock_after_write = auto_lock_titles()

    for pane in panes:
        st = (pane.agent_status or "unknown").lower()
        if st in counts:
            counts[st] += 1
        else:
            counts["other"] += 1
        key = pane.status_key
        desired_keys.add(key)
        prior = _prior_for_pane(pane, previous)
        parents = resolve_association_parents(pane, prior, snapshot=snap)
        prior_view = dict(prior)
        prior_view.update(parents)
        payload = status_write_payload(pane, tabs_by_id, prior_view)
        if not should_write_status_pill(payload, prior):
            skipped_unchanged.append(key)
            write_meta[pane.pane_id] = {
                "last_status_value": prior.get("last_status_value"),
                "last_icon": prior.get("last_icon"),
                "last_color": prior.get("last_color"),
                "last_priority": prior.get("last_priority"),
                "title_lock": bool(prior.get("title_lock")),
                "locked_title": prior.get("locked_title"),
                **parents,
            }
            continue
        proc = cmux_cmd(
            [
                "set-status",
                key,
                payload["value"],
                "--icon",
                payload["icon"],
                "--color",
                payload["color"],
                "--priority",
                str(payload["priority"]),
            ],
            workspace=ws,
        )
        if proc.returncode == 0:
            applied.append(key)
            lock = bool(prior.get("title_lock") or lock_after_write)
            locked_title = payload.get("locked_title")
            if lock and not locked_title:
                locked_title = pane.display_name
            write_meta[pane.pane_id] = {
                "last_status_value": payload["value"],
                "last_icon": payload["icon"],
                "last_color": payload["color"],
                "last_priority": payload["priority"],
                "title_lock": lock,
                "locked_title": locked_title if lock else None,
                **parents,
            }
        else:
            errors.append((proc.stderr or proc.stdout or key).strip())

    stale_cleared: List[str] = []
    if clear_stale:
        for key in list_cmux_herdr_keys(ws):
            if key not in desired_keys:
                proc = cmux_cmd(["clear-status", key], workspace=ws)
                if proc.returncode == 0:
                    stale_cleared.append(key)

    progress = None
    active = counts["working"]
    total = counts["working"] + counts["idle"] + counts["done"] + counts["blocked"]
    if set_progress and total > 0:
        # Progress = fraction still working (busy bar)
        progress = round(active / total, 3)
        label = f"herdr {active}/{total} working"
        cmux_cmd(
            ["set-progress", str(progress), "--label", label],
            workspace=ws,
        )

    summary_line = (
        f"herdr sync: {len(applied)} panes → cmux ws={ws} "
        f"(working={counts['working']} idle={counts['idle']} "
        f"done={counts['done']} blocked={counts['blocked']} unknown={counts['unknown']}"
        + (f" unchanged={len(skipped_unchanged)}" if skipped_unchanged else "")
        + ")"
    )
    if log:
        try:
            cmux_cmd(["log", summary_line], workspace=ws)
        except Exception:
            pass

    association = update_association_map(snap, cmux_workspace=ws, write_meta=write_meta)
    result: Dict[str, Any] = {
        "workspace": ws,
        "applied": applied,
        "skipped_unchanged": skipped_unchanged,
        "stale_cleared": stale_cleared,
        "counts": counts,
        "progress": progress,
        "errors": errors,
        "summary": summary_line,
        "pane_count": len(snap.panes),
        "agent_count": len(panes),
        "associations": association,
        "host_fingerprint": fp,
        "host_fingerprint_key": _parent_key(fp),
        "writer": writer["writer"],
        "native_live": False,
    }
    if fingerprint_warning:
        result["fingerprint_warning"] = fingerprint_warning
    return result


def format_tree(snapshot: Optional[Snapshot] = None) -> str:
    snap = snapshot or fetch_snapshot()
    panes_by_tab: Dict[str, List[Pane]] = {}
    for p in snap.panes:
        panes_by_tab.setdefault(p.tab_id, []).append(p)
    tabs_by_ws: Dict[str, List[Tab]] = {}
    for t in snap.tabs:
        tabs_by_ws.setdefault(t.workspace_id, []).append(t)

    # Orphan tabs from panes
    for p in snap.panes:
        if p.tab_id and p.tab_id not in {t.tab_id for t in snap.tabs}:
            tabs_by_ws.setdefault(p.workspace_id, []).append(
                Tab(
                    tab_id=p.tab_id,
                    workspace_id=p.workspace_id,
                    label=None,
                    agent_status=p.agent_status,
                )
            )

    lines: List[str] = []
    workspaces = snap.workspaces
    if not workspaces:
        # synthesize from panes
        seen = []
        for p in snap.panes:
            if p.workspace_id not in seen:
                seen.append(p.workspace_id)
                workspaces.append(Workspace(workspace_id=p.workspace_id))

    for ws in workspaces:
        mark = "●" if ws.focused else "○"
        label = ws.label or ws.workspace_id
        lines.append(
            f"{mark} workspace {ws.workspace_id}  {label}  "
            f"[{ws.agent_status}] tabs={ws.tab_count or len(tabs_by_ws.get(ws.workspace_id, []))} "
            f"panes={ws.pane_count}"
        )
        tabs = tabs_by_ws.get(ws.workspace_id, [])
        # stable-ish order by number then id
        tabs = sorted(tabs, key=lambda t: (t.number is None, t.number or 0, t.tab_id))
        # dedup tab ids
        seen_tabs = set()
        for tab in tabs:
            if tab.tab_id in seen_tabs:
                continue
            seen_tabs.add(tab.tab_id)
            tmark = "▶" if tab.focused else "·"
            tlabel = tab.label or tab.tab_id
            lines.append(
                f"  {tmark} tab {tab.tab_id}  {tlabel}  [{tab.agent_status}] "
                f"panes={tab.pane_count or len(panes_by_tab.get(tab.tab_id, []))}"
            )
            for pane in sorted(
                panes_by_tab.get(tab.tab_id, []),
                key=lambda p: (not p.focused, p.pane_id),
            ):
                pmark = "▶" if pane.focused else "·"
                agent = pane.agent or "-"
                name = pane.label or ""
                title = ""
                if not name and pane.terminal_title:
                    title = pane.terminal_title[:50]
                display = name or title or ""
                cwd = pane.cwd or ""
                lines.append(
                    f"    {pmark} pane {pane.pane_id}  {agent}/{pane.agent_status}"
                    + (f"  \"{display}\"" if display else "")
                    + (f"  {cwd}" if cwd else "")
                )
    if not lines:
        lines.append("(no herdr workspaces/panes)")
    return "\n".join(lines)


def dual_status() -> Dict[str, Any]:
    """Collect dual cmux+herdr context for `status` subcommand."""
    fp = collect_host_fingerprint()
    info: Dict[str, Any] = {
        "herdr": {
            "env": os.environ.get("HERDR_ENV"),
            "pane_id": os.environ.get("HERDR_PANE_ID"),
            "tab_id": os.environ.get("HERDR_TAB_ID"),
            "workspace_id": os.environ.get("HERDR_WORKSPACE_ID"),
            "socket_path": os.environ.get("HERDR_SOCKET_PATH"),
            "server_pid": fp.get("herdr_server_pid"),
            "socket_exists": bool(
                os.environ.get("HERDR_SOCKET_PATH")
                and os.path.exists(os.environ["HERDR_SOCKET_PATH"])
            ),
            "cli": which("herdr"),
            "available": False,
            "api": {"ok": False},
        },
        "cmux": {
            "surface_id": os.environ.get("CMUX_SURFACE_ID"),
            "tab_id": os.environ.get("CMUX_TAB_ID"),
            "workspace_id": os.environ.get("CMUX_WORKSPACE_ID"),
            "socket_path": os.environ.get("CMUX_SOCKET_PATH"),
            "socket_exists": bool(
                os.environ.get("CMUX_SOCKET_PATH")
                and os.path.exists(os.environ["CMUX_SOCKET_PATH"])
            ),
            "cli": which("cmux"),
            "available": False,
            "resolved_workspace": None,
        },
        "host_fingerprint": fp,
        "host_fingerprint_key": _parent_key(fp),
        "host_fingerprint_missing": fingerprint_missing_fields(fp),
        "nested": bool(os.environ.get("HERDR_ENV"))
        and bool(os.environ.get("CMUX_SOCKET_PATH") or os.environ.get("CMUX_WORKSPACE_ID")),
        "writer": writer_status(fp),
    }
    info["herdr"]["available"] = herdr_available()
    info["cmux"]["available"] = cmux_available()
    try:
        from .cmux_herdr_api import HerdrApi
    except ImportError:
        from cmux_herdr_api import HerdrApi
    try:
        ping = HerdrApi().call("ping")
        info["herdr"]["api"] = {"ok": bool(ping.ok), "via": ping.via}
    except Exception as exc:  # noqa: BLE001
        info["herdr"]["api"] = {"ok": False, "error": str(exc)}
    if info["cmux"]["available"]:
        try:
            info["cmux"]["resolved_workspace"] = resolve_cmux_workspace()
        except Exception as exc:
            info["cmux"]["resolve_error"] = str(exc)
    if info["herdr"]["available"]:
        try:
            snap = fetch_snapshot()
            counts: Dict[str, int] = {}
            for p in snap.panes:
                st = p.agent_status or "unknown"
                counts[st] = counts.get(st, 0) + 1
            info["herdr"]["pane_count"] = len(snap.panes)
            info["herdr"]["tab_count"] = len(snap.tabs)
            info["herdr"]["workspace_count"] = len(snap.workspaces)
            info["herdr"]["status_counts"] = counts
            info["herdr"]["agent_count"] = len([p for p in snap.panes if p.agent])
        except Exception as exc:
            info["herdr"]["error"] = str(exc)
    return info


def focus_tab(tab_id_or_label: str) -> str:
    tabs = fetch_tabs()
    target = None
    for t in tabs:
        if t.tab_id == tab_id_or_label:
            target = t.tab_id
            break
    if target is None:
        needle = tab_id_or_label.lower()
        exact = [t for t in tabs if (t.label or "").lower() == needle]
        if len(exact) == 1:
            target = exact[0].tab_id
        elif len(exact) > 1:
            opts = ", ".join(f"{t.tab_id}({t.label})" for t in exact[:8])
            raise BridgeError(f"ambiguous tab label {tab_id_or_label!r}: {opts}")
        else:
            # Prefix match only (avoid substring false positives like
            # "CMUX" matching "...cmux-herdr...").
            prefixes = [
                t
                for t in tabs
                if (t.label or "").lower().startswith(needle)
            ]
            if len(prefixes) == 1:
                target = prefixes[0].tab_id
            elif len(prefixes) > 1:
                opts = ", ".join(f"{t.tab_id}({t.label})" for t in prefixes[:8])
                raise BridgeError(
                    f"ambiguous tab label {tab_id_or_label!r}: {opts}"
                )
    if target is None:
        raise BridgeError(f"tab not found: {tab_id_or_label}")
    herdr_rpc("tab.focus", {"tab_id": target})
    return target


def focus_pane(pane_id: str) -> str:
    """Focus a pane by id via socket ``agent.focus``.

    Herdr ``pane.focus_direction`` is directional only, so agent focus is
    the id-based path. Do not fall back to ``pane zoom``.
    """
    try:
        herdr_rpc("agent.focus", {"target": pane_id, "pane_id": pane_id})
        return pane_id
    except BridgeError as exc:
        try:
            herdr_rpc("pane.get", {"pane_id": pane_id})
        except BridgeError as inner:
            detail = str(inner).lower()
            if "not found" in detail or "unknown" in detail or "no such" in detail:
                raise BridgeError(f"pane not found: {pane_id}") from inner
        raise BridgeError(str(exc) or f"herdr agent focus failed for pane {pane_id}") from exc


def focus_workspace(workspace_id: str) -> str:
    """Focus a Herdr workspace by id via socket ``workspace.focus``."""
    herdr_rpc("workspace.focus", {"workspace_id": workspace_id})
    return workspace_id


def focus_agent(target: str) -> str:
    """Focus a Herdr agent by pane id / label via socket ``agent.focus``."""
    herdr_rpc("agent.focus", {"target": target, "pane_id": target})
    return target


def _append_read_flags(
    args: List[str],
    *,
    source: Optional[str] = None,
    lines: Optional[int] = None,
    format: Optional[str] = None,
    ansi: bool = False,
    raw: bool = False,
) -> List[str]:
    """Append shared ``pane read`` / ``agent read`` flags to *args*."""
    if source:
        args.extend(["--source", source])
    if lines is not None:
        args.extend(["--lines", str(int(lines))])
    if format:
        args.extend(["--format", format])
    if ansi:
        args.append("--ansi")
    if raw:
        args.append("--raw")
    return args


def read_pane(
    pane_id: str,
    *,
    source: Optional[str] = None,
    lines: Optional[int] = None,
    format: Optional[str] = None,
    ansi: bool = False,
    raw: bool = False,
) -> subprocess.CompletedProcess:
    """Thin wrapper over ``herdr pane read`` (stdout is text, not JSON)."""
    if not which("herdr"):
        raise BridgeError("herdr not found on PATH")
    args = _append_read_flags(
        ["herdr", "pane", "read", pane_id],
        source=source,
        lines=lines,
        format=format,
        ansi=ansi,
        raw=raw,
    )
    return run_cmd(args)


def read_agent(
    target: str,
    *,
    source: Optional[str] = None,
    lines: Optional[int] = None,
    format: Optional[str] = None,
    ansi: bool = False,
) -> subprocess.CompletedProcess:
    """Thin wrapper over ``herdr agent read`` (stdout is text, not JSON)."""
    if not which("herdr"):
        raise BridgeError("herdr not found on PATH")
    args = _append_read_flags(
        ["herdr", "agent", "read", target],
        source=source,
        lines=lines,
        format=format,
        ansi=ansi,
        raw=False,
    )
    return run_cmd(args)


LAUNCH_AGENT_LABEL = "com.cmux-herdr.watch"


def default_herdr_socket_path() -> str:
    """Return the usual Herdr default socket path (does not invent a host)."""
    return os.path.expanduser("~/.config/herdr/herdr.sock")


def sidebar_install_path() -> str:
    """Return the optional cmux custom-sidebar install path."""
    return os.path.expanduser("~/.config/cmux/sidebars/herdr.swift")


def _socket_stat_info(path: str) -> Dict[str, Any]:
    """Return existence / mode / owner for a socket path (best-effort)."""
    info: Dict[str, Any] = {
        "path": path,
        "exists": os.path.exists(path),
        "mode": None,
        "owner": None,
        "uid": None,
        "gid": None,
    }
    if not info["exists"]:
        return info
    try:
        st = os.stat(path)
        info["mode"] = oct(st.st_mode & 0o777)
        info["uid"] = st.st_uid
        info["gid"] = st.st_gid
        try:
            import pwd  # stdlib; may be unavailable on some platforms

            info["owner"] = pwd.getpwuid(st.st_uid).pw_name
        except Exception:
            info["owner"] = str(st.st_uid)
    except OSError as exc:
        info["stat_error"] = str(exc)
    return info


def _herdr_cli_version() -> Optional[str]:
    """Best-effort ``herdr --version`` string, or None when unavailable."""
    if not which("herdr"):
        return None
    try:
        proc = run_cmd(["herdr", "--version"], timeout=5.0)
    except (OSError, subprocess.SubprocessError):
        return None
    text = (proc.stdout or proc.stderr or "").strip()
    if proc.returncode != 0 or not text:
        return None
    return text.splitlines()[0].strip()


def _launchagent_status() -> Dict[str, Any]:
    """Best-effort LaunchAgent loaded check (macOS only; skip elsewhere)."""
    label = LAUNCH_AGENT_LABEL
    if sys.platform != "darwin":
        return {
            "label": label,
            "checked": False,
            "skipped": True,
            "reason": f"not macOS (platform={sys.platform})",
            "loaded": None,
        }
    if not which("launchctl"):
        return {
            "label": label,
            "checked": False,
            "skipped": True,
            "reason": "launchctl not on PATH",
            "loaded": None,
        }
    try:
        uid = os.getuid()
        proc = run_cmd(["launchctl", "print", f"gui/{uid}/{label}"], timeout=5.0)
        if proc.returncode == 0:
            return {
                "label": label,
                "checked": True,
                "skipped": False,
                "loaded": True,
            }
        proc2 = run_cmd(["launchctl", "list", label], timeout=5.0)
        loaded = proc2.returncode == 0
        return {
            "label": label,
            "checked": True,
            "skipped": False,
            "loaded": loaded,
        }
    except (OSError, subprocess.SubprocessError) as exc:
        return {
            "label": label,
            "checked": False,
            "skipped": True,
            "reason": str(exc),
            "loaded": None,
        }


def dry_sync_preview() -> Dict[str, Any]:
    """One-shot dry sync summary: fetch topology, do not write cmux statuses."""
    if not herdr_available():
        reasons: List[str] = []
        if not which("herdr"):
            reasons.append("herdr not on PATH")
        else:
            sock = os.environ.get("HERDR_SOCKET_PATH")
            if sock and not os.path.exists(sock):
                reasons.append(f"HERDR_SOCKET_PATH missing on disk: {sock}")
            elif not sock and not os.path.exists(default_herdr_socket_path()):
                reasons.append(
                    "no HERDR_SOCKET_PATH and default socket absent "
                    f"({default_herdr_socket_path()})"
                )
            else:
                reasons.append("herdr status probe failed / socket unhealthy")
        return {"skipped": True, "reasons": reasons}

    try:
        snap = fetch_snapshot()
    except BridgeError as exc:
        return {"skipped": True, "reasons": [f"snapshot failed: {exc}"]}

    agent_panes = [p for p in snap.panes if p.agent]
    if not agent_panes:
        agent_panes = [
            p
            for p in snap.panes
            if p.agent_status in ("working", "idle", "done", "blocked")
        ]
    counts: Dict[str, int] = {
        "working": 0,
        "idle": 0,
        "done": 0,
        "blocked": 0,
        "unknown": 0,
    }
    for pane in agent_panes:
        st = (pane.agent_status or "unknown").lower()
        if st in counts:
            counts[st] += 1
        else:
            counts["unknown"] += 1

    workspace: Optional[str] = None
    workspace_error: Optional[str] = None
    missing = fingerprint_missing_fields()
    if missing:
        workspace_error = (
            "incomplete host fingerprint (missing "
            + ", ".join(missing)
            + "); dry sync cannot auto-resolve outer workspace"
        )
    else:
        try:
            workspace = resolve_cmux_workspace()
        except BridgeError as exc:
            workspace_error = str(exc)
        except Exception as exc:  # noqa: BLE001
            workspace_error = str(exc)

    return {
        "skipped": False,
        "pane_count": len(snap.panes),
        "agent_count": len(agent_panes),
        "counts": counts,
        "workspace": workspace,
        "workspace_error": workspace_error,
        "summary": (
            f"dry sync: {len(agent_panes)} agent panes "
            f"(working={counts['working']} idle={counts['idle']} "
            f"done={counts['done']} blocked={counts['blocked']})"
            + (f" → ws={workspace}" if workspace else "")
        ),
    }


def _state_binding_detail(
    state_dir: str,
    state_exists: bool,
    fingerprint_complete: bool,
    binding_exists: bool,
    bound_workspace: Optional[str],
) -> str:
    """Human-readable state/binding summary for doctor output."""
    prefix = f"state={state_dir} exists={state_exists}"
    if not fingerprint_complete:
        return f"{prefix}; binding check skipped (incomplete fingerprint)"
    if bound_workspace:
        return f"{prefix}; matching parent binding=yes ({bound_workspace})"
    if binding_exists:
        return f"{prefix}; matching parent binding=file present (workspace unset)"
    return f"{prefix}; matching parent binding=none"


def diagnose_install() -> Dict[str, Any]:
    """Diagnose third-party install health for ``cmux-herdr doctor``.

    Hard failures (``ok=False``):
    - herdr missing from PATH
    - incomplete host fingerprint while claiming a nested env (``HERDR_ENV`` set)
    """
    checks: List[Dict[str, Any]] = []
    hard_failures: List[str] = []

    herdr_path = which("herdr")
    version = _herdr_cli_version() if herdr_path else None
    herdr_ok = bool(herdr_path)
    checks.append(
        {
            "name": "herdr_cli",
            "ok": herdr_ok,
            "hard": True,
            "path": herdr_path,
            "version": version,
            "detail": (
                f"herdr on PATH ({version or 'version unknown'})"
                if herdr_ok
                else "herdr not found on PATH"
            ),
        }
    )
    if not herdr_ok:
        hard_failures.append("herdr not found on PATH")

    env_socket = os.environ.get("HERDR_SOCKET_PATH")
    default_socket = default_herdr_socket_path()
    socket_path = env_socket or default_socket
    socket_info = _socket_stat_info(socket_path)
    socket_info["from_env"] = bool(env_socket)
    socket_info["default_path"] = default_socket
    # Socket presence is advisory here; hard fails come from herdr-on-PATH /
    # nested fingerprint completeness. Still mark ok=False when the env socket
    # is explicitly set but missing on disk.
    socket_check_ok = bool(socket_info["exists"]) if env_socket else True
    checks.append(
        {
            "name": "herdr_socket",
            "ok": socket_check_ok,
            "hard": False,
            "socket": socket_info,
            "detail": (
                f"socket {socket_path} exists "
                f"mode={socket_info.get('mode') or '-'} "
                f"owner={socket_info.get('owner') or '-'}"
                if socket_info["exists"]
                else (
                    f"HERDR_SOCKET_PATH set but missing: {socket_path}"
                    if env_socket
                    else f"default socket absent: {socket_path} (ok if unused)"
                )
            ),
        }
    )

    api_ok = False
    api_via = None
    api_detail = "Herdr API ping not attempted"
    try:
        from .cmux_herdr_api import HerdrApi
    except ImportError:
        from cmux_herdr_api import HerdrApi
    try:
        ping = HerdrApi().call("ping")
        api_ok = bool(ping.ok)
        api_via = ping.via
        api_detail = f"ping via {ping.via}"
    except Exception as exc:  # noqa: BLE001 — doctor must never crash
        api_detail = f"ping failed: {exc}"
    checks.append(
        {
            "name": "herdr_api",
            "ok": api_ok,
            "hard": False,
            "via": api_via,
            "detail": api_detail,
        }
    )

    fp = collect_host_fingerprint()
    missing = fingerprint_missing_fields(fp)
    claiming_nested = bool(os.environ.get("HERDR_ENV"))
    fingerprint_complete = not missing
    # Never invent surface/socket hosts — only report what the env provides.
    fp_ok = fingerprint_complete or not claiming_nested
    fp_detail = (
        f"complete key={_parent_key(fp)}"
        if fingerprint_complete
        else (
            "incomplete (missing "
            + ", ".join(missing)
            + ")"
            + (
                "; hard fail because HERDR_ENV claims nested context"
                if claiming_nested
                else "; ok for non-nested inspect commands"
            )
        )
    )
    checks.append(
        {
            "name": "host_fingerprint",
            "ok": fp_ok,
            "hard": claiming_nested and not fingerprint_complete,
            "fingerprint": fp,
            "fingerprint_key": _parent_key(fp) if fingerprint_complete else None,
            "missing": missing,
            "claiming_nested": claiming_nested,
            "detail": fp_detail,
        }
    )
    if claiming_nested and not fingerprint_complete:
        hard_failures.append(
            "incomplete host fingerprint while HERDR_ENV claims nested env "
            "(missing " + ", ".join(missing) + ")"
        )

    state_dir = _state_dir()
    state_exists = os.path.isdir(state_dir)
    binding_path = _binding_path(fp) if fingerprint_complete else None
    binding_exists = bool(binding_path and os.path.isfile(binding_path))
    bound_workspace = _load_parent_binding(fp) if fingerprint_complete else None
    checks.append(
        {
            "name": "state_binding",
            "ok": True,  # advisory: missing binding is not a hard failure
            "hard": False,
            "state_dir": state_dir,
            "state_dir_exists": state_exists,
            "binding_path": binding_path,
            "binding_exists": binding_exists,
            "bound_workspace": bound_workspace,
            "detail": _state_binding_detail(
                state_dir,
                state_exists,
                fingerprint_complete,
                binding_exists,
                bound_workspace,
            ),
        }
    )

    writer = writer_status(fp)
    if writer["native_live"]:
        writer_detail = (
            f"writer=native (plugin projection skipped); "
            f"marker={writer['marker_path']}"
        )
    elif writer["force_plugin"] and writer["native_detected"]:
        writer_detail = (
            f"writer=plugin-forced ({FORCE_PLUGIN_ENV}=1 overrides native live)"
        )
    else:
        writer_detail = "writer=plugin (no native attachment detected)"
    checks.append(
        {
            "name": "writer",
            "ok": True,
            "hard": False,
            "writer": writer,
            "detail": writer_detail,
        }
    )

    launch = _launchagent_status()
    checks.append(
        {
            "name": "launch_agent",
            "ok": True if launch.get("skipped") else bool(launch.get("loaded")),
            "hard": False,
            "launch_agent": launch,
            "detail": (
                f"LaunchAgent {launch['label']}: skipped ({launch.get('reason')})"
                if launch.get("skipped")
                else (
                    f"LaunchAgent {launch['label']}: "
                    + ("loaded" if launch.get("loaded") else "not loaded")
                )
            ),
        }
    )

    sidebar_path = sidebar_install_path()
    sidebar_exists = os.path.isfile(sidebar_path)
    checks.append(
        {
            "name": "sidebar",
            "ok": True,  # optional install
            "hard": False,
            "path": sidebar_path,
            "exists": sidebar_exists,
            "detail": (
                f"sidebar present: {sidebar_path}"
                if sidebar_exists
                else f"sidebar absent (optional): {sidebar_path}"
            ),
        }
    )

    dry = dry_sync_preview()
    dry_ok = not dry.get("skipped")
    checks.append(
        {
            "name": "dry_sync",
            "ok": dry_ok or not herdr_ok,  # if herdr missing, already hard-failed
            "hard": False,
            "dry_sync": dry,
            "detail": (
                dry.get("summary")
                if dry_ok
                else "dry sync skipped: "
                + "; ".join(dry.get("reasons") or ["unknown"])
            ),
        }
    )

    ok = not hard_failures
    return {
        "ok": ok,
        "hard_failures": hard_failures,
        "checks": checks,
        "claiming_nested": claiming_nested,
        "host_fingerprint": fp,
        "host_fingerprint_missing": missing,
    }


def split_pane(direction: str = "right") -> Any:
    if direction not in ("right", "down"):
        raise BridgeError("--direction must be right or down")
    return herdr_rpc("pane.split", {"direction": direction}) or "ok"
