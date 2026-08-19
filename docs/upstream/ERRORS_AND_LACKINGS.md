# Errors & lackings (frozen inventory)

Updated: `2026-08-19T06:55:56Z`

Living snapshot of **what is still wrong, blocked, or missing** across Herdr ↔ cmux.
Does **not** claim ownership of hot tips. Product code continues on `#10045` / `#8736`;
this file only inventories.

Canonical companions: [OPEN.md](../../OPEN.md), [TMUX_PARITY.md](./TMUX_PARITY.md),
[PARITY_MATRIX.md](./PARITY_MATRIX.md), [AGENT_LANES.md](./AGENT_LANES.md), [STATUS.json](./STATUS.json).

---

## 1. Merge / process blockers (errors)

| ID | Severity | Item | Evidence |
|----|----------|------|----------|
| E-MERGE-10045 | **blocker** | Upstream [#10045](https://github.com/manaflow-ai/cmux/pull/10045) cannot merge | `mergeStateStatus=BLOCKED` (branch protection / required review). Tip `b02b8a954327`. |
| E-MERGE-8736 | **blocker** | Upstream [#8736](https://github.com/manaflow-ai/cmux/pull/8736) unstable | `mergeStateStatus=UNSTABLE`. Tip `2f483ad94f0d`. |
| E-AUTH-REPLY | **ops** | Agents cannot reply/resolve review threads on `manaflow-ai/cmux` | MCP as RaviTharuma + `gh` as cursor[bot] → HTTP 403 on review replies / `resolveReviewThread`. |
| E-AUTH-PR | **ops** | Agents cannot open/merge PRs on `cmux-herdr` via `gh` | `Resource not accessible by integration` on `createPullRequest` / auto-merge. Cursor `ManagePullRequest` needs manual user approval. |
| E-CI-MACOS | **gap** | No visible macOS / `swift test` / Xcode check on PR tips | Tip check-runs only Socket Security + neutral reviewers + skipped codesmith. Package tests / dogfood not proven in CI green on upstream. |
| E-DOGFOOD | **gap** | macOS dogfood of native mirror unchecked | PR7 acceptance / historical PR body still list dogfood boxes open. |

CodeRabbit on both tips: **0 unresolved** threads (review noise cleared; merge still blocked).

---

## 2. Product lackings vs ssh-tmux (native)

Gold standard: `RemoteTmuxWindowMirror`. Target: PR7 / host mirror on `#10045`.

| ID | Severity | Lacking | Notes |
|----|----------|---------|-------|
| L-OUTPUT-STREAM | **major** | No Herdr `%output` byte stream | Native uses bounded **poll** `pane.read` (150ms busy / 500ms idle). Documented in `RemoteHerdrSessionHost`. Latency & CPU scale with pane count. |
| L-SIZING-CLAIM | **major** | Feed-forward client-size claim not full tmux parity | Herdr `pane.resize` is split-edge oriented; tmux `refresh-client -C` style claim still partial. |
| L-TMUX-HELPERS | **minor** | Not every `RemoteTmuxWindowMirror+*` helper ported | Remaining polish: sizing-transaction helpers, edge cases. |
| L-ACCEPT-PR7 | **major** | PR7 acceptance checkboxes open | Real tab-per-Herdr-tab, split without outer duplicate, input routing, close/detach semantics — dogfood required. See [PR7_HERDR_WINDOW_MIRROR.md](./PR7_HERDR_WINDOW_MIRROR.md). |
| L-PROVIDER-UUID | **major** | Herdr `ping` lacks durable server UUID | Path reuse/restart detection incomplete; cmux uses connection generation / endpoint hash as stopgap ([PARITY_MATRIX](./PARITY_MATRIX.md)). |
| L-EVENT-CURSOR | **major** | Event stream has no resumable cursor | After gaps: mark stale, reconnect, full snapshot (no incremental resume). |
| L-CAP-TABLE | **minor** | Protocol 17 exposes booleans, not method inventory | cmux keeps a tested protocol compatibility table; unknown versions degrade. |
| L-MOBILE | **wontfix-v1** | Mobile/remote cmux | Local Unix socket only; omit or read-only projection in v1. |

---

## 3. Plugin / stopgap lackings (explicit, not bugs)

From [OPEN.md](../../OPEN.md) + [TMUX_PARITY.md](./TMUX_PARITY.md):

| ID | Severity | Lacking |
|----|----------|---------|
| L-PLUGIN-PTY | **ceiling** | No Ghostty PTY theft — `attach-pane` is a second client |
| L-PLUGIN-OUTPUT | **ceiling** | Poll `pane.read`; no true `%output` |
| L-PLUGIN-BONSPLIT | **ceiling** | No Bonsplit owner → no divider-drag → `resize-pane` in plugin |
| L-PLUGIN-FLAT | **ceiling** | Flat status-pill projection; not first-class nested hierarchy |
| L-PLUGIN-SURFACE | **ceiling** | Live apply machine runs; surfaces in-memory until native `TerminalPanel` path |

---

## 4. Shared contract still fragile

| ID | Severity | Item |
|----|----------|------|
| L-SINGLE-WRITER | **watch** | Plugin must no-op when native attachment live (handoff/lease). Fork [#18](https://github.com/RaviTharuma/cmux/pull/18) / native handoff — keep verified under dogfood. |
| L-TITLE-LOCK | **watch** | Native-title lock + heuristic-once must not thrash under dual plugin+native |
| L-ASSOC-KEY | **watch** | Plugin `pane_id:session_id` is cache only — never public nested ID |

---

## 5. Branch / PR sprawl (hygiene risk)

Open **fork** PRs on `RaviTharuma/cmux` (may be superseded by `#10045` tip — verify before landing):

- #18 `cursor/herdr-handoff-6e7a` — Add RemoteHerdrHandoff so native yields to the plugin lease (updated 2026-08-17)
- #17 `cursor/herdr-native-mirror-f1c1` — Add native Herdr window mirror (ssh-tmux parity counterpart) (updated 2026-08-18)
- #16 `cursor/herdr-live-machine-6e7a` — Add RemoteHerdr live apply machine (updated 2026-08-17)
- #14 `cursor/herdr-lifecycle-6e7a` — Add RemoteHerdrLifecycle (updated 2026-08-17)
- #13 `cursor/herdr-io-session-6e7a` — Add RemoteHerdr pane I/O and session apply (updated 2026-08-17)
- #12 `cursor/herdr-host-apply-6e7a` — Add RemoteHerdrHostApply (updated 2026-08-18)

Herdr-related branches observed on `cmux-herdr` (18 heads). Treat as parallel lanes; do not force-push shared tips. Full list in [STATUS.json](./STATUS.json).

---

## 6. Upstream design still open

| Artifact | Status |
|----------|--------|
| Issue [#8737](https://github.com/manaflow-ai/cmux/issues/8737) | OPEN — full nested topology design |
| Discussion [#10106](https://github.com/manaflow-ai/cmux/discussions/10106) | OPEN — community poll |
| PR [#10045](https://github.com/manaflow-ai/cmux/pull/10045) | OPEN — native nested + mirror work; **merge BLOCKED** |
| PR [#8736](https://github.com/manaflow-ai/cmux/pull/8736) | OPEN — CLI compat shim; **merge UNSTABLE** |

---

## 7. What is *not* an error right now

- CodeRabbit unresolved review threads on `#10045` / `#8736`: **0**
- Socket Security checks on tips: success
- Intentional non-goals: no `server.stop` on host close; no CLI shell-out on native path; sidebar ≠ ssh-tmux

---

## 8. Refresh rule

When agents go idle or tip SHAs move, update this file + `STATUS.json` on `cursor/agent-lanes-f1c1` (integration_ops lane). Do not rewrite hot product tips from this inventory.
