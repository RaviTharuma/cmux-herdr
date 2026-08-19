# [#](#title)Notifications

# [#](https://cmux-docs-release.vercel.app/docs/notifications#title)Notifications

cmux supports desktop notifications, allowing AI agents and scripts to alert you when they need attention.

## [#](https://cmux-docs-release.vercel.app/docs/notifications#lifecycle)Lifecycle

1.  Received: notification appears in panel, desktop alert fires (if not suppressed)
2.  Unread: badge shown on workspace tab
3.  Read: cleared when you view that workspace
4.  Cleared: removed from panel

### [#](https://cmux-docs-release.vercel.app/docs/notifications#suppression)Suppression

Desktop alerts are suppressed when:

-   The cmux window is focused
-   The specific workspace sending the notification is active
-   The notification panel is open

### [#](https://cmux-docs-release.vercel.app/docs/notifications#notification-panel)Notification panel

Press `⌘⇧I` to open the notification panel. Click a notification to jump to that workspace. Press `⌘⇧U` to jump directly to the workspace with the most recent unread notification.

## [#](https://cmux-docs-release.vercel.app/docs/notifications#custom-command)Custom command

Run a shell command every time a notification is scheduled. Set it in Settings > App > Notification Command. The command runs via /bin/sh -c with these environment variables:

| Variable | Description |
| --- | --- |
| `CMUX_NOTIFICATION_TITLE` | Notification title (workspace name or app name) |
| `CMUX_NOTIFICATION_SUBTITLE` | Notification subtitle |
| `CMUX_NOTIFICATION_BODY` | Notification body text |

Examples

```
# Text-to-speech
say "$CMUX_NOTIFICATION_TITLE"

# Custom sound file
afplay /path/to/sound.aiff

# Log to file
echo "$CMUX_NOTIFICATION_TITLE: $CMUX_NOTIFICATION_BODY" >> ~/notifications.log
```

The command runs independently of the system sound picker. Set the picker to "None" to use only the custom command, or keep both for a system sound plus a custom action.

## [#](https://cmux-docs-release.vercel.app/docs/notifications#notification-hooks)Notification hooks

`cmux.json` can define notification hooks that receive every notification policy as JSON on stdin. Each hook returns JSON on stdout. Hooks are off by default; cmux only runs them when `notifications.hooks` contains at least one enabled hook. cmux applies the returned notification text and effects, so hooks can filter banners, keep or skip sidebar history, run sounds, or stop later hooks.

cmux.json

```
{
  "notifications": {
    "hooks": [
      {
        "id": "quiet-docs",
        "command": "sed 's/"desktop":true/"desktop":false/'",
        "timeoutSeconds": 20
      }
    ]
  }
}
```

Hook input and output

```
{
  "version": 1,
  "notification": {
    "workspaceId": "3B3F0D83-...",
    "surfaceId": "7E9C1A02-...",
    "title": "Codex",
    "subtitle": "Waiting",
    "body": "Agent needs input"
  },
  "context": {
    "cwd": "/path/to/project",
    "configPath": "/path/to/project/.cmux/cmux.json",
    "hookId": "quiet-docs",
    "appFocused": false,
    "focusedPanel": false
  },
  "effects": {
    "record": true,
    "markUnread": true,
    "reorderWorkspace": true,
    "desktop": true,
    "sound": true,
    "command": true,
    "paneFlash": true
  }
}
```

Hooks are inherited from global `~/.config/cmux/cmux.json` and project `.cmux/cmux.json` files from parent directories to the current workspace. Project hooks use the same trust prompt as other project `cmux.json` commands before they run. Feed approval banners also pass through these hooks; disabling `desktop` suppresses the native banner while keeping the Feed item available in cmux. Set `notifications.hooksMode` to `replace` in a project config to ignore inherited hooks. If a hook fails, times out, or returns invalid JSON, cmux uses the default notification behavior and posts a hook failure alert.

## [#](https://cmux-docs-release.vercel.app/docs/notifications#sending)Sending notifications

### [#](https://cmux-docs-release.vercel.app/docs/notifications#cli-usage)CLI

```
cmux notify --title "Task Complete" --body "Your build finished"
cmux notify --title "Claude Code" --subtitle "Waiting" --body "Agent needs input"
```

### [#](https://cmux-docs-release.vercel.app/docs/notifications#osc777-title)OSC 777 (simple)

The RXVT protocol uses a fixed format with title and body:

```
printf '\e]777;notify;My Title;Message body here\a'
```

Shell function

```
notify_osc777() {
    local title="$1"
    local body="$2"
    printf '\e]777;notify;%s;%s\a' "$title" "$body"
}

notify_osc777 "Build Complete" "All tests passed"
```

### [#](https://cmux-docs-release.vercel.app/docs/notifications#osc99-title)OSC 99 (rich)

The Kitty protocol supports subtitles and notification IDs:

```
# Format: ESC ] 99 ; <params> ; <payload> ESC \

# Simple notification
printf '\e]99;i=1;e=1;d=0:Hello World\e\\'

# With title, subtitle, and body
printf '\e]99;i=1;e=1;d=0;p=title:Build Complete\e\\'
printf '\e]99;i=1;e=1;d=0;p=subtitle:Project X\e\\'
printf '\e]99;i=1;e=1;d=1;p=body:All tests passed\e\\'
```

| Feature | OSC 99 | OSC 777 |
| --- | --- | --- |
| Title + body | Yes | Yes |
| Subtitle | Yes | No |
| Notification ID | Yes | No |
| Complexity | Higher | Lower |

Use OSC 777 for simple notifications. Use OSC 99 when you need subtitles or notification IDs. Use the CLI (cmux notify) for the easiest integration.

## [#](https://cmux-docs-release.vercel.app/docs/notifications#claude-code-hooks)Claude Code hooks

cmux integrates with [Claude Code](https://docs.anthropic.com/en/docs/claude-code) via hooks to notify you when tasks complete.

### [#](https://cmux-docs-release.vercel.app/docs/notifications#create-hook-script)1\. Create the hook script

~/.claude/hooks/cmux-notify.sh

```
#!/bin/bash
# Skip if not in cmux
[ -S /tmp/cmux.sock ] || exit 0

EVENT=$(cat)
EVENT_TYPE=$(echo "$EVENT" | jq -r '.hook_event_name // "unknown"')
TOOL=$(echo "$EVENT" | jq -r '.tool_name // ""')

case "$EVENT_TYPE" in
    "Stop")
        cmux notify --title "Claude Code" --body "Session complete"
        ;;
    "PostToolUse")
        [ "$TOOL" = "Task" ] && cmux notify --title "Claude Code" --body "Agent finished"
        ;;
esac
```

```
chmod +x ~/.claude/hooks/cmux-notify.sh
```

### [#](https://cmux-docs-release.vercel.app/docs/notifications#configure-claude)2\. Configure Claude Code

~/.claude/settings.json

```
{
  "hooks": {
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "~/.claude/hooks/cmux-notify.sh"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Task",
        "hooks": [
          {
            "type": "command",
            "command": "~/.claude/hooks/cmux-notify.sh"
          }
        ]
      }
    ]
  }
}
```

Restart Claude Code to apply the hooks.

## [#](https://cmux-docs-release.vercel.app/docs/notifications#copilot-cli-hooks)GitHub Copilot CLI

Copilot CLI supports [hooks](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/coding-agent/use-hooks) that run shell commands at lifecycle events like prompt submission, agent stop, and errors.

~/.copilot/config.json

```
{
  "hooks": {
    "userPromptSubmitted": [
      {
        "type": "command",
        "bash": "if command -v cmux &>/dev/null; then cmux set-status copilot_cli Running; fi",
        "timeoutSec": 3
      }
    ],
    "agentStop": [
      {
        "type": "command",
        "bash": "if command -v cmux &>/dev/null; then cmux notify --title 'Copilot CLI' --body 'Done'; cmux set-status copilot_cli Idle; fi",
        "timeoutSec": 5
      }
    ],
    "errorOccurred": [
      {
        "type": "command",
        "bash": "if command -v cmux &>/dev/null; then cmux notify --title 'Copilot CLI' --subtitle 'Error' --body 'An error occurred'; cmux set-status copilot_cli Error; fi",
        "timeoutSec": 5
      }
    ],
    "sessionEnd": [
      {
        "type": "command",
        "bash": "if command -v cmux &>/dev/null; then cmux clear-status copilot_cli; fi",
        "timeoutSec": 3
      }
    ]
  }
}
```

For repo-level hooks, create a .github/hooks/notify.json file with the same structure:

.github/hooks/notify.json

```
{
  "version": 1,
  "hooks": {
    "userPromptSubmitted": [ ... ],
    "agentStop": [ ... ]
  }
}
```

## [#](https://cmux-docs-release.vercel.app/docs/notifications#integration-examples)Integration examples

### [#](https://cmux-docs-release.vercel.app/docs/notifications#notify-after-long)Notify after long command

~/.zshrc

```
# Add to your shell config
notify-after() {
  "$@"
  local exit_code=$?
  if [ $exit_code -eq 0 ]; then
    cmux notify --title "✓ Command Complete" --body "$1"
  else
    cmux notify --title "✗ Command Failed" --body "$1 (exit $exit_code)"
  fi
  return $exit_code
}

# Usage: notify-after npm run build
```

### [#](https://cmux-docs-release.vercel.app/docs/notifications#python)Python

python

```
import sys

def notify(title: str, body: str):
    """Send OSC 777 notification."""
    sys.stdout.write(f'\x1b]777;notify;{title};{body}\x07')
    sys.stdout.flush()

notify("Script Complete", "Processing finished")
```

### [#](https://cmux-docs-release.vercel.app/docs/notifications#nodejs)Node.js

node

```
function notify(title, body) {
  process.stdout.write(`\x1b]777;notify;${title};${body}\x07`);
}

notify('Build Done', 'webpack finished');
```

### [#](https://cmux-docs-release.vercel.app/docs/notifications#tmux-passthrough)tmux passthrough

If using tmux inside cmux, enable passthrough:

.tmux.conf

```
set -g allow-passthrough on
```

```
printf '\ePtmux;\e\e]777;notify;Title;Body\a\e\\'
```

[Skills](https://cmux-docs-release.vercel.app/docs/skills) [SSH](https://cmux-docs-release.vercel.app/docs/ssh)

Canonical: https://cmux-docs-release.vercel.app/docs/notifications
