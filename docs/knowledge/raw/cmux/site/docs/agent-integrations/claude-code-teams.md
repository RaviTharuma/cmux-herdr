# [#](#title)Claude Code Teams

# [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/claude-code-teams#title)Claude Code Teams

cmux claude-teams launches Claude Code with agent teams enabled. When Claude spawns teammate agents, they appear as native cmux splits instead of tmux panes, with full sidebar metadata and notifications.

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/claude-code-teams#usage)Usage

```
cmux claude-teams
cmux claude-teams --continue
cmux claude-teams --model sonnet
```

All arguments after claude-teams are forwarded to Claude Code. The command defaults teammate mode to auto and sets the environment so Claude uses cmux splits.

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/claude-code-teams#how-it-works)How it works

cmux claude-teams creates a tmux shim script and configures the environment so Claude Code thinks it's running inside tmux. When Claude issues tmux commands to manage teammate panes, the shim translates them into cmux socket API calls.

-   Creates a tmux shim at ~/.cmuxterm/claude-teams-bin/tmux that redirects to cmux \_\_tmux-compat
-   Sets TMUX and TMUX\_PANE environment variables to simulate a tmux session
-   Prepends the shim directory to PATH so Claude finds the shim before real tmux
-   Enables CLAUDE\_CODE\_EXPERIMENTAL\_AGENT\_TEAMS=1 and sets teammate mode to auto

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/claude-code-teams#env-vars)Environment variables

| Variable | Purpose |
| --- | --- |
| `TMUX` | Fake tmux socket path encoding the current cmux workspace and pane |
| `TMUX_PANE` | Fake tmux pane identifier mapped to the current cmux pane |
| `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS` | Enables Claude Code agent teams feature |
| `CMUX_SOCKET_PATH` | Path to the cmux control socket for the shim to connect to |

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/claude-code-teams#directories)Directories

| Path | Purpose |
| --- | --- |
| `~/.cmuxterm/claude-teams-bin/` | Contains the tmux shim script that translates tmux commands to cmux API calls |
| `~/.cmuxterm/tmux-compat-store.json` | Persistent storage for tmux-compat buffers and hooks |

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/claude-code-teams#tmux-commands)Supported tmux commands

The shim translates these tmux commands into cmux operations:

-   `new-session`, `new-window` → creates a new cmux workspace
-   `split-window` → splits the current cmux pane
-   `send-keys` → sends text to a cmux surface
-   `capture-pane` → reads terminal text from a cmux surface
-   `select-pane`, `select-window` → focuses a cmux pane or workspace
-   `kill-pane`, `kill-window` → closes a cmux surface or workspace
-   `list-panes`, `list-windows` → lists cmux panes or workspaces

[Remote tmux](https://cmux-docs-release.vercel.app/docs/remote-tmux) [oh-my-opencode](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-opencode)

Canonical: https://cmux-docs-release.vercel.app/docs/agent-integrations/claude-code-teams
