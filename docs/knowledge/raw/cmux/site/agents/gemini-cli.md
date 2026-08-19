# A terminal for Gemini CLI

cmux is a native macOS terminal built for running AI coding agents, and Google's Gemini CLI is a first-class fit. cmux is just a terminal, so `gemini` runs in any workspace out of the box, and the things that make running agents painful, keeping track of many at once and noticing when they need you, are what cmux is built for.

## Run many Gemini CLI sessions, organized

Open a workspace per task and run Gemini CLI in each. The vertical sidebar shows every workspace with its git branch, directory, ports, and the latest line of Gemini's output, so a dozen parallel sessions stay legible instead of buried in tabs.

## Notification rings when Gemini needs you

When Gemini CLI finishes or asks for input, the pane rings and the sidebar shows an unread badge, so you can let several agents run and come back to the one that needs a decision. Notifications fire automatically, and you can also trigger them from shell hooks.

## Check on Gemini from your phone

cmux has an iOS companion app (beta): pair your iPhone with your Mac and check on your Gemini CLI sessions, with optional notification forwarding, while you are away from the desk.

## Scriptable

Every action is available through the cmux CLI and a Unix socket: create a workspace, launch Gemini CLI in it, send input, read the screen, and drive an in-app browser to verify changes, all from a script.

## FAQ

Does Gemini CLI work in cmux?

Yes. cmux is a standard macOS terminal, so `gemini` runs in any workspace with no extra setup.

Can I run Gemini CLI next to other agents?

Yes. Open a workspace per task and run Gemini CLI, Claude Code, or Codex side by side. The sidebar keeps every session legible.

How do I know when Gemini needs input?

The pane rings and the sidebar shows an unread badge when Gemini CLI finishes or asks for input, so you can let it run and come back when it needs you.

Is cmux free to use with Gemini CLI?

Yes. cmux is free and open source for macOS.

cmux is free and open source for macOS.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

See also

-   [A terminal for coding agents](https://cmux.com/agents)
-   [A terminal for Claude Code](https://cmux.com/agents/claude-code)
-   [A terminal for Codex CLI](https://cmux.com/agents/codex)
-   [Notifications](https://cmux.com/docs/notifications)

Canonical: https://cmux.com/agents/gemini-cli
