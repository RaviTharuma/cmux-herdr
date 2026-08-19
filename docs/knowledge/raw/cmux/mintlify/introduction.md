> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Introduction

> A Ghostty-based macOS terminal with vertical tabs and notifications for AI coding agents

<img className="block dark:hidden" src="https://mintlify.s3.us-west-1.amazonaws.com/manaflow-ai-cmux/images/hero-light.svg" alt="cmux hero" />

<img className="hidden dark:block" src="https://mintlify.s3.us-west-1.amazonaws.com/manaflow-ai-cmux/images/hero-dark.svg" alt="cmux hero" />

## What is cmux?

cmux is a native macOS terminal built with Swift and AppKit that brings vertical tabs, in-app browser, and a powerful notification system designed specifically for working with AI coding agents like Claude Code and OpenCode.

Built on top of [Ghostty](https://github.com/ghostty-org/ghostty) for GPU-accelerated terminal rendering, cmux reads your existing `~/.config/ghostty/config` for themes, fonts, and colors, so you can keep using your favorite terminal configuration.

## Key features

<CardGroup cols={2}>
  <Card title="Notification rings" icon="bell" href="/features/notifications">
    Panes get a blue ring and tabs light up when coding agents need your attention. No more missing important prompts.
  </Card>

  <Card title="Notification panel" icon="list" href="/features/notifications">
    See all pending notifications in one place, jump to the most recent unread with Cmd+Shift+U.
  </Card>

  <Card title="In-app browser" icon="browser" href="/features/browser">
    Split a browser alongside your terminal with a scriptable API ported from agent-browser for automation.
  </Card>

  <Card title="Vertical tabs" icon="sidebar" href="/features/workspaces-and-tabs">
    Sidebar shows git branch, linked PR status, working directory, listening ports, and latest notification text.
  </Card>

  <Card title="Scriptable CLI" icon="terminal" href="/automation/cli-reference">
    CLI and socket API to create workspaces, split panes, send keystrokes, and automate the browser.
  </Card>

  <Card title="Native performance" icon="bolt" href="/quickstart">
    Built with Swift and AppKit, not Electron. Fast startup, low memory usage, GPU-accelerated rendering.
  </Card>
</CardGroup>

## Why cmux?

Running multiple AI coding sessions in parallel is challenging. Native macOS notifications lack context, and with many tabs open, you can't tell which agent needs attention. cmux solves this with:

* **Visual notification system** - Blue rings on panes and glowing tabs show exactly which agent is waiting
* **Notification panel** - All notifications in one place with full context (title, subtitle, body)
* **Sidebar metadata** - Git branch, PR status, working directory, ports, and notification text at a glance
* **Scriptable automation** - Wire `cmux notify` into agent hooks for Claude Code, OpenCode, and custom tools

The in-app browser lets agents snapshot the accessibility tree, get element refs, click, fill forms, and evaluate JS. Split a browser pane next to your terminal and have your coding agent interact with your dev server directly.

## The Zen of cmux

cmux is not prescriptive about how developers hold their tools. It's a terminal and browser with a CLI, and the rest is up to you.

cmux is a **primitive, not a solution**. It gives you a terminal, a browser, notifications, workspaces, splits, tabs, and a CLI to control all of it. cmux doesn't force you into an opinionated way to use coding agents. What you build with the primitives is yours.

The best developers have always built their own tools. Nobody has figured out the best way to work with agents yet. Give a million developers composable primitives and they'll collectively find the most efficient workflows faster than any product team could design top-down.

## Get started

<CardGroup cols={2}>
  <Card title="Installation" icon="download" href="/installation">
    Install cmux via DMG or Homebrew
  </Card>

  <Card title="Quickstart" icon="rocket" href="/quickstart">
    Create your first workspace and set up notifications
  </Card>

  <Card title="CLI Reference" icon="code" href="/automation/cli-reference">
    Explore CLI commands and socket API
  </Card>

  <Card title="Community" icon="discord" href="https://discord.gg/xsgFEVrWCZ">
    Join the Discord community
  </Card>
</CardGroup>
