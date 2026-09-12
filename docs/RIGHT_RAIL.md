# Right-rail Herdr projection

**cmux-herdr** projects nested Herdr agents into cmux’s existing **right rail**
surfaces. It does not invent a Herdr left sidebar or a second multiplexer UI.

Left sidebar stays workspaces/machines navigation (including future nested
Herdr workspaces under a machine). Richer agent UI belongs on the right rail.

## Moshi inspiration (keep vs discard)

Moshi Desktop deep-integrates Herdr through loopback web APIs
(`GET /v1/muxes`, `/v1/workspaces`, panes, focus, transcripts, live PTY,
websocket watch). Useful **concepts** to keep:

| Keep | Why |
|---|---|
| Single normalized agent list | One row per agent identity; no dupes across surfaces |
| Project into host chrome | Feel like built-in cmux-tmux, not a foreign plugin |
| Live watch | `watch` / `sync` refresh without a second scrape |
| Focus jump | `focus-agent` / `focus-pane` CLI verbs |
| Dedup key = pane identity | Same `herdr:<pane_id>` as status pills |

| Discard | Why |
|---|---|
| Moshi Chat View | Foreign to cmux chrome |
| Diffs / web previews | Not cmux right-rail surfaces |
| Phone APNs | Out of scope for this plugin |
| Loopback web gateway / Moshi chrome | cmux already owns the rail enum |
| New `RightSidebarMode.herdr` | Closed enum; do not invent modes |

## Mapping onto cmux right rail

cmux’s closed right-rail modes include roughly
`files | find | vault | sessions | feed | dock | cloud`
([manaflow-ai/cmux#11707](https://github.com/manaflow-ai/cmux/issues/11707)
context). This plugin feeds **existing** modes only:

| Surface | Herdr projection |
|---|---|
| **Agents / sessions** | Deduped agent rows (`status_key = herdr:<pane_id>`), same identity as pills and `sessions` |
| **Vault** | Leave alone (not Herdr). Session paths may appear as metadata only |
| **Feeds** | Optional `feed_events` on `blocked` / `done` transitions; soft-published via `cmux log` when available |
| **Dock** | Quick actions as **documented CLI verbs** (`focus-agent`, `attach-pane`, `sessions`) — no foreign Dock chrome |

Avoid:

```bash
cmux sidebar open herdr
cmux sidebar select herdr
cmux right-sidebar set custom herdr
```

Prefer:

```bash
cmux-herdr rail --json
cmux-herdr agents --rail
cmux right-sidebar set sessions   # when host supports it
cmux right-sidebar set feed
cmux right-sidebar set dock
```

## CLI

```bash
cmux-herdr rail                 # human summary + write XDG snapshot
cmux-herdr rail --json          # full payload
cmux-herdr rail --no-write      # project without persisting
cmux-herdr rail --read          # print last snapshot for this fingerprint
cmux-herdr agents --rail        # same payload as rail --json
cmux-herdr notify TITLE [--body TEXT]   # Herdr notification.show
# Optional: cmux notify … when that verb exists on PATH (doctor reports it)
```

`sync` and `watch` also refresh the rail snapshot so a future native panel
(or today’s Agents/sessions surface) can read without a second Herdr scrape.

## Snapshot file

Under `$XDG_STATE_HOME/cmux-herdr/` (default `~/.local/state/cmux-herdr/`):

```text
rail-<fingerprint>.json
```

Fingerprint matches parent/association keys (`CMUX_SURFACE_ID` +
`HERDR_SOCKET_PATH` + optional pid/workspace). Schema:

- `method`: `cmux.herdr.rail`
- `schema_version`: `1`
- `agents[]`: rows with `status_key` / `pane_id` / status style
- `modes`: `["sessions", "feed", "dock"]`
- `feed_events`, `dock_actions`, `sessions`, `dedupe`

## Extension hooks still missing upstream

Until cmux exposes a stable provider API for right-rail rows:

1. No first-class “inject agent rows into Sessions” plugin hook
2. No typed Feed event bus for Herdr (we soft-log transitions)
3. No Dock action registration from plugins (CLI verbs only)
4. Native Sidebar issue #75 remains blocked for left nesting
5. Native Herdr attach / TTY takeover remains roadmap (#8736 / #10045)

## Next UI steps

1. Host reads `rail-<fp>.json` (or `cmux-herdr rail --json`) into Sessions
2. Wire `feed_events` into `cmux right-sidebar set feed` when the host API is stable
3. Bind Dock buttons to the documented CLI verbs in `dock_actions`
4. Keep left sidebar free of Herdr chrome; nest workspaces under machines separately
5. Never duplicate agents already shown via `herdr:*` pills / sessions

See also: [ARCHITECTURE.md](ARCHITECTURE.md), [PLUGIN_DESIGN.md](PLUGIN_DESIGN.md),
sidebar philosophy in the [README](../README.md#sidebar-philosophy-cmux-native).
