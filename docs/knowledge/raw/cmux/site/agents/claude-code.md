# A terminal for Claude Code

cmux is a native macOS terminal built for running AI coding agents, and Claude Code is a first-class fit. cmux is just a terminal, so `claude` runs in any workspace out of the box, and the things that make running agents painful, keeping track of many at once and noticing when they need you, are what cmux is built for.

## Run many Claude Code sessions, organized

Open a workspace per task and run Claude Code in each. The vertical sidebar shows every workspace with its git branch, directory, ports, and the latest line of Claude's output, so a dozen parallel sessions stay legible instead of buried in tabs.

## Notification rings when Claude needs you

When Claude Code finishes or asks for input, the pane rings and the sidebar shows an unread badge, so you can let several agents run and come back to the one that needs a decision. Notifications fire automatically, and you can also trigger them from Claude Code hooks.

## Claude Code teams as native panes

cmux runs Claude Code's teammate mode with one command, and the teammates spawn as native cmux splits with their own sidebar metadata and notifications, no tmux required. See the [Claude Code teams docs](https://cmux.com/docs/agent-integrations/claude-code-teams).

## Check on Claude from your phone

cmux has an iOS companion app (beta): pair your iPhone with your Mac and check on your Claude Code sessions, with optional notification forwarding, while you are away from the desk.

## Scriptable

Every action is available through the cmux CLI and a Unix socket: create a workspace, launch Claude Code in it, send input, read the screen, and drive an in-app browser to verify changes, all from a script.

## FAQ

Does Claude Code work in cmux?

Yes. cmux is a standard macOS terminal, so `claude` runs in any workspace with no extra setup.

Can I run Claude Code teams in cmux?

Yes. cmux runs Claude Code's teammate mode with one command and teammates spawn as native cmux splits with their own sidebar metadata and notifications, no tmux required.

How do I know when Claude needs input?

The pane rings and the sidebar shows an unread badge when Claude Code finishes or asks for input, so you can let several agents run and come back to the one that needs you.

Is cmux free to use with Claude Code?

Yes. cmux is free and open source for macOS.

cmux is free and open source for macOS.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

See also

-   [A terminal for coding agents](https://cmux.com/agents)
-   [A terminal for Codex CLI](https://cmux.com/agents/codex)
-   [A terminal for OpenCode](https://cmux.com/agents/opencode)
-   [Claude Code teams](https://cmux.com/docs/agent-integrations/claude-code-teams)

Canonical: https://cmux.com/agents/claude-code
