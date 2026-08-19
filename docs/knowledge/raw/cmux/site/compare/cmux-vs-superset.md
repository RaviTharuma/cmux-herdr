# cmux vs Superset

[← Compare cmux](https://cmux.com/compare)


cmux and Superset both help developers run more than one coding agent. The difference is product shape: Superset is an agent workspace with worktree orchestration, while cmux is a native terminal and browser built for supervising any CLI agent.

## Choose by workflow

Choose cmux when you want a fast, free, open source native terminal, visible multitasking, keyboard-driven attention management, and a programmable socket API. Choose Superset when you want a more opinionated task workspace centered on worktree orchestration and built-in review flow.

| Dimension | cmux | Superset |
| --- | --- | --- |
| **Core product** | Native macOS terminal and browser | Agent orchestration workspace |
| **Desktop stack** | Native Swift/AppKit macOS app built on libghostty | Electron/React desktop app using @xterm/xterm and node-pty |
| **Runtime** | Swift/AppKit plus libghostty | Electron desktop app |
| **Agent model** | Any CLI agent, no cmux runtime lock-in | Any CLI agent inside a Superset-managed workspace |
| **Source model** | Free and open source GPL app | Source-available under Elastic License 2.0 |
| **Organization** | Vertical workspaces, splits, unread state, notification rings | Tasks, worktrees, review surfaces |
| **Programmability** | CLI, Unix socket API, browser automation, hooks | CLI beta, MCP, SDK, and automations for workspace and agent orchestration |
| **Best fit** | Developers who live in the terminal and run many tools | Developers who want a packaged agent task manager |

## cmux keeps the terminal as the primitive

If an agent runs in a shell, it runs in cmux. That makes cmux a good fit for teams that switch between Claude Code, Codex, OpenCode, Gemini CLI, Aider, scripts, SSH, and local dev servers.

## Native performance matters at high concurrency

Running many agents is already CPU and memory heavy. cmux uses Swift/AppKit and libghostty, so its terminal UI avoids Electron overhead while agent processes, compilers, and dev servers consume their own resources.

## Attention is a product surface

cmux focuses on the moment an agent needs a human. Notification rings, unread badges, a notification panel, Cmd+Shift+U for latest unread, and Cmd+Control+U for cycling unread work make finished or blocked sessions easy to find.

## Programmability is the escape hatch

cmux exposes workspaces, panes, screen reads, screenshots, and browser automation through its CLI and socket API. Superset also has CLI/MCP/SDK surfaces for workspace and agent orchestration; the cmux difference is that the terminal and browser panes themselves are scriptable.

## Worktree support is never universal

Built-in worktree managers are useful when every task maps cleanly to a branch. cmux stays closer to the terminal, so Claude Code, Codex, scripts, and custom commands can handle repo layouts, non-Git directories, SSH sessions, or one-off setup paths that do not fit a product workspace model.

## FAQ

### Is cmux a Superset replacement?

It depends on what you use Superset for. cmux can replace the terminal supervision layer, while Superset includes a more opinionated task and worktree workflow.

### Can cmux run the same agents?

Yes. cmux runs CLI agents directly, including Claude Code, Codex, OpenCode, Gemini CLI, Aider, Amp, and Cursor CLI.

### Why pick cmux if Superset has worktrees?

Pick cmux if your priority is native terminal performance, keyboard-driven multitasking, editor-neutral terminal workflows, and scriptable terminal/browser surfaces you can adapt to your own workflow.

cmux is free and open source for macOS.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

See also

-   [Compare cmux](https://cmux.com/compare)
-   [Best terminals and agent workspaces for AI coding agents in 2026](https://cmux.com/compare/best-terminal-for-ai-coding-agents)
-   [How to run multiple Claude Code agents in parallel](https://cmux.com/compare/multiple-claude-code-agents-parallel)
-   [cmux vs OpenCode](https://cmux.com/compare/cmux-vs-opencode)
-   [cmux vs tmux](https://cmux.com/compare/cmux-vs-tmux)
-   [A terminal for coding agents](https://cmux.com/agents)
-   [Keyboard shortcuts](https://cmux.com/docs/keyboard-shortcuts)
-   [Browser automation](https://cmux.com/docs/browser-automation)

Canonical: https://cmux.com/compare/cmux-vs-superset
