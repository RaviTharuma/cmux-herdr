# cmux vs iTerm2

[← Compare cmux](https://cmux.com/compare)


iTerm2 is a mature macOS terminal replacement. cmux is a native macOS terminal built on libghostty for running many AI coding agents, with vertical workspaces, notification rings, browser panes, and a scriptable socket API.

## General terminal vs agent supervision terminal

Use iTerm2 when you want a broad, configurable terminal emulator. Use cmux when your terminal has become an agent operations console and you want libghostty terminal rendering plus every session, branch, port, unread state, browser check, notification ring, and Cmd+Shift+U/Cmd+Control+U review shortcut to stay visible.

| Dimension | cmux | iTerm2 |
| --- | --- | --- |
| **Primary job** | Supervise many coding agents | General-purpose macOS terminal |
| **Desktop stack** | Native Swift/AppKit macOS app built on libghostty | Native macOS terminal app, not Electron/Tauri |
| **Agent visibility** | Vertical workspaces, latest output, branch and port metadata | Tabs, windows, splits, profiles, shell integration, and status bar |
| **Rendering engine** | Built on libghostty with GPU-accelerated modern terminal rendering | iTerm2 terminal renderer |
| **Attention model** | Notification rings, Cmd+Shift+U, Cmd+Control+U | OSC notifications, marks and alerts, trigger notifications, and session search |
| **Automation** | CLI, Unix socket API, browser automation, hooks | Python API, deprecated AppleScript, triggers, profiles, and tmux integration |
| **Best fit** | Agent-heavy local development | General terminal power users |

## cmux adds agent state to the terminal

Terminal tabs do not know which agent is blocked, which branch it owns, or which dev server it started. cmux puts that context in the sidebar and makes waiting panes easy to find.

## Browser automation is part of the workflow

iTerm2 also has browser profiles. cmux's difference is that browser panes are addressable from the same CLI and socket workflow as workspaces and terminals, with commands for DOM interaction, screenshots, console and error logs, cookies, storage, and JavaScript evaluation.

## Keyboard shortcuts target attention

Cmd+Shift+U jumps to the latest unread agent. Cmd+Control+U cycles unread sessions while keeping them unread. iTerm2 is mature and configurable, but these agent-review shortcuts are the kind of product detail cmux adds because it is built for heavy multitasking.

## cmux inherits Ghostty strengths

Because cmux is built on libghostty, it gets a modern GPU-accelerated terminal engine, strong rendering performance, broad terminal compatibility, and Ghostty-style configuration for terminal details such as fonts, themes, colors, and cursor behavior.

## Programmability reaches the app

Scripts and agents can create workspaces, split panes, send input, read screens, capture screenshots, and drive browsers through cmux. The terminal becomes controllable infrastructure.

## FAQ

### Should every iTerm2 user switch to cmux?

No. iTerm2 is excellent as a general terminal. cmux is aimed at developers whose terminal is dominated by parallel coding agents.

### Can cmux run normal shell work?

Yes. cmux is still a terminal, so shells, SSH, scripts, tests, package managers, and local services run normally.

### What does cmux add beyond tabs and splits?

cmux adds agent workspace metadata, notification rings, unread navigation, socket-addressable browser surfaces, and a socket API designed around agent-heavy workflows.

cmux is free and open source for macOS.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

See also

-   [Compare cmux](https://cmux.com/compare)
-   [Best terminals and agent workspaces for AI coding agents in 2026](https://cmux.com/compare/best-terminal-for-ai-coding-agents)
-   [How to run multiple Claude Code agents in parallel](https://cmux.com/compare/multiple-claude-code-agents-parallel)
-   [cmux vs Herdr](https://cmux.com/compare/cmux-vs-herdr)
-   [cmux vs Kitty](https://cmux.com/compare/cmux-vs-kitty)
-   [A terminal for coding agents](https://cmux.com/agents)
-   [Keyboard shortcuts](https://cmux.com/docs/keyboard-shortcuts)
-   [Browser automation](https://cmux.com/docs/browser-automation)

Canonical: https://cmux.com/compare/cmux-vs-iterm2
