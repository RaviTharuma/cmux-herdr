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
- native-live single-writer marker (native path only)

Plugin ceiling: surfaces are byte buffers, not Ghostty PTYs. The
*sequence* is the same one AppKit must run. Native
``RemoteHerdrLiveApply`` is the Swift twin.
"""

from __future__ import annotations

from dataclasses import dataclass, field
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

    def set_native_live(self, marker_writer) -> None:
        """Native AppKit claims the single-writer lock."""
        if not self.claim_native_writer:
            return
        marker_writer()
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
    "decode_beta",
    "dispatch",
    "endpoint_hash",
    "grid_match",
    "read_restore",
    "write_restore",
]
