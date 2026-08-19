# [#](#title)Skills

# [#](https://cmux-docs-release.vercel.app/docs/skills#title)Skills

cmux ships skills that teach coding agents how to use cmux CLI control, current-workspace automation, settings, customization, diagnostics, browser surfaces, and markdown panels.

## [#](https://cmux-docs-release.vercel.app/docs/skills#install-title)Install

Use Vercel's `skills` CLI for the standard multi-agent install flow, or use `skills.sh` when you only need the Codex skills directory.

Install with Vercel skills

```
# Install all cmux skills
npx skills add manaflow-ai/cmux -g -y

# Or install just diagnostics
npx skills add manaflow-ai/cmux --skill cmux-diagnostics -g -y
```

Install with skills.sh

```
curl -fsSL https://raw.githubusercontent.com/manaflow-ai/cmux/main/skills.sh | bash
```

By default, `skills.sh` installs into `~/.codex/skills`, or `$CODEX_HOME/skills` when CODEX\_HOME is set. Pass `--dest` when your agent reads skills from another directory.

### [#](https://cmux-docs-release.vercel.app/docs/skills#local-install-title)Install from a checkout

From a cloned cmux checkout, the script uses the local skills directory.

Local commands

```
./skills.sh
./skills.sh --list
./skills.sh --skill cmux --skill cmux-browser
./skills.sh --dest ~/.codex/skills
./skills.sh --dry-run
```

Use --ref to install from a specific branch, tag, or commit when the script downloads from GitHub.

```
curl -fsSL https://raw.githubusercontent.com/manaflow-ai/cmux/main/skills.sh | bash -s -- --ref main
```

## [#](https://cmux-docs-release.vercel.app/docs/skills#included-title)Included skills

Each skill has a SKILL.md file, optional references, and an OpenAI metadata file under agents/openai.yaml.

| Skill | Use | Typical command |
| --- | --- | --- |
| **cmux Core**
`skills/cmux/SKILL.md` |
Controls windows, workspaces, panes, surfaces, focus, moves, reorder, and routing with the cmux CLI.

Use it when an agent needs deterministic placement or navigation in a multi-pane cmux layout.

 | `cmux identify --json` |
| **cmux Workspace**
`skills/cmux-workspace/SKILL.md` |

Keeps automation scoped to the current caller workspace, caller surface, panes, sidebar metadata, and socket context.

Use it when an agent should add panes, send input, inspect state, or open helper surfaces without disrupting another workspace.

 | `cmux current-workspace --json` |
| **cmux Settings**
`skills/cmux-settings/SKILL.md` |

Inspects, edits, validates, and opens ~/.config/cmux/cmux.json with a bundled helper script.

Use it when changing appearance, sidebar, notification, browser, automation, or shortcut settings by JSON path.

 | `skills/cmux-settings/scripts/cmux-settings list-supported` |
| **cmux Customization**
`skills/cmux-customization/SKILL.md` |

Customizes cmux.json actions, plus-button behavior, tab bar buttons, workspace layouts, Dock controls, Feed hooks, sidebar settings, Command Palette entries, shortcuts, and Ghostty-owned terminal preferences.

Use it to make cmux open a user's worktrees, multiple checkouts, SSH sessions, dev tools, or project layout from the exact UI entrypoints they want.

 | `cmux reload-config` |
| **cmux Diagnostics**
`skills/cmux-diagnostics/SKILL.md` |

Runs support-safe checks for cmux CLI health, socket access, hooks, session restore, settings, and agent binaries.

Use it when notifications, hooks, restore, or automation are not behaving as expected.

 | `skills/cmux-diagnostics/scripts/cmux-diagnostics` |
| **cmux Browser**
`skills/cmux-browser/SKILL.md` |

Automates cmux webview surfaces with snapshot refs, DOM actions, waits, screenshots, and session state.

Use it for browser tasks that should run inside cmux instead of a separate browser automation tool.

 | `cmux browser surface:2 snapshot --interactive` |
| **cmux Markdown Viewer**
`skills/cmux-markdown/SKILL.md` |

Opens markdown files in a formatted cmux panel with live reload.

Use it to show plans, docs, notes, and task lists beside the terminal while work is happening.

 | `cmux markdown open plan.md` |

## [#](https://cmux-docs-release.vercel.app/docs/skills#coverage-title)What the skills cover

The top-level SKILL.md files stay short. Deeper command details live in references, scripts, and templates next to each skill.

| Skill | Scope | Reference coverage |
| --- | --- | --- |
| **cmux Core**
`cmux` | Topology control for windows, workspaces, panes, surfaces, focus, moves, reorders, split-off flows, and attention flashes. | Handle syntax, caller targeting, window and workspace lifecycle, pane and surface routing, trigger flash, and health checks. |
| **cmux Workspace**
`cmux-workspace` | Current-workspace automation rules for caller context, additive panes, helper surfaces, sidebar metadata, input, and socket selection. | Workspace command reference covering context, windows, workspaces, panes, surfaces, input, sidebar state, notifications, docs, and tagged reloads. |
| **cmux Settings**
`cmux-settings` | cmux.json settings reads and writes, key lookup, JSONC parsing, safe atomic updates, validation, editor opening, and shortcut binding edits. | Generated settings key list, shortcut action ids, schema URL, supported path detection, and the bundled cmux-settings helper. |
| **cmux Customization**
`cmux-customization` | End-user customization across cmux.json settings, actions, commands, workspace layouts, plus-button click and right-click menus, surface tab bar buttons, Dock controls, Feed and notification hooks, sidebar metadata, Command Palette entries, shortcuts, and Ghostty config boundaries. | Config-surface selection, what can be customized, global versus project-local scope, action examples, plus-button wiring, tab bar button examples, Dock examples, Feed hooks, workspace layout examples, reload steps, validation rules, and safety constraints. |
| **cmux Diagnostics**
`cmux-diagnostics` | Read-only health checks for CLI reachability, socket access, cmux environment, settings validation, hook installation markers, session stores, auto-resume settings, and supported agent binaries. | Bundled support-safe diagnostic script, hook setup commands, session restore interpretation, notification checks, and rules for redacting sensitive files. |
| **cmux Browser**
`cmux-browser` | Browser automation inside cmux webview surfaces, including navigation, DOM actions, waits, state capture, screenshots, and snapshots. | Command mapping, snapshot ref lifecycle, authentication, session persistence, video status, proxy behavior, and reusable automation templates. |
| **cmux Markdown Viewer**
`cmux-markdown` | Formatted markdown panels that live beside terminals and reload as files change. | Command syntax, routing options, live reload behavior, atomic file replacement, and markdown rendering coverage. |

## [#](https://cmux-docs-release.vercel.app/docs/skills#customization-examples-title)Customization examples library

These are reusable workflow patterns that a user can ask an agent to apply with cmux Customization.

| Example | Customizes | Good for |
| --- | --- | --- |
| **Worktree agents**
`worktree-agents` | Plus-button click, plus-button right-click menu, Command Palette, workspace layout | Opening a dedicated worktree or checkout with paired Codex, Claude, SSH, or helper terminals. |
| **Full-stack dev**
`full-stack-dev` | Workspace command, browser preview, Dock controls | Starting frontend, backend, tests, logs, and preview panes in the same repeatable layout. |
| **SSH devbox**
`ssh-devbox` | Workspace command, remote terminal, browser surface | Connecting to a remote environment while keeping local previews, notes, and navigation in cmux. |
| **Review PR**
`review-pr` | Workspace command, browser surface, markdown or terminal notes | Opening GitHub status, the pull request, and review notes in a single workspace. |
| **Docs workspace**
`docs-workspace` | Workspace command, markdown panel, browser preview | Editing documentation with the rendered page, local dev server, and source notes visible. |
| **CI watch**
`ci-watch` | Dock controls, Feed TUI, notification hooks | Watching GitHub Actions, CircleCI, release monitors, and agent hook events without losing the main workspace. |
| **Quick agent buttons**
`quick-agent-buttons` | Surface tab bar buttons, actions, Command Palette entries | Adding one-click Codex, Claude, or custom-agent launchers while keeping built-in tab buttons visible. |

Detailed snippets live in `skills/cmux-customization/references/examples.md`. Agents load that file only when an examples-library pattern is needed.

Example prompts

```
Use $cmux-customization to set up the worktree-agents example.
Use $cmux-customization to create a project-local full-stack-dev layout with Dock controls.
Use $cmux-customization to add quick-agent-buttons for Codex and Claude.
```

## [#](https://cmux-docs-release.vercel.app/docs/skills#help-menu-title)Help menu

The macOS **Help** menu mirrors this docs sidebar and includes **Skills**. Use the Skills item in Help to open this page from the app.

## [#](https://cmux-docs-release.vercel.app/docs/skills#authoring-title)Skill layout

Keep the main SKILL.md concise. Put deeper command tables, scripts, and reusable templates next to the skill.

```
skills/<name>/SKILL.md
skills/<name>/agents/openai.yaml
skills/<name>/references/*.md
skills/<name>/scripts/*
skills/<name>/templates/*
```

When adding a new skill, include `agents/openai.yaml` so the installer exposes a clear name, short description, and default prompt.

## [#](https://cmux-docs-release.vercel.app/docs/skills#suggestions-title)Suggested future skills

These are good candidates if the skill set grows beyond the current installed list.

| Skill idea | Use case | Why it helps |
| --- | --- | --- |
| **cmux SSH**
`cmux-ssh` | Drive remote workspaces, SSH URL launches, browser routing through remote networks, and reconnect behavior. | Remote sessions are a major cmux surface with behavior that differs from local terminal workflows. |
| **cmux Cloud VM**
`cmux-cloud-vm` | Operate Cloud VM create, attach, exec, SSH endpoint, billing, and provider troubleshooting flows. | Cloud VM work crosses the web app, database, provider APIs, and smoke tests. |
| **cmux Vault**
`cmux-vault` | Manage vault-backed agent configuration, credentials, and restore behavior without leaking secrets into prompts. | Vault workflows need stricter handling than normal settings because they affect secrets and agent startup. |

Keep future skills focused on repeatable workflows with real command sequences. If a skill would only restate product docs, add it to docs instead.

## [#](https://cmux-docs-release.vercel.app/docs/skills#related-title)Related docs

-   [Browser Automation](https://cmux-docs-release.vercel.app/docs/browser-automation)
-   [CLI Reference](https://cmux-docs-release.vercel.app/docs/api)
-   [Custom Commands](https://cmux-docs-release.vercel.app/docs/custom-commands)

[Browser Automation](https://cmux-docs-release.vercel.app/docs/browser-automation) [Notifications](https://cmux-docs-release.vercel.app/docs/notifications)

Canonical: https://cmux-docs-release.vercel.app/docs/skills
