# Architecture

**cmux-herdr** is a released **cmux plugin for Herdr**: a Python 3.10+ CLI,
an optional custom sidebar, and an agent skill. It is not an Xcode app, not a
Swift package, and not something you compile.

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

## There is no build

| People sometimes expect | What this repo actually does |
|---|---|
| `make`, `cmake`, `xcodebuild` | Nothing. There are no compiled artifacts. |
| `pip install` / `npm install` | Nothing. Standard library only. |
| A `.app` or Homebrew formula | Install is `./scripts/install.sh` (symlink + copy). |
| `python3 -m py_compile` | Syntax check used by tests, **not** a compiler producing binaries. |

Develop against the clone:

```bash
./bin/cmux-herdr --help
./scripts/test.sh
```

`scripts/install.sh` only symlinks `bin/cmux-herdr` to `~/.local/bin` and copies
the optional sidebar and agent skill. Edits in the clone show up immediately
when the symlink is in place.

## How the CLI is assembled

```text
bin/cmux-herdr          argparse CLI (adds the repo root to sys.path)
        │
        └── import bridge.cmux_herdr_*
                    ├── subprocess: `herdr` and `cmux` CLIs
                    └── Unix socket: Herdr NDJSON RPC (protocol 17)
```

| Module | Job |
|---|---|
| `bridge/cmux_herdr_bridge.py` | Snapshot, status pills, fingerprints, `sync` |
| `bridge/cmux_herdr_mirror.py` | Tab/pane mirror plan + `attach-pane` follower |
| `bridge/cmux_herdr_layout.py` | Herdr layout tree → split plan |
| `bridge/cmux_herdr_engine.py` | Reconcile engine (Python twin of native window mirror) |
| `bridge/cmux_herdr_impose.py` | Divider / ratio planner |
| `bridge/cmux_herdr_host.py` | Ordered apply verbs |
| `bridge/cmux_herdr_io.py` | Pane I/O isolation |
| `bridge/cmux_herdr_session.py` | Session-tab create/close/reorder |
| `bridge/cmux_herdr_control.py` | Named keys, focus rollback, activity |
| `bridge/cmux_herdr_lifecycle.py` | Attach / detach / restore |
| `bridge/cmux_herdr_live.py` | In-memory apply host (Ghostty stand-in) |
| `bridge/cmux_herdr_handoff.py` | Plugin ↔ native writer lease |
| `bridge/cmux_herdr_api.py` | Allowlisted Herdr RPC (never `server.stop`) |
| `bridge/cmux_herdr_socket.py` | Persistent NDJSON socket + `events.subscribe` |
| `bridge/cmux_herdr_pump.py` | Event pump into the live host |

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

CI runs the hermetic suite on Ubuntu with fake `herdr`/`cmux` scripts. That is
enough to catch Python regressions. Dogfood the product on a Mac.

## Optional extras

| Path | Role |
|---|---|
| `sidebars/herdr.swift` | cmux custom sidebar (outer workspaces only) |
| `agent-skill/SKILL.md` | Dual-hierarchy notes for coding agents |
| `scripts/com.cmux-herdr.watch.plist` | LaunchAgent template (`/Users/PLACEHOLDER` is replaced on install) |

## What lives under `docs/upstream/`

Design notes and paste-ready text for **native** Herdr support inside cmux.
You do not need them to install or run this plugin. Start at
[upstream/README.md](upstream/README.md).
