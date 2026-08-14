> **Canonical source of truth is GitHub**, not this draft.
>
> - Community poll: https://github.com/manaflow-ai/cmux/discussions/10106
> - Full nested topology: https://github.com/manaflow-ai/cmux/issues/8737
> - Nested topology v1: https://github.com/manaflow-ai/cmux/pull/10045
> - Hidden compat MVP: https://github.com/manaflow-ai/cmux/pull/8736
> - Update GitHub first when trackers move; then sync this file if still useful.
>
> This file is a paste-ready / design package kept for dual-path history. Prefer the live issue/PR over local text if they diverge.

# Native nested-multiplexer topology for Herdr-hosted agents

## Summary

cmux should be able to discover and display the workspace/tab/pane/agent tree of a supported terminal multiplexer running inside a cmux terminal surface, starting with Herdr.

Today, when Herdr runs in a cmux terminal, cmux sees one terminal surface while Herdr owns the actual inner topology:

```text
cmux window → workspace → pane → terminal surface
                                      └─ Herdr workspace → tab → pane → agent
```

This makes inner agents invisible to cmux navigation, status, unread/attention UI, automation, and restore. A user-space bridge can copy agent state into workspace status pills, but cannot provide first-class hierarchy or correctly scoped actions.

**Native wins as the primary path.** cmux should gain capability-negotiated nested topology: discover an opted-in provider from the host terminal, connect directly to its local socket, import a read-only snapshot, subscribe to events, and render provider-owned virtual descendants beneath the host surface. Mutating actions are forwarded to the provider; cmux does not duplicate PTYs or treat provider panes as cmux-native panes.

The existing user-controlled plugin bridge remains a **flexible compatibility route** for older cmux/Herdr builds and for dogfooding production association patterns (parent maps, title locks, heuristic once-only behavior) while native parity lands.

## User-visible problem

1. Several Herdr panes and agents appear as one generic cmux terminal.
2. cmux cannot focus a specific inner pane or show its working/idle/blocked/done state.
3. `system.tree`, the sidebar, and agent-oriented automation expose only the outer surface.
4. Titles and status have to be flattened into one workspace, causing collisions, thrashing, and stale state.
5. After cmux restart, there is no explicit model for reattaching the outer surface to the same live nested session.
6. Heuristic title/parent association, if left unbounded, rewrites labels after the user or native path has already locked them.

## Hybrid strategy (native primary, plugin flexible)

| Path | Role | When |
|------|------|------|
| **Native (primary)** | Provider-owned virtual descendants under the host surface; socket snapshot + events; capability-gated actions | Target end state in cmux, analogous to cmux’s existing nested-tmux awareness where practical |
| **Plugin bridge (compat)** | CLI + poll/watch + `cmux set-status` / `set-progress` + optional sidebar | Works today without upstream merges; remains fallback and a fixture source |

Both paths should share the same production association rules so behavior does not thrash when a user upgrades from plugin-only to native:

1. **Parent map** — track which panes belong to which parent tab/session/workspace.
2. **State file keyed by `pane_id:session_id`** — durable, small, local association record.
3. **Two-pass association** — (pass A) resolve parentage from authoritative provider data / first successful prompt association; (pass B) render titles/status only from the resolved map, skipping repeated heuristics.
4. **Native-title lock** — once a native or user-owned title is authoritative, do not rewrite it on every poll/event.

## Current stopgap

A standalone bridge (`cmux-herdr`) can poll `herdr pane list` / `agent list` and write namespaced `cmux set-status` and `set-progress` values to the containing workspace. An optional custom sidebar can explain the dual hierarchy. This works without upstream changes and should remain a compatibility fallback.

The stopgap is intentionally not the desired endpoint:

- status pills are a flat projection, not a tree;
- polling loses ordering and can race with close/recreate events;
- focus/split/rename still require a separate Herdr CLI;
- an outer workspace cannot safely infer ownership from inherited environment alone;
- multiple nested sessions in one cmux workspace collide unless the bridge maintains its own parent map;
- unbounded title heuristics fight native/user titles and produce flicker.

### Production association pattern the stopgap should follow

Mirror the field-proven title/parent handling used in agent integration state:

- Store a small state file per association, keyed by **`pane_id:session_id`** (filesystem-safe encoding of both parts).
- Use that record to:
  1. **skip heuristics after the first successful prompt/association**;
  2. **respect a native-title lock** without repeatedly rewriting titles;
  3. remember **which panes belong to which parent** across poll cycles.
- Prefer provider snapshot/events over screen scraping whenever available.
- On pane close / session change, clear only the affected keys so the map cannot resurrect stale parents.

This is a bridge/native-shared behavioral contract, not a substitute for compound nested IDs in cmux.

## Proposed behavior

### Detection and attachment

- Attachment is explicit or based on a narrowly defined terminal-produced descriptor; cmux must not scan arbitrary sockets or execute commands inferred from terminal output.
- The descriptor associates one **host cmux surface** with one **provider instance** and includes provider kind, socket path, and protocol/capability information sufficient for a guarded probe.
- cmux performs a read-only handshake first (`ping` for Herdr), validates compatibility, requests `session.snapshot`, then subscribes to topology and agent events.
- Failure, unsupported protocol, or missing capabilities leaves the terminal fully usable and shows at most a non-blocking “nested provider unavailable” state.

### Presentation

- Provider workspaces, tabs, panes, and agents appear as virtual descendants of the host terminal surface.
- Each row is visibly provider-owned. Inner panes are not inserted into Bonsplit and do not claim native cmux pane/surface UUIDs.
- Focus, prompt, send-input, rename, split, and close are shown only when the negotiated provider capabilities support them.
- Agent states map to cmux semantics without discarding the provider’s original value.
- `system.tree` can optionally include the nested subtree with typed, namespaced IDs and a parent host-surface reference.
- Titles and labels follow **provider authority first**, then an explicit user/native lock; automated association never thrash-locks a title on every update.

### Parentage, titles, and two-pass association

Native and plugin paths should both implement a two-pass model that matches production data flow:

**Pass 1 — Associate (once per pane/session generation)**

- Resolve parent tab/workspace from the provider snapshot/event (authoritative).
- If the provider has not yet emitted parentage, allow a single heuristic/prompt-time association.
- Persist the association under `pane_id:session_id` (or the structured native equivalent of that key).
- After the first successful association, **skip further heuristics** for that key.

**Pass 2 — Render / project**

- Drive UI, status pills, and optional outer titles only from the resolved parent map + provider fields.
- If a **native-title lock** is set (user rename, provider terminal title policy, or host surface policy), do not overwrite the title from heuristics or stale polls.
- Diff before write so unchanged titles/status do not produce UI thrash.

Native cmux should keep this state in memory keyed by compound nested IDs, with optional durable attachment intent only (not live topology). The plugin may keep a small on-disk state file because it has no in-process provider connection.

### Lifecycle and restore

- cmux persists only attachment intent and stable binding metadata, never a copy of live inner PTY state.
- On restore, the terminal surface is restored first. cmux then revalidates the socket and provider identity and fetches a fresh snapshot.
- If the provider is gone or its identity changed, cmux does not replay stale actions or attach solely because a path was reused. The saved binding remains disconnected and may be manually reattached.
- Closing a cmux host surface detaches the observer. It must not stop Herdr or close inner panes unless the user invokes a separately confirmed provider action.
- Association caches (`pane_id:session_id`) are invalidated when provider instance identity changes.

## Protocol facts informing the first implementation

At the source revisions reviewed below, Herdr’s newline-delimited local API provides:

- `ping`, returning version, protocol, and server capabilities;
- `session.snapshot`, including focused IDs, workspaces, tabs, panes, layouts, and agents;
- `events.subscribe`, including workspace/tab/pane lifecycle, focus, pane updates, agent detection/status, and related events;
- direct pane and agent methods for focus, rename, input, split, close, prompt, and wait;
- stable opaque string IDs such as `w2`, `w2:t11`, and `w2:p34` within a running provider session.

Herdr protocol 17 exposes feature booleans in `ServerCapabilities`, not a complete method list. The first adapter should therefore use a cmux-owned compatibility table keyed by tested protocol range and degrade to snapshot-only when mutation support is uncertain. A future Herdr capability list can remove that coupling.

Where practical, native Herdr support should aim for **parity with cmux’s existing tmux nested integration** (discover → observe topology → surface tree/status → capability-gated actions), without forcing Herdr panes into Bonsplit/PTY ownership.

## Security requirements

- Opt-in per attachment; no filesystem-wide socket discovery.
- Connect with a native socket client only after ownership/mode/type checks; reject symlink confusion and path-swap races.
- Keep provider mutation authority scoped to the attached host surface and provider instance. A nested ID alone must never authorize an action.
- Never persist socket credentials or provider output in diagnostics by default.
- Association state files (plugin path) must be mode-restricted, path-safe, and free of secrets/payload dumps.

## Identity requirements

Nested nodes need compound identity, not raw Herdr IDs:

```text
(provider kind, provider instance ID, node kind, provider node ID)
```

The public serialized form should be versioned and opaque (for example a `NestedNodeID` object), not assembled with a delimiter that provider IDs might contain. Every operation additionally carries the host surface’s stable identity and is rejected if the current attachment no longer matches it.

cmux UUIDs remain authoritative for cmux windows/workspaces/panes/surfaces. Herdr IDs remain authoritative inside Herdr. Neither side rewrites the other’s IDs.

The plugin’s `pane_id:session_id` state key is an **association cache key only**. It must not be treated as a public nested node ID in `system.tree` or control-socket APIs.

## Acceptance criteria

- [ ] A supported Herdr session can be attached to exactly one cmux host surface without polling a CLI.
- [ ] Initial snapshot and subscribed events produce an ordered nested workspace/tab/pane/agent tree.
- [ ] Focus and status changes are reflected without recreating the host terminal.
- [ ] Supported actions route to the correct provider instance and node; unsupported actions are absent or disabled.
- [ ] Two Herdr providers with identical raw pane IDs do not collide.
- [ ] Disconnect/reconnect and socket-path reuse cannot apply an action to a different provider instance.
- [ ] Restore revalidates and refreshes live state instead of restoring stale inner topology.
- [ ] Provider failure never breaks terminal input or cmux session restoration.
- [ ] `system.capabilities` advertises nested-topology support and `system.tree` remains backward compatible unless nested data is explicitly requested.
- [ ] Parent association is stable across events: panes remain under the correct parent without repeated heuristic rewrites.
- [ ] After first successful association (or native-title lock), titles are not thrashed by later polls/heuristics.
- [ ] Plugin fallback remains usable on older cmux builds and can be disabled when native attachment is live for the same host surface (no double writers).
- [ ] Unit, protocol fixture, integration, restore, security, and UI tests cover the cases above.

## Non-goals (v1)

- Treating Herdr panes as native Bonsplit panes / Ghostty surfaces.
- Remote provider transport.
- Generic third-party executable plugins inside cmux.
- Automatic Herdr process lifecycle management.
- Unbounded screen-scraping as the primary topology source.

## Source basis

Prepared against:

- cmux `f616cecf3b9e564e49bcd4ac39e2722c1553e6e6`
- Herdr `44f2211608618e56d4bd80ef9c2bac4d9c8be4d2`

Relevant cmux implementation points include `Workspace`, `TerminalController+ControlSystemContext.swift`, `SessionPersistence.swift`, and `CmuxControlSocket` authorization/execution policy. Relevant Herdr contracts include `docs/next/website/src/content/docs/socket-api.mdx`, `src/api/schema/*`, `src/api/server.rs`, and protocol schema 17.

Companion materials in this package:

- [DESIGN.md](./DESIGN.md) — architecture and ownership boundaries
- [PARITY_MATRIX.md](./PARITY_MATRIX.md) — stopgap vs native capability matrix
- [PR_PLAN.md](./PR_PLAN.md) — incremental upstream PR sequence


## Related

- [ANNOYANCES.md](./ANNOYANCES.md) — thrash, flakes, and hard-won lessons
