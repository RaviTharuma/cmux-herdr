# cmux-herdr

User-controlled **plugin bridge** between [Herdr](https://github.com/ogulcancelik/herdr) (inner agent terminal mux) and [cmux](https://github.com/manaflow-ai/cmux) (outer macOS terminal workspace).

It works **today without any cmux upstream PR**.

## Why

When Herdr runs nested inside a cmux terminal, cmux sees one terminal surface while the real agent tabs and panes live inside Herdr. This plugin:

1. Mirrors Herdr agent state into cmux sidebar **status pills** and progress.
2. Provides a **`cmux-herdr` CLI** for topology and control.
3. Ships an optional **custom sidebar** and **agent skill** documenting the dual hierarchy.

## Two-path strategy

| Path | Status |
|---|---|
| **Plugin (this repo)** | Implemented — CLI, bridge, sidebar, skill, installer, and tests |
| **Upstream native MVP** | [PR #8736](https://github.com/manaflow-ai/cmux/pull/8736) — hidden `cmux __herdr-compat` dispatcher (`exec` into Herdr) |
| **Upstream native parity** | [Issue #8737](https://github.com/manaflow-ai/cmux/issues/8737) — first-class nested topology; plugin remains fallback |

See [plugin design](docs/PLUGIN_DESIGN.md), [open limitations](OPEN.md), [concept map](mapping/concept-map.md), and the paste-ready [upstream issue/design package](docs/upstream/).


## Hybrid association state

`sync` / `watch` keep a user-owned cache under `$XDG_STATE_HOME/cmux-herdr/` (default `~/.local/state/cmux-herdr/`):

- `parent-*.json` — locked outer cmux workspace binding for this Herdr socket/workspace
- `associations-*.json` — live inner `pane_id → status_key / agent_session / status` map, pruned each sync

This is the production stopgap data pattern while native nested topology lands. It is cache-only and not authoritative restore state for cmux.

```bash
cmux-herdr associations
cmux-herdr associations --json
```

## Requirements

- macOS with `cmux` and `herdr` on `PATH`
- Python 3.10+ (stdlib only; no pip dependencies)
- A working Herdr socket (usual when `HERDR_ENV=1`)
- Run `sync`/`watch` from a Herdr pane nested in cmux so both socket contexts are available; `tree` and `agents` still work without cmux

## Install

```bash
./scripts/install.sh
# installs ~/.local/bin/cmux-herdr, the sidebar, and the agent skill
```

Uninstall:

```bash
./scripts/uninstall.sh
```

## Quick start

```bash
cmux-herdr status         # dual context + socket health
cmux-herdr tree           # Herdr workspaces → tabs → panes → agents
cmux-herdr agents         # compact agent list
cmux-herdr sync           # one-shot mirror → cmux set-status
cmux-herdr watch          # loop every 3s (Ctrl-C to stop)
cmux-herdr associations   # hybrid pane/session association cache
cmux-herdr clear          # remove herdr:* status pills
```

Control helpers:

```bash
cmux-herdr focus-tab Orchestration
cmux-herdr focus-pane w2:p34
cmux-herdr split --direction right
cmux-herdr json-dump
```

### Custom sidebar

```bash
# Settings → Beta features → Custom sidebars (enable)
cmux sidebar reload
cmux sidebar validate herdr --json
cmux sidebar open herdr
```

The sidebar is a valid cmux interpreted-Swift sidebar and navigates outer cmux workspaces. Live inner-pane state appears through status pills written by `sync` or `watch`; the sidebar interpreter cannot shell out to Herdr.

## Status mapping

| Herdr status | Pill color | Icon |
|---|---|---|
| working | orange `#ff9500` | `hammer` |
| idle | gray | `pause.circle` |
| done | green | `checkmark.circle` |
| blocked | red | `exclamationmark.triangle` |
| unknown | gray | `questionmark.circle` |

Status keys use `herdr:<pane_id>`. Every sync removes stale `herdr:*` keys while preserving unrelated cmux status entries. Progress reports the fraction of agents still working.

## Layout

```text
bin/cmux-herdr              CLI (Python, stdlib only)
bridge/cmux_herdr_bridge.py fetch/map/sync library
bridge/test_bridge_unit.py  pure unit tests
tests/                      mocked CLI and behavior tests
scripts/install.sh          idempotent user install
scripts/uninstall.sh        scoped uninstall
sidebars/herdr.swift        optional cmux custom sidebar
agent-skill/SKILL.md        dual-hierarchy agent instructions
docs/PLUGIN_DESIGN.md       standalone plugin architecture
docs/upstream/              issue, native design, parity matrix, PR plan
mapping/concept-map.md       cmux ↔ Herdr ↔ tmux concepts
shims/README.md              optional shim guidance
```

## Limitations

- The plugin cannot turn inner Herdr panes into native Bonsplit panes; that needs the upstream nested-topology work ([#8737](https://github.com/manaflow-ai/cmux/issues/8737)).
- Polling is the portable fallback. Native integration should subscribe to Herdr events and resynchronize from snapshots.
- Nested shells may carry stale outer cmux IDs. The bridge resolves the live containing workspace before writing status.
- The bridge does not inject a fake `tmux` binary by default; see [shims/README.md](shims/README.md).
- `watch` is manual (no LaunchAgent). Titles/renames are owned by Herdr title tracks, not this plugin.
- PR [#8736](https://github.com/manaflow-ai/cmux/pull/8736) is a hidden compat dispatcher only — not full native parity.

Full stopgap inventory and open checklist: **[OPEN.md](OPEN.md)**.

## Development and verification

```bash
python3 -m py_compile bin/cmux-herdr bridge/cmux_herdr_bridge.py
PYTHONPATH=bridge python3 -m unittest discover -s bridge -p 'test_*.py' -v
PYTHONPATH=bridge python3 -m unittest discover -s tests -p 'test_*.py' -v
./bin/cmux-herdr status
./bin/cmux-herdr tree
./bin/cmux-herdr sync
cmux sidebar validate herdr --json
```

## Native integration proposal

Live trackers: [PR #8736](https://github.com/manaflow-ai/cmux/pull/8736) (MVP dispatcher) and [issue #8737](https://github.com/manaflow-ai/cmux/issues/8737) (full nested topology). Design drafts also live in [docs/upstream/](docs/upstream/).

The standalone bridge intentionally does not pretend that inner Herdr panes are native cmux panes. The upstream package proposes a capability-negotiated nested-topology provider:

- [Paste-ready GitHub issue](docs/upstream/ISSUE.md)
- [Annoyances / thrash report](docs/upstream/ANNOYANCES.md)
- [Technical design](docs/upstream/DESIGN.md)
- [Parity matrix](docs/upstream/PARITY_MATRIX.md)
- [Incremental PR plan](docs/upstream/PR_PLAN.md)

No issue or PR is opened automatically.

## Upstream tracking

- Native design issue: [manaflow-ai/cmux#8737](https://github.com/manaflow-ai/cmux/issues/8737)
- First native compatibility PR: [manaflow-ai/cmux#8736](https://github.com/manaflow-ai/cmux/pull/8736)
