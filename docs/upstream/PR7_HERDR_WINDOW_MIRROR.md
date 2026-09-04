# PR7 — RemoteHerdrWindowMirror (tmux parity)

Paste-ready follow-up to [manaflow-ai/cmux#10045](https://github.com/manaflow-ai/cmux/pull/10045) and issue [#8737](https://github.com/manaflow-ai/cmux/issues/8737).

Base: nested-topology tip (`cursor/nested-topology-herdr-v1-becf` / #10045). Do **not** mix with `#8736` (`__herdr-compat`).

## Implementation status

The **pure reconcile engine + impose planner** live in `Packages/macOS/CmuxNestedTopology` (fork PR [RaviTharuma/cmux#8](https://github.com/RaviTharuma/cmux/pull/8) and follow-ups on #10045):

- `RemoteHerdrLayoutNode` / `RemoteHerdrWindow` / `RemoteHerdrWindowMirror` / `RemoteHerdrSessionMirror`
- `RemoteHerdrSizing` (feed-forward client grid)
- `RemoteHerdrImpose` / `RemoteHerdrImposePlan` — tmux `imposeDividerPlan` contract (binary tree, leaf expand/remove, `plan(w) <= w`, drag hold)
- `RemoteHerdrPaneIO` on `HerdrNestedTopologyClient` (`pane.send_text` / `split` / `resize` / `close` / `read`)
- `RemoteHerdrOutput` (incremental `pane.read` delta)
- `RemoteHerdrHostApply` / `RemoteHerdrPaneRoute` / `RemoteHerdrSessionApply` (draft fork files; not on #10045 yet)
- `RemoteHerdrLifecycle` (draft; attach/detach/restore/`remote.herdr.*`)
- `session.snapshot` now decodes `layouts` (map or `[{tab_id, layout}]`)
- Tests: `RemoteHerdrWindowMirrorTests.swift` + `RemoteHerdrImposeTests`

Plugin counterparts: `src/impose.rs`, `src/host.rs`, `src/io.rs`, `src/session.rs`,
`src/control.rs`, `src/lifecycle.rs`, and `src/live.rs` (running apply host).

**Still host-side (cmux app):** apply the impose plan onto a live `BonsplitController`, Ghostty `TerminalPanel` I/O, and the divider-drag UI that feeds `begin`/`end`. Development of that apply path continues; the planner is no longer a doc-only gap.

## Summary

Copy **`RemoteTmuxWindowMirror`** for Herdr.

#10045 gives a provider-owned **sidebar subtree** and `nested.node.focus`. That is not ssh-tmux. ssh-tmux makes each inner window a real cmux tab whose panes are real Bonsplit + Ghostty surfaces.

This PR adds `RemoteHerdrWindowMirror` / `RemoteHerdrSessionMirror` so Herdr gets the same **surface** contract tmux already has.

### Non-goals for this PR

- Do not replace #10045 sidebar/control-socket (keep it as the session navigator).
- Do not shell out to the `herdr` CLI (Unix socket only, same as #10045).
- Do not invent a third layout model — reuse `RemoteTmuxLayoutNode` JSON (`pane` / `horizontal` / `vertical` + cell rects). Herdr layouts already round-trip through the Rust planner in `cmux-herdr` (`src/layout.rs`).

## Architecture (mirror tmux 1:1)

```text
Herdr session.snapshot + events.subscribe
        │
RemoteHerdrSessionMirror
        │  one window mirror per Herdr tab
        ▼
RemoteHerdrWindowMirror
        ├─ layout / visibleLayout / zoomed     (tmux apply(window:))
        ├─ panelsByPaneId: [PaneID: TerminalPanel]
        ├─ bonsplitController                  (imposeDividerPlan)
        ├─ routeOutput(paneId, data)           (tmux %output)
        ├─ sendKeys → pane.send_*              (tmux send-keys)
        ├─ focus → agent.focus (id) / pane.focus_direction
        ├─ split  → pane.split
        ├─ resize → pane.resize  (drag-end; direction + amount)
        └─ close  → pane.close   (gone from snapshot)
```

Sizing: **feed-forward**, identical invariants to `docs/remote-tmux-sizing-design.md`:

1. Claim reads only window geometry + chrome + cell metrics.
2. Herdr owns the grid (pane rects from snapshot/layouts).
3. Render imposes first-child extents on Bonsplit.
4. Divider drag is a session; settled layout after `pane.resize` is truth.
5. Zoom never tears down hidden panels (base tree vs visible tree).

## Files to add (suggested)

Under `Sources/` next to the tmux mirror (or `Packages/macOS/CmuxNestedTopology` if maintainers prefer the package):

| File | Role |
|---|---|
| `RemoteHerdrLayoutNode.swift` | Typealias or thin wrapper over `RemoteTmuxLayoutNode` + string pane ids |
| `RemoteHerdrWindow.swift` | tab id, title, layout, visibleLayout, zoomed, activePaneId |
| `RemoteHerdrWindowMirror.swift` | copy of `RemoteTmuxWindowMirror` lifecycle (`reconcile`, `routeOutput`, `teardown`) |
| `RemoteHerdrWindowMirror+Bonsplit.swift` | impose tree |
| `RemoteHerdrWindowMirror+DividerSizing.swift` | drag → `pane.resize` |
| `RemoteHerdrSessionMirror.swift` | tab set ↔ cmux tabs; order = Herdr tab numbers |
| `HerdrControlConnection+Output.swift` | subscribe/read push into `routeOutput` |
| Tests | layout parse (golden JSON from plugin fixtures), reconcile create/close, zoom keeps panels, focus projection, ratio imposition |

Reuse, do not fork: Bonsplit impose helpers, geometry snapshot, output-parity re-arm. Parameterize pane id as `String` (Herdr `w2:p34`) instead of `Int` (tmux `%N`).

## Provider methods (protocol 17+)

Required for mirror v1 (capability-gated; disable chrome if missing):

- `session.snapshot` (tabs, panes, **layouts**, focused ids)
- `events.subscribe` (tab/pane/layout/focus)
- `pane.read` or output push
- `pane.send_text` / `pane.send_keys`
- `agent.focus` (pane id / agent name) and `pane.focus_direction` (compass)
  — there is no `pane.focus` method; the live event is `pane.focused`
- `pane.split`
- `pane.resize`
- `pane.close` (user-initiated only; host close still detaches)

## Tests (must land with the PR)

- Layout JSON round-trip against plugin fixtures (`horizontal`/`vertical`/`pane`
  and official Herdr BSP `type: split` / `direction: right|down`).
- Reconcile: new pane creates a panel; gone pane closes it; geometry-only does not bump structure version.
- Zoom: visible tree is one leaf; `panelsByPaneId` still has every base pane.
- Focus: snapshot focused id → Bonsplit selection; user click → provider focus (no loop).
- Order: Herdr tab numbers 1..n match cmux tab order.
- Sizing: claim is independent of measured pane frames (port the tmux unit tests).
- Security: same UID/mode/path identity as #10045; no CLI.

## Acceptance

- [ ] Attaching Herdr to a host surface can **either** show the sidebar tree (#10045) **or** open a mirrored tab set (this PR), gated by the same beta + a “Mirror tabs like ssh-tmux” setting (default on once stable).
- [ ] Splitting a Herdr pane updates the cmux split tree without creating a duplicate outer tab.
- [ ] Typing in a mirrored pane reaches only that Herdr pane.
- [ ] Closing a Herdr pane closes the cmux panel; closing the cmux tab detaches the mirror and does not `server.stop`.
- [ ] Plugin `watch --tmux-parity` is suppressed while this mirror is live (existing single-writer lock from #10045).

## PR body (short)

```markdown
## Summary
Adds RemoteHerdrWindowMirror — ssh-tmux parity for Herdr. Each Herdr tab
becomes a real cmux tab; each pane a Bonsplit + Ghostty surface. Layout,
focus, input, output, split, resize, zoom, and prune follow
RemoteTmuxWindowMirror. Complements nested sidebar topology (#10045);
does not replace it.

## Testing
- [ ] package tests for layout/reconcile/zoom/focus
- [ ] dogfood: Beta → Nested Topology → Mirror tabs like ssh-tmux
```
