# cmux-herdr

User-controlled **plugin bridge** between [Herdr](https://github.com/ogulcancelik/herdr) (inner agent terminal mux) and [cmux](https://github.com/manaflow-ai/cmux) (outer macOS terminal workspace).

It works **today without any cmux upstream PR**. Current stopgap release target: **v0.1.0** (see [CHANGELOG.md](CHANGELOG.md), [RELEASE.md](RELEASE.md)).

## Why

When Herdr runs nested inside a cmux terminal, cmux sees one terminal surface while the real agent tabs and panes live inside Herdr. This plugin:

1. Mirrors Herdr **tabs and panes into real cmux tabs/splits** (`cmux-herdr mirror`) — userspace analogue of cmux `ssh-tmux`.
2. Mirrors Herdr agent state into cmux sidebar **status pills** and progress (`sync` / `watch`).
3. Provides a **`cmux-herdr` CLI** for topology, `attach-pane` followers, and control.
4. Ships an optional **custom sidebar** and **agent skill** documenting the dual hierarchy.

## Two-path strategy

| Path | Status |
|---|---|
| **Plugin (this repo)** | Implemented — CLI, bridge, sidebar, skill, installer, LaunchAgent, tests |
| **Upstream native MVP** | [PR #8736](https://github.com/manaflow-ai/cmux/pull/8736) — open + mergeable; hidden `cmux __herdr-compat` dispatcher (`exec` into Herdr) |
| **Upstream native parity** | [Issue #8737](https://github.com/manaflow-ai/cmux/issues/8737) — first-class nested topology; plugin remains fallback |

See [plugin design](docs/PLUGIN_DESIGN.md), [open limitations](OPEN.md), [concept map](mapping/concept-map.md), and the paste-ready [upstream issue/design package](docs/upstream/).


## Hybrid association state

`sync` / `watch` keep a user-owned cache under `$XDG_STATE_HOME/cmux-herdr/` (default `~/.local/state/cmux-herdr/`):

- `parent-<fingerprint>.json` — locked outer cmux workspace binding for one host fingerprint
- `associations-<fingerprint>.json` — live inner `pane_id → status_key / agent_session / status` map plus `mirrors` (Herdr pane → cmux surface) for deep-mirror reconcile, pruned each sync

**Host fingerprint** (selects which files `sync` / `watch` read/write):

| Piece | Source | Required for auto-resolve |
|---|---|---|
| Outer surface | `CMUX_SURFACE_ID` | yes |
| Herdr socket | `HERDR_SOCKET_PATH` | yes |
| Herdr server pid | `HERDR_SERVER_PID` or `<socket>.pid` / `herdr.pid` beside the socket | optional |
| Inner workspace | `HERDR_WORKSPACE_ID` | scopes associations (defaults to `default`) |

Multiple outer cmux windows/surfaces can keep concurrent binding files without overwriting each other. Pass `--workspace` to override the outer workspace explicitly. If fingerprint pieces are missing, auto-resolve fails with a clear error (it will not silently write pills to the focused / random host); `--workspace` still works but may warn that association keys are weak.

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
- Herdr **0.8+** supported (agent name may live under `agent_session.agent`)

## Install

```bash
./scripts/install.sh
# installs ~/.local/bin/cmux-herdr, the sidebar, and the agent skill
```

Optional continuous mirroring (LaunchAgent sample — issue [#1](https://github.com/RaviTharuma/cmux-herdr/issues/1) closed):

```bash
./scripts/install-watch-service.sh
# plist → ~/Library/LaunchAgents/com.cmux-herdr.watch.plist
# logs  → ~/Library/Logs/cmux-herdr-watch.{out,err}.log
```

Uninstall:

```bash
./scripts/uninstall-watch-service.sh   # if you installed the LaunchAgent
./scripts/uninstall.sh
```

Tagged install after `v0.1.0` exists: see [RELEASE.md](RELEASE.md).

### Install paths

| Artifact | Path |
|---|---|
| CLI | `~/.local/bin/cmux-herdr` |
| Sidebar | `~/.config/cmux/sidebars/herdr.swift` |
| Agent skill | `~/.agents/skills/cmux-herdr/` (and/or `~/.pi/agent/skills/cmux-herdr/`) |
| Association cache | `~/.local/state/cmux-herdr/` |
| LaunchAgent | `~/Library/LaunchAgents/com.cmux-herdr.watch.plist` |
| Watch logs | `~/Library/Logs/cmux-herdr-watch.{out,err}.log` |

## Quick start

```bash
cmux-herdr --version      # reads VERSION (e.g. 0.1.0)
cmux-herdr doctor         # diagnose install / fingerprint / LaunchAgent / dry sync
cmux-herdr status         # dual context + socket health
cmux-herdr tree           # Herdr workspaces → tabs → panes → agents
cmux-herdr agents         # compact agent list
cmux-herdr sync           # one-shot mirror → cmux set-status
cmux-herdr watch          # loop every 3s (Ctrl-C to stop)
cmux-herdr mirror         # project current Herdr tab → cmux tabs/splits
cmux-herdr mirror --all   # project every Herdr tab/pane (ssh-tmux-style)
cmux-herdr mirror --dry-run
cmux-herdr attach-pane w2:p34   # follow one pane in this terminal
cmux-herdr associations   # hybrid pane/session association cache
cmux-herdr clear          # remove herdr:* status pills
```

Control / read helpers:

```bash
cmux-herdr focus-workspace w2
cmux-herdr focus-tab Orchestration
cmux-herdr focus-pane w2:p34
cmux-herdr focus-agent w2:p34
cmux-herdr read-pane w2:p34 --source recent-unwrapped --lines 80
cmux-herdr read-agent reviewer --source recent --lines 40
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

The sidebar is a valid cmux interpreted-Swift sidebar and navigates outer cmux workspaces. After `cmux-herdr mirror`, Herdr tabs appear as **real cmux tabs** in that workspace (plus status pills from `sync` / `watch`). The sidebar interpreter still cannot shell out to Herdr.

## Deep mirror (tabs and panes)

`cmux-herdr mirror` is the plugin analogue of cmux `ssh-tmux` / `RemoteTmuxWindowMirror`:

| Herdr | cmux projection |
|---|---|
| Tab | cmux tab (first pane is the tab root) |
| Extra panes in that tab | cmux splits |
| Pane contents | `cmux-herdr attach-pane` follower (`herdr pane read` + `pane send`) |

Reconcile is **idempotent**: each pane is keyed `herdr-mirror:<pane_id>`, so a second `mirror` keeps existing surfaces and only creates/renames/prunes diffs.

```bash
cmux-herdr mirror              # current $HERDR_TAB_ID only (safe default)
cmux-herdr mirror --all        # full Herdr session
cmux-herdr mirror --dry-run    # plan only
cmux-herdr watch --mirror      # keep tabs/splits + status pills in sync
cmux-herdr mirror --prune      # close cmux surfaces whose Herdr panes are gone
```

This cannot steal Herdr PTYs into Ghostty (native cmux still owns that). It creates **extra cmux viewers** of the live Herdr session, the same idea as attaching a second tmux client.

## Status mapping

| Herdr status | Pill color | Icon |
|---|---|---|
| working | orange `#ff9500` | `hammer` |
| idle | gray | `pause.circle` |
| done | green | `checkmark.circle` |
| blocked | red | `exclamationmark.triangle` |
| unknown | gray | `questionmark.circle` |

Status keys use `herdr:<pane_id>`. Every sync removes stale `herdr:*` keys while preserving unrelated cmux status entries. Progress reports the fraction of agents still working. Pill content depends on Herdr `agent_status`.

## Layout

```text
VERSION                     version source of truth (e.g. 0.1.0)
CHANGELOG.md                release notes
RELEASE.md                  tag / gh release / install-from-tag steps
OPEN.md                     stopgap inventory + open checklist
bin/cmux-herdr              CLI (Python, stdlib only)
bridge/cmux_herdr_bridge.py fetch/map/sync library
bridge/cmux_herdr_mirror.py tab/pane deep-mirror planner + attach-pane
bridge/test_bridge_unit.py  pure unit tests
bridge/test_mirror_unit.py  deep-mirror planner tests
tests/                      mocked CLI and behavior tests
scripts/install.sh          idempotent user install
scripts/uninstall.sh        scoped uninstall
scripts/test.sh             stdlib unittest runner (no pytest)
scripts/com.cmux-herdr.watch.plist   sample LaunchAgent
scripts/install-watch-service.sh     load watch LaunchAgent
scripts/uninstall-watch-service.sh   unload watch LaunchAgent
sidebars/herdr.swift        optional cmux custom sidebar
agent-skill/SKILL.md        dual-hierarchy agent instructions
docs/PLUGIN_DESIGN.md       standalone plugin architecture
docs/upstream/              issue, native design, parity matrix, PR plan
mapping/concept-map.md      cmux ↔ Herdr ↔ tmux concepts
shims/README.md             optional shim guidance
```

## Limitations

- The plugin cannot steal inner Herdr PTYs into Ghostty/Bonsplit the way native `ssh-tmux` does. `mirror` creates extra cmux tabs/splits running `attach-pane` followers instead. Native nested topology remains [issue #8737](https://github.com/manaflow-ai/cmux/issues/8737) / [PR #10045](https://github.com/manaflow-ai/cmux/pull/10045).
- Polling is the portable fallback. Native integration should subscribe to Herdr events and resynchronize from snapshots.
- Nested shells may carry stale outer cmux IDs. The bridge resolves the live containing workspace before writing status.
- Status pills depend on Herdr `agent_status`. Multi-parent hosts need a complete fingerprint (`CMUX_SURFACE_ID` + `HERDR_SOCKET_PATH`); see hybrid association state above ([#2](https://github.com/RaviTharuma/cmux-herdr/issues/2)).
- The bridge does not inject a fake `tmux` binary by default; see [shims/README.md](shims/README.md).
- Continuous `watch` can run in a pane, or via the shipped LaunchAgent (`./scripts/install-watch-service.sh`). Titles/renames are owned by Herdr title tracks, not this plugin.
- PR [#8736](https://github.com/manaflow-ai/cmux/pull/8736) is a hidden compat dispatcher only — not full native parity. Its tip already covers the missing-`herdr`-on-PATH hermetic test (plugin issue [#5](https://github.com/RaviTharuma/cmux-herdr/issues/5) is not an open residual).

Full stopgap inventory and open checklist: **[OPEN.md](OPEN.md)**.

## Development and verification

**Tests are stdlib `unittest` only — no pytest.** Prefer the wrapper:

```bash
./scripts/test.sh
```

Equivalent manual commands:

```bash
python3 -m py_compile bin/cmux-herdr bridge/cmux_herdr_bridge.py
PYTHONPATH=bridge python3 -m unittest discover -s bridge -p 'test_*.py' -v
PYTHONPATH=bridge python3 -m unittest discover -s tests -p 'test_*.py' -v
./bin/cmux-herdr --version
./bin/cmux-herdr doctor
./bin/cmux-herdr status
./bin/cmux-herdr tree
./bin/cmux-herdr sync
cmux sidebar validate herdr --json
```

## Native integration proposal

Live trackers: [PR #8736](https://github.com/manaflow-ai/cmux/pull/8736) (MVP dispatcher, open + mergeable) and [issue #8737](https://github.com/manaflow-ai/cmux/issues/8737) (full nested topology). Design drafts also live in [docs/upstream/](docs/upstream/).

The standalone bridge intentionally does not pretend that inner Herdr panes are native cmux panes. The upstream package proposes a capability-negotiated nested-topology provider:

- [Paste-ready GitHub issue](docs/upstream/ISSUE.md)
- [Annoyances / thrash report](docs/upstream/ANNOYANCES.md)
- [Technical design](docs/upstream/DESIGN.md)
- [Parity matrix](docs/upstream/PARITY_MATRIX.md)
- [Incremental PR plan](docs/upstream/PR_PLAN.md)

No issue or PR is opened automatically. This plugin does **not** implement #8737.

## Upstream tracking

- Native design issue: [manaflow-ai/cmux#8737](https://github.com/manaflow-ai/cmux/issues/8737)
- First native compatibility PR: [manaflow-ai/cmux#8736](https://github.com/manaflow-ai/cmux/pull/8736)
