#!/usr/bin/env python3
"""Drive ``LiveApplyHost`` the way native SessionHost drives Ghostty.

Gold standard: cmux ``RemoteTmuxSessionMirror`` / native Herdr SessionHost.

    topology event  → session.snapshot → apply_session
    pane.updated    → pane.read → route_read_snapshot (isolated)
    focus event     → provider focus (no first-responder steal)
    agent_status    → note_agent_status + cwd (busy-close / tab activity)

The plugin still cannot steal a Ghostty PTY. This pump feeds the in-memory
apply host so ``watch --tmux-parity`` is not a mirror-only loop. Projection
(``mirror_to_cmux``) stays in the CLI; this module never starts a competing
writer when native owns the lease.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Callable, Dict, List, Optional, Sequence

try:
    from .cmux_herdr_api import HerdrApi, extract_agent_status, extract_read_text
except ImportError:  # pragma: no cover
    from cmux_herdr_api import HerdrApi, extract_agent_status, extract_read_text


KIND_TOPOLOGY = "topology"
KIND_OUTPUT = "output"
KIND_FOCUS = "focus"
KIND_STATUS = "status"
KIND_METADATA = "metadata"
KIND_OTHER = "other"

TOPOLOGY_EVENTS = frozenset(
    {
        "workspace.created",
        "workspace.updated",
        "workspace.renamed",
        "workspace.moved",
        "workspace.reordered",
        "workspace.closed",
        "tab.created",
        "tab.closed",
        "tab.renamed",
        "tab.moved",
        "pane.created",
        "pane.closed",
        "pane.moved",
        "pane.exited",
        "pane.resized",
        "layout.updated",
        "layout.changed",
    }
)
OUTPUT_EVENTS = frozenset({"pane.updated", "pane.output_matched"})
FOCUS_EVENTS = frozenset({"pane.focused", "tab.focused", "workspace.focused"})
STATUS_EVENTS = frozenset({"pane.agent_status_changed", "pane.agent_detected"})
METADATA_EVENTS = frozenset({"workspace.metadata_updated"})

WindowsBuilder = Callable[[], Sequence[Any]]


@dataclass
class PumpResult:
    """One pump step. The CLI uses flags to decide whether to remirror."""

    kind: str
    resync: bool = False
    routed_output: bool = False
    focused: bool = False
    status_updated: bool = False
    pane_id: Optional[str] = None
    log: str = ""

    def to_dict(self) -> Dict[str, Any]:
        """JSON-ready debug payload."""
        return {
            "kind": self.kind,
            "resync": self.resync,
            "routed_output": self.routed_output,
            "focused": self.focused,
            "status_updated": self.status_updated,
            "pane_id": self.pane_id,
            "log": self.log,
        }


def unwrap_event(obj: Optional[Dict[str, Any]]) -> Dict[str, Any]:
    """Normalize a protocol-17 event envelope to a flat body dict."""
    if not obj:
        return {}
    data = obj.get("data")
    if isinstance(data, dict):
        body = dict(data)
        event_name = obj.get("event")
        if "type" not in body and isinstance(event_name, str):
            body["type"] = event_name
        return body
    nested = obj.get("event")
    if isinstance(nested, dict):
        return dict(nested)
    params = obj.get("params")
    if isinstance(params, dict) and (
        params.get("type") or params.get("pane_id") or params.get("event")
    ):
        return dict(params)
    return dict(obj)


def event_type(obj: Optional[Dict[str, Any]]) -> str:
    """Return the dotted event name (``pane.updated``), or empty."""
    body = unwrap_event(obj)
    for key in ("type", "event", "name"):
        value = body.get(key)
        if isinstance(value, str) and value:
            return value
    if obj:
        value = obj.get("event")
        if isinstance(value, str) and value:
            return value
    return ""


def event_string(body: Dict[str, Any], *names: str) -> str:
    """First non-empty string field, walking ``pane`` / ``agent`` nests."""
    for name in names:
        value = body.get(name)
        if isinstance(value, str) and value.strip():
            return value.strip()
    for nest_key in ("pane", "agent", "tab", "workspace"):
        nested = body.get(nest_key)
        if isinstance(nested, dict):
            found = event_string(nested, *names)
            if found:
                return found
    return ""


def classify_event(obj: Optional[Dict[str, Any]]) -> str:
    """Bucket a Herdr event the way native SessionHost does."""
    name = event_type(obj)
    if name in TOPOLOGY_EVENTS:
        return KIND_TOPOLOGY
    if name in OUTPUT_EVENTS:
        return KIND_OUTPUT
    if name in FOCUS_EVENTS:
        return KIND_FOCUS
    if name in STATUS_EVENTS:
        return KIND_STATUS
    if name in METADATA_EVENTS:
        return KIND_METADATA
    return KIND_OTHER


class PumpTransport:
    """How the pump reads pane bytes / metadata. Tests inject a fake."""

    def read_pane(self, pane_id: str) -> str:
        """Return the current ``pane.read`` snapshot for ``pane_id``."""
        raise NotImplementedError

    def pane_info(self, pane_id: str) -> Dict[str, Any]:
        """Return ``pane.get`` (cwd, agent_status). Empty dict on failure."""
        raise NotImplementedError


class MemoryTransport(PumpTransport):
    """In-memory pane reads for tests."""

    def __init__(
        self,
        reads: Optional[Dict[str, str]] = None,
        panes: Optional[Dict[str, Dict[str, Any]]] = None,
    ) -> None:
        """Store pane snapshots keyed by pane id."""
        self.reads = dict(reads or {})
        self.panes = dict(panes or {})
        self.read_calls: List[str] = []

    def read_pane(self, pane_id: str) -> str:
        """Return the canned snapshot and record the call."""
        self.read_calls.append(pane_id)
        return self.reads.get(pane_id, "")

    def pane_info(self, pane_id: str) -> Dict[str, Any]:
        """Return canned ``pane.get`` data."""
        return dict(self.panes.get(pane_id) or {})


class ApiTransport(PumpTransport):
    """Live transport over ``HerdrApi`` (socket first)."""

    def __init__(self, api: Optional[HerdrApi] = None) -> None:
        """Use ``api`` or a default socket-first caller."""
        self.api = api or HerdrApi()

    def read_pane(self, pane_id: str) -> str:
        """``pane.read`` → text. Empty string when the pane is gone."""
        try:
            result = self.api.call(
                "pane.read",
                {"pane_id": pane_id, "source": "recent", "lines": 200},
            )
        except Exception:
            return ""
        return extract_read_text(result.result)

    def pane_info(self, pane_id: str) -> Dict[str, Any]:
        """``pane.get`` → dict. Empty when the pane is gone."""
        try:
            result = self.api.call("pane.get", {"pane_id": pane_id})
        except Exception:
            return {}
        payload = result.result
        return payload if isinstance(payload, dict) else {}


@dataclass
class LivePump:
    """SessionHost-style event pump for ``LiveApplyHost``."""

    transport: PumpTransport
    windows_builder: Optional[WindowsBuilder] = None
    log: List[str] = field(default_factory=list)

    def handle_event(self, event: Optional[Dict[str, Any]], host: Any) -> PumpResult:
        """Apply one Herdr event onto the live host.

        Unknown panes are no-ops (tmux ``routeOutput``). Topology events
        resync when a ``windows_builder`` is set; otherwise they only
        flag ``resync`` so the caller can apply.
        """
        if host is None:
            return PumpResult(kind=KIND_OTHER, log="no_host")
        kind = classify_event(event)
        body = unwrap_event(event)
        pane_id = event_string(body, "pane_id")
        if kind == KIND_TOPOLOGY:
            if self.windows_builder is not None:
                return self.resync(host, log=f"event:{event_type(event)}")
            self.log.append(f"topology:{event_type(event)}")
            return PumpResult(
                kind=kind, resync=True, pane_id=pane_id or None, log="topology"
            )
        if kind == KIND_OUTPUT:
            return self._route_output(host, pane_id)
        if kind == KIND_FOCUS:
            return self._route_focus(host, pane_id, body)
        if kind == KIND_STATUS:
            return self._route_status(host, pane_id, body)
        if kind == KIND_METADATA:
            self.log.append("metadata")
            return PumpResult(kind=kind, log="metadata")
        return PumpResult(kind=kind, pane_id=pane_id or None, log="ignored")

    def poll(self, host: Any) -> PumpResult:
        """Timeout tick: ``pane.read`` every live surface (no %output stream)."""
        if host is None:
            return PumpResult(kind=KIND_OUTPUT, log="no_host")
        pane_ids = _live_pane_ids(host)
        routed = 0
        for pane_id in pane_ids:
            if self._paint(host, pane_id):
                routed += 1
        self.log.append(f"poll:{routed}/{len(pane_ids)}")
        return PumpResult(
            kind=KIND_OUTPUT,
            routed_output=routed > 0,
            log=f"poll:{routed}",
        )

    def resync(self, host: Any, *, log: str = "resync") -> PumpResult:
        """Full snapshot apply, then poll outputs into new surfaces."""
        if host is None:
            return PumpResult(kind=KIND_TOPOLOGY, resync=True, log="no_host")
        if self.windows_builder is None:
            self.log.append("resync:no_builder")
            return PumpResult(kind=KIND_TOPOLOGY, resync=True, log="no_builder")
        windows = list(self.windows_builder())
        host.apply_session(windows)
        painted = self.poll(host)
        self.log.append(log)
        return PumpResult(
            kind=KIND_TOPOLOGY,
            resync=True,
            routed_output=painted.routed_output,
            log=log,
        )

    def _route_output(self, host: Any, pane_id: str) -> PumpResult:
        """pane.updated → isolated read snapshot."""
        if not pane_id:
            return PumpResult(kind=KIND_OUTPUT, log="missing_pane")
        routed = self._paint(host, pane_id)
        return PumpResult(
            kind=KIND_OUTPUT,
            routed_output=routed,
            pane_id=pane_id,
            log="output" if routed else "output_noop",
        )

    def _route_focus(self, host: Any, pane_id: str, body: Dict[str, Any]) -> PumpResult:
        """Provider focus: project locally, never echo pane.focus."""
        target = pane_id or event_string(body, "focused_pane_id", "active_pane_id")
        if not target:
            return PumpResult(kind=KIND_FOCUS, resync=True, log="focus_resync")
        applied = False
        apply_fn = getattr(host, "apply_provider_focus", None)
        if callable(apply_fn):
            applied = bool(apply_fn(target))
        cwd = event_string(body, "foreground_cwd", "cwd")
        if cwd and hasattr(host, "route_cwd"):
            host.route_cwd(target, cwd)
        self.log.append(f"focus:{target}")
        return PumpResult(
            kind=KIND_FOCUS,
            focused=applied,
            pane_id=target,
            log="focus",
        )

    def _route_status(self, host: Any, pane_id: str, body: Dict[str, Any]) -> PumpResult:
        """agent_status → tab activity / busy-close; cwd when present."""
        if not pane_id:
            return PumpResult(kind=KIND_STATUS, log="missing_pane")
        status = event_string(body, "agent_status", "status", "state") or "unknown"
        name = event_string(body, "agent", "display_agent", "label")
        note = getattr(host, "note_agent_status", None)
        if callable(note):
            note(pane_id, status, name or None)
        info = self.transport.pane_info(pane_id)
        if not status or status == "unknown":
            extracted = extract_agent_status(info)
            if extracted:
                status = extracted
                if callable(note):
                    note(pane_id, status, name or None)
        cwd = event_string(info, "foreground_cwd", "cwd") if info else ""
        cwd = cwd or event_string(body, "foreground_cwd", "cwd")
        if cwd and hasattr(host, "route_cwd"):
            host.route_cwd(pane_id, cwd)
        self.log.append(f"status:{pane_id}:{status}")
        return PumpResult(
            kind=KIND_STATUS,
            status_updated=True,
            pane_id=pane_id,
            log=f"status:{status}",
        )

    def _paint(self, host: Any, pane_id: str) -> bool:
        """Read one pane and route the incremental delta. Isolated by pane id."""
        text = self.transport.read_pane(pane_id)
        if not text:
            return False
        route = getattr(host, "route_read_snapshot", None)
        if not callable(route):
            return False
        return bool(route(pane_id, text))


def _live_pane_ids(host: Any) -> List[str]:
    """Pane ids with a live in-memory surface."""
    getter = getattr(host, "live_pane_ids", None)
    if callable(getter):
        return list(getter())
    ids: List[str] = []
    windows = getattr(host, "windows", None) or {}
    for mirror in windows.values():
        surfaces = getattr(mirror, "surfaces", None) or {}
        torn = bool(getattr(mirror, "is_torn_down", False))
        for pane_id, surface in surfaces.items():
            if torn:
                continue
            if getattr(surface, "live", True):
                ids.append(pane_id)
    return ids
