# tmux ↔ cmux integration parity (plugin + native)

Gold standard: cmux **`ssh-tmux` / `RemoteTmuxWindowMirror`**.
Both Herdr paths must match that contract as closely as the host allows.

| Layer | Repo | What “parity” means |
|---|---|---|
| **Plugin** | this repo | Userspace analogue: extra cmux viewers + layout/focus/order/prune. Ships today. |
| **Native** | `manaflow-ai/cmux` ([#10045](https://github.com/manaflow-ai/cmux/pull/10045) + **PR7**) | In-app copy of `RemoteTmuxWindowMirror` for Herdr: real Bonsplit panes, Ghostty surfaces, output/input, layout, resize. |

[#10045](https://github.com/manaflow-ai/cmux/pull/10045) is the **sidebar/control-socket** nested-topology v1 (virtual rows). That is *not* ssh-tmux. **PR7 (`RemoteHerdrWindowMirror` + `RemoteHerdrImpose`)** is the surface mirror. Paste-ready plan: [PR7_HERDR_WINDOW_MIRROR.md](./PR7_HERDR_WINDOW_MIRROR.md).

**Development continues.** Plugin userspace is at its ceiling (no PTY theft). Native work continues toward tmux depth: the impose planner is the next landed slice; AppKit Bonsplit/Ghostty apply is the slice after that.

## Capability matrix

Legend: **Yes** = required for tmux parity. Plugin column is what this repo implements. Native v1 sidebar = #10045. Native mirror = PR7.

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
| Output stream into the surface | Poll `herdr pane read` (ANSI/raw) | Read API later | Yes — subscribe pane output / `pane.read` push |
| Typed input → inner pane | `herdr pane send` (cbreak) | Guarded later | Yes — `pane.send_*` from Ghostty input |
| Focus: inner active pane ↔ cmux pane | `--focus` / `--tmux-parity` | `nested.node.focus` | Yes — `noteRemoteActivePane` + `select-pane` analogue |
| Tab **order** follows inner numbers | `--order` / `--tmux-parity` | Virtual row order | Yes — `TabManager` order = Herdr tab numbers |
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

## Honest remaining gaps

### Plugin (cannot close without native cmux)

- No Ghostty PTY theft; `attach-pane` is a second client, like `tmux attach` from another terminal.
- No true `%output` byte stream; poll `pane.read` + incremental delta when the snapshot extends.
- No Bonsplit divider-drag → `resize-pane` (CLI has no drag session).
- `cmux split` / `set-ratio` / `move-tab` / `focus-surface` verbs differ across cmux CLI builds; the bridge tries fallbacks and records errors.

### Native PR7 (must copy tmux, not invent a third model)

Do **not** treat “virtual sidebar rows” as tmux parity. ssh-tmux is a **window mirror**:

1. Herdr tab = cmux tab with its own `BonsplitController`.
2. Herdr pane = `TerminalPanel` whose I/O is bound to that pane id.
3. Herdr layout tree = imposed Bonsplit tree (tmux `reconcileBonsplitTree`).
4. Sizing is feed-forward (see cmux `docs/remote-tmux-sizing-design.md`).
5. Zoom uses base vs visible layout; panels stay alive.

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
