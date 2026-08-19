# cmux vs Herdr

[← Compare cmux](https://cmux.com/compare)


Herdr is an open-source terminal-native agent multiplexer that runs inside the terminal you already use and can keep real panes alive over SSH. cmux is a GPL-licensed native macOS app built on libghostty, with vertical workspaces, notification rings, browser panes, and a socket API for local agent workflows.

## Native Mac app vs terminal-native multiplexer

Use Herdr when you want a tmux-like agent multiplexer inside any terminal, especially over SSH or from a small screen. Use cmux when you want a native Mac terminal app with libghostty rendering, app-level workspaces, built-in browser automation, notification rings, Cmd+Shift+U, Cmd+Control+U, and programmable local control. A practical pairing is running Herdr in a cmux terminal pane, locally or inside an SSH session opened from cmux.

| Dimension | cmux | Herdr |
| --- | --- | --- |
| **Primary surface** | Native macOS terminal and browser app | Terminal-native multiplexer inside an existing terminal |
| **Desktop stack** | Native Swift/AppKit macOS app built on libghostty | Single Rust terminal-native binary, not a GUI desktop app or Electron wrapper |
| **Runtime model** | Local app workspaces, panes, browser surfaces, socket API | Background server, persistent PTYs, attach/detach |
| **Rendering engine** | Built on libghostty for terminal rendering | Uses the terminal emulator you run Herdr inside |
| **Remote fit** | SSH panes and local app supervision | Strong SSH attach and terminal persistence |
| **Agent state** | Pane rings, sidebar unread state, Cmd+Shift+U, Cmd+Control+U | Agent state in a terminal multiplexer |
| **Source model** | Free and open source GPL app | Open source AGPL with commercial licensing available |
| **Best fit** | Mac developers who want native app UX and browser verification | Terminal users who want persistent agent multiplexing anywhere they can SSH |

## Herdr can run inside cmux

Herdr is terminal-native, so it can run inside a normal cmux terminal pane. It also works well inside SSH sessions opened from cmux: Herdr keeps remote panes persistent, while cmux gives the Mac-side native app, browser panes, notifications, and unread shortcuts around the workspace.

## cmux owns the native desktop surface

cmux is built as a Swift/AppKit macOS app with libghostty terminal rendering. That lets it provide vertical tabs, app-level workspaces, native panes, screenshots, browser surfaces, and desktop notification behavior without living inside another terminal.

## Both care about agent state

The honest comparison is not that only cmux understands agents. Herdr tracks agent state too. cmux differentiates with Mac-native pane rings, sidebar unread state, Cmd+Shift+U, Cmd+Control+U, and browser automation beside the terminal.

## Browser automation is part of cmux

cmux can split an in-app browser beside an agent and expose navigation, DOM snapshots, clicking, typing, screenshots, console logs, and network inspection through the same CLI/socket control plane.

## FAQ

### Is Herdr open source?

Yes. Herdr is open source under AGPL with commercial licensing available.

### When is Herdr a better fit?

Use Herdr when you want a terminal-native multiplexer that runs inside another terminal, stays alive over SSH, and works well from remote or small-screen terminal clients.

### When is cmux a better fit?

Use cmux when you want a native Mac app built on libghostty, with terminal workspaces, browser panes, notification rings, unread keyboard shortcuts, and a scriptable desktop control plane.

cmux is free and open source for macOS.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

See also

-   [Compare cmux](https://cmux.com/compare)
-   [Best terminals and agent workspaces for AI coding agents in 2026](https://cmux.com/compare/best-terminal-for-ai-coding-agents)
-   [How to run multiple Claude Code agents in parallel](https://cmux.com/compare/multiple-claude-code-agents-parallel)
-   [cmux vs Ghostty](https://cmux.com/compare/cmux-vs-ghostty)
-   [cmux vs iTerm2](https://cmux.com/compare/cmux-vs-iterm2)
-   [A terminal for coding agents](https://cmux.com/agents)
-   [Keyboard shortcuts](https://cmux.com/docs/keyboard-shortcuts)
-   [Browser automation](https://cmux.com/docs/browser-automation)

Canonical: https://cmux.com/compare/cmux-vs-herdr
