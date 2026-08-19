> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Notification Commands

> Send and manage in-app notifications for AI coding agents

Create and manage cmux notifications. These are designed for AI coding agents to communicate progress, errors, and important events.

## notify

Send a notification to a workspace or surface.

```bash theme={null}
cmux notify --title <text> [--subtitle <text>] [--body <text>] [--workspace <id>] [--surface <id>]
```

**Flags:**

<ParamField path="--title" type="string">
  Notification title (default: "Notification")
</ParamField>

<ParamField path="--subtitle" type="string">
  Notification subtitle (default: empty)
</ParamField>

<ParamField path="--body" type="string">
  Notification body text (default: empty)
</ParamField>

<ParamField path="--workspace" type="string">
  Target workspace (defaults to `$CMUX_WORKSPACE_ID`)
</ParamField>

<ParamField path="--surface" type="string">
  Target surface (defaults to `$CMUX_SURFACE_ID`)
</ParamField>

**Examples:**

<CodeGroup>
  ```bash Simple Notification theme={null}
  cmux notify --title "Build Complete"
  ```

  ```bash With Subtitle and Body theme={null}
  cmux notify --title "Tests Passed" --subtitle "All 42 tests" --body "Build #123 completed successfully"
  ```

  ```bash To Specific Workspace theme={null}
  cmux notify --title "Deployment Ready" --workspace workspace:2
  ```

  ```bash To Specific Surface theme={null}
  cmux notify --title "Task Complete" --surface surface:3 --body "File processing finished"
  ```
</CodeGroup>

**Output:**

```
OK
```

**Note:** If both `--workspace` and `--surface` are omitted, the notification targets the current workspace/surface from environment variables. The workspace ID is resolved to its UUID for routing.

## list-notifications

List all notifications in the app.

```bash theme={null}
cmux list-notifications [--json]
```

**Flags:**

<ParamField path="--json" type="boolean">
  Output results in JSON format
</ParamField>

**Output (text):**

```
<id>:<notification_id>|<workspace_id>|<surface_id>|<read|unread>|<title>|<subtitle>|<body>
```

**Example text output:**

```
1:notif-abc|workspace:1|surface:2|unread|Build Failed|Error in main.swift|Line 42: Syntax error
2:notif-def|workspace:1|none|read|Tests Passed|All tests|42 tests completed
```

**Output (JSON with --json):**

```json theme={null}
[
  {
    "id": "notif-abc",
    "workspace_id": "workspace:1",
    "surface_id": "surface:2",
    "is_read": false,
    "title": "Build Failed",
    "subtitle": "Error in main.swift",
    "body": "Line 42: Syntax error"
  },
  {
    "id": "notif-def",
    "workspace_id": "workspace:1",
    "surface_id": null,
    "is_read": true,
    "title": "Tests Passed",
    "subtitle": "All tests",
    "body": "42 tests completed"
  }
]
```

**Note:** When there are no notifications, the text output is:

```
No notifications
```

## clear-notifications

Clear all notifications from the app.

```bash theme={null}
cmux clear-notifications
```

**Output:**

```
OK
```

## Jump to Unread

While there's no dedicated CLI command for jumping to unread notifications, you can query unread notifications using `list-notifications --json` and filter the results:

```bash theme={null}
# Get unread notifications (requires jq)
cmux list-notifications --json | jq '.[] | select(.is_read == false)'
```

**Example:**

```bash theme={null}
# Count unread notifications
cmux list-notifications --json | jq '[.[] | select(.is_read == false)] | length'
```

## Use Cases

Notifications are designed for AI coding agent workflows:

### Build/Test Results

```bash theme={null}
cmux notify --title "Build Complete" --subtitle "✓ All tests passed" --body "Build #123 finished in 42s"
```

### Error Reporting

```bash theme={null}
cmux notify --title "Compilation Error" --subtitle "main.swift:42" --body "Expected ';' after expression"
```

### Long-Running Tasks

```bash theme={null}
# Start notification
cmux notify --title "Processing" --subtitle "Analyzing codebase" --body "This may take a few minutes..."

# Completion notification
cmux notify --title "Analysis Complete" --subtitle "Found 12 issues" --body "Review results in workspace:3"
```

### Git Operations

```bash theme={null}
cmux notify --title "Push Complete" --subtitle "origin/main" --body "3 commits pushed successfully"
```

### File Watching

```bash theme={null}
cmux notify --title "File Changed" --subtitle "src/app.ts" --body "Hot reload triggered"
```

## Notification Lifecycle

1. **Creation**: Notifications are created with `notify` and linked to a workspace (and optionally a surface)
2. **Display**: They appear in the cmux notification center and may trigger UI indicators
3. **Reading**: Users can mark notifications as read by interacting with them
4. **Persistence**: Notifications persist across app restarts
5. **Clearing**: Use `clear-notifications` to remove all notifications

## Environment Variables

The `notify` command uses these environment variables for context when flags are omitted:

<ParamField path="CMUX_WORKSPACE_ID" type="string">
  Current workspace ID (set automatically in terminal sessions)
</ParamField>

<ParamField path="CMUX_SURFACE_ID" type="string">
  Current surface ID (set automatically in terminal sessions)
</ParamField>

## Global Flags

These flags can be used with notification commands:

<ParamField path="--socket" type="string">
  Path to the cmux socket (default: `/tmp/cmux.sock` or `$CMUX_SOCKET_PATH`)
</ParamField>

<ParamField path="--password" type="string">
  Socket authentication password (or use `$CMUX_SOCKET_PASSWORD`)
</ParamField>

<ParamField path="--json" type="boolean">
  Output results in JSON format (for `list-notifications`)
</ParamField>
