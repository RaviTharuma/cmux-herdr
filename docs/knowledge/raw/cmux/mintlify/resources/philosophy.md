> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Philosophy

> The design philosophy and principles behind cmux

cmux is built on a core belief: the best developers build their own tools. Rather than forcing you into a prescriptive workflow, cmux provides composable primitives that you can assemble in whatever way works best for you.

## The Zen of cmux

cmux is not prescriptive about how developers hold their tools. It's a terminal and browser with a CLI, and the rest is up to you.

cmux is a primitive, not a solution. It gives you a terminal, a browser, notifications, workspaces, splits, tabs, and a CLI to control all of it. cmux doesn't force you into an opinionated way to use coding agents. What you build with the primitives is yours.

The best developers have always built their own tools. Nobody has figured out the best way to work with agents yet, and the teams building closed products definitely haven't either. The developers closest to their own codebases will figure it out first.

Give a million developers composable primitives and they'll collectively find the most efficient workflows faster than any product team could design top-down.

## Core principles

### Primitives over prescriptions

cmux provides building blocks rather than opinionated workflows. You get a terminal, browser, notification system, workspaces, splits, and tabs. How you combine these primitives is entirely up to you.

### Scriptable by default

Everything in cmux is scriptable through the CLI and socket API. Create workspaces and tabs, split panes, send keystrokes, and control the browser programmatically. This enables you to automate your workflow exactly how you need it.

### Native performance

Built with Swift and AppKit, not Electron. cmux is a native macOS app that delivers fast startup times and low memory usage. GPU-accelerated terminal rendering powered by libghostty ensures smooth performance.

### Compatibility first

cmux reads your existing `~/.config/ghostty/config` for themes, fonts, and colors. It's designed to integrate with your existing workflow rather than replace it.

<Note>
  The philosophy behind cmux reflects a broader truth: developers working directly with their own codebases, equipped with composable primitives, will discover the most efficient workflows faster than any centralized product team.
</Note>
