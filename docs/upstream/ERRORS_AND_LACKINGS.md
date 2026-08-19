# Errors & lackings (frozen inventory)

**Freeze ID:** `freeze-2026-08-19T065836Z`  
**Frozen at:** `2026-08-19T06:58:36Z`  
**Repo:** `RaviTharuma/cmux-herdr`

This is the durable snapshot of **what is still wrong, blocked, or missing** across Herdr ↔ cmux.
It does **not** claim ownership of hot product tips. Product code continues on `#10045` / `#8736`;
this file only inventories.

Machine-readable twin: [`ERRORS_AND_LACKINGS.json`](./ERRORS_AND_LACKINGS.json).  
Companions: [OPEN.md](../../OPEN.md), [TMUX_PARITY.md](./TMUX_PARITY.md),
[PARITY_MATRIX.md](./PARITY_MATRIX.md), [AGENT_LANES.md](./AGENT_LANES.md), [STATUS.json](./STATUS.json),
[PR7_HERDR_WINDOW_MIRROR.md](./PR7_HERDR_WINDOW_MIRROR.md), [ANNOYANCES.md](./ANNOYANCES.md).

---

## Snapshot (tips at freeze)

| Track | Tip SHA | Tip message | mergeable | mergeStateStatus | Threads | Unresolved | CodeRabbit |
|-------|---------|-------------|---------|------------------|---------|------------|------------|
| [#10045](https://github.com/manaflow-ai/cmux/pull/10045) native | `b02b8a954327` | fix(herdr): run nested.topology socket tests off the main thread | MERGEABLE | **BLOCKED** | 173 | **0** | success |
| [#8736](https://github.com/manaflow-ai/cmux/pull/8736) plugin shim | `2f483ad94f0d` | Reject alias-local --json and load Herdr-compat strings from the app bundle | MERGEABLE | **UNSTABLE** | 14 | **0** | success |

Fork tip `RaviTharuma/cmux@cursor/nested-topology-herdr-v1-becf` matches `#10045` tip (`b02b8a954327`).

Tip check-runs (both PRs): Socket Security success ×2; cubic / Vercel Agent Review **neutral**; `[code]smith` **skipped**. **No macOS / `swift test` / Xcode check visible.**

`reviewDecision` on both: **null** (only `COMMENTED` reviews; no approving maintainer review recorded).

Mirror sources **are present on `#10045` tip** (`RemoteHerdrWindowMirrorHost.swift`, `RemoteHerdrSessionHost.swift`, `RemoteHerdrLiveApply.swift`) — acceptance / dogfood still open.

---

## 1. Merge / process blockers (errors)

| ID | Severity | Item | Evidence at freeze |
|----|----------|------|--------------------|
| E-MERGE-10045 | **blocker** | Upstream [#10045](https://github.com/manaflow-ai/cmux/pull/10045) cannot merge | `mergeStateStatus=BLOCKED` (branch protection / required review). Tip `b02b8a954327`. |
| E-MERGE-8736 | **blocker** | Upstream [#8736](https://github.com/manaflow-ai/cmux/pull/8736) unstable | `mergeStateStatus=UNSTABLE`. Tip `2f483ad94f0d`. |
| E-NO-APPROVAL | **blocker** | No approving review on either upstream PR | `reviewDecision=null`; review states are `COMMENTED` only (CodeRabbit + RaviTharuma). |
| E-AUTH-REPLY | **ops** | Agents cannot reply/resolve review threads on `manaflow-ai/cmux` | MCP as RaviTharuma + `gh` as cursor[bot] → HTTP 403 on review replies / `resolveReviewThread`. Browser agents often not logged into GitHub. |
| E-AUTH-PR | **ops** | Agents cannot open/merge PRs on `cmux-herdr` via `gh` | `Resource not accessible by integration` on `createPullRequest` / auto-merge. Cursor `ManagePullRequest` needs user approval. |
| E-AUTH-UPSTREAM-WRITE | **ops** | Agents cannot vote/comment on upstream discussion / issues | Same 403 wall for `manaflow-ai/cmux` writes (poll [#10106](https://github.com/manaflow-ai/cmux/discussions/10106)). |
| E-CI-MACOS | **gap** | No visible macOS / `swift test` / Xcode check on PR tips | Tip check-runs only Socket Security + neutral reviewers + skipped codesmith. Package tests / dogfood not proven CI-green on upstream. |
| E-DOGFOOD | **gap** | macOS dogfood of native mirror unchecked | PR7 acceptance checkboxes still open (see §2 / PR7 doc). |
| E-DOC-STALE | **hygiene** | Root [OPEN.md](../../OPEN.md) lagging live merge states | Still says `#8736` “mergeable/clean” and `#10045` “dirty”; live freeze: BLOCKED / UNSTABLE, CR threads 0. Prefer this freeze + STATUS.json. |

CodeRabbit on both tips: **0 unresolved** threads (review noise cleared; merge still blocked/unstable).

---

## 2. Product lackings vs ssh-tmux (native)

Gold standard: `RemoteTmuxWindowMirror`. Target: PR7 / host mirror (code on `#10045` tip; acceptance open).

| ID | Severity | Lacking | Notes |
|----|----------|---------|-------|
| L-OUTPUT-STREAM | **major** | No Herdr `%output` byte stream | Native uses bounded **poll** `pane.read` (150ms busy / 500ms idle). Latency & CPU scale with pane count. |
| L-SIZING-CLAIM | **major** | Feed-forward client-size claim not full tmux parity | Herdr `pane.resize` is split-edge oriented; tmux `refresh-client -C` style claim still partial. |
| L-TMUX-HELPERS | **minor** | Not every `RemoteTmuxWindowMirror+*` helper ported | Remaining polish: sizing-transaction helpers, edge cases. |
| L-ACCEPT-PR7 | **major** | PR7 acceptance checkboxes open | Attach→mirror tabs, split without outer duplicate, input routing, close/detach (no `server.stop`), plugin suppressed while live. See [PR7_HERDR_WINDOW_MIRROR.md](./PR7_HERDR_WINDOW_MIRROR.md) §Acceptance. |
| L-SIDEBAR-ACTIONS | **major** | Sidebar (#10045) mutations still incomplete vs matrix | PARITY_MATRIX: send/split/resize/close on sidebar path still “Later/guarded”; mirror is the mutation surface. |
| L-PROVIDER-UUID | **major** | Herdr `ping` lacks durable server UUID | Path reuse/restart detection incomplete; cmux uses connection generation / endpoint hash as stopgap. |
| L-EVENT-CURSOR | **major** | Event stream has no resumable cursor | After gaps: mark stale, reconnect, full snapshot (no incremental resume). |
| L-CAP-TABLE | **minor** | Protocol 17 exposes booleans, not method inventory | cmux keeps a tested protocol compatibility table; unknown versions degrade. |
| L-MOBILE | **wontfix-v1** | Mobile/remote cmux | Local Unix socket only; omit or read-only projection in v1. |
| L-NO-SERVER-STOP | **by-design** | Host close must not `server.stop` | Intentional non-goal; verify under dogfood (acceptance). |

---

## 3. Plugin / stopgap lackings (explicit ceilings, not bugs)

From [OPEN.md](../../OPEN.md) + [TMUX_PARITY.md](./TMUX_PARITY.md):

| ID | Severity | Lacking |
|----|----------|---------|
| L-PLUGIN-PTY | **ceiling** | No Ghostty PTY theft — `attach-pane` is a second client |
| L-PLUGIN-OUTPUT | **ceiling** | Poll `pane.read`; no true `%output` |
| L-PLUGIN-BONSPLIT | **ceiling** | No Bonsplit owner → no divider-drag → `resize-pane` in plugin |
| L-PLUGIN-FLAT | **ceiling** | Flat status-pill projection; not first-class nested hierarchy |
| L-PLUGIN-SURFACE | **ceiling** | Live apply machine runs; surfaces in-memory until native `TerminalPanel` path |
| L-PLUGIN-REATTACH | **ceiling** | No reattach model after cmux restart (local binding only) |
| L-PLUGIN-INSTALL | **ceiling** | No Homebrew / cmux registry / signed install channel |
| L-PLUGIN-CLI-VARIANCE | **watch** | `cmux split` / `set-ratio` / `move-tab` / `focus-surface` vary across CLI builds |

---

## 4. Shared contract still fragile

| ID | Severity | Item |
|----|----------|------|
| L-SINGLE-WRITER | **watch** | Plugin must no-op when native attachment live (handoff/lease). Fork [#18](https://github.com/RaviTharuma/cmux/pull/18) / native handoff — keep verified under dogfood. |
| L-TITLE-LOCK | **watch** | Native-title lock + heuristic-once must not thrash under dual plugin+native |
| L-ASSOC-KEY | **watch** | Plugin `pane_id:session_id` is cache only — never public nested ID |
| L-SIZE-AUTHORITY | **watch** | Single size-claim writer (`size-authority` / claim) — dual writers thrash SIGWINCH |

---

## 5. Branch / PR sprawl (hygiene risk)

Open **fork** PRs on `RaviTharuma/cmux` (may be superseded by `#10045` tip — verify before landing):

| # | Head → base | Title | Updated |
|---|-------------|-------|---------|
| [18](https://github.com/RaviTharuma/cmux/pull/18) | `cursor/herdr-handoff-6e7a` → live-machine | RemoteHerdrHandoff / plugin lease | 2026-08-17 |
| [17](https://github.com/RaviTharuma/cmux/pull/17) | `cursor/herdr-native-mirror-f1c1` → nested tip | Native Herdr window mirror | 2026-08-18 |
| [16](https://github.com/RaviTharuma/cmux/pull/16) | `cursor/herdr-live-machine-6e7a` → lifecycle | Live apply machine | 2026-08-17 |
| [14](https://github.com/RaviTharuma/cmux/pull/14) | `cursor/herdr-lifecycle-6e7a` → io-session | RemoteHerdrLifecycle | 2026-08-17 |
| [13](https://github.com/RaviTharuma/cmux/pull/13) | `cursor/herdr-io-session-6e7a` → host-apply | Pane I/O + session apply | 2026-08-17 |
| [12](https://github.com/RaviTharuma/cmux/pull/12) | `cursor/herdr-host-apply-6e7a` → nested tip | RemoteHerdrHostApply | 2026-08-18 |

Herdr-related heads on `cmux-herdr` (do not force-push shared tips):  
`cursor/agent-lanes-f1c1`, `cursor/herdr-api-doc-fixes-b5e6`, `cursor/herdr-cmux-knowledge-corpus-f64c`, `cursor/herdr-tmux-parity-advanced-f1c1`, `cursor/plugin-*`, `cursor/tmux-*`, `cursor/upstream-*`, plus stacked `cursor/herdr-*-6e7a` lanes. Full live list in [STATUS.json](./STATUS.json).

---

## 6. Upstream design / engagement still open

| Artifact | Status at freeze |
|----------|------------------|
| Issue [#8737](https://github.com/manaflow-ai/cmux/issues/8737) | OPEN — full nested topology design (updated 2026-08-01) |
| Issue [#9363](https://github.com/manaflow-ai/cmux/issues/9363) | OPEN — Foundation: provider-neutral nested topology model |
| Discussion [#10106](https://github.com/manaflow-ai/cmux/discussions/10106) | OPEN — **1 upvote, 0 comments** (engagement still thin) |
| PR [#10045](https://github.com/manaflow-ai/cmux/pull/10045) | OPEN — native nested + mirror; **merge BLOCKED** |
| PR [#8736](https://github.com/manaflow-ai/cmux/pull/8736) | OPEN — CLI compat shim; **merge UNSTABLE** |

Related filed (not Herdr blockers today): cmux [#8743](https://github.com/manaflow-ai/cmux/issues/8743) / [#8744](https://github.com/manaflow-ai/cmux/issues/8744) (PATH / CLI hygiene — largely addressed on #8736 tip; confirm before closing).

Do **not** confuse [#8673](https://github.com/manaflow-ai/cmux/issues/8673) (Pi extension) with Herdr.

---

## 7. What is *not* an error right now

- CodeRabbit unresolved review threads on `#10045` / `#8736`: **0**
- Socket Security checks on tips: **success**
- Combined commit status context CodeRabbit: **success**
- Fork tip SHA aligned with `#10045` tip
- Intentional non-goals: no `server.stop` on host close; no CLI shell-out on native path; sidebar ≠ ssh-tmux
- Plugin open GitHub issues on `cmux-herdr`: **none** at freeze (residuals tracked here / OPEN.md)

---

## 8. Counts (freeze)

| Bucket | Count |
|--------|-------|
| Errors (blockers/ops/gaps/hygiene) | **9** |
| Native lackings | **10** |
| Plugin ceilings / watches | **8** |
| Shared contract watches | **4** |
| Open fork PRs (RaviTharuma/cmux) | **6** |
| Upstream design artifacts open | **5** (#8737, #9363, #10106, #10045, #8736) |

---

## 9. Refresh rule

When agents go idle or tip SHAs move, bump `freeze_*` + this file + `STATUS.json` on `cursor/agent-lanes-f1c1` (integration_ops lane). Do not rewrite hot product tips from this inventory.

Human-only next steps for blockers: approving review + branch policy for merge; RaviTharuma PAT or logged-in browser for any remaining upstream writes; macOS dogfood of PR7 acceptance.

---

## 10. Progress since freeze (2026-08-19 follow-up)

| Item | Status |
|------|--------|
| E-DOC-STALE | **fixed** on `cmux-herdr` main (#36) |
| `RemoteHerdrHandoff` on `#10045` tip | **landed** via fork [#19](https://github.com/RaviTharuma/cmux/pull/19) → tip `40a2f842c435` |
| Fork drafts #12–#18 | **closed** (superseded by tip) |
| Herdr-beyond-tmux CLI | **shipping** — see [HERDR_BEYOND_TMUX.md](./HERDR_BEYOND_TMUX.md) |
| E-MERGE-10045 / E-MERGE-8736 / E-NO-APPROVAL | **still open** (needs maintainer on `manaflow-ai/cmux`) |
| E-CI-MACOS / E-DOGFOOD / plugin ceilings | **still open** (need AppKit / dogfood) |

| Native claims plugin lease files | **landed** tip `5e33880b165b` via fork [#20](https://github.com/RaviTharuma/cmux/pull/20) — `RemoteHerdrController` → `RemoteHerdrHandoff.claimNative` |
| Plugin OSS v0.3.4 | **released** (MIT, CI, community files) |
