> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Quickstart

> Get up and running with cmux in 5 minutes

This guide will get you from installation to your first meaningful workflow: creating workspaces, setting up notifications, and using the in-app browser.

## Prerequisites

* cmux installed (see [Installation](/installation))
* Basic familiarity with terminal commands

## Launch cmux

Open cmux from your Applications folder or via Spotlight (`Cmd+Space`, type "cmux").

On first launch, you'll see a single workspace with a terminal. The sidebar on the left shows your workspace tabs.

## Create a workspace

Workspaces are the top-level organizing unit in cmux. Each workspace can contain multiple split panes and surfaces (terminals and browsers).

<Steps>
  <Step title="Create a new workspace">
    Press `Cmd+N` or use the CLI:

    ```bash theme={null}
    cmux new-workspace
    ```

    You can also create a workspace in a specific directory:

    ```bash theme={null}
    cmux new-workspace --cwd ~/projects/myapp
    ```
  </Step>

  <Step title="Navigate between workspaces">
    Use keyboard shortcuts to switch workspaces:

    | Shortcut     | Action                 |
    | ------------ | ---------------------- |
    | `Cmd+1–8`    | Jump to workspace 1–8  |
    | `Cmd+9`      | Jump to last workspace |
    | `Ctrl+Cmd+]` | Next workspace         |
    | `Ctrl+Cmd+[` | Previous workspace     |
  </Step>

  <Step title="Rename a workspace">
    Press `Cmd+Shift+R` or use the CLI:

    ```bash theme={null}
    cmux rename-workspace "My Project"
    ```
  </Step>
</Steps>

## Split panes

Split your workspace horizontally or vertically to work with multiple terminals side-by-side.

<Steps>
  <Step title="Split right">
    Press `Cmd+D` to create a vertical split.
  </Step>

  <Step title="Split down">
    Press `Cmd+Shift+D` to create a horizontal split.
  </Step>

  <Step title="Navigate between panes">
    Use `Opt+Cmd+Arrow` to focus panes directionally.
  </Step>

  <Step title="Flash focused pane">
    Press `Cmd+Shift+H` to briefly highlight which pane has focus.
  </Step>
</Steps>

You can also split panes via CLI:

```bash theme={null}
cmux new-split right
cmux new-split down
```

## Set up notifications

Notifications are cmux's killer feature for working with AI coding agents. When an agent needs your attention, its pane gets a blue ring and the tab lights up.

<Steps>
  <Step title="Send a test notification">
    ```bash theme={null}
    cmux notify --title "Build Complete" --subtitle "myapp" --body "Tests passed"
    ```

    You'll see:

    * A blue ring around the current pane
    * The workspace tab glows in the sidebar
    * The notification appears in the notification panel
  </Step>

  <Step title="Open the notification panel">
    Press `Cmd+I` to see all notifications.
  </Step>

  <Step title="Jump to latest unread">
    Press `Cmd+Shift+U` to jump to the most recent unread notification.
  </Step>

  <Step title="Clear notifications">
    ```bash theme={null}
    cmux clear-notifications
    ```
  </Step>
</Steps>

### Terminal notification sequences

cmux also picks up standard terminal notification sequences (OSC 9/99/777):

```bash theme={null}
# OSC 777
echo -e "\033]777;notify;Build Complete;Tests passed\033\\"

# OSC 9
echo -e "\033]9;Build Complete\033\\"

# OSC 99
echo -e "\033]99;i=1:p=body;Build Complete\033\\"
```

## Use the in-app browser

The in-app browser lets you view and automate web pages alongside your terminal.

<Steps>
  <Step title="Open browser in split">
    Press `Cmd+Shift+L` or use the CLI:

    ```bash theme={null}
    cmux browser open http://localhost:3000
    ```
  </Step>

  <Step title="Navigate to a URL">
    Press `Cmd+L` to focus the address bar, type a URL, and press Enter.

    Or via CLI:

    ```bash theme={null}
    cmux browser navigate http://example.com
    ```
  </Step>

  <Step title="Browser shortcuts">
    | Shortcut    | Action                  |
    | ----------- | ----------------------- |
    | `Cmd+L`     | Focus address bar       |
    | `Cmd+[`     | Back                    |
    | `Cmd+]`     | Forward                 |
    | `Cmd+R`     | Reload page             |
    | `Opt+Cmd+I` | Toggle Developer Tools  |
    | `Opt+Cmd+C` | Show JavaScript Console |
  </Step>
</Steps>

### Scriptable browser API

The browser has a scriptable API ported from [agent-browser](https://github.com/vercel-labs/agent-browser) for automation:

```bash theme={null}
# Get current URL
cmux browser get-url

# Navigate back/forward
cmux browser back
cmux browser forward

# Reload page
cmux browser reload
```

For advanced automation (accessibility tree snapshots, element interactions), see the [Browser CLI reference](/cli/browser).

## Integrate with AI coding agents

cmux is designed to work seamlessly with AI coding tools. Here's how to integrate with popular agents:

### Claude Code

cmux has built-in Claude Code integration. To enable it:

<Steps>
  <Step title="Set up the hook">
    cmux can automatically notify you when Claude Code is waiting. The integration is enabled by default in recent versions.

    When Claude Code pauses for input, you'll see a notification in the workspace where it's running.
  </Step>

  <Step title="Resume from notifications">
    Press `Cmd+Shift+U` to jump to the latest Claude Code prompt.
  </Step>
</Steps>

### OpenCode

For OpenCode or other agents, use the `cmux notify` command in your shell prompts or agent hooks:

```bash theme={null}
# In your agent's output handler
cmux notify --title "OpenCode" --subtitle "Waiting for input" --body "$PROMPT_TEXT"
```

### Custom integrations

You can wire `cmux notify` into any tool that supports custom commands or hooks:

```bash theme={null}
# Example: notify when a long build finishes
make build && cmux notify --title "Build" --body "Build succeeded" || cmux notify --title "Build" --body "Build failed"
```

## Keyboard shortcuts reference

Here are the essential shortcuts to get started:

### Workspaces

| Shortcut      | Action                 |
| ------------- | ---------------------- |
| `Cmd+N`       | New workspace          |
| `Cmd+1–8`     | Jump to workspace 1–8  |
| `Cmd+9`       | Jump to last workspace |
| `Ctrl+Cmd+]`  | Next workspace         |
| `Ctrl+Cmd+[`  | Previous workspace     |
| `Cmd+Shift+W` | Close workspace        |
| `Cmd+Shift+R` | Rename workspace       |
| `Cmd+B`       | Toggle sidebar         |

### Split panes

| Shortcut        | Action                   |
| --------------- | ------------------------ |
| `Cmd+D`         | Split right              |
| `Cmd+Shift+D`   | Split down               |
| `Opt+Cmd+Arrow` | Focus pane directionally |
| `Cmd+Shift+H`   | Flash focused panel      |

### Notifications

| Shortcut      | Action                   |
| ------------- | ------------------------ |
| `Cmd+I`       | Show notifications panel |
| `Cmd+Shift+U` | Jump to latest unread    |

### Browser

| Shortcut      | Action                |
| ------------- | --------------------- |
| `Cmd+Shift+L` | Open browser in split |
| `Cmd+L`       | Focus address bar     |
| `Cmd+[`       | Back                  |
| `Cmd+]`       | Forward               |
| `Cmd+R`       | Reload page           |

For a complete list, see [Keyboard Shortcuts](/features/keyboard-shortcuts).

## Next steps

<CardGroup cols={2}>
  <Card title="Workspaces & Tabs" icon="window-restore" href="/features/workspaces-and-tabs">
    Learn advanced workspace management
  </Card>

  <Card title="Notifications" icon="bell" href="/features/notifications">
    Master the notification system
  </Card>

  <Card title="CLI Reference" icon="terminal" href="/automation/cli-reference">
    Explore all CLI commands
  </Card>

  <Card title="Browser Automation" icon="browser" href="/cli/browser">
    Automate the in-app browser
  </Card>
</CardGroup>
