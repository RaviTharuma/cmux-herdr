# oh-my-openagent subagents as native cmux panes

[← Back to blog](https://cmux.com/blog)


[![](https://cmux.com/_next/image?url=%2Favatars%2Flawrencecchen.jpg&w=64&q=75&dpl=dpl_AT5HfVg9fuRYjj7rCgqhY9VxGYH8)Lawrence Chen@lawrencecchen](https://x.com/lawrencecchen)March 30, 2026

`cmux omo` integrates oh-my-openagent (formerly oh-my-opencode), a plugin for OpenCode that orchestrates specialist agents across Claude, GPT, and Gemini in parallel. Each agent gets its own tmux pane. `cmux omo` uses the same tmux shim as [`cmux claude-teams`](https://cmux.com/docs/agent-integrations/claude-code-teams): fake `TMUX` env var, every tmux command translated to cmux splits. It auto-installs the plugin into a shadow config so `~/.config/opencode` stays untouched.

The shim also intercepts `terminal-notifier` calls and routes them through `cmux notify`. Works over SSH via the Go relay daemon.

[Read the docs →](https://cmux.com/docs/agent-integrations/oh-my-opencode)

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

[Claude Code teammate agents as native cmux panes](https://cmux.com/blog/cmux-claude-teams) [cmux is now GPL](https://cmux.com/blog/gpl)

Canonical: https://cmux.com/blog/cmux-omo
