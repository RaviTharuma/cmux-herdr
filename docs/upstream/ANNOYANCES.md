> **Canonical source of truth is GitHub**, not this draft.
>
> - Full nested topology: https://github.com/manaflow-ai/cmux/issues/8737
> - Hidden compat MVP: https://github.com/manaflow-ai/cmux/pull/8736
> - Update GitHub first when trackers move; then sync this file if still useful.
>
> This file is a paste-ready / design package kept for dual-path history. Prefer the live issue/PR over local text if they diverge.

# Annoyances, thrash, and hard-won lessons

This is the unfiltered engineering report of everything that hurt while building:

1. the **user-space plugin** (`RaviTharuma/cmux-herdr`)
2. the **upstream CMUX MVP PR** ([manaflow-ai/cmux#8736](https://github.com/manaflow-ai/cmux/pull/8736))
3. the **upstream nested-topology issue** ([manaflow-ai/cmux#issues/8737](https://github.com/manaflow-ai/cmux/issues/8737))
4. the **pi-subagents Herdr placement PR** ([edxeth/pi-subagents#16](https://github.com/edxeth/pi-subagents/pull/16))

It is intentionally blunt. Use it for PR descriptions, review replies, and “why we did it this way” notes.

---

## TL;DR

| Layer | What hurt most |
|---|---|
| Product model | cmux sees one surface; Herdr owns the real tree. Status pills are a flat lie. |
| Plugin bridge | Wrong outer workspace thrashing, shell panes polluting agent lists, JSON shape variance, no events. |
| Association cache | Needed a parent lock + pane map just to stop thrash; still not restore truth. |
| Upstream MVP PR | Hidden dispatcher is a shim, not nested topology; review nits on vendor naming / `--json`. |
| Upstream full issue | Huge design surface; native must not own Herdr PTYs or re-parent panes. |
| pi-subagents placement | Default was tabs (wrong UX); sibling stacking needs owned-anchor state; tests leaked state across cases. |
| macOS testing | `/var` vs `/private/var` path flakes; poisoned mutex panic culture; socket/env pollution. |
| Dual path tax | Plugin + native must share association/title-lock semantics or upgrades thrash titles. |

---

## 1. The fundamental product annoyance

### 1.1 Nested topology is invisible

Reality:

```text
cmux window → workspace → pane → terminal surface
                                      └─ Herdr workspace → tab → pane → agent
```

What cmux thinks:

```text
cmux window → workspace → one fat terminal
```

Consequences:

1. Inner agents do not appear in cmux navigation.
2. Unread / attention / automation cannot target a real Herdr pane.
3. Restore cannot reattach to the correct inner node.
4. Focus/split actions from cmux hit the outer surface, not the intended agent.
5. Users end up alt-tabbing mentally between two UIs that both think they own the tree.

### 1.2 Status pills are a stopgap, not a hierarchy

`cmux set-status` can only paint flat workspace pills:

- `herdr:w2:p2B  working  pi  …`
- No parent/child edges.
- No tab grouping.
- No “focus this inner pane” as a first-class node.

This is useful, and also deeply unsatisfying. Every time someone says “just sync better,” the answer is: **sync cannot invent hierarchy cmux refuses to model.**

### 1.3 Dual hierarchy cognitive load

Operators must learn:

- outer cmux workspace IDs (`workspace:16`)
- inner Herdr workspace IDs (`w2`)
- outer surface UUIDs
- inner pane IDs (`w2:p2B`)
- which CLI talks to which world (`cmux` vs `herdr` vs `cmux-herdr`)

The plugin skill exists mainly because this is easy to get wrong.

---

## 2. Plugin bridge annoyances (`cmux-herdr`)

### 2.1 Wrong workspace thrash (worst early bug class)

Nested shells inherit stale `CMUX_WORKSPACE_ID`.

If sync writes statuses into the wrong outer workspace:

- pills appear on a workspace the user is not looking at
- the correct workspace looks “dead”
- next focus change looks like a flaky bridge

**Fix shape:**

- require a **host fingerprint** (`CMUX_SURFACE_ID` + `HERDR_SOCKET_PATH` + optional Herdr server pid); fail loud when pieces are missing
- persist **per-fingerprint** parent bindings under `~/.local/state/cmux-herdr/parent-<fingerprint>.json`
- key binding by outer surface + Herdr socket (+ pid when known) + Herdr workspace so multi-window hosts do not collide
- do not re-bind on every outer focus twitch without an explicit reason; never probe the bare focused workspace when the fingerprint is incomplete
- `--workspace` remains an explicit override

### 2.2 Shell panes are not agents

Herdr pane list includes ordinary shells.

If you mirror everything:

- status bar fills with noise
- “working/idle” semantics collapse
- users think half their terminals are agents

**Fix shape:**

- prefer panes with `agent` set
- fall back only to panes with real agent statuses
- never invent agent identity for bare shells

### 2.3 Stale status keys accumulate forever without pruning

Each pane gets `herdr:<pane_id>`.

When panes die:

- keys remain until explicitly cleared
- progress counts drift
- “ghost agents” haunt the sidebar

**Fix shape:**

- on every sync, compute desired key set
- clear only stale `herdr:*` keys
- never wipe unrelated cmux statuses

### 2.4 Polling instead of events

`watch` is a sleep loop (default 3s).

Annoyances:

- up to ~3s lag on status changes
- wasted work when nothing changed
- no backpressure story
- feels “almost live” which is worse than honestly batchy

Native path must subscribe to Herdr events. Plugin cannot pretend it has that.

### 2.5 JSON shape variance from Herdr CLI

Herdr responses are not one clean schema forever:

- sometimes `{ result: … }`
- sometimes direct objects/arrays
- sometimes leading noise before JSON
- list endpoints named inconsistently (`workspaces` vs `workspace_list`, etc.)

**Tax paid:** defensive `_parse_json_payload`, multiple key fallbacks, explicit errors with stderr.

### 2.6 Missing tools must degrade, not explode

Environments vary:

- `herdr` missing
- `cmux` missing
- socket missing / dead
- workspace unresolvable

Early versions raised too hard for “status” style commands.

**Fix shape:**

- availability probes that return structured “unavailable”
- user-facing errors only when an action truly cannot proceed
- never require both tools just to print help

### 2.7 Association map was inevitable

After parent binding, we still needed:

- pane → status key
- pane → agent session path/id
- prune list of gone panes
- inspection CLI (`cmux-herdr associations`)

This is the production data pattern:

- small state file
- keyed by parent identity (socket/workspace)
- rewritten each sync
- **cache only**, not restore authority

If native later rehydrates from this file as truth, we will reintroduce thrash and ghosts.

### 2.8 Install / path / packaging friction

- must run from repo without install (`sys.path` hacks)
- must also install to `~/.local/bin`
- skill lives under `~/.pi/agent/skills/cmux-herdr`
- docs, bridge, and skill drift unless updated together
- `pytest` not always present; `unittest` is the reliable runner

### 2.9 Test infrastructure annoyances

- unit tests originally imported `cmux_herdr_bridge` without package path setup
- behavior tests mocked the wrong helper name (`list_cmux_status_keys` vs `list_cmux_herdr_keys`)
- association tests needed temp `XDG_STATE_HOME` isolation
- live smoke against real Herdr can see 40–60 panes and is noisy

### 2.10 Explicit non-goals people keep asking for

The plugin does **not**:

1. create first-class nested Bonsplit/cmux nodes
2. provide event-driven updates
3. reattach after cmux restart as a real session identity
4. own Herdr PTYs
5. replace Herdr’s own UX

Saying “no” repeatedly is part of the job.

---

## 3. Upstream CMUX MVP PR annoyances (#8736)

PR: [Add hidden Herdr compatibility dispatcher](https://github.com/manaflow-ai/cmux/pull/8736)

### 3.1 It is deliberately a shim, and that is politically annoying

The PR adds `cmux __herdr-compat …` so cmux can talk to Herdr without waiting for full nested topology.

People read it as “native Herdr support.” It is not.

It is:

- hidden command surface
- argument translation
- exec into `herdr`
- tests + localization

It is **not**:

- nested tree model
- virtual descendants under a host surface
- restore
- focus routing into inner panes

### 3.2 Hidden command vs discoverability tension

If it is hidden, agents/tools still need to find it.

If it is documented loudly, reviewers reject “vendor-specific public API.”

Current compromise:

- hidden dispatcher name
- help text exists
- command suggestions include it carefully
- issue #8737 carries the real design

### 3.3 Review thrash already on the PR

Concrete review pain:

1. **`--json` handling asymmetry**  
   `status` gets JSON treatment; `snapshot` / list aliases may silently ignore cmux-level `--json` depending on Herdr subcommand behavior. Integrators cannot tell if the flag was honored.

2. **Vendor name leakage in user-facing errors**  
   Strings naming “Herdr” and `__herdr-compat` conflict with cmux’s “don’t expose upstream vendor / internal command names” style rules.

3. **Executable resolution edge cases**  
   Missing files, directories, non-executables, bundled path traps. Had to harden discovery.

4. **`execv` allocation failure path**  
   Rare, but must fail with localized message and exit 126 instead of crashing.

5. **Scope creep pressure**  
   Every review comment invites “while you’re here, model the tree.” That belongs in later PRs from the plan, not this shim.

### 3.4 Localizable strings tax

Every user-facing string needs xcstrings updates. Easy to miss. Easy for CI/review bots to nag. Required anyway.

### 3.5 Why this PR still had to exist

Without a tiny compatibility seam:

- plugin remains the only integration
- native work has no incremental on-ramp
- argument/error behavior gets re-litigated later inside larger PRs

Painful, but sequencing-correct.

---

## 4. Upstream full issue annoyances (#8737)

Issue: [Native nested-multiplexer topology for Herdr-hosted agents](https://github.com/manaflow-ai/cmux/issues/8737)

### 4.1 Design surface is huge

Must specify all of:

- capability negotiation
- provider attachment lifecycle
- snapshot + event model
- virtual node identity (compound IDs, generations)
- action forwarding vs local mutation
- title authority / locks
- restore / reattach
- authorization from control socket
- multi-provider future (not Herdr-only forever)
- plugin coexistence rules

If any of these are vague, implementers will invent incompatible behavior.

### 4.2 Ownership boundary is easy to violate

Hard rule:

> cmux must **not** duplicate PTYs or treat provider panes as cmux-native panes.

Violations look “simpler” in the short term and become unrecoverable architecture debt.

### 4.3 Identity collisions

Raw provider IDs are not globally unique across:

- two Herdr instances
- reconnect generations
- host surfaces

Need compound identity + generation rejection, or focus/actions hit the wrong node after reconnect.

### 4.4 Title thrash class of bugs

Multiple writers want to set titles:

- provider
- user rename
- host labeler
- plugin status mirror
- agent bootstrap scripts

Without **native-title lock + diff-before-write**:

- labels bounce every poll
- user renames get overwritten
- unread badges look haunted

### 4.5 Heuristic association must run once

Inferring parent/session from prompts or titles is useful once and poisonous forever.

Contract:

1. key by `pane_id:session_id` (or native compound equivalent)
2. record parent map
3. skip heuristic after first successful association
4. prune on pane/session death
5. never rehydrate association cache as restore authority

### 4.6 PR sequencing is mandatory, not optional

From `PR_PLAN.md`, landing everything at once guarantees:

- unreviewable diffs
- mixed read-path and mutation bugs
- restore broken while UI looks “done”

Native must land roughly as:

1. protocol / capabilities / read snapshot
2. virtual tree render
3. action forwarding
4. events
5. restore
6. plugin demotion / coexistence

### 4.7 Rust poison-mutex culture

In shared-state paths, `.unwrap()` on poisoned mutexes turns one worker failure into host death.

Policy we wrote into the plan:

- prefer `unwrap_or_default()` / explicit poison recovery
- do not panic the host because a provider map lock got poisoned

### 4.8 Docs thrash while refining the issue

Local ISSUE/PR_PLAN were refined after the first GitHub submission.

Risk:

- GitHub issue body drifts from repo docs
- reviewers comment on stale text

Mitigation:

- keep repo docs canonical
- point issue at plugin repo docs
- update issue when contract changes (association keying, title lock, single-writer rule)

---

## 5. pi-subagents Herdr placement annoyances (#16 + follow-on)

PR: [fix(herdr): open interactive subagents as same-tab pane splits](https://github.com/edxeth/pi-subagents/pull/16)

### 5.1 Default UX was wrong for nested agent work

Old default: dedicated child **tabs**.

Why that sucks for this workflow:

- parent disappears under a new tab
- “right-stack like cmux prefer-split” is what users expect
- comparing parent/child context requires manual tab ping-pong

New default: **`right-stack`**

- first child splits right of parent
- later siblings stack down in that column
- parent stays visible

### 5.2 Sibling stacking needs owned-anchor state

Without memory of “which panes belong to this parent”:

- second child cannot reliably stack under the first child
- heuristics pick random newest pane
- splits walk across unrelated panes

**Fix shape (Zellij-like hybrid):**

- small placement state file/map
- track parent → owned child panes
- choose newest **live owned sibling** as stack anchor
- isolate by Herdr socket so concurrent tests/sessions do not collide

### 5.3 Test isolation was a landmine

Symptoms:

- first test expects parent-right split
- later test fails because previous child still “owned”
- default policy tests flake depending on order

Root causes:

- global placement state
- shared default socket identity
- env vars leaking across cases

**Fix shape:**

- `resetHerdrPlacementState(socket)`
- unique `HERDR_SOCKET_PATH` per test dir
- clear mux env thoroughly (`HERDR_WORKSPACE_ID`, socket, placement overrides)

### 5.4 macOS `/var` vs `/private/var` flake

Background launch test asserted:

```text
PWD=/var/folders/.../background-workspace
```

Actual:

```text
PWD=/private/var/folders/.../background-workspace
```

Same directory. Brittle string match. Wasted time.

**Fix shape:** normalize by stripping `/private` before compare, or assert path equivalence properly.

### 5.5 Fake Herdr scripts vs real lifecycle

Interactive tests stage fake `herdr` on `PATH`.

Annoyances:

- must emulate enough of split/tab/pane APIs
- exit/sentinel timing can still flake under load
- “records child exit status…” timed out once under parallel noise even when logic was fine

### 5.6 Placement policy matrix is easy to under-specify

Needed explicit behavior for:

| Policy | First child | Later siblings |
|---|---|---|
| `right-stack` (default) | right of parent | down from owned anchor |
| `down-stack` | down from parent | right from first child (column growth variant) |
| `right` / `down` | simple split | no owned stacking |
| `tab` | dedicated tab | previous default escape hatch |

If docs and code disagree, users file “random split” bugs that are actually policy confusion.

### 5.7 Local uncommitted follow-on vs opened PR

PR #16 opened with same-tab split work.

Local branch later gained:

- full owned right-stack policy
- anchor state module
- README policy docs
- isolation fixes

Annoyance: GitHub PR body can lag local truth until force-pushed/updated. Reviewers may approve an older story.

---

## 6. Cross-cutting engineering thrash

### 6.1 Two paths, one behavioral contract

Plugin and native must share:

1. association keying
2. parent map usage
3. heuristic-once
4. title lock
5. single writer when native attachment is live
6. prune-on-death

If native ignores plugin lessons, users upgrading will see title/parent thrash and blame “cmux regression” or “plugin regression” alternately.

### 6.2 “Just use tmux semantics” is incomplete

Nested tmux awareness is a useful analogy, not a drop-in:

- Herdr has agent session metadata tmux lacks
- cmux already has workspace/status/automation concepts
- provider-forwarded actions differ from local pane ops
- restore identity is richer and more dangerous

### 6.3 Environment variable soup

Recurring vars:

- `CMUX_WORKSPACE_ID`
- `CMUX_SURFACE_ID`
- `HERDR_SOCKET_PATH`
- `HERDR_WORKSPACE_ID`
- `PI_SUBAGENT_HERDR_PLACEMENT`
- `XDG_STATE_HOME`

Any one stale value creates a bug that looks like code.

### 6.4 State directory conventions

We standardized on:

- `$XDG_STATE_HOME/cmux-herdr/` (default `~/.local/state/cmux-herdr/`)
- mode-safe parent/association files
- filenames sanitized from socket+workspace

Still easy to:

- point tests at real home state
- forget isolation
- treat cache files as durable truth

### 6.5 Tooling inconsistencies

- Node test runner + `--experimental-strip-types` for pi-subagents
- Python unittest for plugin
- Swift/xcode world for cmux PR
- `gh` for upstream artifacts
- review bots (CodeRabbit / Greptile / Cubic) generating noise mixed with real nits

Context switching cost is real.

### 6.6 Live systems are messy

A real sync against the working machine showed:

- dozens of panes
- mixed claude/pi agents
- many idle sessions
- stale keys needing bulk clear

Demo-friendly “2 panes” examples hide production chaos. The association cache and prune logic exist because production is chaotic.

---

## 7. Process / communication annoyances

1. **People ask “issue or plugin?”** — answer is both; stopgap + native track.
2. **People treat status pills as done** — they are not hierarchy.
3. **PR #8736 gets over-read as full native support** — it is a hidden compat dispatcher.
4. **Issue #8737 is large** — necessary, but hard to review emotionally.
5. **Local docs refined after filing** — keep repo canonical; update GitHub when contract changes.
6. **pi-subagents placement PR opened before full right-stack follow-on finished** — update before review solidifies around old default.
7. **Every layer has a different “source of truth”** — cmux tree, Herdr tree, plugin cache, agent session files. Document which is authoritative for each operation.

---

## 8. Concrete checklist of bugs we already paid for

Use this as a regression list.

### Plugin

- [ ] stale `CMUX_WORKSPACE_ID` writes pills to wrong workspace
- [ ] parent binding lost across outer focus changes
- [ ] shell panes mirrored as agents
- [ ] dead pane status keys not cleared
- [ ] clearing `herdr:*` accidentally clears unrelated statuses
- [ ] missing `herdr`/`cmux` crashes status command
- [ ] association file grows forever (no prune)
- [ ] two Herdr sockets share one state file
- [ ] treat association cache as restore authority

### CMUX native / MVP

- [ ] public vendor-specific command surface instead of hidden/capability path
- [ ] `--json` silently ignored on some compat aliases
- [ ] user-facing errors leak internal command names
- [ ] executable resolution accepts directories/non-executables
- [ ] nested nodes implemented as real cmux panes/PTYs
- [ ] action applied after provider generation change
- [ ] duplicate raw provider IDs across attachments collide
- [ ] title writers overwrite user/provider locks
- [ ] heuristic re-runs every event and reparents nodes
- [ ] mutex poison panics host

### pi-subagents Herdr placement

- [ ] default opens tabs when user wanted visible parent+child split
- [ ] second sibling splits from wrong anchor
- [ ] placement state shared across sockets/tests
- [ ] env leaks change policy mid-suite
- [ ] macOS `/private` path assertion flakes
- [ ] background launches incorrectly require Herdr mux

---

## 9. What we would do differently next time

1. Write the **behavioral contract** (association key, title lock, single writer, prune) before any UI/PR wording.
2. Open the upstream issue with that contract, then implement plugin as a dogfood of the same contract.
3. Keep MVP PR aggressively tiny; refuse tree/render comments there.
4. For placement, implement owned-anchor state in the first PR, not as a follow-on.
5. Normalize paths and isolate env/state in tests on day one.
6. Assume production has 50+ panes, mixed agents, and stale IDs.
7. Document non-goals in the first screen of README/OPEN so “can it restore?” is answered before demos.

---

## 10. Artifact map

| Artifact | Role | Status |
|---|---|---|
| [cmux#8736](https://github.com/manaflow-ai/cmux/pull/8736) | Hidden Herdr compat dispatcher (shim) | Open |
| [cmux#8737](https://github.com/manaflow-ai/cmux/issues/8737) | Full native nested topology design | Open |
| [RaviTharuma/cmux-herdr](https://github.com/RaviTharuma/cmux-herdr) | User-space plugin/stopgap + docs | Implemented (local uncommitted association refinements may exist) |
| [pi-subagents#16](https://github.com/edxeth/pi-subagents/pull/16) | Same-tab / placement fixes for Herdr subagents | Open (local right-stack follow-on may be ahead of remote) |
| `docs/upstream/ISSUE.md` | Canonical long-form native issue text | Repo |
| `docs/upstream/PR_PLAN.md` | Incremental native PR sequence | Repo |
| `OPEN.md` | Stopgap limitations + live links | Repo |
| this file | Annoyance/thrash report | Repo |

---

## 11. One-paragraph version for a PR body

Building nested Herdr awareness hurt in predictable ways: cmux only sees one surface while Herdr owns the real tree; status pills can mirror agent state but cannot invent hierarchy; wrong outer workspace bindings and unpruned `herdr:*` keys thrash or ghost the UI; JSON/CLI variance and missing-tool environments require defensive probes; association/parent state is necessary production cache but must never become restore authority; the upstream MVP is intentionally only a hidden compat dispatcher and already attracts scope creep plus vendor-naming/`--json` review nits; the full native design must capability-negotiate read-only virtual descendants, forward mutations, lock titles, run heuristics once, and reject stale provider generations without owning PTYs; and on the launcher side, Herdr subagents needed owned right-stack placement state with strict per-socket test isolation because tab defaults and shared anchors produced wrong splits and order-dependent flakes (including macOS `/private` path noise). Native remains the primary path; the plugin stays the flexible fallback and dogfood harness for the same behavioral contract.
