# [#](#title)Task Manager

# [#](https://cmux-docs-release.vercel.app/docs/task-manager#title)Task Manager

Task Manager shows resource usage for cmux windows, workspaces, panes, terminal processes, coding agents, and browser webviews.

## [#](https://cmux-docs-release.vercel.app/docs/task-manager#open)Open Task Manager

Run the CLI command from any cmux terminal:

```
cmux top
```

You can also press Cmd+Shift+P and choose Task Manager from the command palette.

## [#](https://cmux-docs-release.vercel.app/docs/task-manager#what-it-shows)What it shows

-   Windows, workspaces, panes, and surfaces
-   Workspace ownership for terminal and browser activity
-   Known coding agent processes such as Claude Code, Codex, and OpenCode
-   Browser webviews and related helper processes

## [#](https://cmux-docs-release.vercel.app/docs/task-manager#workflow)Troubleshooting workflow

1.  Open Task Manager when cmux feels slow, fans spin up, or an agent appears stuck.
2.  Sort or scan for the process using the most CPU or memory.
3.  Jump back to the owning workspace or surface.
4.  Stop, restart, or split the overloaded process with the right context visible.

## [#](https://cmux-docs-release.vercel.app/docs/task-manager#when-to-use)When to use it

Use Task Manager when Activity Monitor shows load from cmux but does not identify the responsible workspace, agent, or browser pane.

Read the [Task Manager feature story](https://cmux-docs-release.vercel.app/blog/task-manager) for the workflow and launch context.

[Vault](https://cmux-docs-release.vercel.app/docs/vault) [Custom Commands](https://cmux-docs-release.vercel.app/docs/custom-commands)

Canonical: https://cmux-docs-release.vercel.app/docs/task-manager
