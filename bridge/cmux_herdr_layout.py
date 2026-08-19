#!/usr/bin/env python3
"""Herdr layout trees → cmux split plan (tmux ``RemoteTmuxLayoutNode`` analogue).

cmux ssh-tmux treats a window layout as a cell-grid tree:

- ``pane`` — a leaf
- ``horizontal`` — children left → right (cmux split ``right``)
- ``vertical`` — children top → bottom (cmux split ``down``)

Herdr publishes the same idea via ``session.snapshot`` layouts, ``herdr pane layout``,
or per-pane geometry. This module parses those shapes, reconstructs a
binary tree from rects when only geometry is present, and emits a sequential
split plan the userspace mirror can apply with ``cmux split`` / ``set-ratio``.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, Iterable, List, Optional, Sequence, Tuple


@dataclass(frozen=True)
class LayoutRect:
    """Cell (or pixel) rectangle. Units are whatever Herdr reported."""

    x: int = 0
    y: int = 0
    width: int = 1
    height: int = 1

    def union(self, other: "LayoutRect") -> "LayoutRect":
        """Return the bounding rectangle of ``self`` and ``other``."""
        x = min(self.x, other.x)
        y = min(self.y, other.y)
        right = max(self.x + self.width, other.x + other.width)
        bottom = max(self.y + self.height, other.y + other.height)
        return LayoutRect(x, y, max(1, right - x), max(1, bottom - y))

    def first_child_ratio(self, child: "LayoutRect", axis: str) -> float:
        """Fraction of this rect occupied by ``child`` along ``axis``.

        ``axis`` is ``horizontal`` (width) or ``vertical`` (height). Clamped so
        cmux dividers never receive 0/1 (which some CLIs reject).
        """
        if axis == "horizontal":
            span = self.width
            part = child.width
        else:
            span = self.height
            part = child.height
        if span <= 0:
            return 0.5
        return max(0.05, min(0.95, part / span))


@dataclass
class LayoutNode:
    """One node in a Herdr tab's pane tree.

    ``kind`` is ``pane``, ``horizontal``, or ``vertical`` — the same vocabulary
    as ``RemoteTmuxLayoutNode``.
    """

    kind: str
    pane_id: Optional[str] = None
    children: List["LayoutNode"] = field(default_factory=list)
    rect: LayoutRect = field(default_factory=LayoutRect)

    @property
    def pane_ids_in_order(self) -> List[str]:
        """Depth-first left→right leaf ids (tmux ``paneIDsInOrder``)."""
        if self.kind == "pane":
            return [self.pane_id] if self.pane_id else []
        out: List[str] = []
        for child in self.children:
            out.extend(child.pane_ids_in_order)
        return out

    def first_child_ratio(self) -> Optional[float]:
        """Divider fraction for the first child of a split, or None for a leaf."""
        if self.kind == "pane" or len(self.children) < 2:
            return None
        axis = "horizontal" if self.kind == "horizontal" else "vertical"
        return self.rect.first_child_ratio(self.children[0].rect, axis)

    def structure_signature(self) -> str:
        """Stable fingerprint of split nesting + pane set (not geometry)."""
        if self.kind == "pane":
            return f"p:{self.pane_id or ''}"
        inner = ",".join(child.structure_signature() for child in self.children)
        return f"{self.kind[0]}:{inner}"


@dataclass(frozen=True)
class SplitSpec:
    """One sequential cmux split needed to realize a layout tree."""

    pane_id: str
    split_from_pane_id: str
    direction: str  # "right" | "down"
    ratio: Optional[float] = None


def _as_int(value: Any, default: int = 0) -> int:
    if isinstance(value, bool):
        return default
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        return int(value)
    if isinstance(value, str) and value.strip():
        try:
            return int(float(value.strip()))
        except ValueError:
            return default
    return default


def rect_from_mapping(raw: Any) -> Optional[LayoutRect]:
    """Best-effort rectangle from a pane dict, geometry object, or layout node."""
    if not isinstance(raw, dict):
        return None
    geom = raw.get("geometry") if isinstance(raw.get("geometry"), dict) else None
    rect = raw.get("rect") if isinstance(raw.get("rect"), dict) else None
    src = geom or rect or raw
    width = _as_int(
        src.get("width")
        or src.get("cols")
        or src.get("columns")
        or src.get("pane_width")
        or src.get("w"),
        0,
    )
    height = _as_int(
        src.get("height")
        or src.get("rows")
        or src.get("pane_height")
        or src.get("h"),
        0,
    )
    x = _as_int(src.get("x") or src.get("left") or src.get("pane_left") or src.get("col"), 0)
    y = _as_int(src.get("y") or src.get("top") or src.get("pane_top") or src.get("row"), 0)
    if width <= 0 and height <= 0:
        return None
    return LayoutRect(x=x, y=y, width=max(1, width), height=max(1, height))


def pane_is_zoomed(raw: Any) -> bool:
    """True when Herdr reports this pane as the zoomed (visible) leaf."""
    if not isinstance(raw, dict):
        return False
    if raw.get("zoomed") or raw.get("is_zoomed") or raw.get("zoom"):
        return True
    flags = str(raw.get("flags") or "")
    return "Z" in flags


def parse_layout(raw: Any) -> Optional[LayoutNode]:
    """Parse a Herdr/tmux-shaped layout payload into a ``LayoutNode``.

    Accepted shapes (recursive):

    - tmux JSON: ``{width,height,x,y, pane|horizontal|vertical}``
    - binary split: ``{type:split, direction, ratio, first, second}``
    - n-ary split: ``{kind:hsplit|vsplit, children, sizes}``
    - leaf: ``{type:pane, pane_id}`` / ``{pane_id}``
    - wrapped: ``{layout: ...}`` / ``{root: ...}`` / ``{tree: ...}``
    """
    if raw is None:
        return None
    if isinstance(raw, list):
        if not raw:
            return None
        if len(raw) == 1:
            return parse_layout(raw[0])
        children = [parse_layout(item) for item in raw]
        children = [c for c in children if c is not None]
        if not children:
            return None
        return _nary_split("vertical", children)
    if not isinstance(raw, dict):
        return None

    for wrap in ("layout", "root", "tree", "node"):
        if wrap in raw and raw[wrap] is not raw:
            nested = parse_layout(raw[wrap])
            if nested:
                return nested

    pane_id = _pane_id_from(raw)
    kind = str(raw.get("kind") or raw.get("type") or raw.get("orientation") or "").lower()

    if "horizontal" in raw and isinstance(raw["horizontal"], list):
        return _from_tmux_named(raw, "horizontal")
    if "vertical" in raw and isinstance(raw["vertical"], list):
        return _from_tmux_named(raw, "vertical")

    if kind in ("pane", "leaf") or (pane_id and not _looks_like_split(raw, kind)):
        rect = rect_from_mapping(raw) or LayoutRect()
        return LayoutNode(kind="pane", pane_id=pane_id, rect=rect)

    direction = _normalize_direction(
        raw.get("direction")
        or raw.get("dir")
        or raw.get("axis")
        or (kind if kind != "split" else None)
    )
    children_raw = (
        raw.get("children")
        or raw.get("nodes")
        or raw.get("panes")
    )
    first = raw.get("first") or raw.get("a") or raw.get("left") or raw.get("top")
    second = raw.get("second") or raw.get("b") or raw.get("right") or raw.get("bottom")
    if first is not None or second is not None:
        kids = [parse_layout(first), parse_layout(second)]
        kids = [k for k in kids if k is not None]
        if kids:
            return _nary_split(direction or "horizontal", kids, raw)

    if isinstance(children_raw, list) and children_raw:
        kids = [parse_layout(item) for item in children_raw]
        kids = [k for k in kids if k is not None]
        if kids:
            return _nary_split(direction or "horizontal", kids, raw)

    if pane_id:
        return LayoutNode(
            kind="pane",
            pane_id=pane_id,
            rect=rect_from_mapping(raw) or LayoutRect(),
        )
    return None


def layouts_by_tab_id(raw: Any) -> Dict[str, LayoutNode]:
    """Index parsed layout trees by Herdr ``tab_id``.

    Accepts a dict of tab_id → tree, a list of ``{tab_id, layout}``, or a
    session snapshot that nests ``layouts`` / ``tabs``.
    """
    out: Dict[str, LayoutNode] = {}
    if raw is None:
        return out
    if isinstance(raw, dict):
        nested = raw.get("layouts") or raw.get("result")
        if isinstance(nested, (dict, list)) and nested is not raw:
            out.update(layouts_by_tab_id(nested))
            if out:
                return out
        if "tab_id" in raw and ("layout" in raw or "tree" in raw or "root" in raw):
            node = parse_layout(raw)
            tab_id = str(raw.get("tab_id") or "")
            if node and tab_id:
                out[tab_id] = node
            return out
        # Map of tab_id → tree, but skip snapshot metadata keys.
        skip = {
            "type",
            "workspaces",
            "tabs",
            "panes",
            "agents",
            "focused",
            "result",
        }
        for key, value in raw.items():
            if key in skip or not isinstance(key, str):
                continue
            node = parse_layout(value)
            if node:
                out[str(key)] = node
        if "tabs" in raw and isinstance(raw["tabs"], list):
            out.update(layouts_by_tab_id(raw["tabs"]))
        return out
    if isinstance(raw, list):
        for item in raw:
            if isinstance(item, dict) and item.get("tab_id"):
                node = parse_layout(item)
                if node:
                    out[str(item["tab_id"])] = node
            elif isinstance(item, dict):
                out.update(layouts_by_tab_id(item))
    return out


def tree_from_rects(
    items: Sequence[Tuple[str, LayoutRect]],
) -> Optional[LayoutNode]:
    """Reconstruct a binary split tree from pane rectangles (BSP).

    Prefers a left/right cut (horizontal children) when both axes work.
    Returns None when fewer than one pane is supplied.
    """
    cleaned = [(pid, rect) for pid, rect in items if pid]
    if not cleaned:
        return None
    return _bsp(cleaned)


def split_specs(root: LayoutNode) -> List[SplitSpec]:
    """Sequential splits to create every non-root leaf under ``root``.

    The first DFS leaf is the tab-root (already a cmux tab). Each later first
    leaf of a sibling is split off the previous sibling's first leaf, matching
    how ``cmux split --dir right|down`` grows a tree without Bonsplit APIs.
    """
    specs: List[SplitSpec] = []

    def walk(node: LayoutNode) -> None:
        if node.kind == "pane" or len(node.children) < 2:
            for child in node.children:
                walk(child)
            return
        direction = "right" if node.kind == "horizontal" else "down"
        ratio = node.first_child_ratio()
        first_leaves = [child.pane_ids_in_order for child in node.children]
        anchor = first_leaves[0][0] if first_leaves[0] else ""
        for index, child in enumerate(node.children):
            if index == 0:
                walk(child)
                continue
            leaves = first_leaves[index]
            if not leaves or not anchor:
                walk(child)
                continue
            specs.append(
                SplitSpec(
                    pane_id=leaves[0],
                    split_from_pane_id=anchor,
                    direction=direction,
                    ratio=ratio if index == 1 else _remaining_ratio(node, index),
                )
            )
            walk(child)
            anchor = leaves[0]

    walk(root)
    return specs


def _remaining_ratio(node: LayoutNode, index: int) -> Optional[float]:
    """Ratio of child ``index-1`` versus the rest of the split from that child on."""
    rest = node.children[index - 1 :]
    if len(rest) < 2:
        return None
    bound = rest[0].rect
    for child in rest[1:]:
        bound = bound.union(child.rect)
    axis = "horizontal" if node.kind == "horizontal" else "vertical"
    return bound.first_child_ratio(rest[0].rect, axis)


def _pane_id_from(raw: Dict[str, Any]) -> Optional[str]:
    for key in ("pane_id", "pane", "id"):
        value = raw.get(key)
        if isinstance(value, (str, int)) and str(value).strip():
            text = str(value).strip()
            if key == "id" and text in ("horizontal", "vertical", "split"):
                continue
            return text
    return None


def _looks_like_split(raw: Dict[str, Any], kind: str) -> bool:
    if kind in (
        "split",
        "hsplit",
        "vsplit",
        "horizontal",
        "vertical",
        "row",
        "column",
        "cols",
        "rows",
    ):
        return True
    return any(
        key in raw
        for key in (
            "children",
            "first",
            "second",
            "horizontal",
            "vertical",
            "nodes",
        )
    )


def _normalize_direction(value: Any) -> Optional[str]:
    text = str(value or "").lower().strip()
    if text in ("horizontal", "hsplit", "h", "row", "cols", "right", "left", "x"):
        return "horizontal"
    if text in ("vertical", "vsplit", "v", "column", "rows", "down", "up", "y"):
        return "vertical"
    if text in ("split",):
        return "horizontal"
    return None


def _from_tmux_named(raw: Dict[str, Any], axis: str) -> LayoutNode:
    kids = [parse_layout(item) for item in raw.get(axis) or []]
    kids = [k for k in kids if k is not None]
    return _nary_split(axis, kids, raw)


def _nary_split(
    direction: str,
    children: List[LayoutNode],
    raw: Optional[Dict[str, Any]] = None,
) -> LayoutNode:
    if len(children) == 1:
        return children[0]
    rect = rect_from_mapping(raw) if raw else None
    if rect is None and children:
        rect = children[0].rect
        for child in children[1:]:
            rect = rect.union(child.rect)
    kind = direction if direction in ("horizontal", "vertical") else "horizontal"
    return LayoutNode(kind=kind, children=list(children), rect=rect or LayoutRect())


def _bsp(items: List[Tuple[str, LayoutRect]]) -> LayoutNode:
    if len(items) == 1:
        pane_id, rect = items[0]
        return LayoutNode(kind="pane", pane_id=pane_id, rect=rect)
    bound = items[0][1]
    for _, rect in items[1:]:
        bound = bound.union(rect)
    partitioned = _partition_axis(items, "x") or _partition_axis(items, "y")
    if partitioned is None:
        # Degraded: stack remaining panes top-to-bottom in id order so the
        # mirror still creates one split per pane instead of dropping them.
        items = sorted(items, key=lambda pair: pair[0])
        kids = [
            LayoutNode(kind="pane", pane_id=pid, rect=rect) for pid, rect in items
        ]
        return LayoutNode(kind="vertical", children=kids, rect=bound)
    left, right, axis = partitioned
    kind = "horizontal" if axis == "x" else "vertical"
    return LayoutNode(
        kind=kind,
        children=[_bsp(left), _bsp(right)],
        rect=bound,
    )


def _partition_axis(
    items: List[Tuple[str, LayoutRect]],
    axis: str,
) -> Optional[Tuple[List[Tuple[str, LayoutRect]], List[Tuple[str, LayoutRect]], str]]:
    def start(rect: LayoutRect) -> int:
        return rect.x if axis == "x" else rect.y

    def end(rect: LayoutRect) -> int:
        return (rect.x + rect.width) if axis == "x" else (rect.y + rect.height)

    ordered = sorted(items, key=lambda pair: (start(pair[1]), pair[0]))
    for cut in range(1, len(ordered)):
        left = ordered[:cut]
        right = ordered[cut:]
        left_end = max(end(rect) for _, rect in left)
        right_start = min(start(rect) for _, rect in right)
        # Allow a 1-cell separator like tmux pane borders.
        if left_end <= right_start + 1:
            return left, right, axis
    return None


def pane_rects_from_objects(
    panes: Iterable[Any],
) -> List[Tuple[str, LayoutRect]]:
    """Pull ``(pane_id, rect)`` pairs from Pane-like objects or raw dicts."""
    out: List[Tuple[str, LayoutRect]] = []
    for pane in panes:
        if isinstance(pane, dict):
            pane_id = str(pane.get("pane_id") or "")
            raw = pane
        else:
            pane_id = str(getattr(pane, "pane_id", "") or "")
            raw = getattr(pane, "raw", None)
            if not isinstance(raw, dict):
                raw = {}
        rect = rect_from_mapping(raw)
        if pane_id and rect:
            out.append((pane_id, rect))
    return out
