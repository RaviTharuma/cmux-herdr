# [#](#title)Dock

# [#](https://cmux-docs-release.vercel.app/docs/dock#title)Dock

Dock lets you add terminal UI controls to the right sidebar. Each control is a command from JSON, rendered in its own Ghostty-backed terminal section. Use it for feeds, logs, queues, git status, dev servers, or any TUI your team wants near every workspace.

## [#](https://cmux-docs-release.vercel.app/docs/dock#config-title)Configuration

cmux looks for Dock configuration in this order:

1.  `.cmux/dock.json` for the current repo, nearest parent project, and nested project directories.
2.  `~/.config/cmux/dock.json` for your personal default Dock or when there is no repo.

If a project and global config both exist, the project config wins. Nested project configs apply to their directory tree. If no project config exists, Dock uses the global config. If neither file exists, Dock opens empty.

Project Dock configs can start commands. cmux asks you to trust a project config before launching its controls.

## [#](https://cmux-docs-release.vercel.app/docs/dock#example-title)Example dock.json

A Dock file is a JSON object with a controls array. Commit project controls to .cmux/dock.json when you want teammates to share the same Dock. Replace example commands with tools your repo actually has.

.cmux/dock.json

```
{
  "controls": [
    {
      "id": "git",
      "title": "Git",
      "command": "lazygit",
      "height": 300
    },
    {
      "id": "logs",
      "title": "Logs",
      "command": "tail -f ./logs/development.log",
      "cwd": "."
    },
    {
      "id": "feed",
      "title": "Feed",
      "command": "cmux feed tui --opentui",
      "height": 320
    }
  ]
}
```

## [#](https://cmux-docs-release.vercel.app/docs/dock#fields-title)Fields

| Field | Description |
| --- | --- |
| `id` | Stable unique identifier for the control. Keep this short and do not reuse it for another command. |
| `title` | Label shown in the Dock header. |
| `command` | Command to run in the Dock terminal. It starts inside your login shell. |
| `cwd` | Optional working directory. Relative paths resolve from the project root for project configs, or from your home directory for global configs. |
| `height` | Optional requested terminal height in points. Controls without a height share the remaining space. |
| `env` | Optional environment variables passed only to that control. |

## [#](https://cmux-docs-release.vercel.app/docs/dock#sharing-title)Sharing with a team

Dock is designed to be shared with source control when the commands belong to the repo.

-   Put repo-specific controls in .cmux/dock.json and commit the file.
-   Put personal controls in ~/.config/cmux/dock.json, especially outside a repo, and keep that file out of shared source control.
-   Do not put secrets in dock.json. Read secrets from your shell, a local env file, or your existing dev tooling.

## [#](https://cmux-docs-release.vercel.app/docs/dock#agent-prompt-title)Ask an agent to set it up

Use this prompt when you want a coding agent to create Dock controls. It tells the agent to run \`cmux docs dock\`, inspect the project, and ask before guessing.

Agent prompt

```
Set up cmux Dock controls for the current context.

First, learn the feature before editing:
1. Run `cmux docs dock` if the cmux CLI is available. If it is not, read https://cmux.com/docs/dock.
2. Inspect the repository or current directory to understand the project type, scripts, package manager, dev servers, logs, task runners, test commands, and any existing TUI tools.
3. If the desired Dock is ambiguous, ask the user what they want monitored or controlled before writing files.

Dock is cmux's right-sidebar terminal control area. A Dock config is JSON with a top-level `controls` array. Each control runs a command in its own Ghostty-backed terminal section using the user's login shell. Controls are useful for project dashboards, git/status views, dev server or build status, test watchers, log tails, queues, local services, or a custom TUI such as `cmux feed tui --opentui` when that feed is useful.

Choose where to write the config:
- In a repository or project directory, create or edit `.cmux/dock.json` so teammates can share it.
- For a personal default outside a repo, create or edit `~/.config/cmux/dock.json`.
- If both exist, project `.cmux/dock.json` is more specific for that project. Nested project configs apply to that directory tree; use the nearest relevant project config instead of writing unrelated controls globally.
- If there is no repo and no clear project root, use the global config only after confirming the user wants a personal Dock.

Schema:
{
  "controls": [
    {
      "id": "short-stable-id",
      "title": "Human label",
      "command": "safe command to run",
      "cwd": "optional/path",
      "height": 220,
      "env": { "NAME": "value" }
    }
  ]
}

Rules:
- Keep ids stable, lowercase, and unique.
- Use `cwd` for subdirectories; relative paths resolve from the config base.
- Use `height` only when a control needs a fixed amount of vertical space.
- Use `env` only for non-secret values needed by one control.
- Do not put secrets, tokens, or machine-specific private paths in a shared project config.
- Prefer commands that are safe to start repeatedly and make sense in a terminal.
- Do not invent unavailable scripts. Read package files, Makefiles, Procfiles, README docs, config files, and existing tooling first.
- Keep shared project Docks portable for teammates. Put personal or machine-specific controls in the global Dock.

Deliverable:
- Create or update the appropriate dock.json.
- Preserve existing useful controls unless the user asked to replace them.
- Validate that the JSON parses.
- Summarize what each control does and any commands the user should review before trusting the Dock config.
```

[Custom Commands](https://cmux-docs-release.vercel.app/docs/custom-commands) [Keyboard Shortcuts](https://cmux-docs-release.vercel.app/docs/keyboard-shortcuts)

Canonical: https://cmux-docs-release.vercel.app/docs/dock
