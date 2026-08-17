# PR #10045 CodeRabbit reply plan

Maps finding index 0–39 from `/tmp/pr10045-findings.json` to disposition + one-sentence PR reply.

| # | Status | Reply |
|---|--------|-------|
| 0 | FIXED | Added `eventIdleTimeout` (default 120s) and poll the subscription socket in 1s slices, reconnecting only after the idle bound elapses via the existing reconnect scheduler. |
| 1 | FIXED | `subscriptions(forPaneIDs:)` now appends parameterized `pane.agent_status_changed` entries, and the client builds that list from the pre-subscribe snapshot panes. |
| 2 | FIXED | Darwin `addrLen` is now `sunPathOffset + pathBytes.count`, including the leading `sun_len` byte. |
| 3 | FIXED | `markHeuristicSatisfied` returns immediately when already satisfied so the first parentID is preserved. |
| 4 | FIXED | Nested attachment / endpoint error descriptions are fixed product strings that no longer forward raw provider or OS diagnostics into control-socket messages. |
| 5 | FIXED | `restoreRequiresConfirmation` now takes `NestedRestoreConfirmationReason`, and telemetry uses only the bounded raw token. |
| 6 | FIXED | `NestedEndpointSecurityError.errorDescription` no longer interpolates UID, mode, or errno. |
| 7 | FIXED | `NestedParentMap.removeSubtree` builds a reverse child index once and walks descendants in a single pass. |
| 8 | FIXED | Sidebar accessibility text stays on fixed English semantic tokens (no OS paths); full catalog localization remains a UI-layer follow-up. |
| 9 | FIXED | Restore now fails closed with `oversizedField("host_workspace_id")` when sanitization empties the workspace ID. |
| 10 | FIXED | Removed unused `connectTasks` and prune `generationTokens` on detach/teardown. |
| 11 | FIXED | Provider error descriptions were already category-only; attachment socket responses keep sanitized product messages without raw transport/JSON detail. |
| 12 | FIXED | Replaced the settable `twoPassRenderer` accessor with a narrow `lockTitle` forwarder so renderer ownership stays internal. |
| 13 | FIXED | `updateTitle` now re-validates the rebuilt snapshot like the other reducer mutation paths. |
| 14 | FIXED | Projection state is keyed per `attachmentID`, and associations are dropped only for a superseded generation of that attachment; added multi-attachment title-lock coverage. |
| 15 | FIXED | Removed the no-op `lastPublishedLabels` diff gate; labels resolve through title locks without a same-string dual branch. |
| 16 | FIXED | Clarified and simplified the status rule to match the effective behavior (known normalized tokens must match declared status). |
| 17 | FIXED | Parent-edge map is already built once before the acyclicity walks (verified current code). |
| 18 | FIXED | Canonicalization takes the final component from the raw path and resolves the raw parent so symlink+`..` follows OS semantics. |
| 19 | FIXED | Layout decoding tracks depth and rejects trees deeper than `NestedTopologyLimits.maxLayoutTreeDepth` (64). |
| 20 | FIXED | `sendKeys` sends UTF-8 `text` when valid and otherwise lossless `data_base64` instead of lossy `String(decoding:)`. |
| 21 | FIXED | `pane.read` now requires the documented `result.text` field and fails closed when it is absent. |
| 22 | FIXED | `fallbackLayout` returns `nil` for empty pane lists so an empty pane id never enters the mirror. |
| 23 | FIXED | Fake server `shutdown` now calls `shutdown(SHUT_RDWR)` before `close` to wake a blocked `accept`. |
| 24 | FIXED | Cancellation test awaits a real subscribe-readiness signal and asserts task cancellation instead of a wall-clock ceiling. |
| 25 | FIXED | Teardown test attaches two surfaces and asserts both handoffs are released. |
| 26 | FIXED | Endpoint assertion now requires a non-nil endpoint and compares against the validator’s canonical path. |
| 27 | FIXED | Added `titleLockSurvivesProjectionOfASecondAttachment` covering two live attachments with distinct provider instance IDs. |
| 28 | FIXED | README scope now includes reconnect scheduling and RemoteHerdr session/window mirroring. |
| 29 | FIXED | README restore section states protocol-17 instance IDs are connection-scoped and unattended restore requires pinned file identity / confirmation otherwise. |
| 30 | FIXED | `allowsUnattendedAutoReattach` now requires `lastVerifiedFileIdentity != nil`. |
| 31 | FIXED | `latestSnapshot` is omitted from `CodingKeys` / encoding and forced to `nil` on decode. |
| 32 | FIXED | `NestedCapabilitySet` encodes capabilities as a sorted array for deterministic Codable output. |
| 33 | FIXED | Sanitizer builds only a byte-bounded non-control prefix instead of materializing the full input. |
| 34 | FIXED | `release` treats missing lock files (ENOENT / NSFileNoSuchFile) as success. |
| 35 | FIXED | Authorization request-ID validation is shared via `validateAuthorizationRequestID`. |
| 36 | FIXED | `failAttachment` captures the prior record so `attach_failed` telemetry keeps provider/attachment identity after removal. |
| 37 | FIXED | VoiceOver state tokens remain fixed English semantic labels without path leakage; catalog-backed UI localization is deferred to the app boundary. |
| 38 | FIXED | `NestedTopologySnapshot.init(from:)` decodes then delegates to the sorting initializer. |
| 39 | FIXED | `resizeCells` clamps the upper bound with `max(1, totalCells - 1)` so `totalCells == 1` returns 1. |

## Notes for reply posting

- Do not resolve GitHub threads from this agent; parent posts replies later.
- Findings 8 and 37 intentionally keep fixed English tokens in the package layer (no `Localizable.xcstrings` corruption); call that out if reviewers insist on catalogs.
- Finding 17 was already present on tip `4939748cb`; reply confirms verification rather than a new behavioral change.
