<h1 align="center">cmux-herdr</h1>
<p align="center"><strong>A cmux plugin for Herdr</strong></p>
<p align="center">
  Herdr pane viewers and agent status inside cmux.
  External attach through the plugin today; native Herdr UI is a roadmap.
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

**cmux-herdr** is a plugin for [Herdr](https://github.com/herdrdev/herdr)
running inside [cmux](https://github.com/manaflow-ai/cmux). It projects agent
status and creates external pane viewers using `cmux-herdr attach-pane`.
It does not transfer Herdr TTY ownership into Ghostty or implement cmux's
builtin native tmux integration. Making cmux the native UI of Herdr is a
roadmap, not a capability shipped by this plugin.

The current source version is **v0.7.0**. This is a plugin for `cmux.app`, not
a patch to it. The plugin manager downloads a checksum-verified Rust binary;
users need neither Python nor a Rust toolchain.

## Install

Official install is the cmux plugin manager plus the `cmux-herdr` CLI.
Native Herdr chrome is roadmap work associated with upstream proposals
[#8736](https://github.com/manaflow-ai/cmux/pull/8736) and
[#10045](https://github.com/manaflow-ai/cmux/pull/10045), not present in the
audited cmux commit `829c6af45478ef5c2196801824c35f5cb4dc5d69`.
Native Sidebar issue #75 remains **blocked**, not fixed by v0.7.0.
This plugin does not copy a custom `herdr` sidebar into `~/.config/cmux/sidebars/`.

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

## Capability matrix

This is the release capability summary. Native design documents describe targets,
not additional features shipped by the plugin.

| Capability | Status | Scope |
|---|---|---|
| **External pane viewers** | Shipped plugin path | cmux terminal surfaces run `attach-pane`; Herdr retains the TTYs. |
| **Live watch and mirror** | Shipped plugin path | Userspace tab/split, layout, focus, order and prune projection; not builtin ssh-tmux output streaming or native TTY takeover. |
| **Status pills** | Shipped plugin path | Agent status chips and session progress on the containing cmux workspace. |
| **CLI and agent skill** | Shipped plugin path | Topology, control, external viewers and the allowlisted Herdr socket API; never `server.stop`. |
| **Writer coordination** | Shipped protocol | Instance-owned leases and recognition of native ownership records; a record does not prove a native controller exists. |
| **Native Herdr attach / window mirror** | Planned, not shipped | Requires upstream AppKit/Bonsplit/Ghostty integration; plugin lifecycle models are not native attachment. |
| **Integrated multi-workspace native sidebars** | Planned, blocked | Native Sidebar issue #75 remains blocked. The plugin sidebar TUI and experimental custom sidebars are not this integration. |

Source evidence and command-level constraints:
[stabilization audit](docs/upstream/STABILIZATION_AUDIT.md#source-pinned-capability-matrix).

## Quick start

Run these **inside a Herdr pane nested in cmux** so both sockets are in the
environment:

```bash
cmux-herdr doctor
cmux-herdr watch
```

`watch` is enough. You live in cmux chrome. `--pills-only` writes status
chips without projecting tabs and panes.


## Sidebar philosophy (cmux-native)

- **Left sidebar** stays workspaces/machines navigation. Do not invent
  Herdr-only chrome there. The intended nesting is: Herdr workspaces appear as
  ordinary sub-workspaces under the relevant machine connection — the same way
  cmux already nests workspaces under machines.
- **Right sidebar** is the home for richer Herdr UI/actions/status beyond
  navigation (ssh-tmux fashion): Agents/sessions, Feeds, and Dock — not a
  foreign Herdr panel. Project with `cmux-herdr rail` / `agents --rail`; see
  [docs/RIGHT_RAIL.md](docs/RIGHT_RAIL.md).
- Until native nested workspace sync lands, use the CLI (`sessions`, `doctor`,
  `watch`, `rail`, `attach`) and existing status/mirror surfaces. Do not
  duplicate the same agent in pills, sessions, and a custom list.

## Commands

| Command | What it does |
|---|---|
| `doctor` | Diagnose plugin install, host fingerprint, LaunchAgent |
| `status` | Show nested cmux + Herdr context |
| `tree` / `agents` | Inner topology, compact agent list (`agents --rail` → right-rail JSON) |
| `rail` | Project deduped agents into cmux right-rail JSON + `rail-<fp>.json` |
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
| `sessions` | List Herdr sessions (`remote.tmux.sessions` JSON shape) |
| `observe` | Subscribe to a Herdr method (for example `pane_surfaces` / `sessions`) |
| `json-dump` | Full snapshot for debugging (redact personal paths before sharing) |
| `update-service` | Opt-in Herdr auto-update (LaunchAgent / systemd user timer); off by default |

`cmux-herdr --help` lists flags for every subcommand. Herdr-only verbs with no
tmux analogue: [docs/upstream/HERDR_BEYOND_TMUX.md](docs/upstream/HERDR_BEYOND_TMUX.md).

### Optional Herdr auto-update

`update-service` is **opt-in**. It is not installed by the plugin manager. When
enabled, it registers a LaunchAgent (macOS) or systemd user timer (Linux),
writes only a marker-owned block under `[update]` in Herdr's config, checks
about every six hours, runs `herdr update --handoff`, and restores the previous
binary if the update fails after replacing it. You pass the release manifest
yourself — the plugin does not pin a third-party Herdr fork.

```bash
cmux-herdr update-service install \
  --manifest-url https://example.com/herdr-preview.json \
  --channel preview
# plist  → ~/Library/LaunchAgents/com.cmux-herdr.herdr-auto-update.plist
# logs   → ~/Library/Logs/cmux-herdr-herdr-auto-update.{out,err}.log
# timer  → ~/.config/systemd/user/com.cmux-herdr.herdr-auto-update.timer
# logs   → journalctl --user -u com.cmux-herdr.herdr-auto-update.service

cmux-herdr update-service status
cmux-herdr update-service run       # one-shot check (same path the timer uses)
cmux-herdr update-service uninstall
```

Install refuses to overwrite a different active channel or manifest URL. Uninstall
removes only the managed marker block and the service units this command
registered. Backups of prior Herdr binaries are kept under the plugin state dir
(bounded retention).

## Requirements

- macOS with `cmux` and `herdr` on `PATH`; a cmux build that includes `cmux sidebar plugin`
- A working Herdr socket (usual when `HERDR_ENV=1`)
- Herdr **0.8+** (agent name may live under `agent_session.agent`)
- `sync` / `watch` / `mirror` from a nested pane so both contexts exist; `tree` and `agents` still work without cmux
The plugin manager supplies the checksum-verified runtime binary. Contributors
building from source additionally need Rust/Cargo; users do not.

## How it works

```text
cmux.app  (outer terminal host; plugin creates external Herdr viewers)
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

**Single writer.** Plugin `sync` / `watch` / `mirror` / `attach` / `observe` /
`restore` yield to a fresh foreign writer. A dead pid or expired heartbeat
is stale. Native records are a compatibility protocol for future integration,
not evidence of shipped native topology. `CMUX_HERDR_NATIVE_LIVE=1` explicitly
asserts native ownership; `CMUX_HERDR_FORCE_PLUGIN=1` forces the plugin.
`CMUX_HERDR_LOCK_TITLES=1` locks each display name after the first successful
write. This is a handoff, not Ghostty PTY theft.

Full design: [docs/PLUGIN_DESIGN.md](docs/PLUGIN_DESIGN.md) ·
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) ·
[mapping/concept-map.md](mapping/concept-map.md).

## Status mapping

| Herdr status | Native icon | Priority |
|---|---|---|
| working | `hammer` | 80 |
| idle | `pause.circle` | 40 |
| done | `checkmark.circle` | 30 |
| blocked | `exclamationmark.triangle` | 90 |
| unknown | `questionmark.circle` | 10 |

Every sync removes stale `herdr:*` keys and leaves unrelated cmux status alone.
Progress is the fraction of agents still working. The sidebar shows the status
label (working, idle, done), not the raw key.

Native metadata receives text, SF Symbols and priority, without `--color`.
cmux owns theme and selected-row contrast. Its audited generic status API
interprets explicit hex colors, not adaptive semantic color tokens; omission
also clears a previous explicit tint on successful replacement. Legacy cached
colors remain retryable until that write succeeds, then unchanged syncs deduplicate.
Status meaning remains visible in text and icons, not plugin-owned state colors.
cmux deliberately substitutes selected foregrounds for contrast; this is not
an upstream defect established by #75, and the full colored-selected request
is not closed by this change.

The plugin-manager `sidebar` entrypoint is a **terminal workspace fallback**,
not a native metadata component. It retains socket navigation, `>` selection
and `*` active-workspace markers, using terminal-default foreground/background
without a reverse-video selection palette. Agent status uses native metadata
where exposed; no separate custom sidebar/theme or invented native plugin API
is introduced. See the [source-pinned capability audit](docs/upstream/STABILIZATION_AUDIT.md).

## Deep mirror

`cmux-herdr watch` is the product path. It turns on the full reconcile
contract (all tabs, prune, layout tree, ratios, tab order, focus) so inner
Herdr sessions appear as real cmux tabs and panes. `mirror` remains the
one-shot / scoped tool. Shipped versus planned status is defined by the
[capability matrix](#capability-matrix); [tmux design targets](docs/upstream/TMUX_PARITY.md)
are not release guarantees.

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

- Extra viewers, not TTY takeover. `RemoteHerdrWindowMirror` is roadmap work, absent from the audited cmux source. Native Sidebar issue #75 remains blocked.
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
That roadmap is tracked by [cmux#8737](https://github.com/manaflow-ai/cmux/issues/8737)
and related proposals. This plugin provides external viewers today. Its lease
protocol anticipates native cooperation but does not ship a native controller.

**Does it need a cmux PR to work?**
No. Install it with the plugin manager and run it.

## Planned native Herdr GUI

Integrated multi-workspace native sidebars and native pane attachment are
**planned capabilities, not shipped features**. They require upstream cmux
integration; installing this plugin does not enable them. Native Sidebar issue
#75 is **blocked**; plugin stabilization does not close it. The
[capability matrix](#capability-matrix) above defines current release scope;
[the stabilization audit](docs/upstream/STABILIZATION_AUDIT.md) provides source evidence.
Design notes live in [docs/upstream/](docs/upstream/README.md).

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
