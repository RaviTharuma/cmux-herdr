# Architecture

**cmux-herdr** is a released **cmux plugin for Herdr**: a Rust binary with
CLI, sidebar, mirror engine, and opt-in update service, plus thin POSIX-sh
launchers and an agent skill. Users install prebuilt checksum-verified assets;
contributors use Cargo.

```text
your Mac
  └── cmux.app          outer terminal workspace (macOS GUI)
        └── a terminal running herdr
              └── Herdr tabs / panes / AI agents
                    └── cmux-herdr (this plugin)
                          talks to both CLIs + the Herdr Unix socket
```

When Herdr is nested inside cmux, cmux normally sees **one** terminal. The
plugin copies Herdr agent status into cmux sidebar pills and can create extra
cmux tabs that *follow* Herdr panes. It cannot steal Herdr's real TTYs into
Ghostty; that is native cmux work, tracked upstream.

## Build and distribution

The plugin manager runs `bin/cmux-herdr-fetch`, which detects one of four
supported targets (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`), downloads the release
binary and `SHA256SUMS` over HTTPS, verifies the checksum, and installs it
atomically. `bin/cmux-herdr` and `bin/cmux-herdr-sidebar` are thin launchers.
Users need neither Python nor Rust. Unusual architectures and offline
development may use `cargo build --release` as a source fallback.

Develop against the clone:

```bash
./bin/cmux-herdr --help
./scripts/test.sh
```

## How the CLI is assembled

```text
cmux-herdr (Rust binary)
        │
        ├── CLI / sidebar / update-service
        ├── socket + API + state + handoff
        └── mirror / layout / live / lifecycle engines
                    ├── subprocess: `herdr` and `cmux` CLIs
                    └── Unix socket: Herdr NDJSON RPC (protocol 17)

```
Runtime modules live in `src/*.rs`; `bin/*` only resolves and execs the binary.

| Module | Job |
|---|---|
| `src/model.rs`, `src/bridge.rs` | Snapshot and topology models, status pills, fingerprints |
| `src/mirror.rs`, `src/layout.rs`, `src/impose.rs` | Tab/pane mirror and split planning |
| `src/engine.rs`, `src/live.rs`, `src/host.rs`, `src/io.rs` | Reconcile and pane I/O |
| `src/session.rs`, `src/control.rs`, `src/lifecycle.rs` | Session, focus, attach/detach/restore |
| `src/api.rs`, `src/socket.rs`, `src/pump.rs` | Allowlisted RPC and event pump |
| `src/state.rs`, `src/handoff.rs` | State store and writer lease |
| `src/update.rs` | Opt-in `update-service` and platform registration |

Version comes from the `VERSION` file at the repo root.
`cmux-herdr --version` reads that file.

## Runtime data (stays on your machine)

Default directory: `$XDG_STATE_HOME/cmux-herdr/`
(usually `~/.local/state/cmux-herdr/`).

Files are keyed by a **host fingerprint** (`CMUX_SURFACE_ID` +
`HERDR_SOCKET_PATH` + optional pid + Herdr workspace id) so two cmux windows
do not overwrite each other.

| File | Meaning |
|---|---|
| `parent-<fp>.json` | Which outer cmux workspace this host writes to |
| `associations-<fp>.json` | Pane → status / mirror / title-lock map |
| `writer-<fp>.json` | One-writer lease (`owner`, `pid`, heartbeat) |
| `restore-<hash>.json` | Last attach, `mode: reattach` only |

Never commit these files. They can contain local pane ids and titles.

On macOS the plugin also **reads** `~/Library/Application Support/cmux-herdr/`
if a native cmux build wrote a lease there.

## macOS vs Linux

| | macOS | Linux (CI / this kind of VM) |
|---|---|---|
| Product use (`sync`, `mirror`, sidebar, LaunchAgent) | Yes, with `cmux` + `herdr` | No (those apps are macOS) |
| `./scripts/test.sh` | Yes | Yes |
| `--version` / `--help` / most of `doctor` | Yes | Yes (`doctor` skips LaunchAgent) |

CI runs the hermetic Cargo suite on Ubuntu with fake `herdr`/`cmux` scripts. That
is enough to catch Rust regressions. Dogfood the product on a Mac.

## Optional extras

| Path | Role |
|---|---|
| `cmux-plugin.toml` | Plugin-manager manifest (`kind=sidebar`) |
| `bin/cmux-herdr-sidebar` | Sidebar TUI the official manager runs |
| `sidebars/herdr.js` | Experimental leftover sidebar (not default-installed) |
| `sidebars/herdr.swift` | Experimental leftover Swift sidebar (not default-installed) |
| `agent-skill/SKILL.md` | Dual-hierarchy notes for coding agents |
| `scripts/com.cmux-herdr.watch.plist` | LaunchAgent template (`/Users/PLACEHOLDER` is replaced on install) |

## What lives under `docs/upstream/`

Design notes and paste-ready text for **native** Herdr support inside cmux.
You do not need them to install or run this plugin. Start at
[upstream/README.md](upstream/README.md).
