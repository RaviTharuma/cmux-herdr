# [#](#title)Custom Commands

# [#](https://cmux-docs-release.vercel.app/docs/custom-commands#title)Custom Commands

Define actions, custom commands, and workspace layouts by adding .cmux/cmux.json to your project or ~/.config/cmux/cmux.json globally. Actions can appear in the surface tab bar and Command Palette.

## [#](https://cmux-docs-release.vercel.app/docs/custom-commands#file-locations)File locations

cmux looks for configuration in two places:

-   **Per-project:** `./.cmux/cmux.json` - lives in your project directory, takes precedence
-   **Fallback local:** `./cmux.json` - still supported for existing repos
-   **Global:** `~/.config/cmux/cmux.json` - applies to all projects, fills in commands not defined locally

Local actions and commands override global entries with the same ID or name.

The action registry is a nightly feature. Install the latest nightly build before using `actions`, `shortcut`, or `ui.surfaceTabBar.buttons`.

Project-local actions show up in the surface tab bar and Command Palette immediately. The first run still prompts for trust. Trust is per exact action fingerprint, not per repo. Project-local image icons stay locked until that action is trusted.

If a project or global config has a schema error, cmux falls back to the next valid config and shows a **cmux.json Schema Error** row in Command Palette. Select it to open the config file.

Edit cmux.json, then press Cmd+Shift+, or run cmux reload-config to apply changes.

## [#](https://cmux-docs-release.vercel.app/docs/custom-commands#schema)Schema

`commands` still define reusable shell commands and workspace layouts. Nightly builds add an `actions` registry. Actions are the public IDs shared by the surface tab bar, the Command Palette, and action-level shortcuts.

cmux.json

```
{
  "actions": {
    "cmux.newTerminal": {
      "type": "command",
      "title": "Codex",
      "subtitle": "Open Codex in a new terminal tab",
      "command": "codex --yolo",
      "target": "newTabInCurrentPane",
      "shortcut": "cmd+t",
      "icon": { "type": "image", "path": "./icons/codex.svg" }
    },
    "claude": {
      "type": "command",
      "title": "Claude Code",
      "command": "claude --dangerously-skip-permissions",
      "target": "newTabInCurrentPane",
      "shortcut": "cmd+shift+c",
      "icon": { "type": "image", "path": "./icons/claude.svg" }
    },
    "opencode": {
      "type": "command",
      "title": "OpenCode",
      "command": "opencode",
      "target": "newTabInCurrentPane",
      "palette": false,
      "icon": { "type": "emoji", "value": "🧪", "scale": 0.9 }
    },
    "web-dev": {
      "type": "workspaceCommand",
      "title": "Web Dev",
      "commandName": "Web Dev"
    }
  },
  "ui": {
    "surfaceTabBar": {
      "buttons": [
        "cmux.newTerminal",
        "cmux.newBrowser",
        "cmux.splitRight",
        "cmux.splitDown",
        "claude"
      ]
    }
  },
  "commands": [
    {
      "name": "Web Dev",
      "keywords": ["dev", "start"],
      "workspace": { ... }
    }
  ]
}
```

### [#](https://cmux-docs-release.vercel.app/docs/custom-commands#nightly-action-registry)Nightly action registry

`actions` maps stable IDs to runnable behavior. Use the built-in IDs `cmux.newTerminal`, `cmux.newBrowser`, `cmux.splitRight`, and `cmux.splitDown` to override the defaults. Use your own IDs for project-specific tools.

`palette` defaults to `true`. Set it to `false` to keep an action out of Command Palette while still making it available to the surface tab bar or a shortcut. `shortcut` uses the same syntax as settings shortcuts, for example `cmd+shift+c` or `["cmd+k", "cmd+c"]`.

`ui.surfaceTabBar.buttons` replaces the default button list when present. Leave out a built-in ID to hide it. Icons always use an object shape: `{ "type": "symbol", "name": "play.circle" }`, `{ "type": "emoji", "value": "🧪", "scale": 0.9 }`, or `{ "type": "image", "path": "./icons/codex.svg" }`. Image paths are relative to the config file. Emoji `scale` is optional and defaults to `1`. SVG, PDF, PNG, JPEG, GIF, TIFF, BMP, HEIC, HEIF, WebP, AVIF, ICO, and ICNS are supported.

Each button entry can be either an action ID string or a button object. Use a button object when you want the same action with a different surface label, icon, or tooltip. The resolved button title is also used as the trust prompt title.

Put any approval or permission flags directly in the command string you actually want to run. The default action target is `newTabInCurrentPane`, so the common pattern is to open a new terminal tab in the current pane and start Codex, Claude Code, or OpenCode there.

## [#](https://cmux-docs-release.vercel.app/docs/custom-commands#custom-actions)Custom actions and Command Palette

An `actions` entry is the reusable thing cmux runs. Use actions when the same behavior should be available from the Command Palette, surface tab bar, shortcuts, or the plus-button menu. Keep `commands` for reusable shell commands and workspace layouts. Set `palette` to false when an action should stay out of Command Palette.

### [#](https://cmux-docs-release.vercel.app/docs/custom-commands#action-types)Action types

-   `"builtin"`: Alias a built-in cmux action such as cmux.newTerminal, cmux.newBrowser, cmux.splitRight, or cmux.splitDown.
-   `"command"`: Run shell text in a terminal. Use target to choose the current terminal or a new tab in the current pane.
-   `"agent"`: Start a coding agent in a new terminal tab. Any CLI name works (claude, codex, opencode, or your own binary), with optional args.
-   `"workspaceCommand"`: Run a named workspace definition from commands. Use this for multi-pane layouts, custom working directories, and startup commands.
-   `"workspace"`: Create a workspace from an inline workspace definition, with optional restart behavior. Offered in the new-workspace plus-button menu automatically.

### [#](https://cmux-docs-release.vercel.app/docs/custom-commands#action-fields)Action fields

-   `title`: Command Palette row title, surface button label, menu item title, and trust prompt title unless an entry overrides it.
-   `subtitle` / `description`: Command Palette secondary text. description is accepted as an alias for subtitle.
-   `keywords`: Extra search terms for Command Palette.
-   `palette`: Defaults to true for custom actions. Set false to hide the action from Command Palette while keeping it callable elsewhere.
-   `shortcut`: Optional action shortcut, using the same single-stroke or two-stroke chord syntax as settings shortcuts.
-   `target`: For command and agent actions only. Use currentTerminal or newTabInCurrentPane.
-   `confirm`: Ask before running the action.
-   `newWorkspaceMenu`: Set true to offer any action in the new-workspace plus-button menu, or false to hide a workspace action from it.

### [#](https://cmux-docs-release.vercel.app/docs/custom-commands#command-palette-behavior)Command Palette behavior

Command Palette reads the resolved action registry. Custom action IDs are added as rows when `palette` is not false. Legacy `commands` are added automatically as custom rows unless an action with the same generated ID already exists. Built-in commands keep their normal palette labels, but overriding a built-in ID such as `cmux.newTerminal` changes the behavior behind that shared entrypoint.

## [#](https://cmux-docs-release.vercel.app/docs/custom-commands#new-workspace-button)Custom plus button actions

Use `ui.newWorkspace.action` to override what the plus button does. Use `ui.newWorkspace.contextMenu` (or the `rightClick` alias) to define the ordered right-click menu. Menu entries can be action IDs, action objects, or `{ "type": "separator" }`.

cmux.json

```
{
  "actions": {
    "worktree-agents": {
      "type": "workspaceCommand",
      "title": "Worktree Agents",
      "commandName": "Worktree Agents",
      "icon": { "type": "symbol", "name": "folder.badge.plus" }
    }
  },
  "ui": {
    "newWorkspace": {
      "action": "worktree-agents",
      "contextMenu": [
        { "action": "worktree-agents", "title": "Worktree Agents" },
        { "type": "separator" },
        { "action": "cmux.newTerminal", "title": "New Terminal" },
        { "action": "cmux.newBrowser", "title": "New Browser" }
      ]
    }
  },
  "commands": [
    {
      "name": "Worktree Agents",
      "description": "Create a fresh Git worktree and start Codex and Claude inside it",
      "workspace": {
        "name": "Worktree Agents",
        "cwd": ".",
        "layout": {
          "direction": "horizontal",
          "split": 0.38,
          "children": [
            {
              "pane": {
                "surfaces": [
                  {
                    "type": "terminal",
                    "name": "Worktree",
                    "command": "set -euo pipefail; state=\"${TMPDIR:-/tmp}/cmux-worktree-${CMUX_WORKSPACE_ID:-manual}.dir\"; rm -f \"$state\"; repo=$(git rev-parse --show-toplevel); mkdir -p \"$repo/../worktrees\"; slug=agents-$(date +%Y%m%d-%H%M%S); dir=\"$repo/../worktrees/$slug\"; git -C \"$repo\" worktree add -b \"$slug\" \"$dir\"; printf \"%s\\n\" \"$dir\" > \"$state\"; cd \"$dir\"; exec \"${SHELL:-/bin/zsh}\" -l",
                    "focus": true
                  }
                ]
              }
            },
            {
              "direction": "vertical",
              "split": 0.5,
              "children": [
                {
                  "pane": {
                    "surfaces": [
                      {
                        "type": "terminal",
                        "name": "Codex",
                        "command": "state=\"${TMPDIR:-/tmp}/cmux-worktree-${CMUX_WORKSPACE_ID:-manual}.dir\"; echo \"Waiting for worktree...\"; while [ ! -s \"$state\" ]; do sleep 0.2; done; dir=$(cat \"$state\"); cd \"$dir\"; exec codex --yolo"
                      }
                    ]
                  }
                },
                {
                  "pane": {
                    "surfaces": [
                      {
                        "type": "terminal",
                        "name": "Claude",
                        "command": "state=\"${TMPDIR:-/tmp}/cmux-worktree-${CMUX_WORKSPACE_ID:-manual}.dir\"; echo \"Waiting for worktree...\"; while [ ! -s \"$state\" ]; do sleep 0.2; done; dir=$(cat \"$state\"); cd \"$dir\"; exec claude --dangerously-skip-permissions"
                      }
                    ]
                  }
                }
              ]
            }
          ]
        }
      }
    }
  ]
}
```

This example makes the normal plus click run the `worktree-agents` action. The workspace command from `commands` uses a visible setup terminal to create the Git `worktree` first. Codex and Claude start at the same time, wait for the workspace-specific state file, then cd into the created directory before exec.

## [#](https://cmux-docs-release.vercel.app/docs/custom-commands#workspace-layouts)Workspace layouts

A workspace layout is stored as a `workspace` action, embedding a full workspace definition directly in the action so it needs no separate `commands` entry. It supports the same workspace fields plus `setup`, a bootstrap command sent to the first terminal before that terminal's own command:

cmux.json

```
{
  "actions": {
    "review-setup": {
      "type": "workspace",
      "title": "Review Setup",
      "icon": { "type": "symbol", "name": "rectangle.stack.badge.plus" },
      "restart": "confirm",
      "workspace": {
        "name": "Review",
        "cwd": "~/code/app",
        "setup": "git fetch --all --prune",
        "layout": {
          "direction": "horizontal",
          "split": 0.5,
          "children": [
            {
              "pane": {
                "surfaces": [
                  { "type": "terminal", "name": "Claude", "command": "claude", "focus": true }
                ]
              }
            },
            {
              "pane": {
                "surfaces": [
                  { "type": "terminal", "name": "OpenCode", "command": "opencode" }
                ]
              }
            }
          ]
        }
      }
    }
  }
}
```

Workspace layouts appear in the plus-button right-click menu automatically. Set `newWorkspaceMenu` to `false` to hide one, or to `true` on any other action to add it to the menu.

You can also create these without writing JSON: right-click the plus button and choose **Save Workspace as Layout** to capture the current workspace's splits, directories, running agents, and browser tabs into `~/.config/cmux/cmux.json`. They are stored as entries in the `actions` block. **Customize Workspace Layouts** opens that file in your editor.

### [#](https://cmux-docs-release.vercel.app/docs/custom-commands#default-workspace-layout)Default for new workspaces

Use the **Default for New Workspace** submenu to pick which saved workspace layout plain New Workspace should run, or choose None to return to a blank terminal. The save dialog also has a **Use as default for new workspaces** checkbox. The setting is `ui.newWorkspace.action`; a project-local `.cmux/cmux.json` value wins over `~/.config/cmux/cmux.json`.

## [#](https://cmux-docs-release.vercel.app/docs/custom-commands#simple-commands)Simple commands

A simple command runs a shell command in the currently focused terminal:

cmux.json

```
{
  "commands": [
    {
      "name": "Run Tests",
      "keywords": ["test", "check"],
      "command": "npm test",
      "confirm": true
    }
  ]
}
```

### [#](https://cmux-docs-release.vercel.app/docs/custom-commands#simple-command-fields)Fields

-   `name`: Displayed in the command palette (required)
-   `description`: Optional description
-   `keywords`: Extra search terms for the command palette
-   `command`: Shell command to run in the focused terminal
-   `confirm`: Show a confirmation dialog before running

Simple commands run in the focused terminal's current working directory. If your command relies on project-relative paths, prefix it with `cd "$(git rev-parse --show-toplevel)" &&` to run from the repo root, or `cd /your/path &&` for any specific directory.

## [#](https://cmux-docs-release.vercel.app/docs/custom-commands#workspace-commands)Workspace commands

A workspace command creates a new workspace with a custom layout of splits, terminals, and browser panes:

cmux.json

```
{
  "commands": [
    {
      "name": "Dev Environment",
      "keywords": ["dev", "fullstack"],
      "workspace": {
        "name": "Dev",
        "cwd": ".",
        "layout": {
          "direction": "horizontal",
          "split": 0.5,
          "children": [
            {
              "pane": {
                "surfaces": [
                  {
                    "type": "terminal",
                    "name": "Frontend",
                    "command": "npm run dev",
                    "focus": true
                  }
                ]
              }
            },
            {
              "pane": {
                "surfaces": [
                  {
                    "type": "terminal",
                    "name": "Backend",
                    "command": "cargo watch -x run",
                    "cwd": "./server",
                    "env": { "RUST_LOG": "debug" }
                  }
                ]
              }
            }
          ]
        }
      }
    }
  ]
}
```

### [#](https://cmux-docs-release.vercel.app/docs/custom-commands#workspace-fields)Workspace fields

-   `name`: Workspace tab name (defaults to command name)
-   `cwd`: Working directory for the workspace
-   `color`: Workspace tab color
-   `env`: Environment variables inherited by every shell in the workspace
-   `setup`: Bootstrap command sent to the workspace's first terminal before that terminal's own command
-   `layout`: Layout tree defining splits and panes

### [#](https://cmux-docs-release.vercel.app/docs/custom-commands#restart-behavior)Restart behavior

Controls what happens when a workspace with the same name already exists:

-   `"new"`: Create a new workspace (default)
-   `"ignore"`: Switch to the existing workspace
-   `"recreate"`: Close and recreate without asking
-   `"confirm"`: Ask the user before recreating

## [#](https://cmux-docs-release.vercel.app/docs/custom-commands#layout-tree)Layout tree

The layout tree defines how panes are arranged using recursive split nodes:

### [#](https://cmux-docs-release.vercel.app/docs/custom-commands#split-node)Split node

Divides space into two children:

-   `direction`: `"horizontal"` or `"vertical"`
-   `split`: Divider position from 0.1 to 0.9 (default 0.5)
-   `children`: Exactly two child nodes (split or pane)

### [#](https://cmux-docs-release.vercel.app/docs/custom-commands#pane-node)Pane node

A leaf node containing one or more surfaces (tabs within the pane).

## [#](https://cmux-docs-release.vercel.app/docs/custom-commands#surface-definition)Surface definition

Each surface in a pane can be a terminal or a browser:

-   `type`: `"terminal"` or `"browser"`
-   `name`: Custom tab title
-   `command`: Shell command to auto-run on creation (terminal only)
-   `cwd`: Working directory for this surface
-   `env`: Environment variables as key-value pairs
-   `url`: URL to open (browser only)
-   `focus`: Focus this surface after creation

### [#](https://cmux-docs-release.vercel.app/docs/custom-commands#cwd-resolution)Working directory resolution

-   `.` or omitted: workspace working directory
-   `./subdir`: relative to workspace working directory
-   `~/path`: expanded to home directory
-   Absolute path: used as-is

## [#](https://cmux-docs-release.vercel.app/docs/custom-commands#full-example)Full example

cmux.json

```
{
  "actions": {
    "web-dev": { "type": "workspaceCommand", "commandName": "Web Dev" },
    "cmux.newTerminal": {
      "type": "command",
      "title": "Codex",
      "command": "codex --yolo",
      "target": "newTabInCurrentPane",
      "shortcut": "cmd+t",
      "icon": { "type": "image", "path": "./icons/codex.svg" }
    },
    "claude": {
      "type": "command",
      "title": "Claude Code",
      "command": "claude --dangerously-skip-permissions",
      "target": "newTabInCurrentPane",
      "shortcut": "cmd+shift+c",
      "icon": { "type": "image", "path": "./icons/claude.svg" }
    },
    "start-dev": {
      "type": "command",
      "command": "npm run dev",
      "target": "newTabInCurrentPane",
      "icon": { "type": "symbol", "name": "play.circle" }
    }
  },
  "ui": {
    "surfaceTabBar": {
      "buttons": [
        "cmux.newTerminal",
        "cmux.newBrowser",
        "cmux.splitRight",
        "cmux.splitDown",
        {
          "action": "claude",
          "title": "Claude Here"
        },
        "start-dev"
      ]
    }
  },
  "commands": [
    {
      "name": "Web Dev",
      "description": "Docs site with live preview",
      "keywords": ["web", "docs", "next", "frontend"],
      "workspace": {
        "name": "Web Dev",
        "cwd": "./web",
        "color": "#3b82f6",
        "layout": {
          "direction": "horizontal",
          "split": 0.5,
          "children": [
            {
              "pane": {
                "surfaces": [
                  {
                    "type": "terminal",
                    "name": "Next.js",
                    "command": "npm run dev",
                    "focus": true
                  }
                ]
              }
            },
            {
              "direction": "vertical",
              "split": 0.6,
              "children": [
                {
                  "pane": {
                    "surfaces": [
                      {
                        "type": "browser",
                        "name": "Preview",
                        "url": "http://localhost:3777"
                      }
                    ]
                  }
                },
                {
                  "pane": {
                    "surfaces": [
                      {
                        "type": "terminal",
                        "name": "Shell",
                        "env": { "NODE_ENV": "development" }
                      }
                    ]
                  }
                }
              ]
            }
          ]
        }
      }
    },
    {
      "name": "Debug Log",
      "description": "Tail the debug event log from the running dev app",
      "keywords": ["log", "debug", "tail", "events"],
      "workspace": {
        "name": "Debug Log",
        "layout": {
          "direction": "horizontal",
          "split": 0.5,
          "children": [
            {
              "pane": {
                "surfaces": [
                  {
                    "type": "terminal",
                    "name": "Events",
                    "command": "tail -f /tmp/cmux-debug.log",
                    "focus": true
                  }
                ]
              }
            },
            {
              "pane": {
                "surfaces": [
                  {
                    "type": "terminal",
                    "name": "Shell"
                  }
                ]
              }
            }
          ]
        }
      }
    },
    {
      "name": "Setup",
      "description": "Initialize submodules and build dependencies",
      "keywords": ["setup", "init", "install"],
      "command": "./scripts/setup.sh",
      "confirm": true
    },
    {
      "name": "Reload",
      "description": "Build and launch the debug app tagged to the current branch",
      "keywords": ["reload", "build", "run", "launch"],
      "command": "./scripts/reload.sh --tag $(git branch --show-current)"
    },
    {
      "name": "Run Unit Tests",
      "keywords": ["test", "unit"],
      "command": "./scripts/test-unit.sh",
      "confirm": true
    }
  ]
}
```

[Task Manager](https://cmux-docs-release.vercel.app/docs/task-manager) [Dock](https://cmux-docs-release.vercel.app/docs/dock)

Canonical: https://cmux-docs-release.vercel.app/docs/custom-commands
