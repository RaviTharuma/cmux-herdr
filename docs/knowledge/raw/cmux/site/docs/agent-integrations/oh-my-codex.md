# [#](#title)oh-my-codex

# [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-codex#title)oh-my-codex

cmux omx launches Oh My Codex (OMX) in a cmux-aware environment. OMX is a multi-agent orchestration layer for OpenAI Codex CLI with 30+ specialized agent roles, workflow skills, and tmux-based parallel team execution. When OMX spawns team panes or HUD displays, they become native cmux splits.

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-codex#usage)Usage

```
cmux omx
cmux omx --madmax --high
cmux omx team
```

All arguments after omx are forwarded to the omx CLI.

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-codex#what-you-get)What you get

OMX's team mode and HUD use tmux for pane management. With cmux omx, those panes become native cmux splits:

-   Team worker panes (Codex/Claude sessions) appear as cmux splits in the workspace
-   HUD status display shows model, branch, context, and token usage in a split pane
-   Auto-layout management arranges agent panes in a main-vertical grid
-   Your main session stays in the primary pane while workers operate beside it

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-codex#prerequisites)Prerequisites

```
npm install -g @openai/codex oh-my-codex
omx setup
omx doctor
```

OMX requires OpenAI Codex CLI and a working Codex auth setup. Run omx doctor to verify your installation.

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-codex#how-it-works)How it works

Same pattern as other cmux agent integrations. A tmux shim intercepts tmux commands from OMX and translates them into cmux API calls.

-   Creates a tmux shim at ~/.cmuxterm/omx-bin/tmux that redirects to cmux \_\_tmux-compat
-   Sets TMUX and TMUX\_PANE to simulate a tmux session
-   Prepends the shim directory to PATH
-   Execs into omx with all remaining arguments forwarded

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-codex#directories)Directories

| Path | Purpose |
| --- | --- |
| `~/.cmuxterm/omx-bin/` | Contains the tmux shim script |
| `~/.cmuxterm/tmux-compat-store.json` | Persistent storage for tmux-compat buffers and hooks |

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-codex#env-vars)Environment variables

| Variable | Purpose |
| --- | --- |
| `TMUX` | Fake tmux socket path encoding the current cmux workspace and pane |
| `TMUX_PANE` | Fake tmux pane identifier mapped to the current cmux pane |
| `CMUX_SOCKET_PATH` | Path to the cmux control socket for the shim to connect to |

[oh-my-opencode](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-opencode) [oh-my-pi](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-pi)

Canonical: https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-codex
