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
