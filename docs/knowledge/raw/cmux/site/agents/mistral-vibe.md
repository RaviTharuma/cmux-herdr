# Mistral Vibe: A terminal for coding agents

cmux is a native macOS terminal built on Ghostty for running AI coding agents. It is just a terminal, so any agent CLI runs out of the box, and the things that make running agents painful, keeping track of many at once and noticing when they need you, are what cmux is built for.

## Run many agents, organized

Open a workspace per task and run any agent in each. The vertical sidebar shows every workspace with its git branch, directory, ports, and the latest line of output, so a dozen parallel agents stay legible instead of buried in tabs.

## Notification rings when an agent needs you

When an agent finishes or asks for input, the pane rings and the sidebar shows an unread badge, so you can let several agents run and come back to the one that needs a decision. Notifications fire automatically and you can also trigger them from agent hooks.

## Scriptable

Every action is available through the cmux CLI and a Unix socket: create a workspace, launch an agent in it, send input, read the screen, and drive an in-app browser to verify changes, all from a script.

## FAQ

Which coding agents work in cmux?

cmux is a standard macOS terminal, so any CLI agent works: Claude Code, Codex, OpenCode, Gemini CLI, Aider, Amp, Cursor, and anything else you run from a shell.

Do I have to configure each agent?

No. Agents run exactly as they do in any terminal. cmux adds workspaces, notification rings, and a scriptable socket on top without changing how the agent itself runs.

Can I run several agents at once?

Yes. Open a workspace per task, run a different agent in each, and the sidebar keeps every session legible with its branch, directory, and latest output line.

Is cmux free?

Yes. cmux is free and open source for macOS.

cmux is free and open source for macOS.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

See also

-   [A terminal for coding agents](https://cmux.com/agents)
-   [A terminal for Claude Code](https://cmux.com/agents/claude-code)
-   [A terminal for Codex CLI](https://cmux.com/agents/codex)
-   [Pi](https://cmux.com/agents/pi)

Canonical: https://cmux.com/agents/mistral-vibe
