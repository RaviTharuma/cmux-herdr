# cmux vs Zed

[← Compare cmux](https://cmux.com/compare)


Zed is a fast open-source code editor with collaborative and AI features. cmux is a GPL-licensed native macOS terminal and browser workspace for supervising shell-native agents beside Zed or any other editor.

## Fast editor vs terminal agent workspace

Use Zed when you want a fast open-source editor with collaboration, integrated AI, ACP External Agents, and Terminal Threads. Use cmux when you want Claude Code, Codex, OpenCode, Gemini CLI, test runners, dev servers, SSH, and browser checks visible in a libghostty-based terminal app with notification rings, Cmd+Shift+U, Cmd+Control+U, and a scriptable socket API.

| Dimension | cmux | Zed |
| --- | --- | --- |
| **Primary surface** | Terminal, browser, workspaces, splits | Code editor with collaboration and AI features |
| **Desktop stack** | Native Swift/AppKit macOS app built on libghostty | Rust/GPUI GPU-accelerated desktop editor, not Electron/Tauri |
| **Source model** | Free and open source GPL app | Open source; primarily GPL-3.0-or-later with Apache-2.0 components where marked |
| **Agent workflow** | Any CLI agent or shell tool in visible terminal workspaces | Zed Agent, ACP External Agents, and terminal-backed agent threads inside the editor |
| **Terminal engine** | Built on libghostty for terminal rendering | Built-in terminal emulator using Alacritty as its backend |
| **Attention model** | Pane rings, sidebar unread state, Cmd+Shift+U, Cmd+Control+U | Editor tabs, panels, and notifications |
| **Best fit** | Supervising many terminal agents beside any editor | Editing code quickly with collaborative editor features |

## cmux and Zed can be paired

cmux does not need to be your editor. Use Zed for editing and cmux for terminal agents, browser checks, local services, logs, and review queues.

## Terminal agents need a terminal control plane

When agents run in shells, the workflow includes commands, tests, file watchers, ports, logs, permissions, and prompts. cmux makes those sessions visible as workspaces with branch, directory, latest output, and unread state.

## Notifications are first-class

cmux does not treat agent completion as a disposable desktop notification. It keeps unread state in the app, rings the pane, and gives you Cmd+Shift+U and Cmd+Control+U for fast review navigation.

## The workflow is programmable

The cmux CLI and socket API let scripts and agents create workspaces, split panes, send input, read screens, capture screenshots, and drive browser panes around whatever editor you use.

## FAQ

### Does cmux replace Zed?

No. Zed is an editor. cmux is a terminal and browser workspace for running agents and tools beside the editor.

### Is Zed open source?

Yes. Zed is open source, so the comparison is about product layer rather than open versus closed source.

### Why use cmux with Zed?

Use cmux when you want terminal agent supervision, notification rings, browser automation, and scriptable workspaces outside the editor.

cmux is free and open source for macOS.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

See also

-   [Compare cmux](https://cmux.com/compare)
-   [Best terminals and agent workspaces for AI coding agents in 2026](https://cmux.com/compare/best-terminal-for-ai-coding-agents)
-   [How to run multiple Claude Code agents in parallel](https://cmux.com/compare/multiple-claude-code-agents-parallel)
-   [cmux vs Windsurf](https://cmux.com/compare/cmux-vs-windsurf)
-   [A terminal for coding agents](https://cmux.com/agents)
-   [Keyboard shortcuts](https://cmux.com/docs/keyboard-shortcuts)
-   [Browser automation](https://cmux.com/docs/browser-automation)

Canonical: https://cmux.com/compare/cmux-vs-zed
