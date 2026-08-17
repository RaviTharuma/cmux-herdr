#!/usr/bin/env python3
"""Pane I/O routing + focus projection (tmux ``routeOutput`` / ``setActivePane``).

Gold standard: ``RemoteTmuxWindowMirror.routeOutput``, ``connectionSendKeys``,
and ``setActivePane(fromTmux:)``. A byte written for pane A must never appear
on pane B. Typed input reaches only the bound pane. Provider focus never
echoes ``pane.focus`` back (that loop is how a second client fights the first).

AppKit/Ghostty stay native. This module is the userspace contract plus an
in-memory router that proves isolation without a PTY.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Dict, List, Optional, Set, Tuple

try:
    from .cmux_herdr_engine import output_delta
except ImportError:
    from cmux_herdr_engine import output_delta


# Screen/tmux window-title: ESC k <title> ST (ESC \). Same bytes as
# RemoteTmuxScreenTitleFilter — a chunk may split the sequence anywhere.
_ESC = 0x1B
_ST_SLASH = 0x5C  # backslash
_K = ord("k")

_STATE_TEXT = 0
_STATE_ESC = 1
_STATE_TITLE = 2
_STATE_TITLE_ESC = 3


class TitleEscapeFilter:
    """Stateful strip of ``ESC k … ESC \\`` across chunk boundaries.

    Tmux ``%output`` forwards the raw PTY copy, including screen title
    sequences a shell emits under ``TERM=screen*``. Ghostty is xterm-style
    and would paint the title onto the grid. The ssh-tmux mirror strips
    them; Herdr must do the same.
    """

    def __init__(self) -> None:
        """Start in the text state (nothing held)."""
        self._state = _STATE_TEXT

    def reset(self) -> None:
        """Drop held escape state (full-redraw / new snapshot)."""
        self._state = _STATE_TEXT

    def filter(self, data: bytes) -> bytes:
        """Return ``data`` with title sequences removed.

        Args:
            data: One output chunk (may be empty).

        Returns:
            Bytes safe to write to an xterm-style surface.
        """
        if not data:
            return b""
        if self._state == _STATE_TEXT and _ESC not in data:
            return data
        out = bytearray()
        state = self._state
        for byte in data:
            if state == _STATE_TEXT:
                if byte == _ESC:
                    state = _STATE_ESC
                else:
                    out.append(byte)
            elif state == _STATE_ESC:
                if byte == _K:
                    state = _STATE_TITLE
                elif byte == _ESC:
                    out.append(_ESC)
                else:
                    out.append(_ESC)
                    out.append(byte)
                    state = _STATE_TEXT
            elif state == _STATE_TITLE:
                if byte == _ESC:
                    state = _STATE_TITLE_ESC
            elif state == _STATE_TITLE_ESC:
                if byte == _ST_SLASH:
                    state = _STATE_TEXT
                elif byte != _ESC:
                    state = _STATE_TITLE
        self._state = state
        return bytes(out)

    def filter_text(self, text: str) -> str:
        """UTF-8 helper for ``pane.read`` snapshots."""
        return self.filter(text.encode("utf-8", errors="surrogateescape")).decode(
            "utf-8", errors="surrogateescape"
        )


@dataclass(frozen=True)
class SurfaceWrite:
    """One isolated write to a single pane surface."""

    pane_id: str
    surface_id: str
    data: bytes
    full_redraw: bool = False


@dataclass(frozen=True)
class InputSend:
    """Typed bytes destined for exactly one Herdr pane."""

    pane_id: str
    data: bytes


@dataclass(frozen=True)
class FocusProjection:
    """Result of projecting focus (provider event or user click)."""

    pane_id: Optional[str]
    send_to_provider: bool
    changed: bool
    source: str


@dataclass(frozen=True)
class CwdUpdate:
    """Tab working-directory update (only the active pane may emit this)."""

    pane_id: str
    tab_id: str
    path: str
    apply_to_tab: bool


@dataclass
class PaneIORouter:
    """In-memory I/O router proving tmux isolation without Ghostty.

    Surfaces are bound after ``create_panel``. Unknown panes are a no-op —
    never a write to “whatever is focused.” That is the cross-pane invariant.
    """

    surfaces: Dict[str, str] = field(default_factory=dict)
    buffers: Dict[str, bytes] = field(default_factory=dict)
    last_snapshot: Dict[str, str] = field(default_factory=dict)
    title_filters: Dict[str, TitleEscapeFilter] = field(default_factory=dict)
    live_pane_ids: Set[str] = field(default_factory=set)
    cwd_by_pane: Dict[str, str] = field(default_factory=dict)
    active_pane_id: Optional[str] = None
    pending_user_focus: Optional[str] = None
    log: List[str] = field(default_factory=list)

    def bind(self, pane_id: str, surface_id: str) -> None:
        """Record a host surface for ``pane_id`` (tmux ``panelsByPaneId``)."""
        self.surfaces[pane_id] = surface_id
        self.buffers.setdefault(surface_id, b"")
        self.live_pane_ids.add(pane_id)
        self.title_filters.setdefault(pane_id, TitleEscapeFilter())
        self.log.append(f"bind:{pane_id}:{surface_id}")

    def unbind(self, pane_id: str) -> None:
        """Drop the surface binding when the BASE pane is gone."""
        self.surfaces.pop(pane_id, None)
        self.last_snapshot.pop(pane_id, None)
        self.title_filters.pop(pane_id, None)
        self.cwd_by_pane.pop(pane_id, None)
        self.live_pane_ids.discard(pane_id)
        if self.active_pane_id == pane_id:
            self.active_pane_id = None
        if self.pending_user_focus == pane_id:
            self.pending_user_focus = None
        self.log.append(f"unbind:{pane_id}")

    def set_live_panes(self, pane_ids: List[str]) -> None:
        """Replace the BASE pane set (zoom-hidden panes stay live)."""
        self.live_pane_ids = set(pane_ids)

    def route_output(self, pane_id: str, data: bytes) -> Optional[SurfaceWrite]:
        """Route a ``%output``-style chunk to exactly one surface.

        Unknown panes are a no-op (tmux ``routeOutput``). Empty data is a
        no-op. Title escapes are stripped with a per-pane stateful filter.

        Args:
            pane_id: Herdr pane that produced the bytes.
            data: Raw chunk.

        Returns:
            The write that was applied, or None.
        """
        surface_id = self.surfaces.get(pane_id)
        if surface_id is None or not data:
            return None
        filt = self.title_filters.setdefault(pane_id, TitleEscapeFilter())
        cleaned = filt.filter(data)
        if not cleaned:
            return None
        self.buffers[surface_id] = self.buffers.get(surface_id, b"") + cleaned
        write = SurfaceWrite(
            pane_id=pane_id, surface_id=surface_id, data=cleaned, full_redraw=False
        )
        self.log.append(f"out:{pane_id}:{len(cleaned)}")
        return write

    def route_output_text(self, pane_id: str, current: str) -> Optional[SurfaceWrite]:
        """Incremental ``pane.read`` paint (plugin stand-in for ``%output``).

        Uses ``output_delta``. A full redraw resets the title filter and
        replaces the surface buffer instead of appending.

        Args:
            pane_id: Herdr pane.
            current: Latest snapshot text.

        Returns:
            The write that was applied, or None when unchanged / unbound.
        """
        surface_id = self.surfaces.get(pane_id)
        if surface_id is None:
            return None
        previous = self.last_snapshot.get(pane_id)
        chunk, full_redraw = output_delta(previous, current)
        self.last_snapshot[pane_id] = current
        if not chunk and not full_redraw:
            return None
        filt = self.title_filters.setdefault(pane_id, TitleEscapeFilter())
        if full_redraw:
            filt.reset()
            cleaned = filt.filter_text(chunk)
            encoded = cleaned.encode("utf-8", errors="surrogateescape")
            self.buffers[surface_id] = encoded
        else:
            cleaned = filt.filter_text(chunk)
            encoded = cleaned.encode("utf-8", errors="surrogateescape")
            if not encoded:
                return None
            self.buffers[surface_id] = self.buffers.get(surface_id, b"") + encoded
        write = SurfaceWrite(
            pane_id=pane_id,
            surface_id=surface_id,
            data=encoded,
            full_redraw=full_redraw,
        )
        self.log.append(f"out-text:{pane_id}:redraw={int(full_redraw)}:{len(encoded)}")
        return write

    def route_input(self, pane_id: str, data: bytes) -> Optional[InputSend]:
        """Forward typed bytes to the bound pane only (tmux ``sendKeys``).

        Args:
            pane_id: Target Herdr pane (never “whatever is focused”).
            data: Key bytes.

        Returns:
            The send, or None when unbound / empty.
        """
        if not data or pane_id not in self.surfaces:
            return None
        send = InputSend(pane_id=pane_id, data=data)
        self.log.append(f"in:{pane_id}:{len(data)}")
        return send

    def route_input_to_focus(self, data: bytes) -> Optional[InputSend]:
        """Send to the active pane only. No-op when focus is unknown."""
        if self.active_pane_id is None:
            return None
        return self.route_input(self.active_pane_id, data)

    def note_remote_active(self, pane_id: str) -> FocusProjection:
        """Provider focus (tmux ``noteRemoteActivePane`` / ``fromTmux: true``).

        Always projects locally. Never sends ``pane.focus`` (echo-loop gate).
        Unknown panes are tolerated — the matching layout may still be pending.

        Args:
            pane_id: Provider-reported active pane.

        Returns:
            Projection with ``send_to_provider=False``.
        """
        changed = self.active_pane_id != pane_id
        self.active_pane_id = pane_id
        if self.pending_user_focus == pane_id:
            self.pending_user_focus = None
        self.log.append(f"focus-provider:{pane_id}")
        return FocusProjection(
            pane_id=pane_id,
            send_to_provider=False,
            changed=changed,
            source="provider",
        )

    def user_focus(self, pane_id: str) -> FocusProjection:
        """User click (tmux ``setActivePane(fromTmux: false)``).

        Requires a live BASE pane. Sends ``pane.focus`` unless this pane is
        already the in-flight user request (provider echo not yet back).

        Args:
            pane_id: Pane the user selected.

        Returns:
            Projection. ``send_to_provider`` is the echo-loop gate.
        """
        if pane_id not in self.live_pane_ids and pane_id not in self.surfaces:
            self.log.append(f"focus-user-unknown:{pane_id}")
            return FocusProjection(
                pane_id=None,
                send_to_provider=False,
                changed=False,
                source="user",
            )
        changed = self.active_pane_id != pane_id
        self.active_pane_id = pane_id
        if self.pending_user_focus == pane_id:
            self.log.append(f"focus-user-pending:{pane_id}")
            return FocusProjection(
                pane_id=pane_id,
                send_to_provider=False,
                changed=changed,
                source="user",
            )
        self.pending_user_focus = pane_id
        self.log.append(f"focus-user-send:{pane_id}")
        return FocusProjection(
            pane_id=pane_id,
            send_to_provider=True,
            changed=changed,
            source="user",
        )

    def project_focus(self, pane_id: str, *, from_provider: bool) -> FocusProjection:
        """Dispatch focus by source. Provider never echoes; user may send."""
        if from_provider:
            return self.note_remote_active(pane_id)
        return self.user_focus(pane_id)

    def route_cwd(self, pane_id: str, path: str, tab_id: str) -> Optional[CwdUpdate]:
        """Cache cwd; apply to the tab only when this pane is active.

        Tmux: a background pane's ``cd`` must not hijack the tab folder.

        Args:
            pane_id: Reporting pane.
            path: Working directory.
            tab_id: Window/tab that owns the pane.

        Returns:
            Update, or None when the path is empty.
        """
        trimmed = path.strip()
        if not trimmed:
            return None
        self.cwd_by_pane[pane_id] = trimmed
        apply = self.active_pane_id == pane_id
        update = CwdUpdate(
            pane_id=pane_id, tab_id=tab_id, path=trimmed, apply_to_tab=apply
        )
        self.log.append(f"cwd:{pane_id}:tab={int(apply)}")
        return update

    def buffer_for(self, pane_id: str) -> bytes:
        """Return bytes written to ``pane_id``'s surface (empty if unbound)."""
        surface_id = self.surfaces.get(pane_id)
        if surface_id is None:
            return b""
        return self.buffers.get(surface_id, b"")


def route_output(
    router: PaneIORouter, pane_id: str, data: bytes
) -> Optional[SurfaceWrite]:
    """Module-level twin of ``RemoteTmuxWindowMirror.routeOutput``."""
    return router.route_output(pane_id, data)


def route_input(
    router: PaneIORouter, pane_id: str, data: bytes
) -> Optional[InputSend]:
    """Module-level twin of tmux ``sendKeys``."""
    return router.route_input(pane_id, data)


def project_focus(
    router: PaneIORouter, pane_id: str, *, from_provider: bool
) -> FocusProjection:
    """Module-level twin of tmux ``setActivePane(fromTmux:)``."""
    return router.project_focus(pane_id, from_provider=from_provider)
