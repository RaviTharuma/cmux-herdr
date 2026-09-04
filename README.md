<h1 align="center">cmux-herdr</h1>
<p align="center"><strong>A cmux plugin for Herdr</strong></p>
<p align="center">
  cmux becomes the official UI of Herdr. Herdr is the engine.
  Native cmux chrome — mouse, Reorderable, tabs, and panes —
  not a boxed-in Herdr window.
</p>

<p align="center">
  <a href="https://github.com/RaviTharuma/cmux-herdr/actions/workflows/ci.yml"><img src="https://github.com/RaviTharuma/cmux-herdr/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://github.com/RaviTharuma/cmux-herdr/releases/latest"><img src="https://img.shields.io/github/v/release/RaviTharuma/cmux-herdr" alt="Release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/runtime-Rust-brown.svg" alt="Rust binary" /></a>
  <a href="https://github.com/topics/plugin"><img src="https://img.shields.io/badge/kind-cmux%20plugin-4c71f2.svg" alt="cmux plugin" /></a>
</p>

<p align="center">
  English ·
  <a href="docs/de/README.md">Deutsch</a>
  ·
  <a href="https://github.com/manaflow-ai/cmux">cmux</a>
  ·
  <a href="https://github.com/herdrdev/herdr">Herdr</a>
  ·
  <a href="CHANGELOG.md">changelog</a>
</p>

**cmux-herdr** is the plugin you install when [Herdr](https://github.com/herdrdev/herdr)
runs *inside* [cmux](https://github.com/manaflow-ai/cmux). cmux is the official
GUI. Herdr is the engine. Without this plugin, every agent collapses into one
cmux tab titled roughly `herdr`. With it, Herdr sessions are real cmux
workspaces, tabs, and panes — mouse, drag-and-drop, focus, order — not an
iframe and not a nested Herdr chrome box.

The current source version is **v0.7.0**. This is a plugin for `cmux.app`, not
a patch to it. The plugin manager downloads a checksum-verified Rust binary;
users need neither Python nor a Rust toolchain.

## Install

Official install is the cmux plugin manager plus the `cmux-herdr` CLI.
Native Herdr chrome is parent cmux ([#8736](https://github.com/manaflow-ai/cmux/pull/8736)
`__herdr-compat`, [#10045](https://github.com/manaflow-ai/cmux/pull/10045) nested
topology). This plugin does not copy a custom `herdr` sidebar into
`~/.config/cmux/sidebars/`.

```bash
cmux sidebar plugin install https://github.com/RaviTharuma/cmux-herdr.git
cmux sidebar plugin use cmux-herdr
cmux sidebar plugin update cmux-herdr
cmux sidebar plugin remove cmux-herdr
```

That clones into `$XDG_DATA_HOME/cmux/mux-plugins/cmux-herdr` (or
`~/.local/share/cmux/mux-plugins/cmux-herdr`). The plugin-manager build step
uses `bin/cmux-herdr-fetch` to select one of four release targets, download the
matching binary and `SHA256SUMS` over HTTPS, verify the checksum, and install it
atomically. `bin/cmux-herdr` and `bin/cmux-herdr-sidebar` are thin POSIX-sh
launchers for that binary. A source build (`cargo build --release`) is only a
fallback for unusual architectures or offline development.
## Development

The canonical checks are:

```bash
./scripts/test.sh          # cargo fmt --check, cargo clippy -- -D warnings, cargo test
./bin/cmux-herdr --version
./bin/cmux-herdr --help
./bin/cmux-herdr doctor
./bin/cmux-herdr-sidebar --help
```

Runtime implementation is under `src/*.rs`; `bin/*` contains only thin POSIX-sh
launchers. Layout of the repo: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
Index: [docs/README.md](docs/README.md).


### After install

```bash
cmux-herdr doctor
cmux-herdr watch
```

`watch` is the live GUI path (tmux-parity on by default). Contributor symlink
of the CLI (not a custom-sidebar copy): [CONTRIBUTING.md](CONTRIBUTING.md).
Release notes: [RELEASE.md](RELEASE.md). `sidebars/herdr.js` and `herdr.swift`
remain in the repo as experimental leftovers, not the default install.

## Features

| | |
|---|---|
| **Native cmux chrome** | Parent cmux owns Herdr windows, tabs, and panes (`#8736` / `#10045`). This plugin does not install a custom `herdr` sidebar. |
| **Live watch** | `cmux-herdr watch` keeps pills and surfaces in sync and projects Herdr tabs/panes into real cmux tabs and splits. Optional LaunchAgent so it survives closing the pane. |
| **Tab and pane mirror** | Layout, focus, order, and prune — the same contract cmux gives tmux over SSH. `watch` projects Herdr tabs/panes into real cmux tabs and splits. |
| **Status pills** | Every Herdr agent becomes a status chip on the cmux workspace — working, idle, done, blocked — plus a progress bar for the session. |
| **One CLI** | Topology (`tree`, `agents`), control (`new-tab`, `send`, `agent-prompt`), attach/detach/restore, and the published Herdr socket API — never `server.stop`. |
| **Agent skill** | Ships a `cmux-herdr` skill so coding agents drive Herdr through cmux chrome instead of treating it as tmux. |
| **Safe handoff** | Plugin and native cmux share one writer lease. If native nested topology is live, this plugin yields. If native dies, watch can resume. |

## Quick start

Run these **inside a Herdr pane nested in cmux** so both sockets are in the
environment:

```bash
cmux-herdr doctor
cmux-herdr watch
```

`watch` is enough. You live in cmux chrome. `--pills-only` writes status
chips without projecting tabs and panes.

## Commands

| Command | What it does |
|---|---|
| `doctor` | Diagnose plugin install, host fingerprint, LaunchAgent |
| `status` | Show nested cmux + Herdr context |
| `tree` / `agents` | Inner topology, compact agent list |
| `watch` | Live pills + real cmux tabs/panes (`--pills-only` skips projection) |
| `sync` | One-shot status pills |
| `mirror` | Project Herdr tabs/panes into cmux tabs/splits (`--all`, `--prune`, `--dry-run`) |
| `attach-pane` | Follow one Herdr pane in this terminal |
| `attach` / `detach` / `restore` | Live apply host; detach leaves Herdr running; restore never replays a stale tree |
| `associations` | Read the pane → status-key cache |
| `lock-title` / `unlock-title` | Pin a pill display name |
| `lease` | Inspect the plugin ↔ native writer lease |
| `clear` | Remove `herdr:*` pills; leave other cmux status alone |
| `focus-workspace` / `focus-tab` / `focus-pane` / `focus-agent` | Jump in the inner mux |
| `read-pane` / `read-agent` | Read terminal output |
| `send` / `send-key` / `agent-prompt` | Type, key chords, wait-until-done prompts |
| `new-tab` / `close-pane` / `split` / `zoom-pane` / `resize-pane` | Inner layout |
| `layout` / `set-ratio` / `move-pane` / `focus-dir` / `move-tab` / `rename-pane` | Tab geometry |
| `start-agent` / `agent-explain` / `agent-view` / `process-info` | Agent extras |
| `worktree` / `manifests` / `notify` / `window-title` | Herdr-only surface |
| `api` | Allowlisted Herdr RPC (`--list`; never `server.stop`) |
| `observe` | Subscribe to a Herdr method (for example `pane_surfaces`) |
| `json-dump` | Full snapshot for debugging (redact personal paths before sharing) |

`cmux-herdr --help` lists flags for every subcommand. Herdr-only verbs with no
tmux analogue: [docs/upstream/HERDR_BEYOND_TMUX.md](docs/upstream/HERDR_BEYOND_TMUX.md).

## Requirements

- macOS with `cmux` and `herdr` on `PATH`; a cmux build that includes `cmux sidebar plugin`
- A working Herdr socket (usual when `HERDR_ENV=1`)
- Herdr **0.8+** (agent name may live under `agent_session.agent`)
- `sync` / `watch` / `mirror` from a nested pane so both contexts exist; `tree` and `agents` still work without cmux
The plugin manager supplies the checksum-verified runtime binary. Contributors
building from source additionally need Rust/Cargo; users do not.

## How it works

```text
cmux.app  (the Herdr GUI: windows, workspaces, tabs, panes)
   └── Herdr engine
          └── tabs / panes / agents
                 └── cmux-herdr
                       herdr CLI + Unix socket  →  snapshot
                       cmux CLI                 →  pills, tabs, splits
```

`sync` and `watch` keep a user-owned cache under `$XDG_STATE_HOME/cmux-herdr/`
(default `~/.local/state/cmux-herdr/`):

| File | Role |
|---|---|
| `parent-<fingerprint>.json` | Locked outer cmux workspace for this host |
| `associations-<fingerprint>.json` | Live `pane_id → status_key / agent_session / status`, plus mirrors and title locks |
| `writer-<fingerprint>.json` | Single-writer lease (`owner`, `pid`, `heartbeat_ms`) |
| `restore-<endpointHash>.json` | Last attach (`mode: reattach` only) |

**Host fingerprint** (selects which files to read/write):

| Piece | Source | Required for auto-resolve |
|---|---|---|
| Outer surface | `CMUX_SURFACE_ID` | yes |
| Herdr socket | `HERDR_SOCKET_PATH` | yes |
| Herdr server pid | `HERDR_SERVER_PID` or a pid file beside the socket | optional |
| Inner workspace | `HERDR_WORKSPACE_ID` | scopes associations (defaults to `default`) |

Missing fingerprint pieces fail closed with a clear error — the plugin will not
guess a host and write pills onto a random workspace. `--workspace` still
overrides. This cache is not authoritative restore state for cmux.

**Single writer.** If native nested topology holds the lease, plugin `sync` /
`watch` / `mirror` / `attach` / `observe` / `restore` yield. A dead pid or
expired heartbeat is stale and the other path may resume. `CMUX_HERDR_NATIVE_LIVE=1`
is an explicit native claim. `CMUX_HERDR_FORCE_PLUGIN=1` forces the plugin.
`CMUX_HERDR_LOCK_TITLES=1` locks each display name after the first successful
write. This is a handoff, not Ghostty PTY theft.

Full design: [docs/PLUGIN_DESIGN.md](docs/PLUGIN_DESIGN.md) ·
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) ·
[mapping/concept-map.md](mapping/concept-map.md).

## Status mapping

| Herdr status | Pill color | Icon |
|---|---|---|
| working | orange `#ff9500` | `hammer` |
| idle | gray | `pause.circle` |
| done | green | `checkmark.circle` |
| blocked | red | `exclamationmark.triangle` |
| unknown | gray | `questionmark.circle` |

Every sync removes stale `herdr:*` keys and leaves unrelated cmux status alone.
Progress is the fraction of agents still working. The sidebar shows the status
label (working, idle, done), not the raw key.

## Deep mirror

`cmux-herdr watch` is the product path. It turns on the full reconcile
contract (all tabs, prune, layout tree, ratios, tab order, focus) so inner
Herdr sessions appear as real cmux tabs and panes. `mirror` remains the
one-shot / scoped tool. Matrix:
[docs/upstream/TMUX_PARITY.md](docs/upstream/TMUX_PARITY.md).

| Herdr | cmux projection |
|---|---|
| Tab | cmux tab (first pane is the tab root); order follows Herdr tab numbers |
| Extra panes | cmux splits from the layout tree (`horizontal` → right, `vertical` → down) |
| Split ratios | `cmux set-ratio` from layout cell rects |
| Focused pane | matching cmux surface |
| Pane contents | `cmux-herdr attach-pane` follower (`herdr pane read` + `pane send-text`) |

Reconcile is idempotent: each pane is keyed `herdr-mirror:<pane_id>`. A second
`watch` keeps existing surfaces and only creates, renames, or prunes diffs.

```bash
cmux-herdr watch                  # product path: live tabs/splits + pills
cmux-herdr watch --pills-only     # pills, no projection
cmux-herdr mirror                 # current $HERDR_TAB_ID only (safe one-shot)
cmux-herdr mirror --all           # full Herdr session
cmux-herdr mirror --tmux-parity
cmux-herdr mirror --dry-run       # plan only
cmux-herdr mirror --prune         # close cmux surfaces whose Herdr panes are gone
```

This cannot steal Herdr PTYs into Ghostty. It creates **extra cmux viewers** of
the live Herdr session — the same idea as attaching a second tmux client.

## Limitations

- Extra viewers, not PTY theft. Native window mirror is the cmux `RemoteHerdrWindowMirror` track.
- Nested shells can carry stale outer cmux IDs. The plugin re-resolves the live containing workspace before writing status.
- Multi-parent hosts need a complete fingerprint (`CMUX_SURFACE_ID` + `HERDR_SOCKET_PATH`).
- The plugin does not inject a fake `tmux` binary; see [shims/README.md](shims/README.md).
- Titles and renames are owned by Herdr title tracks, not this plugin.

Inventory and open checklist: **[OPEN.md](OPEN.md)**.

## FAQ

**Is this shipped inside cmux.app?**
No. It is a user-installed cmux plugin. You keep it when you upgrade cmux.

**How is this different from `herdr-plugin-cmux`?**
[lachieh/herdr-plugin-cmux](https://github.com/lachieh/herdr-plugin-cmux) is a
*Herdr* plugin (`herdr plugin install …`) that adds sidebar rows from the Herdr
side. **cmux-herdr** is the *cmux* plugin: official `cmux sidebar plugin`
install, `cmux-herdr` CLI, `watch` as the live GUI path, and agent skill.
You can use one or both; they share the idea, not the install.

**Will native cmux nested topology replace this?**
That work lives on [cmux#8737](https://github.com/manaflow-ai/cmux/issues/8737)
and related PRs. This plugin is the supported path today and stays the
compatibility fallback. Plugin and native share a writer lease so they do not
fight.

**Does it need a cmux PR to work?**
No. Install it with the plugin manager and run it.

## Native cmux track

Not required to use the plugin. Design notes live in
[docs/upstream/](docs/upstream/README.md).

| Track | Link |
|---|---|
| Community poll | [Discussion #10106](https://github.com/manaflow-ai/cmux/discussions/10106) |
| Compat dispatcher | [PR #8736](https://github.com/manaflow-ai/cmux/pull/8736) |
| Nested topology sidebar | [PR #10045](https://github.com/manaflow-ai/cmux/pull/10045) |
| Window-mirror engine | [RaviTharuma/cmux#8](https://github.com/RaviTharuma/cmux/pull/8) |
| Full design issue | [Issue #8737](https://github.com/manaflow-ai/cmux/issues/8737) |

## Development

The canonical checks are:

```bash
./scripts/test.sh          # cargo fmt --check, cargo clippy -- -D warnings, cargo test
./bin/cmux-herdr --version
./bin/cmux-herdr --help
./bin/cmux-herdr doctor
./bin/cmux-herdr-sidebar --help
```

Runtime implementation is under `src/*.rs`; `bin/*` contains only thin POSIX-sh
launchers. Layout of the repo: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
Index: [docs/README.md](docs/README.md).

## Contributing

Bug reports and PRs are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md)
and the [Code of Conduct](CODE_OF_CONDUCT.md). Security issues go through
[SECURITY.md](SECURITY.md) (private advisory), not a public issue.

Maintainer notes: [docs/MAINTAINING.md](docs/MAINTAINING.md) (English) and
[docs/de/GITHUB.md](docs/de/GITHUB.md) (Deutsch).

## License

[MIT](LICENSE) © 2026 Ravi Tharuma.
