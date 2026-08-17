#!/usr/bin/env python3
"""Cmux-tmux attach / detach / restore / observability mapped onto Herdr.

Gold standard: ``RemoteTmuxController+Attach``, ``RemoteTmuxAttachWindowTarget``,
``RemoteTmuxWindowRegistry``, ``RemoteTmuxPostAttachAction``,
``RemoteTmuxMirrorTeardownReason``, and ``TerminalController+RemoteTmux``
(``remote.tmux.*``).

Herdr has no SSH, ControlMaster, or ``tmux -CC``. The wire is the Unix
socket (``session.snapshot`` / ``events.subscribe`` / ``pane.*``). Same
*user* contract:

- beta gate (``Mirror tabs like ssh-tmux``)
- one endpoint cannot split across windows (existing-mirror affinity)
- re-entrant attach is rejected
- a live connection is reused; a dead one is replaced (never cache a
  connection that failed to start)
- detach leaves the Herdr session alive (never ``server.stop``)
- last-tab close and window-quit both detach — Herdr has no safe
  kill-session analogue of tmux ``markKillSessionsOnClose``
- restore after restart **reattaches** (fresh snapshot); it never
  replays a stale Bonsplit tree
- control-socket twins: ``remote.herdr.sessions`` / ``attach`` /
  ``mirror`` / ``window`` / ``detach`` / ``state`` / ``pane_surfaces`` /
  ``pane_grids``
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Dict, List, Optional, Sequence, Tuple

SOCKET_METHODS = (
    "remote.herdr.sessions",
    "remote.herdr.attach",
    "remote.herdr.mirror",
    "remote.herdr.window",
    "remote.herdr.detach",
    "remote.herdr.state",
    "remote.herdr.pane_surfaces",
    "remote.herdr.pane_grids",
)

_SESSION_METHODS = frozenset(
    {
        "remote.herdr.attach",
        "remote.herdr.detach",
        "remote.herdr.state",
        "remote.herdr.pane_surfaces",
        "remote.herdr.pane_grids",
    }
)

# Catalog key twin of tmux ``betaFeatures.remoteTmux``. Default off until
# the live AppKit apply is stable (PR7 acceptance).
SETTING_KEY = "betaFeatures.remoteHerdrMirror"

TEARDOWN_SESSION_ENDED = "session_ended"
TEARDOWN_EXPLICIT_DETACH = "explicit_detach"

POST_RESEED = "reseed"
POST_APPLY_CLIENT_SIZE = "apply_client_size"


def endpoint_hash(socket_path: str) -> str:
    """Stable endpoint id (tmux ``connectionHash``). Never log the path."""
    return hashlib.sha256(socket_path.encode("utf-8")).hexdigest()[:16]


def has_hidden_character(value: str) -> bool:
    """Reject control / format / separator scalars (tmux socket trust boundary)."""
    for char in value:
        code = ord(char)
        if code < 32 or code == 0x7F:
            return True
        if 0x80 <= code <= 0x9F:
            return True
    return False


def validate_socket_path(value: Optional[str]) -> Optional[str]:
    """Accept an absolute Unix socket path; reject injection-shaped values."""
    if value is None:
        return None
    trimmed = value.strip()
    if not trimmed or not trimmed.startswith("/") or trimmed.startswith("-"):
        return None
    if has_hidden_character(trimmed) or "\x00" in trimmed:
        return None
    return trimmed


def validate_session_name(value: Optional[str]) -> Optional[str]:
    """Require a non-empty session id after trim."""
    if value is None:
        return None
    trimmed = value.strip()
    return trimmed or None


def decode_beta(value: object, default: bool = False) -> bool:
    """Decode the beta flag the way tmux reads ``remoteTmux`` from defaults."""
    if value is None:
        return default
    if isinstance(value, bool):
        return value
    if isinstance(value, (int, float)) and value in (0, 1):
        return bool(value)
    if isinstance(value, str):
        lowered = value.strip().lower()
        if lowered in ("1", "true", "yes", "on"):
            return True
        if lowered in ("0", "false", "no", "off"):
            return False
    return default


@dataclass(frozen=True)
class DiscoveredSession:
    """One Herdr session from ``session.snapshot`` (tmux ``RemoteTmuxSession``)."""

    session_id: str
    name: str
    window_count: int = 0
    attached: bool = False


@dataclass(frozen=True)
class AttachWindowTarget:
    """Window-routing intent, preserved across the discover await.

    Kinds match ``RemoteTmuxAttachWindowTarget``: dedicated new window,
    explicit (resolved or not), or contextual with optional preferred id.
    """

    kind: str
    window_id: Optional[str] = None

    def resolve(
        self,
        existing_mirror_window_id: Optional[str],
        active_window_id: Optional[str],
        is_live: Callable[[str], bool],
    ) -> Optional[str]:
        """Resolve the live destination; existing-host affinity wins.

        A live mirror for this endpoint stays first so one Herdr socket
        cannot be split across cmux windows (tmux attachHost rule).
        """
        if existing_mirror_window_id and is_live(existing_mirror_window_id):
            return existing_mirror_window_id
        if self.kind == "dedicated_new_window":
            return None
        if self.kind == "explicit":
            if self.window_id and is_live(self.window_id):
                return self.window_id
            return None
        if self.kind == "unresolved_explicit":
            return None
        if self.kind == "contextual":
            if self.window_id and is_live(self.window_id):
                return self.window_id
            if active_window_id and is_live(active_window_id):
                return active_window_id
            return None
        return None


def window_target_from_params(
    params: Dict[str, object],
    *,
    dedicated: bool = False,
) -> AttachWindowTarget:
    """Parse socket routing the way ``remoteTmuxAttachWindowTarget`` does.

    A present ``window_id`` key that is null is ``unresolved_explicit`` —
    never fall back to the active window. ``remote.herdr.window`` is
    always a dedicated new window.
    """
    if dedicated:
        return AttachWindowTarget(kind="dedicated_new_window")
    if "window_id" in params:
        raw = params.get("window_id")
        if raw is None or raw == "":
            return AttachWindowTarget(kind="unresolved_explicit")
        return AttachWindowTarget(kind="explicit", window_id=str(raw))
    preferred = params.get("preferred_window_id")
    return AttachWindowTarget(
        kind="contextual",
        window_id=str(preferred) if preferred else None,
    )


@dataclass(frozen=True)
class MirrorRecord:
    """One mirrored Herdr session (tmux session-mirror workspace)."""

    session_id: str
    window_id: str
    workspace_id: Optional[str]


@dataclass
class ConnectionRecord:
    """Live socket attachment (tmux ``RemoteTmuxControlConnection`` snapshot)."""

    session_id: str
    started: bool = False
    snapshot_received: bool = False
    exited: bool = False
    window_ids: List[str] = field(default_factory=list)
    total_output_bytes: int = 0
    pane_output_bytes: Dict[str, int] = field(default_factory=dict)
    recent_events: List[str] = field(default_factory=list)
    client_size_applied: bool = False


def connection_action(existing: Optional[ConnectionRecord]) -> str:
    """Reuse a live connection; replace a dead one; otherwise start."""
    if existing is None:
        return "start"
    if existing.exited:
        return "replace"
    return "reuse"


def may_cache_connection(connection: ConnectionRecord) -> bool:
    """Never insert a connection that failed to start (tmux attach cache rule)."""
    return connection.started and not connection.exited


def post_attach_action(*, replaced_dead: bool) -> str:
    """Reconnect reseeds every pane; first connect applies the stored grid."""
    if replaced_dead:
        return POST_RESEED
    return POST_APPLY_CLIENT_SIZE


def host_close_policy(source: str) -> str:
    """Map a host close onto detach.

    Tmux ``markKillSessionsOnClose`` kills the remote tmux session when
    the last mirrored workspace tab closes. Herdr has no published
    session-kill that is not ``server.stop``, so every host close
    detaches and leaves the provider session running.
    """
    if source in (
        "last_workspace_tab",
        "window_quit",
        "app_terminate",
        "explicit_detach",
        "host_tab",
        "host_panel",
    ):
        return "detach"
    return "noop"


@dataclass(frozen=True)
class AttachPlan:
    """Pure attach decision (no AppKit, no socket I/O)."""

    outcome: str
    window_id: Optional[str] = None
    create_window: bool = False
    sessions_to_mirror: Tuple[str, ...] = ()
    sessions_to_reuse: Tuple[str, ...] = ()
    purge_session_ids: Tuple[str, ...] = ()
    move_workspace_ids: Tuple[str, ...] = ()
    post_attach: Optional[str] = None
    discard_window_on_fail: bool = False
    activate: bool = False
    reason: Optional[str] = None


def _live(windows: Sequence[str]) -> Callable[[str], bool]:
    live = set(windows)
    return lambda window_id: window_id in live


def plan_attach(
    target: AttachWindowTarget,
    *,
    enabled: bool,
    app_ready: bool,
    already_attaching: bool,
    existing_mirror_window_id: Optional[str],
    active_window_id: Optional[str],
    live_windows: Sequence[str],
    sessions: Optional[Sequence[DiscoveredSession]] = None,
    mirrors: Optional[Sequence[MirrorRecord]] = None,
    live_session_ids: Optional[Sequence[str]] = None,
    activate: bool = False,
    mirrored_workspace_ids: Optional[Sequence[str]] = None,
) -> AttachPlan:
    """Decide attach the way ``RemoteTmuxController.attachHost`` does.

    Pass ``sessions=None`` for the preflight (reject a guaranteed-invalid
    destination *before* discovery). After discovery, call again with the
    session list. Dedicated windows are created only after a non-empty
    discovery so a failed attach never leaves empty chrome.
    """
    if not enabled:
        return AttachPlan(outcome="disabled", reason="beta_disabled")
    if not app_ready:
        return AttachPlan(outcome="unreachable", reason="app_not_ready")
    if already_attaching:
        return AttachPlan(outcome="already_attaching", reason="reentrant")

    is_live = _live(live_windows)
    if target.kind != "dedicated_new_window":
        resolved = target.resolve(
            existing_mirror_window_id, active_window_id, is_live
        )
        if resolved is None:
            return AttachPlan(outcome="invalid_target", reason="window_unresolved")

    if sessions is None:
        return AttachPlan(outcome="discover")

    if not sessions:
        return AttachPlan(outcome="no_sessions", reason="empty_discovery")

    records = list(mirrors or ())
    dead = tuple(
        record.session_id for record in records if record.workspace_id is None
    )
    live_records = [record for record in records if record.workspace_id]
    live_ids = set(live_session_ids or ())
    discovered_ids = tuple(item.session_id for item in sessions)

    if target.kind == "dedicated_new_window":
        create_window = True
        window_id = None
        move_ids = tuple(
            record.workspace_id
            for record in live_records
            if record.workspace_id
        )
    else:
        create_window = False
        window_id = target.resolve(
            existing_mirror_window_id, active_window_id, is_live
        )
        if window_id is None:
            return AttachPlan(outcome="invalid_target", reason="window_lost")
        move_ids = ()

    reuse: List[str] = []
    create: List[str] = []
    for session_id in discovered_ids:
        if session_id in live_ids and session_id not in dead:
            reuse.append(session_id)
        else:
            create.append(session_id)

    if mirrored_workspace_ids is not None and not mirrored_workspace_ids:
        return AttachPlan(
            outcome="failed_empty",
            window_id=window_id,
            create_window=create_window,
            purge_session_ids=dead,
            discard_window_on_fail=create_window,
            reason="no_workspaces",
        )

    replaced = bool(dead) or any(session_id not in live_ids for session_id in reuse)
    if not create and reuse:
        outcome = "reused"
        post = POST_RESEED if replaced else None
    else:
        outcome = "mirrored"
        post = post_attach_action(replaced_dead=bool(dead) and not reuse)

    return AttachPlan(
        outcome=outcome,
        window_id=window_id,
        create_window=create_window,
        sessions_to_mirror=tuple(create),
        sessions_to_reuse=tuple(reuse),
        purge_session_ids=dead,
        move_workspace_ids=move_ids if create_window else (),
        post_attach=post or (POST_APPLY_CLIENT_SIZE if create else post),
        discard_window_on_fail=create_window,
        activate=activate,
    )


def plan_restore(
    record: "RestoreRecord",
    *,
    enabled: bool,
    app_ready: bool,
    sessions: Sequence[DiscoveredSession],
    live_windows: Sequence[str],
    active_window_id: Optional[str] = None,
) -> AttachPlan:
    """Reattach after a cmux restart. Never returns ``replay_tree``.

    Sidebar revalidation is not enough: a stale Bonsplit tree after
    restart is the tmux bug this copies the fix for. Fresh snapshot,
    then ``reseed``.
    """
    if not enabled:
        return AttachPlan(outcome="disabled", reason="beta_disabled")
    # Window ids do not survive a cmux restart. A live persisted id is
    # reused; otherwise dedicated stays dedicated and everything else
    # falls back to the active window — never fail closed on a stale UUID.
    if record.window_id and record.window_id in set(live_windows):
        target = AttachWindowTarget(kind="explicit", window_id=record.window_id)
    elif record.target_kind == "dedicated_new_window":
        target = AttachWindowTarget(kind="dedicated_new_window")
    else:
        target = AttachWindowTarget(kind="contextual", window_id=None)
    plan = plan_attach(
        target,
        enabled=enabled,
        app_ready=app_ready,
        already_attaching=False,
        existing_mirror_window_id=None,
        active_window_id=active_window_id,
        live_windows=live_windows,
        sessions=sessions,
        mirrors=(),
        live_session_ids=(),
        activate=True,
    )
    if plan.outcome in ("mirrored", "reused"):
        return AttachPlan(
            outcome="mirrored",
            window_id=plan.window_id,
            create_window=plan.create_window,
            sessions_to_mirror=plan.sessions_to_mirror or tuple(
                item.session_id for item in sessions
            ),
            sessions_to_reuse=(),
            purge_session_ids=(),
            move_workspace_ids=(),
            post_attach=POST_RESEED,
            discard_window_on_fail=plan.discard_window_on_fail,
            activate=True,
            reason="restore_reattach",
        )
    return plan


@dataclass(frozen=True)
class RestoreRecord:
    """Last successful attach. Host persists this; the planner only reads it."""

    endpoint_hash: str
    socket_path: str
    session_ids: Tuple[str, ...]
    target_kind: str
    window_id: Optional[str] = None

    def to_dict(self) -> Dict[str, object]:
        """JSON-safe persist payload. ``replay_tree`` is never a field."""
        return {
            "endpoint_hash": self.endpoint_hash,
            "socket_path": self.socket_path,
            "session_ids": list(self.session_ids),
            "target_kind": self.target_kind,
            "window_id": self.window_id,
            "mode": "reattach",
        }

    @classmethod
    def from_dict(cls, payload: Dict[str, object]) -> Optional["RestoreRecord"]:
        """Load a persist payload. Rejects a stale-tree replay marker."""
        if payload.get("mode") == "replay_tree":
            return None
        socket = validate_socket_path(str(payload.get("socket_path") or ""))
        endpoint = str(payload.get("endpoint_hash") or "")
        sessions = payload.get("session_ids") or []
        kind = str(payload.get("target_kind") or "")
        if not socket or not endpoint or not kind:
            return None
        if not isinstance(sessions, list) or not sessions:
            return None
        window = payload.get("window_id")
        return cls(
            endpoint_hash=endpoint,
            socket_path=socket,
            session_ids=tuple(str(item) for item in sessions),
            target_kind=kind,
            window_id=str(window) if window else None,
        )


def write_restore(path: Path, record: RestoreRecord) -> None:
    """Atomically persist the last attach (host-owned file)."""
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(record.to_dict(), indent=2) + "\n", encoding="utf-8")
    tmp.replace(path)


def read_restore(path: Path) -> Optional[RestoreRecord]:
    """Read a persist file; missing or stale-tree payloads are ignored."""
    if not path.is_file():
        return None
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    if not isinstance(payload, dict):
        return None
    return RestoreRecord.from_dict(payload)


class AttachRegistry:
    """Re-entrant attach guard (tmux ``RemoteTmuxWindowRegistry``)."""

    def __init__(self) -> None:
        self._pending: set[str] = set()

    def begin_attach(self, endpoint: str) -> bool:
        """Record an in-flight attach; ``False`` if one is already running."""
        if endpoint in self._pending:
            return False
        self._pending.add(endpoint)
        return True

    def end_attach(self, endpoint: str) -> None:
        """Clear the in-flight marker (the ``defer``)."""
        self._pending.discard(endpoint)

    def is_attaching(self, endpoint: str) -> bool:
        """Whether ``endpoint`` currently has an attach in flight."""
        return endpoint in self._pending


def existing_mirror_window(
    mirrors: Sequence[MirrorRecord],
    live_windows: Sequence[str],
) -> Optional[str]:
    """First live window that already owns a mirror for this endpoint."""
    live = set(live_windows)
    for record in mirrors:
        if record.workspace_id and record.window_id in live:
            return record.window_id
    return None


def purge_dead_mirrors(mirrors: Sequence[MirrorRecord]) -> List[MirrorRecord]:
    """Drop mirrors whose workspace died without a controller detach.

    Tmux: a stale key makes ``mirrorSessions`` skip recreation while the
    dead workspace fails the manager filter, so every retry mirrors
    nothing. Purge first.
    """
    return [record for record in mirrors if record.workspace_id]


def session_payload(session: DiscoveredSession) -> Dict[str, object]:
    """Serialize one session for ``remote.herdr.sessions``."""
    return {
        "id": session.session_id,
        "name": session.name,
        "windows": session.window_count,
        "attached": session.attached,
    }


def grid_match(
    assigned_cols: int,
    assigned_rows: int,
    rendered_cols: int,
    rendered_rows: int,
    *,
    exact_cols: bool,
    exact_rows: bool,
) -> bool:
    """Tmux ``pane_grids`` render contract: exact on the split axis, fill on the other."""
    cols_ok = (
        rendered_cols == assigned_cols if exact_cols else rendered_cols >= assigned_cols
    )
    rows_ok = (
        rendered_rows == assigned_rows if exact_rows else rendered_rows >= assigned_rows
    )
    return cols_ok and rows_ok


def pane_grid_payload(
    tab_id: str,
    panes: Sequence[Dict[str, object]],
    *,
    structure_version: int = 0,
    zoomed: bool = False,
    base_cols: int = 0,
    base_rows: int = 0,
    pushed: Optional[Tuple[int, int]] = None,
    visible_for_sizing: bool = True,
) -> Dict[str, object]:
    """One window row for ``remote.herdr.pane_grids`` (tmux sizing snapshot)."""
    rows: List[Dict[str, object]] = []
    for pane in panes:
        entry: Dict[str, object] = {
            "pane_id": pane["pane_id"],
            "assigned": {"cols": pane["assigned_cols"], "rows": pane["assigned_rows"]},
            "has_panel": bool(pane.get("has_panel", True)),
        }
        rendered_cols = pane.get("rendered_cols")
        rendered_rows = pane.get("rendered_rows")
        if rendered_cols is not None and rendered_rows is not None:
            entry["rendered"] = {"cols": rendered_cols, "rows": rendered_rows}
            entry["match"] = grid_match(
                int(pane["assigned_cols"]),
                int(pane["assigned_rows"]),
                int(rendered_cols),
                int(rendered_rows),
                exact_cols=bool(pane.get("exact_cols", False)),
                exact_rows=bool(pane.get("exact_rows", False)),
            )
        rows.append(entry)
    payload: Dict[str, object] = {
        "tab_id": tab_id,
        "structure_version": structure_version,
        "zoomed": zoomed,
        "base": {"cols": base_cols, "rows": base_rows},
        "panes": rows,
        "visible_for_sizing": visible_for_sizing,
    }
    if pushed is not None:
        payload["pushed"] = {"cols": pushed[0], "rows": pushed[1]}
    return payload


def dispatch(
    method: str,
    params: Optional[Dict[str, object]] = None,
    *,
    enabled: bool,
) -> Dict[str, object]:
    """Validate a ``remote.herdr.*`` call at the socket trust boundary.

    Does not talk to Herdr or AppKit. The host runs the matching
    controller method after this returns ``ok``.
    """
    body = params or {}
    if method not in SOCKET_METHODS:
        return {"ok": False, "code": "unknown_method"}
    if not enabled:
        return {"ok": False, "code": "disabled"}
    socket = validate_socket_path(
        str(body.get("socket") or body.get("socket_path") or "")
    )
    if socket is None:
        return {"ok": False, "code": "invalid_params"}
    session = None
    if method in _SESSION_METHODS:
        session = validate_session_name(
            None if body.get("session") is None else str(body.get("session"))
        )
        if session is None:
            return {"ok": False, "code": "invalid_params"}
    dedicated = method == "remote.herdr.window"
    target = window_target_from_params(body, dedicated=dedicated)
    activate = bool(body.get("activate", False))
    return {
        "ok": True,
        "method": method,
        "socket": socket,
        "session": session,
        "target": target,
        "activate": activate,
        "create": bool(body.get("create", False)),
    }


class LifecycleController:
    """Stateful twin of ``RemoteTmuxController`` attach/detach (no AppKit)."""

    def __init__(self, *, enabled: bool = True, app_ready: bool = True) -> None:
        self.enabled = enabled
        self.app_ready = app_ready
        self.registry = AttachRegistry()
        self.mirrors: Dict[str, MirrorRecord] = {}
        self.connections: Dict[str, ConnectionRecord] = {}
        self.live_windows: List[str] = ["win-active"]
        self.active_window_id: Optional[str] = "win-active"
        self.persist: Optional[RestoreRecord] = None
        self.events: List[Dict[str, object]] = []
        self._window_seq = 0
        self.server_stopped = False

    def _log(self, event: str, **fields: object) -> None:
        row = {"event": event, **fields}
        self.events.append(row)

    def _existing_window(self) -> Optional[str]:
        return existing_mirror_window(list(self.mirrors.values()), self.live_windows)

    def attach(
        self,
        socket_path: str,
        sessions: Sequence[DiscoveredSession],
        target: AttachWindowTarget,
        *,
        activate: bool = False,
    ) -> Dict[str, object]:
        """Attach discovered sessions into one window (tmux ``attachHost``)."""
        hashed = endpoint_hash(socket_path)
        preflight = plan_attach(
            target,
            enabled=self.enabled,
            app_ready=self.app_ready,
            already_attaching=self.registry.is_attaching(hashed),
            existing_mirror_window_id=self._existing_window(),
            active_window_id=self.active_window_id,
            live_windows=self.live_windows,
            sessions=None,
            activate=activate,
        )
        if preflight.outcome != "discover":
            self._log("attach_reject", reason=preflight.reason, endpoint_hash=hashed)
            return {"ok": False, "outcome": preflight.outcome, "reason": preflight.reason}

        if not self.registry.begin_attach(hashed):
            self._log("attach_reject", reason="reentrant", endpoint_hash=hashed)
            return {"ok": False, "outcome": "already_attaching", "reason": "reentrant"}
        try:
            self.mirrors = {
                key: value
                for key, value in self.mirrors.items()
                if value.workspace_id
            }
            live_ids = [
                session_id
                for session_id, conn in self.connections.items()
                if may_cache_connection(conn)
            ]
            plan = plan_attach(
                target,
                enabled=self.enabled,
                app_ready=self.app_ready,
                already_attaching=False,
                existing_mirror_window_id=self._existing_window(),
                active_window_id=self.active_window_id,
                live_windows=self.live_windows,
                sessions=sessions,
                mirrors=list(self.mirrors.values()),
                live_session_ids=live_ids,
                activate=activate,
            )
            if plan.outcome in ("no_sessions", "invalid_target", "failed_empty"):
                self._log("attach_reject", reason=plan.reason, endpoint_hash=hashed)
                return {"ok": False, "outcome": plan.outcome, "reason": plan.reason}

            window_id = plan.window_id
            if plan.create_window:
                self._window_seq += 1
                window_id = f"win-new-{self._window_seq}"
                self.live_windows.append(window_id)
                for session_id, record in list(self.mirrors.items()):
                    if record.workspace_id:
                        self.mirrors[session_id] = MirrorRecord(
                            session_id=session_id,
                            window_id=window_id,
                            workspace_id=record.workspace_id,
                        )

            workspace_ids: List[str] = []
            for session_id in plan.sessions_to_reuse:
                record = self.mirrors.get(session_id)
                if record and record.workspace_id:
                    workspace_ids.append(record.workspace_id)
            for session_id in plan.sessions_to_mirror:
                workspace_id = f"ws-{session_id}"
                self.mirrors[session_id] = MirrorRecord(
                    session_id=session_id,
                    window_id=window_id or "win-active",
                    workspace_id=workspace_id,
                )
                previous = self.connections.get(session_id)
                action = connection_action(previous)
                if action != "reuse":
                    self.connections[session_id] = ConnectionRecord(
                        session_id=session_id,
                        started=True,
                        snapshot_received=True,
                    )
                workspace_ids.append(workspace_id)

            if not workspace_ids:
                if plan.create_window and window_id in self.live_windows:
                    self.live_windows.remove(window_id)
                self._log("attach_reject", reason="no_workspaces", endpoint_hash=hashed)
                return {"ok": False, "outcome": "failed_empty", "reason": "no_workspaces"}

            self.persist = RestoreRecord(
                endpoint_hash=hashed,
                socket_path=socket_path,
                session_ids=tuple(item.session_id for item in sessions),
                target_kind=target.kind,
                window_id=window_id,
            )
            if activate and window_id:
                self.active_window_id = window_id
            self._log(
                "attach_ok",
                outcome=plan.outcome,
                endpoint_hash=hashed,
                session_count=len(workspace_ids),
            )
            return {
                "ok": True,
                "outcome": plan.outcome,
                "window_id": window_id,
                "workspace_ids": workspace_ids,
                "post_attach": plan.post_attach,
                "server_stopped": self.server_stopped,
            }
        finally:
            self.registry.end_attach(hashed)

    def detach(
        self,
        session_id: str,
        *,
        reason: str = TEARDOWN_EXPLICIT_DETACH,
    ) -> Dict[str, object]:
        """Detach one session mirror. Never stops the Herdr server."""
        if not self.enabled:
            return {"ok": False, "outcome": "disabled", "reason": "beta_disabled"}
        self.mirrors.pop(session_id, None)
        connection = self.connections.pop(session_id, None)
        if connection is not None:
            connection.exited = True
        self._log("detach", session_id=session_id, reason=reason)
        return {
            "ok": True,
            "outcome": "detached",
            "session": session_id,
            "reason": reason,
            "server_stopped": False,
        }

    def close_host(self, source: str) -> Dict[str, object]:
        """Host chrome close: detach every live mirror, never ``server.stop``."""
        action = host_close_policy(source)
        if action != "detach":
            return {"ok": True, "outcome": "noop", "server_stopped": False}
        detached = [self.detach(session_id) for session_id in list(self.mirrors)]
        return {
            "ok": True,
            "outcome": "detach",
            "detached": len(detached),
            "server_stopped": False,
        }

    def restore(self, sessions: Sequence[DiscoveredSession]) -> Dict[str, object]:
        """Reattach from persist after process restart."""
        if self.persist is None:
            return {"ok": False, "outcome": "no_persist"}
        plan = plan_restore(
            self.persist,
            enabled=self.enabled,
            app_ready=self.app_ready,
            sessions=sessions,
            live_windows=self.live_windows,
            active_window_id=self.active_window_id,
        )
        if plan.outcome not in ("mirrored", "reused"):
            self._log("restore", outcome=plan.outcome, reason=plan.reason)
            return {"ok": False, "outcome": plan.outcome, "reason": plan.reason}
        if self.persist.window_id and self.persist.window_id in self.live_windows:
            target = AttachWindowTarget(
                kind="explicit", window_id=self.persist.window_id
            )
        elif self.persist.target_kind == "dedicated_new_window":
            target = AttachWindowTarget(kind="dedicated_new_window")
        else:
            target = AttachWindowTarget(kind="contextual")
        result = self.attach(
            self.persist.socket_path,
            sessions,
            target,
            activate=True,
        )
        if result.get("ok"):
            result["post_attach"] = POST_RESEED
            result["mode"] = "reattach"
        self._log("restore", outcome=result.get("outcome"))
        return result

    def state(self, session_id: str) -> Dict[str, object]:
        """``remote.herdr.state`` snapshot (tmux control-connection diagnostics)."""
        connection = self.connections.get(session_id)
        if connection is None or connection.exited:
            return {"session": session_id, "attached": False}
        return {
            "session": session_id,
            "attached": True,
            "started": connection.started,
            "snapshot_received": connection.snapshot_received,
            "exited": connection.exited,
            "window_count": len(connection.window_ids),
            "window_ids": list(connection.window_ids),
            "total_output_bytes": connection.total_output_bytes,
            "pane_output_bytes": dict(connection.pane_output_bytes),
            "recent_events": list(connection.recent_events),
        }


def note_output(connection: ConnectionRecord, pane_id: str, byte_count: int) -> None:
    """Accumulate ``%output``-style byte counts for ``remote.herdr.state``."""
    connection.total_output_bytes += byte_count
    connection.pane_output_bytes[pane_id] = (
        connection.pane_output_bytes.get(pane_id, 0) + byte_count
    )
    connection.recent_events.append(f"output pane={pane_id} bytes={byte_count}")
    if len(connection.recent_events) > 32:
        del connection.recent_events[:-32]


__all__ = [
    "AttachPlan",
    "AttachRegistry",
    "AttachWindowTarget",
    "ConnectionRecord",
    "DiscoveredSession",
    "LifecycleController",
    "MirrorRecord",
    "POST_APPLY_CLIENT_SIZE",
    "POST_RESEED",
    "RestoreRecord",
    "SETTING_KEY",
    "SOCKET_METHODS",
    "TEARDOWN_EXPLICIT_DETACH",
    "TEARDOWN_SESSION_ENDED",
    "connection_action",
    "decode_beta",
    "dispatch",
    "endpoint_hash",
    "existing_mirror_window",
    "grid_match",
    "has_hidden_character",
    "host_close_policy",
    "may_cache_connection",
    "note_output",
    "pane_grid_payload",
    "plan_attach",
    "plan_restore",
    "post_attach_action",
    "purge_dead_mirrors",
    "read_restore",
    "session_payload",
    "validate_session_name",
    "validate_socket_path",
    "window_target_from_params",
    "write_restore",
]
