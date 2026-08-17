#!/usr/bin/env python3
"""Host-apply verbs for one impose + reconcile pass (tmux AppKit seam).

``cmux_herdr_impose`` decides *what* the tree should look like.
This module linearizes that plan into the **ordered verbs** a host must
run — create panels first (tmux ``makePanel`` before ``rebuildBonsplitTree``),
then mutate the tree, then impose dividers (skipping a held drag split),
then focus.

The plugin interprets the same verbs via ``cmux split`` / ``set-ratio``.
Native ``RemoteHerdrHostApply`` is the Swift twin. Neither path owns
AppKit; a ``FakeBonsplitHost`` here proves the order without Ghostty.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Dict, List, Optional, Set

try:
    from .cmux_herdr_engine import ReconcileResult
    from .cmux_herdr_impose import (
        DividerLeaf,
        DividerNode,
        DividerSplit,
        ImposePlan,
        LeafExpansion,
        TreeAction,
    )
except ImportError:
    from cmux_herdr_engine import ReconcileResult
    from cmux_herdr_impose import (
        DividerLeaf,
        DividerNode,
        DividerSplit,
        ImposePlan,
        LeafExpansion,
        TreeAction,
    )


@dataclass(frozen=True)
class HostAction:
    """One host verb. Order in a list is load-bearing (tmux apply order)."""

    op: str
    pane_id: Optional[str] = None
    split_from_pane_id: Optional[str] = None
    orientation: Optional[str] = None
    fraction: Optional[float] = None
    first_extent: Optional[float] = None
    insert_first: bool = False
    surface_id: Optional[str] = None
    skip_split_key: Optional[str] = None
    split_key: Optional[str] = None


def _expansion(action: TreeAction) -> Optional[LeafExpansion]:
    return action.expansion


def divider_impose_actions(
    node: DividerNode,
    *,
    held_split_key: Optional[str] = None,
    key_prefix: str = "s",
) -> List[HostAction]:
    """Walk the binary tree and emit one ``impose_divider`` per split.

    A held split (in-flight ``pane.resize``) is skipped so re-impose cannot
    bounce the user's divider — same as tmux ``skippingSubtree``.
    """
    actions: List[HostAction] = []

    def walk(current: DividerNode, key: str) -> None:
        if isinstance(current, DividerLeaf):
            return
        if not isinstance(current, DividerSplit):
            return
        if held_split_key is None or key != held_split_key:
            actions.append(
                HostAction(
                    op="impose_divider",
                    orientation=current.orientation,
                    fraction=current.fraction,
                    first_extent=current.first_extent,
                    split_key=key,
                )
            )
        walk(current.first, f"{key}.0")
        walk(current.second, f"{key}.1")

    walk(node, key_prefix)
    return actions


def host_actions(
    result: ReconcileResult,
    plan: ImposePlan,
) -> List[HostAction]:
    """Linearize one reconcile + impose pass into host verbs.

    Order copies ``RemoteTmuxWindowMirror.apply``:

    1. ``create_panel`` for every new BASE pane (panels must exist before rebuild)
    2. ``close_panel`` for gone BASE panes only (zoom never emits this)
    3. tree mutate of the VISIBLE tree (zoom may ``remove_leaf`` without close)
    4. ``impose_divider`` for each binary split (skip held)
    5. ``focus`` the active pane
    """
    actions: List[HostAction] = []
    for pane_id in result.created_pane_ids:
        actions.append(HostAction(op="create_panel", pane_id=pane_id))
    # BASE lifecycle only. Zoom can ``remove_leaf`` from the VISIBLE tree
    # without closing the hidden panel (tmux base-vs-visible).
    for pane_id in result.closed_pane_ids:
        actions.append(HostAction(op="close_panel", pane_id=pane_id))

    kind = plan.tree_action.kind
    if kind == "rebuild":
        actions.append(HostAction(op="rebuild_tree"))
    elif kind == "keep":
        actions.append(HostAction(op="keep_tree"))
    elif kind == "expand_leaf":
        expansion = _expansion(plan.tree_action)
        if expansion is not None:
            actions.append(
                HostAction(
                    op="expand_leaf",
                    pane_id=expansion.new_pane_id,
                    split_from_pane_id=expansion.existing_pane_id,
                    orientation=expansion.orientation,
                    fraction=expansion.fraction,
                    insert_first=expansion.insert_first,
                )
            )
        else:
            actions.append(HostAction(op="rebuild_tree"))
    elif kind == "remove_leaf":
        actions.append(
            HostAction(op="remove_leaf", pane_id=plan.tree_action.removed_pane_id)
        )
    else:
        actions.append(HostAction(op="rebuild_tree"))

    actions.extend(
        divider_impose_actions(
            plan.divider_tree, held_split_key=plan.held_split_key
        )
    )
    if plan.focus_pane_id:
        actions.append(HostAction(op="focus", pane_id=plan.focus_pane_id))
    return actions


@dataclass
class FakeBonsplitHost:
    """In-memory host used to prove apply order without AppKit/Ghostty.

    Surfaces are created before the tree is rebuilt. Zoom-hidden panes stay
    in ``panels`` even when they are absent from the visible divider tree.
    """

    panels: Set[str] = field(default_factory=set)
    surfaces: Dict[str, str] = field(default_factory=dict)
    focus: Optional[str] = None
    last_tree_op: Optional[str] = None
    imposed: List[HostAction] = field(default_factory=list)
    log: List[str] = field(default_factory=list)

    def apply(self, actions: List[HostAction]) -> None:
        """Apply verbs in order. Unknown ops raise — fail closed."""
        for action in actions:
            self._apply_one(action)

    def _apply_one(self, action: HostAction) -> None:
        op = action.op
        if op == "create_panel":
            if not action.pane_id:
                raise ValueError("create_panel requires pane_id")
            self.panels.add(action.pane_id)
            self.log.append(f"create:{action.pane_id}")
            return
        if op == "close_panel":
            if action.pane_id:
                self.panels.discard(action.pane_id)
                self.surfaces.pop(action.pane_id, None)
                self.log.append(f"close:{action.pane_id}")
            return
        if op == "bind_surface":
            if action.pane_id and action.surface_id:
                if action.pane_id not in self.panels:
                    raise ValueError(
                        f"bind_surface before create_panel: {action.pane_id}"
                    )
                self.surfaces[action.pane_id] = action.surface_id
            return
        if op in ("rebuild_tree", "keep_tree", "expand_leaf", "remove_leaf"):
            missing = [pane for pane in self._required_panes(action) if pane not in self.panels]
            if missing:
                raise ValueError(f"{op} missing panels {missing}")
            self.last_tree_op = op
            self.log.append(op)
            return
        if op == "impose_divider":
            self.imposed.append(action)
            self.log.append(f"impose:{action.split_key}")
            return
        if op == "focus":
            if action.pane_id and action.pane_id not in self.panels:
                raise ValueError(f"focus missing panel {action.pane_id}")
            self.focus = action.pane_id
            self.log.append(f"focus:{action.pane_id}")
            return
        raise ValueError(f"unknown host op {op}")

    def _required_panes(self, action: HostAction) -> List[str]:
        if action.op == "expand_leaf":
            return [p for p in (action.split_from_pane_id, action.pane_id) if p]
        return []
