# Native nested-multiplexer topology for Herdr-hosted agents

## Summary

cmux should be able to discover and display the workspace/tab/pane/agent tree of a supported terminal multiplexer running inside a cmux terminal surface, starting with Herdr.

Today, when Herdr runs in a cmux terminal, cmux sees one terminal surface while Herdr owns the actual inner topology:

```text
cmux window → workspace → pane → terminal surface
                                      └─ Herdr workspace → tab → pane → agent
```

This makes inner agents invisible to cmux navigation, status, unread/attention UI, automation, and restore. A user-space bridge can copy agent state into workspace status pills, but cannot provide first-class hierarchy or correctly scoped actions.

The proposed primary path is native, capability-negotiated nested topology. cmux discovers an opted-in provider from the host terminal, connects directly to its local socket, imports a read-only snapshot, subscribes to events, and renders provider-owned virtual descendants beneath the host surface. Mutating actions are forwarded to the provider; cmux does not duplicate PTYs or treat provider panes as cmux-native panes.

## User-visible problem

1. Several Herdr panes and agents appear as one generic cmux terminal.
2. cmux cannot focus a specific inner pane or show its working/idle/blocked/done state.
3. `system.tree`, the sidebar, and agent-oriented automation expose only the outer surface.
4. Titles and status have to be flattened into one workspace, causing collisions and stale state.
5. After cmux restart, there is no explicit model for reattaching the outer surface to the same live nested session.

## Current stopgap

A standalone bridge can poll `herdr pane list` / `agent list` and write namespaced `cmux set-status` and `set-progress` values to the containing workspace. An optional custom sidebar can explain the dual hierarchy. This works without upstream changes and should remain a compatibility fallback.

The stopgap is intentionally not the desired endpoint:

- status pills are a flat projection, not a tree;
- polling loses ordering and can race with close/recreate events;
- focus/split/rename still require a separate Herdr CLI;
- an outer workspace cannot safely infer ownership from inherited environment alone;
- multiple nested sessions in one cmux workspace collide unless the bridge maintains its own parent map.

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

### Lifecycle and restore

- cmux persists only attachment intent and stable binding metadata, never a copy of live inner PTY state.
- On restore, the terminal surface is restored first. cmux then revalidates the socket and provider identity and fetches a fresh snapshot.
- If the provider is gone or its identity changed, cmux does not replay stale actions or attach solely because a path was reused. The saved binding remains disconnected and may be manually reattached.
- Closing a cmux host surface detaches the observer. It must not stop Herdr or close inner panes unless the user invokes a separately confirmed provider action.

## Protocol facts informing the first implementation

At the source revisions reviewed below, Herdr’s newline-delimited local API provides:

- `ping`, returning version, protocol, and server capabilities;
- `session.snapshot`, including focused IDs, workspaces, tabs, panes, layouts, and agents;
- `events.subscribe`, including workspace/tab/pane lifecycle, focus, pane updates, agent detection/status, and related events;
- direct pane and agent methods for focus, rename, input, split, close, prompt, and wait;
- stable opaque string IDs such as `w2`, `w2:t11`, and `w2:p34` within a running provider session.

Herdr protocol 17 exposes feature booleans in `ServerCapabilities`, not a complete method list. The first adapter should therefore use a cmux-owned compatibility table keyed by tested protocol range and degrade to snapshot-only when mutation support is uncertain. A future Herdr capability list can remove that coupling.

## Security requirements

- Opt-in per attachment; no filesystem-wide socket discovery.
- Connect with a native socket client; do not invoke a shell or interpolate IDs/paths into commands.
- Require a local socket owned by the current user and reject unsafe path/file substitutions. Herdr currently creates its API socket mode `0600`; cmux should verify owner and restrictive permissions before connecting.
- Treat all provider strings as untrusted display data: bound sizes and counts, reject malformed UTF-8/JSON, and never interpret titles, cwd, metadata, or IDs as commands or paths to open automatically.
- Apply connection, line, message, snapshot, depth, node-count, and event-rate limits. Reconnect with bounded exponential backoff.
- Keep provider mutation authority scoped to the attached host surface and provider instance. A nested ID alone must never authorize an action.
- Never persist socket credentials or provider output in diagnostics by default.

## Identity requirements

Nested nodes need compound identity, not raw Herdr IDs:

```text
(provider kind, provider instance ID, node kind, provider node ID)
```

The public serialized form should be versioned and opaque (for example a `NestedNodeID` object), not assembled with a delimiter that provider IDs might contain. Every operation additionally carries the host surface’s stable identity and is rejected if the current attachment no longer matches it.

cmux UUIDs remain authoritative for cmux windows/workspaces/panes/surfaces. Herdr IDs remain authoritative inside Herdr. Neither side rewrites the other’s IDs.

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
- [ ] Unit, protocol fixture, integration, restore, security, and UI tests cover the cases above.

## Non-goals for the first change

- General-purpose arbitrary plugin execution inside cmux.
- Reparenting Herdr PTYs into Bonsplit.
- Making cmux responsible for Herdr process/session persistence.
- Remote socket forwarding, cross-user providers, or network providers.
- Removing the existing status-pill bridge.

## Implementation note

This issue should be delivered as reviewable layers: model/IDs, read-only Herdr adapter, attachment lifecycle, UI/read API, then guarded actions and restore. See the accompanying design and PR plan for boundaries and tests.

## Source basis

Prepared against:

- cmux `f616cecf3b9e564e49bcd4ac39e2722c1553e6e6`
- Herdr `44f2211608618e56d4bd80ef9c2bac4d9c8be4d2`

Relevant cmux implementation points include `Workspace`, `TerminalController+ControlSystemContext.swift`, `SessionPersistence.swift`, and `CmuxControlSocket` authorization/execution policy. Relevant Herdr contracts include `docs/next/website/src/content/docs/socket-api.mdx`, `src/api/schema/*`, `src/api/server.rs`, and protocol schema 17.
