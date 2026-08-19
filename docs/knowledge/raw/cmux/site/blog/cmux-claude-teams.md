# Claude Code teammate agents as native cmux panes

[← Back to blog](https://cmux.com/blog)


[![](https://cmux.com/_next/image?url=%2Favatars%2Flawrencecchen.jpg&w=64&q=75&dpl=dpl_AT5HfVg9fuRYjj7rCgqhY9VxGYH8)Lawrence Chen@lawrencecchen](https://x.com/lawrencecchen)March 30, 2026

Claude Code has an experimental [teammate mode](https://code.claude.com/docs/en/agent-teams) that spawns sub-agents in parallel. It requires tmux and setting `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`. `cmux claude-teams` handles all of that for you: it sets the env var, places a tmux shim on PATH that translates every tmux command (`split-window`, `send-keys`, `capture-pane`) into cmux's native split/workspace API, and execs into `claude --teammate-mode auto`. One command.

The shim translates `split-window` into `surface.split`, `send-keys` into `surface.send_text`, `capture-pane` into `surface.read_text`. Teammates stack vertically in a right column, auto-equalizing as agents spawn and exit. Every pane shows up in the sidebar with notifications. Works over SSH via the Go relay daemon.

The same shim powers [`cmux omo`](https://cmux.com/docs/agent-integrations/oh-my-opencode) for oh-my-openagent (formerly oh-my-opencode).

[Read the docs →](https://cmux.com/docs/agent-integrations/claude-code-teams)

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

[cmux SSH](https://cmux.com/blog/cmux-ssh) [oh-my-openagent subagents as native cmux panes](https://cmux.com/blog/cmux-omo)

Canonical: https://cmux.com/blog/cmux-claude-teams
