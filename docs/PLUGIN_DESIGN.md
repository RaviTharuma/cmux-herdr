# Plugin design: cmux-herdr

**cmux-herdr** is the cmux plugin for nested Herdr. Product overview:
[README.md](../README.md).

## Problem

When **Herdr runs nested inside cmux**, outer cmux users and agents only see one surface titled roughly `herdr`, while the real multi-agent topology (tabs/panes/statuses) lives inside Herdr. Without this plugin, cmux sidebars and automation are blind to inner agents.

## Two-path strategy

| Path | What | When |
|------|------|------|
| **This plugin** | Released pack: CLI + bridge + sidebar + agent skill + LaunchAgent | Install today; no cmux source patch |
| **Upstream native** | First-class nested topology inside cmux.app | Requires cmux changes; plugin stays as fallback |

This repository implements the plugin. It must not depend on unmerged cmux patches.

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
│  herdr pane list / pane read / pane send-text    │
└─────────────────────────────────────────────┘
```

### Components

1. **`bridge/cmux_herdr_bridge.py`** — pure-ish library:
   - `fetch_panes` / `fetch_snapshot`
   - `map_status_to_style`
   - `resolve_cmux_workspace` (nested env is often wrong)
   - `sync_to_cmux`
2. **`bin/cmux-herdr`** — user CLI (`status`, `tree`, `sync`, `watch`, `api`, focus helpers, …)
3. **`bridge/cmux_herdr_api.py`** — socket-first allowlisted Herdr RPC (never `server.stop`)
4. **`bridge/cmux_herdr_pump.py`** — SessionHost-style event pump into `LiveApplyHost`
5. **`sidebars/herdr.js`** (+ `herdr.swift` fallback) — interpreted custom sidebar named Herdr: live `workspaces` (reorder, select, context menu), host Ghostty/cmux theme tokens, and live statuses / tabs. `.js` wins for drag. No iframe, no CLI cheat-sheet. Agent pills still come from `cmux-herdr watch`.
6. **`agent-skill/SKILL.md`** — teaches agents the dual hierarchy
7. **`cmux-plugin.toml` + `bin/cmux-herdr-sidebar`** — official plugin-manager install
8. **`scripts/install.sh` / `uninstall.sh`** — contributor symlink CLI + optional JS/Swift sidebar + skill

## Why status pills plus optional tab/pane mirror

Custom sidebars run in a **restricted Swift interpreter** and primarily see the **cmux workspace model**. They may not shell out to `herdr`. Therefore:

- `sync` / `watch` push live agent state into `cmux set-status` keys (`herdr:<pane_id>`).
- `mirror` creates real cmux tabs/splits running `attach-pane` followers so the outer workspace *looks* like ssh-tmux (idempotent `herdr-mirror:<pane_id>` keys).
- The sidebar lists live outer workspaces, their cmux statuses (`herdr:<pane_id>` when projected), coding-agent sessions, and the focused workspace's surfaces. It still cannot call Herdr itself.

Until native nested topology lands, `cmux-herdr watch` is the supported live deep mirror.

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

## Writer contract (plugin + native)

Shared rules so upgrades do not thrash titles or parentage:

1. **Association key** — `pane_id:session_id` (falls back to `pane_id`).
2. **Parent map** — persist `parent_tab_id` / `parent_workspace_id`; render from the map.
3. **Heuristic once** — env / sole-tab inference runs only before `heuristic_satisfied`.
4. **Native-title lock** — locked display names are not overwritten; always diff before `set-status`.
5. **Single writer** — plugin and native share one lease (`writer-<fingerprint>.json` plus `native-live-*` / `plugin-live-*`). A dead pid or expired heartbeat does not hold the lock. Escape hatch: `CMUX_HERDR_FORCE_PLUGIN=1`.

## Non-goals (plugin path)

- Patching cmux.app or shipping a signed sidebar bundle
- Stealing Herdr PTYs into Ghostty (native `ssh-tmux` / PR7 `RemoteHerdrWindowMirror`)
- Network calls
- Requiring root or Homebrew formula (plain user install is enough)

## Upstream path (future)

Native tmux parity (PR7) copies `RemoteTmuxWindowMirror` in `CmuxNestedTopology`
(`RemoteHerdrWindowMirror` engine + pane I/O). AppKit/Bonsplit/Ghostty host
wiring is still required in the cmux app. This plugin stays the fallback
(`watch`) when native attachment is not live.

Until then, `cmux-herdr watch` is the supported live deep mirror.
