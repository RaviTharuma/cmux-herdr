# A terminal for Amp

cmux is a native macOS terminal built for running AI coding agents, and Sourcegraph's Amp is a first-class fit. cmux is just a terminal, so `amp` runs in any workspace out of the box, and the things that make running agents painful, keeping track of many at once and noticing when they need you, are what cmux is built for.

## Run many Amp sessions, organized

Open a workspace per task and run Amp in each. The vertical sidebar shows every workspace with its git branch, directory, ports, and the latest line of Amp's output, so a dozen parallel sessions stay legible instead of buried in tabs.

## Notification rings when Amp needs you

When Amp finishes or asks for input, the pane rings and the sidebar shows an unread badge, so you can let several agents run and come back to the one that needs a decision. Notifications fire automatically, and you can also trigger them from shell hooks.

## Check on Amp from your phone

cmux has an iOS companion app (beta): pair your iPhone with your Mac and check on your Amp sessions, with optional notification forwarding, while you are away from the desk.

## Scriptable

Every action is available through the cmux CLI and a Unix socket: create a workspace, launch Amp in it, send input, read the screen, and drive an in-app browser to verify changes, all from a script.

## FAQ

Does Amp work in cmux?

Yes. cmux is a standard macOS terminal, so `amp` runs in any workspace with no extra setup.

Can I run Amp next to other agents?

Yes. Open a workspace per task and run Amp, Claude Code, or Codex side by side. The sidebar keeps every session legible.

How do I know when Amp needs input?

The pane rings and the sidebar shows an unread badge when Amp finishes or asks for input, so you can let it run and come back when it needs you.

Is cmux free to use with Amp?

Yes. cmux is free and open source for macOS.

cmux is free and open source for macOS.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

See also

-   [A terminal for coding agents](https://cmux.com/agents)
-   [A terminal for Claude Code](https://cmux.com/agents/claude-code)
-   [A terminal for Codex CLI](https://cmux.com/agents/codex)
-   [Notifications](https://cmux.com/docs/notifications)

Canonical: https://cmux.com/agents/amp
