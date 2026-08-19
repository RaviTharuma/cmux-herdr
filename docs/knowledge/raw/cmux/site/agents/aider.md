# A terminal for Aider

cmux is a native macOS terminal built for running AI coding agents, and Aider is a first-class fit. cmux is just a terminal, so `aider` runs in any workspace out of the box, and the things that make running agents painful, keeping track of many at once and noticing when they need you, are what cmux is built for.

## Run many Aider sessions, organized

Open a workspace per task and run Aider in each. The vertical sidebar shows every workspace with its git branch, directory, ports, and the latest line of output, so a dozen parallel sessions stay legible instead of buried in tabs. Aider commits as it works, and each workspace shows the branch it is on.

## Notification rings when Aider needs you

When Aider finishes a change or asks for confirmation, the pane rings and the sidebar shows an unread badge, so you can let several agents run and come back to the one that needs a decision. Notifications fire automatically, and you can also trigger them from shell hooks.

## Check on Aider from your phone

cmux has an iOS companion app (beta): pair your iPhone with your Mac and check on your Aider sessions, with optional notification forwarding, while you are away from the desk.

## Scriptable

Every action is available through the cmux CLI and a Unix socket: create a workspace, launch Aider in it, send input, read the screen, and drive an in-app browser to verify changes, all from a script.

## FAQ

Does Aider work in cmux?

Yes. cmux is a standard macOS terminal, so `aider` runs in any workspace with no extra setup.

Can I run Aider on several branches at once?

Yes. Open a workspace per branch and run Aider in each. The sidebar shows the git branch and directory for every workspace so parallel work stays clear.

How do I know when Aider needs confirmation?

The pane rings and the sidebar shows an unread badge when Aider finishes or asks for input, so you can let it run and come back when it needs you.

Is cmux free to use with Aider?

Yes. cmux is free and open source for macOS.

cmux is free and open source for macOS.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

See also

-   [A terminal for coding agents](https://cmux.com/agents)
-   [A terminal for Claude Code](https://cmux.com/agents/claude-code)
-   [A terminal for Codex CLI](https://cmux.com/agents/codex)
-   [Get started with cmux](https://cmux.com/docs/getting-started)

Canonical: https://cmux.com/agents/aider
