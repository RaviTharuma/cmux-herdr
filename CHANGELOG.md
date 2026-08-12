# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] — 2026-08-12 — 0.2 prep

### Added

- **`cmux-herdr doctor`**: diagnose third-party install health — herdr on PATH /
  version, socket path (env or default) with mode/owner, host fingerprint
  completeness (never invents hosts), `$XDG_STATE_HOME/cmux-herdr/` binding state,
  LaunchAgent `com.cmux-herdr.watch` (macOS `launchctl` best-effort; skipped
  elsewhere), optional sidebar install path, and a one-shot dry sync summary.
  Exits non-zero on hard failures (herdr missing, or incomplete fingerprint when
  `HERDR_ENV` claims a nested env).
- **`cmux-herdr read-pane <pane_id>`** / **`read-agent <target>`**: thin wrappers
  over `herdr pane read` / `herdr agent read` with `--source` / `--lines` /
  `--format` / `--ansi` (and `--raw` for pane).
- **`cmux-herdr focus-workspace <id>`** / **`focus-agent <target>`**: complete the
  focus helpers alongside existing `focus-tab` / `focus-pane`.

### Fixed

- **Multi-parent host fingerprint bindings** ([#2](https://github.com/RaviTharuma/cmux-herdr/issues/2)):
  parent / association files are keyed by a stable host fingerprint
  (`CMUX_SURFACE_ID` + `HERDR_SOCKET_PATH` + optional Herdr server pid +
  `HERDR_WORKSPACE_ID`) so multiple outer cmux windows/surfaces keep concurrent
  `parent-<fingerprint>.json` / `associations-<fingerprint>.json` files under
  `$XDG_STATE_HOME/cmux-herdr/`. `sync` / `watch` select the binding for the
  invoking environment; `--workspace` remains an explicit override.
- Auto-resolve fails loudly when fingerprint pieces are missing (never probes the
  bare focused workspace / random host). Incomplete fingerprint with `--workspace`
  logs a warning that association keys may collide.
- **`focus-pane`**: remove misleading `herdr pane zoom … --off` “focus” fallback;
  report a clear error when `herdr agent focus` fails.

## [0.1.0] — 2026-08-12

First tagged stopgap release of the user-controlled cmux ↔ Herdr plugin bridge.
Works today without any cmux upstream merge.

### Added

- **CLI bridge** (`cmux-herdr`): `status`, `tree`, `agents`, `sync`, `watch`, `clear`,
  focus helpers, `split`, `json-dump`, and `associations`.
- **Status pills**: mirror Herdr agent state into cmux `set-status` keys (`herdr:<pane_id>`)
  with progress; stale `herdr:*` keys cleared each sync.
- **Hybrid associations cache** under `$XDG_STATE_HOME/cmux-herdr/` (default
  `~/.local/state/cmux-herdr/`): parent workspace binding + live pane/session map.
- **Custom sidebar** (`sidebars/herdr.swift`) for outer cmux workspace navigation.
- **LaunchAgent sample** + install helpers:
  `scripts/com.cmux-herdr.watch.plist`,
  `scripts/install-watch-service.sh`,
  `scripts/uninstall-watch-service.sh`
  (plugin issue [#1](https://github.com/RaviTharuma/cmux-herdr/issues/1) closed).
- **Agent skill** (`agent-skill/SKILL.md`) documenting the dual hierarchy.
- **Installer**: idempotent `./scripts/install.sh` / scoped `./scripts/uninstall.sh`.
- **Hermetic tests**: stdlib `unittest` only via `./scripts/test.sh` (no pytest).
- **VERSION** file as the version source of truth; `cmux-herdr --version` reads it.
- Dual-path tracking docs linking:
  - [manaflow-ai/cmux#8736](https://github.com/manaflow-ai/cmux/pull/8736) — hidden `__herdr-compat` MVP (open, mergeable)
  - [manaflow-ai/cmux#8737](https://github.com/manaflow-ai/cmux/issues/8737) — native nested topology (open; not implemented here)

### Fixed

- **Herdr 0.8** pane parsing: agent name nested under `agent_session.agent` when top-level
  `agent` is absent.
- CLI / bridge unit tests hermetic with mocks (no live sockets required for the suite).

### Notes / docs

- Upstream PR [#8736](https://github.com/manaflow-ai/cmux/pull/8736) tip includes the
  missing-`herdr`-on-PATH hermetic coverage tracked by plugin issue
  [#5](https://github.com/RaviTharuma/cmux-herdr/issues/5); that residual is closed in docs.
- Multi-parent binding ([#2](https://github.com/RaviTharuma/cmux-herdr/issues/2)) was
  deferred from this release; see **Unreleased / 0.2 prep**.
- Native nested topology ([#8737](https://github.com/manaflow-ai/cmux/issues/8737)) is
  intentionally out of scope for the plugin.

[0.1.0]: https://github.com/RaviTharuma/cmux-herdr/releases/tag/v0.1.0
