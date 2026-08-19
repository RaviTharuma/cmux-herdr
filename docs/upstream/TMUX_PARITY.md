# tmux ↔ cmux integration parity (plugin + native)

Gold standard: cmux **`ssh-tmux` / `RemoteTmuxWindowMirror`**.
Both Herdr paths must match that contract as closely as the host allows.

| Layer | Repo | What “parity” means |
|---|---|---|
| **Plugin** | this repo | Userspace analogue: extra cmux viewers + layout/focus/order/prune. Ships today. |
| **Native** | `manaflow-ai/cmux` ([#10045](https://github.com/manaflow-ai/cmux/pull/10045) + **PR7**) | In-app copy of `RemoteTmuxWindowMirror` for Herdr: real Bonsplit panes, Ghostty surfaces, output/input, layout, resize. |

[#10045](https://github.com/manaflow-ai/cmux/pull/10045) is the **sidebar/control-socket** nested-topology v1 (virtual rows). That is *not* ssh-tmux. **PR7 (`RemoteHerdrWindowMirror` + `RemoteHerdrImpose`)** is the surface mirror. Paste-ready plan: [PR7_HERDR_WINDOW_MIRROR.md](./PR7_HERDR_WINDOW_MIRROR.md).

**Development continues.** The live apply *machine* now runs in the plugin (`LiveApplyHost`: makePanel, output, drag, focus, size, attach/restore). Plugin surfaces are still in-memory Ghostty analogues (no PTY theft). Native AppKit must swap those surfaces for real `TerminalPanel`s. Native review of #10045 is a separate track — see [LANES.md](./LANES.md).

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
| Feed-forward sizing (claim size, inner mux owns grid) | No one-shot claim: `pane.resize` is split-edge only; `terminal session --cols/--rows` is a live stream | n/a | Target: copy tmux `updateClientSize`; do not invent `pane.resize` cols/rows |
| Event-driven reconcile + snapshot resync | Persistent `events.subscribe` + SessionHost pump (`watch --tmux-parity`) | `events.subscribe` | Yes — events + snapshot after gaps |
| Output stream into the surface | Pump `pane.read` → isolated `route_read_snapshot` (never cross-pane; strip `ESC k` titles) | Read API later | Yes — `routeOutput(paneId, data)` into that Ghostty only |
| Typed input → inner pane | `herdr pane send-text` (cbreak) + `route_input` (bound pane only) | Guarded later | Yes — `pane.send_*` from Ghostty input for that pane only |
| Focus: inner active pane ↔ cmux pane | `--focus` / `--tmux-parity` + `project_focus` (provider never echoes) | `nested.node.focus` | Yes — `noteRemoteActivePane` + `select-pane` analogue |
| Tab **order** follows inner numbers | `--order` / `--tmux-parity` + `session_actions` (`create_tab` / `close_tab` / `reorder_tabs`) | Virtual row order | Yes — `TabManager` order = Herdr tab numbers |
| **Prune** gone panes (close surfaces) | `--prune` (default on `--tmux-parity`) | Event close | Yes — teardown panel like tmux reconcile |
| **Zoom** does not destroy hidden pane panels | Mapped viewers kept | n/a | Yes — base vs visible layout (copy tmux) |
| Idempotent reconcile (re-run is a no-op) | `herdr-mirror:<pane_id>` keys | Compound nested IDs | Yes — paneId → panel map |
| Titles from inner window/tab, not pane-border noise | Tab label on tab-root | Provider labels | Yes — copy tmux `windowTitle` rule |
| Single writer vs plugin | Shared lease: yield, resume on stale, same restore file | Plugin suppression | Same files (`RemoteHerdrHandoff`) |
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
| `makePanel` → Ghostty `TerminalPanel` per inner pane | Live machine | Plugin: in-memory `GhosttySurface`. Native must bind `TerminalPanel` |
| `reconcileBonsplitTree` / `imposeDividerPlan` on a live controller | Live machine | `LiveWindowMirror.apply_window` runs host verbs in tmux order |
| `%output` → `surface.processRemoteOutput` | Live machine | Isolated write + title strip; native swaps the surface |
| Ghostty typing → `send-keys` / named keys (Up, F1, PageDown, …) | Live machine | `send-key` CLI + `InputForwarder` |
| Input forwarder with byte budget / overflow | Live machine | 256 KiB, epoch on detach |
| Pane **seed** (scrollback gated on Ghostty grid ready) | Live machine | `seed_pane` waits for grid match |
| Title `ESC k … ST` strip on the live stream | Live machine | Stripped before `process_remote_output` |
| Provider focus does not steal first responder | Live machine | `is_applying_focus` leaves the keyboard alone |
| Optimistic user focus + **rollback** if command rejected | Live machine | `FocusController.command_rejected` |
| Focus navigation (adjacent pane, keep first responder) | Live machine | `navigate_focus` |
| User split from cmux chrome → inner `split-window` | Live machine | `user_split` → `pane.split` |
| Divider drag begin/hold/end → `resize-pane` | Live machine | `begin_drag` / `end_drag`; native must own the Bonsplit divider |
| Feed-forward `updateClientSize` from window geometry | Live machine | Claims from container + cell metrics only |
| Zoom: base tree keeps panels, visible tree renders | Live machine | Hidden pane stays `live` |
| Prune gone panes / close gone tabs | Live machine | `close_panel` / session `close_tab` |
| Tab order = inner window order; drag-reorder pushes back | Plugin `move-tab` / live session host | Native TabManager still to apply |
| Close default local tab once mirrors exist | Live machine | `close_default_tabs` |
| Session rename → workspace title (no echo loop) | Live machine | inbound `apply_session_title` |
| Active-pane cwd → tab folder (background `cd` ignored) | Live machine | `route_cwd` + focus promote; native calls `updateRemotePanelDirectory` |
| Busy-pane close confirmation | Live machine | `agent_status` → `confirm_then_close_pane` |
| Tab activity / unread / active command name | Live machine | `tab_activity` |
| Host close **detaches**; does not `kill-server` / `server.stop` | Live machine | `detach()` |
| Attach / detach / reuse connection / beta setting | Live machine | `LiveApplyHost.attach` + `SETTING_KEY` |
| Control-socket observability (`pane_surfaces`, `pane_grids`, attach/detach) | Live machine | `cmux-herdr observe` |
| “Mirror tabs like ssh-tmux” setting next to sidebar | Live machine (key) | Native Settings row still to land |
| Single-writer: one lease, resume if the other path dies | Live machine | `cmux_herdr_handoff` (pid + heartbeat; shared restore) |
| Restore after cmux restart (reattach, not stale tree) | Live machine | `restore()` reseeds; never `replay_tree` |

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

This is the plugin ceiling. Further depth is AppKit, not more Python.

- No Ghostty PTY theft; `attach-pane` is a second client, like `tmux attach` from another terminal. Reads/sends/resizes now use the same Unix-socket RPC as native SessionHost, then CLI fallback.
- No true `%output` byte stream; poll `pane.read` + incremental delta when the snapshot extends. Subscribe gaps force a snapshot resync. Timeout ticks paint only (no chrome remirror).
- No Bonsplit divider-drag → `resize-pane` (the plugin has no Bonsplit owner). Native [RaviTharuma/cmux#17](https://github.com/RaviTharuma/cmux/pull/17) owns that host path.
- `cmux split` / `set-ratio` / `move-tab` / `focus-surface` verbs differ across cmux CLI builds; the bridge tries fallbacks and records errors.
- The live apply machine runs `make_panel` / output / drag / focus /
  size / attach in-process. Surfaces are in-memory Ghostty analogues — the
  plugin cannot open a real `TerminalPanel`.

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
cmux-herdr watch --tmux-parity           # attach, pump I/O, remirror, detach on stop
cmux-herdr attach / detach / restore     # apply-host lifecycle (never server.stop)
cmux-herdr api --list                    # published Herdr methods
cmux-herdr new-tab / close-pane / send   # inner mux control (socket-first)
cmux-herdr set-ratio --tab w2:t1 --ratio 0.6
cmux-herdr move-pane w2:p2 --tab w2:t2 --split right
cmux-herdr focus-dir right
cmux-herdr move-tab w2:t1 --index 0
cmux-herdr rename-pane w2:p2 logs
cmux-herdr start-agent reviewer --kind codex --pane w2:p3
cmux-herdr notify "sync complete"
cmux-herdr send-key <pane_id> C-Up       # encodes to Herdr ctrl+up
cmux-herdr observe --method pane_surfaces
cmux-herdr attach-pane <pane_id>         # cbreak + SIGWINCH + ANSI read
```

Safe default remains `mirror` (current tab only, no prune) so casual use does not close extra tabs.

## Herdr beyond tmux

Agent/worktree/manifest/title verbs that ssh-tmux does not have: see
[HERDR_BEYOND_TMUX.md](./HERDR_BEYOND_TMUX.md).



### Native lease wire (2026-08-19)

`RemoteHerdrController` claims `RemoteHerdrHandoff` (`writer-*` / `native-live`) on attach and releases on detach, so plugin `sync` / `watch` / `mirror` yield while the mirror is live. Nested sidebar still uses `NestedPluginWriterHandoff` locks in parallel.

### Native size-authority wire (2026-08-19)

Same attach path also claims `size-authority-<fingerprint>` with the `native` sentinel (cleared on detach). Plugin `attach-pane` SIGWINCH handlers call `may_claim_client_size`, which no-ops when native owns the writer lease **or** when the file is `native` / `native:*`. Inspect via `cmux-herdr lease` / `doctor` (`size_authority` check).

### Native lease heartbeat (2026-08-19)

Lease freshness requires a live pid **and** `heartbeat_ms` within TTL (45s). `RemoteHerdrController` refreshes `heartbeatNative` + size-authority every ~15s while any session host is live, and samples wall clock before each write (so `heartbeat_ms` is not frozen at store init). Plugin `watch` already calls `heartbeat_plugin_writer`.

### Native title-lock association wire (2026-08-19)

`RemoteHerdrAssociationStore` writes plugin-format `associations-<fingerprint>.json` (`title_lock` / `locked_title`) on tab/pane title updates. Plugin sync after detach respects those locks. `NestedPluginWriterHandoff` locks now carry `pid` + `heartbeat_ms` and expire when stale (same TTL), refreshed on the controller heartbeat loop.

## Native landing sequence

1. Land or rebase [#10045](https://github.com/manaflow-ai/cmux/pull/10045) (sidebar + socket + focus).
2. Open **PR7** from [PR7_HERDR_WINDOW_MIRROR.md](./PR7_HERDR_WINDOW_MIRROR.md) against `manaflow-ai/cmux` (fork branch off current nested-topology tip).
3. Keep this plugin as fallback and as the layout-planner fixture source (`bridge/cmux_herdr_layout.py` matches the Swift node JSON).
