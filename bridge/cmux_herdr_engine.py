#!/usr/bin/env python3
"""Pure Herdr window-mirror engine (Python twin of RemoteHerdrWindowMirror).

This is the ssh-tmux reconcile contract in userspace:

- panel lifecycle follows the BASE layout tree
- zoom never creates or closes surfaces
- geometry-only updates do not bump structure version
- session tab order follows Herdr tab numbers
- client-size claim is feed-forward (window geometry + cell metrics)
- pane-read polling yields an incremental output delta when possible

AppKit/Bonsplit/Ghostty stay in native cmux. The plugin applies the same
diffs via ``cmux split`` / ``attach-pane``. ``impose_after_apply`` is the
Python twin of native ``RemoteHerdrImposePlan`` (divider fractions, tree
action, drag-hold).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Dict, List, Optional, Sequence, Tuple

if TYPE_CHECKING:
    from .cmux_herdr_impose import ImposePlan

try:
    from .cmux_herdr_layout import LayoutNode, SplitSpec, split_specs
except ImportError:  # running as a loose file with PYTHONPATH=bridge
    from cmux_herdr_layout import LayoutNode, SplitSpec, split_specs


@dataclass
class HerdrWindow:
    """One Herdr tab as a tmux window: base layout, optional zoom, focus."""

    tab_id: str
    title: str
    order_index: int
    layout: LayoutNode
    visible_layout: Optional[LayoutNode] = None
    zoomed: bool = False
    active_pane_id: Optional[str] = None

    def __post_init__(self) -> None:
        """Drop visible layout unless zoomed (tmux BASE vs VISIBLE)."""
        if not self.zoomed:
            self.visible_layout = None

    @property
    def rendered_layout(self) -> LayoutNode:
        """Tree actually shown (zoomed leaf or base)."""
        return self.visible_layout or self.layout

    @property
    def base_pane_ids(self) -> List[str]:
        """Pane ids that must keep surfaces (zoom must not destroy them)."""
        return list(self.layout.pane_ids_in_order)


@dataclass
class WindowMirrorState:
    """Live mirror of one Herdr tab after the last ``apply_window`` pass."""

    tab_id: str
    title: str
    layout: LayoutNode
    visible_layout: Optional[LayoutNode]
    zoomed: bool
    active_pane_id: Optional[str]
    pane_ids: List[str]
    layout_structure_version: int
    surface_id_by_pane_id: Dict[str, str] = field(default_factory=dict)


@dataclass
class ReconcileResult:
    """Diff produced by one ``apply_window`` pass."""

    created_pane_ids: List[str]
    closed_pane_ids: List[str]
    kept_pane_ids: List[str]
    structure_changed: bool
    title_changed: bool
    focus_pane_id: Optional[str]
    split_specs: List[SplitSpec]
    rendered_layout: LayoutNode


@dataclass
class SessionReconcile:
    """Tab-set diff (tmux RemoteTmuxSessionMirror analogue)."""

    created_tab_ids: List[str]
    closed_tab_ids: List[str]
    kept_tab_ids: List[str]
    ordered_tab_ids: List[str]
    order_changed: bool


def apply_window(
    window: HerdrWindow,
    previous: Optional[WindowMirrorState],
) -> Tuple[WindowMirrorState, ReconcileResult]:
    """Apply a full window update. Zoom never creates or closes panels.

    Args:
        window: Desired Herdr tab (base + optional visible/zoom).
        previous: Prior mirror state, or None on first apply.

    Returns:
        Updated state and the host actions implied by the diff.
    """
    live = window.base_pane_ids
    live_set = set(live)
    previous_ids = list(previous.pane_ids) if previous else []
    previous_set = set(previous_ids)
    created = [pane_id for pane_id in live if pane_id not in previous_set]
    closed = [pane_id for pane_id in previous_ids if pane_id not in live_set]
    kept = [pane_id for pane_id in live if pane_id in previous_set]
    if previous is None:
        structure_changed = True
    else:
        structure_changed = (
            previous.layout.structure_signature() != window.layout.structure_signature()
        )
    title_changed = True if previous is None else previous.title != window.title
    version = previous.layout_structure_version if previous else 0
    if structure_changed and previous is not None:
        version += 1
    focus = None
    if window.active_pane_id and window.active_pane_id in live_set:
        focus = window.active_pane_id
    elif live:
        focus = live[0]
    surfaces = dict(previous.surface_id_by_pane_id) if previous else {}
    for pane_id in closed:
        surfaces.pop(pane_id, None)
    state = WindowMirrorState(
        tab_id=window.tab_id,
        title=window.title,
        layout=window.layout,
        visible_layout=window.visible_layout,
        zoomed=window.zoomed,
        active_pane_id=focus,
        pane_ids=live,
        layout_structure_version=version,
        surface_id_by_pane_id=surfaces,
    )
    created_set = set(created)
    specs = [spec for spec in split_specs(window.layout) if spec.pane_id in created_set]
    result = ReconcileResult(
        created_pane_ids=created,
        closed_pane_ids=closed,
        kept_pane_ids=kept,
        structure_changed=structure_changed,
        title_changed=title_changed,
        focus_pane_id=focus,
        split_specs=specs,
        rendered_layout=window.rendered_layout,
    )
    return state, result


def bind_surface(state: WindowMirrorState, pane_id: str, surface_id: str) -> None:
    """Record a host surface id after TerminalPanel / attach-pane creation."""
    state.surface_id_by_pane_id[pane_id] = surface_id


def reconcile_session(
    windows: Sequence[HerdrWindow],
    previous_tab_ids: Sequence[str],
) -> SessionReconcile:
    """Diff desired windows against previously mirrored tab ids.

    Args:
        windows: Herdr tabs (any order).
        previous_tab_ids: Tab ids currently mirrored, in host order.

    Returns:
        Create/close/keep plus whether kept-tab order changed.
    """
    ordered = [
        window.tab_id
        for window in sorted(
            windows, key=lambda item: (item.order_index, item.tab_id)
        )
    ]
    desired = set(ordered)
    previous = list(previous_tab_ids)
    previous_set = set(previous)
    created = [tab_id for tab_id in ordered if tab_id not in previous_set]
    closed = [tab_id for tab_id in previous if tab_id not in desired]
    kept = [tab_id for tab_id in ordered if tab_id in previous_set]
    previous_live = [tab_id for tab_id in previous if tab_id in desired]
    kept_in_desired = [tab_id for tab_id in ordered if tab_id in previous_set]
    return SessionReconcile(
        created_tab_ids=created,
        closed_tab_ids=closed,
        kept_tab_ids=kept,
        ordered_tab_ids=ordered,
        order_changed=previous_live != kept_in_desired,
    )


def client_grid(
    content_width: float,
    content_height: float,
    cell_width: float,
    cell_height: float,
    chrome_width: float = 0.0,
    chrome_height: float = 0.0,
) -> Optional[Tuple[int, int]]:
    """Feed-forward client-size claim (tmux ``updateClientSize`` analogue).

    Reads only window geometry, chrome constants, and cell metrics — never a
    measured pane frame. That invariant prevents the container from growing
    each pass.

    Returns:
        ``(cols, rows)`` or None when the claim would be empty.
    """
    if cell_width <= 0 or cell_height <= 0:
        return None
    available_width = content_width - chrome_width
    available_height = content_height - chrome_height
    if available_width <= 0 or available_height <= 0:
        return None
    cols = int(available_width / cell_width)
    rows = int(available_height / cell_height)
    if cols < 1 or rows < 1:
        return None
    return cols, rows


def resize_cells(dragged_extent: float, axis_span: float, total_cells: int) -> int:
    """Convert a dragged first-child extent into a cell span for ``pane.resize``.

    Args:
        dragged_extent: First-child size in the same units as ``axis_span``.
        axis_span: Full split axis length.
        total_cells: Herdr grid size along that axis.

    Returns:
        Cell count in ``[1, total_cells - 1]`` (or 1 when total is 1).
    """
    if axis_span <= 0 or total_cells < 1:
        return 1
    fraction = min(0.95, max(0.05, dragged_extent / axis_span))
    cells = int(round(fraction * total_cells))
    return min(max(total_cells - 1, 1), max(1, cells))


def output_delta(previous: Optional[str], current: str) -> Tuple[str, bool]:
    """Incremental pane-output delta (plugin stand-in for tmux ``%output``).

    If ``current`` extends ``previous``, return only the suffix. Otherwise
    the host must full-redraw (scrollback replaced / screen reset).

    Args:
        previous: Last painted snapshot, or None on first paint.
        current: Latest ``pane.read`` text.

    Returns:
        ``(chunk, full_redraw)``. Empty chunk means no write.
    """
    if previous is None:
        return current, True
    if current == previous:
        return "", False
    if current.startswith(previous):
        return current[len(previous) :], False
    return current, True


def impose_after_apply(
    result: ReconcileResult,
    previous_rendered: Optional[LayoutNode] = None,
    title: str = "",
) -> "ImposePlan":
    """Host impose plan for one ``apply_window`` result (Swift twin entry)."""
    try:
        from .cmux_herdr_impose import plan_from_reconcile
    except ImportError:
        from cmux_herdr_impose import plan_from_reconcile

    return plan_from_reconcile(
        result,
        previous_rendered=previous_rendered,
        title=title,
    )
