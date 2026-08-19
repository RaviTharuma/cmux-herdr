# cmux history

[← Back to blog](https://cmux.com/blog)


[![](https://cmux.com/_next/image?url=%2Favatars%2Flawrencecchen.jpg&w=64&q=75&dpl=dpl_AT5HfVg9fuRYjj7rCgqhY9VxGYH8)Lawrence Chen@lawrencecchen](https://x.com/lawrencecchen)June 2, 2026

You close a terminal tab to clear clutter, then realize you needed it. Or you close a whole workspace and lose the agent that was mid task. cmux now keeps a history of what you closed and what you focused, so getting it back takes one shortcut.

## Reopen what you closed

Press `Cmd+Shift+T` to reopen the last thing you closed. It works for terminal tabs, browser tabs, workspaces, and windows, and restores them one at a time in the order you closed them. Each surface comes back where it was, with its panes intact.

## Agent sessions come back too

History also reopens closed agent sessions. Reopen a Claude Code, Codex, or OpenCode session and it resumes where it left off. Run `cmux hooks setup` once so cmux can track agent sessions, then they reopen like everything else.

```
cmux hooks setup
```

This builds on cmux [session restore](https://cmux.com/blog/session-restore), which resumes supported agents through their own native resume commands.

## Jump back to where you were

The back and forward buttons in the titlebar, or `Cmd+[` and `Cmd+]`, jump to the previous and next workspace or window you were working in, the same way back and forward work in a browser.

## The full history pane

When you need more than the last item, open the full History pane: a searchable, day grouped view of everything you closed and focused, with timestamps and a Clear Closed action.

See the [keyboard shortcuts](https://cmux.com/docs/keyboard-shortcuts) for the full list, including how to rebind history navigation.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

[cmux home](https://cmux.com/blog/cmux-home) [cmux Finder](https://cmux.com/blog/cmux-finder)

Canonical: https://cmux.com/blog/cmux-history
