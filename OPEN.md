# Open items and stopgap limitations

This repo is the **plugin / stopgap** path. It works today without cmux accepting any PR.
Native first-class nested topology is tracked upstream; this file records what the plugin
does **not** claim to solve, and what is still open on each path.

## Live upstream artifacts

| Artifact | URL | Status |
|---|---|---|
| Native MVP PR (hidden `__herdr-compat` dispatcher) | https://github.com/manaflow-ai/cmux/pull/8736 | Open |
| Full nested-topology design issue | https://github.com/manaflow-ai/cmux/issues/8737 | Open |
| This plugin | https://github.com/RaviTharuma/cmux-herdr | Implemented |
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
   the common nested-shell case; it is not a full multi-parent model.

5. **No upstream install channel.**
   Install is `./scripts/install.sh` from this repo (or a clone). There is no Homebrew formula,
   cmux plugin registry entry, or signed app bundle. Fine for a user-controlled stopgap;
   document that if you hand the plugin to someone else.

6. **`watch` is manual.**
   There is no launchd/user-service unit. Users who want continuous mirroring run
   `cmux-herdr watch` in a dedicated pane (or wrap it themselves).

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

- [ ] Optional: document or ship a sample `launchd` plist / LaunchAgent for `cmux-herdr watch`.
- [ ] Optional: multi-parent binding when several Herdr surfaces live in different cmux workspaces
      (today: one binding file; good enough for the single nested host case).
- [ ] Keep README / this file in sync when PR #8736 merges or #8737 moves.
- [ ] Consider a tagged release + short changelog once the MVP PR lands or is rejected
      (so the fallback story is versioned either way).

### B. Native MVP PR (#8736) — review polish

Owned by the CMUX-Herdr Integration chat / worktree `cmux-herdr-native`.
As of the last sibling audit, **uncommitted** polish already covers most bot review nits:

| Review item | Uncommitted status |
|---|---|
| `launchFailed` no longer leaks path / `strerror` | Done in WT |
| Localize strings across full catalog (~20 locales) | Done in WT |
| De-duplicate supported-command list | Done in WT |
| Safer PATH resolution (no directory shadow) | Done in WT |
| Named free helper for `execv` argv | Done in WT |
| Help / usage test | Done in WT |
| Missing-`herdr`-on-PATH hermetic test | **Still open** |
| Commit + push + reply on review threads | **Still open** |

### C. Full native parity (#8737) — long pole

Not started (by design). Blocks:

- Capability negotiation + provider socket attach from host surface
- Read-only nested snapshot import + event subscription
- Virtual descendants under host surface (not real cmux PTYs)
- Forwarded focus/split/mutate actions
- Reattach / restore model
- UI: tree, attention, unread scoped to inner agents

Start only after MVP lands or maintainers signal interest on #8737.

## Coordination

| Track | Owner | Do not thrash |
|---|---|---|
| Plugin + dual-path design | This repo / Integration chat | — |
| PR #8736 polish / land | Integration chat (`w2:t17`) | Sibling chats: review-only unless asked |
| Issue #8737 design | Integration chat | No implementation until signal |
| Herdr chat/task titles | Title worktrees / title owners | Integration stays out of rename policy |

## Quick verify (plugin)

```bash
python3 -m py_compile bin/cmux-herdr bridge/cmux_herdr_bridge.py
PYTHONPATH=bridge python3 -m unittest discover -s bridge -p 'test_*.py' -v
PYTHONPATH=bridge python3 -m unittest discover -s tests -p 'test_*.py' -v
cmux-herdr status
cmux-herdr tree
cmux-herdr sync
```
