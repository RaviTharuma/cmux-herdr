# cmux is built on Ghostty

cmux is not a fork of Ghostty. It embeds [libghostty](https://github.com/ghostty-org/ghostty), the library at the core of the Ghostty terminal, for GPU-accelerated rendering, the same way an app uses WebKit for web views. Ghostty is a standalone terminal; cmux is a different application built on top of its rendering engine.

## What cmux adds on top

libghostty gives cmux a fast, accurate terminal. cmux builds an application around it for multitasking, organization, and programmability:

-   Workspaces in a vertical sidebar, each showing its git branch, working directory, ports, and the latest line of agent output.
-   Notification rings when a pane needs your attention, plus an iOS companion app to check on your terminals from your phone.
-   Vertical tabs and split panes that scale to dozens of sessions.
-   A CLI and Unix socket API to script workspaces, panes, input, and an in-app browser.

## Why libghostty

Reusing libghostty means cmux inherits Ghostty's rendering quality and performance instead of reimplementing a terminal, and stays focused on the workspace, organization, and automation layer that sits above the terminal grid. Your existing `~/.config/ghostty/config` for themes, fonts, and colors is read directly.

cmux is free and open source for macOS.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

See also

-   [Best terminal for Mac](https://cmux.com/best-terminal-for-mac)
-   [Configuration (Ghostty config)](https://cmux.com/docs/configuration)
-   [Get started with cmux](https://cmux.com/docs/getting-started)

Canonical: https://cmux.com/built-on-ghostty
