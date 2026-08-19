# Task Manager in cmux

[← Back to blog](https://cmux.com/blog)


[![](https://cmux.com/_next/image?url=%2Favatars%2Flawrencecchen.jpg&w=64&q=75&dpl=dpl_AT5HfVg9fuRYjj7rCgqhY9VxGYH8)Lawrence Chen@lawrencecchen](https://x.com/lawrencecchen)May 22, 2026

cmux now has a task manager for checking how much CPU and RAM your coding agents and terminal processes are using.

Open it from the CLI:

```
cmux top
```

Or press Cmd+Shift+P and choose Task Manager from the command palette.

Requires cmux v0.64.4+.

## A workflow for noisy agent runs

1.  Open Task Manager when fans spin up, a workspace gets sluggish, or an agent appears stuck.
2.  Scan the live process list for the workspace, pane, surface, agent, or browser webview using the most CPU or memory.
3.  Jump from the manager back to the matching surface so you can inspect the actual process context.
4.  Stop, restart, or split the problematic task with the same information the operating system would show, scoped to cmux.

## When it helps

Task Manager is most useful when several Claude Code, Codex, OpenCode, or browser panes are running at once and Activity Monitor cannot tell you which workspace owns the load.

## FAQ

### Does it identify coding agents?

Yes. cmux attributes known agent processes to the workspace and surface where they are running so the resource view matches your sidebar layout.

### Can I use it without opening the window?

Yes. Run cmux top for a terminal snapshot when you want the same information in scripts, SSH sessions, or a plain shell.

Read the [Task Manager docs](https://cmux.com/docs/task-manager) for the command, window entrypoint, and recommended troubleshooting flow.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

[Passkey auth in the cmux browser](https://cmux.com/blog/passkey-auth) [A better markdown viewer in cmux](https://cmux.com/blog/markdown-viewer)

Canonical: https://cmux.com/blog/task-manager
