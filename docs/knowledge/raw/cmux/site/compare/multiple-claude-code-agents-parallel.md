# How to run multiple Claude Code agents in parallel

[← Compare cmux](https://cmux.com/compare)


Parallel Claude Code work needs isolation, visibility, and a fast way to find the session that needs input. The same is true for Codex, OpenCode, Gemini CLI, Aider, Amp, Cursor CLI, and any agent that runs in a shell. cmux gives every task a visible workspace and keeps sessions readable while they run side by side.

## Safe parallel agent workflow

Create one branch, worktree, checkout, repo folder, or custom workspace per task, open each in cmux, run the agent normally, then use unread shortcuts and cmux notifications from hooks, OSC, or cmux notify to review sessions that need you. Claude Code, Codex, and OpenCode stay the agents; cmux handles the supervision surface.

| Approach | Isolation | Visibility | Best use |
| --- | --- | --- | --- |
| **One terminal tab per task** | Manual | Weak | Small experiments |
| **tmux panes** | Manual | Medium | Remote shell workflows |
| **Manual Git worktrees** | Strong | Manual | Careful parallel work |
| **cmux workspaces** | Works with your branch, worktree, checkout, or repo setup | Strong | Many visible agent sessions on macOS |
| **cmux claude-teams** | Native pane mapping for teammate sessions | Strong | Claude Code agent teams with native cmux splits instead of tmux/iTerm2 panes |

## Give every task a visible home

The main failure mode of parallel agents is losing track of which terminal owns which task. cmux workspaces keep the branch, directory, ports, unread state, and latest output visible in the sidebar.

## Use notification rings as the review queue

When an agent emits a cmux notification through hooks, OSC 99/777, or cmux notify, cmux marks the workspace unread and rings the pane. Cmd+Shift+U jumps to the latest unread item. Cmd+Control+U marks the current item as oldest unread and jumps to the next latest unread, which is useful when several agents finish at once.

## Keep each agent unchanged

cmux does not replace Claude Code, Codex, or OpenCode. You run the same CLIs with your existing accounts and settings, while cmux adds workspaces, notifications, and socket automation around the terminal session.

## Use teammate mode when it fits

For Claude Code teammate mode, cmux can translate tmux-style teammate panes into native cmux splits, so spawned teammates stay visible with sidebar metadata and notifications.

## FAQ

### Does cmux create Git worktrees automatically?

Not by default. cmux can be scripted to create workspaces around your branch, worktree, multi-repo, no-git, code-to-main, or remote-dev flow, so you can keep the repository setup that fits your project.

### Can Claude Code and Codex run next to each other?

Yes. cmux is agent-agnostic, so Claude Code, Codex, OpenCode, and other CLI agents can run in separate workspaces at the same time.

### What shortcut matters most for parallel agents?

Cmd+Shift+U is the core supervision shortcut because it jumps to the latest unread item. Cmd+Control+U is useful when you want to mark the current item as oldest unread and jump to the next latest unread item.

cmux is free and open source for macOS.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

See also

-   [Compare cmux](https://cmux.com/compare)
-   [Best terminals and agent workspaces for AI coding agents in 2026](https://cmux.com/compare/best-terminal-for-ai-coding-agents)
-   [cmux vs Zed](https://cmux.com/compare/cmux-vs-zed)
-   [A terminal for coding agents](https://cmux.com/agents)
-   [Keyboard shortcuts](https://cmux.com/docs/keyboard-shortcuts)
-   [Browser automation](https://cmux.com/docs/browser-automation)

Canonical: https://cmux.com/compare/multiple-claude-code-agents-parallel
