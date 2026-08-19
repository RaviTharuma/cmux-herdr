# cmux vs VS Code

[← Compare cmux](https://cmux.com/compare)


VS Code is a general-purpose editor and extension platform with agent workflows. cmux is a GPL-licensed native macOS terminal and browser workspace for supervising shell-native agents beside any editor, including VS Code.

## Editor platform vs terminal control plane

Use VS Code when you want editing, extensions, built-in agent sessions, and agent/browser tools inside VS Code. Use cmux when you want Claude Code, Codex, OpenCode, Gemini CLI, dev servers, SSH, scripts, and browser checks visible in a libghostty-based terminal workspace with notification rings and keyboard shortcuts for unread work.

| Dimension | cmux | VS Code |
| --- | --- | --- |
| **Primary surface** | Terminal, browser, workspaces, splits | Editor, extensions, terminals, and agent views |
| **Desktop stack** | Native Swift/AppKit macOS app built on libghostty | Electron desktop editor |
| **Source model** | Free and open source GPL app | Code - OSS is MIT; Microsoft VS Code is a distribution with Microsoft-specific customizations under the Microsoft product license |
| **Agent workflow** | Any CLI agent or shell tool in visible workspaces | Built-in VS Code agents, Copilot CLI/cloud/third-party agent types, browser tools, and extensions |
| **Attention model** | Pane rings, sidebar unread state, Cmd+Shift+U, Cmd+Control+U | Editor notifications, views, tabs, and extension UI |
| **Browser workflow** | Built-in browser panes with socket automation | Integrated browser, Live Preview/HTML previews, browser debugging, agent browser tools, and external browser flow |
| **Best fit** | Supervising many terminal agents beside any editor | Editing code with a broad extension ecosystem |

## cmux works beside VS Code

The practical setup is often VS Code for editing and cmux for agent operations. The editor stays focused while cmux tracks shells, agents, local services, browser panes, and review queues.

## Terminal agents need terminal state

Claude Code, Codex, OpenCode, and similar CLIs spend much of their time running commands, tests, and tools. cmux keeps that work visible with branch, directory, port, latest notification text, and unread metadata in the sidebar.

## Notifications are designed for multitasking

cmux was built by people who run many agents at once. Pane rings, unread badges, Cmd+Shift+U, and Cmd+Control+U turn completed or blocked agent sessions into a review queue.

## Programmability stays outside the editor

The cmux CLI and socket API let agents create workspaces, split panes, read screens, send input, capture screenshots, and drive browser panes without depending on one editor extension model.

## FAQ

### Does cmux replace VS Code?

No. cmux is a terminal and browser workspace. It works well beside VS Code or any other editor.

### Is VS Code open source?

The Code - OSS repository is MIT licensed. Microsoft Visual Studio Code is a distribution of Code - OSS with Microsoft-specific customizations under the Microsoft product license.

### Why use cmux with VS Code?

Use cmux when your agent work spans terminals, browsers, scripts, SSH, local services, and multiple repos, and you want that workflow visible outside the editor.

cmux is free and open source for macOS.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

See also

-   [Compare cmux](https://cmux.com/compare)
-   [Best terminals and agent workspaces for AI coding agents in 2026](https://cmux.com/compare/best-terminal-for-ai-coding-agents)
-   [How to run multiple Claude Code agents in parallel](https://cmux.com/compare/multiple-claude-code-agents-parallel)
-   [cmux vs tmux](https://cmux.com/compare/cmux-vs-tmux)
-   [cmux vs Warp](https://cmux.com/compare/cmux-vs-warp)
-   [A terminal for coding agents](https://cmux.com/agents)
-   [Keyboard shortcuts](https://cmux.com/docs/keyboard-shortcuts)
-   [Browser automation](https://cmux.com/docs/browser-automation)

Canonical: https://cmux.com/compare/cmux-vs-vscode
