# cmux-herdr

[![CI](https://github.com/RaviTharuma/cmux-herdr/actions/workflows/ci.yml/badge.svg)](https://github.com/RaviTharuma/cmux-herdr/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Python 3.10+](https://img.shields.io/badge/python-3.10%2B-blue.svg)](https://www.python.org/downloads/)

User-controlled **plugin bridge** between [Herdr](https://github.com/herdrdev/herdr)
(inner agent terminal mux) and [cmux](https://github.com/manaflow-ai/cmux)
(outer macOS terminal workspace).

It works **today without any cmux upstream PR**. Current release: **v0.3.4**
([changelog](CHANGELOG.md), [how to tag](RELEASE.md)).

**Deutsch:** [Kurzüberblick](docs/de/README.md) · [GitHub erklärt](docs/de/GITHUB.md)

There is **no compile step** and nothing to `pip`/`npm` install. The plugin is
plain Python 3.10+ (standard library only). `./scripts/test.sh` syntax-checks
the sources and runs stdlib `unittest`. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

Documentation index: [docs/README.md](docs/README.md).
How to contribute: [CONTRIBUTING.md](CONTRIBUTING.md).
License: [MIT](LICENSE).

## Why

When Herdr runs nested inside a cmux terminal, cmux sees one terminal surface while the real agent tabs and panes live inside Herdr. This plugin:

1. Mirrors Herdr **tabs and panes into real cmux tabs/splits** (`cmux-herdr mirror`) — userspace analogue of cmux `ssh-tmux`.
2. Mirrors Herdr agent state into cmux sidebar **status pills** and progress (`sync` / `watch`).
3. Provides a **`cmux-herdr` CLI** for topology, `attach-pane` followers, and the published Herdr control surface (`api`, tabs/panes/workspaces/agents).
4. Ships an optional **custom sidebar** and **agent skill** documenting the dual hierarchy.

## Two-path strategy

| Path | Status |
|---|---|
| **Plugin (this repo)** | Implemented — CLI, bridge, sidebar, skill, installer, LaunchAgent, tests; `mirror --tmux-parity`; persistent `events.subscribe`; window-mirror engine |
| **Upstream community poll** | [Discussion #10106](https://github.com/manaflow-ai/cmux/discussions/10106) — native Herdr as tmux counterpart |
| **Upstream native MVP** | [PR #8736](https://github.com/manaflow-ai/cmux/pull/8736) — open + mergeable; hidden `cmux __herdr-compat` dispatcher (`exec` into Herdr) |
| **Upstream native sidebar** | [PR #10045](https://github.com/manaflow-ai/cmux/pull/10045) — nested topology tree + focus (not ssh-tmux) |
| **Upstream native tmux parity** | Engine PR: [RaviTharuma/cmux#8](https://github.com/RaviTharuma/cmux/pull/8) (`RemoteHerdrWindowMirror` in `CmuxNestedTopology`). AppKit/Bonsplit/Ghostty host wiring still required. See [PR7](docs/upstream/PR7_HERDR_WINDOW_MIRROR.md). |

See [plugin design](docs/PLUGIN_DESIGN.md), [open limitations](OPEN.md), [tmux parity](docs/upstream/TMUX_PARITY.md), [concept map](mapping/concept-map.md), and the [upstream design notes](docs/upstream/README.md) (native cmux work — not required to use this plugin).


## Hybrid association state

`sync` / `watch` keep a user-owned cache under `$XDG_STATE_HOME/cmux-herdr/` (default `~/.local/state/cmux-herdr/`):

- `parent-<fingerprint>.json` — locked outer cmux workspace binding for one host fingerprint
- `associations-<fingerprint>.json` — live inner `pane_id → status_key / agent_session / status` map plus `mirrors` (Herdr pane → cmux surface), `parent_tab_id`, `heuristic_satisfied`, `title_lock`, last written pill; pruned each sync
- `writer-<fingerprint>.json` / `native-live-<fingerprint>` / `plugin-live-<fingerprint>` — shared lease: one writer. JSON has `owner`, `pid`, `heartbeat_ms`. A dead pid or expired heartbeat is stale and the other path may resume. Legacy `native-live` (`1` / `live`) still counts while its mtime is fresh. Native also reads `~/Library/Application Support/cmux-herdr/` when that directory exists.
- `restore-<endpointHash>.json` — last attach (`mode: reattach` only). Shared so restart works regardless of who attached.

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
cmux-herdr lock-title w2:p34 --title Orchestrator
cmux-herdr unlock-title w2:p34
```

**Single writer (both paths).** Plugin and native share one lease. If native is live, plugin `sync` / `watch` / `mirror` / `attach` / `observe` / `restore` yield and do not start a competing apply host. If the plugin watch holds the lease, native should yield (same files). If native dies, the lease goes stale and `watch --tmux-parity` may resume. `CMUX_HERDR_NATIVE_LIVE=1` is an explicit native claim. `CMUX_HERDR_FORCE_PLUGIN=1` forces the plugin. `CMUX_HERDR_LOCK_TITLES=1` locks each display name after the first successful write. This is a handoff, not Ghostty PTY theft.

## Requirements

- macOS with `cmux` and `herdr` on `PATH`
- Python 3.10+ (stdlib only; **not** on PyPI; no pip dependencies)
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

Install a tagged release (see [RELEASE.md](RELEASE.md)):

```bash
git clone --branch v0.3.4 --depth 1 \
  https://github.com/RaviTharuma/cmux-herdr.git
cd cmux-herdr
./scripts/install.sh
```

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
cmux-herdr mirror --tmux-parity  # ssh-tmux contract (all + prune + layout/focus/order)
cmux-herdr mirror --dry-run
cmux-herdr attach-pane w2:p34   # follow one pane in this terminal
cmux-herdr associations   # hybrid pane/session association cache
cmux-herdr lock-title w2:p34 --title Orchestrator
cmux-herdr unlock-title w2:p34
cmux-herdr send-key w2:p34 C-Up       # encodes to Herdr ctrl+up
cmux-herdr observe --method pane_surfaces
cmux-herdr attach         # live apply host (tmux attach analogue)
cmux-herdr detach         # leaves the Herdr session running
cmux-herdr restore        # reattach after restart; never replay a stale tree
cmux-herdr api --list     # published Herdr methods (never server.stop)
cmux-herdr api pane.close --params '{"pane_id":"w2:p34"}'
cmux-herdr new-tab --label logs
cmux-herdr close-pane w2:p34          # --force if the agent is busy
cmux-herdr send w2:p34 echo hello
cmux-herdr agent-prompt w2:p34 "run tests" --wait --until done
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
cmux-herdr zoom-pane w2:p34 --mode on
cmux-herdr resize-pane w2:p34 --direction right --amount 0.1
cmux-herdr layout --tab w2:t1
cmux-herdr set-ratio --tab w2:t1 --ratio 0.6
cmux-herdr move-pane w2:p2 --tab w2:t2
cmux-herdr focus-dir right
cmux-herdr move-tab w2:t1 --index 0
cmux-herdr rename-pane w2:p2 logs
cmux-herdr start-agent reviewer --kind codex --pane w2:p3
cmux-herdr notify "tests done" --body "all green"
cmux-herdr agent-explain w2:p3
cmux-herdr agent-view w2:p3 diff
cmux-herdr process-info w2:p3
cmux-herdr worktree list
cmux-herdr manifests
cmux-herdr window-title "cmux-herdr"
cmux-herdr json-dump
```

Herdr-only surface (no ssh-tmux analogue): [docs/upstream/HERDR_BEYOND_TMUX.md](docs/upstream/HERDR_BEYOND_TMUX.md).

### Custom sidebar

```bash
# Settings → Beta features → Custom sidebars (enable)
cmux sidebar reload
cmux sidebar validate herdr --json
cmux sidebar open herdr
```

The sidebar is a valid cmux interpreted-Swift sidebar and navigates outer cmux workspaces. After `cmux-herdr mirror`, Herdr tabs appear as **real cmux tabs** in that workspace (plus status pills from `sync` / `watch`). The sidebar interpreter still cannot shell out to Herdr.

## Deep mirror (tabs and panes)

`cmux-herdr mirror` is the plugin analogue of cmux `ssh-tmux` / `RemoteTmuxWindowMirror`.
`--tmux-parity` turns on the same reconcile contract tmux gets natively (full session,
prune, layout tree, ratios, tab order, focus). Canonical matrix:
[docs/upstream/TMUX_PARITY.md](docs/upstream/TMUX_PARITY.md).

| Herdr | cmux projection |
|---|---|
| Tab | cmux tab (first pane is the tab root); order follows Herdr tab numbers |
| Extra panes in that tab | cmux splits from the Herdr layout tree (`horizontal` → right, `vertical` → down) |
| Split ratios | `cmux set-ratio` from layout cell rects |
| Focused pane | matching cmux surface (`--focus` / `--tmux-parity`) |
| Pane contents | `cmux-herdr attach-pane` follower (`herdr pane read` + `pane send-text`) |

Reconcile is **idempotent**: each pane is keyed `herdr-mirror:<pane_id>`, so a second `mirror` keeps existing surfaces and only creates/renames/prunes diffs.

```bash
cmux-herdr mirror              # current $HERDR_TAB_ID only (safe default)
cmux-herdr mirror --all        # full Herdr session
cmux-herdr mirror --tmux-parity
cmux-herdr mirror --dry-run    # plan only
cmux-herdr watch --tmux-parity # live tabs/splits + status pills + event wait
cmux-herdr mirror --prune      # close cmux surfaces whose Herdr panes are gone
```

This cannot steal Herdr PTYs into Ghostty (native PR7 still owns that). It creates **extra cmux viewers** of the live Herdr session, the same idea as attaching a second tmux client.

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
VERSION                     version source of truth (e.g. 0.3.4)
CHANGELOG.md                release notes
RELEASE.md                  tag / gh release / install-from-tag steps
OPEN.md                     stopgap inventory + open checklist
bin/cmux-herdr              CLI (Python, stdlib only)
bridge/cmux_herdr_bridge.py fetch/map/sync library
bridge/cmux_herdr_mirror.py tab/pane deep-mirror planner + attach-pane
bridge/cmux_herdr_layout.py Herdr layout tree (tmux RemoteTmuxLayoutNode analogue)
bridge/cmux_herdr_impose.py Bonsplit impose planner
bridge/cmux_herdr_host.py   host-apply verb order
bridge/cmux_herdr_io.py     isolated pane I/O + title-escape strip
bridge/cmux_herdr_session.py session-tab verbs
bridge/cmux_herdr_control.py named keys, focus rollback, seed, activity
bridge/cmux_herdr_lifecycle.py attach / detach / restore / remote.herdr.*
bridge/cmux_herdr_live.py   running apply host (in-memory Ghostty analogue)
bridge/cmux_herdr_api.py    socket-first allowlisted Herdr RPC
bridge/cmux_herdr_pump.py   SessionHost-style event pump into the apply host
bridge/cmux_herdr_handoff.py plugin ↔ native writer lease
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
docs/upstream/              issue, native design, tmux parity, PR7 window mirror
docs/upstream/TMUX_PARITY.md  ssh-tmux capability matrix (plugin + native)
mapping/concept-map.md      cmux ↔ Herdr ↔ tmux concepts
shims/README.md             optional shim guidance
```

## Limitations

- The plugin cannot steal inner Herdr PTYs into Ghostty/Bonsplit the way native `ssh-tmux` does. `mirror --tmux-parity` creates extra cmux tabs/splits running `attach-pane` followers instead (layout, focus, order, prune, tmux divider fractions via `cmux_herdr_impose.py`). Native window mirror is PR7 (`RemoteHerdrWindowMirror` + `RemoteHerdrImpose`); sidebar nested topology is [PR #10045](https://github.com/manaflow-ai/cmux/pull/10045). Tmux-depth development continues on the native apply path. See [docs/upstream/TMUX_PARITY.md](docs/upstream/TMUX_PARITY.md).
- `watch --tmux-parity` prefers a persistent Herdr Unix-socket `events.subscribe` session when present; otherwise it polls. Native PR7 should push bytes into Ghostty.
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
python3 -m py_compile bin/cmux-herdr bridge/cmux_herdr_*.py
PYTHONPATH=. python3 -m unittest discover -s bridge -p 'test_*.py' -v
PYTHONPATH=. python3 -m unittest discover -s tests -p 'test_*.py' -v
./bin/cmux-herdr --version
./bin/cmux-herdr doctor
./bin/cmux-herdr status
./bin/cmux-herdr tree
./bin/cmux-herdr sync
cmux sidebar validate herdr --json
```

## Native integration proposal

Live trackers: [discussion #10106](https://github.com/manaflow-ai/cmux/discussions/10106) (community poll), [PR #8736](https://github.com/manaflow-ai/cmux/pull/8736) (MVP dispatcher), [PR #10045](https://github.com/manaflow-ai/cmux/pull/10045) (nested topology v1), [RaviTharuma/cmux#8](https://github.com/RaviTharuma/cmux/pull/8) (window-mirror engine, merged), and [issue #8737](https://github.com/manaflow-ai/cmux/issues/8737) (full nested topology). Design drafts also live in [docs/upstream/](docs/upstream/).

The standalone bridge intentionally does not pretend that inner Herdr panes are native cmux panes. The upstream package proposes a capability-negotiated nested-topology provider:

- [Paste-ready GitHub issue](docs/upstream/ISSUE.md)
- [Annoyances / thrash report](docs/upstream/ANNOYANCES.md)
- [Technical design](docs/upstream/DESIGN.md)
- [Parity matrix](docs/upstream/PARITY_MATRIX.md)
- [Incremental PR plan](docs/upstream/PR_PLAN.md)

No issue or PR is opened automatically. This plugin does **not** implement #8737.

## Contributing

Bug reports and PRs are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md)
and the [Code of Conduct](CODE_OF_CONDUCT.md). Security issues go through
[SECURITY.md](SECURITY.md) (private advisory), not a public issue.

First time publishing or maintaining this GitHub repo?
[docs/MAINTAINING.md](docs/MAINTAINING.md) (English) and
[docs/de/GITHUB.md](docs/de/GITHUB.md) (Deutsch).

## License

[MIT](LICENSE) © 2026 Ravi Tharuma.

## Upstream tracking

- Community poll: [manaflow-ai/cmux#10106](https://github.com/manaflow-ai/cmux/discussions/10106)
- Native design issue: [manaflow-ai/cmux#8737](https://github.com/manaflow-ai/cmux/issues/8737)
- Nested topology v1: [manaflow-ai/cmux#10045](https://github.com/manaflow-ai/cmux/pull/10045)
- Window-mirror engine: [RaviTharuma/cmux#8](https://github.com/RaviTharuma/cmux/pull/8) (merged)
- First native compatibility PR: [manaflow-ai/cmux#8736](https://github.com/manaflow-ai/cmux/pull/8736)
