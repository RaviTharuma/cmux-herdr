# [#](#title)Vault

# [#](https://cmux-docs-release.vercel.app/docs/vault#title)Vault

Vault is a right-sidebar pane for finding old AI coding agent sessions by transcript content instead of by terminal history.

## [#](https://cmux-docs-release.vercel.app/docs/vault#what-it-indexes)What Vault indexes

Vault reads local session records from supported agents and makes their transcripts searchable inside cmux.

-   Codex sessions
-   Claude Code sessions
-   OpenCode sessions
-   Pi sessions

## [#](https://cmux-docs-release.vercel.app/docs/vault#workflow)Workflow

1.  Open Vault from the right sidebar.
2.  Search for a file, branch, issue title, error message, or phrase from the conversation.
3.  Drag a matching session into the current workspace.
4.  Resume from the recovered context beside the code or browser pane you are using now.

## [#](https://cmux-docs-release.vercel.app/docs/vault#when-to-use)When to use Vault

Use Vault when you remember what an agent worked on but not which workspace, date, or shell command started it. It is built for rediscovering prior agent context, not for restoring the current app layout after relaunch.

## [#](https://cmux-docs-release.vercel.app/docs/vault#limits)Limits

Vault can only index session formats that cmux knows how to read on the local machine. It does not search private remote hosts unless those sessions are available to the local cmux installation.

Read the [Vault feature story](https://cmux-docs-release.vercel.app/blog/cmux-vault) for a workflow-oriented overview.

[Session Restore](https://cmux-docs-release.vercel.app/docs/session-restore) [Task Manager](https://cmux-docs-release.vercel.app/docs/task-manager)

Canonical: https://cmux-docs-release.vercel.app/docs/vault
