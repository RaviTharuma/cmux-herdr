# cmux vs tmux

[← Compare cmux](https://cmux.com/compare)


tmux is the classic terminal multiplexer: portable, scriptable, and excellent over SSH. cmux is a native macOS terminal workspace for AI coding agents, adding visual organization, notification rings, browser panes, and a Unix socket API.

## Multiplexer vs agent operations console

Use tmux when portability, detach/reattach, and remote terminal multiplexing are the priority. Use cmux when you are on macOS and want a libghostty-based native terminal app with visible agent metadata, browser automation, unread navigation, and lower-friction review across many local agent sessions.

| Dimension | cmux | tmux |
| --- | --- | --- |
| **Primary job** | Native macOS agent supervision | Terminal multiplexing |
| **Desktop stack** | Native Swift/AppKit macOS app built on libghostty | Terminal multiplexer running inside your terminal |
| **Platform** | macOS app built on Swift/AppKit and libghostty | Runs inside terminals across Unix-like systems |
| **Rendering engine** | Built on libghostty for terminal rendering | Uses the outer terminal emulator's rendering |
| **Organization** | Workspaces, vertical sidebar, branch and port metadata | Sessions, windows, panes, status line |
| **Attention model** | Notification rings, unread badges, Cmd+Shift+U, Cmd+Control+U | Status-line alerts for activity, silence, and bells, plus hooks and shell customization |
| **Best fit** | Local AI coding agent control on Mac | Remote shells and portable terminal multiplexing |

## tmux is still excellent infrastructure

For remote servers, persistent shell sessions, and keyboard-driven multiplexing inside any terminal, tmux remains a strong default. cmux focuses on the local Mac app layer around coding agents.

## cmux makes agent state visible

Agent work benefits from names, branches, directories, ports, latest output, unread state, and browser checks. cmux keeps that context visible instead of burying it in pane titles or status scripts.

## cmux brings its own Ghostty-based renderer

tmux depends on the terminal emulator around it. cmux is the terminal app, using libghostty for smooth GPU-accelerated rendering and Ghostty-style visual configuration.

## Native UI lowers review friction

cmux uses a vertical sidebar, split panes, browser surfaces, screenshots, notification rings, Cmd+Shift+U, and Cmd+Control+U so the review loop feels like app navigation rather than terminal archaeology.

## The socket API is app-level automation

tmux is deeply scriptable for terminal multiplexing. cmux also exposes the app around the terminal: workspaces, browser panes, screen reads, screenshots, and notification metadata.

## FAQ

### Can I still use tmux inside cmux?

Yes. tmux can run inside a cmux terminal pane when you need its session model or remote workflow.

### Why not manage agents with tmux panes?

You can, but cmux adds native sidebar metadata, unread navigation, notification rings, browser panes, and an app-level socket API around those sessions.

### When is tmux the better choice?

Use tmux for remote servers, portable detach/reattach, and workflows that must run inside any terminal.

cmux is free and open source for macOS.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

See also

-   [Compare cmux](https://cmux.com/compare)
-   [Best terminals and agent workspaces for AI coding agents in 2026](https://cmux.com/compare/best-terminal-for-ai-coding-agents)
-   [How to run multiple Claude Code agents in parallel](https://cmux.com/compare/multiple-claude-code-agents-parallel)
-   [cmux vs Superset](https://cmux.com/compare/cmux-vs-superset)
-   [cmux vs VS Code](https://cmux.com/compare/cmux-vs-vscode)
-   [A terminal for coding agents](https://cmux.com/agents)
-   [Keyboard shortcuts](https://cmux.com/docs/keyboard-shortcuts)
-   [Browser automation](https://cmux.com/docs/browser-automation)

Canonical: https://cmux.com/compare/cmux-vs-tmux
