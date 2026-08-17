> **Canonical source of truth is GitHub**, not this draft.
>
> - Community poll: https://github.com/manaflow-ai/cmux/discussions/10106
> - Full nested topology: https://github.com/manaflow-ai/cmux/issues/8737
> - Nested topology v1: https://github.com/manaflow-ai/cmux/pull/10045
> - Hidden compat MVP: https://github.com/manaflow-ai/cmux/pull/8736
> - Update GitHub first when trackers move; then sync this file if still useful.
>
> This file is a paste-ready / design package kept for dual-path history. Prefer the live issue/PR over local text if they diverge.

# Design: native nested topology providers in cmux

## Status

Proposed upstream architecture. Herdr is the first adapter. The existing user-controlled status bridge remains a stopgap and compatibility path.

Source review baseline:

- cmux `f616cecf3b9e564e49bcd4ac39e2722c1553e6e6`
- Herdr `44f2211608618e56d4bd80ef9c2bac4d9c8be4d2` (API protocol 17)

## Goals

1. Represent a nested provider’s workspace/tab/pane/agent topology beneath the cmux terminal that hosts it (sidebar navigator — #10045).
2. Mirror Herdr tabs/panes into real cmux tabs/Bonsplit/Ghostty surfaces the same way ssh-tmux does (`RemoteHerdrWindowMirror` — PR7).
3. Update from provider events, with snapshot-based recovery.
4. Route only negotiated, explicitly authorized actions to the correct live provider.
5. Restore attachment intent without persisting or replaying stale provider state.
6. Generalize the model enough for another nested mux without turning cmux into an arbitrary plugin host.

## Non-goals

- Parsing terminal screen contents to reconstruct topology.
- Executing provider CLIs or arbitrary provider-supplied code.
- Managing Herdr process lifetime.
- Remote provider transport in v1.
- Replacing ssh-tmux’s feed-forward sizing with a feedback controller.

## Existing architecture and constraints

cmux’s public tree is assembled from app/window `TabManager` state, `Workspace`, Bonsplit pane summaries, and panel/surface summaries in `TerminalController+ControlSystemContext.swift`. `Workspace.sessionSnapshot` persists `SessionWorkspaceSnapshot`, panel snapshots, layout, status, logs, and related state through `SessionPersistence.swift`. Control-socket methods are centrally advertised and assigned execution/authorization policy.

That architecture is why **sidebar v1** must not pretend a Herdr pane *is* a cmux pane: a virtual descendant has no `Panel` / Ghostty `Surface` / Bonsplit `PaneID`.

**PR7 (`RemoteHerdrWindowMirror`) is the exception that ssh-tmux already made for tmux:** the inner mux remains the grid authority, but cmux *does* create real `TerminalPanel`s whose I/O is bound to inner pane ids, and imposes the inner layout tree on Bonsplit. Herdr still owns the PTY processes; cmux owns the viewer surfaces. Same split as remote tmux.

Do not invent a third model. Copy `RemoteTmuxWindowMirror`. Canonical mapping: [TMUX_PARITY.md](./TMUX_PARITY.md).

Herdr’s separate local API is newline-delimited JSON. `session.snapshot` already provides a coherent topology and focused IDs. `events.subscribe` provides typed events. The API socket is restricted to mode `0600`, but has no documented application-level bearer authentication. `ping` reports version, protocol, and limited capabilities.

## High-level architecture

```text
                       cmux app/window scope
┌────────────────────────────────────────────────────────────────┐
│ NestedTopologyAttachmentCoordinator                            │
│  host stable surface ID → Attachment                           │
│      ├─ security-validated endpoint                            │
│      ├─ provider instance/generation + capabilities            │
│      ├─ NestedTopologyStore (snapshot + reducer)               │
│      └─ NestedTopologyProviderClient                           │
│             └─ HerdrNestedTopologyClient ── Unix socket        │
│                                                                │
│ Sidebar projection        Control-socket projection            │
│ (virtual descendants)     (capability-gated structured JSON)   │
└────────────────────────────────────────────────────────────────┘
```

### Component responsibilities

**Provider client**

- protocol encoding/decoding;
- handshake and compatibility;
- request correlation;
- event stream and cancellation;
- transport limits/timeouts;
- no UI or cmux workspace knowledge.

**Attachment coordinator**

- security validation and user consent;
- maps a host stable surface to a live provider;
- connection/reconnect lifecycle;
- owns task cancellation;
- verifies current attachment before actions;
- publishes connection and topology state off the main actor.

**Topology store/reducer**

- provider-neutral immutable state;
- validates IDs/parents/order/focus;
- applies events serially;
- marks stale and requests resync on inconsistency;
- computes bounded diffs for observers.

**UI/control projections**

- read snapshots only;
- publish provider-owned virtual rows;
- never perform socket I/O while holding main-actor UI state;
- dispatch actions back through the coordinator.

## Data model

```swift
struct NestedNodeID: Hashable, Codable, Sendable {
    let version: UInt8
    let providerKind: NestedProviderKind
    let providerInstanceID: String
    let kind: NestedNodeKind
    let rawID: String
}

struct NestedTopologySnapshot: Sendable {
    let attachmentID: UUID
    let hostStableSurfaceID: UUID
    let provider: ProviderHandshake
    let workspaces: [NestedWorkspaceNode]
    let tabs: [NestedTabNode]
    let panes: [NestedPaneNode]
    let agents: [NestedAgentNode]
    let focus: NestedFocus
}
```

Raw IDs are never globally keyed. Internal dictionaries key by full `NestedNodeID`. Parent references are typed (`tab.workspaceID`, `pane.tabID`, `agent.paneID`) and must resolve within the same provider instance.

### Host identity

Use the terminal panel’s stable surface identity for durable parentage, with current workspace/panel IDs as runtime lookup aids. The attachment follows an existing surface move. If the underlying terminal is replaced rather than moved, it receives no attachment unless explicitly restored/rebound.

### Provider instance identity

A socket path is an endpoint locator, not identity. Ideal identity is a random UUID generated by Herdr at server start and returned by `ping`. Until available:

- assign a random cmux `connectionGeneration` per successful connection;
- pin socket file identity for that connection;
- never automatically treat a post-disconnect endpoint as the same provider for mutating operations;
- allow a fresh read-only snapshot after revalidation;
- require manual confirmation for durable restore/rebind.

Agent `session_id` may identify an agent conversation but is not a provider identity and is optional. It can be represented as bounded metadata, never as the topology primary key.

### Status

Normalize known values to a cmux presentation enum while retaining `providerRawStatus` for forward compatibility:

| Herdr | cmux presentation |
|---|---|
| `working` | working/progress-active |
| `idle` | idle |
| `blocked` | attention/blocked |
| `done` | done |
| unknown future value | unknown, raw retained |

Status changes do not implicitly set outer workspace unread state. Any future attention synthesis needs an explicit policy and tests.

## Herdr adapter

### Handshake

1. Validate endpoint metadata.
2. Connect with a deadline.
3. Send `{ "id": ..., "method": "ping", "params": {} }`.
4. Decode `version`, `protocol`, and capabilities.
5. Select an adapter profile from a tested compatibility table.
6. Request `session.snapshot`.
7. Establish subscriptions and reconcile ordering as described below.

Herdr protocol 17’s capabilities (`live_handoff`, `detached_server_daemon`) do not advertise topology methods. For protocol 17, cmux may ship an explicit tested profile. It must fail closed for unknown incompatible protocols rather than trying method names.

### Snapshot and event ordering

The desired protocol sequence is subscribe-with-cursor, snapshot-at-cursor, then events after cursor. The reviewed Herdr API documents subscription acknowledgement but not a resumable cursor/snapshot boundary. Until clarified, use a conservative reconciliation sequence:

1. open subscription connection;
2. wait for subscription acknowledgement;
3. fetch a snapshot over a separate connection;
4. buffer bounded events received during snapshot;
5. install snapshot, then apply buffered events;
6. if an event cannot be reconciled (e.g. update/close for unknown identity), mark stale and fetch another snapshot.

Because events may be duplicated or race with the snapshot, reducers must be idempotent where semantics allow. Buffer overflow forces resync. Long term, prefer a provider revision/cursor.

Subscribe to the minimum set needed: workspace/tab/pane create, update/rename/move/focus/close; pane agent detection/status/exit; and any explicit agent/session events required by schema. Ignore unknown event types safely.

### I/O behavior

- one JSON object per line;
- UTF-8 only;
- random request IDs and exact response correlation;
- separate or serialized request connections according to the Herdr API contract;
- bounded line (below Herdr’s 1 MiB initial request bound), total snapshot, collection counts, string lengths, and metadata entries;
- connect/request/write-idle deadlines;
- cancellation closes descriptors;
- no retries for mutations unless the operation has an idempotency key/known outcome;
- reconnect reads a full snapshot before publishing live state.

## Attachment and discovery

### Manual attachment (v1)

The safe first flow is a user action on a terminal surface: “Attach nested provider…”. cmux can prefill `$HERDR_SOCKET_PATH` captured from the terminal’s trusted launch environment, but presents the endpoint/provider and requires confirmation. An authenticated cmux socket method may also request attachment to a specific host surface.

Environment values are hints. They can be stale, inherited, or attacker-controlled by a process in the terminal. They do not prove endpoint ownership or that a given Herdr pane belongs to this surface.

### Optional descriptor (later)

A bounded versioned OSC descriptor emitted by the nested client can associate a proposal with the exact emitting surface. It should contain provider kind, socket locator, and non-secret context. Receipt creates an attachment proposal, not authority. Clear/detach semantics are required to avoid stale associations.

### Nested stopgap parent map

The external bridge cannot extend cmux’s tree. To support multiple host surfaces safely while native support rolls out, it may keep a small user-owned state file mapping `(cmux stable surface ID, provider endpoint/instance hint)` to observed Herdr pane/session IDs and status keys. That map should be atomically rewritten, mode `0600`, pruned on observation, and treated as cache only. Native cmux must not import it as authoritative restore state.

## Security model

### Trust boundaries

- Terminal child processes and environment: untrusted hints.
- Provider socket peer: same-user but potentially malicious/compromised.
- Provider topology/display fields: untrusted data.
- cmux attachment coordinator: trusted confused-deputy boundary.
- cmux control-socket caller: governed by existing authorization policy in `CmuxControlSocket`.

### Endpoint checks

Before connect:

- endpoint is local Unix-domain only;
- resolve allowed path form without following an attacker-controlled final symlink;
- `lstat` identifies a socket owned by `geteuid()`;
- permissions do not grant group/other access (Herdr expected `0600`);
- capture device/inode (and platform-available generation metadata).

After connect, recheck path identity where meaningful. Unix path checks cannot fully authenticate the connected server; durable provider `instance_id` is still required. Do not support elevated/cross-user sockets.

### Authorization and action binding

An action target contains:

```text
host stable surface ID
attachment ID
expected provider instance ID / connection generation
typed NestedNodeID
operation + typed parameters
```

The coordinator resolves all fields against one current immutable attachment snapshot immediately before encoding. Any mismatch rejects the action. This prevents a raw `w2:p1` from selecting another Herdr session after reconnect.

Read and mutation capabilities are separate. The cmux control socket should advertise and authorize nested methods explicitly in `system.capabilities` and `ControlCommandExecutionPolicy`. No provider method passthrough is exposed; cmux supports a fixed allowlist of semantic operations.

### Input and rendering limits

- strip or visibly replace terminal control characters in labels;
- cap every displayed field and metadata map;
- cap nodes per type and total depth;
- rate-limit event application and UI publication;
- never auto-open provider cwd, URL-looking titles, or metadata;
- redact endpoint paths/provider raw payloads from analytics and ordinary logs;
- expose diagnostics only through an explicit, scrubbed export.

## Capability negotiation

There are two capability layers.

### Provider → cmux

Semantic capabilities, preferably advertised by the provider:

- `topology.snapshot.v1`
- `topology.events.v1`
- `topology.focus.v1`
- `topology.rename.v1`
- `pane.input.v1`
- `pane.split.v1`
- `agent.prompt.v1`

For current Herdr, an adapter profile maps tested protocol 17 methods to these capabilities. Unknown protocol does not inherit the profile.

### cmux → clients/UI

`system.capabilities` advertises only features that the cmux build implements, such as:

- `nested_topology.read.v1`
- `nested_topology.attach.v1`
- `nested_topology.focus.v1`

Per-attachment provider capability remains in attachment/tree output, because a cmux build may support focus while a connected provider does not.

UI actions require the intersection:

```text
cmux implementation ∩ control-socket authorization ∩ provider capability ∩ live attachment state
```

## State machine

```text
Detached
  └─ attach request → Validating → Connecting → Negotiating → Syncing → Live
                         │             │             │          │       │
                         └─────────────┴─────────────┴──────────┴→ Failed/Incompatible
Live ─ event inconsistency / EOF → Stale → Backoff → Validating
Any state ─ host close/user detach → Detached (cancel all tasks)
```

Only `Live` accepts mutations. Read projections may expose the last snapshot in `Stale`, visibly marked and for a bounded retention period. Generation changes invalidate outstanding action handles.

## Restore

### Persisted shape

Add an optional, versioned descriptor to the terminal’s `SessionPanelSnapshot` (or equivalent surface-owned persistence):

```swift
struct SessionNestedAttachmentSnapshot: Codable, Sendable {
    var version: Int
    var providerKind: String
    var endpointLocator: String?       // only if policy allows
    var lastProviderInstanceID: String?
    var reattachPolicy: ReattachPolicy // manual or verifiedAutomatic
}
```

Do not persist:

- nested topology snapshot;
- provider output or metadata;
- pending actions;
- connection generation;
- credentials/capability tokens.

### Restore sequence

1. Decode optional data with backward-compatible defaults.
2. Restore cmux workspace/layout/panel as today.
3. Re-adopt/check stable surface identity using existing restoration protections.
4. Start attachment validation only after the terminal exists.
5. Verify endpoint and provider identity.
6. Fetch a fresh provider snapshot and subscribe.
7. Publish live tree, or disconnected state on any mismatch.

Restoring closed items follows the same rules. If stable surface identity cannot be safely re-adopted, do not transfer the attachment silently.

## Public API shape

Prefer additive structured JSON. Either:

- `system.tree` parameter `include_nested: true`, or
- `nested.topology.list` scoped by host surface/workspace/window.

Example node (illustrative, not final wire contract):

```json
{
  "id": {
    "version": 1,
    "provider_kind": "herdr",
    "provider_instance_id": "…",
    "node_kind": "pane",
    "raw_id": "w2:p34"
  },
  "parent_id": { "node_kind": "tab", "raw_id": "w2:t17", "…": "…" },
  "host_surface_id": "<stable UUID>",
  "label": "tests",
  "focused": true,
  "agent": { "kind": "pi", "status": "working" },
  "stale": false
}
```

Default `system.tree` behavior remains compatible. Nested actions accept this structured ID plus expected attachment/provider generation, never a colon-split shorthand.

## Threading and performance

- Socket parsing and topology reduction run outside the main actor.
- Publish coalesced immutable snapshots/diffs to UI at a bounded cadence.
- Do not perform provider I/O inside `TerminalController` main-lane witnesses.
- Use actor isolation (or a serial executor) for each attachment rather than shared mutable dictionaries.
- Limit topology size before creating SwiftUI rows.
- Measure a large fixture (e.g. 20 workspaces/100 tabs/500 panes/500 agents) for decode, reduction, publication, and sidebar expansion.

## Error handling and observability

User-facing states distinguish unsafe endpoint, incompatible protocol, disconnected provider, and malformed provider data. Detailed errors are locally inspectable but scrub socket paths and provider content from telemetry. Counters may include provider kind, protocol number, phase, coarse error class, reconnect count, node-count bucket, and duration.

There is no crash/retry loop on persistent protocol errors. Backoff resets only after a stable live interval or explicit user action.

## Test strategy

### Model/reducer

- compound-ID collision resistance;
- parent/kind validation, cycles, duplicate creates, unknown updates, close cascades;
- focus uniqueness and order changes;
- future statuses/events;
- all resource limits.

### Protocol adapter

- golden `ping`, snapshot, subscription acknowledgement, and event fixtures from protocol 17;
- partial/multiple frames, malformed UTF-8/JSON, wrong IDs, provider errors;
- timeout, cancellation, EOF, oversized input;
- snapshot/event races, buffer overflow, reconnect and full resync.

### Security

- non-socket, symlink, wrong owner, permissive mode, path replacement;
- endpoint reused by a new provider;
- stale action generation and cross-attachment raw-ID collision;
- malicious labels/control characters and large metadata;
- control-socket authorization/capability policy.

### cmux integration

- attach/detach on a real terminal surface;
- host move and close;
- two providers in one workspace;
- sidebar tree and accessibility;
- additive `system.tree` output and advertised capabilities;
- terminal input unaffected by provider failure.

### Restore

- old snapshot decode/new round trip;
- no topology/secrets persisted;
- provider absent, path changed, instance changed, successful verified reattach;
- panel close during restore and closed-item recovery.

### Manual compatibility

Test supported Herdr release(s), unknown/newer protocol, server restart, high event volume, cmux restart, and fallback bridge coexistence. The native attachment should either suppress only its own bridge keys or document duplicate status behavior; it must not delete unrelated workspace statuses.

## Alternatives rejected

**Keep only status pills.** Useful fallback, but cannot provide hierarchy, identity, event ordering, or scoped actions.

**Model inner panes as cmux panels without a window mirror.** Incorrect if done as a sidebar-only identity rewrite. Correct when done as `RemoteHerdrWindowMirror` (PR7), copying ssh-tmux: Herdr owns PTYs; cmux owns viewer surfaces and Bonsplit extents.

**Run `herdr` CLI for every operation.** Adds process/shell complexity, weakens cancellation and typing, and cannot provide a robust event stream.

**Trust inherited environment and socket path.** Both can be stale or attacker-controlled and neither identifies a server instance.

**Persist the last nested tree.** Creates convincing but stale state and risks applying actions to a replacement provider. Fetch live state after restore instead.

## Open upstream questions

1. Will Herdr add a server-lifetime `instance_id` and semantic capability list?
2. What ordering guarantee exists between `events.subscribe` acknowledgement and `session.snapshot`; can Herdr expose a revision/cursor?
3. Which cmux sidebar location best represents descendants of a surface without confusing them with Bonsplit panes?
4. Should the first public API extend `system.tree` or use a separate nested domain?
5. Is endpoint persistence acceptable, or should restore retain only a manual reattach proposal?
