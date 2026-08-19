# cmux vs Ghostty

[← Compare cmux](https://cmux.com/compare)


Ghostty is a fast terminal emulator. cmux is a separate native macOS app built on libghostty, adding workspaces, vertical tabs, notification rings, browser panes, and a socket API for coding-agent workflows.

## Same rendering engine family, different job

Use Ghostty when you want a focused terminal emulator. Use cmux when you want libghostty rendering inside an agent workspace with persistent unread state, browser panes, and a Unix socket that lets agents create panes, read terminals, capture screenshots, and drive web checks.

| Dimension | cmux | Ghostty |
| --- | --- | --- |
| **Primary job** | Agent supervision and multitasking | General-purpose terminal |
| **Desktop stack** | Native Swift/AppKit macOS app built on libghostty | Native SwiftUI macOS app with a shared Zig core and libghostty |
| **Rendering** | Built on libghostty | Metal on macOS, OpenGL on Linux, shared Ghostty/libghostty terminal core |
| **Organization** | Vertical workspaces, splits, sidebar metadata | Terminal windows, tabs, splits |
| **Agent attention** | Notification rings, Cmd+Shift+U, Cmd+Control+U | Desktop notifications, tabs, splits, and AppleScript; no documented cmux-style unread rings or browser panes |
| **Automation** | cmux CLI, socket API, browser automation | AppleScript and Shortcuts for terminal windows, tabs, splits, and input |

## cmux is not a Ghostty fork

Ghostty is an excellent terminal emulator and now has AppleScript for terminal automation. cmux goes further for agent workflows: terminal control, workspace state, notifications, screenshots, and browser panes are exposed through one CLI/socket surface, so an agent can run commands and verify a web app without leaving the workspace.

## Vertical workspaces scale better for agents

Agent sessions benefit from persistent names, branch context, directory context, ports, and unread state. cmux puts that metadata in the sidebar so dozens of sessions stay scannable.

## Notifications become navigable state

Desktop notifications are easy to miss once they clear. cmux keeps unread state inside the app, rings the pane, marks the sidebar, and lets Cmd+Shift+U jump to the latest unread session.

## Automation reaches outside the shell

cmux exposes app-level actions through a CLI and Unix socket, including browser automation. Agents can inspect web pages, click, type, read terminal screens, and take screenshots from the same workspace.

## FAQ

### Is cmux built by the Ghostty team?

No. cmux is a separate project that uses libghostty for terminal rendering.

### Does cmux read Ghostty config?

Yes. cmux reads your Ghostty config for terminal rendering choices such as fonts, themes, colors, and cursor behavior.

### When should I use cmux instead of Ghostty?

Use cmux when you run multiple coding agents and want workspaces, notification rings, unread navigation, browser panes, and programmability around them.

cmux is free and open source for macOS.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

See also

-   [Compare cmux](https://cmux.com/compare)
-   [Best terminals and agent workspaces for AI coding agents in 2026](https://cmux.com/compare/best-terminal-for-ai-coding-agents)
-   [How to run multiple Claude Code agents in parallel](https://cmux.com/compare/multiple-claude-code-agents-parallel)
-   [cmux vs Devin](https://cmux.com/compare/cmux-vs-devin)
-   [cmux vs Herdr](https://cmux.com/compare/cmux-vs-herdr)
-   [A terminal for coding agents](https://cmux.com/agents)
-   [Keyboard shortcuts](https://cmux.com/docs/keyboard-shortcuts)
-   [Browser automation](https://cmux.com/docs/browser-automation)

Canonical: https://cmux.com/compare/cmux-vs-ghostty
