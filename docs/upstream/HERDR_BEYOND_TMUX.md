# Herdr beyond tmux

Features Herdr exposes that **ssh-tmux has no analogue for**, and how this
plugin surfaces them. Tmux parity remains the gold standard for *window
mirror* behavior; these verbs are additive product depth.

Canonical allowlist: `bridge/cmux_herdr_api.py` → `ALLOWED_METHODS`.
CLI entrypoints live in `bin/cmux-herdr`.

## Why this exists

cmux’s native tmux integration mirrors panes, layout, focus, I/O, and
sizing. Herdr also owns **agents**, **manifests**, **worktrees**, and
client chrome titles. Leaving those only behind `cmux-herdr api …` made
the plugin look incomplete next to Herdr’s own CLI even when the
tmux-parity contract was already at its userspace ceiling.

## CLI surface (shipped)

| Command | Herdr RPC | Notes |
|---|---|---|
| `agent-explain <target>` | `agent.explain` | Semantic explain for a pane/agent |
| `agent-view <target> [view] [--clear]` | `agent.view.set` / `clear` | Agent UI view |
| `process-info <pane>` | `pane.process_info` | Foreground process metadata |
| `release-agent <pane>` | `pane.release_agent` | Drop agent binding |
| `clear-agent-authority <pane>` | `pane.clear_agent_authority` | Clear authority metadata |
| `window-title [title] [--clear]` | `client.window_title.*` | Outer Herdr window title |
| `layout-apply --tree JSON [--tab]` | `layout.apply` | Push a layout tree |
| `manifests [--reload]` | `server.agent_manifests` / `reload_agent_manifests` | Agent kind catalog |
| `worktree list\|create\|open\|remove` | `worktree.*` | Provider worktrees |
| `workspace-move <id> [--index] [--block]` | `workspace.move` / `move_block` | Workspace strip / blocks |

Already shipped earlier (also Herdr-only vs tmux): `start-agent`,
`rename-agent`, `agent-prompt`, `agent-wait`, `notify`, `wait-output`,
status pills from `agent_status`, busy-close confirmation.

## Transport

Socket-first via `HerdrApi`. When the Unix socket is down, these verbs fall
back to documented `herdr` CLI argv (`build_cli_argv`). `layout.apply` stays
socket-only (no stable CLI tree passthrough).

## Still via `api` only (on purpose)

| Method | Why no dedicated CLI yet |
|---|---|
| `pane.report_agent*` / `pane.report_metadata` | Provider → host reporting; not a user verb |
| `integration.install` / `uninstall` | Dangerous; keep behind explicit `api` |
| `popup.close` | Niche; `api popup.close` is enough |
| `server.reload_config` | Ops; prefer Herdr’s own CLI for host ops |
| `events.*` | Owned by `watch` / SessionHost pump |

## Native path

Native `#10045` / PR7 must keep the same lease files as
`bridge/cmux_herdr_handoff.py` (`RemoteHerdrHandoff` twin). Agent chrome
(status / explain / manifests) belongs in the nested sidebar navigator,
not only in the window mirror.

## Honesty

These commands do **not** claim Ghostty PTY theft or Bonsplit ownership.
They close the Herdr-specific CLI gap; tmux-parity ceilings in
[TMUX_PARITY.md](./TMUX_PARITY.md) and [ERRORS_AND_LACKINGS.md](./ERRORS_AND_LACKINGS.md)
still apply.
