# Stabilization audit — 2026-09-09

Initial findings were recorded before implementation; the capability matrix below corrects the initial command assumptions. Scope: plugin stabilization, not native integration development. Upstream repositories and live sessions remain untouched.

## Audited source identities

| Domain | Checkout | Exact commit |
| --- | --- | --- |
| Plugin | cmux-herdr native-theme baseline | `20f200738740f6c4140b3e5cb42bff65d935433b` |
| Native host (no Herdr implementation found) | manaflow-ai/cmux | `829c6af45478ef5c2196801824c35f5cb4dc5d69` |
| Builtin RemoteTmux / tmux compatibility | same cmux checkout, not a separate repository | `829c6af45478ef5c2196801824c35f5cb4dc5d69` |
| Herdr runtime / protocol | herdrdev/herdr | `b9ce96869e89937278d673d70ae4c135dd318469` |

### Native theme passthrough and terminal fallback

At the pinned cmux source, `Sources/TerminalController.swift`'s
`upsertSidebarMetadata` (14728 onward) reads optional `color` and replaces the
entire `SidebarStatusEntry`; omitting color resets an old tint. The generic
status path exposes hex, not an adaptive semantic-token contract.
`Sources/ContentView.swift:16639` deliberately returns the active foreground
for a selected explicitly colored entry; otherwise it uses explicit hex or
native secondary foreground. Its `usesInvertedActiveForeground` is `isActive`
(15563), passed to metadata rows at 15836. The AppKit row-cell path likewise
substitutes selected foreground for contrast (row-cell source, line 747).
This is deliberate native behavior, not an upstream defect justified solely
by #75; the full colored-selected request remains unfulfilled.

The plugin sends native status text/icons/priorities without a color argument.
Successful writes clear legacy cached colors; failed writes preserve retry
state. Neither native selection nor a host theme is overridden.
Separately, `cmux-plugin.toml` declares `kind = "sidebar"` and launches
`bin/cmux-herdr-sidebar`, which executes the Rust terminal `sidebar` command.
`src/sidebar.rs` uses the JSON-lines socket commands `identify`,
`list-workspaces`, and `select-workspace`; this fallback is not a native
metadata row. It preserves textual selection/active markers and default
terminal colors, with no reverse-video selection background. Native metadata
is preferred for agent status; no unavailable native plugin component API is
invented and no separate custom theme/sidebar or tmux shim is added.

These are source-only capabilities, not a macOS visual/E2E verification.
No native upstream Herdr implementation files were found in the inspected
cmux Sources/CLI checkout; builtin RemoteTmux remains a distinct reference.

## Capability findings

The audited cmux commit contains no Herdr-named implementation files or native Herdr integration in Sources/CLI. Statements in TMUX_PARITY.md about native Herdr controller/association wiring describe a target or other historical work, not capabilities present at this commit. Plugin LiveApplyHost objects are models, not native TerminalPanel/Bonsplit surfaces.

Builtin tmux is the concrete baseline: `Sources/RemoteTmuxWindowMirror+Bonsplit.swift` reconciles native tree structure, creates native tabs, and imposes divider geometry; `Sources/RemoteTmuxSessionMirror+OutputRouting.swift` routes per-pane bytes with stateful title/notification filtering and snapshot reseeding. `CLI/CMUXCLI+TmuxCompatLaunchContext.swift` validates inherited launch context rather than borrowing global focus. These capabilities must not be attributed to the plugin.

Herdr api/schema/panes.rs and app/api/panes.rs distinguish pane.resize (direction plus fractional amount applied to the layout) from terminal geometry. pane.read returns a terminal snapshot, not a raw output byte stream. app/api/session.rs builds session.snapshot from workspace/tab/layout identities. events.subscribe exists in the API schema/server, but does not turn pane.read polling into tmux control-mode output routing.

### Source-pinned capability matrix

All cmux references below refer to commit `829c6af45478ef5c2196801824c35f5cb4dc5d69`.

| Capability | Actual contract / evidence | Boundary |
| --- | --- | --- |
| Create external terminal viewer | `CLI/cmux.swift:7134–7175`: `new-surface --type terminal --pane P` | Not `create-terminal`; omitting `--pane` uses focus, not a guaranteed new pane. |
| Launch attach follower | `CLI/cmux.swift:27631–27658`: `respawn-pane --surface S --command C` → `surface.respawn` | Shell command starts an external viewer; no native TTY takeover. |
| Targeted split | `CLI/cmux.swift:7001–7027`: `new-split <direction> --surface S` → `surface.split` | Returned surface is already a terminal; do not create an additional tab. `split` is not this CLI contract. |
| Rename | `CLI/cmux.swift:11360–11411`: `rename-tab --surface S --title=TITLE` | Equals form protects the initial option scan. The upstream forwarding to `runTabAction` re-expands title tokens; flag-shaped titles are not proven safe end-to-end. |
| Rollback | `CLI/cmux.swift:7190+`: `close-surface --surface S` with explicit workspace | Close only the newly created identity; no focused-surface fallback. |
| Native tmux geometry | `Sources/RemoteTmuxWindowMirror+Bonsplit.swift` | Builtin cmux capability, not plugin native Herdr support. |
| Native tmux output | `Sources/RemoteTmuxSessionMirror+OutputRouting.swift` | Byte routing and reseeding, unlike Herdr `pane.read` snapshots. |
| Native Herdr UI / Sidebar | No native Herdr implementation at the audited commit | Roadmap; native Sidebar issue #75 is blocked, not fixed. |

The previous assumption that `create-terminal` / `split` were supported cmux projection verbs was incorrect. Linux fixtures exercise source-aligned argv and response handling, not native AppKit/Bonsplit integration.

## Initial stabilization findings (before fixes)

- handoff.rs ignores resolve_writer's supplied PID; fresh foreign plugin ownership does not stop claims/heartbeats. Lease writes use a shared temporary filename without a read/check/write lock. Native claims can replace a foreign native owner. Releases unlink paths without verifying owner, PID, or fingerprint. Global markers can leak ownership across hosts.
- Shared restore cleanup is unconditional. Watch shutdown remembers a socket after yielding and can remove successor restore state.
- Continuous watch attaches live state and projects mutations despite --dry-run; watch_project hardcodes false for mirror dry-run.
- mirror.rs joins attach argv with spaces. Targeted terminal creation falls back to untargeted commands; creation failure after a successful split leaks the split. The existing refusal to create an orphan tab when splitting fails is already correct.
- install.sh overwrites an unrelated binary; uninstall.sh deletes basename-selected artifacts and calls a PATH-selected executable. Skill directories lack ownership checks. install.sh/uninstall.sh are tracked 100644; other shell entry points are executable.
- LaunchAgent installation accepts broken symlinks, substitutes HOME without XML escaping, and lacks explicit host binding. Uninstall can unload a label without proving plist ownership.
- Release publication edits/clobbers published assets instead of staging an immutable draft. Build artifacts are not smoke-executed.
- Fetch bootstrap silently invokes Cargo after download/platform failures. Source builds need explicit opt-in. Checksum mismatch already fails closed.
- API transport already avoids replaying potentially sent mutations; no replacement transport is needed.

## Scoped implementation and verification sequence

1. Serialize lease transitions with stable `rustix` flock guards, unique atomic temporary files, fingerprint filtering, retained instance tokens, and conditional release/restore cleanup. Preserve tokenless compatibility reads without granting generation authority.
2. Add continuous-watch dry-run regression; skip live attachment/heartbeat/persistence and propagate dry-run through projection and cleanup.
3. Add shell-metacharacter argv and split-failure regressions; quote each argument, retain pane targeting, and roll back only a newly created split on subsequent failure.
4. Exercise installers with isolated HOME/PATH and fake host CLIs; enforce provenance, executable entry points, XML-safe fixed argv and explicit host binding. Never run real launchctl.
5. Exercise fetch/release fixtures; require explicit Cargo opt-in, smoke-check artifacts, and publish draft-first without replacing published assets.
6. Run the combined Rust gate using `RUSTUP_TOOLCHAIN=1.98.0` and `CARGO_TARGET_DIR=/dev/shm/cmux-herdr-stabilize-target`, then release help/version smoke. Coordinate commit and PR only after the combined gate; no commit/push has been made by the runtime worker.

## Verification limitations

Pinned 1.98.1 lacks applicable Clippy; verification uses 1.98.0. The last runtime-targeted gate passed 250 tests across six suites before the final title/doctor corrections. The user reported `cargo check --all-targets` passing in 4.17s. Combined verification is required after packaging integration. Macbook13 is offline; macbook16 has no trusted host key. Live macOS E2E is blocked and is not claimed. Linux fixtures cannot establish native AppKit or LaunchAgent runtime correctness.
