# cmux Vault

[← Back to blog](https://cmux.com/blog)


[![](https://cmux.com/_next/image?url=%2Favatars%2Flawrencecchen.jpg&w=64&q=75&dpl=dpl_AT5HfVg9fuRYjj7rCgqhY9VxGYH8)Lawrence Chen@lawrencecchen](https://x.com/lawrencecchen)May 22, 2026

cmux now has a Vault pane in the right sidebar for old agent sessions.

It indexes Codex, Claude Code, OpenCode, and Pi sessions, with full-text search across transcripts.

Drag a session from Vault into a workspace to reopen it where you are working.

Requires cmux v0.64+.

## A workflow for finding old agent work

1.  Open Vault from the right sidebar when you remember the task but not the workspace or date.
2.  Search for a filename, issue title, error message, branch name, or phrase from the agent transcript.
3.  Drag the matching session into the active workspace so it reopens beside the code you are touching now.
4.  Continue from the recovered context instead of asking a fresh agent to rediscover the same state.

## When Vault beats terminal history

Shell history tells you what command started an agent. Vault searches what the agent actually discussed, changed, and reported, which is closer to how developers remember old work.

## FAQ

### Which sessions are indexed?

Vault indexes supported Codex, Claude Code, OpenCode, and Pi sessions when cmux can read their local session records.

### Is Vault the same as session restore?

No. Session restore brings back the current app layout after relaunch. Vault is for searching older agent sessions and reopening one on demand.

Read the [Vault docs](https://cmux.com/docs/vault) for indexed agents, search workflow, and restore limits.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

[cmux Finder](https://cmux.com/blog/cmux-finder) [Passkey auth in the cmux browser](https://cmux.com/blog/passkey-auth)

Canonical: https://cmux.com/blog/cmux-vault
