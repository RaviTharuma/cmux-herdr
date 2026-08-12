# Open items and stopgap limitations

This repo is the **plugin / stopgap** path. It works today without cmux accepting any PR.
Native first-class nested topology is tracked upstream; this file records what the plugin
does **not** claim to solve, and what is still open on each path.

As of **2026-08-12** (prep for tagged **v0.1.0**).

## Live upstream artifacts

| Artifact | URL | Status |
|---|---|---|
| Native MVP PR (hidden `__herdr-compat` dispatcher) | https://github.com/manaflow-ai/cmux/pull/8736 | Open + mergeable |
| Full nested-topology design issue | https://github.com/manaflow-ai/cmux/issues/8737 | Open (no implementation in this repo) |
| This plugin | https://github.com/RaviTharuma/cmux-herdr | Implemented; tagging v0.1.0 |
| Thrash / annoyance report | [docs/upstream/ANNOYANCES.md](./docs/upstream/ANNOYANCES.md) | Living doc |

Cross-links: PR and issue reference each other; both point back here as the fallback.

## What this plugin solves today

- Mirror Herdr agent state into cmux workspace **status pills** (`herdr:<pane_id>`) and progress.
- CLI for topology and control: `status`, `tree`, `agents`, `sync`, `watch`, `clear`, focus helpers, `split`, `json-dump`.
- Persist Herdr-parent → cmux-workspace binding so outer focus changes do not thrash status writes.
- Skip ordinary shell panes (no agent) so they are not mirrored as agents.
- Clear stale `herdr:*` keys on each sync while leaving unrelated cmux statuses alone.
- Optional custom sidebar + agent skill documenting the dual hierarchy.
- Hybrid pane/session association cache (`cmux-herdr associations`), pruned each sync.
- Idempotent install / scoped uninstall.
- Sample LaunchAgent for continuous `watch` via `./scripts/install-watch-service.sh`
  (issue [#1](https://github.com/RaviTharuma/cmux-herdr/issues/1) closed).
- Herdr **0.8** pane parsing: agent name under `agent_session.agent` when top-level `agent` is absent.
- Hermetic stdlib `unittest` suite (`./scripts/test.sh`).

## Explicit limitations (not bugs)

1. **No first-class nested hierarchy in cmux.**
   Inner Herdr workspaces/tabs/panes never become Bonsplit objects. Status pills are a flat
   projection onto one outer workspace. Full hierarchy is issue [#8737](https://github.com/manaflow-ai/cmux/issues/8737).

2. **Polling, not events.**
   `watch` loops on an interval (default 3s). Native path should subscribe to Herdr events
   and resync from snapshots.

3. **No reattach model after cmux restart.**
   Parent binding is best-effort local state under `~/.local/state/cmux-herdr/`. There is no
   cmux-owned surface↔provider session identity. That is part of the native design.

4. **One outer workspace projection.**
   Multi-window / multi-surface Herdr hosts can collide if multiple Herdr parents map into
   the same cmux workspace, or if outer workspace IDs rotate. Binding persistence mitigates
   the common nested-shell case; it is not a full multi-parent model
   ([#2](https://github.com/RaviTharuma/cmux-herdr/issues/2) still open).

5. **No upstream install channel.**
   Install is `./scripts/install.sh` from this repo (or a tagged clone). There is no Homebrew
   formula, cmux plugin registry entry, or signed app bundle. Fine for a user-controlled
   stopgap; see [RELEASE.md](./RELEASE.md) for tag-based install.

6. **Statuses depend on Herdr `agent_status`.**
   Pills only mirror what Herdr reports. Ordinary shell panes without an agent are skipped;
   unknown/missing statuses map to gray `questionmark.circle`.

7. **Titles are out of scope.**
   Chat/task title generation and rename policy belong to the Herdr native title tracks
   (`herdr-task-titles` / related worktrees). This plugin only **reads** titles for display
   names in status pills; it does not write them.

8. **Hidden compat dispatcher ≠ native parity.**
   PR [#8736](https://github.com/manaflow-ai/cmux/pull/8736) only adds
   `cmux __herdr-compat …` aliases that `exec` into Herdr. It does **not** connect to the
   Herdr socket from cmux, import nested topology, or render virtual descendants.
   Do not treat MVP merge as “native parity done.”

## Open work by path

### A. Plugin residual (this repo)

- [x] Sample `launchd` LaunchAgent for `cmux-herdr watch` (`scripts/com.cmux-herdr.watch.plist` + install/uninstall helpers). Users can install with `./scripts/install-watch-service.sh`.
- [ ] Optional: multi-parent binding when several Herdr surfaces live in different cmux workspaces
      (today: one binding file; good enough for the single nested host case) — issue [#2](https://github.com/RaviTharuma/cmux-herdr/issues/2).
- [x] Upstream draft banners point at live #8737 / #8736 (prefer GitHub over local drafts).
- [x] `./scripts/test.sh` — stdlib unittest only (no pytest).
- [x] Herdr 0.8 `agent_session.agent` parsing.
- [x] Release artifacts for v0.1.0 (`VERSION`, `CHANGELOG.md`, `RELEASE.md`) — tag after merge per [RELEASE.md](./RELEASE.md).

### B. Native MVP PR (#8736) — open + mergeable

Owned by the CMUX-Herdr Integration chat / worktree `cmux-herdr-native`.
As of 2026-08-12 the PR tip is **open and mergeable** (hidden `__herdr-compat`).
Review polish on the tip includes:

| Review item | Status on PR tip |
|---|---|
| `launchFailed` no longer leaks path / `strerror` | Done |
| Localize strings across full catalog (~20 locales) | Done |
| De-duplicate supported-command list | Done |
| Safer PATH resolution (no directory shadow) | Done |
| Named free helper for `execv` argv | Done |
| Help / usage test | Done |
| Missing-`herdr`-on-PATH hermetic test | **Done** (plugin issue [#5](https://github.com/RaviTharuma/cmux-herdr/issues/5) closed in docs) |
| Commit + push of polish | Done on tip |

Remaining for that PR is maintainer review / merge — not more missing-PATH work.

### C. Full native parity (#8737) — long pole

Not started (by design). Blocks:

- Capability negotiation + provider socket attach from host surface
- Read-only nested snapshot import + event subscription
- Virtual descendants under host surface (not real cmux PTYs)
- Forwarded focus/split/mutate actions
- Reattach / restore model
- UI: tree, attention, unread scoped to inner agents

Start only after MVP lands or maintainers signal interest on #8737.
**Do not expand this plugin into #8737 native implementation.**

## Coordination

| Track | Owner | Do not thrash |
|---|---|---|
| Plugin + dual-path design | This repo / Integration chat | — |
| PR #8736 polish / land | Integration chat (`w2:t17`) | Sibling chats: review-only unless asked |
| Issue #8737 design | Integration chat | No implementation until signal |
| Herdr chat/task titles | Title worktrees / title owners | Integration stays out of rename policy |

## Quick verify (plugin)

```bash
./scripts/test.sh
./bin/cmux-herdr --version
cmux-herdr status
cmux-herdr tree
cmux-herdr sync
```


## Filed annoyance issues (2026-07-23; status as of 2026-08-12)

### cmux (`manaflow-ai/cmux`)
- https://github.com/manaflow-ai/cmux/issues/8743 — PATH resolver treats directories as executables
- https://github.com/manaflow-ai/cmux/issues/8744 — CLI hygiene: launch errors, locales, missing-PATH tests, command-list dedupe

### this plugin (`RaviTharuma/cmux-herdr`)
- https://github.com/RaviTharuma/cmux-herdr/issues/1 — LaunchAgent for `watch` (**closed**; sample + `install-watch-service.sh` shipped)
- https://github.com/RaviTharuma/cmux-herdr/issues/2 — multi-parent binding collisions (**still open**; optional enhancement — not in v0.1.0)
- https://github.com/RaviTharuma/cmux-herdr/issues/3 — no tagged release (close after tagging v0.1.0 per [RELEASE.md](./RELEASE.md))
- https://github.com/RaviTharuma/cmux-herdr/issues/4 — upstream draft drift (**closed**; canonical banners added)
- https://github.com/RaviTharuma/cmux-herdr/issues/5 — PR #8736 missing-PATH residual (**done on PR tip**; docs closed — not a plugin runtime gap)
- https://github.com/RaviTharuma/cmux-herdr/issues/6 — unittest vs pytest docs (**closed**; `test.sh` + README note)
- https://github.com/RaviTharuma/cmux-herdr/issues/7 — dual-chat worktree thrash (**closed**; OPEN.md coordination table)
