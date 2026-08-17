# Open items and stopgap limitations

This repo is the **plugin / stopgap** path. It works today without cmux accepting any PR.
Native first-class nested topology is tracked upstream; this file records what the plugin
does **not** claim to solve, and what is still open on each path.

As of **2026-08-12** (prep for tagged **v0.1.0**).

## Live upstream artifacts

| Artifact | URL | Status |
|---|---|---|
| Community poll (native Herdr as tmux counterpart) | https://github.com/manaflow-ai/cmux/discussions/10106 | Open |
| Native MVP PR (hidden `__herdr-compat` dispatcher) | https://github.com/manaflow-ai/cmux/pull/8736 | Open + mergeable |
| Nested topology v1 PR | https://github.com/manaflow-ai/cmux/pull/10045 | Open |
| Window-mirror engine | https://github.com/RaviTharuma/cmux/pull/8 | Merged |
| Full nested-topology design issue | https://github.com/manaflow-ai/cmux/issues/8737 | Open (no implementation in this repo) |
| This plugin | https://github.com/RaviTharuma/cmux-herdr | Implemented; v0.2.0 tagged |
| Thrash / annoyance report | [docs/upstream/ANNOYANCES.md](./docs/upstream/ANNOYANCES.md) | Living doc |

Cross-links: PR and issue reference each other; both point back here as the fallback.

## What this plugin solves today

- Mirror Herdr **tabs/panes into real cmux tabs/splits** (`cmux-herdr mirror`,
  `--tmux-parity` for the ssh-tmux contract) with `attach-pane` followers.
- Mirror Herdr agent state into cmux workspace **status pills** (`herdr:<pane_id>`) and progress.
- CLI for topology and control: `status`, `doctor`, `tree`, `agents`, `sync`, `watch`,
  `mirror`, `attach-pane`, `send-key`, `observe`, `attach`, `detach`, `restore`,
  `clear`, focus helpers, `read-pane` / `read-agent`, `split`, `json-dump`.
- Persist per-host-fingerprint Herdr-parent → cmux-workspace bindings so outer focus
  changes do not thrash status writes and multi-window hosts do not collide.
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

1. **No Ghostty PTY theft.**
   `mirror --tmux-parity` creates extra cmux tabs/splits that *follow* Herdr
   panes via `attach-pane` (poll `herdr pane read`, forward `pane send`,
   SIGWINCH resize). It does not insert Herdr PTYs into Bonsplit the way native
   `ssh-tmux` / PR7 `RemoteHerdrWindowMirror` does. See
   [docs/upstream/TMUX_PARITY.md](./docs/upstream/TMUX_PARITY.md).

2. **Event-driven watch, poll fallback.**
   `watch --tmux-parity` holds one Herdr Unix-socket `events.subscribe` session
   when `HERDR_SOCKET_PATH` is live, then resyncs; otherwise it polls. Native
   PR7 feeds `%output`-style bytes into Ghostty; the plugin still polls
   `pane.read` and applies an incremental delta.

3. **Live apply runs in the plugin; Ghostty panels are still native.**
   `bridge/cmux_herdr_live.py` is the ssh-tmux apply machine
   (makePanel, output, drag, focus, size, attach/restore). Surfaces are
   in-memory until native `TerminalPanel.processRemoteOutput` is wired.
   Restore **reattaches** (never a stale Bonsplit tree).

4. **Flat outer workspace projection (not nested hierarchy).**
   Status pills are still a flat projection onto one outer workspace per host fingerprint.
   Multi-window / multi-surface Herdr hosts keep distinct `parent-<fingerprint>.json`
   bindings (issue [#2](https://github.com/RaviTharuma/cmux-herdr/issues/2)); full nested
   hierarchy remains [#8737](https://github.com/manaflow-ai/cmux/issues/8737).

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
- [x] Multi-parent host-fingerprint bindings when several Herdr surfaces live in different
      cmux windows/workspaces (`parent-<fingerprint>.json` / `associations-<fingerprint>.json`;
      sync/watch select the invoking env) — issue [#2](https://github.com/RaviTharuma/cmux-herdr/issues/2).
- [x] v0.2 CLI pack: `doctor`, `read-pane` / `read-agent`, `focus-workspace` / `focus-agent`,
      hardened `focus-pane` (no zoom fallback).
- [x] Upstream draft banners point at live #8737 / #8736 (prefer GitHub over local drafts).
- [x] `./scripts/test.sh` — stdlib unittest only (no pytest).
- [x] Herdr 0.8 `agent_session.agent` parsing.
- [x] Release artifacts for v0.1.0 (`VERSION`, `CHANGELOG.md`, `RELEASE.md`) — tag after merge per [RELEASE.md](./RELEASE.md).
- [x] Userspace deep mirror: `mirror` / `attach-pane` / `watch --mirror` project
      Herdr tabs/panes into real cmux tabs/splits (idempotent `herdr-mirror:<pane_id>`
      keys). Not Ghostty PTY theft — extra viewers, like a second tmux client.
- [x] **Persistent NDJSON session + engine:** `HerdrEventSession` for
      `watch --tmux-parity`; `cmux_herdr_engine.py` is the Python twin of
      `RemoteHerdrWindowMirror` (zoom/close/structure version/sizing/output
      delta). Remaining plugin gap is PTY theft / divider-drag (native PR7).
- [x] **tmux-parity plugin:** `mirror --tmux-parity` / `watch --tmux-parity` —
      layout tree, ratios, tab order, focus, prune, attach cbreak/SIGWINCH/ANSI.
      `watch --tmux-parity` attaches on start and detaches on stop.
- [x] CLI `attach` / `detach` / `restore` / `send-key` / `observe` for the
      live apply host. Detach never calls `server.stop`.
- [x] Single-writer guard when native attachment is live (`CMUX_HERDR_NATIVE_LIVE` /
      `native-live-<fingerprint>` marker; `CMUX_HERDR_FORCE_PLUGIN` escape hatch).
- [x] **Dual-path handoff** (`bridge/cmux_herdr_handoff.py`): plugin and
      native share one lease + one restore file. Dead pid / expired
      heartbeat is stale (plugin may resume). `attach` / `observe` /
      `restore` / `watch` yield when native owns; they do not start a
      competing in-memory host. Host close still never `server.stop`.
- [x] Native-title lock + diff-before-write (`lock-title` / `unlock-title`,
      `CMUX_HERDR_LOCK_TITLES`).
- [x] Heuristic-once parent map (`parent_tab_id` + `heuristic_satisfied`; session
      identity change resets locks).
- [x] Mirror / `--tmux-parity` yields when native attachment is live (same single-writer
      rule as status pills).
- [x] Engine-owned reconcile drives `mirror_to_cmux` (no dual DesiredMirror/engine drift).
- [x] Single size-claim writer (no per-viewer SIGWINCH resize war).
- [x] Fail-closed layout application (no orphan-tab fallback on split failure).
- [x] Socket-first snapshot for `watch --tmux-parity` (CLI fan-out only on socket drop).
- [x] **Bonsplit impose planner** (`bridge/cmux_herdr_impose.py`, Swift twin
      `RemoteHerdrImpose`): tmux +1 divider-cell fractions, targeted leaf
      expand/remove, `plan(w) <= w`, divider-drag hold/resolve. Plugin applies
      fractions via `set-ratio`.
- [x] **Host-apply verb list** (`bridge/cmux_herdr_host.py`): ordered
      create→tree→impose→focus (tmux `makePanel` before rebuild). Fake host
      proves the order. Native twin stays on a **separate** fork branch —
      do not land on #10045 while the other chat is mid CodeRabbit. See
      [docs/upstream/LANES.md](docs/upstream/LANES.md).
- [x] **Cmux-tmux control depth** (`bridge/cmux_herdr_control.py`):
      named keys, input budget, focus rollback, adjacent pane, user
      split, seed queue, agent-status activity, detach-on-host-close.
- [x] **I/O + session host** (`bridge/cmux_herdr_io.py` /
      `bridge/cmux_herdr_session.py`): isolated `route_output` /
      `route_input`, provider-vs-user focus (no echo loop), session
      create/close/reorder verbs. Native twins stay off #10045 until
      CodeRabbit 5/5.

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

Two native layers (do not collapse them):

1. **Sidebar nested topology** — [PR #10045](https://github.com/manaflow-ai/cmux/pull/10045) (open, dirty). Virtual rows + `nested.node.focus`. **Not** ssh-tmux.
2. **Window mirror (PR7)** — paste-ready [docs/upstream/PR7_HERDR_WINDOW_MIRROR.md](./docs/upstream/PR7_HERDR_WINDOW_MIRROR.md). Copy `RemoteTmuxWindowMirror`: real cmux tabs, Bonsplit panes, Ghostty I/O, layout, resize, zoom, prune. Honest missing list (seed, named keys, focus rollback, attach/detach, …): [TMUX_PARITY.md — contract vs live](./docs/upstream/TMUX_PARITY.md#contract-vs-live-appkit).

Matrix: [docs/upstream/TMUX_PARITY.md](./docs/upstream/TMUX_PARITY.md).

**Do not expand this plugin into #8737 Swift.** Keep `--tmux-parity` as the userspace stand-in.

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
cmux-herdr doctor
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
- https://github.com/RaviTharuma/cmux-herdr/issues/2 — multi-parent binding collisions (**closable** when host-fingerprint PR merges; prep for 0.2)
- https://github.com/RaviTharuma/cmux-herdr/issues/3 — no tagged release (close after tagging v0.1.0 per [RELEASE.md](./RELEASE.md))
- https://github.com/RaviTharuma/cmux-herdr/issues/4 — upstream draft drift (**closed**; canonical banners added)
- https://github.com/RaviTharuma/cmux-herdr/issues/5 — PR #8736 missing-PATH residual (**done on PR tip**; docs closed — not a plugin runtime gap)
- https://github.com/RaviTharuma/cmux-herdr/issues/6 — unittest vs pytest docs (**closed**; `test.sh` + README note)
- https://github.com/RaviTharuma/cmux-herdr/issues/7 — dual-chat worktree thrash (**closed**; OPEN.md coordination table)
