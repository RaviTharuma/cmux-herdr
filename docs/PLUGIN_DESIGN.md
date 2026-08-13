# Plugin design: cmux-herdr

## Problem

On this machine, **herdr runs nested inside cmux**. Outer cmux users/agents only see one surface titled roughly `herdr`, while the real multi-agent topology (tabs/panes/statuses) lives inside herdr. Without a bridge, cmux sidebars and automation are blind to inner agents.

## Two-path strategy

| Path | What | When |
|------|------|------|
| **Plugin (now)** | User-controlled repo: CLI + bridge + optional sidebar + agent skill | Works today, **no cmux upstream PR** |
| **Upstream (later)** | Native cmux awareness of nested herdr (first-class tree/status) | Requires cmux changes; plugin stays as fallback/compat |

This repo implements the plugin path only. It must not depend on unmerged cmux patches.

## Architecture

```
┌─────────────────────────────────────────────┐
│ cmux (outer)                                │
│  window → workspace → tab/pane → surface    │
│       ▲ status pills / progress             │
│       ▲ mirrored tabs/splits (optional)     │
│       │ cmux set-status / create-terminal   │
└───────┼─────────────────────────────────────┘
        │
   cmux-herdr sync|watch|mirror
        │
┌───────┼─────────────────────────────────────┐
│ herdr (inner)                               │
│  workspace → tab → pane → agent             │
│  herdr pane list / pane read / pane send    │
└─────────────────────────────────────────────┘
```

### Components

1. **`bridge/cmux_herdr_bridge.py`** — pure-ish library:
   - `fetch_panes` / `fetch_snapshot`
   - `map_status_to_style`
   - `resolve_cmux_workspace` (nested env is often wrong)
   - `sync_to_cmux`
2. **`bin/cmux-herdr`** — user CLI (`status`, `tree`, `sync`, `watch`, focus helpers, …)
3. **`sidebars/herdr.swift`** — best-effort custom sidebar (navigator + instructions; live agent rows come from status pills written by the bridge)
4. **`agent-skill/SKILL.md`** — teaches agents the dual hierarchy
5. **`scripts/install.sh` / `uninstall.sh`** — symlink CLI, install sidebar + skill

## Why status pills plus optional tab/pane mirror

Custom sidebars run in a **restricted Swift interpreter** and primarily see the **cmux workspace model**. They may not shell out to `herdr`. Therefore:

- `sync` / `watch` push live agent state into `cmux set-status` keys (`herdr:<pane_id>`).
- `mirror` creates real cmux tabs/splits running `attach-pane` followers so the outer workspace *looks* like ssh-tmux (idempotent `herdr-mirror:<pane_id>` keys).
- The sidebar lists outer workspaces / the tabs `mirror` created; it still cannot call Herdr itself.

Until native nested topology lands, `cmux-herdr watch --tmux-parity` is the supported live deep mirror.

## Workspace resolution caveat

When a shell is inside herdr inside cmux, `CMUX_WORKSPACE_ID` is sometimes stale or equal to a tab id. The bridge:

1. Requires a host fingerprint (`CMUX_SURFACE_ID` + `HERDR_SOCKET_PATH`; optional Herdr server pid)
2. Loads `parent-<fingerprint>.json` for that invoking environment when still valid
3. Resolves via `cmux identify --surface <CMUX_SURFACE_ID> --json`, then a validated `CMUX_WORKSPACE_ID`
4. Persists a new binding for that fingerprint only (never probes the bare focused workspace)

Callers can override with `cmux-herdr sync --workspace …`. Missing fingerprint pieces fail loudly on auto-resolve.

## Status mapping

| herdr `agent_status` | icon | color | priority |
|----------------------|------|-------|----------|
| working | hammer | `#ff9500` | 80 |
| idle | pause.circle | `#8e8e93` | 40 |
| done | checkmark.circle | `#34c759` | 30 |
| blocked | exclamationmark.triangle | `#ff3b30` | 90 |
| unknown | questionmark.circle | `#8e8e93` | 10 |

Progress bar: `working / (working+idle+done+blocked)`.

## Non-goals (plugin path)

- Patching cmux.app or shipping a signed sidebar bundle
- Stealing Herdr PTYs into Ghostty (native `ssh-tmux` / PR7 `RemoteHerdrWindowMirror`)
- Network calls
- Requiring root or Homebrew formula (plain user install is enough)

## Upstream path (future)

Native tmux parity (PR7) would:

- Copy `RemoteTmuxWindowMirror` for Herdr (real tabs/panes, layout, I/O)
- Keep #10045 sidebar as the session navigator
- Keep this plugin as fallback (`watch --tmux-parity`) when native attachment is not live

Until then, `cmux-herdr watch --tmux-parity` is the supported live deep mirror.
