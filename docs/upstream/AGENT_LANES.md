# Agent lanes (parallel-safe)

Updated: `2026-08-19T06:52:54Z`

Multiple agents run at once (this cloud environment **and** a MacBook). Treat this file as the traffic rules. Prefer additive work on your own branch. Never force-push another lane's tip.

## Lanes

| Lane | Owns | Do not touch unless you own the lane |
|------|------|--------------------------------------|
| `native_integration` | Upstream [#10045](https://github.com/manaflow-ai/cmux/pull/10045) tip `cursor/nested-topology-herdr-v1-becf` | Force-push; drive-by refactors of mirror hosts while another agent is mid-fix |
| `plugin_compat` | Upstream [#8736](https://github.com/manaflow-ai/cmux/pull/8736) tip `feat/herdr-native-mvp` + plugin/bridge | Mixing plugin CLI shim into native package PRs |
| `docs` | Narrative docs under `docs/**` (parity matrices, plans, walkthroughs) | Overwriting docs another docs agent just rewrote |
| `knowledge` | Research notes / capture / Airtable-style acquisition | Same paths as docs without checking |
| `integration_ops` | Status snapshots, reply/resolve helpers, CI readiness, coordination | Rewriting product code on `#10045` tip without a fresh tip pull |

## Hard rules

1. **Canonical product code for native Herdr** lands on `#10045` head only after `git fetch` + rebase/ff on current tip. No `--force` to that branch except recovering a known broken PLACEHOLDER tip.
2. **Plugin and native stay separate tracks** (`#8736` vs `#10045`). Shared contracts go in tests/docs, not by merging the PRs together.
3. Before editing a hot file (`RemoteHerdr*`, `CmuxNestedTopology`, plugin bridge), check tip SHA in `STATUS.json` and recent commits. If tip moved in the last few minutes, re-fetch.
4. Docs agents: keep large rewrites on a docs branch; do not reset `#10045`.
5. Integration_ops agents: may update `STATUS.json`, reply scripts under `docs/upstream/patches/`, and open side branches `cursor/*-f1c1` — not silent tip rewrites.

## Current snapshot

See [`STATUS.json`](./STATUS.json).

- `#10045` unresolved review threads: **0** / 173
- `#8736` unresolved review threads: **0** / 14

## This agent (integration_ops)

- Will **not** force-push `#10045` tip while other agents are active.
- Will monitor CodeRabbit / merge readiness and only land code on a **side branch** then fold via one-shot Actions when tip is quiet.
- Leaves `docs/**` narrative ownership to the docs/knowledge agents unless fixing a broken link in patches/scripts.

## Sibling branches observed (do not hijack)

Recently fetched on `cmux-herdr` / related work:

- `cursor/plugin-polish-6e7a` — plugin polish (other agent)
- `cursor/tmux-gap-inventory-6e7a` — tmux gap inventory (other agent)
- `cursor/tmux-impose-depth-6e7a` — tmux impose depth (other agent)
- `cursor/nested-topology-herdr-v1-becf` — **shared** upstream tip for `#10045` (hot)
- `feat/herdr-native-mvp` — **shared** upstream tip for `#8736` (hot)

If you need to contribute to those topics, rebase onto their tip and open a follow-up PR; do not rewrite history.


## Freshness (2026-08-19T06:52:54Z)

| Track | Tip SHA | Unresolved | Merge |
|-------|---------|------------|-------|
| #10045 native | `b02b8a954327` | 0 | BLOCKED |
| #8736 plugin | `2f483ad94f0d` | 0 | UNSTABLE |

Idle/side branches are listed in `STATUS.json` → `idle_or_side_branches`. Refresh that file when agents go idle; leave hot tips alone unless you own the lane.

## Errors & lackings

Frozen inventory: [`ERRORS_AND_LACKINGS.md`](./ERRORS_AND_LACKINGS.md) / [`ERRORS_AND_LACKINGS.json`](./ERRORS_AND_LACKINGS.json).
