# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.1] - 2026-09-02

### Changed

- Demote the custom `herdr` sidebar. Official install is
  `cmux sidebar plugin install` plus the `cmux-herdr` CLI (`watch` /
  `doctor` / `status` / `mirror`). `install.sh` no longer copies
  `sidebars/herdr.js` or `herdr.swift` into `~/.config/cmux/sidebars/`.
  README and the German overview no longer document `cmux sidebar open herdr`
  as the UX. Native Herdr chrome is parent cmux (#8736, #10045).
  `sidebars/herdr.{js,swift}` remain in the repo as experimental leftovers.
  Uninstall deletes leftover `~/.config/cmux/sidebars/herdr.{js,swift}` if
  present. Doctor treats a missing custom sidebar as expected; a leftover
  copy is reported softly as demoted.

## [0.6.0] - 2026-09-02

### Changed

- Sidebar is native cmux chrome named **Herdr**: `sidebars/herdr.js` is the
  product path (live drag; `.js` wins over `.swift`), with `herdr.swift` as
  fallback. No iframe, no bridge caption, no CLI cheat-sheet, no dual-hierarchy
  explainer. Click a row to `workspace.select` / `surface.focus`. Status chips
  show working/idle/done, not raw `herdr:` keys.
- `cmux-herdr watch` defaults to pane mirroring (layout, focus, order, prune).
  `--pills-only` writes status chips without projecting tabs.
- README marketing is "cmux as the UI, Herdr as the engine". Generated
  screenshot PNGs are removed (no fake cmux chrome).

### Added

- Interpreted JS sidebar with `Reorderable`, context menus, and Ghostty/cmux
  theme tokens.

## [0.5.0] - 2026-09-02

### Added

- Official cmux plugin-manager install for the CLI: `cmux-plugin.toml`
  (`kind = "sidebar"`, name `cmux-herdr`). `[build]` is chmod +x, not Cargo.
- README hero and feature screenshots of the **native** `herdr` sidebar
  (lab/mock chrome only).
- Interpreted Swift sidebar binds live cmux workspaces with `Reorderable`
  mouse/DnD and Ghostty/cmux theme tokens (no invented team; #63).

### Changed

- Documented user UI is the native sidebar: copy `sidebars/herdr.swift` to
  `~/.config/cmux/sidebars`, then `cmux sidebar validate herdr` /
  `cmux sidebar open herdr`. Plugin-manager install/use/update/remove remains
  the official CLI checkout. `./scripts/install.sh` is contributor/dev.
- Refreshed upstream status after Austin's direct #8736 follow-up fixes and
  the latest current-`main` update of #10045.

## [0.4.0] - 2026-08-22

### Changed

- README and German overview rewritten as a plugin landing page (product title,
  tagline, features, install-first, command table). CLI `--help` now describes
  **cmux-herdr** as the cmux plugin for Herdr.
- Plugin sizing yields while native cmux owns size authority.

### Added

- README release badge and public topic links (`cmux`, `herdr`, `tmux`,
  `macos`, `cli`, `python`).
- GitHub Actions `release` workflow: pushing tag `vX.Y.Z` runs tests and
  publishes the GitHub Release from `CHANGELOG.md`
  (`scripts/changelog_notes.py`).
- Native-writer lease heartbeat, title-lock diagnostics, and association-wire
  status in `doctor`.

### Fixed

- LaunchAgent installs replace only their own plist, so alternate `HOME`
  installs and tests cannot evict a same-label real-user agent.

## [0.3.4] - 2026-08-19

### Added

- MIT [LICENSE](LICENSE), [CONTRIBUTING.md](CONTRIBUTING.md),
  [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md), [SECURITY.md](SECURITY.md),
  GitHub issue/PR templates, [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md),
  maintainer guides ([docs/MAINTAINING.md](docs/MAINTAINING.md),
  [docs/de/](docs/de/)), and GitHub Actions CI (`./scripts/test.sh` on
  Python 3.10–3.13).
- `cmux-herdr lease` — inspect plugin↔native writer lease.
- Ignore local agent scratch (`.tmp-push/`, `proto_push_args.json`).

### Security

- Removed `docs/live-env-snapshot.txt` (local hostname, home-relative paths,
  and workspace titles). No API keys were in that file. Older git history
  still contains the blob; `main` is not force-pushed.
- Removed `.github/workflows/auto-squash-merge.yml`, which could squash-merge
  public PRs whose branch name started with `cursor/`.

## [0.3.3] - 2026-08-19

### Added

- **CLI fallbacks for Herdr-beyond-tmux verbs** when the Unix socket is down
  (`agent.explain` / `agent.view.*` / `pane.process_info` / release+authority /
  window title / manifests / worktrees / workspace-move).
- Behavioral CLI tests for those wrappers.

- **Herdr-beyond-tmux CLI**: first-class commands for agent explain/view,
  process-info, release/clear agent authority, window title, layout-apply,
  agent manifests, worktrees, and workspace-move. See
  [docs/upstream/HERDR_BEYOND_TMUX.md](./docs/upstream/HERDR_BEYOND_TMUX.md).

### Fixed

- **Herdr 0.8 CLI/RPC contract**: CLI fallbacks now match
  [herdr.dev/docs/cli-reference](https://herdr.dev/docs/cli-reference/).
  `pane.send_text` maps to `herdr pane send-text` (not `pane send --text`).
  `agent.start` is `herdr agent start <name> --kind <kind> --pane <id>`.
  Waits use `--timeout` (not `--timeout-ms`); `pane.wait_for_output` is
  `herdr pane wait-output --match/--regex`; `pane.swap` uses
  `--source-pane`/`--target-pane`; `pane.current`/`neighbor`/`layout` use
  `--pane`/`--current`. `agent.wait` no longer invents `--until done`.
  `agent.prompt --until` implies `--wait`. `tab.move` is socket-only.
  Plugin `--force` on `close-pane` stays a local busy-close gate and is
  not forwarded to Herdr. Named keys (`C-Up`, `F5`) encode to Herdr
  combos (`ctrl+up`, `f5`). User focus talks `agent.focus`; there is no
  `pane.focus` method (the event remains `pane.focused`). Claim-size
  `pane.resize --cols/--rows` is not a Herdr API and is no longer sent.
  README Herdr repo URL is `herdrdev/herdr`. Default cmux socket example
  is `/tmp/cmux.sock`.

See git history on `main` for the remainder of the 0.3.3 / 0.2.0 / 0.1.0 notes;
this 0.6.1 patch demotes the custom sidebar and does not rewrite older entries.

[Unreleased]: https://github.com/RaviTharuma/cmux-herdr/compare/v0.6.1...HEAD
[0.6.1]: https://github.com/RaviTharuma/cmux-herdr/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/RaviTharuma/cmux-herdr/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/RaviTharuma/cmux-herdr/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/RaviTharuma/cmux-herdr/compare/v0.3.4...v0.4.0
[0.3.4]: https://github.com/RaviTharuma/cmux-herdr/releases/tag/v0.3.4
[0.3.3]: https://github.com/RaviTharuma/cmux-herdr/releases/tag/v0.3.3
[0.2.0]: https://github.com/RaviTharuma/cmux-herdr/releases/tag/v0.2.0
[0.1.0]: https://github.com/RaviTharuma/cmux-herdr/releases/tag/v0.1.0
