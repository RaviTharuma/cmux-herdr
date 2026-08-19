# Session restore in cmux

[← Back to blog](https://cmux.com/blog)


[![](https://cmux.com/_next/image?url=%2Favatars%2Flawrencecchen.jpg&w=64&q=75&dpl=dpl_AT5HfVg9fuRYjj7rCgqhY9VxGYH8)Lawrence Chen@lawrencecchen](https://x.com/lawrencecchen)May 13, 2026

Terminal workflows survive interruptions better when the app can reconstruct the shape of your work. cmux now treats the workspace layout as durable state instead of something tied to one app process.

The important boundary is live process state. cmux restores what it owns and what supported tools expose through their own resume APIs. It does not checkpoint arbitrary terminal processes.

If you are looking for how to restore Claude Code, OpenCode, opencode, Codex, Gemini CLI, Antigravity CLI, Grok Build CLI, Amp, Cursor CLI, Rovo Dev, Copilot, CodeBuddy, Factory, Qoder, or Hermes Agent sessions after a terminal crash, install cmux hooks and keep agent resume enabled.

## What always comes back

After a normal relaunch, cmux restores the app-level session snapshot:

-   Window, workspace, and pane layout
-   Working directories
-   Terminal scrollback, best effort
-   Browser URL and navigation history

## Agent sessions need hooks

Claude Code, Codex, Grok Build CLI, OpenCode, Pi, Amp, Cursor CLI, Gemini CLI, Antigravity CLI, Rovo Dev, Hermes Agent, Copilot, CodeBuddy, Factory, and Qoder can resume when cmux has a native session ID. For most agents, install the integration with `cmux hooks setup`.

```
cmux hooks setup
```

The setup command installs supported agents whose binaries are on PATH and skips the rest. Claude Code is handled by the cmux Claude wrapper when Claude integration is enabled in Settings.

## How it works

cmux writes a JSON session snapshot under Application Support with the window tree, workspace metadata, pane layout, terminal cwd, scrollback replay data, and browser navigation state.

Agent hooks write session mappings under ~/.cmuxterm. On restore, cmux rebuilds the UI first. If automatic agent resume is enabled, it launches each supported agent with that agent's native resume command and the saved session ID.

## What stays out of scope

tmux, vim, shells, and unsupported tools reopen as normal terminals unless they have a cmux integration that records a safe native resume command. That keeps restore predictable and avoids replaying stale prompts or secrets.

## Recommended setup

1.  Install the agent CLI you use, then run cmux hooks setup so cmux can capture native session IDs.
2.  Run agents in normal cmux terminals. The hooks record the session ID, cwd, workspace, and surface as work happens.
3.  After a relaunch, cmux rebuilds the window and pane tree before starting any saved agent resume commands.
4.  Check that the restored agent is continuing the same upstream session, then keep working in the same workspace.

## FAQ

### Does this recover after a crash?

It recovers app-owned layout and supported agent sessions from the last saved snapshot. It cannot recover arbitrary process memory that the agent or shell never exposed through a resume API.

### Should I still use tmux?

Use tmux when you need tmux semantics on a remote host. Use cmux session restore when you want app layout, browser state, and supported agent resume to come back together.

Read the [session restore docs](https://cmux.com/docs/session-restore) for setup commands, supported agents, and troubleshooting.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

[Unread workspace shortcuts in cmux](https://cmux.com/blog/unread-shortcuts) [cmux SSH](https://cmux.com/blog/cmux-ssh)

Canonical: https://cmux.com/blog/session-restore
