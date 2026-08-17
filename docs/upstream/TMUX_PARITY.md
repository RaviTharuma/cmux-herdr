# tmux ↔ cmux integration parity (plugin + native)

Gold standard: cmux **`ssh-tmux` / `RemoteTmuxWindowMirror`**.
Both Herdr paths must match that contract as closely as the host allows.

| Layer | Repo | What “parity” means |
|---|---|---|
| **Plugin** | this repo | Userspace analogue: extra cmux viewers + layout/focus/order/prune. Ships today. |
| **Native** | `manaflow-ai/cmux` ([#10045](https://github.com/manaflow-ai/cmux/pull/10045) + **PR7**) | In-app copy of `RemoteTmuxWindowMirror` for Herdr: real Bonsplit panes, Ghostty surfaces, output/input, layout, resize. |

[#10045](https://github.com/manaflow-ai/cmux/pull/10045) is the **sidebar/control-socket** nested-topology v1 (virtual rows). That is *not* ssh-tmux. **PR7 (`RemoteHerdrWindowMirror` + `RemoteHerdrImpose`)** is the surface mirror. Paste-ready plan: [PR7_HERDR_WINDOW_MIRROR.md](./PR7_HERDR_WINDOW_MIRROR.md).

**Development continues.** Plugin userspace is at its ceiling (no PTY theft). Native work continues toward tmux depth: impose, host-apply, I/O, session tabs, control mutations, and attach/detach/restore/observability are landed as contracts; AppKit Bonsplit/Ghostty apply is the slice after that. Another chat owns #10045 CodeRabbit — see [LANES.md](./LANES.md).

## Capability matrix

Legend: **Yes** in the plugin column is shipped userspace. **Yes** in the Native-mirror (PR7) column is the **target contract**, not live AppKit. See [Contract vs live](#contract-vs-live-appkit) — do not read that column as “already wired into Ghostty.”

| ssh-tmux behavior | Plugin (`cmux-herdr`) | Native sidebar (#10045) | Native mirror (PR7) |
|---|---|---|---|
| Each inner **window/tab** → real cmux **tab** | Yes (`mirror` / `--tmux-parity`) | Virtual row only | Yes — one `RemoteHerdrWindowMirror` per Herdr tab |
| Each inner **pane** → real **Bonsplit pane + TerminalPanel** | Extra viewer via `attach-pane` (not PTY theft) | Virtual row only | Yes — `makePanel(paneId)` like tmux |
| Layout tree is source of truth (`horizontal` / `vertical` / leaf) | Yes (`cmux_herdr_layout.py`, same JSON shape as `RemoteTmuxLayoutNode`) | Layout hints later | Yes — parse Herdr layouts into the same node type |
| Pane create-order = DFS `paneIDsInOrder` | Yes | Snapshot order | Yes |
| Split direction from tree (not alternate right/down) | Yes | n/a | Yes — `split-window` analogue = `pane.split` |
| Divider **ratio** from assigned cells | `cmux set-ratio` via `RemoteHerdrImpose` fractions (tmux +1 cell) | n/a | Yes — `RemoteHerdrImpose.plan` → host `imposeDividerPlan()` |
| User **divider drag** → inner `resize-pane` | Session model only (`begin`/`resolve`/`end`); no Bonsplit owner | n/a | Yes — same drag-end → `pane.resize` round trip |
| Feed-forward sizing (claim size, inner mux owns grid) | SIGWINCH → `herdr pane resize` | n/a | Yes — copy tmux `updateClientSize` / `refresh-client -C` |
| Output stream into the surface | Poll `herdr pane read` + isolated `route_output` (never cross-pane; strip `ESC k` titles) | Read API later | Yes — `routeOutput(paneId, data)` into that Ghostty only |
| Typed input → inner pane | `herdr pane send` (cbreak) + `route_input` (bound pane only) | Guarded later | Yes — `pane.send_*` from Ghostty input for that pane only |
| Focus: inner active pane ↔ cmux pane | `--focus` / `--tmux-parity` + `project_focus` (provider never echoes) | `nested.node.focus` | Yes — `noteRemoteActivePane` + `select-pane` analogue |
| Tab **order** follows inner numbers | `--order` / `--tmux-parity` + `session_actions` (`create_tab` / `close_tab` / `reorder_tabs`) | Virtual row order | Yes — `TabManager` order = Herdr tab numbers |
| **Prune** gone panes (close surfaces) | `--prune` (default on `--tmux-parity`) | Event close | Yes — teardown panel like tmux reconcile |
| **Zoom** does not destroy hidden pane panels | Mapped viewers kept | n/a | Yes — base vs visible layout (copy tmux) |
| Event-driven reconcile + snapshot resync | Persistent `events.subscribe` via `watch --tmux-parity` | `events.subscribe` | Yes — events + snapshot after gaps |
| Idempotent reconcile (re-run is a no-op) | `herdr-mirror:<pane_id>` keys | Compound nested IDs | Yes — paneId → panel map |
| Titles from inner window/tab, not pane-border noise | Tab label on tab-root | Provider labels | Yes — copy tmux `windowTitle` rule |
| Single writer vs plugin | Plugin yields if native attachment live (pills + mirror) | Plugin suppression | Same |
| Engine-owned reconcile | `apply_window` drives create/prune; geometry-only skips recreate | Snapshot order | Copy tmux structure version |
| Fail-closed layout apply | Split failure never orphans a new tab | n/a | Host impose must not invent panes |
| Single size-claim writer | `size-authority-<fp>` / `CMUX_HERDR_SIZE_AUTHORITY` | n/a | One `refresh-client -C` claim |
| Socket-first topology | `session.snapshot` over Unix socket; CLI fallback | Direct adapter | Same wire |

## Contract vs live AppKit

Tmux in cmux is a **live machine**: roughly 75 `RemoteTmux*.swift` files owning `BonsplitController`, Ghostty `TerminalPanel`s, SSH/`tmux -CC`, `%output`, pane seeds, divider drag, focus rollback, and control-socket methods (`remote.tmux.*`).

Herdr today is three layers, and only the first two run in a real app:

| Layer | What it actually is |
|---|---|
| Plugin `--tmux-parity` | Extra cmux viewers (`attach-pane`). Ships. Ceiling: no PTY theft. |
| Native sidebar (#10045) | Session **navigator** (virtual rows + socket + focus). Not a window mirror. |
| Native PR7 engines | Pure reconcile / impose / host-apply / I/O / session **contracts**. Not applied onto a live `BonsplitController`. Draft twins sit on fork PRs ([#12](https://github.com/RaviTharuma/cmux/pull/12), [#13](https://github.com/RaviTharuma/cmux/pull/13)) until #10045 CodeRabbit finishes. |

Until AppKit apply lands, “Herdr has tmux parity” is false. The contracts exist so the apply slice does not invent a third model.

## Tmux-live features Herdr does not have yet

Status: **live** = runs in cmux today for tmux. Herdr column is honest.

### Must copy (user-visible ssh-tmux)

| Tmux live behavior | Herdr now | Notes |
|---|---|---|
| `makePanel` → Ghostty `TerminalPanel` per inner pane | Missing | Plugin uses a second client; native engine only diffs pane ids |
| `reconcileBonsplitTree` / `imposeDividerPlan` on a live controller | Contract only | Planner + verb list exist; no AppKit apply |
| `%output` → `surface.processRemoteOutput` | Contract only | Plugin polls `pane.read`; no Ghostty write |
| Ghostty typing → `send-keys` / named keys (Up, F1, PageDown, …) | Contract only | `encode_named_key` → `pane.send_keys` + CSI fallback |
| Input forwarder with byte budget / overflow | Contract only | `InputForwarder` (256 KiB, epoch on detach) |
| Pane **seed** (scrollback gated on Ghostty grid ready) | Contract only | `PaneSeedQueue` from `pane.read`; overflow defers full reseed |
| Title `ESC k … ST` strip on the live stream | Contract only | Plugin/native filter exists; not on a Ghostty surface |
| Provider focus does not steal first responder | Contract only | Tmux `focusBonsplitPane` skips unchanged + `isApplyingTmuxFocus` |
| Optimistic user focus + **rollback** if command rejected | Contract only | `FocusController.command_rejected` |
| Focus navigation (adjacent pane, keep first responder) | Contract only | `adjacent_pane` on the layout tree |
| User split from cmux chrome → inner `split-window` | Contract only | `request_split` → `pane.split` |
| Divider drag begin/hold/end → `resize-pane` | Contract only | No Bonsplit drag owner for Herdr |
| Feed-forward `updateClientSize` from window geometry | Contract only | Plugin SIGWINCH; no live sizing transaction / grid parity |
| Zoom: base tree keeps panels, visible tree renders | Contract only | Engine keeps ids; no live Bonsplit zoom |
| Prune gone panes / close gone tabs | Plugin yes / native contract | No live `panel.close()` / `teardown()` |
| Tab order = inner window order; drag-reorder pushes back | Plugin `move-tab` / native contract | No `reorderRemoteTmuxMirrorTabs` twin |
| Close default local tab once mirrors exist | Contract only | Tmux `closeDefaultTabsIfNeeded` |
| Session rename → workspace title (no echo loop) | Contract only | `apply_session_title` (inbound only, ANSI stripped) |
| Active-pane cwd → tab folder (background `cd` ignored) | Contract only | Not wired to `updateRemotePanelDirectory` |
| Busy-pane close confirmation | Contract only | Herdr `agent_status` working/blocked → confirm, then `pane.close` |
| Tab activity / unread / active command name | Contract only | `tab_activity` from `agent_status` (Herdr-native, richer than tmux) |
| Host close **detaches**; does not `kill-server` / `server.stop` | Contract only | `close_intent("host_tab")` → detach; AppKit must honor it |
| Attach / detach / reuse connection / beta setting | Contract only | `LifecycleController` + `betaFeatures.remoteHerdrMirror`; AppKit must own the live connection |
| Control-socket observability (`pane_surfaces`, `pane_grids`, attach/detach) | Contract only | `remote.herdr.*` twins of `remote.tmux.*` |
| “Mirror tabs like ssh-tmux” setting next to sidebar | Contract only | Setting key exists; no Settings UI yet |
| Single-writer: suppress plugin while native mirror is live | Plugin yield exists | Native AppKit must set the live marker |
| Restore after cmux restart (reattach, not stale tree) | Contract only | Persist + `plan_restore` reseeds; never `replay_tree` |

### Nice-to-have / later (tmux has them; Herdr analogue TBD)

| Tmux live behavior | Why it can wait |
|---|---|
| `respawn-pane` | Herdr panes are provider-owned processes; confirm a Herdr method before copying |
| Directional `resize-pane -L/-R/-U/-D` and `N%` | Absolute `pane.resize` is enough for v1 |
| Agent-fork new window (`requestAgentForkNewWindow`) | Herdr already has agent metadata; do not invent a third fork model |
| Alt-screen / no-reflow classification | Copy when Herdr publishes the equivalent of `pane_current_command` + alt-screen |
| UI content oracle / sizing lab tests | Need a live surface first |
| Swap-pane / join-pane | Tmux-specific; Herdr layout republish covers most cases |

### Do **not** copy (tmux transport, not the product)

SSH ControlMaster, `tmux -CC` parser, `%layout-change` wire format, control-mode line quoting, missing-tmux diagnostics. Herdr’s wire is the Unix socket (`session.snapshot` / `events.subscribe` / `pane.*`). Same **user** contract, different pipe.

## Honest remaining gaps (short)

### Plugin (cannot close without native cmux)

- No Ghostty PTY theft; `attach-pane` is a second client, like `tmux attach` from another terminal.
- No true `%output` byte stream; poll `pane.read` + incremental delta when the snapshot extends.
- No Bonsplit divider-drag → `resize-pane` (CLI has no drag session).
- `cmux split` / `set-ratio` / `move-tab` / `focus-surface` verbs differ across cmux CLI builds; the bridge tries fallbacks and records errors.
- Named keys, pane seed, busy-close, tab activity, attach/detach, and
  `remote.herdr.*` are **contracts** in the bridge; the plugin still
  cannot open Ghostty panels or a live Bonsplit tree.

### Native PR7 (must copy tmux, not invent a third model)

Do **not** treat “virtual sidebar rows” as tmux parity. ssh-tmux is a **window mirror**:

1. Herdr tab = cmux tab with its own `BonsplitController`.
2. Herdr pane = `TerminalPanel` whose I/O is bound to that pane id.
3. Herdr layout tree = imposed Bonsplit tree (tmux `reconcileBonsplitTree`).
4. Sizing is feed-forward (see cmux `docs/remote-tmux-sizing-design.md`).
5. Zoom uses base vs visible layout; panels stay alive.
6. Then the table above: seed, named keys, focus rollback, user split, attach controller, detach-on-close, restore-reattach, `remote.herdr.*`.

Sidebar nested topology (#10045) stays as the **session navigator** (workspaces/agents/status), the same way cmux still has a tmux session list beside the mirrored window.

## Plugin CLI (this repo)

```bash
cmux-herdr mirror --tmux-parity          # full ssh-tmux contract
cmux-herdr watch --tmux-parity           # live reconcile + event wait
cmux-herdr mirror --focus --order --ratios --prune --all
cmux-herdr attach-pane <pane_id>         # cbreak + SIGWINCH + ANSI read
```

Safe default remains `mirror` (current tab only, no prune) so casual use does not close extra tabs.

## Native landing sequence

1. Land or rebase [#10045](https://github.com/manaflow-ai/cmux/pull/10045) (sidebar + socket + focus).
2. Open **PR7** from [PR7_HERDR_WINDOW_MIRROR.md](./PR7_HERDR_WINDOW_MIRROR.md) against `manaflow-ai/cmux` (fork branch off current nested-topology tip).
3. Keep this plugin as fallback and as the layout-planner fixture source (`bridge/cmux_herdr_layout.py` matches the Swift node JSON).
