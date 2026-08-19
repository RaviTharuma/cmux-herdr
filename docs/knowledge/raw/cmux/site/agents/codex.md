# A terminal for Codex CLI

cmux is a native macOS terminal built for AI coding agents, and the OpenAI Codex CLI runs in it out of the box. cmux is just a terminal, so `codex` works in any workspace, with cmux adding the multitasking, organization, and programmability around it.

## Organize many Codex sessions

Run Codex in its own workspace per task. The vertical sidebar shows each one with its git branch, directory, ports, and latest output, so several parallel Codex runs stay organized instead of lost in tabs.

## Notification rings when Codex needs you

When Codex finishes or asks for input, the pane rings and the sidebar flags it unread, so you can run several at once and return to the one that needs attention. Notifications fire automatically and can also be driven from agent hooks.

## oh-my-codex

cmux ships an `oh-my-codex` integration that runs Codex in a cmux-aware environment so its activity surfaces as native cmux panes. See the [oh-my-codex docs](https://cmux.com/docs/agent-integrations/oh-my-codex).

## Check on Codex from your phone

cmux has an iOS companion app (beta): pair your iPhone with your Mac to check on your Codex runs, with optional notification forwarding, while you are away from the desk.

## Scriptable

Everything is available through the cmux CLI and a Unix socket: create a workspace, launch Codex, send input, read the screen, take screenshots, and drive an in-app browser, all from a script.

## FAQ

Does Codex CLI work in cmux?

Yes. cmux is a standard macOS terminal, so `codex` runs in any workspace with no extra setup.

Can I run Codex next to other agents?

Yes. Open a workspace per task and run Codex, Claude Code, or OpenCode side by side. The sidebar keeps every session legible.

How do I know when Codex needs input?

The pane rings and the sidebar shows an unread badge when Codex finishes or asks for input, so you can let it run and come back when it needs you.

Is cmux free to use with Codex?

Yes. cmux is free and open source for macOS.

cmux is free and open source for macOS.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

See also

-   [A terminal for coding agents](https://cmux.com/agents)
-   [A terminal for Claude Code](https://cmux.com/agents/claude-code)
-   [A terminal for OpenCode](https://cmux.com/agents/opencode)
-   [oh-my-codex](https://cmux.com/docs/agent-integrations/oh-my-codex)

Canonical: https://cmux.com/agents/codex
