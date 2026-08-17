#!/usr/bin/env python3
"""Cmux-tmux user mutations mapped onto Herdr methods.

Gold standard: ``RemoteTmuxWindowMirror+ControlMutations``,
``RemoteTmuxPaneInputForwarder``, ``routeSeed``, ``navigateFocus``,
``RemoteTmuxMirrorTabActivity``, and host-close detach.

This is the same *depth* cmux built for tmux, limited to methods Herdr
actually has (``pane.send`` / ``send-keys``, ``pane.split``, ``pane.focus``,
``pane.close``, ``pane.read``, ``agent_status``). SSH, ``tmux -CC``, and
``respawn-pane`` are not copied — Herdr does not have them.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Sequence, Tuple

_TITLE_ESCAPES = re.compile(r"\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b.")

try:
    from .cmux_herdr_layout import LayoutNode
except ImportError:
    from cmux_herdr_layout import LayoutNode


# Tmux ``RemoteTmuxKeyName`` bases. CSI is the pane.send fallback when
# pane.send_keys is unavailable.
_NAMED_CSI: Dict[str, bytes] = {
    "Up": b"\x1b[A",
    "Down": b"\x1b[B",
    "Right": b"\x1b[C",
    "Left": b"\x1b[D",
    "Home": b"\x1b[H",
    "End": b"\x1b[F",
    "PPage": b"\x1b[5~",
    "NPage": b"\x1b[6~",
    "DC": b"\x1b[3~",
    "IC": b"\x1b[2~",
    "F1": b"\x1bOP",
    "F2": b"\x1bOQ",
    "F3": b"\x1bOR",
    "F4": b"\x1bOS",
    "F5": b"\x1b[15~",
    "F6": b"\x1b[17~",
    "F7": b"\x1b[18~",
    "F8": b"\x1b[19~",
    "F9": b"\x1b[20~",
    "F10": b"\x1b[21~",
    "F11": b"\x1b[23~",
    "F12": b"\x1b[24~",
}

_BUSY_STATUSES = frozenset({"working", "blocked", "running", "command"})
DEFAULT_INPUT_BUDGET = 256 * 1024
DEFAULT_SEED_BUDGET = 256 * 1024


@dataclass(frozen=True)
class ProviderInput:
    """One input event for a single Herdr pane (tmux ``TerminalManualInput``)."""

    pane_id: str
    kind: str  # "text" | "key"
    text: Optional[str] = None
    key: Optional[str] = None
    csi: Optional[bytes] = None

    @property
    def byte_count(self) -> int:
        """Bytes reserved against the input budget."""
        if self.kind == "key":
            return len(self.csi or b"") or 1
        return len((self.text or "").encode("utf-8")) or 1


def encode_named_key(pane_id: str, raw_name: str) -> Optional[ProviderInput]:
    """Resolve a tmux-style key name (``C-Up``, ``F5``) for Herdr.

    Prefers ``pane.send_keys`` via ``key``. ``csi`` is the ``pane.send``
    fallback so arrows still work on builds that only accept text.

    Args:
        pane_id: Bound Herdr pane.
        raw_name: Tmux ``RemoteTmuxKeyName`` string.

    Returns:
        Input, or None when the name is unknown (never invent a key).
    """
    if not raw_name or not pane_id:
        return None
    parts = [part for part in raw_name.split("-") if part]
    if not parts:
        return None
    base = parts[-1]
    mods = {part for part in parts[:-1] if part in {"C", "M", "S"}}
    csi = _NAMED_CSI.get(base)
    if csi is None:
        return None
    if mods:
        csi = _csi_with_modifiers(base, csi, mods)
    return ProviderInput(
        pane_id=pane_id, kind="key", key="-".join(list(sorted(mods)) + [base]), csi=csi
    )


def _csi_with_modifiers(base: str, csi: bytes, mods: set) -> bytes:
    """Xterm modifier parameter: 1 + shift + 2*alt + 4*ctrl."""
    code = 1
    if "S" in mods:
        code += 1
    if "M" in mods:
        code += 2
    if "C" in mods:
        code += 4
    if code == 1:
        return csi
    if csi.endswith(b"~"):
        core = csi[2:-1]
        return b"\x1b[" + core + f";{code}~".encode("ascii")
    if csi.startswith(b"\x1bO") and len(csi) == 3:
        letter = {ord("P"): b"11", ord("Q"): b"12", ord("R"): b"13", ord("S"): b"14"}.get(
            csi[2]
        )
        if letter:
            return b"\x1b[" + letter + f";{code}~".encode("ascii")
    if csi.startswith(b"\x1b[") and len(csi) == 3:
        return b"\x1b[1;" + str(code).encode("ascii") + csi[-1:]
    return csi


def encode_manual_input(pane_id: str, text: Optional[str] = None, key: Optional[str] = None) -> Optional[ProviderInput]:
    """Build one Ghostty-style manual input. Key wins over text."""
    if key:
        return encode_named_key(pane_id, key)
    if text:
        return ProviderInput(pane_id=pane_id, kind="text", text=text)
    return None


@dataclass
class InputForwarder:
    """Bounded Ghostty→Herdr input queue (tmux ``RemoteTmuxPaneInputForwarder``)."""

    maximum_pending_bytes: int = DEFAULT_INPUT_BUDGET
    pending_bytes: int = 0
    epoch: int = 0
    active: bool = True
    queue: List[ProviderInput] = field(default_factory=list)
    overflowed: bool = False

    def enqueue(self, item: ProviderInput) -> str:
        """Reserve bytes and queue. Returns ``enqueued``, ``inactive``, or ``overflow``."""
        if not self.active:
            return "inactive"
        size = item.byte_count
        if self.pending_bytes + size > self.maximum_pending_bytes:
            self.overflowed = True
            return "overflow"
        self.pending_bytes += size
        self.queue.append(item)
        return "enqueued"

    def drain(self) -> List[ProviderInput]:
        """Pop queued inputs and release their reservation."""
        items = list(self.queue)
        self.queue.clear()
        self.pending_bytes = 0
        return items

    def deactivate(self) -> None:
        """Detach: drop queued keys so they cannot hit a reused pane id."""
        self.active = False
        self.epoch += 1
        self.queue.clear()
        self.pending_bytes = 0


@dataclass
class PendingFocus:
    """In-flight user focus awaiting provider confirmation."""

    request_id: str
    pane_id: str
    previous_pane_id: Optional[str]


@dataclass
class FocusCommand:
    """Result of a user or provider focus mutation."""

    pane_id: Optional[str]
    send_to_provider: bool
    rolled_back: bool = False
    request_id: Optional[str] = None


@dataclass
class FocusController:
    """Optimistic user focus with rollback (tmux ``requestControlFocus``)."""

    live_pane_ids: List[str] = field(default_factory=list)
    active_pane_id: Optional[str] = None
    pending: Optional[PendingFocus] = None
    _next_id: int = 0

    def user_select(self, pane_id: str) -> FocusCommand:
        """Project immediately and send ``pane.focus``. Unknown panes are a no-op."""
        if pane_id not in self.live_pane_ids:
            return FocusCommand(pane_id=None, send_to_provider=False)
        if self.pending and self.pending.pane_id == pane_id:
            return FocusCommand(
                pane_id=pane_id,
                send_to_provider=False,
                request_id=self.pending.request_id,
            )
        self._next_id += 1
        request_id = f"f{self._next_id}"
        self.pending = PendingFocus(
            request_id=request_id,
            pane_id=pane_id,
            previous_pane_id=self.active_pane_id,
        )
        self.active_pane_id = pane_id
        return FocusCommand(pane_id=pane_id, send_to_provider=True, request_id=request_id)

    def command_rejected(self, request_id: str) -> FocusCommand:
        """Roll back when Herdr rejects ``pane.focus``."""
        pending = self.pending
        if pending is None or pending.request_id != request_id:
            return FocusCommand(pane_id=self.active_pane_id, send_to_provider=False)
        self.pending = None
        self.active_pane_id = pending.previous_pane_id
        return FocusCommand(
            pane_id=pending.previous_pane_id,
            send_to_provider=False,
            rolled_back=True,
            request_id=request_id,
        )

    def provider_confirms(self, pane_id: str) -> FocusCommand:
        """Authoritative provider focus. Clears a matching pending request."""
        if self.pending and self.pending.pane_id == pane_id:
            self.pending = None
        self.active_pane_id = pane_id
        return FocusCommand(pane_id=pane_id, send_to_provider=False)


@dataclass(frozen=True)
class UserSplit:
    """User-initiated split from cmux chrome (tmux ``requestSplit``)."""

    from_pane_id: str
    orientation: str  # "horizontal" | "vertical"
    insert_first: bool = False
    focus_created: bool = True


def request_split(
    from_pane_id: str,
    *,
    vertical: bool,
    insert_first: bool = False,
    focus_created: bool = True,
) -> Optional[UserSplit]:
    """Build a ``pane.split`` request. Empty pane id is a no-op."""
    if not from_pane_id:
        return None
    return UserSplit(
        from_pane_id=from_pane_id,
        orientation="vertical" if vertical else "horizontal",
        insert_first=insert_first,
        focus_created=focus_created,
    )


def adjacent_pane(node: LayoutNode, pane_id: str, direction: str) -> Optional[str]:
    """Neighbor leaf in the layout tree (tmux ``navigateFocus``).

    ``direction`` is ``left`` / ``right`` / ``up`` / ``down``. Provider focus
    must not use this path — it would steal the first responder.

    Args:
        node: Visible (or base) layout.
        pane_id: Currently focused leaf.
        direction: Compass direction.

    Returns:
        Neighbor pane id, or None at an edge / unknown pane.
    """
    path = _path_to_pane(node, pane_id)
    if not path:
        return None
    want_horizontal = direction in ("left", "right")
    for index in range(len(path) - 1, 0, -1):
        parent = path[index - 1]
        child = path[index]
        axis_ok = (parent.kind == "horizontal") == want_horizontal
        if not axis_ok or parent.kind == "pane":
            continue
        try:
            child_index = parent.children.index(child)
        except ValueError:
            continue
        if direction in ("left", "up") and child_index > 0:
            return _edge_pane(parent.children[child_index - 1], direction)
        if direction in ("right", "down") and child_index + 1 < len(parent.children):
            return _edge_pane(parent.children[child_index + 1], direction)
    return None


def _path_to_pane(node: LayoutNode, pane_id: str) -> Optional[List[LayoutNode]]:
    if node.kind == "pane":
        return [node] if node.pane_id == pane_id else None
    for child in node.children:
        found = _path_to_pane(child, pane_id)
        if found:
            return [node] + found
    return None


def _edge_pane(node: LayoutNode, approaching: str) -> Optional[str]:
    if node.kind == "pane":
        return node.pane_id
    if not node.children:
        return None
    # Entering a subtree: land on the near edge. Only walk to the far
    # child when the split axis matches the approach.
    if approaching == "left" and node.kind == "horizontal":
        return _edge_pane(node.children[-1], approaching)
    if approaching == "up" and node.kind == "vertical":
        return _edge_pane(node.children[-1], approaching)
    return _edge_pane(node.children[0], approaching)


@dataclass
class PaneSeedQueue:
    """Hold ``pane.read`` seed bytes until the surface grid is ready.

    Twin of tmux ``routeSeed`` / pending-byte ceiling. Overflow marks a
    deferred full reseed instead of painting a truncated snapshot.
    """

    maximum_bytes: int = DEFAULT_SEED_BUDGET
    pending: Dict[str, bytes] = field(default_factory=dict)
    kinds: Dict[str, str] = field(default_factory=dict)
    targets: Dict[str, Tuple[int, int]] = field(default_factory=dict)
    deferred_full: set = field(default_factory=set)

    def queue(
        self,
        pane_id: str,
        data: bytes,
        *,
        kind: str = "full",
        target_grid: Optional[Tuple[int, int]] = None,
    ) -> str:
        """Queue seed bytes. Returns ``queued``, ``overflow``, or ``empty``."""
        if not data:
            return "empty"
        if len(data) > self.maximum_bytes:
            self.deferred_full.add(pane_id)
            self.pending.pop(pane_id, None)
            return "overflow"
        self.pending[pane_id] = data
        self.kinds[pane_id] = kind
        if target_grid:
            self.targets[pane_id] = target_grid
        return "queued"

    def note_ready(self, pane_id: str, cols: int, rows: int) -> Optional[bytes]:
        """Flush when the surface matches the target grid (or no target)."""
        target = self.targets.get(pane_id)
        if target is not None and target != (cols, rows):
            return None
        data = self.pending.pop(pane_id, None)
        self.kinds.pop(pane_id, None)
        self.targets.pop(pane_id, None)
        if data is not None:
            self.deferred_full.discard(pane_id)
        return data


@dataclass(frozen=True)
class TabActivity:
    """Tab chrome from Herdr ``agent_status`` (tmux ``MirrorTabActivity``)."""

    has_active_command: bool
    active_command_name: Optional[str]
    needs_close_confirmation: bool


def tab_activity(
    statuses: Dict[str, str],
    agents: Optional[Dict[str, str]] = None,
) -> TabActivity:
    """Project pane statuses onto tab activity.

    Herdr has agent status tmux lacks. ``working`` / ``blocked`` are the
    busy set — same role as tmux ``pane_current_command`` for close confirm
    and unread chrome.

    Args:
        statuses: pane id → ``agent_status``.
        agents: optional pane id → agent display name.

    Returns:
        Activity for the tab that owns these panes.
    """
    names = agents or {}
    busy_panes = [
        pane_id
        for pane_id, status in statuses.items()
        if (status or "").lower() in _BUSY_STATUSES
    ]
    command = None
    for pane_id in busy_panes:
        label = names.get(pane_id)
        if label:
            command = label
            break
    return TabActivity(
        has_active_command=bool(busy_panes),
        active_command_name=command,
        needs_close_confirmation=bool(busy_panes),
    )


@dataclass(frozen=True)
class CloseIntent:
    """What a close gesture may do. Host close never kills Herdr."""

    action: str  # "detach" | "close_pane" | "confirm_then_close_pane" | "noop"
    pane_id: Optional[str] = None


def close_intent(
    source: str,
    *,
    pane_id: Optional[str] = None,
    agent_status: Optional[str] = None,
) -> CloseIntent:
    """Map a close gesture the way tmux does — detach vs kill-pane.

    ``host_tab`` / ``host_panel`` / ``detach`` → detach only (never
    ``pane.close`` or ``server.stop``). ``user_pane`` → ``pane.close``,
    with confirmation when the pane is busy.
    """
    if source in ("host_tab", "host_panel", "detach"):
        return CloseIntent(action="detach")
    if source != "user_pane" or not pane_id:
        return CloseIntent(action="noop")
    if (agent_status or "").lower() in _BUSY_STATUSES:
        return CloseIntent(action="confirm_then_close_pane", pane_id=pane_id)
    return CloseIntent(action="close_pane", pane_id=pane_id)


def apply_session_title(
    name: str,
    *,
    current: Optional[str] = None,
    propagate_to_provider: bool = False,
) -> Optional[str]:
    """Re-title the workspace from a Herdr session/tab rename.

    Tmux ``applySessionNameToWorkspaceTitle``: provider is source of truth,
    control characters are stripped, and ``propagate_to_provider`` must stay
    false on the inbound path or rename echoes forever.
    """
    if propagate_to_provider:
        return None
    cleaned = _TITLE_ESCAPES.sub("", name)
    cleaned = "".join(ch for ch in cleaned if ch.isprintable())
    cleaned = cleaned.strip()
    if not cleaned or cleaned == current:
        return None
    return cleaned[:200]


def pane_surface_entries(
    bindings: Sequence[Tuple[str, str, str, bool]],
) -> List[Dict[str, object]]:
    """Observability rows (tmux ``remote.tmux.pane_surfaces``).

    Each tuple is ``(tab_id, pane_id, surface_id, on_screen)``. Hidden tabs
    keep their last render; oracles must skip ``on_screen=False``.
    """
    rows = [
        {
            "tab_id": tab_id,
            "pane_id": pane_id,
            "surface_id": surface_id,
            "on_screen": on_screen,
        }
        for tab_id, pane_id, surface_id, on_screen in bindings
    ]
    rows.sort(key=lambda row: (str(row["tab_id"]), str(row["pane_id"])))
    return rows
