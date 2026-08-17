#!/usr/bin/env python3
"""Host-agnostic Bonsplit impose plan (tmux ``imposeDividerPlan`` analogue).

AppKit/Bonsplit stay in native cmux. This module owns the ssh-tmux contract
the host must apply after each ``apply_window`` pass:

- right-associated binary split tree (first child vs combined rest)
- tree action: rebuild / keep / expand-leaf / remove-leaf
- divider fraction with the tmux +1 divider-cell formula
- exact first-child extent when parent size + cell metrics exist
- ``plan(w) <= w``: the plan parent never exceeds the banked region
- divider-drag session: begin / resolve hold / end → ``pane.resize`` cells

The plugin uses the same plan for ``cmux set-ratio``. Native
``RemoteHerdrImposePlan`` is the Swift twin.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import List, Optional, Sequence, Tuple, Union

try:
    from .cmux_herdr_layout import LayoutNode, LayoutRect, SplitSpec, split_specs
except ImportError:  # running as a loose file with PYTHONPATH=bridge
    from cmux_herdr_layout import LayoutNode, LayoutRect, SplitSpec, split_specs


@dataclass(frozen=True)
class ImposeSize:
    """Point (or pixel) size of a container or planned pane outer."""

    width: float
    height: float


@dataclass(frozen=True)
class ImposeMetrics:
    """Cell + chrome metrics used to turn cell rects into point extents.

    Production hosts pass Ghostty/Bonsplit numbers. Tests may use zeros for
    chrome and still exercise the ``plan(w) <= w`` invariant.
    """

    cell_width: float
    cell_height: float
    divider_thickness: float = 0.0
    tab_bar_height: float = 0.0
    surface_pad_width: float = 0.0
    surface_pad_height: float = 0.0
    minimum_pane_extent: float = 0.0


@dataclass(frozen=True)
class DividerLeaf:
    """Binary-tree leaf: one Herdr pane."""

    pane_id: str
    outer: Optional[ImposeSize] = None


@dataclass(frozen=True)
class DividerSplit:
    """Right-associated binary split (tmux ``RemoteTmuxNativeSplitTree``)."""

    orientation: str  # "horizontal" | "vertical"
    fraction: float
    first_extent: Optional[float]
    first: "DividerNode"
    second: "DividerNode"


DividerNode = Union[DividerLeaf, DividerSplit]


@dataclass(frozen=True)
class LeafExpansion:
    """Targeted +1 pane: split an existing leaf instead of rebuilding."""

    existing_pane_id: str
    new_pane_id: str
    orientation: str
    insert_first: bool
    fraction: float


@dataclass(frozen=True)
class TreeAction:
    """What the host must do to the Bonsplit tree before imposing extents."""

    kind: str  # rebuild | keep | expand_leaf | remove_leaf
    expansion: Optional[LeafExpansion] = None
    removed_pane_id: Optional[str] = None


@dataclass(frozen=True)
class DividerDragHold:
    """In-flight ``pane.resize`` after a user divider drag (tmux hold)."""

    split_key: str
    axis: str  # "horizontal" | "vertical"
    target_cells: int


@dataclass(frozen=True)
class ImposePlan:
    """One impose pass the host (or plugin ``set-ratio``) must apply."""

    tree_action: TreeAction
    divider_tree: DividerNode
    focus_pane_id: Optional[str]
    title: str
    held_split_key: Optional[str]
    fractions: Tuple[float, ...]


def clamp_ratio(value: float) -> float:
    """Keep a divider fraction inside the range cmux/Bonsplit accept."""
    return min(0.95, max(0.05, float(value)))


def divider_fraction(
    first_span: int,
    rest_spans: Sequence[int],
) -> float:
    """Tmux ``dividerFraction``: first / (first + rest + 1 divider cell)."""
    rest = sum(max(0, int(span)) for span in rest_spans)
    first = max(0, int(first_span))
    return clamp_ratio(first / max(1, first + rest + 1))


def region_bounded_plan_parent(
    render: Optional[ImposeSize],
    region: Optional[ImposeSize],
) -> Optional[ImposeSize]:
    """Parent a divider plan may divide: exact-fit frame bounded by the region.

    INVARIANT plan(w) <= w: never plan past the banked container. A
    claimed≠layout disagreement must heal through the claim channel, not
    by growing the window one point per pass.
    """
    parent = render if render is not None else region
    if parent is None:
        return None
    if region is None:
        return parent
    return ImposeSize(
        width=min(parent.width, region.width),
        height=min(parent.height, region.height),
    )


def same_shape_and_pane_ids(lhs: LayoutNode, rhs: LayoutNode) -> bool:
    """True when split nesting and pane ids match (geometry ignored)."""
    if lhs.kind != rhs.kind:
        return False
    if lhs.kind == "pane":
        return lhs.pane_id == rhs.pane_id
    if len(lhs.children) != len(rhs.children):
        return False
    return all(
        same_shape_and_pane_ids(left, right)
        for left, right in zip(lhs.children, rhs.children)
    )


def _span(node: LayoutNode, horizontal: bool) -> int:
    return node.rect.width if horizontal else node.rect.height


def _two_leaf_split(node: LayoutNode) -> Optional[Tuple[str, List[str], float]]:
    if node.kind == "pane" or len(node.children) != 2:
        return None
    pane_ids: List[str] = []
    for child in node.children:
        if child.kind != "pane" or not child.pane_id:
            return None
        pane_ids.append(child.pane_id)
    orientation = "horizontal" if node.kind == "horizontal" else "vertical"
    fraction = divider_fraction(
        _span(node.children[0], orientation == "horizontal"),
        [_span(node.children[1], orientation == "horizontal")],
    )
    return orientation, pane_ids, fraction


def leaf_expansion(
    old_node: LayoutNode,
    new_node: LayoutNode,
    added_pane_id: str,
) -> Optional[LeafExpansion]:
    """Find a +1 leaf expansion (tmux ``leafExpansion``)."""
    if old_node.kind == "pane" and old_node.pane_id:
        split = _two_leaf_split(new_node)
        if split and old_node.pane_id in split[1] and added_pane_id in split[1]:
            orientation, pane_ids, fraction = split
            return LeafExpansion(
                existing_pane_id=old_node.pane_id,
                new_pane_id=added_pane_id,
                orientation=orientation,
                insert_first=pane_ids[0] == added_pane_id,
                fraction=fraction,
            )
        return None
    if (
        old_node.kind in ("horizontal", "vertical")
        and old_node.kind == new_node.kind
        and len(old_node.children) == len(new_node.children)
    ):
        for old_child, new_child in zip(old_node.children, new_node.children):
            found = leaf_expansion(old_child, new_child, added_pane_id)
            if found is not None:
                return found
    return None


def tree_action(
    previous_rendered: Optional[LayoutNode],
    rendered: LayoutNode,
) -> TreeAction:
    """Decide rebuild vs keep vs targeted leaf expand/remove."""
    if previous_rendered is None:
        return TreeAction(kind="rebuild")
    if same_shape_and_pane_ids(previous_rendered, rendered):
        return TreeAction(kind="keep")
    old_ids = previous_rendered.pane_ids_in_order
    new_ids = rendered.pane_ids_in_order
    old_set = set(old_ids)
    new_set = set(new_ids)
    added = new_set - old_set
    removed = old_set - new_set
    if len(new_set) == len(old_set) + 1 and len(added) == 1:
        expansion = leaf_expansion(previous_rendered, rendered, next(iter(added)))
        if expansion is not None:
            return TreeAction(kind="expand_leaf", expansion=expansion)
    if len(old_set) == len(new_set) + 1 and len(removed) == 1:
        return TreeAction(kind="remove_leaf", removed_pane_id=next(iter(removed)))
    return TreeAction(kind="rebuild")


def _first_extent(
    *,
    first_span: int,
    rest_span: int,
    parent_extent: float,
    metrics: ImposeMetrics,
    horizontal: bool,
) -> Tuple[float, float]:
    """Return ``(first_extent, fraction)`` along one axis."""
    available = parent_extent - metrics.divider_thickness
    if available <= 0:
        fraction = divider_fraction(first_span, [rest_span])
        return 0.0, fraction
    cell = metrics.cell_width if horizontal else metrics.cell_height
    pad = metrics.surface_pad_width if horizontal else metrics.surface_pad_height
    first_ideal = first_span * cell + pad
    rest_ideal = rest_span * cell + pad
    total_ideal = first_ideal + rest_ideal
    if total_ideal <= 0:
        fraction = divider_fraction(first_span, [rest_span])
        return available * fraction, fraction
    raw = available * (first_ideal / total_ideal)
    floor = metrics.minimum_pane_extent
    if floor > 0 and available > 2 * floor:
        raw = min(available - floor, max(floor, raw))
    raw = min(available, max(0.0, raw))
    fraction = clamp_ratio(raw / available) if available else 0.5
    return raw, fraction


def binary_tree(
    node: LayoutNode,
    *,
    metrics: Optional[ImposeMetrics] = None,
    parent: Optional[ImposeSize] = None,
) -> DividerNode:
    """Right-associated binary view of an n-ary Herdr layout."""
    if node.kind == "pane":
        return DividerLeaf(pane_id=node.pane_id or "", outer=parent)
    horizontal = node.kind == "horizontal"
    orientation = "horizontal" if horizontal else "vertical"
    children = list(node.children)
    if not children:
        return DividerLeaf(pane_id="", outer=parent)
    if len(children) == 1:
        return binary_tree(children[0], metrics=metrics, parent=parent)

    first = children[0]
    rest = children[1:]
    first_span = _span(first, horizontal)
    rest_span = sum(_span(child, horizontal) for child in rest)
    first_size: Optional[ImposeSize] = None
    second_size: Optional[ImposeSize] = None
    first_extent: Optional[float] = None
    if parent is not None and metrics is not None:
        parent_extent = parent.width if horizontal else parent.height
        extent, fraction = _first_extent(
            first_span=first_span,
            rest_span=rest_span,
            parent_extent=parent_extent,
            metrics=metrics,
            horizontal=horizontal,
        )
        first_extent = extent
        if horizontal:
            first_size = ImposeSize(width=extent, height=parent.height)
            second_size = ImposeSize(
                width=max(0.0, parent.width - extent - metrics.divider_thickness),
                height=parent.height,
            )
        else:
            first_size = ImposeSize(width=parent.width, height=extent)
            second_size = ImposeSize(
                width=parent.width,
                height=max(0.0, parent.height - extent - metrics.divider_thickness),
            )
    else:
        fraction = divider_fraction(first_span, [_span(child, horizontal) for child in rest])

    rest_node = rest[0] if len(rest) == 1 else _combine(rest, horizontal)
    return DividerSplit(
        orientation=orientation,
        fraction=fraction,
        first_extent=first_extent,
        first=binary_tree(first, metrics=metrics, parent=first_size),
        second=binary_tree(rest_node, metrics=metrics, parent=second_size),
    )


def _combine(children: Sequence[LayoutNode], horizontal: bool) -> LayoutNode:
    """Synthesize the rest-of-split node (tmux ``combined(children:)``)."""
    items = list(children)
    if len(items) == 1:
        return items[0]
    xs = [child.rect.x for child in items]
    ys = [child.rect.y for child in items]
    rights = [child.rect.x + child.rect.width for child in items]
    bottoms = [child.rect.y + child.rect.height for child in items]
    min_x = min(xs)
    min_y = min(ys)
    rect = LayoutRect(
        min_x,
        min_y,
        max(1, max(rights) - min_x),
        max(1, max(bottoms) - min_y),
    )
    return LayoutNode(
        kind="horizontal" if horizontal else "vertical",
        children=list(items),
        rect=rect,
    )


def collect_fractions(node: DividerNode) -> Tuple[float, ...]:
    """Depth-first divider fractions (plugin ``set-ratio`` order)."""
    if isinstance(node, DividerLeaf):
        return ()
    return (node.fraction,) + collect_fractions(node.first) + collect_fractions(
        node.second
    )


def begin_divider_drag(
    split_key: str,
    axis: str,
    assigned_cells: int,
) -> DividerDragHold:
    """Start a divider-drag session (tmux ``splitTabBarDividerDragDidBegin``)."""
    return DividerDragHold(
        split_key=split_key,
        axis=axis,
        target_cells=max(1, int(assigned_cells)),
    )


def resolve_divider_hold(
    hold: Optional[DividerDragHold],
    *,
    assigned_cells: Optional[int],
    split_still_exists: bool,
) -> Optional[DividerDragHold]:
    """Clear the hold when the reply landed or the split vanished."""
    if hold is None:
        return None
    if not split_still_exists or assigned_cells is None:
        return None
    if assigned_cells == hold.target_cells:
        return None
    return hold


def end_divider_drag(
    *,
    dragged_extent: float,
    axis_span: float,
    total_cells: int,
    assigned_cells: int,
) -> Tuple[int, bool]:
    """Convert a settled drag into ``pane.resize`` cells.

    Returns ``(cells, should_send)``. A no-op (same cells) must not send —
    tmux never replies to a no-op, and the hold would park forever.
    """
    try:
        from .cmux_herdr_engine import resize_cells
    except ImportError:
        from cmux_herdr_engine import resize_cells

    cells = resize_cells(dragged_extent, axis_span, total_cells)
    return cells, cells != assigned_cells


def plan_impose(
    rendered: LayoutNode,
    *,
    previous_rendered: Optional[LayoutNode] = None,
    focus_pane_id: Optional[str] = None,
    title: str = "",
    metrics: Optional[ImposeMetrics] = None,
    render_size: Optional[ImposeSize] = None,
    region_size: Optional[ImposeSize] = None,
    hold: Optional[DividerDragHold] = None,
) -> ImposePlan:
    """Build the host impose plan for one rendered (visible) layout tree."""
    parent = region_bounded_plan_parent(render_size, region_size)
    tree = binary_tree(rendered, metrics=metrics, parent=parent)
    action = tree_action(previous_rendered, rendered)
    held = hold.split_key if hold is not None else None
    return ImposePlan(
        tree_action=action,
        divider_tree=tree,
        focus_pane_id=focus_pane_id,
        title=title,
        held_split_key=held,
        fractions=collect_fractions(tree),
    )


def specs_with_impose_fractions(node: LayoutNode) -> List[SplitSpec]:
    """``split_specs`` with tmux divider fractions overlaid in DFS order.

    Plugin ``cmux set-ratio`` cannot impose point extents; the fraction is
    the userspace stand-in for ``imposeDividerPlan``.
    """
    specs = split_specs(node)
    fractions = plan_impose(node).fractions
    overlaid: List[SplitSpec] = []
    for index, spec in enumerate(specs):
        ratio = fractions[index] if index < len(fractions) else spec.ratio
        overlaid.append(
            SplitSpec(
                pane_id=spec.pane_id,
                split_from_pane_id=spec.split_from_pane_id,
                direction=spec.direction,
                ratio=ratio,
            )
        )
    return overlaid


def plan_from_reconcile(
    result: object,
    *,
    previous_rendered: Optional[LayoutNode] = None,
    title: str = "",
    metrics: Optional[ImposeMetrics] = None,
    render_size: Optional[ImposeSize] = None,
    region_size: Optional[ImposeSize] = None,
    hold: Optional[DividerDragHold] = None,
) -> ImposePlan:
    """Impose plan from one ``apply_window`` result (engine → host)."""
    return plan_impose(
        result.rendered_layout,
        previous_rendered=previous_rendered,
        focus_pane_id=result.focus_pane_id,
        title=title,
        metrics=metrics,
        render_size=render_size,
        region_size=region_size,
        hold=hold,
    )
