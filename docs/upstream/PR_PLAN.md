# Upstream PR plan: native nested topology

## Principles

- Native is the primary path; the external bridge remains fallback and a fixture source.
- Keep provider protocol code out of `Workspace` and SwiftUI views.
- Land read-only topology before mutations.
- Add behavior through capability-gated, additive APIs.
- Do not combine this work with cmux pane/layout refactors.

## PR 0 — Contract confirmation (Herdr, if needed)

**Goal:** remove identity and negotiation ambiguity before unattended restore or broad actions.

Propose small Herdr API additions, independently reviewable:

1. `ping` returns a random, stable-for-server-lifetime `instance_id`.
2. Capabilities advertise method or semantic tokens needed by cmux, e.g. `session.snapshot.v1`, `events.subscribe.v1`, `topology.focus.v1`, rather than requiring version guesses.
3. Document event ordering and whether the subscription acknowledgement creates a race-free snapshot boundary. If it does not, add a revision/cursor or document snapshot-after-subscribe reconciliation.

cmux may begin read-only prototype work against protocol 17, but **must not** claim durable auto-reattach based only on socket path/version.

**Herdr tests:** ping serialization, uniqueness/lifetime, schema output, backward-compatible decode, event boundary/cursor semantics.

## PR 1 — Provider-neutral model and IDs

**Goal:** introduce no UI and no socket connection, only domain types and reducers.

Suggested new area: `Packages/macOS/CmuxNestedTopology` (name subject to maintainer preference).

Add:

- `NestedProviderKind`
- `NestedProviderInstanceID`
- `NestedNodeKind`
- `NestedNodeID` compound value
- immutable `NestedTopologySnapshot`
- workspace/tab/pane/agent node values
- `NestedTopologyEvent`
- pure reducer with validation and limits
- capability set and connection-state values

Rules:

- provider raw IDs stay opaque strings;
- parent relationships and kind are validated;
- duplicate IDs, cycles, excessive depth/count, invalid status, and oversized fields fail deterministically;
- equality/diffing does not depend on display title;
- public encoding is structured and versioned.

**Tests:** collision across provider instances, same raw ID across kinds, malformed parent, duplicate event, close cascade, focus invariants, bounds, deterministic ordering, Codable round trips.

## PR 2 — Read-only Herdr socket adapter

**Goal:** connect, negotiate, snapshot, subscribe, and resync without touching app UI.

Add a provider-neutral protocol such as:

```swift
protocol NestedTopologyProviderClient: Sendable {
    func handshake() async throws -> ProviderHandshake
    func snapshot() async throws -> NestedTopologySnapshot
    func events() -> AsyncThrowingStream<NestedTopologyEvent, Error>
}
```

Implement `HerdrNestedTopologyClient` using the documented newline-delimited JSON socket API:

- typed request IDs;
- `ping` compatibility validation;
- `session.snapshot` decoding;
- `events.subscribe` decoding;
- cancellation closes the descriptor promptly;
- bounded line/snapshot/event sizes and deadlines;
- reconnect backoff and mandatory full resnapshot;
- no shell, no `herdr` CLI dependency.

Keep protocol 17 adaptation in one compatibility file generated or checked against Herdr’s published schema (`herdr api schema --json`). Unknown fields are tolerated; missing required fields are errors.

**Tests:** golden protocol-17 snapshot/events, fragmented reads, multiple lines/read, malformed JSON, wrong response ID, error response, timeout, EOF, cancellation, oversized line, unsupported protocol, reconnect/resnapshot. Use a temporary Unix socket fake server; do not require a running Herdr in unit tests.

## PR 3 — Secure attachment lifecycle

**Goal:** bind one provider connection to one host cmux terminal surface.

Add an attachment coordinator/store owned at app/window scope, not by a SwiftUI row. A record contains:

- host workspace ID and stable surface identity;
- provider kind;
- canonical socket location plus pre-connect file identity;
- provider instance ID / connection generation;
- negotiated capabilities;
- state (`disconnected`, `connecting`, `live`, `stale`, `incompatible`, `rejected`).

Initial attachment should be user-confirmed through UI or an authenticated cmux control-socket method. Environment/OSC discovery may prefill a proposal but must not establish authority by itself.

Security checks:

- local Unix socket only;
- `lstat`/no symlink confusion, current UID ownership, restrictive mode;
- identity check around connect to reduce path-swap races;
- per-attachment limits and cancellation;
- strings sanitized before publication;
- no socket path or payload in default telemetry.

Integrate surface close/move lifecycle. Moving a host surface preserves its stable-surface attachment; closing it detaches without invoking `server.stop` or child closes.

**Tests:** opt-in, wrong UID/mode/type, symlink/path replacement, duplicate attachment, host close, host move, app/window teardown, reconnect cancellation, two sessions with identical raw IDs.

## PR 4 — Read UI and control-socket parity

**Goal:** make native topology useful without mutation risk.

UI:

- add an expandable provider-owned subtree beneath the host terminal row (or the closest sidebar hierarchy accepted by maintainers);
- show workspace/tab/pane labels, focused state, agent identity/status, disconnected/stale state;
- preserve existing workspace/surface selection and accessibility behavior;
- virtual children never enter Bonsplit or `Workspace.panels`.

Control socket:

- advertise semantic capability such as `nested_topology.read.v1` through `system.capabilities`;
- extend `system.tree` only behind `include_nested: true`, or add `nested.topology.list`; keep default response byte-for-byte compatible where practical;
- include structured compound ID, node kind, provider kind/instance, host stable surface ID, parent ID, state, and bounded display metadata.

Follow existing cmux dispatch architecture: add the methods to the central capability list and `ControlCommandExecutionPolicy`, then route through a narrow context/witness rather than putting protocol I/O on the main actor.

**Tests:** capability list, default tree compatibility, nested tree JSON, ordering, stale state, accessibility labels, selection/focus regressions, large-tree responsiveness, existing `system.tree` suites.

## PR 5 — Capability-gated focus and safe actions

**Goal:** add mutations incrementally, starting with focus.

1. Add `nested.node.focus` with a structured target and expected provider instance/generation.
2. Resolve host surface and current attachment atomically immediately before send.
3. Reject stale generation, wrong host, wrong kind, unsupported capability, or disconnected provider.
4. Forward typed JSON and refresh/reconcile from provider events; do not optimistically invent topology.

Follow-up action groups should be separate PRs:

- rename;
- read/prompt/send input;
- split/move/resize/layout;
- close (with confirmation and explicit semantics).

Never fall back to synthesized keystrokes or shell commands when a method is unavailable.

**Tests:** allowed focus, capability absent, stale instance, node closed during action, response/event race, duplicate raw IDs in two attachments, authorization from control socket, destructive confirmation.

## PR 6 — Restore semantics

**Goal:** persist attachment intent safely using existing session snapshots.

Extend `SessionPanelSnapshot` (or a narrowly nested optional value) with a versioned attachment descriptor. Preserve Codable defaults so older snapshots decode. Persist:

- provider kind;
- opt-in/reattach policy;
- non-secret endpoint locator if approved;
- last verified provider instance ID as a comparison value;
- no nested node snapshot, output, token, or bearer credential.

In `Workspace.restoreSessionSnapshot`, defer provider reattachment until the terminal panel and stable surface identity exist. Re-run all security and compatibility checks, compare provider identity, and fetch a new snapshot. If identity proof is unavailable or mismatched, restore as disconnected and require confirmation.

**Tests:** old snapshot decode, new round trip, no secret/payload persistence, missing provider, changed socket identity, changed provider instance, successful fresh snapshot, restore cancellation when panel closes, crash-recovery/closed-item restore behavior.

## Optional PR 7 — Discovery descriptor

Only after manual attachment is stable, define a terminal-to-host descriptor. Prefer a structured, bounded OSC message carrying provider kind and endpoint hint. It must:

- be disabled or confirmation-required by default;
- bind to the emitting surface;
- contain no authority by itself;
- reject control characters/oversize/unknown versions;
- never cause automatic command execution.

Do not infer permanent parentage merely from inherited `CMUX_*`/`HERDR_*` environment values.

## Files likely involved

Exact placement should follow maintainer feedback, but current source boundaries indicate:

- new package/provider model and adapter files;
- `Sources/Workspace.swift` only for narrow snapshot/restore hooks;
- `Sources/SessionPersistence.swift` for optional attachment persistence;
- sidebar workspace/surface row model/view files for presentation;
- `Sources/TerminalController+ControlSystemContext.swift` or a new nested context for public reads/actions;
- `Sources/TerminalController.swift` capability/method dispatch;
- `Packages/macOS/CmuxControlSocket/.../ControlCommandExecutionPolicy.swift` for lane and authorization policy;
- corresponding `cmuxTests`, package tests, and targeted UI tests.

Avoid embedding Herdr protocol structs in `Workspace`, `TerminalController.swift`, or sidebar views.

## Verification per PR

Run the smallest package/unit suite first, then targeted cmux tests. Before merging each app-integrated PR:

```bash
# Illustrative; use the repository's current documented scheme/destination.
xcodebuild test -workspace cmux.xcworkspace -scheme cmux -only-testing:cmuxTests/<TargetedSuite>
```

Also run:

- package tests for the new model/adapter package;
- `TerminalControllerSocketSecurityTests` and capability-policy tests when socket methods change;
- session snapshot/restore suites when persistence changes;
- sidebar/navigation UI tests when rows change;
- a manual matrix: no Herdr, supported Herdr, unsupported protocol, disconnect/restart, two providers, cmux restart.

## Rollout

1. Hide behind a beta setting/feature flag.
2. Ship read-only telemetry for connection errors and protocol versions, redacting paths/IDs.
3. Enable manual attachment by default after crash/performance review.
4. Enable discovery proposals later; do not silently autoattach until Herdr exposes durable instance identity and the restore threat model is satisfied.
5. Keep the status-pill plugin documented as fallback for older cmux/Herdr versions.

## Explicitly deferred

- generic third-party executable plugins;
- remote/network provider transport;
- mobile-issued nested mutations;
- automatic server startup/shutdown;
- migration of Herdr panes into native cmux PTYs;
- unified persistence of provider processes.
