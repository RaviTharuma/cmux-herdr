> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Notifications

> Visual notification system for AI coding agents with notification rings, badges, and a unified panel

cmux provides a comprehensive notification system designed specifically for managing multiple AI coding agents in parallel. The system helps you track which agents need your attention across workspaces, tabs, and split panes.

## Notification rings

When a coding agent sends a notification (via OSC sequences or the `cmux notify` CLI), the active pane displays a blue ring around it, making it immediately visible which terminal needs attention.

<Info>
  Notification rings appear on panes when agents use terminal sequences OSC 9, OSC 99, or OSC 777, or when you explicitly call `cmux notify` from your agent hooks.
</Info>

The sidebar tabs also light up with notification indicators showing:

* The latest notification text
* Git branch and working directory
* Linked PR status and number
* Listening ports

## Notification panel

Access all pending notifications in a unified panel where you can:

* See all unread notifications across all workspaces
* View notification title, subtitle, and body
* Jump directly to the workspace/pane that triggered the notification
* Mark notifications as read or clear them

<Note>
  The notification panel preserves context from each notification, including which workspace and surface it came from, so you can quickly navigate to the right place.
</Note>

## Triggering notifications

### From terminal sequences

cmux automatically picks up standard terminal notification sequences:

```bash theme={null}
# OSC 9 (iTerm2 format)
printf "\033]9;%s\007" "Task complete"

# OSC 99 (custom format)
printf "\033]99;%s\007" "Agent waiting for input"

# OSC 777 (tmux format)
printf "\033]777;notify;%s;%s\007" "Title" "Body"
```

### Using the CLI

The `cmux notify` command lets you wire notifications into agent hooks:

```bash theme={null}
cmux notify "Claude is waiting for your input"

# With title and subtitle
cmux notify --title "Build Complete" --subtitle "main.rs" "Compiled successfully"
```

<Note>
  Integrate `cmux notify` into your Claude Code, OpenCode, or other agent hooks to get notified exactly when agents need you.
</Note>

## Keyboard shortcuts

| Shortcut                               | Action                             |
| -------------------------------------- | ---------------------------------- |
| <kbd>⌘</kbd> <kbd>I</kbd>              | Show notifications panel           |
| <kbd>⌘</kbd> <kbd>⇧</kbd> <kbd>U</kbd> | Jump to latest unread notification |

## Notification behavior

### Auto-dismiss on focus

Notifications are automatically marked as read when:

* The app is focused AND
* The workspace tab containing the notification is active AND
* The surface (pane) that triggered the notification is focused

This prevents notification noise when you're already looking at the agent that's waiting.

### Workspace auto-reorder

When a notification arrives, the workspace automatically moves to the top of the sidebar (if workspace auto-reorder is enabled in settings). This keeps active agents visible even when you have many workspaces open.

### Dock badge

The dock icon displays the total unread notification count. You can disable this in Settings → Notifications if you prefer a cleaner dock.

<Info>
  For tagged debug builds (e.g., `./scripts/reload.sh --tag feature-test`), the dock badge shows the tag name alongside the unread count, like `feature-test:3`.
</Info>

## System notifications

cmux requests macOS notification permissions on first use. If you denied permissions initially:

1. cmux will prompt you to enable notifications when the next notification arrives
2. Click "Open Settings" to jump to System Settings
3. Enable notifications for cmux

The notification body in macOS Notification Center matches what the agent sends, giving you context even when the app is in the background.

## Notification store

All notifications are kept in memory during the session. The notification store:

* Tracks read/unread state per notification
* Maintains notification history for the current session
* Automatically deduplicates notifications from the same surface
* Preserves the most recent notification per workspace for sidebar display

Notifications are cleared when you close the workspace or quit the app.
