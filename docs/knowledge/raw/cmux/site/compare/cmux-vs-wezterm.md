# cmux vs WezTerm

[← Compare cmux](https://cmux.com/compare)


WezTerm is a GPU-accelerated cross-platform terminal emulator and multiplexer. cmux is a native macOS terminal and browser workspace built on libghostty that adds agent-aware workspaces, notification rings, unread keyboard navigation, and a Unix socket API.

## Cross-platform terminal multiplexer vs Mac agent console

Use WezTerm when you want a powerful cross-platform terminal, Lua configuration, and terminal/mux automation. Use cmux when you work locally on a Mac, run many coding agents, and want notification rings, Cmd+Shift+U, Cmd+Control+U, browser panes, sidebar metadata, and app-level socket control.

| Dimension | cmux | WezTerm |
| --- | --- | --- |
| **Primary job** | Native macOS agent supervision | Cross-platform terminal emulator and multiplexer |
| **Desktop stack** | Native Swift/AppKit macOS app built on libghostty | Rust GPU terminal emulator and multiplexer |
| **Rendering engine** | Built on libghostty for modern GPU-accelerated terminal rendering | GPU-accelerated terminal renderer with built-in mux |
| **Organization** | Vertical workspaces, branch and port metadata, unread state | Tabs, panes, windows, and multiplexing |
| **Attention model** | Notification rings, unread badges, Cmd+Shift+U, Cmd+Control+U | Terminal notifications and custom status |
| **Browser workflow** | Built-in browser panes with automation | External browser for links and web checks |
| **Best fit** | Mac developers multitasking across agents | Users needing cross-platform terminal power |

## WezTerm is broader as a terminal

WezTerm runs across platforms and includes terminal multiplexing features. cmux intentionally focuses on the macOS agent workflow and the UI around local coding sessions.

## cmux uses Ghostty's terminal core

cmux uses libghostty for terminal rendering, giving the Mac app a modern GPU-accelerated terminal engine, strong compatibility with modern terminal protocols, graphics support, and Ghostty-style visual configuration.

## cmux makes agent attention visible

When many agents run at once, the hardest problem is finding the one waiting for review. cmux keeps unread state visible with pane rings and sidebar badges, then makes it keyboard-addressable.

## Shortcuts encode the multitasking workflow

Cmd+Shift+U jumps to the latest unread pane. Cmd+Control+U cycles unread sessions without clearing them. Those are not generic terminal shortcuts; they are agent-review shortcuts.

## Browser automation lives beside terminals

cmux can split browser panes next to terminals and expose browser automation through the same socket API, so agents can verify local web apps without leaving the workspace.

## FAQ

### Can WezTerm run AI coding agents?

Yes. WezTerm can run CLI agents like any terminal and automate terminal panes through its CLI. cmux adds agent-specific unread state, notification rings, sidebar metadata, and browser automation beside those terminals.

### Why choose cmux on macOS?

Choose cmux when your bottleneck is multitasking across many local agents and you want the terminal app itself to track attention.

### When is WezTerm the better choice?

Use WezTerm when cross-platform terminal behavior, terminal multiplexing, and Lua-configured terminal workflows matter more than a Mac-native agent console.

cmux is free and open source for macOS.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

See also

-   [Compare cmux](https://cmux.com/compare)
-   [Best terminals and agent workspaces for AI coding agents in 2026](https://cmux.com/compare/best-terminal-for-ai-coding-agents)
-   [How to run multiple Claude Code agents in parallel](https://cmux.com/compare/multiple-claude-code-agents-parallel)
-   [cmux vs Warp](https://cmux.com/compare/cmux-vs-warp)
-   [cmux vs Windsurf](https://cmux.com/compare/cmux-vs-windsurf)
-   [A terminal for coding agents](https://cmux.com/agents)
-   [Keyboard shortcuts](https://cmux.com/docs/keyboard-shortcuts)
-   [Browser automation](https://cmux.com/docs/browser-automation)

Canonical: https://cmux.com/compare/cmux-vs-wezterm
