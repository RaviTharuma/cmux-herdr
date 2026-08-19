# Herdr nested in cmux — synthesis

This is the dual-hierarchy this plugin exists to bridge. Official products do
not treat each other as first-class: cmux's compare page says a practical
pairing is **running Herdr inside a cmux terminal pane**; Herdr's docs never
mention cmux. Native nested topology is still upstream (cmux discussion #10106,
issue #8737, PRs #8736 / #10045, window-mirror engine).

## Two trees at once

When Herdr runs in a cmux surface, cmux sees **one** terminal surface. The real
agent tabs/panes live **inside** Herdr.

```
cmux window
  └── cmux workspace          ← sidebar row, CMUX_WORKSPACE_ID
        └── cmux pane
              └── cmux surface  ← CMUX_SURFACE_ID, one Ghostty PTY
                    └── herdr client (TUI)
                          └── herdr session/server (HERDR_SOCKET_PATH)
                                └── herdr workspace (w2)
                                      └── herdr tab (w2:t11)
                                            └── herdr pane (w2:p34) + agent_status
```

IDs are different namespaces (cmux UUIDs/refs vs Herdr `w2:p34`). Never assume
they line up without the association cache.

The concept map already in this repo (`mapping/concept-map.md`) is still
correct. Extra official detail from this scrape:

- cmux "workspace" is the **sidebar tab**; Herdr "workspace" is a **project
  container inside the mux**. Mapping them 1:1 is wrong.
- cmux "tab" in the UI often means workspace; a **surface** is the tab-inside-pane.
  Herdr "tab" is a layout of panes (tmux-window analogue).
- cmux remote-tmux maps tmux session→workspace, window→tab, pane→split.
  This plugin's `mirror --tmux-parity` is the Herdr analogue of that contract,
  but it cannot steal Herdr PTYs into Ghostty (native PR7).

## Control planes

| Job | Use |
|---|---|
| Outer tree | `cmux tree` / `cmux identify --json` |
| Inner tree | `herdr pane list` / `cmux-herdr tree` |
| Outer status pills | `cmux set-status` / `set-progress` (plugin `sync`/`watch`) |
| Inner agent state | Herdr `agent_status` + `pane report-agent` |
| Follow one inner pane | `cmux-herdr attach-pane` (`herdr pane read` + send + SIGWINCH) |
| Native remote tmux | `cmux ssh-tmux` (tmux only, beta) |
| Native Herdr in Ghostty | **not shipped**; plugin followers + upstream PRs |

Both CLIs speak JSON-RPC over a Unix socket. The envelopes are similar
(`id`/`method`/`params`) but **method names and IDs are not interchangeable**.
`herdr api` / `cmux rpc` stay on their own sockets.

Fingerprint required for plugin auto-resolve (do not guess a host):

- `CMUX_SURFACE_ID`
- `HERDR_SOCKET_PATH` (existing file)
- optional `HERDR_SERVER_PID`, `HERDR_WORKSPACE_ID`

## Status mapping this plugin already implements

Herdr `working|idle|done|blocked|unknown` → cmux pills (`herdr:<pane_id>`),
colors/icons as in the repo README. Sync must prune stale `herdr:*` keys and
leave unrelated cmux status alone.

## Restore: different products, different guarantees

- **Herdr detach:** processes stay alive (strongest path). Restart needs snapshot
  + optional pane history + native agent resume tokens.
- **cmux relaunch:** layout/metadata/browser; processes generally do not;
  agents resume via **cmux hooks**, not Herdr integrations.
- Nested: a cmux restart kills the Herdr **client** surface; the Herdr **server**
  may still be alive on the Mac if it was not in that PTY's process tree — but
  a typical "Herdr launched inside this pane" setup dies with the pane. Treat
  Herdr server lifetime as independent only when it was started as a true
  background session (`herdr` then detach), not as a one-shot TUI in that PTY.

## Official overlap and collisions

- **Both** detect agents and show attention state. Herdr: sidebar rollup +
  `blocked/working/done`. cmux: notification rings, unread jump, OSC 9/99/777,
  `set-status` pills.
- **Both** have plugin/skill systems. Herdr plugins are argv + manifest talking
  to `herdr`. cmux skills teach agents to drive `cmux`. cmux custom sidebars
  cannot shell out to Herdr (interpreter has no child processes) — hence this
  plugin's CLI + optional sidebar that only navigates **outer** workspaces.
- **Both** restore agent sessions via integrations/hooks, with different
  version gates. Do not mix `herdr integration install claude` with
  `cmux hooks setup claude` as if they were one database.
- **License/compare page** claims Herdr is AGPL; Herdr GitHub says Apache-2.0.
  Recorded in SOURCES.md.
- Name collision: Go `soheilhy/cmux` and oh-my-opencode's `CmuxMultiplexer`
  are unrelated (or only loosely analogous).

## DeepWiki vs official docs

DeepWiki is a **code wiki** (Rust/Swift modules, data flow). Official `llms.txt`
docs are the **user/agent contract**. If they disagree, official docs win for
CLI/API; DeepWiki wins for "where in the source is this". DeepWiki cmux index
is ~5 weeks older than this scrape (2026-07-14 vs 2026-08-19).

## What this scrape does not replace

- Live `herdr api schema --json` from an installed binary (protocol can move
  faster than docs).
- Live `cmux capabilities --json` from a running app.
- Upstream cmux Herdr PRs' actual Swift (see `docs/upstream/` in this repo).
- Japanese/Chinese Herdr translations.
