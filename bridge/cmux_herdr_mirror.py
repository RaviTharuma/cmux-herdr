#!/usr/bin/env python3
"""Userspace deep mirror: project Herdr tabs/panes into real cmux tabs/splits.

This is the plugin analogue of cmux ``ssh-tmux`` / ``RemoteTmuxWindowMirror``.
``--tmux-parity`` turns on the same reconcile contract tmux gets natively:

- each Herdr tab → a cmux tab (first pane is the tab root)
- remaining panes → cmux splits driven by the Herdr layout tree
  (direction + first-child ratio), not alternate right/down
- tab order follows Herdr tab numbers (``cmux move-tab``)
- Herdr focused pane is projected onto the matching cmux surface
- gone panes are pruned (tmux closes vanished panes by default)
- zoom keeps mapped viewers; it does not destroy hidden pane surfaces
- each mirrored surface runs ``cmux-herdr attach-pane`` (``pane read`` +
  ``pane send-text``). SIGWINCH cannot claim the inner grid: Herdr
  ``pane.resize`` is split-edge only.

It cannot steal Herdr PTYs into Ghostty (that needs native
``RemoteHerdrWindowMirror``). It *can* create extra cmux viewers of the live
Herdr session, keyed so reconcile is idempotent.
"""

from __future__ import annotations

import json
import os
import select
import sys
import time
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, List, Optional, Sequence, Set, Tuple

try:
    from .cmux_herdr_bridge import (
        BridgeError,
        Pane,
        Snapshot,
        Tab,
        _load_association_map,
        _log_native_skip_once,
        _save_association_map,
        cmux_cmd,
        collect_host_fingerprint,
        fetch_snapshot,
        focus_pane,
        herdr_json,
        herdr_rpc,
        native_attachment_is_live,
        resolve_cmux_workspace,
        run_cmd,
        sync_to_cmux,
        which,
        writer_status,
    )
    from .cmux_herdr_engine import (
        HerdrWindow,
        WindowMirrorState,
        apply_window,
        output_delta,
        reconcile_session,
    )
    from .cmux_herdr_impose import specs_with_impose_fractions
    from .cmux_herdr_layout import (
        layouts_by_tab_id,
        pane_is_zoomed,
        pane_rects_from_objects,
        parse_layout,
        split_specs,
        tree_from_rects,
    )
    from .cmux_herdr_live import apply_live_windows
    from .cmux_herdr_socket import HerdrEventSession
except ImportError:  # running as a loose file with PYTHONPATH=bridge
    from cmux_herdr_bridge import (
        BridgeError,
        Pane,
        Snapshot,
        Tab,
        _load_association_map,
        _log_native_skip_once,
        _save_association_map,
        cmux_cmd,
        collect_host_fingerprint,
        fetch_snapshot,
        focus_pane,
        herdr_json,
        herdr_rpc,
        native_attachment_is_live,
        resolve_cmux_workspace,
        run_cmd,
        sync_to_cmux,
        which,
        writer_status,
    )
    from cmux_herdr_engine import (
        HerdrWindow,
        WindowMirrorState,
        apply_window,
        output_delta,
        reconcile_session,
    )
    from cmux_herdr_impose import specs_with_impose_fractions
    from cmux_herdr_layout import (
        layouts_by_tab_id,
        pane_is_zoomed,
        pane_rects_from_objects,
        parse_layout,
        split_specs,
        tree_from_rects,
    )
    from cmux_herdr_live import apply_live_windows
    from cmux_herdr_socket import HerdrEventSession

ATTACH_ENV = "CMUX_HERDR_ATTACH_PANE"
SIZE_AUTHORITY_ENV = "CMUX_HERDR_SIZE_AUTHORITY"
MIRROR_KEY_PREFIX = "herdr-mirror:"
DEFAULT_ATTACH_INTERVAL = 0.25


@dataclass(frozen=True)
class DesiredMirror:
    """One Herdr pane that should exist as a cmux surface."""

    pane_id: str
    tab_id: str
    workspace_id: str
    title: str
    role: str  # "tab-root" | "split"
    split_direction: str  # "right" | "down"
    agent: Optional[str] = None
    agent_status: str = "unknown"
    split_ratio: Optional[float] = None
    split_from_pane_id: Optional[str] = None
    tab_number: Optional[int] = None
    tab_index: Optional[int] = None
    focused: bool = False
    zoomed: bool = False
    visible: bool = True

    @property
    def key(self) -> str:
        return f"{MIRROR_KEY_PREFIX}{self.pane_id}"


@dataclass
class MirrorAction:
    """One reconcile step.

    ``op`` is create_tab, create_split, rename, keep, prune, set_ratio,
    move_tab, or focus.
    """

    op: str
    pane_id: str
    title: str
    tab_id: str = ""
    role: str = "tab-root"
    split_direction: str = "right"
    key: str = ""
    surface_id: Optional[str] = None
    split_from_surface_id: Optional[str] = None
    split_from_pane_id: Optional[str] = None
    ratio: Optional[float] = None
    tab_index: Optional[int] = None
    reason: str = ""


@dataclass
class MirrorPlan:
    actions: List[MirrorAction] = field(default_factory=list)
    scope: str = "current-tab"
    desired_count: int = 0

    @property
    def creates(self) -> List[MirrorAction]:
        return [a for a in self.actions if a.op in ("create_tab", "create_split")]

    @property
    def renames(self) -> List[MirrorAction]:
        return [a for a in self.actions if a.op == "rename"]

    @property
    def prunes(self) -> List[MirrorAction]:
        return [a for a in self.actions if a.op == "prune"]

    @property
    def keeps(self) -> List[MirrorAction]:
        return [a for a in self.actions if a.op == "keep"]

    @property
    def ratio_updates(self) -> List[MirrorAction]:
        return [a for a in self.actions if a.op == "set_ratio"]

    @property
    def moves(self) -> List[MirrorAction]:
        return [a for a in self.actions if a.op == "move_tab"]

    @property
    def focuses(self) -> List[MirrorAction]:
        return [a for a in self.actions if a.op == "focus"]


def mirror_key_for_pane(pane_id: str) -> str:
    """Return the idempotency key used for a mirrored Herdr pane."""
    return f"{MIRROR_KEY_PREFIX}{pane_id}"


def is_attach_process() -> bool:
    """True when this process is already a pane follower (do not nest mirror)."""
    return bool(os.environ.get(ATTACH_ENV))


def _pane_title(pane: Pane, tab: Optional[Tab], *, role: str) -> str:
    if role == "tab-root" and tab and tab.label:
        return str(tab.label)
    name = pane.display_name
    if name and name != pane.pane_id:
        return name
    if pane.agent:
        return f"{pane.agent}@{pane.pane_id}"
    if tab and tab.label:
        return str(tab.label)
    return pane.pane_id


def _split_direction_for_index(index: int) -> str:
    """Alternate right/down so a tab with many panes is not a single row."""
    return "right" if index % 2 == 1 else "down"


def _tab_layout_node(snapshot: Snapshot, tab_id: str, members: Sequence[Pane]):
    """Prefer Herdr's published tree; fall back to reconstructing from pane rects."""
    indexed = layouts_by_tab_id(getattr(snapshot, "layouts", None))
    node = indexed.get(tab_id)
    if node is not None:
        return node
    rects = pane_rects_from_objects(members)
    if len(rects) >= 2:
        return tree_from_rects(rects)
    if len(rects) == 1:
        return parse_layout(
            {
                "pane_id": rects[0][0],
                "x": rects[0][1].x,
                "y": rects[0][1].y,
                "width": rects[0][1].width,
                "height": rects[0][1].height,
            }
        )
    return None


def desired_mirrors(
    snapshot: Snapshot,
    *,
    scope: str = "current-tab",
    current_tab_id: Optional[str] = None,
    current_workspace_id: Optional[str] = None,
    use_layout: bool = True,
) -> List[DesiredMirror]:
    """Build the desired cmux projection from a Herdr snapshot.

    ``scope``:
    - ``current-tab`` — only the invoking Herdr tab (safe default)
    - ``workspace`` — every tab in the current Herdr workspace
    - ``all`` — every pane in the snapshot (ssh-tmux-style full session)

    When ``use_layout`` is true (default), split direction/ratio and pane
    create-order come from the Herdr layout tree or pane geometry — the same
    contract as ``RemoteTmuxWindowMirror.paneIDsInOrder``.
    """
    if scope not in ("current-tab", "workspace", "all"):
        raise BridgeError("scope must be current-tab, workspace, or all")

    tabs_by_id = {t.tab_id: t for t in snapshot.tabs if t.tab_id}
    panes = [p for p in snapshot.panes if p.pane_id]
    if scope == "current-tab":
        tab_id = current_tab_id or os.environ.get("HERDR_TAB_ID")
        if not tab_id:
            raise BridgeError(
                "scope current-tab needs HERDR_TAB_ID or --tab "
                "(or pass --all / --herdr-workspace)"
            )
        panes = [p for p in panes if p.tab_id == tab_id]
    elif scope == "workspace":
        workspace_id = current_workspace_id or os.environ.get("HERDR_WORKSPACE_ID")
        if not workspace_id:
            raise BridgeError(
                "scope workspace needs HERDR_WORKSPACE_ID or --herdr-workspace"
            )
        panes = [p for p in panes if p.workspace_id == workspace_id]

    grouped: Dict[str, List[Pane]] = {}
    for pane in panes:
        grouped.setdefault(pane.tab_id or pane.pane_id, []).append(pane)

    def tab_sort_key(tab_id: str) -> Tuple[int, int, str]:
        tab = tabs_by_id.get(tab_id)
        number = tab.number if tab and isinstance(tab.number, int) else 10**9
        return (0 if tab else 1, number, tab_id)

    ordered_tab_ids = sorted(grouped, key=tab_sort_key)
    desired: List[DesiredMirror] = []
    for tab_index, tab_id in enumerate(ordered_tab_ids):
        members = grouped[tab_id]
        tab = tabs_by_id.get(tab_id)
        spec_by_id: Dict[str, Any] = {}
        order: List[str] = []
        if use_layout:
            node = _tab_layout_node(snapshot, tab_id, members)
            if node is not None:
                order = node.pane_ids_in_order
                spec_by_id = {
                    spec.pane_id: spec for spec in specs_with_impose_fractions(node)
                }

        def member_key(pane: Pane) -> Tuple[int, int, str]:
            if order and pane.pane_id in order:
                return (0, order.index(pane.pane_id), pane.pane_id)
            return (1, 0 if pane.focused else 1, pane.pane_id)

        members_sorted = sorted(members, key=member_key)
        zoomed_id = next(
            (
                pane.pane_id
                for pane in members_sorted
                if pane.focused and pane_is_zoomed(pane.raw)
            ),
            None,
        )
        if zoomed_id is None:
            zoomed_id = next(
                (
                    pane.pane_id
                    for pane in members_sorted
                    if pane_is_zoomed(pane.raw)
                ),
                None,
            )
        for index, pane in enumerate(members_sorted):
            spec = spec_by_id.get(pane.pane_id)
            role = "tab-root" if index == 0 else "split"
            direction = spec.direction if spec else _split_direction_for_index(index)
            desired.append(
                DesiredMirror(
                    pane_id=pane.pane_id,
                    tab_id=tab_id,
                    workspace_id=pane.workspace_id,
                    title=_pane_title(pane, tab, role=role)[:80],
                    role=role,
                    split_direction=direction,
                    agent=pane.agent,
                    agent_status=pane.agent_status or "unknown",
                    split_ratio=spec.ratio if spec else None,
                    split_from_pane_id=spec.split_from_pane_id if spec else None,
                    tab_number=tab.number if tab else None,
                    tab_index=tab_index,
                    focused=bool(pane.focused),
                    zoomed=pane.pane_id == zoomed_id,
                    visible=zoomed_id is None or pane.pane_id == zoomed_id,
                )
            )
    return desired


def build_herdr_windows(
    snapshot: Snapshot,
    desired: Sequence[DesiredMirror],
) -> List[HerdrWindow]:
    """Build engine ``HerdrWindow`` values from a snapshot + desired projection.

    The engine uses the BASE layout for panel lifecycle (zoom never closes
    surfaces). Visible/zoom state is carried for focus and ratio renders.
    """
    by_tab: Dict[str, List[DesiredMirror]] = {}
    for item in desired:
        by_tab.setdefault(item.tab_id, []).append(item)
    panes_by_id = {p.pane_id: p for p in snapshot.panes if p.pane_id}
    windows: List[HerdrWindow] = []
    for tab_id, items in by_tab.items():
        members = [panes_by_id[i.pane_id] for i in items if i.pane_id in panes_by_id]
        node = _tab_layout_node(snapshot, tab_id, members)
        if node is None:
            ordered_ids = [i.pane_id for i in items]
            if not ordered_ids:
                continue
            if len(ordered_ids) == 1:
                node = parse_layout({"pane_id": ordered_ids[0]})
            else:
                node = parse_layout(
                    {"horizontal": [{"pane_id": pid} for pid in ordered_ids]}
                )
        if node is None:
            continue
        root = next((i for i in items if i.role == "tab-root"), items[0])
        zoomed = next((i for i in items if i.zoomed), None)
        active = next((i.pane_id for i in items if i.focused), None)
        visible = None
        if zoomed is not None:
            visible = parse_layout({"pane_id": zoomed.pane_id})
        windows.append(
            HerdrWindow(
                tab_id=tab_id,
                title=root.title,
                order_index=root.tab_index if root.tab_index is not None else 0,
                layout=node,
                visible_layout=visible,
                zoomed=zoomed is not None,
                active_pane_id=active or (zoomed.pane_id if zoomed else root.pane_id),
            )
        )
    return windows


def _window_state_from_mirrors(
    tab_id: str,
    window: HerdrWindow,
    existing: Dict[str, Any],
) -> Optional[WindowMirrorState]:
    """Rehydrate engine state from the association ``mirrors`` map."""
    surfaces: Dict[str, str] = {}
    version = 0
    for pane_id, entry in existing.items():
        if not isinstance(entry, dict):
            continue
        if str(entry.get("tab_id") or "") != tab_id:
            continue
        surface = entry.get("cmux_surface_id")
        if isinstance(surface, str) and surface:
            surfaces[pane_id] = surface
        raw_version = entry.get("layout_structure_version")
        if isinstance(raw_version, int):
            version = max(version, raw_version)
    if not surfaces and version == 0:
        return None
    return WindowMirrorState(
        tab_id=tab_id,
        title=window.title,
        layout=window.layout,
        visible_layout=window.visible_layout,
        zoomed=window.zoomed,
        active_pane_id=window.active_pane_id,
        pane_ids=list(surfaces.keys()) or list(window.base_pane_ids),
        layout_structure_version=version,
        surface_id_by_pane_id=surfaces,
    )


def reconcile_engine_for_desired(
    snapshot: Snapshot,
    desired: Sequence[DesiredMirror],
    existing: Dict[str, Any],
) -> Dict[str, Any]:
    """Run the window-mirror engine and return reconcile hints for planning.

    Returns:
        Dict with ``created_pane_ids``, ``closed_pane_ids``, ``protected_pane_ids``
        (base panes that must survive zoom), ``structure_changed_tabs``,
        ``ordered_tab_ids``, and per-tab ``states``.
    """
    windows = build_herdr_windows(snapshot, desired)
    created: List[str] = []
    closed: List[str] = []
    protected: Set[str] = set()
    structure_changed_tabs: Set[str] = set()
    states: Dict[str, WindowMirrorState] = {}
    for window in windows:
        protected.update(window.base_pane_ids)
        previous = _window_state_from_mirrors(window.tab_id, window, existing)
        state, result = apply_window(window, previous)
        states[window.tab_id] = state
        created.extend(result.created_pane_ids)
        closed.extend(result.closed_pane_ids)
        if result.structure_changed:
            structure_changed_tabs.add(window.tab_id)
    previous_tabs = sorted(
        {
            str(entry.get("tab_id"))
            for entry in existing.values()
            if isinstance(entry, dict) and entry.get("tab_id")
        }
    )
    session = reconcile_session(windows, previous_tabs)
    closed_tabs = set(session.closed_tab_ids)
    for pane_id, entry in existing.items():
        if not isinstance(entry, dict):
            continue
        if str(entry.get("tab_id") or "") in closed_tabs:
            closed.append(pane_id)
    return {
        "created_pane_ids": created,
        "closed_pane_ids": sorted(set(closed)),
        "protected_pane_ids": sorted(protected),
        "structure_changed_tabs": sorted(structure_changed_tabs),
        "ordered_tab_ids": session.ordered_tab_ids,
        "order_changed": session.order_changed,
        "states": states,
        "windows": windows,
    }


def size_authority_path(fp: Optional[Dict[str, Any]] = None) -> str:
    """Per-fingerprint file naming the single pane allowed to claim client size."""
    try:
        from .cmux_herdr_bridge import _parent_key, _state_dir
    except ImportError:
        from cmux_herdr_bridge import _parent_key, _state_dir
    return os.path.join(_state_dir(), f"size-authority-{_parent_key(fp)}")


def write_size_authority(pane_id: Optional[str], fp: Optional[Dict[str, Any]] = None) -> None:
    """Persist which mirrored pane may forward SIGWINCH to Herdr."""
    try:
        from .cmux_herdr_bridge import _state_dir
    except ImportError:
        from cmux_herdr_bridge import _state_dir
    path = size_authority_path(fp)
    directory = _state_dir()
    os.makedirs(directory, mode=0o700, exist_ok=True)
    if not pane_id:
        try:
            os.unlink(path)
        except FileNotFoundError:
            pass
        return
    temporary = f"{path}.tmp"
    with open(temporary, "w", encoding="utf-8") as handle:
        handle.write(pane_id.strip() + "\n")
    os.chmod(temporary, 0o600)
    os.replace(temporary, path)


def read_size_authority(fp: Optional[Dict[str, Any]] = None) -> Optional[str]:
    """Return the pane id currently allowed to claim Herdr client size."""
    try:
        with open(size_authority_path(fp), "r", encoding="utf-8") as handle:
            text = handle.read().strip()
        return text or None
    except OSError:
        return None


def may_claim_client_size(pane_id: str) -> bool:
    """True when this attach viewer is the sole size-claim writer.

    Policy (tmux feed-forward analogue): only one viewer per host fingerprint
    may call ``herdr pane resize``. Prefer ``CMUX_HERDR_SIZE_AUTHORITY``, then
    the size-authority state file written by mirror/watch on focus changes.

    When native owns the writer lease (or wrote the ``native`` size-authority
    sentinel), every plugin attach-pane must no-op so SIGWINCH does not thrash.
    """
    try:
        from .cmux_herdr_bridge import native_attachment_is_live
        from .cmux_herdr_handoff import FORCE_PLUGIN_ENV, env_truthy
    except ImportError:
        from cmux_herdr_bridge import native_attachment_is_live
        from cmux_herdr_handoff import FORCE_PLUGIN_ENV, env_truthy

    if native_attachment_is_live() and not env_truthy(FORCE_PLUGIN_ENV):
        return False

    env_auth = (os.environ.get(SIZE_AUTHORITY_ENV) or "").strip()
    if env_auth:
        return env_auth == pane_id
    file_auth = read_size_authority()
    if file_auth:
        # Native attachment writes the literal token ``native`` (no pane match).
        if file_auth == "native" or file_auth.startswith("native:"):
            return False
        return file_auth == pane_id
    # Single-viewer / first-attach fallback: allow until mirror elects one.
    return True


def plan_mirror(
    desired: Sequence[DesiredMirror],
    existing: Dict[str, Any],
    *,
    live_surface_ids: Optional[Set[str]] = None,
    prune: bool = False,
    sync_focus: bool = False,
    sync_order: bool = False,
    sync_ratios: bool = False,
    engine: Optional[Dict[str, Any]] = None,
) -> MirrorPlan:
    """Diff desired Herdr panes against the persisted mirror map.

    Idempotent: a second call with the same desired set and live surfaces
    yields only ``keep`` actions (plus ``rename`` when a title changed).
    Missing/dead mapped surfaces are recreated. Extra mapped panes are
    pruned only when ``prune`` is true (tmux-parity default).

    ``sync_ratios`` / ``sync_order`` / ``sync_focus`` emit extra actions that
    match ssh-tmux: impose divider fractions, tab order, and active pane.
    """
    existing_mirrors = existing if isinstance(existing, dict) else {}
    desired_ids = {item.pane_id for item in desired}
    tab_root_surface: Dict[str, str] = {}
    actions: List[MirrorAction] = []
    engine_hints = engine if isinstance(engine, dict) else {}
    protected = set(engine_hints.get("protected_pane_ids") or [])
    structure_changed_tabs = set(engine_hints.get("structure_changed_tabs") or [])
    engine_closed = set(engine_hints.get("closed_pane_ids") or [])
    # When the engine ran, only create panes it says are new — geometry-only
    # updates must not recreate splits. First-ever mirror has empty previous
    # state so every pane is in created_pane_ids.
    engine_created: Optional[Set[str]] = None
    if engine_hints:
        engine_created = set(engine_hints.get("created_pane_ids") or [])

    def mapped_surface(pane_id: str) -> Optional[str]:
        entry = existing_mirrors.get(pane_id)
        if not isinstance(entry, dict):
            return None
        surface = entry.get("cmux_surface_id")
        if not isinstance(surface, str) or not surface:
            return None
        if live_surface_ids is not None and surface not in live_surface_ids:
            return None
        return surface

    def _base(
        item: DesiredMirror,
        *,
        op: str,
        surface: Optional[str] = None,
        reason: str = "",
    ) -> MirrorAction:
        return MirrorAction(
            op=op,
            pane_id=item.pane_id,
            title=item.title,
            tab_id=item.tab_id,
            role=item.role,
            split_direction=item.split_direction,
            key=item.key,
            surface_id=surface,
            split_from_pane_id=item.split_from_pane_id,
            ratio=item.split_ratio,
            tab_index=item.tab_index,
            reason=reason,
        )

    for item in desired:
        surface = mapped_surface(item.pane_id)
        entry = existing_mirrors.get(item.pane_id)
        prior_title = ""
        title_locked = False
        if isinstance(entry, dict):
            prior_title = str(entry.get("title") or "")
            title_locked = bool(entry.get("title_lock"))
        if item.role == "tab-root" and surface:
            tab_root_surface[item.tab_id] = surface
        if surface:
            if item.role == "tab-root":
                tab_root_surface[item.tab_id] = surface
            if prior_title and prior_title != item.title and not title_locked:
                actions.append(_base(item, op="rename", surface=surface, reason="title changed"))
            else:
                actions.append(_base(item, op="keep", surface=surface))
            continue

        # Dead/missing surface: recreate only when the engine says this pane is
        # newly created OR we have no engine hints (legacy planner path).
        if engine_created is not None and item.pane_id not in engine_created:
            # Surface vanished but structure is unchanged — recreate allowed
            # only when the pane is still in the base tree (protected).
            if item.pane_id not in protected:
                continue

        if item.role == "tab-root":
            actions.append(_base(item, op="create_tab", reason="missing tab-root surface"))
        else:
            action = _base(item, op="create_split", reason="missing split surface")
            action.split_from_surface_id = tab_root_surface.get(item.tab_id)
            actions.append(action)

    if prune:
        for pane_id, entry in sorted(existing_mirrors.items()):
            if pane_id in desired_ids:
                continue
            if not isinstance(entry, dict):
                continue
            # Zoom / base-layout protection: never prune a pane the engine says
            # must keep a surface (tmux base-vs-visible contract).
            if pane_id in protected:
                continue
            # When the engine ran, only prune panes it explicitly closed.
            if engine_hints and pane_id not in engine_closed:
                continue
            surface = entry.get("cmux_surface_id")
            actions.append(
                MirrorAction(
                    op="prune",
                    pane_id=pane_id,
                    title=str(entry.get("title") or pane_id),
                    tab_id=str(entry.get("tab_id") or ""),
                    role=str(entry.get("role") or "split"),
                    key=str(entry.get("key") or mirror_key_for_pane(pane_id)),
                    surface_id=surface if isinstance(surface, str) else None,
                    reason="herdr pane gone",
                )
            )

    if sync_ratios:
        for item in desired:
            if item.role != "split" or item.split_ratio is None:
                continue
            surface = mapped_surface(item.pane_id)
            if not surface:
                continue
            entry = existing_mirrors.get(item.pane_id)
            prior = None
            if isinstance(entry, dict):
                prior = entry.get("split_ratio")
            if prior == item.split_ratio:
                continue
            actions.append(
                _base(
                    item,
                    op="set_ratio",
                    surface=surface,
                    reason="layout ratio",
                )
            )

    if sync_order:
        for item in desired:
            if item.role != "tab-root" or item.tab_index is None:
                continue
            surface = mapped_surface(item.pane_id)
            entry = existing_mirrors.get(item.pane_id)
            prior_index = None
            if isinstance(entry, dict) and isinstance(entry.get("tab_index"), int):
                prior_index = entry.get("tab_index")
            if surface and prior_index == item.tab_index:
                continue
            actions.append(
                _base(
                    item,
                    op="move_tab",
                    surface=surface,
                    reason="herdr tab order",
                )
            )

    if sync_focus:
        focused = next((item for item in desired if item.focused), None)
        if focused is None:
            focused = next((item for item in desired if item.zoomed), None)
        prior_focused = next(
            (
                pane_id
                for pane_id, entry in existing_mirrors.items()
                if isinstance(entry, dict) and entry.get("focused")
            ),
            None,
        )
        if focused is not None and prior_focused != focused.pane_id:
            actions.append(
                _base(
                    focused,
                    op="focus",
                    surface=mapped_surface(focused.pane_id),
                    reason="herdr focused pane",
                )
            )

    return MirrorPlan(actions=actions, desired_count=len(desired))


def _extract_cmux_id(payload: Any, *keys: str) -> Optional[str]:
    """Pull a surface/pane id out of heterogeneous cmux JSON."""
    if isinstance(payload, str) and payload.strip():
        return payload.strip()
    if not isinstance(payload, dict):
        return None
    for key in keys:
        value = payload.get(key)
        if isinstance(value, str) and value.strip():
            return value.strip()
    for nested_key in ("result", "payload", "surface", "pane", "terminal"):
        nested = payload.get(nested_key)
        found = _extract_cmux_id(nested, *keys) if nested is not None else None
        if found:
            return found
    return None


def parse_cmux_json(proc_stdout: str) -> Any:
    """Parse cmux CLI JSON, tolerating a leading OK line."""
    text = (proc_stdout or "").strip()
    if not text:
        return None
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        for line in text.splitlines():
            line = line.strip()
            if line.startswith("{") or line.startswith("["):
                return json.loads(line)
    return None


def cmux_json(args: Sequence[str], *, workspace: Optional[str] = None) -> Any:
    """Run a cmux CLI command preferring ``--json``."""
    with_json = list(args)
    if "--json" not in with_json:
        with_json.append("--json")
    proc = cmux_cmd(with_json, workspace=workspace)
    if proc.returncode == 0:
        parsed = parse_cmux_json(proc.stdout)
        if parsed is not None:
            return parsed
        return {"ok": True, "stdout": (proc.stdout or "").strip()}
    # Some cmux builds accept the verb without --json.
    proc = cmux_cmd(list(args), workspace=workspace)
    if proc.returncode != 0:
        err = (proc.stderr or proc.stdout or "").strip()
        raise BridgeError(f"cmux {' '.join(args)} failed: {err or proc.returncode}")
    parsed = parse_cmux_json(proc.stdout)
    return parsed if parsed is not None else {"ok": True, "stdout": (proc.stdout or "").strip()}


def _attach_argv(pane_id: str) -> List[str]:
    cli = which("cmux-herdr") or os.path.abspath(
        os.path.join(os.path.dirname(__file__), "..", "bin", "cmux-herdr")
    )
    return [cli, "attach-pane", pane_id]


def _create_terminal(
    *,
    key: str,
    name: str,
    command: str,
    workspace: Optional[str],
    pane: Optional[str] = None,
) -> Dict[str, Any]:
    """Create (or reuse) a cmux terminal running the attach follower."""
    attempts: List[List[str]] = [
        [
            "create-terminal",
            "--key",
            key,
            "--name",
            name,
            "--command",
            command,
        ],
        [
            "run",
            "--key",
            key,
            "--name",
            name,
            "--command",
            command,
        ],
        ["run", "--name", name, "--command", command],
    ]
    if pane:
        attempts.insert(
            0,
            [
                "create-terminal",
                "--key",
                key,
                "--name",
                name,
                "--command",
                command,
                "--pane",
                pane,
            ],
        )
    errors: List[str] = []
    for args in attempts:
        try:
            payload = cmux_json(args, workspace=workspace)
            surface = _extract_cmux_id(
                payload,
                "surface_id",
                "surface_ref",
                "id",
                "pane_id",
                "terminal_id",
            )
            return {
                "payload": payload,
                "cmux_surface_id": surface,
                "cmux_pane_id": _extract_cmux_id(
                    payload, "pane_id", "pane_ref", "pane"
                ),
                "args": args,
            }
        except BridgeError as exc:
            errors.append(str(exc))
    raise BridgeError(
        "could not create cmux terminal for mirror key "
        f"{key}: " + " | ".join(errors[-3:])
    )


def _split_pane(
    *,
    from_surface: str,
    direction: str,
    workspace: Optional[str],
) -> Dict[str, Any]:
    """Split an existing mirrored surface. Falls back to a new tab on failure."""
    dir_flag = "right" if direction == "right" else "down"
    attempts = [
        ["split", "--pane", from_surface, "--dir", dir_flag],
        ["split", from_surface, dir_flag],
        ["new-pane-right", "--pane", from_surface]
        if direction == "right"
        else ["new-pane", "--pane", from_surface],
    ]
    errors: List[str] = []
    for args in attempts:
        try:
            payload = cmux_json(args, workspace=workspace)
            return {
                "payload": payload,
                "cmux_surface_id": _extract_cmux_id(
                    payload, "surface_id", "surface_ref", "id"
                ),
                "cmux_pane_id": _extract_cmux_id(
                    payload, "pane_id", "pane_ref", "id"
                ),
                "args": args,
            }
        except BridgeError as exc:
            errors.append(str(exc))
    raise BridgeError(
        f"could not split cmux surface {from_surface}: " + " | ".join(errors[-3:])
    )


def _rename_surface(surface_id: str, title: str, *, workspace: Optional[str]) -> None:
    attempts = [
        ["rename-surface", surface_id, title],
        ["rename-surface", "--surface", surface_id, "--name", title],
    ]
    last_error = None
    for args in attempts:
        try:
            cmux_json(args, workspace=workspace)
            return
        except BridgeError as exc:
            last_error = exc
    if last_error:
        raise last_error


def _close_surface(surface_id: str, *, workspace: Optional[str]) -> None:
    attempts = [
        ["close-surface", surface_id],
        ["close-surface", "--surface", surface_id],
        ["close-terminal", surface_id],
    ]
    last_error = None
    for args in attempts:
        try:
            cmux_json(args, workspace=workspace)
            return
        except BridgeError as exc:
            last_error = exc
    if last_error:
        raise last_error


def _set_split_ratio(
    surface_id: str, ratio: float, *, workspace: Optional[str]
) -> None:
    """Impose a first-child divider fraction (tmux ``imposeDividerPlan`` analogue)."""
    ratio_s = f"{max(0.05, min(0.95, float(ratio))):.4f}"
    attempts = [
        ["set-ratio", "--pane", surface_id, "--ratio", ratio_s],
        ["set-split-ratio", "--pane", surface_id, "--ratio", ratio_s],
        ["set-ratio", surface_id, ratio_s],
        ["apply-layout", "--pane", surface_id, "--ratio", ratio_s],
    ]
    last_error = None
    for args in attempts:
        try:
            cmux_json(args, workspace=workspace)
            return
        except BridgeError as exc:
            last_error = exc
    if last_error:
        raise last_error


def _move_tab(
    surface_id: str, index: int, *, workspace: Optional[str]
) -> None:
    """Place a mirrored tab-root at Herdr's tab number order."""
    index_s = str(max(0, int(index)))
    attempts = [
        ["move-tab", "--surface", surface_id, "--index", index_s],
        ["move-tab", surface_id, index_s],
        ["move-tab", "--pane", surface_id, "--to", index_s],
    ]
    last_error = None
    for args in attempts:
        try:
            cmux_json(args, workspace=workspace)
            return
        except BridgeError as exc:
            last_error = exc
    if last_error:
        raise last_error


def _focus_surface(surface_id: str, *, workspace: Optional[str]) -> None:
    """Focus the cmux surface that mirrors Herdr's active pane."""
    attempts = [
        ["focus-surface", surface_id],
        ["select-pane", surface_id],
        ["focus", "--surface", surface_id],
        ["focus-pane", surface_id],
    ]
    last_error = None
    for args in attempts:
        try:
            cmux_json(args, workspace=workspace)
            return
        except BridgeError as exc:
            last_error = exc
    if last_error:
        raise last_error


def _cmux_focused_surface(*, workspace: Optional[str] = None) -> Optional[str]:
    """Best-effort currently focused cmux surface id."""
    for args in (
        ["identify", "--json"],
        ["focused", "--json"],
        ["identify"],
    ):
        try:
            payload = cmux_json(args, workspace=workspace)
        except BridgeError:
            continue
        found = _extract_cmux_id(
            payload, "surface_id", "surface_ref", "focused_surface_id", "id"
        )
        if found:
            return found
    return None


def list_live_surface_ids(*, workspace: Optional[str] = None) -> Optional[Set[str]]:
    """Best-effort set of live cmux surface ids. None means 'could not probe'."""
    for args in (["tree"], ["list-terminals"], ["ids", "--kind", "surface"]):
        try:
            payload = cmux_json(args, workspace=workspace)
        except BridgeError:
            continue
        found: Set[str] = set()
        _collect_ids(payload, found)
        if found:
            return found
    return None


def _collect_ids(node: Any, found: Set[str]) -> None:
    if isinstance(node, dict):
        for key, value in node.items():
            if key in (
                "surface_id",
                "surface_ref",
                "id",
                "terminal_id",
                "pane_id",
            ) and isinstance(value, str):
                found.add(value)
            else:
                _collect_ids(value, found)
    elif isinstance(node, list):
        for item in node:
            _collect_ids(item, found)


def load_mirrors() -> Dict[str, Any]:
    """Return the persisted pane_id → cmux surface map."""
    state = _load_association_map()
    mirrors = state.get("mirrors")
    return mirrors if isinstance(mirrors, dict) else {}


def save_mirrors(mirrors: Dict[str, Any], *, cmux_workspace: Optional[str] = None) -> None:
    """Persist the mirror map beside the association cache (same fingerprint file)."""
    state = _load_association_map()
    state["mirrors"] = mirrors
    if cmux_workspace:
        state["cmux_workspace"] = cmux_workspace
    _save_association_map(state)


def apply_mirror_plan(
    plan: MirrorPlan,
    *,
    existing: Dict[str, Any],
    workspace: Optional[str] = None,
    dry_run: bool = False,
    log: bool = True,
    engine_states: Optional[Dict[str, WindowMirrorState]] = None,
) -> Dict[str, Any]:
    """Execute a plan against cmux. Safe to re-run: keeps the map in sync."""
    mirrors = {key: dict(value) for key, value in existing.items() if isinstance(value, dict)}
    created: List[str] = []
    renamed: List[str] = []
    pruned: List[str] = []
    kept: List[str] = []
    ratios: List[str] = []
    moved: List[str] = []
    focused: List[str] = []
    errors: List[str] = []
    tab_root_surface: Dict[str, str] = {}
    states = engine_states if isinstance(engine_states, dict) else {}

    for pane_id, entry in mirrors.items():
        if entry.get("role") == "tab-root" and entry.get("cmux_surface_id"):
            tab_root_surface[str(entry.get("tab_id") or "")] = str(entry["cmux_surface_id"])

    for action in plan.actions:
        if action.op == "keep":
            kept.append(action.pane_id)
            if action.surface_id and action.role == "tab-root":
                tab_root_surface[action.tab_id] = action.surface_id
            continue
        if dry_run:
            continue
        try:
            if action.op == "rename" and action.surface_id:
                _rename_surface(action.surface_id, action.title, workspace=workspace)
                mirrors.setdefault(action.pane_id, {})["title"] = action.title
                renamed.append(action.pane_id)
            elif action.op == "prune" and action.surface_id:
                _close_surface(action.surface_id, workspace=workspace)
                mirrors.pop(action.pane_id, None)
                pruned.append(action.pane_id)
            elif action.op == "set_ratio" and action.surface_id and action.ratio is not None:
                _set_split_ratio(
                    action.surface_id, action.ratio, workspace=workspace
                )
                mirrors.setdefault(action.pane_id, {})["split_ratio"] = action.ratio
                ratios.append(action.pane_id)
            elif action.op == "move_tab" and action.tab_index is not None:
                surface = action.surface_id or (
                    mirrors.get(action.pane_id) or {}
                ).get("cmux_surface_id")
                if surface:
                    _move_tab(str(surface), action.tab_index, workspace=workspace)
                    mirrors.setdefault(action.pane_id, {})["tab_index"] = action.tab_index
                    moved.append(action.pane_id)
            elif action.op == "focus":
                surface = action.surface_id or (
                    mirrors.get(action.pane_id) or {}
                ).get("cmux_surface_id")
                if surface:
                    _focus_surface(str(surface), workspace=workspace)
                for pane_id, entry in mirrors.items():
                    if isinstance(entry, dict):
                        entry["focused"] = pane_id == action.pane_id
                focused.append(action.pane_id)
            elif action.op in ("create_tab", "create_split"):
                command = " ".join(_attach_argv(action.pane_id))
                created_info: Dict[str, Any]
                if action.op == "create_split":
                    split_from = None
                    if action.split_from_pane_id:
                        from_entry = mirrors.get(action.split_from_pane_id)
                        if isinstance(from_entry, dict):
                            split_from = from_entry.get("cmux_surface_id")
                    split_from = (
                        split_from
                        or action.split_from_surface_id
                        or tab_root_surface.get(action.tab_id)
                    )
                    if split_from:
                        try:
                            split_info = _split_pane(
                                from_surface=split_from,
                                direction=action.split_direction,
                                workspace=workspace,
                            )
                            created_info = _create_terminal(
                                key=action.key,
                                name=action.title,
                                command=command,
                                workspace=workspace,
                                pane=split_info.get("cmux_pane_id")
                                or split_info.get("cmux_surface_id"),
                            )
                            if not created_info.get("cmux_surface_id"):
                                created_info["cmux_surface_id"] = split_info.get(
                                    "cmux_surface_id"
                                )
                            created_info["cmux_pane_id"] = created_info.get(
                                "cmux_pane_id"
                            ) or split_info.get("cmux_pane_id")
                        except BridgeError as exc:
                            # Fail closed: never fall back to an orphan tab that
                            # destroys the layout tree and looks "green".
                            errors.append(
                                f"create_split {action.pane_id}: {exc} "
                                "(refusing orphan-tab fallback)"
                            )
                            continue
                    else:
                        errors.append(
                            f"create_split {action.pane_id}: no split-from surface "
                            "(refusing orphan-tab fallback)"
                        )
                        continue
                else:
                    created_info = _create_terminal(
                        key=action.key,
                        name=action.title,
                        command=command,
                        workspace=workspace,
                    )
                surface_id = created_info.get("cmux_surface_id")
                if surface_id and action.role == "tab-root":
                    tab_root_surface[action.tab_id] = str(surface_id)
                    try:
                        _rename_surface(str(surface_id), action.title, workspace=workspace)
                    except BridgeError:
                        pass
                mirrors[action.pane_id] = {
                    "pane_id": action.pane_id,
                    "tab_id": action.tab_id,
                    "role": action.role,
                    "title": action.title,
                    "key": action.key,
                    "cmux_surface_id": surface_id,
                    "cmux_pane_id": created_info.get("cmux_pane_id"),
                    "split_direction": action.split_direction,
                    "split_ratio": action.ratio,
                    "split_from_pane_id": action.split_from_pane_id,
                    "tab_index": action.tab_index,
                    "focused": False,
                    "updated_at": time.time(),
                }
                state = states.get(action.tab_id)
                if state is not None:
                    mirrors[action.pane_id]["layout_structure_version"] = (
                        state.layout_structure_version
                    )
                    if surface_id:
                        state.surface_id_by_pane_id[action.pane_id] = str(surface_id)
                if surface_id and action.op == "create_split" and action.ratio is not None:
                    try:
                        _set_split_ratio(
                            str(surface_id), action.ratio, workspace=workspace
                        )
                        ratios.append(action.pane_id)
                    except BridgeError as exc:
                        errors.append(f"set_ratio {action.pane_id}: {exc}")
                created.append(action.pane_id)
        except BridgeError as exc:
            errors.append(f"{action.op} {action.pane_id}: {exc}")

    if not dry_run:
        # Stamp layout_structure_version onto kept panes so the next engine
        # pass can detect geometry-only vs structural changes.
        for pane_id, entry in mirrors.items():
            if not isinstance(entry, dict):
                continue
            state = states.get(str(entry.get("tab_id") or ""))
            if state is None:
                continue
            entry["layout_structure_version"] = state.layout_structure_version
            surface = entry.get("cmux_surface_id")
            if isinstance(surface, str) and surface:
                state.surface_id_by_pane_id[pane_id] = surface
        save_mirrors(mirrors, cmux_workspace=workspace)
        if log:
            summary = (
                f"herdr mirror: created={len(created)} renamed={len(renamed)} "
                f"kept={len(kept)} pruned={len(pruned)} ratios={len(ratios)} "
                f"moved={len(moved)} focused={len(focused)} errors={len(errors)}"
            )
            try:
                cmux_cmd(["log", summary], workspace=workspace)
            except Exception:
                pass

    return {
        "created": created,
        "renamed": renamed,
        "kept": kept,
        "pruned": pruned,
        "ratios": ratios,
        "moved": moved,
        "focused": focused,
        "errors": errors,
        "dry_run": dry_run,
        "mirrors": mirrors,
        "actions": [
            {
                "op": a.op,
                "pane_id": a.pane_id,
                "title": a.title,
                "tab_id": a.tab_id,
                "role": a.role,
                "reason": a.reason,
                "split_direction": a.split_direction,
                "ratio": a.ratio,
                "tab_index": a.tab_index,
            }
            for a in plan.actions
        ],
    }


def mirror_to_cmux(
    *,
    scope: str = "current-tab",
    workspace: Optional[str] = None,
    herdr_workspace: Optional[str] = None,
    tab: Optional[str] = None,
    prune: bool = False,
    dry_run: bool = False,
    sync_status: bool = True,
    log: bool = True,
    use_layout: bool = True,
    sync_focus: bool = False,
    sync_order: bool = False,
    sync_ratios: bool = False,
    tmux_parity: bool = False,
    snapshot: Optional[Snapshot] = None,
) -> Dict[str, Any]:
    """Reconcile Herdr topology into cmux tabs/splits, then refresh status pills.

    ``tmux_parity`` is the ssh-tmux contract: full session, prune gone panes,
    layout-driven splits/ratios, tab order, and focus projection.

    Yields to native attachment when ``native_attachment_is_live`` (single writer).
    """
    if is_attach_process():
        raise BridgeError(
            "refusing to nest mirror inside attach-pane "
            f"({ATTACH_ENV}={os.environ.get(ATTACH_ENV)})"
        )
    writer = writer_status()
    if writer["native_live"]:
        _log_native_skip_once(log)
        empty_plan = {
            "created": [],
            "renamed": [],
            "kept": [],
            "pruned": [],
            "ratios": [],
            "moved": [],
            "focused": [],
            "errors": [],
            "dry_run": dry_run,
            "mirrors": load_mirrors(),
            "actions": [],
        }
        return {
            "scope": scope,
            "workspace": workspace,
            "desired_count": 0,
            "tmux_parity": tmux_parity,
            "sync_focus": sync_focus,
            "sync_order": sync_order,
            "sync_ratios": sync_ratios,
            "plan": empty_plan,
            "status_sync": None,
            "host_fingerprint": collect_host_fingerprint(),
            "writer": writer["writer"],
            "native_live": True,
            "skipped_reason": "native_live",
        }
    if tmux_parity:
        scope = "all"
        prune = True
        use_layout = True
        sync_focus = True
        sync_order = True
        sync_ratios = True
    snap = snapshot or fetch_snapshot()
    desired = desired_mirrors(
        snap,
        scope=scope,
        current_tab_id=tab,
        current_workspace_id=herdr_workspace,
        use_layout=use_layout,
    )
    ws = workspace
    if not ws and not dry_run:
        ws = resolve_cmux_workspace()
    existing = load_mirrors()
    engine = reconcile_engine_for_desired(snap, desired, existing)
    live_ids = None if dry_run else list_live_surface_ids(workspace=ws)
    plan = plan_mirror(
        desired,
        existing,
        live_surface_ids=live_ids,
        prune=prune,
        sync_focus=sync_focus,
        sync_order=sync_order,
        sync_ratios=sync_ratios,
        engine=engine,
    )
    plan.scope = scope
    applied = apply_mirror_plan(
        plan,
        existing=existing,
        workspace=ws,
        dry_run=dry_run,
        log=log,
        engine_states=engine.get("states") if isinstance(engine.get("states"), dict) else None,
    )
    if sync_focus and not dry_run:
        try:
            _sync_focus_from_cmux(applied.get("mirrors") or {}, workspace=ws)
        except BridgeError as exc:
            applied.setdefault("errors", []).append(f"focus reverse: {exc}")
        focused_id = next(
            (
                pane_id
                for pane_id, entry in (applied.get("mirrors") or {}).items()
                if isinstance(entry, dict) and entry.get("focused")
            ),
            None,
        )
        if not focused_id and desired:
            focused_id = next((d.pane_id for d in desired if d.focused), desired[0].pane_id)
        write_size_authority(focused_id)
    live_report = None
    if tmux_parity:
        live_windows = build_herdr_windows(snap, desired)
        live_host = apply_live_windows(live_windows)
        live_report = {
            "tabs": list(live_host.windows),
            "make_panel": [
                pane_id
                for mirror in live_host.windows.values()
                for pane_id in mirror.surfaces
            ],
            "pane_surfaces": live_host.pane_surfaces(),
            "pane_grids": live_host.pane_grids(),
            "defaults_open": live_host.defaults_open,
            "ops": list(live_host.log),
        }

    status_summary = None
    if sync_status and not dry_run:
        try:
            status_summary = sync_to_cmux(
                snap, workspace=ws, log=log
            )
        except BridgeError as exc:
            applied.setdefault("errors", []).append(f"status sync: {exc}")
    return {
        "scope": scope,
        "workspace": ws,
        "desired_count": len(desired),
        "tmux_parity": tmux_parity,
        "sync_focus": sync_focus,
        "sync_order": sync_order,
        "sync_ratios": sync_ratios,
        "plan": applied,
        "status_sync": status_summary,
        "host_fingerprint": collect_host_fingerprint(),
        "writer": writer["writer"],
        "native_live": False,
        "live": live_report,
        "engine": {
            "created_pane_ids": engine.get("created_pane_ids"),
            "closed_pane_ids": engine.get("closed_pane_ids"),
            "structure_changed_tabs": engine.get("structure_changed_tabs"),
            "order_changed": engine.get("order_changed"),
        },
    }


def _sync_focus_from_cmux(
    mirrors: Dict[str, Any], *, workspace: Optional[str]
) -> None:
    """If the user focused a mirrored cmux surface, forward that to Herdr.

    ssh-tmux sends ``select-pane`` on click. Plugin viewers do the same when
    we can see which cmux surface is focused. Herdr remains authority when
    the focused cmux surface is not one of ours.
    """
    surface = _cmux_focused_surface(workspace=workspace)
    if not surface:
        return
    for pane_id, entry in mirrors.items():
        if not isinstance(entry, dict):
            continue
        if entry.get("cmux_surface_id") != surface:
            continue
        if entry.get("focused"):
            return
        try:
            focus_pane(pane_id)
        except BridgeError:
            return
        for other_id, other in mirrors.items():
            if isinstance(other, dict):
                other["focused"] = other_id == pane_id
        save_mirrors(mirrors, cmux_workspace=workspace)
        return


def format_mirror_plan(result: Dict[str, Any]) -> str:
    """Human-readable mirror reconcile summary."""
    if result.get("native_live") or result.get("skipped_reason") == "native_live":
        return (
            f"herdr → cmux mirror  SKIPPED (native attachment live; "
            f"writer={result.get('writer') or 'native'})  "
            f"set CMUX_HERDR_FORCE_PLUGIN=1 to force plugin projection"
        )
    plan = result.get("plan") if isinstance(result.get("plan"), dict) else {}
    lines = [
        f"herdr → cmux mirror  scope={result.get('scope')}  "
        f"desired={result.get('desired_count', 0)}  "
        f"cmux_ws={result.get('workspace') or '-'}"
        + ("  DRY-RUN" if plan.get("dry_run") else ""),
        f"  created {len(plan.get('created') or [])}: "
        + ", ".join(plan.get("created") or []) ,
        f"  renamed {len(plan.get('renamed') or [])}: "
        + ", ".join(plan.get("renamed") or []),
        f"  kept    {len(plan.get('kept') or [])}",
        f"  pruned  {len(plan.get('pruned') or [])}: "
        + ", ".join(plan.get("pruned") or []),
        f"  ratios  {len(plan.get('ratios') or [])}",
        f"  moved   {len(plan.get('moved') or [])}",
        f"  focused {len(plan.get('focused') or [])}",
    ]
    if result.get("tmux_parity"):
        lines[0] += "  tmux-parity"
    live = result.get("live") if isinstance(result.get("live"), dict) else {}
    if live:
        lines.append(
            f"  live    panels={len(live.get('make_panel') or [])} "
            f"tabs={len(live.get('tabs') or [])}"
        )
    engine = result.get("engine") if isinstance(result.get("engine"), dict) else {}
    if engine.get("structure_changed_tabs") is not None:
        changed = engine.get("structure_changed_tabs") or []
        lines.append(
            f"  engine  structure_changed_tabs={changed or '[]'} "
            f"order_changed={engine.get('order_changed')}"
        )
    errors = plan.get("errors") or []
    if errors:
        lines.append(f"  errors  {len(errors)}")
        for err in errors[:12]:
            lines.append(f"    {err}")
    actions = plan.get("actions") or []
    if actions:
        lines.append("  actions:")
        for action in actions[:40]:
            lines.append(
                f"    {action.get('op'):12} {action.get('pane_id')}  "
                f"{action.get('title') or ''}  {action.get('reason') or ''}".rstrip()
            )
    return "\n".join(lines)


def send_pane_text(pane_id: str, text: str) -> None:
    """Forward text to a Herdr pane. Attach stays read-only if send is unavailable."""
    if not text:
        return
    try:
        herdr_rpc("pane.send_text", {"pane_id": pane_id, "text": text})
        return
    except BridgeError:
        pass
    if not which("herdr"):
        raise BridgeError("herdr not found on PATH")
    proc = run_cmd(["herdr", "pane", "send-text", pane_id, text], timeout=5.0)
    if proc.returncode == 0:
        return
    last_error = (proc.stderr or proc.stdout or str(proc.returncode)).strip()
    raise BridgeError(last_error or f"herdr pane send-text failed for {pane_id}")


def send_pane_named_key(pane_id: str, name: str) -> Dict[str, Any]:
    """Send a tmux-style named key (``C-Up``, ``F5``) as a Herdr combo.

    Encodes to ``ctrl+up`` / ``f5`` and calls ``pane.send_keys``. Falls back
    to CSI bytes on ``pane.send_text`` when send-keys is missing or the key
    has no combo (Home/End/Page). Never types the key name as literal text.
    Unknown names fail closed.
    """
    try:
        from .cmux_herdr_control import encode_named_key
    except ImportError:
        from cmux_herdr_control import encode_named_key

    item = encode_named_key(pane_id, name)
    if item is None:
        raise BridgeError(f"unknown key name: {name}")
    if item.key:
        try:
            herdr_rpc(
                "pane.send_keys",
                {"pane_id": pane_id, "keys": item.key, "key": item.key},
            )
            return {"pane_id": pane_id, "key": item.key, "via": "send_keys"}
        except BridgeError:
            pass
    if item.csi:
        send_pane_text(pane_id, item.csi.decode("latin-1"))
        return {"pane_id": pane_id, "key": item.key, "via": "csi"}
    raise BridgeError(f"could not send key {name} to {pane_id}")


def read_pane_text(pane_id: str, *, lines: int = 200, ansi: bool = True) -> str:
    """Read current Herdr pane contents for the attach follower.

    Socket-first ``pane.read`` (same wire as native SessionHost). Prefers
    ANSI so the viewer looks closer to a tmux ``%output`` feed. Falls back
    to the documented CLI when the socket is down.
    """
    try:
        from .cmux_herdr_api import extract_read_text
    except ImportError:
        from cmux_herdr_api import extract_read_text

    params: Dict[str, Any] = {
        "pane_id": pane_id,
        "source": "recent",
        "lines": lines,
    }
    if ansi:
        params["ansi"] = True
    try:
        text = extract_read_text(herdr_rpc("pane.read", params))
        if text:
            return text
    except BridgeError:
        pass
    if ansi:
        try:
            plain = dict(params)
            plain.pop("ansi", None)
            plain["source"] = "recent-unwrapped"
            text = extract_read_text(herdr_rpc("pane.read", plain))
            if text:
                return text
        except BridgeError:
            pass
    attempts: List[List[str]] = []
    if ansi:
        attempts.append(
            [
                "pane",
                "read",
                pane_id,
                "--source",
                "recent-unwrapped",
                "--lines",
                str(lines),
                "--ansi",
            ]
        )
        attempts.append(
            [
                "pane",
                "read",
                pane_id,
                "--source",
                "recent",
                "--lines",
                str(lines),
                "--raw",
            ]
        )
    attempts.append(
        ["pane", "read", pane_id, "--source", "recent-unwrapped", "--lines", str(lines)]
    )
    last_error: Optional[BridgeError] = None
    for args in attempts:
        try:
            data = herdr_json(args, timeout=8.0)
            if isinstance(data, dict):
                result = data.get("result") if isinstance(data.get("result"), dict) else data
                for key in ("text", "output", "content", "body"):
                    value = result.get(key) if isinstance(result, dict) else None
                    if isinstance(value, str):
                        return value
                if isinstance(result, dict) and isinstance(result.get("lines"), list):
                    return "\n".join(str(line) for line in result["lines"])
            return json.dumps(data, indent=2, default=str)
        except BridgeError as exc:
            last_error = exc
            continue
    proc = run_cmd(
        ["herdr", "pane", "read", pane_id, "--source", "recent-unwrapped", "--lines", str(lines)],
        timeout=8.0,
    )
    if proc.returncode != 0:
        raise BridgeError(
            (
                proc.stderr
                or proc.stdout
                or (str(last_error) if last_error else f"pane read failed for {pane_id}")
            ).strip()
        )
    return proc.stdout or ""


def resize_herdr_pane(pane_id: str, cols: int, rows: int) -> None:
    """Viewer size hint. Herdr has no claim-size ``pane.resize``.

    Official ``pane.resize`` is split-edge (``--direction`` + ``--amount``).
    Feed-forward inner PTY size is ``herdr terminal session control|observe
    --cols --rows``, a long-lived stream this plugin does not open. Calling
    invented ``--cols/--rows`` flags (or socket params) is a no-op on 0.8
    and is not attempted.
    """
    if cols <= 0 or rows <= 0 or not pane_id:
        return
    return


def attach_pane_loop(
    pane_id: str,
    *,
    interval: float = DEFAULT_ATTACH_INTERVAL,
    lines: int = 200,
    send_input: bool = True,
    stdout=None,
    clock: Callable[[], float] = time.time,
    sleeper: Callable[[float], None] = time.sleep,
    max_iterations: Optional[int] = None,
    read_once: Optional[Callable[[], str]] = None,
    raw_tty: bool = True,
    follow_resize: bool = True,
    ansi: bool = True,
) -> int:
    """Follow a Herdr pane in this terminal (plugin stand-in for a tmux PTY feed).

    ``max_iterations`` is for tests. Production attach runs until the pane
    disappears or the user hits Ctrl-C.

    ``raw_tty`` puts stdin in cbreak so keystrokes forward immediately
    (``send-keys`` analogue). ``follow_resize`` installs SIGWINCH only as
    a size-authority lock; Herdr cannot be claimed via ``pane.resize``.
    """
    out = stdout or sys.stdout
    os.environ[ATTACH_ENV] = pane_id
    last = None
    iteration = 0
    header = (
        f"cmux-herdr attach-pane {pane_id}  (Ctrl-C to detach this viewer; "
        "Herdr pane stays alive)\n"
    )
    old_tty = None
    termios_mod = None
    if raw_tty and send_input and sys.stdin.isatty():
        try:
            import termios
            import tty

            termios_mod = termios
            old_tty = termios.tcgetattr(sys.stdin.fileno())
            tty.setcbreak(sys.stdin.fileno())
        except Exception:
            old_tty = None
            termios_mod = None

    if follow_resize:
        _install_resize_handler(pane_id)

    try:
        while True:
            iteration += 1
            try:
                text = (
                    read_once()
                    if read_once
                    else read_pane_text(pane_id, lines=lines, ansi=ansi)
                )
            except BridgeError as exc:
                out.write(f"\ncmux-herdr: pane {pane_id} gone ({exc})\n")
                out.flush()
                return 1
            chunk, full_redraw = output_delta(last, text)
            if last is None or full_redraw:
                out.write("\033[H\033[2J")
                out.write(header)
                out.write(text)
                if not text.endswith("\n"):
                    out.write("\n")
                out.flush()
            elif chunk:
                out.write(chunk)
                out.flush()
            last = text
            if send_input:
                _drain_stdin_to_pane(pane_id)
            if max_iterations is not None and iteration >= max_iterations:
                return 0
            sleeper(max(0.05, interval))
            _ = clock()
    finally:
        if old_tty is not None and termios_mod is not None:
            try:
                termios_mod.tcsetattr(sys.stdin.fileno(), termios_mod.TCSADRAIN, old_tty)
            except Exception:
                pass


def _drain_stdin_to_pane(pane_id: str) -> None:
    if not sys.stdin.isatty():
        return
    try:
        ready, _, _ = select.select([sys.stdin], [], [], 0)
    except (OSError, ValueError):
        return
    if not ready:
        return
    chunk = sys.stdin.read(1)
    if not chunk:
        return
    try:
        send_pane_text(pane_id, chunk)
    except BridgeError:
        pass


def _install_resize_handler(pane_id: str) -> None:
    """Forward SIGWINCH only when this viewer is the size authority.

    Multiple attach-pane processes must not fight over inner size.
    Herdr 0.8 has no one-shot claim-size RPC, so this handler records
    the election and otherwise no-ops (see ``resize_herdr_pane``).
    """
    try:
        import shutil
        import signal
    except ImportError:
        return

    def _on_winch(_signum, _frame) -> None:  # noqa: ARG001
        if not may_claim_client_size(pane_id):
            return
        try:
            size = shutil.get_terminal_size()
            resize_herdr_pane(pane_id, size.columns, size.lines)
        except Exception:
            return

    try:
        signal.signal(signal.SIGWINCH, _on_winch)
        _on_winch(None, None)
    except (ValueError, OSError, AttributeError):
        return


def wait_herdr_event(*, timeout: float = 3.0) -> bool:
    """Block until a Herdr socket event arrives, or ``timeout`` elapses.

    One-shot helper for tests and callers that do not hold a session.
    ``watch --tmux-parity`` uses ``HerdrEventSession`` so subscribe stays open.

    Returns True when at least one event line was read.
    """
    session = HerdrEventSession.try_open(timeout=max(0.1, timeout))
    if session is None:
        time.sleep(max(0.05, timeout))
        return False
    try:
        return session.wait(timeout=timeout) is not None
    finally:
        session.close()
