> **Canonical source of truth is GitHub**, not this draft.
>
> - Full nested topology: https://github.com/manaflow-ai/cmux/issues/8737
> - Hidden compat MVP: https://github.com/manaflow-ai/cmux/pull/8736
> - Update GitHub first when trackers move; then sync this file if still useful.
>
> This file is a paste-ready / design package kept for dual-path history. Prefer the live issue/PR over local text if they diverge.

# Upstream PR plan: native nested topology

## Principles

- **Native wins as the primary path**; the external bridge remains fallback, dogfood surface, and a fixture source.
- Keep the **plugin route flexible** while pushing native parity with cmux’s nested-tmux-style awareness where practical.
- Keep provider protocol code out of `Workspace` and SwiftUI views.
- Land read-only topology before mutations.
- Add behavior through capability-gated, additive APIs.
- Do not combine this work with cmux pane/layout refactors.
- Share **two-pass association + native-title lock** semantics between plugin and native so upgrades do not thrash titles/parentage.
- Prefer robust, reviewable implementation edits: shell for directory/scaffold, focused Python/codegen only where it reduces mechanical error; no drive-by refactors.
- In Rust provider/integration code paths, handle poisoned mutexes gracefully with `unwrap_or_default()` (or equivalent recovery) rather than panicking the host.

## Shared behavioral contract (plugin + native)

Before or alongside UI work, encode these rules in tests so both paths stay aligned:

1. **State key** — association records are keyed by `pane_id:session_id` (plugin: small state file; native: in-memory map keyed by compound nested ID + provider instance generation).
2. **Parent map** — each pane records its parent tab/workspace; render uses the map rather than re-inferring parents every tick.
3. **Heuristic once** — skip heuristic association after the first successful prompt/association for that key.
4. **Native-title lock** — when the provider/user/host marks a title authoritative, writers must not overwrite it; always diff before write.
5. **Single writer** — if native attachment is live for a host surface, the plugin must not also project competing `herdr:*` status/title updates for the same logical panes (documented escape hatch to force plugin-only).

## PR 0 — Contract confirmation (Herdr, if needed)

**Goal:** remove identity and negotiation ambiguity before unattended restore or broad actions.

Propose small Herdr API additions, independently reviewable:

1. `ping` returns a random, stable-for-server-lifetime `instance_id`.
2. Capabilities advertise method or semantic tokens needed by cmux, e.g. `session.snapshot.v1`, `events.subscribe.v1`, `topology.focus.v1`, rather than requiring version guesses.
3. Document event ordering and whether the subscription acknowledgement creates a race-free snapshot boundary. If it does not, add a revision/cursor or document snapshot-after-subscribe reconciliation.
4. Document title authority: which fields are user-locked vs provider-default vs terminal OSC, so cmux/plugin can implement native-title lock without guessing.

cmux may begin read-only prototype work against protocol 17, but **must not** claim durable auto-reattach based only on socket path/version.

**Herdr tests:** ping serialization, uniqueness/lifetime, schema output, backward-compatible decode, event boundary/cursor semantics, title-authority fixtures.

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
- association/title-lock value types used by the two-pass renderer (not filesystem I/O)

Rules:

- provider raw IDs stay opaque strings;
- parent relationships and kind are validated;
- duplicate IDs, cycles, excessive depth/count, invalid status, and oversized fields fail deterministically;
- equality/diffing does not depend on display title;
- public encoding is structured and versioned;
- title lock and “heuristic already satisfied” flags are explicit fields, not inferred from string prefixes.

**Tests:** collision across provider instances, same raw ID across kinds, malformed parent, duplicate event, close cascade, focus invariants, bounds, deterministic ordering, Codable round trips, heuristic-once, title-lock suppresses overwrite, parent map stability across reshuffled event batches.

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

On reconnect/resnapshot, drop association entries whose provider instance generation no longer matches; never reuse `pane_id:session_id` locks across instance identities.

**Tests:** golden protocol-17 snapshot/events, fragmented reads, multiple lines/read, malformed JSON, wrong response ID, error response, timeout, EOF, cancellation, oversized line, unsupported protocol, reconnect/resnapshot, association cache invalidation on instance change. Use a temporary Unix socket fake server; do not require a live Herdr in unit tests.

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

When attachment becomes `live`, emit a signal the plugin can use to **disable competing writers** for that host surface (documentation + optional env/file lock). When attachment leaves `live`, plugin fallback may resume.

**Tests:** opt-in, wrong UID/mode/type, symlink/path replacement, duplicate attachment, host close, host move, app/window teardown, reconnect cancellation, two sessions with identical raw IDs, plugin single-writer handoff.

## PR 4 — Read UI and control-socket parity

**Goal:** make native topology useful without mutation risk.

UI:

- add an expandable provider-owned subtree beneath the host terminal row (or the closest sidebar hierarchy accepted by maintainers);
- show workspace/tab/pane labels, focused state, agent identity/status, disconnected/stale state;
- preserve existing workspace/surface selection and accessibility behavior;
- virtual children never enter Bonsplit or `Workspace.panels`;
- apply two-pass render: parent map + locks drive labels; no per-frame heuristic retitling;
- diff-based updates only (no title/status thrash when provider echoes the same value).

Control socket:

- advertise semantic capability such as `nested_topology.read.v1` through `system.capabilities`;
- extend `system.tree` only behind `include_nested: true`, or add `nested.topology.list`; keep default response byte-for-byte compatible where practical;
- include structured compound ID, node kind, provider kind/instance, host stable surface ID, parent ID, state, and bounded display metadata.

Follow existing cmux dispatch architecture: add the methods to the central capability list and `ControlCommandExecutionPolicy`, then route through a narrow context/witness rather than putting protocol I/O on the main actor.

**Tests:** capability list, default tree compatibility, nested tree JSON, ordering, stale state, accessibility labels, selection/focus regressions, large-tree responsiveness, existing `system.tree` suites, no-thrash title updates, parent stability under event reorder.

## PR 5 — Capability-gated focus and safe actions

**Goal:** add mutations incrementally, starting with focus.

1. Add `nested.node.focus` with a structured target and expected provider instance/generation.
2. Resolve host surface and current attachment atomically immediately before send.
3. Reject stale generation, wrong host, wrong kind, unsupported capability, or disconnected provider.
4. Forward typed JSON and refresh/reconcile from provider events; do not optimistically invent topology.

Follow-up action groups should be separate PRs:

- rename (must set/respect native-title lock);
- read/prompt/send input;
- split/move/resize/layout;
- close (with confirmation and explicit semantics).

Never fall back to synthesized keystrokes or shell commands when a method is unavailable.

**Tests:** allowed focus, capability absent, stale instance, node closed during action, response/event race, duplicate raw IDs in two attachments, authorization from control socket, destructive confirmation, rename respects title lock.

## PR 6 — Restore semantics

**Goal:** persist attachment intent safely using existing session snapshots.

Extend `SessionPanelSnapshot` (or a narrowly nested optional value) with a versioned attachment descriptor. Preserve Codable defaults so older snapshots decode. Persist:

- provider kind;
- opt-in/reattach policy;
- non-secret endpoint locator if approved;
- last verified provider instance ID as a comparison value;
- no nested node snapshot, output, token, or bearer credential;
- no plugin association state files inside cmux session snapshots.

In `Workspace.restoreSessionSnapshot`, defer provider reattachment until the terminal panel and stable surface identity exist. Re-run all security and compatibility checks, compare provider identity, and fetch a new snapshot. If identity proof is unavailable or mismatched, restore as disconnected and require confirmation.

**Tests:** old snapshot decode, new round trip, no secret/payload persistence, missing provider, changed socket identity, changed provider instance, successful fresh snapshot, restore cancellation when panel closes, crash-recovery/closed-item restore behavior, association cache not rehydrated from stale session snapshots.

## Optional PR 7 — Discovery descriptor

Only after manual attachment is stable, define a terminal-to-host descriptor. Prefer a structured, bounded OSC message carrying provider kind and endpoint hint. It must:

- be disabled or confirmation-required by default;
- bind to the emitting surface;
- contain no authority by itself;
- reject control characters/oversize/unknown versions;
- never cause automatic command execution.

Do not infer permanent parentage merely from inherited `CMUX_*`/`HERDR_*` environment values. Env may seed pass-1 association proposals only.

## Plugin track (parallel, non-blocking)

These keep the compatibility route honest while native PRs land. They are **not** substitutes for PRs 1–6.

1. **Association state file** — implement `pane_id:session_id` records under a mode-`0700` directory; skip heuristics after first prompt; clear on pane/session death.
2. **Parent map** — track pane → parent tab/workspace across `watch` cycles; use map for status key stability.
3. **Native-title lock** — do not overwrite titles once locked; diff before `set-status` / title writes.
4. **Single-writer guard** — if native nested attachment is detected for the host surface, no-op projection (log once).
5. **Fixtures** — export anonymized snapshot/event JSON from live Herdr for cmux adapter tests.

Suggested verification for the plugin track:

```bash
python -m unittest discover -s bridge -v
python -m unittest discover -s tests -v
# dogfood: watch cycle does not rewrite a locked title; parent map survives pane status flicker
```

## Files likely involved

Exact placement should follow maintainer feedback, but current source boundaries indicate:

- new package/provider model and adapter files;
- `Sources/Workspace.swift` only for narrow snapshot/restore hooks;
- `Sources/SessionPersistence.swift` for optional attachment persistence;
- sidebar workspace/surface row model/view files for presentation;
- `Sources/TerminalController+ControlSystemContext.swift` or a new nested context for public reads/actions;
- `Sources/TerminalController.swift` capability/method dispatch;
- `Packages/macOS/CmuxControlSocket/.../ControlCommandExecutionPolicy.swift` for lane and authorization policy;
- corresponding `cmuxTests`, package tests, and UI tests;
- (plugin repo) `bridge/cmux_herdr_bridge.py`, state-file helpers, and bridge unit/behavior tests.

Implementation hygiene:

- scaffold new directories/files with shell;
- use small, reviewable patches for model/reducer code;
- for mechanical schema extractions or large status rewrites, prefer a short Python script over error-prone hand edits;
- in Rust integration paths touching shared state, prefer `unwrap_or_default()` (or explicit poison recovery) over `.unwrap()` on mutex locks.

## Verification plan (cmux)

```bash
# package / unit
swift test --package-path Packages/macOS/CmuxNestedTopology   # name TBD
# app tests (as maintained in-repo)
xcodebuild test -scheme cmux -only-testing:cmuxTests
```

Also run:

- package tests for the new model/adapter package;
- `TerminalControllerSocketSecurityTests` and capability-policy tests when socket methods change;
- session snapshot/restore suites when persistence changes;
- sidebar/navigation UI tests when rows change;
- a manual matrix: no Herdr, supported Herdr, unsupported protocol, disconnect/restart, two providers, cmux restart, title-lock holds under event flood, plugin disabled while native live.

## Rollout

1. Hide behind a beta setting/feature flag.
2. Ship read-only telemetry for connection errors and protocol versions, redacting paths/IDs.
3. Enable manual attachment by default after crash/performance review.
4. Enable discovery proposals later; do not silently autoattach until Herdr exposes durable instance identity and the restore threat model is satisfied.
5. Keep the status-pill plugin documented as fallback for older cmux/Herdr versions.
6. Document the single-writer rule so dogfood users do not run plugin `watch` against a surface with live native attachment unless they opt out of native projection.

## Explicitly deferred

- generic third-party executable plugins;
- remote/network provider transport;
- mobile-issued nested mutations;
- automatic server startup/shutdown;
- migration of Herdr panes into native cmux PTYs;
- unified persistence of provider processes;
- using screen scraping as the primary native topology source.
