---
name: cmux-herdr
description: Navigate dual hierarchy when herdr runs nested inside cmux. Use herdr for inner tabs/panes/agents and cmux for outer window/workspace/surface. Bridge state with cmux-herdr CLI.
---

# cmux-herdr

## When this skill applies

You are in a **nested** environment if most of these env vars are set:

```
HERDR_ENV=1
HERDR_PANE_ID / HERDR_TAB_ID / HERDR_WORKSPACE_ID / HERDR_SOCKET_PATH
CMUX_SURFACE_ID / CMUX_WORKSPACE_ID / CMUX_TAB_ID / CMUX_SOCKET_PATH
```

Detect quickly:

```bash
cmux-herdr status
```

## Dual hierarchy (do not collapse them)

| Layer | Outer (cmux) | Inner (herdr) |
|-------|--------------|---------------|
| Top   | window       | session/server |
| Mid   | workspace    | workspace (`w2`) |
| Tab   | tab/surface row | tab (`w2:t11`) |
| Leaf  | pane → surface (terminal) | pane (`w2:p34`) hosting an agent |

**Never assume tmux exists.** herdr is the inner mux. cmux is the outer macOS terminal workspace app.

## Which CLI to use

### Use `herdr` when you need to…
- List/focus **inner** tabs and panes: `herdr tab list`, `herdr pane list`
- Split an agent pane: `herdr pane split --current --direction right|down`
- Drive agents: `herdr agent list|focus|prompt|wait`
- Read pane output: `herdr pane read <pane_id>`

### Use `cmux` when you need to…
- Outer layout: workspaces, surfaces, windows (`cmux tree`, `cmux open`, …)
- Sidebar status pills / progress: `cmux set-status`, `cmux set-progress`
- App settings/docs: `cmux docs …`

### Use `cmux-herdr` when you need to…
- Diagnose install / fingerprint / LaunchAgent: `cmux-herdr doctor`
- See both contexts: `cmux-herdr status`
- Pretty inner topology: `cmux-herdr tree`
- Mirror agents into outer sidebar: `cmux-herdr sync` or `cmux-herdr watch`
- Project Herdr tabs/panes into real cmux tabs/splits: `cmux-herdr mirror` (current tab) or `cmux-herdr mirror --tmux-parity` (ssh-tmux contract)
- Follow one pane in this terminal: `cmux-herdr attach-pane <pane_id>`
- Keep deep mirror live: `cmux-herdr watch --tmux-parity`
- Compact agent list: `cmux-herdr agents`
- Focus helpers: `focus-workspace`, `focus-tab`, `focus-pane`, `focus-agent`
- Read helpers: `read-pane <pane_id>`, `read-agent <target>`
- Inspect / clear mirrored pills: `cmux-herdr associations`, `cmux-herdr clear`

## Safe agent splitting

To spawn parallel agents **without breaking the outer cmux layout**:

1. Stay inside herdr — do **not** open a new cmux workspace per subagent by default.
2. Split the current herdr pane:
   ```bash
   cmux-herdr split --direction right
   # or
   herdr pane split --current --direction down
   ```
3. Start/attach the agent in the new pane (`herdr agent start …` or your usual launcher).
4. Refresh outer visibility:
   ```bash
   cmux-herdr sync
   ```

Outer cmux still shows one (or few) surfaces hosting herdr; inner topology is what multiplies.

## Status bridge contract

- Keys: `herdr:<pane_id>` (e.g. `herdr:w2:p34`)
- Colors: working=orange, idle=gray, done=green, blocked=red, unknown=gray
- Progress: `working / (working+idle+done+blocked)` via `cmux set-progress`
- Stale keys for gone panes are cleared on each sync

## Common pitfalls

1. **Wrong workspace for set-status** — nested shells may have stale `CMUX_WORKSPACE_ID`. Prefer `cmux-herdr sync` (auto-resolves via host fingerprint) or pass `--workspace`.
2. **Missing host fingerprint** — auto sync/watch need `CMUX_SURFACE_ID` + `HERDR_SOCKET_PATH` (optional `HERDR_SERVER_PID`). Without them the CLI errors instead of writing pills to a random outer host.
3. **Using cmux to split agent fleets** — that creates outer surfaces; use herdr splits instead.
4. **Assuming `herdr pane focus <id>`** — directional only; use `herdr agent focus <pane_id>` or `cmux-herdr focus-pane`.
5. **Looking for tmux sockets** — use `$HERDR_SOCKET_PATH` / `$CMUX_SOCKET_PATH`.

## Debug

```bash
cmux-herdr json-dump
herdr pane list
cmux tree --id-format both
cmux list-status --workspace <resolved>
```

## Hybrid association cache

`cmux-herdr sync` rewrites `~/.local/state/cmux-herdr/associations-<fingerprint>.json` (and matching `parent-<fingerprint>.json`) keyed by outer surface + Herdr socket (+ optional server pid). Inspect with `cmux-herdr associations`. Treat it as cache only.

If native nested attachment is live (`CMUX_HERDR_NATIVE_LIVE=1` or a `native-live-<fingerprint>` marker), `sync`/`watch` skip pill writes (single-writer). `CMUX_HERDR_FORCE_PLUGIN=1` overrides. Use `cmux-herdr lock-title` / `unlock-title` so polls do not thrash a locked display name.
