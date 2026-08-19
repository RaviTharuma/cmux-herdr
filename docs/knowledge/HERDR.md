# Herdr — synthesized product knowledge

Source of truth for commands and protocol: official docs in
`raw/herdr/docs-master/` plus `raw/indexes/herdr-llms-full.txt` (complete
concatenated stable docs, release **0.8.0**). `master` MDX was also saved and
is slightly ahead of the tagged stable index.

Herdr is a **terminal-native agent multiplexer**: a background server owns real
PTYs; clients attach to render them. It is mouse-first, agent-aware, and
scriptable through the same CLI and Unix-socket (Windows named-pipe) API.

License on GitHub: Apache-2.0. Binary: one Rust executable for Linux, macOS,
and Windows (Windows generally available with documented limits).

## Concept model

Teach in this order:

| Concept | What it is | Public ID |
|---|---|---|
| Session | Persistent server namespace. Default `herdr` attaches to `default`. Named sessions (`herdr session attach work`) are fully separate sockets and state. | session name |
| Workspace | Project/task container. Owns tabs and panes. Sidebar rolls agent state up here. | `w1` |
| Tab | Layout inside a workspace (`agents`, `logs`, `server`). | `w1:t1` |
| Pane | Real terminal. Splits right or down. Survives client detach. | `w1:p1` |
| Agent | Recognized process *inside* a pane. Not a separate tree node. | live name or pane ID |
| Modes | Terminal (keys go to pane), prefix (`ctrl+b` then one action), navigate (persistent workspace nav). | — |

Closed tab/pane IDs are not reused. Moving a pane across workspaces assigns a
new workspace-qualified pane ID; the old ID remains an alias only for that
process's inherited caller context.

## Client / server

- Server owns panes, processes, detection, plugins, session files.
- Client is the TUI. Detach with `prefix+q` (`ctrl+b` then `q`). Panes keep running.
- `herdr server stop` ends the session and its processes.
- `herdr --no-session` is a single-process escape hatch.
- Nested `herdr` from a pane is blocked (`HERDR_ENV=1`).
- Headless fallback terminal size is 120×40; override with `[server] headless_cols/rows`.

Socket: `$HERDR_SOCKET_PATH` (typical `~/.config/herdr/herdr.sock`). Named-session
logs live under `sessions/<name>/` in the config dir.

## Environment injected into panes

| Variable | Meaning |
|---|---|
| `HERDR_ENV=1` | This process is inside a Herdr pane. |
| `HERDR_PANE_ID` / `HERDR_TAB_ID` / `HERDR_WORKSPACE_ID` | Public IDs. |
| `HERDR_SOCKET_PATH` | Control socket. |
| `HERDR_BIN_PATH` | Running `herdr` binary (plugins/hooks should call this). |
| `HERDR_SESSION` | Named-session selector for CLI. |
| `HERDR_CONFIG_PATH` | Config override. |
| `HERDR_PROCESS_DETECTION` | Linux: `native` (default) or `child-groups`. Server-side; needs restart. |
| `HERDR_AGENT=<kind>` | Hint when a sandbox wrapper hides the real agent process. |
| `HERDR_LOG` | e.g. `herdr=debug`. |
| `HERDR_DISABLE_SOUND` | Mute sounds. |

Plugin panes also get `HERDR_PLUGIN_ID`, `HERDR_PLUGIN_ROOT`,
`HERDR_PLUGIN_CONFIG_DIR`, `HERDR_PLUGIN_STATE_DIR`,
`HERDR_PLUGIN_ENTRYPOINT_ID`, `HERDR_PLUGIN_CONTEXT_JSON`. Herdr-managed vars
win over caller `--env`.

## Install and channels

```bash
curl -fsSL https://herdr.dev/install.sh | sh   # Linux/macOS; stable channel
# Windows: irm https://herdr.dev/install.ps1 | iex
brew install herdr
mise use -g herdr
nix profile install github:herdrdev/herdr/v0.x.y
herdr update
herdr channel show | set stable | set preview
herdr --version
```

Config: `~/.config/herdr/config.toml` (Windows `%APPDATA%\herdr\config.toml`).
Print defaults with `herdr --default-config`. Reload: `herdr server reload-config`.
167 canonical keys are listed in `INVENTORIES.md` / `raw/herdr/data/config-reference.json`.

## Keyboard (optional; mouse covers everything)

Prefix default `ctrl+b`. `prefix+?` lists live bindings. First five:

| Action | Key |
|---|---|
| New tab | `prefix+c` |
| Split right / down | `prefix+v` / `prefix+minus` |
| Move panes | `prefix+h/j/k/l` |
| Workspace nav | `prefix+w` |
| Detach | `prefix+q` |

Also: zoom `prefix+z`, close pane `prefix+x`, resize mode `prefix+r`, copy mode
`prefix+[`, tabs `prefix+n/p` and `1..9`, sidebar `prefix+b`. Every binding
including the prefix is `[keys]` in config. Prefix-free `ctrl+alt` chords are
documented on the keyboard page; OS/outer-terminal stolen chords are the usual
failure mode.

Copy mode does **not** pause the pane. Mouse drag-select copies without copy mode.

## Agent detection and states

States: `working`, `blocked`, `done`, `idle`, `unknown`.

- `idle`: ready for input **and** its tab has been seen in the focused Herdr UI.
- `done`: same underlying idle after unseen background work; focusing the tab or
  `pane focus` / `agent focus` marks it seen. **CLI reads do not mark seen.**
- `blocked`: known approval/question UI (strict for screen-manifest agents).
- `unknown`: agent present, classification not confident. Not proof of success.

Each pane has one **status authority**. Lifecycle-hook integrations (Pi, OMP,
Kimi, OpenCode, Kilo, MastraCode), when installed and reporting, own
idle/working/blocked and skip screen fallback. Session-only integrations
(Claude, Codex, Copilot, Devin, Droid, Qoder, Qwen, Cursor, Hermes,
Antigravity, Grok) provide resume tokens; **state still comes from screen
manifests**. Amp, Kiro, Maki: screen only, no integration. Gemini CLI and Cline:
detected but less tested.

Manifests: bundled + remote updates from herdr.dev (no restart) + local overrides
in `~/.config/herdr/agent-detection/<agent>.toml`. Debug with
`herdr agent explain`. Nested tmux *inside* a pane hides the agent (`tmux` is
the foreground process).

`agent start --kind` kinds: `pi`, `claude`, `codex`, `gemini`, `cursor`,
`devin`, `agy`, `cline`, `omp`, `mastracode`, `opencode`, `copilot`, `kimi`,
`kiro`, `droid`, `amp`, `grok`, `hermes`, `kilo`, `qodercli`, `qwen`, `maki`.

Names: `[a-z][a-z0-9_-]{0,31}`, unique among live agents. Follow the occupant;
cleared on exit/release/replace.

## CLI surface (groups)

The installed binary is authority (`herdr --help`, then group with no
subcommand). Most control commands print JSON. Syntax errors exit 2; server
errors JSON on stderr exit 1.

**Launch / meta:** `herdr`, `--session`, `--remote`, `--handoff`, `--no-session`,
`--default-config`, `--version`, `update`, `channel`, `completion`/`completions`,
`status` / `status server` / `status client`, `api schema`, `api snapshot`.

**Server:** `server`, `server stop`, `reload-config`, `agent-manifests`,
`update-agent-manifests`, `reload-agent-manifests`. Never run `server stop`
from a live session unless the user wants to kill all panes.

**Sessions:** `session list|attach|stop|delete`.

**Workspaces:** `create|list|get|focus|rename|report-metadata|close` with
`--cwd`, `--label`, `--env KEY=VALUE`, `--focus` / `--no-focus`. Create also
makes first tab + root pane (IDs in `.result.workspace|.tab|.root_pane`).

**Worktrees:** Git checkouts as workspaces. `list|create|open|remove`.
`workspace close` does not delete the checkout; `worktree remove` runs
`git worktree remove` and never deletes the branch (`--force` if dirty).

**Tabs:** `create|list|get|focus|rename|close`. Last tab close closes the
workspace. `confirm_close` can return `confirmation_required` for worktree groups.

**Panes:** topology (`split`, `swap`, `move`, `zoom`, `layout`, `neighbor`,
`edges`, `focus`, `resize`, `rename`, `close`, `current`, `get`, `list`,
`process-info`, `input --right-click herdr|pane`). I/O (`read`, `send-text`,
`send-keys`, `run`, `wait-output`). Agent reporting (`report-agent`,
`report-agent-session`, `release-agent`, `report-metadata`). Split default is
**no focus steal**; `--focus` to select the new pane. `--current` uses caller
`HERDR_PANE_ID`.

Read sources: `visible`, `recent` (default 80 rows), `recent-unwrapped` (best
for logs), `detection` (bottom-buffer, always plain text). `--format ansi` to
keep escapes. Alternate-screen agents (Claude, OpenCode) can mouse-scroll for
history on idle `recent*` reads when `--lines` exceeds the viewport.

**Agents:** `list|get|read|send-keys|prompt|rename|focus|wait|attach|start|explain`.
`agent start` **never creates layout** — needs an available shell pane.
`agent prompt` honors bracketed paste, refuses `agent_blocked` without sending,
`--wait` requires a lifecycle change within 5s or `agent_prompt_stalled`.
`agent wait` is event-driven and pins the occupant.

**Terminal attach:** `terminal attach`, `terminal session control|observe`
(newline-delimited `terminal.frame` / `terminal.closed`; control stdin commands
`terminal.input|resize|scroll|release`), `terminal title set|clear`.

**Notifications:** `notification show` with `--position`, `--sound none|done|request`.

**Integrations:** `install|uninstall|status` for the agents listed above
(`antigravity-cli` on the integrations page).

**Plugins:** `install owner/repo[/subdir]`, `list`, `uninstall`, `enable`,
`disable`, `link`, `unlink`, `config-dir`, `action list|invoke`, `log list`,
`pane open|focus|close`. Manifest `herdr-plugin.toml`. No sandbox. Entire CLI
is the plugin API.

Full command lines: [INVENTORIES.md](INVENTORIES.md) and
`raw/herdr/docs-master/cli-reference.mdx`.

## Socket API

Newline-delimited JSON over the local socket. Prefer CLI unless you need raw
RPC or long-lived `events.subscribe`.

```json
{"id":"req-1","method":"workspace.list","params":{}}
```

Schema: `herdr api schema --json`. Canonical method list is in INVENTORIES.md.
Notable behaviors:

- `session.snapshot` — one-shot bootstrap cache; then subscribe to events.
- `pane.input.set` exists on the socket even though the CLI uses `pane input`.
- `pane.graphics.*` requires `[experimental].kitty_graphics = true`.
- `layout.apply` restores structure, **not** live PTYs.
- `agent.view.set` / `clear` — declarative Agents-sidebar projection (plugin-owned).
- `events.subscribe` / `events.wait`.

Event types to subscribe:

- Workspace: `created`, `updated`, `metadata_updated`, `renamed`, `moved`,
  `reordered`, `closed`, `focused`
- Tab: `created`, `closed`, `focused`, `renamed`, `moved`
- Pane: `created`, `updated`, `closed`, `focused`, `moved`, `exited`,
  `agent_detected`, `output_matched`, `agent_status_changed`, `scroll_changed`
- Layout: `layout.updated`
- Worktree: `created`, `opened`, `removed`

`pane.moved` is a real move (no fake close/create). Spinner-only OSC title
changes do not emit `pane.updated` if `terminal_title_stripped` is unchanged.

## Session restore paths

| Case | Processes | Layout | Screen | Agent conversation |
|---|---|---|---|---|
| Detach/reattach | live | yes | live PTY | yes |
| Server restart | no | snapshot | only if `[experimental] pane_history` | only native resume tokens |
| Update `--handoff` | best-effort live transfer | yes | live if handoff works | yes if processes live |

Native resume (default `[session] resume_agents_on_restore = true`) uses
integration-reported session refs. Minimum integration versions and resume
argv are in `session-state.mdx` (Pi `--session`, Claude `--resume`, Codex
`resume`, OpenCode `--session`, …). Pane history is **off** by default
(secrets in scrollback) → `session-history.json`.

Live handoff is experimental/opt-in; in-flight waits/subscriptions/sockets may
drop. Clients reconnect.

## Remote

1. SSH in, run `herdr` on the remote (simplest; works from phones).
2. `herdr --remote workbox` / `ssh://user@host:2222` — local thin client, remote
   server. `--remote-keybindings server` to use remote maps. `--handoff` for
   live server replacement.

## Plugins and marketplace

Manifest declares `[[build]]`, `[[startup]]`, `[[actions]]`, `[[events]]`,
`[[panes]]`, `[[link_handlers]]`. `min_herdr_version` required. Pane placements:
`overlay` (default), `popup`, `split`, `tab`, `zoomed`. Popups are not Herdr
panes (no `HERDR_PANE_ID`, not in pane/agent APIs). GitHub install:
`herdr plugin install owner/repo[/subdir]`. Marketplace: GitHub today; tagging
for a future listing.

## Skill file

`skills/herdr/SKILL.md` (saved in `raw/herdr/skills/`): only when `HERDR_ENV=1`
and the user asked for Herdr. Default sibling split in current tab, `--no-focus`,
`--cwd "$PWD"`, parse IDs from JSON, never `server stop`. Install:
`npx skills add herdrdev/herdr --skill herdr -g`.

## DeepWiki architecture map (code-oriented)

DeepWiki pages (see INVENTORIES.md) cover: app orchestration, headless
server/client protocol, PTY runtime, layout engine, Ghostty VT, OSC metadata,
Kitty graphics, input encoding, detection manifests, integrations, resume, TUI
geometry, modal input, sidebar, theming, socket/CLI/automation APIs, plugins,
SSH remote lifecycle, platform (unix vs ConPTY), config/keybindings, worktrees,
self-update, tests, website/docs pipeline, glossary.

Use that extract when mapping a Herdr concept to a Rust module; use official
docs for the public contract.
