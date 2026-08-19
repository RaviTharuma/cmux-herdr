# cmux SSH

[← Back to blog](https://cmux.com/blog)


[![](https://cmux.com/_next/image?url=%2Favatars%2Flawrencecchen.jpg&w=64&q=75&dpl=dpl_AT5HfVg9fuRYjj7rCgqhY9VxGYH8)Lawrence Chen@lawrencecchen](https://x.com/lawrencecchen)March 30, 2026

`cmux ssh user@remote` creates a workspace for the remote machine. Drag an image into a remote Claude Code session and it gets uploaded automatically. Browser panes route through the remote network, so localhost just works. Uses your `~/.ssh/config`, reconnects on drops.

## A remote development workflow

1.  Connect with cmux ssh and let cmux create a dedicated workspace for the remote host.
2.  Open a browser pane next to the terminal and use localhost URLs as if the dev server were local.
3.  Run Claude Code, Codex, OpenCode, or another agent on the remote machine and let cmux route notifications back to your Mac.
4.  Drag files or images into the remote terminal when the agent needs local context or test fixtures.

-   Browser panes route through the remote machine, so `localhost:3000` reaches the remote dev server without port forwarding
-   Drag an image into a remote terminal to upload via scp
-   Coding agents on the remote box send notifications to your local sidebar
-   `cmux claude-teams` and `cmux omo` work over SSH, spawning teammate panes locally while computation runs remote
-   The sidebar shows connection state and detected listening ports

## FAQ

### Do I still need ssh -L for localhost?

Usually no. Browser panes inside the remote workspace route HTTP and WebSocket traffic through the remote machine, so local preview URLs resolve from the remote host.

### Does it use my existing SSH config?

Yes. cmux reads ~/.ssh/config for aliases, identity files, proxy settings, and host options, then adds cmux-specific routing and reconnect behavior.

[Read the SSH docs →](https://cmux.com/docs/ssh)

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

[Session restore in cmux](https://cmux.com/blog/session-restore) [Claude Code teammate agents as native cmux panes](https://cmux.com/blog/cmux-claude-teams)

Canonical: https://cmux.com/blog/cmux-ssh
