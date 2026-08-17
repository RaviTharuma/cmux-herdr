#!/usr/bin/env python3
"""Shared plugin ↔ native writer lease and restore handoff.

The plugin path (this repo) and the native cmux path (``RemoteHerdr*``)
are one system. They share:

- the same lease files (who may project onto cmux chrome)
- the same restore record (``mode: reattach`` only)
- the same setting key / socket methods (owned by lifecycle)

They do **not** invent tmux transport Herdr does not have (SSH,
ControlMaster, ``tmux -CC``, ``%layout-change``, ``respawn-pane``,
``kill-server``). Host close always detaches.

A dead owner does not hold the lock: a lease is stale when its pid is
dead, or when a legacy marker has no heartbeat within the TTL.
"""

from __future__ import annotations

import json
import os
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Sequence, Tuple

SCHEMA = 1
OWNER_PLUGIN = "plugin"
OWNER_NATIVE = "native"
OWNERS = frozenset({OWNER_PLUGIN, OWNER_NATIVE})

NATIVE_LIVE_ENV = "CMUX_HERDR_NATIVE_LIVE"
FORCE_PLUGIN_ENV = "CMUX_HERDR_FORCE_PLUGIN"
LEASE_TTL_ENV = "CMUX_HERDR_LEASE_TTL_MS"
NATIVE_STATE_ENV = "CMUX_HERDR_NATIVE_STATE_DIR"

DEFAULT_TTL_MS = 45_000
OUTCOME_NATIVE_OWNS = "native_owns"
OUTCOME_PLUGIN_OWNS = "plugin_owns"

_TRUTHY = frozenset({"1", "true", "yes", "on"})


def env_truthy(name: str) -> bool:
    """Return True when *name* is a conventional truthy env flag."""
    raw = (os.environ.get(name) or "").strip().lower()
    return raw in _TRUTHY


def now_ms() -> int:
    """Current Unix time in milliseconds."""
    return int(time.time() * 1000)


def lease_ttl_ms() -> int:
    """Freshness window for a heartbeat without a live pid."""
    raw = (os.environ.get(LEASE_TTL_ENV) or "").strip()
    if raw.isdigit():
        return max(1_000, int(raw))
    return DEFAULT_TTL_MS


def pid_alive(pid: int) -> bool:
    """True when *pid* still exists. PermissionError means it exists."""
    if pid <= 0:
        return False
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    except OSError:
        return False
    return True


def xdg_state_dir() -> Path:
    """Plugin / XDG state root (``$XDG_STATE_HOME/cmux-herdr``)."""
    root = os.environ.get("XDG_STATE_HOME") or os.path.expanduser("~/.local/state")
    return Path(root) / "cmux-herdr"


def application_support_dir() -> Optional[Path]:
    """macOS native state root, or an explicit override.

    Native AppKit historically wrote ``~/Library/Application Support/cmux-herdr``.
    The plugin reads that directory when it exists so a live native lease is
    visible on the same machine. Tests set ``CMUX_HERDR_NATIVE_STATE_DIR``.
    """
    override = (os.environ.get(NATIVE_STATE_ENV) or "").strip()
    if override:
        return Path(override)
    home = os.environ.get("HOME") or os.path.expanduser("~")
    mac = Path(home) / "Library" / "Application Support" / "cmux-herdr"
    if mac.is_dir():
        return mac
    return None


def state_dirs() -> List[Path]:
    """Directories both paths read; XDG is always first (plugin canonical)."""
    dirs: List[Path] = [xdg_state_dir()]
    extra = application_support_dir()
    if extra is not None and extra.resolve() != dirs[0].resolve():
        dirs.append(extra)
    return dirs


@dataclass(frozen=True)
class WriterLease:
    """One writer's claim on a host fingerprint."""

    owner: str
    pid: int
    heartbeat_ms: int
    fingerprint: str
    endpoint_hash: str = ""
    socket_path: str = ""
    schema: int = SCHEMA
    path: str = ""

    def to_dict(self) -> Dict[str, Any]:
        """JSON payload both paths parse. No secrets."""
        return {
            "schema": self.schema,
            "owner": self.owner,
            "pid": self.pid,
            "heartbeat_ms": self.heartbeat_ms,
            "fingerprint": self.fingerprint,
            "endpoint_hash": self.endpoint_hash,
            "socket_path": self.socket_path,
        }

    def is_fresh(self, *, now: Optional[int] = None, ttl: Optional[int] = None) -> bool:
        """Dead pid is immediately stale; legacy markers use heartbeat TTL."""
        clock = now if now is not None else now_ms()
        window = ttl if ttl is not None else lease_ttl_ms()
        if self.pid > 0:
            if not pid_alive(self.pid):
                return False
            return (clock - self.heartbeat_ms) <= window
        return (clock - self.heartbeat_ms) <= window


def _mtime_ms(path: Path) -> int:
    try:
        return int(path.stat().st_mtime * 1000)
    except OSError:
        return 0


def parse_lease_text(
    text: str,
    *,
    path: Path,
    fallback_owner: Optional[str] = None,
    fallback_fingerprint: str = "",
) -> Optional[WriterLease]:
    """Parse a lease file. JSON is canonical; ``1`` / ``live`` is legacy native."""
    stripped = text.strip()
    if not stripped:
        return None
    try:
        payload = json.loads(stripped)
    except json.JSONDecodeError:
        payload = None
    if isinstance(payload, dict):
        owner = str(payload.get("owner") or "")
        if owner not in OWNERS:
            return None
        try:
            pid = int(payload.get("pid") or 0)
        except (TypeError, ValueError):
            pid = 0
        try:
            heartbeat = int(payload.get("heartbeat_ms") or 0)
        except (TypeError, ValueError):
            heartbeat = 0
        if heartbeat <= 0:
            heartbeat = _mtime_ms(path)
        try:
            schema = int(payload.get("schema") or SCHEMA)
        except (TypeError, ValueError):
            schema = SCHEMA
        return WriterLease(
            owner=owner,
            pid=pid,
            heartbeat_ms=heartbeat,
            fingerprint=str(payload.get("fingerprint") or fallback_fingerprint),
            endpoint_hash=str(payload.get("endpoint_hash") or ""),
            socket_path=str(payload.get("socket_path") or ""),
            schema=schema,
            path=str(path),
        )
    if fallback_owner in OWNERS and stripped.lower() in {"1", "live", "yes", "on", "true"}:
        return WriterLease(
            owner=fallback_owner,
            pid=0,
            heartbeat_ms=_mtime_ms(path),
            fingerprint=fallback_fingerprint,
            path=str(path),
            schema=0,
        )
    return None


def read_lease_file(path: Path, *, fingerprint: str = "") -> Optional[WriterLease]:
    """Read one lease file. Missing / corrupt files are ignored."""
    if not path.is_file():
        return None
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return None
    name = path.name
    fallback = None
    if name.startswith("native-live"):
        fallback = OWNER_NATIVE
    elif name.startswith("plugin-live"):
        fallback = OWNER_PLUGIN
    return parse_lease_text(
        text,
        path=path,
        fallback_owner=fallback,
        fallback_fingerprint=fingerprint,
    )


def writer_paths(fingerprint: str, owner: str) -> List[Path]:
    """All files one owner writes so the other path can find the lease."""
    if owner not in OWNERS:
        raise ValueError(f"unknown handoff owner: {owner}")
    paths: List[Path] = []
    for root in state_dirs():
        paths.append(root / f"writer-{fingerprint}.json")
        paths.append(root / f"{owner}-live-{fingerprint}")
        paths.append(root / f"{owner}-live")
    return paths


def legacy_native_marker_path(fingerprint: str) -> Path:
    """XDG ``native-live-<fingerprint>`` (plugin v0.2 readers)."""
    return xdg_state_dir() / f"native-live-{fingerprint}"


def plugin_marker_path(fingerprint: str) -> Path:
    """XDG ``plugin-live-<fingerprint>``."""
    return xdg_state_dir() / f"plugin-live-{fingerprint}"


def _candidate_paths(fingerprint: str) -> List[Path]:
    """Every file either path might have written for this host."""
    paths: List[Path] = []
    seen = set()
    for root in state_dirs():
        for name in (
            f"writer-{fingerprint}.json",
            f"native-live-{fingerprint}",
            f"plugin-live-{fingerprint}",
            "native-live",
            "plugin-live",
            "writer.json",
        ):
            path = root / name
            key = str(path)
            if key in seen:
                continue
            seen.add(key)
            paths.append(path)
    return paths


def load_leases(fingerprint: str) -> List[WriterLease]:
    """Load every readable lease for *fingerprint* (fresh and stale)."""
    found: List[WriterLease] = []
    for path in _candidate_paths(fingerprint):
        lease = read_lease_file(path, fingerprint=fingerprint)
        if lease is not None:
            found.append(lease)
    return found


def pick_lease(
    leases: Sequence[WriterLease],
    *,
    now: Optional[int] = None,
) -> Tuple[Optional[WriterLease], List[WriterLease]]:
    """Return the freshest live lease and the stale ones.

    Native wins a tie: it owns the real Ghostty surfaces.
    """
    clock = now if now is not None else now_ms()
    fresh = [item for item in leases if item.is_fresh(now=clock)]
    stale = [item for item in leases if not item.is_fresh(now=clock)]
    if not fresh:
        return None, stale
    fresh = sorted(
        fresh,
        key=lambda item: (item.owner == OWNER_NATIVE, item.heartbeat_ms),
        reverse=True,
    )
    return fresh[0], stale


@dataclass(frozen=True)
class WriterDecision:
    """Resolved single-writer state for one host."""

    writer: str
    owner: Optional[str]
    native_live: bool
    plugin_live: bool
    native_detected: bool
    plugin_detected: bool
    force_plugin: bool
    env_native_live: bool
    lease_stale: bool
    lease: Optional[WriterLease]
    fingerprint: str

    @property
    def yields(self) -> bool:
        """True when this process must not start a competing apply host."""
        if self.force_plugin:
            return False
        return self.native_live

    @property
    def outcome(self) -> str:
        """CLI / observe token when this process is not the writer."""
        if self.native_live:
            return OUTCOME_NATIVE_OWNS
        if self.plugin_live:
            return OUTCOME_PLUGIN_OWNS
        return "unclaimed"

    def payload(self, *, action: str, method: Optional[str] = None) -> Dict[str, Any]:
        """Machine-readable yield / status blob. Never claims server.stop."""
        body: Dict[str, Any] = {
            "ok": True,
            "outcome": self.outcome if self.yields or self.plugin_live else "unclaimed",
            "writer": self.writer,
            "action": action,
            "server_stopped": False,
            "competing": False,
            "native_live": self.native_live,
            "plugin_live": self.plugin_live,
            "lease_stale": self.lease_stale,
            "fingerprint": self.fingerprint,
        }
        if method:
            body["method"] = method
        if self.lease is not None:
            body["lease"] = self.lease.to_dict()
        return body


def resolve_writer(
    fingerprint: str,
    *,
    our_pid: Optional[int] = None,
    now: Optional[int] = None,
) -> WriterDecision:
    """Decide who may project for *fingerprint*.

    ``CMUX_HERDR_NATIVE_LIVE=1`` is an explicit native claim (no pid).
    ``CMUX_HERDR_FORCE_PLUGIN=1`` lets the plugin dogfood over a live native.
    """
    force = env_truthy(FORCE_PLUGIN_ENV)
    env_native = env_truthy(NATIVE_LIVE_ENV)
    leases = load_leases(fingerprint)
    live, stale = pick_lease(leases, now=now)
    native_file = any(item.owner == OWNER_NATIVE for item in leases)
    plugin_file = any(item.owner == OWNER_PLUGIN for item in leases)

    owner: Optional[str] = None
    if env_native and not force:
        owner = OWNER_NATIVE
    elif live is not None:
        owner = live.owner

    ours = our_pid if our_pid is not None else os.getpid()
    plugin_is_us = (
        live is not None
        and live.owner == OWNER_PLUGIN
        and live.pid == ours
    )
    plugin_is_other = (
        live is not None
        and live.owner == OWNER_PLUGIN
        and live.pid != ours
        and live.pid > 0
    )

    native_live = owner == OWNER_NATIVE and not force
    plugin_live = owner == OWNER_PLUGIN and not native_live

    if force and (env_native or (live is not None and live.owner == OWNER_NATIVE)):
        writer = "plugin-forced"
    elif native_live:
        writer = OWNER_NATIVE
    else:
        writer = OWNER_PLUGIN

    return WriterDecision(
        writer=writer,
        owner=owner if not force else OWNER_PLUGIN,
        native_live=native_live,
        plugin_live=plugin_live,
        native_detected=env_native or native_file,
        plugin_detected=plugin_file,
        force_plugin=force,
        env_native_live=env_native,
        lease_stale=live is None and bool(stale),
        lease=live,
        fingerprint=fingerprint,
    )


def _atomic_write(path: Path, payload: Dict[str, Any]) -> None:
    """Write JSON replace-in-place so readers never see a partial lease."""
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    tmp.replace(path)


def _unlink(path: Path) -> bool:
    try:
        path.unlink()
        return True
    except FileNotFoundError:
        return False
    except OSError:
        return False


def write_lease(
    owner: str,
    fingerprint: str,
    *,
    socket_path: str = "",
    endpoint_hash: str = "",
    pid: Optional[int] = None,
    heartbeat: Optional[int] = None,
) -> WriterLease:
    """Claim *owner* on every shared path (XDG + native state dir)."""
    lease = WriterLease(
        owner=owner,
        pid=os.getpid() if pid is None else pid,
        heartbeat_ms=now_ms() if heartbeat is None else heartbeat,
        fingerprint=fingerprint,
        endpoint_hash=endpoint_hash,
        socket_path=socket_path,
    )
    payload = lease.to_dict()
    last = ""
    for path in writer_paths(fingerprint, owner):
        _atomic_write(path, payload)
        last = str(path)
    return WriterLease(**{**lease.__dict__, "path": last})


def clear_owner(owner: str, fingerprint: str) -> None:
    """Drop one owner's files. The other owner's lease is left intact."""
    for path in writer_paths(fingerprint, owner):
        _unlink(path)


def claim_plugin_writer(
    fingerprint: str,
    *,
    socket_path: str = "",
    endpoint_hash: str = "",
) -> Optional[WriterLease]:
    """Plugin attach / watch owns the host. No-op when native is live."""
    decision = resolve_writer(fingerprint)
    if decision.yields:
        return None
    if decision.force_plugin:
        clear_owner(OWNER_NATIVE, fingerprint)
    return write_lease(
        OWNER_PLUGIN,
        fingerprint,
        socket_path=socket_path,
        endpoint_hash=endpoint_hash,
    )


def release_plugin_writer(fingerprint: str) -> None:
    """Plugin detach / watch stop. Never stops Herdr."""
    clear_owner(OWNER_PLUGIN, fingerprint)


def heartbeat_plugin_writer(
    fingerprint: str,
    *,
    socket_path: str = "",
    endpoint_hash: str = "",
) -> Optional[WriterLease]:
    """Refresh the plugin lease so a live watch is not treated as stale."""
    decision = resolve_writer(fingerprint, our_pid=os.getpid())
    if decision.yields:
        return None
    if decision.lease is None or decision.lease.owner != OWNER_PLUGIN:
        return claim_plugin_writer(
            fingerprint,
            socket_path=socket_path,
            endpoint_hash=endpoint_hash,
        )
    return write_lease(
        OWNER_PLUGIN,
        fingerprint,
        socket_path=socket_path or decision.lease.socket_path,
        endpoint_hash=endpoint_hash or decision.lease.endpoint_hash,
    )


def claim_native_writer(
    fingerprint: str,
    *,
    socket_path: str = "",
    endpoint_hash: str = "",
    pid: Optional[int] = None,
) -> Optional[WriterLease]:
    """Native AppKit / dogfood helper claims the host.

    Yields when a *other* plugin process already holds a fresh lease,
    unless ``CMUX_HERDR_NATIVE_LIVE`` is set (explicit native takeover).
    """
    decision = resolve_writer(fingerprint)
    if (
        decision.plugin_live
        and decision.lease is not None
        and decision.lease.pid != (pid if pid is not None else os.getpid())
        and not env_truthy(NATIVE_LIVE_ENV)
        and not decision.force_plugin
    ):
        return None
    clear_owner(OWNER_PLUGIN, fingerprint)
    return write_lease(
        OWNER_NATIVE,
        fingerprint,
        socket_path=socket_path,
        endpoint_hash=endpoint_hash,
        pid=pid,
    )


def release_native_writer(fingerprint: str) -> None:
    """Native detach. Leaves the Herdr session running."""
    clear_owner(OWNER_NATIVE, fingerprint)


def writer_status(fingerprint: str) -> Dict[str, Any]:
    """Describe which path may project ``herdr:*`` pills / live apply."""
    decision = resolve_writer(fingerprint)
    marker = legacy_native_marker_path(fingerprint)
    plugin = plugin_marker_path(fingerprint)
    global_native = [root / "native-live" for root in state_dirs()]
    return {
        "writer": decision.writer,
        "native_live": decision.native_live,
        "plugin_live": decision.plugin_live,
        "native_detected": decision.native_detected,
        "plugin_detected": decision.plugin_detected,
        "force_plugin": decision.force_plugin,
        "env_native_live": decision.env_native_live,
        "lease_stale": decision.lease_stale,
        "lease": decision.lease.to_dict() if decision.lease else None,
        "marker_path": str(marker),
        "marker_exists": marker.is_file(),
        "plugin_marker_path": str(plugin),
        "plugin_marker_exists": plugin.is_file(),
        "global_marker_exists": any(path.is_file() for path in global_native),
        "fingerprint": fingerprint,
    }


def restore_paths(endpoint_hash: str) -> List[Path]:
    """Shared restore files (``restore-<endpointHash>.json``) in every state dir."""
    return [root / f"restore-{endpoint_hash}.json" for root in state_dirs()]


def write_shared_restore(endpoint_hash: str, payload: Dict[str, Any]) -> str:
    """Persist the last attach so either path can reattach after restart."""
    if payload.get("mode") == "replay_tree":
        raise ValueError("restore payload must not use mode=replay_tree")
    body = dict(payload)
    body.setdefault("mode", "reattach")
    last = ""
    for path in restore_paths(endpoint_hash):
        _atomic_write(path, body)
        last = str(path)
    return last


def read_shared_restore(endpoint_hash: str) -> Optional[Dict[str, Any]]:
    """Read the first valid restore file. ``replay_tree`` is rejected."""
    for path in restore_paths(endpoint_hash):
        if not path.is_file():
            continue
        try:
            payload = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        if not isinstance(payload, dict):
            continue
        if payload.get("mode") == "replay_tree":
            continue
        return payload
    return None


def clear_shared_restore(endpoint_hash: str) -> bool:
    """Drop restore files after an explicit detach."""
    removed = False
    for path in restore_paths(endpoint_hash):
        if _unlink(path):
            removed = True
    return removed


def observe_foreign(decision: WriterDecision, method: str) -> Dict[str, Any]:
    """Observability when the other path owns the live surfaces.

    We do not invent Ghostty pane grids we cannot see. Herdr extras
    such as ``agent_status`` stay on the owner.
    """
    body = decision.payload(action="observe", method=method)
    body["mirrored"] = True
    body["panes"] = []
    body["windows"] = []
    return body


def iter_stale_leases(fingerprint: str) -> Iterable[WriterLease]:
    """Stale leases left behind after a crash (tests / doctor)."""
    _live, stale = pick_lease(load_leases(fingerprint))
    return tuple(stale)


__all__ = [
    "DEFAULT_TTL_MS",
    "FORCE_PLUGIN_ENV",
    "LEASE_TTL_ENV",
    "NATIVE_LIVE_ENV",
    "NATIVE_STATE_ENV",
    "OUTCOME_NATIVE_OWNS",
    "OUTCOME_PLUGIN_OWNS",
    "OWNER_NATIVE",
    "OWNER_PLUGIN",
    "SCHEMA",
    "WriterDecision",
    "WriterLease",
    "application_support_dir",
    "claim_native_writer",
    "claim_plugin_writer",
    "clear_owner",
    "clear_shared_restore",
    "env_truthy",
    "heartbeat_plugin_writer",
    "legacy_native_marker_path",
    "load_leases",
    "observe_foreign",
    "parse_lease_text",
    "pick_lease",
    "pid_alive",
    "plugin_marker_path",
    "read_lease_file",
    "read_shared_restore",
    "release_native_writer",
    "release_plugin_writer",
    "resolve_writer",
    "restore_paths",
    "state_dirs",
    "write_lease",
    "write_shared_restore",
    "writer_paths",
    "writer_status",
    "xdg_state_dir",
]
