# cmux vs Kitty

[← Compare cmux](https://cmux.com/compare)


Kitty is a fast, feature-rich, GPU-based terminal emulator. cmux is a native macOS terminal and browser workspace built on libghostty for developers who run many AI coding agents and need visible workspaces, notifications, unread navigation, and app-level automation.

## Feature-rich terminal vs agent multitasking workspace

Use Kitty when you want a powerful configurable terminal. Use cmux when coding agents have turned your terminal into a multitasking queue and you need Cmd+Shift+U, Cmd+Control+U, notification rings, workspace metadata, browser checks, and scriptable control.

| Dimension | cmux | Kitty |
| --- | --- | --- |
| **Primary job** | Agent supervision and multitasking | Fast feature-rich terminal emulation |
| **Desktop stack** | Native Swift/AppKit macOS app built on libghostty | OpenGL-rendered terminal emulator written in C, Python, and Go, not an Electron or Tauri app |
| **Rendering engine** | Built on libghostty for modern GPU-accelerated terminal rendering | Kitty terminal renderer and protocol ecosystem |
| **Agent visibility** | Vertical workspaces, latest output, branch and port metadata | Windows, tabs, layouts, sessions, kittens, and remote-control scripting |
| **Attention model** | Notification rings, unread badges, Cmd+Shift+U, Cmd+Control+U | OSC 99 desktop notifications and kitten notify, with custom scripting for unread workflows |
| **Browser workflow** | Built-in browser panes with automation | Opens URLs through OS/default handlers; no built-in browser pane or DOM automation |
| **Best fit** | Mac users running many local agents | Terminal users who want deep terminal features |

## Kitty is a great terminal primitive

Kitty has strong terminal features and performance. cmux focuses on the layer above: which agent is running, which one needs you, which branch it owns, and which browser check belongs beside it.

## cmux gets Ghostty's terminal engine

cmux uses libghostty for terminal rendering, so it inherits a modern GPU-accelerated engine, strong terminal compatibility, graphics support, synchronized rendering behavior, and Ghostty-style configuration for visual terminal details.

## cmux treats notifications as state

Desktop notifications are easy to miss when ten agents finish. cmux keeps unread state in the app, rings the pane, and gives you shortcuts to move through the queue without losing track.

## Keyboard shortcuts are built for multitasking

Cmd+Shift+U jumps to the latest unread agent. Cmd+Control+U cycles unread work while keeping it unread. Those shortcuts exist because cmux is built by people who run many agents at once.

## Programmability includes the app shell

The cmux CLI and socket API can create workspaces, split panes, send input, read screens, drive browser panes, and update sidebar state from scripts or agents.

## FAQ

### Can Kitty run Claude Code, Codex, and OpenCode?

Yes. Kitty is a terminal, so CLI agents run there. cmux adds agent-aware workspace organization and notification navigation around those CLIs.

### Why use cmux if Kitty is scriptable?

cmux exposes higher-level app primitives: workspaces, panes, browser surfaces, screenshots, unread state, notification rings, and sidebar metadata.

### When is Kitty the better choice?

Use Kitty when you want a configurable terminal emulator and plan to build the agent workflow around it yourself.

cmux is free and open source for macOS.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

See also

-   [Compare cmux](https://cmux.com/compare)
-   [Best terminals and agent workspaces for AI coding agents in 2026](https://cmux.com/compare/best-terminal-for-ai-coding-agents)
-   [How to run multiple Claude Code agents in parallel](https://cmux.com/compare/multiple-claude-code-agents-parallel)
-   [cmux vs iTerm2](https://cmux.com/compare/cmux-vs-iterm2)
-   [cmux vs OpenCode](https://cmux.com/compare/cmux-vs-opencode)
-   [A terminal for coding agents](https://cmux.com/agents)
-   [Keyboard shortcuts](https://cmux.com/docs/keyboard-shortcuts)
-   [Browser automation](https://cmux.com/docs/browser-automation)

Canonical: https://cmux.com/compare/cmux-vs-kitty
