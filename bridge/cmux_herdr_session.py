#!/usr/bin/env python3
"""Session-tab host verbs (tmux ``RemoteTmuxSessionMirror.rebuildTopology``).

``reconcile_session`` decides *which* Herdr tabs exist and in what order.
This module linearizes that diff into the **ordered verbs** a host must run:

1. ``create_tab`` for new Herdr tabs (desired order — tmux ``windowOrder``)
2. ``rename_tab`` when a kept tab's title changed
3. ``close_tab`` for gone Herdr tabs (teardown the window mirror)
4. ``close_default_tabs`` once the first real mirror tab exists
5. ``reorder_tabs`` when kept-tab order drifted (count > 1)
6. ``focus_tab`` after the topology transaction (explicit request only)

Native ``RemoteHerdrSessionApply`` is the Swift twin. Neither path owns
AppKit; ``FakeSessionHost`` proves the order without TabManager.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Dict, List, Optional, Sequence, Tuple

try:
    from .cmux_herdr_engine import SessionReconcile
except ImportError:
    from cmux_herdr_engine import SessionReconcile


@dataclass(frozen=True)
class SessionAction:
    """One session-level host verb. Order in a list is load-bearing."""

    op: str
    tab_id: Optional[str] = None
    title: Optional[str] = None
    ordered_tab_ids: Optional[Tuple[str, ...]] = None


def session_actions(
    session: SessionReconcile,
    *,
    titles: Optional[Dict[str, str]] = None,
    previous_titles: Optional[Dict[str, str]] = None,
    defaults_open: bool = False,
    focus_tab_id: Optional[str] = None,
) -> List[SessionAction]:
    """Linearize one session reconcile into host verbs.

    Order copies ``RemoteTmuxSessionMirror.rebuildTopology``: create/refresh
    live windows first, close leftovers, drop the workspace default tab,
    then reorder the strip to Herdr tab numbers.

    Args:
        session: Diff from ``reconcile_session``.
        titles: Current tab titles keyed by Herdr tab id.
        previous_titles: Titles at the last apply (rename detection).
        defaults_open: True while the host still shows its default local tab.
        focus_tab_id: Optional tab to select after the transaction.

    Returns:
        Ordered verbs. Empty when the session is already in sync and no
        defaults/focus work remains.
    """
    current_titles = titles or {}
    prior_titles = previous_titles or {}
    actions: List[SessionAction] = []

    for tab_id in session.created_tab_ids:
        actions.append(
            SessionAction(
                op="create_tab",
                tab_id=tab_id,
                title=current_titles.get(tab_id, tab_id),
            )
        )

    for tab_id in session.kept_tab_ids:
        new_title = current_titles.get(tab_id)
        old_title = prior_titles.get(tab_id)
        if new_title and new_title != old_title:
            actions.append(
                SessionAction(op="rename_tab", tab_id=tab_id, title=new_title)
            )

    for tab_id in session.closed_tab_ids:
        actions.append(SessionAction(op="close_tab", tab_id=tab_id))

    if defaults_open and session.ordered_tab_ids:
        actions.append(SessionAction(op="close_default_tabs"))

    if session.order_changed and len(session.ordered_tab_ids) > 1:
        actions.append(
            SessionAction(
                op="reorder_tabs",
                ordered_tab_ids=tuple(session.ordered_tab_ids),
            )
        )

    if focus_tab_id and focus_tab_id in session.ordered_tab_ids:
        actions.append(SessionAction(op="focus_tab", tab_id=focus_tab_id))

    return actions


@dataclass
class FakeSessionHost:
    """In-memory tab strip used to prove session apply without AppKit.

    Tabs are appended on create (tmux arrival order) and only match Herdr
    numbers after ``reorder_tabs``. Closing a tab tears down its mirror.
    """

    tabs: List[str] = field(default_factory=list)
    titles: Dict[str, str] = field(default_factory=dict)
    mirrors: Dict[str, bool] = field(default_factory=dict)
    default_tabs: List[str] = field(default_factory=lambda: ["default"])
    defaults_closed: bool = False
    focus: Optional[str] = None
    log: List[str] = field(default_factory=list)

    def apply(self, actions: Sequence[SessionAction]) -> None:
        """Apply verbs in order. Unknown ops raise — fail closed."""
        for action in actions:
            self._apply_one(action)

    def _apply_one(self, action: SessionAction) -> None:
        op = action.op
        if op == "create_tab":
            if not action.tab_id:
                raise ValueError("create_tab requires tab_id")
            if action.tab_id not in self.tabs:
                self.tabs.append(action.tab_id)
            self.titles[action.tab_id] = action.title or action.tab_id
            self.mirrors[action.tab_id] = True
            self.log.append(f"create:{action.tab_id}")
            return
        if op == "rename_tab":
            if action.tab_id and action.tab_id in self.tabs and action.title:
                self.titles[action.tab_id] = action.title
                self.log.append(f"rename:{action.tab_id}")
            return
        if op == "close_tab":
            if action.tab_id and action.tab_id in self.tabs:
                self.tabs.remove(action.tab_id)
                self.titles.pop(action.tab_id, None)
                self.mirrors.pop(action.tab_id, None)
                if self.focus == action.tab_id:
                    self.focus = None
                self.log.append(f"close:{action.tab_id}")
            return
        if op == "close_default_tabs":
            if self.defaults_closed or not self.tabs:
                return
            for tab_id in list(self.default_tabs):
                if tab_id in self.tabs:
                    continue
                self.log.append(f"close-default:{tab_id}")
            self.default_tabs = []
            self.defaults_closed = True
            return
        if op == "reorder_tabs":
            desired = [tab_id for tab_id in (action.ordered_tab_ids or ()) if tab_id in self.tabs]
            extras = [tab_id for tab_id in self.tabs if tab_id not in desired]
            self.tabs = desired + extras
            self.log.append("reorder:" + ",".join(self.tabs))
            return
        if op == "focus_tab":
            if action.tab_id and action.tab_id not in self.tabs:
                raise ValueError(f"focus_tab missing tab {action.tab_id}")
            self.focus = action.tab_id
            self.log.append(f"focus:{action.tab_id}")
            return
        raise ValueError(f"unknown session op {op}")
