# Open items and plugin limitations

**cmux-herdr** is the released cmux plugin for Herdr. It works today without a
cmux source patch. Native first-class nested topology is tracked upstream; this
file records what the plugin does **not** claim to solve, and what is still
open on each path.

As of **2026-08-26** (errors/lackings freeze live).

## Live upstream artifacts

| Artifact | URL | Status |
|---|---|---|
| Native MVP PR (hidden `__herdr-compat` dispatcher) | https://github.com/manaflow-ai/cmux/pull/8736 | Open; CR **0**/14; tip `57a3fda07eb7`; Austin pushed four follow-up fixes; now conflicts with newer `main` — maintainer-owned refresh/review pending |
| Nested topology Herdr v1 PR | https://github.com/manaflow-ai/cmux/pull/10045 | Open; CR **0**/173; tip `44a3e25eae7e`; current and mergeable — maintainer review/merge permission required |
| Full nested-topology design issue | https://github.com/manaflow-ai/cmux/issues/8737 | Open (native work on cmux fork / #10045) |
| Community poll (native Herdr vs plugin) | https://github.com/manaflow-ai/cmux/discussions/10106 | Open; **1 upvote / 0 comments** |
| Errors & lackings freeze | [docs/upstream/ERRORS_AND_LACKINGS.md](./docs/upstream/ERRORS_AND_LACKINGS.md) | Freeze `freeze-2026-08-19T065836Z` |
| Herdr beyond tmux | [docs/upstream/HERDR_BEYOND_TMUX.md](./docs/upstream/HERDR_BEYOND_TMUX.md) | Agent/worktree/manifest CLI (no tmux analogue) |
| This plugin | https://github.com/RaviTharuma/cmux-herdr | Implemented; latest tag on `main` (see `VERSION`) |
| Thrash / annoyance report | [docs/upstream/ANNOYANCES.md](./docs/upstream/ANNOYANCES.md) | Living doc |

Cross-links: PR and issue reference each other; both point back here as the fallback.

## What this plugin solves today

- Mirror Herdr **tabs/panes into real cmux tabs/splits** (`cmux-herdr watch`
  defaults to the ssh-tmux contract; `mirror` is the one-shot tool) with
  `attach-pane` followers.
- Mirror Herdr agent state into cmux workspace **status pills** (`herdr:<pane_id>`) and progress.
- CLI for topology and control: `status`, `doctor`, `tree`, `agents`, `sync`, `watch`,
  `mirror`, `attach-pane`, `send-key`, `observe`, `attach`, `detach`, `restore`,
  `api` / tab-pane-workspace-agent verbs, `clear`, focus helpers, `read-pane` /
  `read-agent`, `split`, `json-dump`.
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
   panes via `attach-pane` (poll `herdr pane read`, forward `pane send-text`).
   SIGWINCH cannot claim the inner grid (`pane.resize` is split-edge only).
   It does not insert Herdr PTYs into Bonsplit the way native
   `ssh-tmux` / PR7 `RemoteHerdrWindowMirror` does. See
   [docs/upstream/TMUX_PARITY.md](./docs/upstream/TMUX_PARITY.md).

2. **Event-driven watch, poll fallback.**
   `watch --tmux-parity` holds one Herdr Unix-socket `events.subscribe` session
   when `HERDR_SOCKET_PATH` is live, then pumps events into `LiveApplyHost`
   (topology → resync, `pane.updated` → `pane.read` delta, focus, agent_status)
   and remirrors cmux viewers on topology or interval. Native PR7 feeds
   `%output`-style bytes into Ghostty; the plugin still polls `pane.read`.

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

5. **Install is the official cmux plugin manager.**
   Users run `cmux sidebar plugin install` / `use` / `update` / `remove`.
   `./scripts/install.sh` is contributor/dev only. There is no Homebrew
   formula or signed app bundle. See [RELEASE.md](./RELEASE.md).

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
- [x] **Herdr control-surface parity** (`bridge/cmux_herdr_api.py`):
      socket-first allowlisted RPC + CLI verbs for tabs/panes/workspaces/
      agents/layout. Never `server.stop`. Control still works when native
      owns the display lease.
- [x] **Live SessionHost pump** (`bridge/cmux_herdr_pump.py`):
      `watch --tmux-parity` routes `pane.read`, focus, and `agent_status`
      into `LiveApplyHost`. Isolated per pane. Not Ghostty PTY theft.
- [x] **Persistent RPC + input drain**: one Herdr socket for pump reads;
      first paint seeds; queued keys flush to `pane.send_*`; focus/split
      go socket-first. Doctor pings the API. `workspace.focused` does not
      full-resync. CLI fallback is a single `herdr` invoke.
- [x] **Control CLI pack**: `set-ratio` / `move-pane` / `focus-dir` /
      `move-tab` / `rename-pane` / `rename-agent` / `start-agent` /
      `notify` / `wait-output`.
- [x] **SessionHost watch ceiling**: subscribe gap → snapshot resync;
      timeout poll does not remirror; `attach-pane` I/O is socket-first
      (`pane.read` / `pane.send_*` / `pane.resize`). Snapshot uses the
      shared HerdrApi session. This is as far as userspace can go without
      Ghostty PTY theft / a live Bonsplit controller.
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
- [x] **Herdr-beyond-tmux CLI**: `agent-explain`, `agent-view`, `process-info`,
      `release-agent`, `clear-agent-authority`, `window-title`, `layout-apply`,
      `manifests`, `worktree`, `workspace-move` — see
      [docs/upstream/HERDR_BEYOND_TMUX.md](docs/upstream/HERDR_BEYOND_TMUX.md).
- [x] **Host-apply verb list** (`bridge/cmux_herdr_host.py`): ordered
      create→tree→impose→focus (tmux `makePanel` before rebuild). Fake host
      proves the order. Native twin stays on a **separate** cmux-fork branch —
      do not land it on #10045 from this plugin. See
      [docs/upstream/LANES.md](docs/upstream/LANES.md).
- [x] **Cmux-tmux control depth** (`bridge/cmux_herdr_control.py`):
      named keys, input budget, focus rollback, adjacent pane, user
      split, seed queue, agent-status activity, detach-on-host-close.
- [x] **I/O + session host** (`bridge/cmux_herdr_io.py` /
      `bridge/cmux_herdr_session.py`): isolated `route_output` /
      `route_input`, provider-vs-user focus (no echo loop), session
      create/close/reorder verbs. Native twins stay off #10045 until that
      PR's review is idle.

### B. Native MVP PR (#8736) — open, merge UNSTABLE

Owned on the native cmux fork / worktree `cmux-herdr-native`, not in this plugin repo.
As of 2026-08-23 the PR tip is **open**, CR threads **0**, `mergeStateStatus=UNSTABLE` (hidden `__herdr-compat`).
Needs maintainer approving review / merge — not more missing-PATH work.

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

1. **Sidebar nested topology** — [PR #10045](https://github.com/manaflow-ai/cmux/pull/10045) (open and mergeable; tip `44a3e25eae7e`). Virtual rows + `nested.node.focus` + PR7 host mirror on tip. **Not** full ssh-tmux until dogfood/acceptance closes.
2. **Window mirror (PR7)** — live AppKit host is on `#10045` tip (`RemoteHerdrWindowMirrorHost*`); earlier fork drafts #12–#18 are closed. Plugin `--tmux-parity` is the
   userspace stand-in and is at ceiling: socket RPC + SessionHost pump +
   attach-pane followers. Remaining depth is Ghostty `TerminalPanel` /
   Bonsplit ownership, which this repo cannot do.

Matrix: [docs/upstream/TMUX_PARITY.md](./docs/upstream/TMUX_PARITY.md).

**Do not expand this plugin into #8737 Swift.** Keep `--tmux-parity` as the userspace stand-in.

## Coordination

| Track | Owner | Do not thrash |
|---|---|---|
| Plugin + dual-path design | This repository | — |
| PR #8736 polish / land | Native cmux fork | Review-only from this plugin unless asked |
| Issue #8737 design | Native cmux fork | No Swift implementation in this repo |
| Herdr chat/task titles | Herdr title tracks | This plugin only *reads* titles |

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
- https://github.com/RaviTharuma/cmux-herdr/issues/3 — no tagged release (**closed**; tags through v0.3.x, see [RELEASE.md](./RELEASE.md))
- https://github.com/RaviTharuma/cmux-herdr/issues/4 — upstream draft drift (**closed**; canonical banners added)
- https://github.com/RaviTharuma/cmux-herdr/issues/5 — PR #8736 missing-PATH residual (**done on PR tip**; docs closed — not a plugin runtime gap)
- https://github.com/RaviTharuma/cmux-herdr/issues/6 — unittest vs pytest docs (**closed**; `test.sh` + README note)
- https://github.com/RaviTharuma/cmux-herdr/issues/7 — dual-chat worktree thrash (**closed**; OPEN.md coordination table)
