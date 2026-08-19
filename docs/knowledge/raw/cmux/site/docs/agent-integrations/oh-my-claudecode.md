# [#](#title)oh-my-claudecode

# [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-claudecode#title)oh-my-claudecode

cmux omc launches Oh My Claude Code (OMC) in a cmux-aware environment. OMC is a multi-agent orchestration system for Claude Code with 19 specialized agents, smart model routing, and tmux-based team pipelines. When OMC spawns team panes, they become native cmux splits.

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-claudecode#usage)Usage

```
cmux omc
cmux omc team 3:claude "implement feature"
cmux omc --watch
```

All arguments after omc are forwarded to the omc CLI.

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-claudecode#what-you-get)What you get

OMC's team mode uses tmux for pane management. With cmux omc, those panes become native cmux splits:

-   Team worker panes (Claude/Codex/Gemini sessions) appear as cmux splits in the workspace
-   HUD monitoring display shows live status in a split pane
-   Auto-layout management arranges agent panes in a main-vertical grid
-   Your main session stays in the primary pane while agents work beside it

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-claudecode#prerequisites)Prerequisites

```
npm install -g oh-my-claude-sisyphus
```

OMC requires Claude Code CLI installed and authenticated.

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-claudecode#how-it-works)How it works

Same pattern as cmux claude-teams. A tmux shim intercepts tmux commands from OMC and translates them into cmux API calls.

-   Creates a tmux shim at ~/.cmuxterm/omc-bin/tmux that redirects to cmux \_\_tmux-compat
-   Sets TMUX and TMUX\_PANE to simulate a tmux session
-   Prepends the shim directory to PATH
-   Injects NODE\_OPTIONS restore module for Claude Code compatibility
-   Execs into omc with all remaining arguments forwarded

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-claudecode#directories)Directories

| Path | Purpose |
| --- | --- |
| `~/.cmuxterm/omc-bin/` | Contains the tmux shim script |
| `~/.cmuxterm/tmux-compat-store.json` | Persistent storage for tmux-compat buffers and hooks |

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-claudecode#env-vars)Environment variables

| Variable | Purpose |
| --- | --- |
| `TMUX` | Fake tmux socket path encoding the current cmux workspace and pane |
| `TMUX_PANE` | Fake tmux pane identifier mapped to the current cmux pane |
| `CMUX_SOCKET_PATH` | Path to the cmux control socket for the shim to connect to |

[oh-my-pi](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-pi) [Changelog](https://cmux-docs-release.vercel.app/docs/changelog)

Canonical: https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-claudecode
