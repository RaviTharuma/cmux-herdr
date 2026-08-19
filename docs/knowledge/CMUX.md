# cmux — synthesized product knowledge

Prefer **cmux.com** markdown (`raw/cmux/site/`) and GitHub
`docs/cli-contract.md` over the Mintlify mirror. Mintlify is older (socket
path `/tmp/cmux.sock`, smaller command set) but still useful for JSON-RPC
examples.

cmux is a **native macOS terminal** (Swift/AppKit, libghostty / Ghostty engine,
GPL). Vertical sidebar workspaces, split panes, in-app browser, notification
rings, CLI + Unix socket. Requires macOS 14+. Not a multiplexer in the tmux
sense: the app *is* the desktop surface.

## Concept hierarchy

Official docs (`docs/concepts.md`):

```
Window (macOS window)
  └── Workspace  (sidebar entry; UI often says "tab")
        └── Pane (split region)
              └── Surface (tab *inside* a pane; terminal or browser)
                    └── Panel (internal: Ghostty terminal or WKWebView)
```

Handles: `window:N`, `workspace:N`, `pane:N`, `surface:N` (plus UUIDs).
`--id-format refs|uuids|both`. Env inside terminals:

| Variable | Meaning |
|---|---|
| `CMUX_WORKSPACE_ID` | UUID of current workspace |
| `CMUX_SURFACE_ID` | UUID of current surface |
| `CMUX_TAB_ID` | Alias of surface for tab commands |
| `CMUX_SOCKET_PATH` | Control socket (also `CMUX_SOCKET` deprecated alias; mismatch fails) |
| `CMUX_SOCKET_PASSWORD` | Optional auth |
| `CMUX_SOCKET_MODE` | `cmuxOnly` / `allowAll` / `off` (aliases accepted) |
| `CMUX_SOCKET_ENABLE` | Force socket on/off |
| `TERM_PROGRAM=ghostty`, `TERM=xterm-ghostty` | Distinguish from stock Ghostty by also checking `CMUX_*` |

Default socket on the **website** CLI page: `/tmp/cmux.sock` (debug
`/tmp/cmux-debug.sock`). Production app also uses a user-state socket (this
plugin documents `~/.local/state/cmux/cmux-501.sock`). Always honor
`CMUX_SOCKET_PATH`. Access modes: off, cmux-processes-only (default), allowAll
(env override). Password mode exists (Mintlify + CLI `--password`).

`cmux identify --json` is the way to resolve caller vs focused workspace
(`caller.workspace_ref` / `focused.workspace_ref`). Nested shells can carry
**stale** outer IDs — this plugin re-resolves before writing status.

## Install

DMG from cmux.com (Sparkle auto-update) or:

```bash
brew tap manaflow-ai/cmux
brew install --cask cmux
sudo ln -sf "/Applications/cmux.app/Contents/Resources/bin/cmux" /usr/local/bin/cmux
```

Inside cmux terminals the CLI is already on PATH.

Config: `~/.config/cmux/cmux.json` (cmux-owned: shortcuts, sidebar, notifications,
browser, automation). Terminal look (font, theme, scrollback, opacity) is
**Ghostty** `~/.config/ghostty/config`. `cmux reload-config` reloads both.
Legacy settings.json paths are fallback only.

## Session restore

Restores windows/workspaces/panes, cwd, best-effort scrollback, browser URL/history.
Does **not** checkpoint arbitrary processes (tmux/vim/shells reopen as new
terminals) unless:

- Agent hooks captured a native resume token (`cmux hooks setup`), or
- A surface has an explicit resume binding:
  `cmux surface resume set --kind tmux --checkpoint work --shell "tmux attach -t work"`.

## CLI — families (from `cli-contract.md`)

Global: `--socket`, `--password`, `--json`, `--id-format`, `--window`.
`cmux <path>` opens a directory without needing the socket.
`--help` / `--version` work without a socket.

**App / meta:** `welcome`, `docs`, `settings`, `config`, `shortcuts`, `open`,
`feedback`, `auth status|login|logout`, `ping`, `capabilities`, `identify`,
`rpc`, `events`, `reload-config`, `themes`, `restore`, `restore-session`.

**Windows:** `list-windows`, `current-window`, `new-window`, `focus-window`,
`close-window`, `window displays|display|default-display`.

**Workspaces:** `list-workspaces`, `new-workspace` (`--cwd`, `--command`,
`--env`, `--env-file`, layout), `current-workspace`, `select-workspace`,
`close-workspace`, `rename-workspace` (`rename-window` alias),
`move-workspace-to-window`, `reorder-workspace`, `reorder-workspaces`,
`workspace-action` (pin, color, description, mark-read, close-others, …),
`workspace list|create|env|close|rename|select|status|reconnect|disconnect|group`,
`move-tab-to-new-workspace`. Workspace env cannot override protected `CMUX_*` /
`TERM*`. `workspace status set` pins todo lanes
(`todo|working|needs-attention|review|done|auto`).

**Todos / comments:** `todo add|list|check|uncheck|start|edit|rm|clear|set|open`
(max 50 items). `comments list` (diff-viewer comments; socket `comments.list`).

**Panes / surfaces:** `list-panes`, `new-pane`, `focus-pane`, `list-pane-surfaces`,
`new-surface`, `new-split`, `close-surface`, `move-surface`, `split-off`,
`reorder-surface`, `tab-action`, `rename-tab`, `drag-surface-to-split`,
`refresh-surfaces`, `list-panels` / `focus-panel` (aliases), `tree`, `top`,
`surface-health`, `trigger-flash`, `debug-terminals`.

**I/O:** `read-screen` (`--scrollback`, `--lines`), `send`, `send-key`,
`send-panel`, `send-key-panel`. Keys like `enter`, `ctrl+c`. `\n` / `\t`
unescaped by CLI.

**Notifications:** `notify`, `list-notifications`, `dismiss-notification`,
`mark-notification-read`, `open-notification`, `jump-to-unread`,
`clear-notifications`. OSC 9/99/777 also ring panes.

**Sidebar metadata (this plugin's `sync` target):** `set-status`, `clear-status`,
`list-status`, `set-progress`, `clear-progress`, `log`, `clear-log`, `list-log`,
`sidebar-state`. Status pills are keyed (e.g. `cmux set-status build Running
--icon hammer --color "#ff9500"`).

**Custom / right sidebar:** `sidebar validate|reload|select|open`,
`right-sidebar toggle|show|hide|focus|set|mode` with modes
`files|find|vault|sessions|feed|dock`.

**Browser:** large `browser *` family (open, navigate, snapshot, eval, wait,
click/type/fill/press, get/is/find, screenshot, cookies, storage, network,
viewport, profiles, …). Legacy aliases: `open-browser`, `navigate`, `get-url`, …
`disable-browser` / `enable-browser` / `browser-status`.

**SSH / remote:** `ssh` (`-A`/`-a` agent forwarding), `ssh-session-list|attach|cleanup`,
`ssh-pty-attach`, `remote-daemon-status`, `workspace reconnect|disconnect`.
Website docs add **`cmux ssh-tmux`** (beta Remote tmux / `tmux -CC` mirror) and
socket methods `remote.tmux.sessions|attach|mirror|detach|state`. Mosh-tmux is
a separate roaming path (named tmux session, not native split mirror).

**tmux-compat:** `capture-pane`, `resize-pane`, `pipe-pane`, `wait-for`,
`swap-pane`, `break-pane`, `join-pane`, `next-window` / `previous-window` /
`last-window`, `last-pane`, `find-window`, `clear-history`, `set-hook`,
`set-buffer` / `paste-buffer` / `list-buffers`, `respawn-pane`,
`display-message`. Placeholders (unsupported): `popup`, `bind-key`,
`unbind-key`, `copy-mode`. Hidden: `__tmux-compat`.

**Hooks / agents:** `hooks setup|uninstall`, per-agent install, Feed conversion.
Supported hook agents include claude, codex, grok, opencode, pi, amp, cursor,
gemini, kimi, rovodev, copilot, codebuddy, factory, qoder, antigravity, omp.
Launchers: `claude-teams`, `codex-teams`, `omo`, `omx`, `omc`.
`agent-hibernation`. `markdown` viewer.

**Cloud / iOS remotes:** `vm`/`cloud` (ls, create, shell, rm, ssh, exec),
`remotes add|list|remove` (Tailscale-only hosts for the iOS companion).

**Internal / test:** `vm-pty-attach`, `set-app-focus`, `simulate-app-active`,
`restore` (surface process replace).

Full table: [INVENTORIES.md](INVENTORIES.md) and `raw/cmux/github-docs/cli-contract.md`.

## Socket API (v2 JSON-RPC)

```json
{"id":"req-1","method":"workspace.list","params":{}}
```

Legacy v1 `{"command":"..."}` is **not** supported on the current website.
Mintlify still shows a mix; do not emit v1.

Core methods documented on Mintlify/examples: `system.ping|capabilities|identify`,
`window.*`, `workspace.*`, `pane.list|create|focus|surfaces`,
`surface.list|create|split|close|focus|move|reorder|read_text|send_text|send_key|trigger_flash|health`,
`tab.action`, `notification.*`, plus the large `browser.*` tree. Website
sidebar metadata still documents some v1-ish `set_status` text forms — prefer
the CLI `cmux set-status` which this plugin already uses.

`cmux rpc` calls a raw v2 method. `cmux events` streams reconnectable NDJSON.

## Custom sidebars

GitHub `docs/custom-sidebars.md` (not on cmux.com/docs as `.md`):

- Files: `~/.config/cmux/sidebars/<name>.swift` (interpreted SwiftUI subset) or `.json`.
- No imports, no child processes in the interpreter. Compiled ExtensionKit is a
  different sandbox lane.
- `cmux sidebar open <name>` hot-reloads as a pane. Toggle: Settings → Custom
  Sidebars / `customSidebars.beta.enabled`.
- Renderer: `inProcess` (default, native hover/keyboard) vs `remote` (crash
  isolation, click-only).
- Bind live `workspaces` context; tap with `cmux(...)` actions; `Reorderable`
  for workspace lists.

This plugin ships `sidebars/herdr.swift` into that directory.

## Remote tmux (beta) — why this plugin exists

Settings → Beta Features → Remote tmux. `cmux ssh-tmux <destination>` mirrors a
**remote tmux** session into native cmux workspaces/tabs/splits via `tmux -CC`.

| tmux | cmux projection |
|---|---|
| session | workspace |
| window | tab |
| pane | native split inside that tab |

Two-way: cmux split/close → `split-window`; tab reorder → `swap-window`. Needs
tmux 3.2+. Does **not** apply to Herdr. Native Herdr window-mirror is a
separate upstream track (`RemoteHerdrWindowMirror`, this plugin's
`docs/upstream/`).

## Skills

Install: `npx skills add manaflow-ai/cmux -g -y` or `skills.sh`.

Documented end-user skills: `cmux`, `cmux-workspace`, `cmux-settings`,
`cmux-customization`, `cmux-diagnostics`, `cmux-browser`, `cmux-markdown`.
Repo also has contributor skills (`cmux-architecture`, `cmux-custom-sidebar`,
`cmux-socket-policy`, `cmux-keyboard-shortcuts`, …). Suggested-but-unshipped:
`cmux-ssh`, `cmux-cloud-vm`, `cmux-vault`.

## Keyboard (cmux-owned, not Ghostty)

Full table: `raw/cmux/site/docs/keyboard-shortcuts.md`. Highlights from concepts:

- New workspace `⌘N`, jump `⌘1–9`, close `⌘⇧W`
- Split right `⌘D`, down `⌘⇧D`, pane nav `⌥⌘` arrows
- New surface `⌘T`, surface nav `⌘[` / `⌘]`
- New window `⌘⇧N`
- Jump to latest unread (site marketing: `⌘⇧U` / `⌃⌘U`)
- Two-step chords in `cmux.json` (`["ctrl+b","c"]` tmux-style)

Terminal keybindings still come from Ghostty config.

## Other product surfaces (scraped, not core CLI)

From llms.txt / site: iOS companion, Linux page, Cloud VMs, Vault, Feed, Dock,
Task Manager, TextBox, Finder, Fork, markdown viewer, passkeys in browser,
Founders Edition / pricing / enterprise, compare pages vs Ghostty, iTerm2,
tmux, Warp, **Herdr**, Conductor, Cursor, Devin, etc.

DeepWiki (81 pages, last indexed 2026-07-14) maps Swift types: `TabManager`,
`WorkspacesModel`, `BonsplitController`, `GhosttyTerminalView`, `CmuxWebView`,
`TerminalController` (socket), `SessionPersistenceStore`, `RemoteTmuxController`,
ExtensionKit custom sidebars, iOS app, Ghostty fork / OSC, CI/release/Homebrew.
Extract: `raw/deepwiki/cmux-extracted.md`.
