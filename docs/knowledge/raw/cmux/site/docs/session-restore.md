# Session Restore

cmux saves the shape of your work so relaunching the app can bring back the same windows, workspaces, panes, terminal context, and browser state.

## What cmux restores

After relaunch, cmux restores app-owned layout and metadata:

-   Window, workspace, and pane layout
-   Working directories
-   Terminal scrollback, best effort
-   Browser URL and navigation history

cmux does not checkpoint arbitrary live process state. tmux, vim, shells, and unsupported terminal apps reopen as normal terminals unless they have their own cmux resume integration.

## Agent session resume

Supported AI coding agents can resume when cmux has captured the agent's native session ID. Install hooks after installing the agent CLI so the agent binary is on PATH:

```
cmux hooks setup
cmux hooks setup codex
cmux hooks setup grok
cmux hooks setup antigravity
cmux hooks setup omp
cmux hooks setup --agent opencode
```

Running cmux hooks setup installs every supported integration it can find and prints a summary for skipped agents. Use an agent name when you only want one integration.

## Custom surface resume commands

Advanced users and integrations can bind any terminal surface to a restart command. cmux stores public CLI and socket-created bindings for inspection and manual restore unless you approve a signed command prefix.

```
cmux surface resume set --kind tmux --checkpoint work --shell "tmux attach -t work"
cmux surface resume show --json
cmux surface resume clear --checkpoint work
```

Review or edit approved prefixes in Settings > Terminal > Resume Commands. cmux only auto-runs resume bindings it marks trusted, such as live process-detected tmux bindings or user-approved prefixes. cmux still does not checkpoint arbitrary process memory. Sensitive environment keys such as tokens, passwords, secrets, and API keys are dropped before a resume binding is stored. Approved prefixes are also bound to the working directory and exact environment values when present.

## Supported agents

| Agent | Binary | Resume command | Feed bridge |
| --- | --- | --- | --- |
| Claude Code | `claude` | `claude --resume <id>` | PermissionRequest |
| Codex | `codex` | `codex resume <id>` | PreToolUse, PermissionRequest |
| Grok / Grok Build CLI | `grok` | `grok -r <id>` | PreToolUse |
| OpenCode | `opencode` | `opencode --session <id>` | plugin event bus |
| Pi | `pi` | `pi --session <id>` | none |
| OMP | `omp` | `omp --session <id>` | none |
| Campfire | `campfire` | `campfire --session <id>` | none |
| Amp | `amp` | `amp threads continue <id>` | none |
| Cursor CLI | `cursor-agent` | `cursor-agent --resume <id>` | beforeShellExecution |
| Gemini | `gemini` | `gemini --resume <id>` | PreToolUse |
| Antigravity CLI | `agy` | `agy --conversation <id>` | PreToolUse, PostToolUse |
| Rovo Dev | `acli` | `acli rovodev run --restore <id>` | none |
| Hermes Agent | `hermes` | `hermes --resume <id>` | pre\_tool\_call, post\_tool\_call, pre\_approval\_request, post\_approval\_response |
| Copilot | `copilot` | `copilot --resume <id>` | PreToolUse |
| CodeBuddy | `codebuddy` | `codebuddy --resume <id>` | PreToolUse |
| Factory | `droid` | `droid --resume <id>` | PreToolUse |
| Qoder | `qodercli` | `qodercli --resume <id>` | PreToolUse |

Claude Code is handled by the cmux Claude wrapper when Claude integration is enabled in Settings. Antigravity also accepts agy as the setup alias, and Rovo Dev accepts rovo.

## Manual restore

cmux restores the last saved snapshot on normal launch. You can also reapply the previous snapshot manually:

-   History > Restore Previous App Launch
-   `⌘ ⇧ O`
-   `cmux restore-session`

## Disable automatic agent resume

To restore panes without launching saved agent resume commands, turn off Settings > Terminal > Resume Agent Sessions on Reopen or set:

~/.config/cmux/cmux.json

```
{
  "terminal": {
    "autoResumeAgentSessions": false
  }
}
```

This only disables agent resume commands. cmux still restores layout, working directories, scrollback, and browser history.

## How it works

1.  cmux writes a versioned JSON snapshot to ~/Library/Application Support/cmux/session-<bundle-id>.json, plus a previous-session cache for manual reopen.
2.  Terminal scrollback is stored as bounded text and replayed through a temporary file on restore. This is best effort because terminal apps can redraw or clear their screen.
3.  Agent hooks write ~/.cmuxterm/<agent>-hook-sessions.json with the agent session ID, cmux workspace ID, surface ID, cwd, process ID when available, and a sanitized launch command.
4.  On restore, cmux rebuilds windows and panes first. If automatic agent resume is enabled, it launches a one-shot shell command that runs the agent's native resume command with the saved session ID.

The regular [configuration docs](https://cmux-docs-release.vercel.app/docs/configuration) cover cmux.json. Session restore keeps app layout separate from Ghostty terminal rendering settings.

[TextBox (Beta)](https://cmux-docs-release.vercel.app/docs/textbox) [Vault](https://cmux-docs-release.vercel.app/docs/vault)

Canonical: https://cmux-docs-release.vercel.app/docs/session-restore
