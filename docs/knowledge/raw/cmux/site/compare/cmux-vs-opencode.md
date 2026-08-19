# cmux vs OpenCode

[← Compare cmux](https://cmux.com/compare)


OpenCode is an open source coding agent available in terminal, desktop, and IDE surfaces. cmux is the native terminal and browser workspace where OpenCode and other CLI agents can run side by side.

## Agent vs agent workspace

Use OpenCode when you want an open source coding agent. Use cmux when you want a native macOS workspace that can supervise OpenCode, Claude Code, Codex CLI, Gemini CLI, Aider, Amp, Cursor CLI, and ordinary shell workflows together, alongside separate Codex app and IDE extension workflows.

| Dimension | cmux | OpenCode |
| --- | --- | --- |
| **Primary job** | Terminal and browser supervision layer | Open source coding agent |
| **Desktop stack** | Native Swift/AppKit macOS app built on libghostty | Terminal TUI plus Electron desktop beta |
| **Agent choice** | Any CLI agent or shell command | OpenCode with configured model providers |
| **Parallel work** | Many workspaces, panes, unread rings, branch metadata | OpenCode sessions and agent features |
| **Programmability** | cmux CLI, Unix socket API, browser automation | OpenCode CLI, config, SDK, and custom tools |
| **Best fit** | Operators running several agents and tools | Developers who want OpenCode as the agent |

## OpenCode can run inside cmux

OpenCode is an agent. cmux is the terminal surface that can keep OpenCode, Claude Code, Codex CLI, and other shell agents visible together. Codex app and IDE extension workflows can stay in their own surfaces while cmux supervises the long-running terminals where agents, tests, servers, and logs actually sit.

## cmux keeps agent choice portable

If one model or agent is better for a task, run it in a separate workspace. cmux adds organization and attention tracking without requiring every task to use the same agent.

## The missing layer is supervision

When several agent sessions run for minutes or hours, the hard part is knowing which one needs review. cmux uses unread state, notification rings, Cmd+Shift+U, and Cmd+Control+U for that loop.

## Browser and terminal automation share one surface

cmux exposes terminal screen reads, screenshots, and browser automation through the socket API, so agents can verify web changes from the same workspace where they ran commands.

## FAQ

### Is cmux an alternative to OpenCode?

No. OpenCode is a coding agent. cmux is a terminal and browser workspace that can run OpenCode and other agents.

### Can I run several OpenCode sessions in cmux?

Yes. Use separate cmux workspaces or panes so each session has visible context, unread state, and keyboard navigation.

### Why use cmux if OpenCode has its own surfaces?

cmux is useful when OpenCode is one part of a wider workflow that includes other agents, shell tools, dev servers, SSH, and browser checks.

cmux is free and open source for macOS.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

See also

-   [Compare cmux](https://cmux.com/compare)
-   [Best terminals and agent workspaces for AI coding agents in 2026](https://cmux.com/compare/best-terminal-for-ai-coding-agents)
-   [How to run multiple Claude Code agents in parallel](https://cmux.com/compare/multiple-claude-code-agents-parallel)
-   [cmux vs Kitty](https://cmux.com/compare/cmux-vs-kitty)
-   [cmux vs Superset](https://cmux.com/compare/cmux-vs-superset)
-   [A terminal for coding agents](https://cmux.com/agents)
-   [Keyboard shortcuts](https://cmux.com/docs/keyboard-shortcuts)
-   [Browser automation](https://cmux.com/docs/browser-automation)

Canonical: https://cmux.com/compare/cmux-vs-opencode
