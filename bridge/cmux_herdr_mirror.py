#!/usr/bin/env python3
"""Userspace deep mirror: project Herdr tabs/panes into real cmux tabs/splits.

This is the plugin analogue of cmux ``ssh-tmux`` / ``RemoteTmuxWindowMirror``:

- each Herdr tab becomes a cmux tab (first pane is the tab root)
- remaining Herdr panes in that tab become cmux splits
- each mirrored surface runs ``cmux-herdr attach-pane`` which follows
  ``herdr pane read`` and forwards typed input via ``herdr pane send``

It cannot steal Herdr PTYs into Ghostty (that needs native cmux). It *can*
create extra cmux viewers of the live Herdr session, keyed so reconcile is
idempotent (same input → same surfaces, no duplicate tabs on re-run).
"""

from __future__ import annotations

import json
import os
import select
import subprocess
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
        _save_association_map,
        cmux_cmd,
        collect_host_fingerprint,
        fetch_snapshot,
        herdr_json,
        resolve_cmux_workspace,
        run_cmd,
        sync_to_cmux,
        which,
    )
except ImportError:  # running as a loose file with PYTHONPATH=bridge
    from cmux_herdr_bridge import (
        BridgeError,
        Pane,
        Snapshot,
        Tab,
        _load_association_map,
        _save_association_map,
        cmux_cmd,
        collect_host_fingerprint,
        fetch_snapshot,
        herdr_json,
        resolve_cmux_workspace,
        run_cmd,
        sync_to_cmux,
        which,
    )

ATTACH_ENV = "CMUX_HERDR_ATTACH_PANE"
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

    @property
    def key(self) -> str:
        return f"{MIRROR_KEY_PREFIX}{self.pane_id}"


@dataclass
class MirrorAction:
    """One reconcile step. ``op`` is create_tab, create_split, rename, keep, prune."""

    op: str
    pane_id: str
    title: str
    tab_id: str = ""
    role: str = "tab-root"
    split_direction: str = "right"
    key: str = ""
    surface_id: Optional[str] = None
    split_from_surface_id: Optional[str] = None
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


def desired_mirrors(
    snapshot: Snapshot,
    *,
    scope: str = "current-tab",
    current_tab_id: Optional[str] = None,
    current_workspace_id: Optional[str] = None,
) -> List[DesiredMirror]:
    """Build the desired cmux projection from a Herdr snapshot.

    ``scope``:
    - ``current-tab`` — only the invoking Herdr tab (safe default)
    - ``workspace`` — every tab in the current Herdr workspace
    - ``all`` — every pane in the snapshot (ssh-tmux-style full session)
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

    desired: List[DesiredMirror] = []
    for tab_id in sorted(grouped, key=tab_sort_key):
        members = sorted(
            grouped[tab_id],
            key=lambda p: (not p.focused, p.pane_id),
        )
        tab = tabs_by_id.get(tab_id)
        for index, pane in enumerate(members):
            role = "tab-root" if index == 0 else "split"
            desired.append(
                DesiredMirror(
                    pane_id=pane.pane_id,
                    tab_id=tab_id,
                    workspace_id=pane.workspace_id,
                    title=_pane_title(pane, tab, role=role)[:80],
                    role=role,
                    split_direction=_split_direction_for_index(index),
                    agent=pane.agent,
                    agent_status=pane.agent_status or "unknown",
                )
            )
    return desired


def plan_mirror(
    desired: Sequence[DesiredMirror],
    existing: Dict[str, Any],
    *,
    live_surface_ids: Optional[Set[str]] = None,
    prune: bool = False,
) -> MirrorPlan:
    """Diff desired Herdr panes against the persisted mirror map.

    Idempotent: a second call with the same desired set and live surfaces
    yields only ``keep`` actions (plus ``rename`` when a title changed).
    Missing/dead mapped surfaces are recreated. Extra mapped panes are
    pruned only when ``prune`` is true.
    """
    existing_mirrors = existing if isinstance(existing, dict) else {}
    desired_ids = {item.pane_id for item in desired}
    tab_root_surface: Dict[str, str] = {}
    actions: List[MirrorAction] = []

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

    for item in desired:
        surface = mapped_surface(item.pane_id)
        entry = existing_mirrors.get(item.pane_id)
        prior_title = ""
        if isinstance(entry, dict):
            prior_title = str(entry.get("title") or "")
        if item.role == "tab-root" and surface:
            tab_root_surface[item.tab_id] = surface
        if surface:
            if item.role == "tab-root":
                tab_root_surface[item.tab_id] = surface
            if prior_title and prior_title != item.title:
                actions.append(
                    MirrorAction(
                        op="rename",
                        pane_id=item.pane_id,
                        title=item.title,
                        tab_id=item.tab_id,
                        role=item.role,
                        key=item.key,
                        surface_id=surface,
                        reason="title changed",
                    )
                )
            else:
                actions.append(
                    MirrorAction(
                        op="keep",
                        pane_id=item.pane_id,
                        title=item.title,
                        tab_id=item.tab_id,
                        role=item.role,
                        key=item.key,
                        surface_id=surface,
                    )
                )
            continue

        if item.role == "tab-root":
            actions.append(
                MirrorAction(
                    op="create_tab",
                    pane_id=item.pane_id,
                    title=item.title,
                    tab_id=item.tab_id,
                    role=item.role,
                    key=item.key,
                    reason="missing tab-root surface",
                )
            )
        else:
            actions.append(
                MirrorAction(
                    op="create_split",
                    pane_id=item.pane_id,
                    title=item.title,
                    tab_id=item.tab_id,
                    role=item.role,
                    split_direction=item.split_direction,
                    key=item.key,
                    split_from_surface_id=tab_root_surface.get(item.tab_id),
                    reason="missing split surface",
                )
            )

    if prune:
        for pane_id, entry in sorted(existing_mirrors.items()):
            if pane_id in desired_ids:
                continue
            if not isinstance(entry, dict):
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
) -> Dict[str, Any]:
    """Execute a plan against cmux. Safe to re-run: keeps the map in sync."""
    mirrors = {key: dict(value) for key, value in existing.items() if isinstance(value, dict)}
    created: List[str] = []
    renamed: List[str] = []
    pruned: List[str] = []
    kept: List[str] = []
    errors: List[str] = []
    tab_root_surface: Dict[str, str] = {}

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
            elif action.op in ("create_tab", "create_split"):
                command = " ".join(_attach_argv(action.pane_id))
                created_info: Dict[str, Any]
                if action.op == "create_split":
                    split_from = action.split_from_surface_id or tab_root_surface.get(
                        action.tab_id
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
                        except BridgeError:
                            created_info = _create_terminal(
                                key=action.key,
                                name=action.title,
                                command=command,
                                workspace=workspace,
                            )
                    else:
                        created_info = _create_terminal(
                            key=action.key,
                            name=action.title,
                            command=command,
                            workspace=workspace,
                        )
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
                    "updated_at": time.time(),
                }
                created.append(action.pane_id)
        except BridgeError as exc:
            errors.append(f"{action.op} {action.pane_id}: {exc}")

    if not dry_run:
        save_mirrors(mirrors, cmux_workspace=workspace)
        if log:
            summary = (
                f"herdr mirror: created={len(created)} renamed={len(renamed)} "
                f"kept={len(kept)} pruned={len(pruned)} errors={len(errors)}"
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
) -> Dict[str, Any]:
    """Reconcile Herdr topology into cmux tabs/splits, then refresh status pills."""
    if is_attach_process():
        raise BridgeError(
            "refusing to nest mirror inside attach-pane "
            f"({ATTACH_ENV}={os.environ.get(ATTACH_ENV)})"
        )
    snap = fetch_snapshot()
    desired = desired_mirrors(
        snap,
        scope=scope,
        current_tab_id=tab,
        current_workspace_id=herdr_workspace,
    )
    ws = workspace
    if not ws and not dry_run:
        ws = resolve_cmux_workspace()
    existing = load_mirrors()
    live_ids = None if dry_run else list_live_surface_ids(workspace=ws)
    plan = plan_mirror(desired, existing, live_surface_ids=live_ids, prune=prune)
    plan.scope = scope
    applied = apply_mirror_plan(
        plan,
        existing=existing,
        workspace=ws,
        dry_run=dry_run,
        log=log,
    )
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
        "plan": applied,
        "status_sync": status_summary,
        "host_fingerprint": collect_host_fingerprint(),
    }


def format_mirror_plan(result: Dict[str, Any]) -> str:
    """Human-readable mirror reconcile summary."""
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
    ]
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
    if not which("herdr"):
        raise BridgeError("herdr not found on PATH")
    flag_attempts = (
        ["herdr", "pane", "send", pane_id, "--text", text],
        ["herdr", "pane", "send-keys", pane_id, text],
    )
    last_error = ""
    for args in flag_attempts:
        proc = run_cmd(args, timeout=5.0)
        if proc.returncode == 0:
            return
        last_error = (proc.stderr or proc.stdout or str(proc.returncode)).strip()
    stdin_args = ["herdr", "pane", "send", pane_id]
    proc = subprocess.run(
        stdin_args,
        input=text,
        capture_output=True,
        text=True,
        timeout=5.0,
    )
    if proc.returncode == 0:
        return
    last_error = (proc.stderr or proc.stdout or last_error or str(proc.returncode)).strip()
    raise BridgeError(last_error or f"herdr pane send failed for {pane_id}")


def read_pane_text(pane_id: str, *, lines: int = 200) -> str:
    """Read current Herdr pane contents for the attach follower."""
    args = ["pane", "read", pane_id, "--source", "recent-unwrapped", "--lines", str(lines)]
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
    except BridgeError:
        proc = run_cmd(
            ["herdr", "pane", "read", pane_id, "--source", "recent-unwrapped", "--lines", str(lines)],
            timeout=8.0,
        )
        if proc.returncode != 0:
            raise BridgeError(
                (proc.stderr or proc.stdout or f"pane read failed for {pane_id}").strip()
            )
        return proc.stdout or ""


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
) -> int:
    """Follow a Herdr pane in this terminal (plugin stand-in for a tmux PTY feed).

    ``max_iterations`` is for tests. Production attach runs until the pane
    disappears or the user hits Ctrl-C.
    """
    out = stdout or sys.stdout
    os.environ[ATTACH_ENV] = pane_id
    last = None
    iteration = 0
    header = (
        f"cmux-herdr attach-pane {pane_id}  (Ctrl-C to detach this viewer; "
        "Herdr pane stays alive)\n"
    )
    while True:
        iteration += 1
        try:
            text = read_once() if read_once else read_pane_text(pane_id, lines=lines)
        except BridgeError as exc:
            out.write(f"\ncmux-herdr: pane {pane_id} gone ({exc})\n")
            out.flush()
            return 1
        if text != last:
            out.write("\033[H\033[2J")
            out.write(header)
            out.write(text)
            if not text.endswith("\n"):
                out.write("\n")
            out.flush()
            last = text
        if send_input:
            _drain_stdin_to_pane(pane_id)
        if max_iterations is not None and iteration >= max_iterations:
            return 0
        sleeper(max(0.05, interval))
        _ = clock()


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
