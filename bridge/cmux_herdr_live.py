#!/usr/bin/env python3
"""Live ssh-tmux apply machine for Herdr.

This is the depth that was still missing: not another planner, the
*running* host that tmux already has in ``RemoteTmuxWindowMirror``.

One ``LiveApplyHost`` owns:

- ``make_panel`` → in-memory Ghostty surface (native swaps in
  ``TerminalPanel`` + ``processRemoteOutput``)
- ``%output`` isolation + title-escape strip
- typed input + named keys to the bound pane only
- Bonsplit impose (create panels *before* rebuild)
- divider-drag begin / hold / end → ``pane.resize`` cells
- first-responder rules (``is_applying_focus``: provider must not steal)
- feed-forward ``update_client_size`` (never reads pane frames)
- zoom keeps hidden panels
- attach / detach / restore (never ``server.stop``)
- tab activity / busy-close from ``agent_status``
- ``remote.herdr.*`` observability
- shared plugin ↔ native writer lease (one live apply host)

Plugin ceiling: surfaces are byte buffers, not Ghostty PTYs. The
*sequence* is the same one AppKit must run. Native
``RemoteHerdrLiveApply`` is the Swift twin. The two paths share
``cmux_herdr_handoff`` so they do not double-project.
"""

from __future__ import annotations

import os
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Tuple

try:
    from .cmux_herdr_control import (
        FocusController,
        InputForwarder,
        PaneSeedQueue,
        adjacent_pane,
        apply_session_title,
        close_intent,
        encode_named_key,
        pane_surface_entries,
        request_split,
        tab_activity,
    )
    from .cmux_herdr_engine import (
        HerdrWindow,
        WindowMirrorState,
        apply_window,
        client_grid,
        impose_after_apply,
        output_delta,
        reconcile_session,
    )
    from .cmux_herdr_host import FakeBonsplitHost, HostAction, host_actions
    from .cmux_herdr_impose import (
        begin_divider_drag,
        end_divider_drag,
        resolve_divider_hold,
    )
    from .cmux_herdr_io import PaneIORouter, TitleEscapeFilter
    from .cmux_herdr_lifecycle import (
        POST_APPLY_CLIENT_SIZE,
        POST_RESEED,
        SETTING_KEY,
        SOCKET_METHODS,
        AttachWindowTarget,
        DiscoveredSession,
        LifecycleController,
        RestoreRecord,
        decode_beta,
        dispatch,
        endpoint_hash,
        grid_match,
        pane_grid_payload,
        read_restore,
        write_restore,
    )
    from .cmux_herdr_session import FakeSessionHost, session_actions
except ImportError:
    from cmux_herdr_control import (
        FocusController,
        InputForwarder,
        PaneSeedQueue,
        adjacent_pane,
        apply_session_title,
        close_intent,
        encode_named_key,
        pane_surface_entries,
        request_split,
        tab_activity,
    )
    from cmux_herdr_engine import (
        HerdrWindow,
        WindowMirrorState,
        apply_window,
        client_grid,
        impose_after_apply,
        output_delta,
        reconcile_session,
    )
    from cmux_herdr_host import FakeBonsplitHost, HostAction, host_actions
    from cmux_herdr_impose import (
        begin_divider_drag,
        end_divider_drag,
        resolve_divider_hold,
    )
    from cmux_herdr_io import PaneIORouter, TitleEscapeFilter
    from cmux_herdr_lifecycle import (
        POST_APPLY_CLIENT_SIZE,
        POST_RESEED,
        SETTING_KEY,
        SOCKET_METHODS,
        AttachWindowTarget,
        DiscoveredSession,
        LifecycleController,
        RestoreRecord,
        decode_beta,
        dispatch,
        endpoint_hash,
        grid_match,
        pane_grid_payload,
        read_restore,
        write_restore,
    )
    from cmux_herdr_session import FakeSessionHost, session_actions


@dataclass
class GhosttySurface:
    """In-memory Ghostty analogue of ``TerminalSurface`` (manual-mirror I/O).

    Native replaces this with a real panel whose
    ``surface.processRemoteOutput`` receives the same bytes.
    """

    pane_id: str
    surface_id: str
    buffer: bytes = b""
    cols: int = 80
    rows: int = 24
    first_responder: bool = False
    live: bool = True
    in_window: bool = True

    def process_remote_output(self, data: bytes) -> None:
        """Append cleaned bytes (tmux ``surface.processRemoteOutput``)."""
        if not data or not self.live:
            return
        self.buffer += data

    def resize_grid(self, cols: int, rows: int) -> None:
        """Record the feed-forward grid this surface was claimed at."""
        if cols >= 1 and rows >= 1:
            self.cols = cols
            self.rows = rows


@dataclass
class LiveWindowMirror:
    """One Herdr tab: Bonsplit host + Ghostty surfaces (tmux window mirror)."""

    tab_id: str
    title: str
    bonsplit: FakeBonsplitHost = field(default_factory=FakeBonsplitHost)
    io: PaneIORouter = field(default_factory=PaneIORouter)
    focus: FocusController = field(default_factory=FocusController)
    seed: PaneSeedQueue = field(default_factory=PaneSeedQueue)
    input: InputForwarder = field(default_factory=InputForwarder)
    surfaces: Dict[str, GhosttySurface] = field(default_factory=dict)
    state: Optional[WindowMirrorState] = None
    is_applying_focus: bool = False
    is_applying_layout: bool = False
    is_torn_down: bool = False
    is_visible_for_sizing: bool = True
    container_width: float = 800.0
    container_height: float = 400.0
    cell_width: float = 8.0
    cell_height: float = 16.0
    last_client_grid: Optional[Tuple[int, int]] = None
    drag_hold: object = None
    structure_version: int = 0
    tab_cwd: Optional[str] = None

    def make_panel(self, pane_id: str) -> GhosttySurface:
        """Create the Ghostty surface *before* the Bonsplit rebuild."""
        if pane_id in self.surfaces and self.surfaces[pane_id].live:
            return self.surfaces[pane_id]
        surface_id = f"surf-{self.tab_id}-{pane_id}"
        surface = GhosttySurface(pane_id=pane_id, surface_id=surface_id)
        self.surfaces[pane_id] = surface
        self.io.bind(pane_id, surface_id)
        return surface

    def close_panel(self, pane_id: str) -> None:
        """Tear down a BASE pane (zoom must not call this)."""
        surface = self.surfaces.get(pane_id)
        if surface is not None:
            surface.live = False
        self.surfaces.pop(pane_id, None)
        self.io.unbind(pane_id)

    def apply_window(self, window: HerdrWindow) -> List[str]:
        """Reconcile + impose + focus. Tmux ``RemoteTmuxWindowMirror.apply``."""
        if self.is_torn_down:
            return []
        previous = self.state
        previous_rendered = previous.layout if previous else None
        self.state, result = apply_window(window, previous)
        self.title = window.title
        self.structure_version = self.state.layout_structure_version
        plan = impose_after_apply(
            result, previous_rendered=previous_rendered, title=window.title
        )
        actions = host_actions(result, plan)
        log: List[str] = []
        self.is_applying_layout = True
        try:
            for action in actions:
                log.append(self._apply_host_action(action))
        finally:
            self.is_applying_layout = False
        self.io.set_live_panes(list(self.state.pane_ids))
        self.focus.live_pane_ids = list(self.state.pane_ids)
        if result.focus_pane_id:
            self._apply_provider_focus(result.focus_pane_id)
        self.title = apply_session_title(window.title, current=self.title) or window.title
        self._apply_cached_cwd()
        return [item for item in log if item]

    def _apply_host_action(self, action: HostAction) -> str:
        if action.op == "create_panel" and action.pane_id:
            self.make_panel(action.pane_id)
            self.bonsplit.apply([action])
            return f"make_panel:{action.pane_id}"
        if action.op == "close_panel" and action.pane_id:
            self.close_panel(action.pane_id)
            self.bonsplit.apply([action])
            return f"close_panel:{action.pane_id}"
        if action.op == "focus" and action.pane_id:
            return ""
        self.bonsplit.apply([action])
        return action.op

    def route_output(self, pane_id: str, data: bytes) -> bool:
        """``%output`` → exactly one surface. Unknown pane is a no-op."""
        write = self.io.route_output(pane_id, data)
        if write is None:
            return False
        surface = self.surfaces.get(pane_id)
        if surface is None or not surface.live:
            return False
        surface.process_remote_output(write.data)
        return True

    def route_read_snapshot(self, pane_id: str, text: str) -> bool:
        """Poll ``pane.read`` and paint the incremental delta."""
        chunk, _full = output_delta(self.io.last_snapshot.get(pane_id), text)
        self.io.last_snapshot[pane_id] = text
        if not chunk:
            return False
        return self.route_output(pane_id, chunk.encode("utf-8", errors="surrogateescape"))

    def send_text(self, pane_id: str, text: str) -> str:
        """Typed input to the bound pane only."""
        try:
            from .cmux_herdr_control import ProviderInput
        except ImportError:
            from cmux_herdr_control import ProviderInput

        send = self.io.route_input(pane_id, text.encode("utf-8"))
        if send is None:
            return "inactive"
        return self.input.enqueue(ProviderInput(pane_id=pane_id, kind="text", text=text))

    def send_named_key(self, pane_id: str, name: str) -> str:
        """Ghostty named key → ``pane.send_keys`` + CSI fallback."""
        item = encode_named_key(pane_id, name)
        if item is None:
            return "unknown"
        if pane_id not in self.surfaces:
            return "inactive"
        return self.input.enqueue(item)

    def _apply_provider_focus(self, pane_id: str) -> None:
        """Provider focus: project locally, never echo, do not steal keyboard."""
        self.is_applying_focus = True
        try:
            self.focus.provider_confirms(pane_id)
            self.io.note_remote_active(pane_id)
            # Do not touch first_responder. Tmux ``isApplyingTmuxFocus``
            # projects the strip dot without stealing the keyboard.
            self._apply_cached_cwd()
        finally:
            self.is_applying_focus = False

    def route_cwd(self, pane_id: str, path: str) -> Optional[object]:
        """Cache cwd; apply to the tab only when this pane is active.

        Tmux ``updateRemotePanelDirectory``: a background ``cd`` must not
        hijack the tab folder.
        """
        update = self.io.route_cwd(pane_id, path, self.tab_id)
        if update is not None and update.apply_to_tab:
            self.tab_cwd = update.path
        return update

    def _apply_cached_cwd(self) -> None:
        """Promote the active pane's cached cwd onto the tab folder."""
        active = self.io.active_pane_id or self.focus.active_pane_id
        if not active:
            return
        path = self.io.cwd_by_pane.get(active)
        if path:
            self.tab_cwd = path

    def user_focus(self, pane_id: str) -> Optional[str]:
        """User click: optimistic select, send ``pane.focus`` once."""
        if pane_id not in self.surfaces:
            return None
        command = self.focus.user_select(pane_id)
        surface = self.surfaces[pane_id]
        if not self.is_applying_focus:
            for other in self.surfaces.values():
                other.first_responder = False
            surface.first_responder = True
        self.io.user_focus(pane_id)
        self._apply_cached_cwd()
        return command.pane_id if command.send_to_provider or command.pane_id else None

    def navigate_focus(self, direction: str) -> Optional[str]:
        """Adjacent-pane focus (tmux ``navigateFocus``)."""
        if self.state is None or self.focus.active_pane_id is None:
            return None
        neighbor = adjacent_pane(self.state.layout, self.focus.active_pane_id, direction)
        if neighbor:
            return self.user_focus(neighbor)
        return None

    def user_split(self, pane_id: str, direction: str) -> Optional[object]:
        """User chrome split → ``pane.split``."""
        if pane_id not in self.surfaces:
            return None
        vertical = direction in ("down", "vertical", "below")
        return request_split(pane_id, vertical=vertical)

    def update_client_size(self) -> Optional[Tuple[int, int]]:
        """Feed-forward claim. Never reads a measured pane frame."""
        if not self.is_visible_for_sizing or self.is_torn_down:
            return None
        grid = client_grid(
            self.container_width,
            self.container_height,
            self.cell_width,
            self.cell_height,
        )
        if grid is None or grid == self.last_client_grid:
            return grid
        self.last_client_grid = grid
        for surface in self.surfaces.values():
            if surface.live:
                surface.resize_grid(grid[0], grid[1])
        return grid

    def begin_drag(self, split_key: str, axis: str, assigned_cells: int) -> None:
        """Divider-drag begin (tmux ``splitTabBarDividerDragDidBegin``)."""
        self.drag_hold = begin_divider_drag(split_key, axis, assigned_cells)

    def end_drag(
        self,
        *,
        dragged_extent: float,
        axis_span: float,
        total_cells: int,
        assigned_cells: int,
    ) -> Tuple[int, bool]:
        """Divider-drag end → cells + whether to send ``pane.resize``."""
        cells, should_send = end_divider_drag(
            dragged_extent=dragged_extent,
            axis_span=axis_span,
            total_cells=total_cells,
            assigned_cells=assigned_cells,
        )
        if should_send and self.drag_hold is not None:
            self.drag_hold = resolve_divider_hold(
                self.drag_hold,
                assigned_cells=None if not should_send else assigned_cells,
                split_still_exists=True,
            )
        if should_send:
            self.drag_hold = begin_divider_drag(
                getattr(self.drag_hold, "split_key", "s") if self.drag_hold else "s",
                getattr(self.drag_hold, "axis", "horizontal") if self.drag_hold else "horizontal",
                cells,
            )
        else:
            self.drag_hold = None
        return cells, should_send

    def note_resize_reply(self, assigned_cells: int, split_exists: bool = True) -> None:
        """Clear the drag hold when Herdr publishes the sent span."""
        self.drag_hold = resolve_divider_hold(
            self.drag_hold,
            assigned_cells=assigned_cells,
            split_still_exists=split_exists,
        )

    def seed_pane(self, pane_id: str, data: bytes, cols: int, rows: int) -> Optional[bytes]:
        """Hold scrollback until the Ghostty grid matches (tmux pane seed)."""
        self.seed.queue(pane_id, data, kind="full", target_grid=(cols, rows))
        surface = self.surfaces.get(pane_id)
        current = (surface.cols, surface.rows) if surface else (0, 0)
        flushed = self.seed.note_ready(pane_id, current[0], current[1])
        if flushed:
            self.route_output(pane_id, flushed)
        return flushed

    def teardown(self) -> None:
        """Detach this window mirror. Surfaces go inert; Herdr stays up."""
        self.is_torn_down = True
        self.input.deactivate()
        for surface in self.surfaces.values():
            surface.live = False
            surface.first_responder = False

    def pane_grids(self) -> Dict[str, object]:
        """``remote.herdr.pane_grids`` row for this tab."""
        panes = []
        layout = self.state.layout if self.state else None
        for pane_id, surface in sorted(self.surfaces.items()):
            assigned = (surface.cols, surface.rows)
            if layout is not None:
                for node in _walk_leaves(layout):
                    if node[0] == pane_id:
                        assigned = (max(1, node[1]), max(1, node[2]))
                        break
            panes.append(
                {
                    "pane_id": pane_id,
                    "assigned_cols": assigned[0],
                    "assigned_rows": assigned[1],
                    "rendered_cols": surface.cols,
                    "rendered_rows": surface.rows,
                    "exact_cols": True,
                    "exact_rows": True,
                    "has_panel": surface.live,
                }
            )
        return pane_grid_payload(
            self.tab_id,
            panes,
            structure_version=self.structure_version,
            zoomed=bool(self.state.zoomed) if self.state else False,
            base_cols=layout.rect.width if layout else 0,
            base_rows=layout.rect.height if layout else 0,
            pushed=self.last_client_grid,
            visible_for_sizing=self.is_visible_for_sizing,
        )


def _walk_leaves(node) -> List[Tuple[str, int, int]]:
    if node.kind == "pane" and node.pane_id:
        return [(node.pane_id, node.rect.width, node.rect.height)]
    rows: List[Tuple[str, int, int]] = []
    for child in node.children:
        rows.extend(_walk_leaves(child))
    return rows


class LiveApplyHost:
    """Session-level live machine (tmux ``RemoteTmuxSessionMirror`` + controller)."""

    def __init__(
        self,
        *,
        enabled: bool = True,
        socket_path: str = "/tmp/herdr.sock",
        claim_native_writer: bool = False,
    ) -> None:
        self.enabled = enabled
        self.socket_path = socket_path
        self.claim_native_writer = claim_native_writer
        self.windows: Dict[str, LiveWindowMirror] = {}
        self.session_host = FakeSessionHost()
        self.lifecycle = LifecycleController(enabled=enabled)
        self.previous_tab_ids: List[str] = []
        self.previous_titles: Dict[str, str] = {}
        self.defaults_open = True
        self.agent_statuses: Dict[str, str] = {}
        self.agent_names: Dict[str, str] = {}
        self.focused_workspace_id: Optional[str] = None
        self.native_live = False
        self.server_stopped = False
        self.log: List[str] = []

    def attach(self, sessions: Sequence[DiscoveredSession], *, activate: bool = True) -> Dict[str, object]:
        """Attach discovered Herdr sessions (tmux ``attachHost``)."""
        if not self.enabled:
            return {"ok": False, "outcome": "disabled"}
        result = self.lifecycle.attach(
            self.socket_path,
            sessions,
            AttachWindowTarget(kind="contextual"),
            activate=activate,
        )
        if result.get("ok") and result.get("post_attach") == POST_APPLY_CLIENT_SIZE:
            for mirror in self.windows.values():
                mirror.update_client_size()
        if result.get("ok") and result.get("post_attach") == POST_RESEED:
            for mirror in self.windows.values():
                for pane_id, surface in mirror.surfaces.items():
                    mirror.seed_pane(pane_id, surface.buffer, surface.cols, surface.rows)
        self.log.append(f"attach:{result.get('outcome')}")
        return result

    def apply_session(self, windows: Sequence[HerdrWindow]) -> Dict[str, object]:
        """Create/close tabs, then apply each window mirror."""
        if not self.enabled:
            return {"ok": False, "outcome": "disabled"}
        session = reconcile_session(windows, self.previous_tab_ids)
        titles = {window.tab_id: window.title for window in windows}
        actions = session_actions(
            session,
            titles=titles,
            previous_titles=self.previous_titles,
            defaults_open=self.defaults_open,
        )
        self.session_host.apply(actions)
        if any(item.op == "close_default_tabs" for item in actions):
            self.defaults_open = False
        for tab_id in session.closed_tab_ids:
            mirror = self.windows.pop(tab_id, None)
            if mirror is not None:
                mirror.teardown()
        applied: List[str] = []
        for window in windows:
            mirror = self.windows.get(window.tab_id)
            if mirror is None:
                mirror = LiveWindowMirror(tab_id=window.tab_id, title=window.title)
                self.windows[window.tab_id] = mirror
            applied.extend(mirror.apply_window(window))
            mirror.update_client_size()
        self.previous_tab_ids = list(session.ordered_tab_ids)
        self.previous_titles = titles
        self.log.append(f"session:tabs={len(self.windows)}")
        return {
            "ok": True,
            "tabs": list(self.windows),
            "session_ops": [item.op for item in actions],
            "window_ops": applied,
            "defaults_open": self.defaults_open,
        }

    def route_output(self, pane_id: str, data: bytes) -> bool:
        """Route bytes to the window that owns ``pane_id``."""
        for mirror in self.windows.values():
            if pane_id in mirror.surfaces:
                return mirror.route_output(pane_id, data)
        return False

    def route_cwd(self, pane_id: str, path: str) -> Optional[object]:
        """Active-pane cwd → tab folder. Background ``cd`` is ignored."""
        for mirror in self.windows.values():
            if pane_id in mirror.surfaces:
                return mirror.route_cwd(pane_id, path)
        return None

    def route_read_snapshot(self, pane_id: str, text: str) -> bool:
        """Poll ``pane.read`` into the window that owns ``pane_id``."""
        for mirror in self.windows.values():
            if pane_id in mirror.surfaces:
                return mirror.route_read_snapshot(pane_id, text)
        return False

    def paint_read(self, pane_id: str, text: str) -> bool:
        """First snapshot seeds (tmux pane seed); later ticks apply a delta."""
        for mirror in self.windows.values():
            if pane_id not in mirror.surfaces:
                continue
            if pane_id in mirror.io.last_snapshot:
                return mirror.route_read_snapshot(pane_id, text)
            surface = mirror.surfaces[pane_id]
            data = text.encode("utf-8", errors="surrogateescape")
            flushed = mirror.seed_pane(pane_id, data, surface.cols, surface.rows)
            if flushed:
                mirror.io.last_snapshot[pane_id] = text
                return True
            return False
        return False

    def apply_provider_focus(self, pane_id: str) -> bool:
        """Project provider focus without stealing first responder."""
        for mirror in self.windows.values():
            if pane_id in mirror.surfaces:
                mirror._apply_provider_focus(pane_id)
                return True
        return False

    def apply_tab_focus(self, tab_id: str) -> bool:
        """Project inner tab focus onto the session host (no pane echo)."""
        if tab_id not in self.windows:
            return False
        try:
            from .cmux_herdr_session import SessionAction
        except ImportError:
            from cmux_herdr_session import SessionAction

        self.session_host.apply([SessionAction(op="focus_tab", tab_id=tab_id)])
        return True

    def apply_workspace_focus(self, workspace_id: str) -> bool:
        """Record provider workspace focus without a full session resync.

        The plugin apply host is one Herdr workspace. A matching id is a
        no-op success (tmux session already selected). Unknown ids return
        False so the pump can resync.
        """
        if not workspace_id:
            return False
        self.focused_workspace_id = workspace_id
        self.log.append(f"workspace_focus:{workspace_id}")
        return True

    def drain_input(self) -> List:
        """Pop queued Ghostty→Herdr input from every live window."""
        items: List = []
        for mirror in self.windows.values():
            if mirror.is_torn_down:
                continue
            items.extend(mirror.input.drain())
        return items

    def note_agent_status(
        self, pane_id: str, status: str, name: Optional[str] = None
    ) -> None:
        """Record ``agent_status`` for tab activity / busy-close."""
        if pane_id and status:
            self.agent_statuses[pane_id] = status
        if pane_id and name:
            self.agent_names[pane_id] = name

    def live_pane_ids(self) -> List[str]:
        """Pane ids that currently have a live in-memory surface."""
        ids: List[str] = []
        for mirror in self.windows.values():
            if mirror.is_torn_down:
                continue
            for pane_id, surface in mirror.surfaces.items():
                if surface.live:
                    ids.append(pane_id)
        return ids

    def detach(self) -> Dict[str, object]:
        """Host close: teardown every mirror, never stop Herdr."""
        for mirror in self.windows.values():
            mirror.teardown()
        self.windows.clear()
        closed = self.lifecycle.close_host("host_tab")
        self.native_live = False
        self.log.append("detach")
        return {
            "ok": True,
            "outcome": "detach",
            "server_stopped": False,
            "lifecycle": closed,
        }

    def restore(self, sessions: Sequence[DiscoveredSession], windows: Sequence[HerdrWindow]) -> Dict[str, object]:
        """Reattach after restart: fresh apply + reseed, never replay_tree."""
        restored = self.lifecycle.restore(sessions)
        applied = self.apply_session(windows)
        for mirror in self.windows.values():
            for pane_id, surface in mirror.surfaces.items():
                mirror.seed_pane(pane_id, surface.buffer, surface.cols, surface.rows)
        return {
            "ok": bool(restored.get("ok") and applied.get("ok")),
            "mode": "reattach",
            "post_attach": POST_RESEED,
            "restore": restored,
            "apply": applied,
        }

    def set_native_live(self, marker_writer=None) -> None:
        """Native AppKit claims the single-writer lock."""
        if not self.claim_native_writer:
            return
        if marker_writer is not None:
            marker_writer()
        else:
            _handoff().claim_native_writer(
                _fingerprint_key(),
                socket_path=self.socket_path,
                endpoint_hash=endpoint_hash(self.socket_path) if self.socket_path else "",
            )
        self.native_live = True
        self.log.append("native_live")

    def close_user_pane(self, pane_id: str) -> object:
        """User pane ✕ — confirm when ``agent_status`` is busy."""
        status = self.agent_statuses.get(pane_id)
        return close_intent("user_pane", pane_id=pane_id, agent_status=status)

    def activity(self) -> object:
        """Tab chrome from Herdr ``agent_status``."""
        return tab_activity(self.agent_statuses, self.agent_names)

    def state(self, session_id: str) -> Dict[str, object]:
        """``remote.herdr.state``."""
        payload = self.lifecycle.state(session_id)
        payload["window_count"] = len(self.windows)
        payload["window_ids"] = list(self.windows)
        payload["total_output_bytes"] = sum(
            len(surface.buffer)
            for mirror in self.windows.values()
            for surface in mirror.surfaces.values()
        )
        return payload

    def pane_surfaces(self) -> List[Dict[str, object]]:
        """``remote.herdr.pane_surfaces``."""
        rows = []
        for tab_id, mirror in self.windows.items():
            for pane_id, surface in mirror.surfaces.items():
                rows.append((tab_id, pane_id, surface.surface_id, surface.live and not mirror.is_torn_down))
        return pane_surface_entries(rows)

    def pane_grids(self) -> List[Dict[str, object]]:
        """``remote.herdr.pane_grids``."""
        return [mirror.pane_grids() for mirror in self.windows.values()]

    def observe(self, method: str, params: Optional[Dict[str, object]] = None) -> Dict[str, object]:
        """Validate + serve one ``remote.herdr.*`` call."""
        gate = dispatch(method, params or {"socket": self.socket_path, "session": "main"}, enabled=self.enabled)
        if not gate.get("ok"):
            return gate
        if method == "remote.herdr.pane_surfaces":
            gate["panes"] = self.pane_surfaces()
            gate["mirrored"] = bool(self.windows)
        elif method == "remote.herdr.pane_grids":
            gate["windows"] = self.pane_grids()
            gate["mirrored"] = bool(self.windows)
        elif method == "remote.herdr.state":
            gate.update(self.state(str(gate.get("session") or "main")))
        elif method == "remote.herdr.detach":
            gate.update(self.detach())
        return gate


def apply_live_windows(
    windows: Sequence[HerdrWindow],
    *,
    host: Optional[LiveApplyHost] = None,
    enabled: bool = True,
) -> LiveApplyHost:
    """Run one live apply pass. Used by ``mirror --tmux-parity``."""
    machine = host or LiveApplyHost(enabled=enabled)
    machine.apply_session(windows)
    return machine


def _fingerprint_key() -> str:
    """Host fingerprint the lease files are keyed by."""
    try:
        from .cmux_herdr_bridge import _parent_key
    except ImportError:
        from cmux_herdr_bridge import _parent_key
    return _parent_key()


def _handoff():
    """Shared plugin ↔ native lease helpers."""
    try:
        from . import cmux_herdr_handoff as handoff
    except ImportError:
        import cmux_herdr_handoff as handoff
    return handoff


def restore_record_path(socket_path: str) -> Path:
    """Canonical persist file (XDG). Copies also land in the native state dir."""
    return _handoff().restore_paths(endpoint_hash(socket_path))[0]


def resolve_socket_path(explicit: Optional[str] = None) -> Optional[str]:
    """Prefer ``explicit``, then ``HERDR_SOCKET_PATH``, then the default socket."""
    try:
        from .cmux_herdr_bridge import default_herdr_socket_path
        from .cmux_herdr_lifecycle import validate_socket_path
    except ImportError:
        from cmux_herdr_bridge import default_herdr_socket_path
        from cmux_herdr_lifecycle import validate_socket_path

    for raw in (explicit, os.environ.get("HERDR_SOCKET_PATH"), default_herdr_socket_path()):
        validated = validate_socket_path(raw)
        if validated:
            return validated
    return None


def sessions_from_snapshot(snap) -> List[DiscoveredSession]:
    """Map a Herdr snapshot onto attach-session rows."""
    rows: List[DiscoveredSession] = []
    for workspace in getattr(snap, "workspaces", None) or []:
        session_id = getattr(workspace, "workspace_id", "") or "main"
        rows.append(
            DiscoveredSession(
                session_id=session_id,
                name=getattr(workspace, "label", None) or session_id,
                window_count=int(getattr(workspace, "tab_count", 0) or 0),
            )
        )
    if rows:
        return rows
    tabs = getattr(snap, "tabs", None) or []
    return [DiscoveredSession("main", "main", window_count=len(tabs))]


def persist_host_restore(host: LiveApplyHost) -> Optional[str]:
    """Write the last attach so either path can reattach after restart."""
    record = host.lifecycle.persist
    if record is None:
        return None
    return _handoff().write_shared_restore(record.endpoint_hash, record.to_dict())


def clear_host_restore(socket_path: str) -> bool:
    """Drop persist files after an explicit detach."""
    return _handoff().clear_shared_restore(endpoint_hash(socket_path))


def _foreign_payload(action: str, method: Optional[str] = None) -> Optional[Dict[str, object]]:
    """Return a yield blob when the other path already owns the host."""
    handoff = _handoff()
    decision = handoff.resolve_writer(_fingerprint_key())
    if action == "observe" and (
        decision.yields
        or (
            decision.plugin_live
            and decision.lease is not None
            and decision.lease.pid not in (0, os.getpid())
        )
    ):
        return handoff.observe_foreign(decision, method or "remote.herdr.state")
    if decision.yields:
        body = decision.payload(action=action, method=method)
        if action == "restore":
            body["mode"] = "reattach"
        return body
    return None


def attach_live(
    windows: Sequence[HerdrWindow],
    sessions: Sequence[DiscoveredSession],
    *,
    socket_path: str,
    activate: bool = True,
    persist: bool = True,
) -> Tuple[Optional[LiveApplyHost], Dict[str, object]]:
    """Apply windows, attach, and optionally persist the restore record.

    When native already owns a fresh lease, this does not start a
    competing in-memory host.
    """
    yielded = _foreign_payload("attach")
    if yielded is not None:
        yielded["restore_path"] = None
        yielded["apply"] = None
        yielded["attach"] = {"ok": True, "outcome": yielded.get("outcome")}
        return None, yielded
    host = LiveApplyHost(enabled=True, socket_path=socket_path)
    applied = host.apply_session(windows)
    attached = host.attach(sessions, activate=activate)
    path = persist_host_restore(host) if persist and attached.get("ok") else None
    if attached.get("ok"):
        _handoff().claim_plugin_writer(
            _fingerprint_key(),
            socket_path=socket_path,
            endpoint_hash=endpoint_hash(socket_path),
        )
    return host, {
        "ok": bool(applied.get("ok") and attached.get("ok")),
        "apply": applied,
        "attach": attached,
        "restore_path": path,
        "server_stopped": False,
        "writer": "plugin",
        "outcome": (attached or {}).get("outcome"),
    }


def restore_live(
    windows: Sequence[HerdrWindow],
    sessions: Sequence[DiscoveredSession],
    *,
    socket_path: str,
) -> Tuple[Optional[LiveApplyHost], Dict[str, object]]:
    """Reattach from the persist file. Never replays a stale tree."""
    yielded = _foreign_payload("restore")
    if yielded is not None:
        return None, yielded
    host = LiveApplyHost(enabled=True, socket_path=socket_path)
    hashed = endpoint_hash(socket_path)
    payload = _handoff().read_shared_restore(hashed)
    record = RestoreRecord.from_dict(payload) if payload else read_restore(
        restore_record_path(socket_path)
    )
    if record is None:
        return host, {"ok": False, "outcome": "no_persist", "server_stopped": False}
    host.lifecycle.persist = record
    restored = host.restore(sessions, windows)
    path = persist_host_restore(host) if restored.get("ok") else None
    if restored.get("ok"):
        _handoff().claim_plugin_writer(
            _fingerprint_key(),
            socket_path=socket_path,
            endpoint_hash=hashed,
        )
    restored["restore_path"] = path
    restored["server_stopped"] = False
    restored["writer"] = "plugin"
    return host, restored


def observe_live(
    windows: Sequence[HerdrWindow],
    *,
    socket_path: str,
    method: str,
    session: str = "main",
) -> Tuple[Optional[LiveApplyHost], Dict[str, object]]:
    """Serve ``remote.herdr.*``. Yields when the other path owns surfaces."""
    yielded = _foreign_payload("observe", method=method)
    if yielded is not None:
        return None, yielded
    host = apply_live_windows(windows)
    host.socket_path = socket_path
    return host, host.observe(method, {"socket": socket_path, "session": session})


def detach_live(
    windows: Sequence[HerdrWindow],
    *,
    socket_path: str,
) -> Dict[str, object]:
    """Detach plugin mirrors. Does not tear down a live native host."""
    yielded = _foreign_payload("detach")
    if yielded is not None:
        yielded["detached"] = False
        yielded["restore_cleared"] = False
        return yielded
    host = apply_live_windows(windows)
    host.socket_path = socket_path
    closed = host.detach()
    closed["restore_cleared"] = clear_host_restore(socket_path)
    closed["detached"] = True
    _handoff().release_plugin_writer(_fingerprint_key())
    return closed


__all__ = [
    "GhosttySurface",
    "LiveApplyHost",
    "LiveWindowMirror",
    "POST_APPLY_CLIENT_SIZE",
    "POST_RESEED",
    "SETTING_KEY",
    "SOCKET_METHODS",
    "TitleEscapeFilter",
    "apply_live_windows",
    "attach_live",
    "clear_host_restore",
    "decode_beta",
    "detach_live",
    "dispatch",
    "endpoint_hash",
    "grid_match",
    "observe_live",
    "persist_host_restore",
    "read_restore",
    "resolve_socket_path",
    "restore_live",
    "restore_record_path",
    "sessions_from_snapshot",
    "write_restore",
]
