> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Claude Code integration

> Get real-time notifications from Claude Code sessions in cmux

Integrate Claude Code with cmux to receive visual notifications when Claude needs your input. When a coding session is waiting for you, the workspace tab lights up with a blue notification ring.

## How it works

Claude Code can trigger hooks at key lifecycle events:

* **session-start**: When a new coding session begins
* **notification**: When Claude needs your attention (waiting for input, approval, etc.)
* **stop**: When the session ends

The `cmux claude-hook` command connects these events to cmux's notification system, workspace status indicators, and session tracking.

## Setup

<Steps>
  <Step title="Verify cmux CLI is available">
    The `cmux` CLI is installed automatically with the cmux app. Check that it's accessible:

    ```bash theme={null}
    which cmux
    cmux --version
    ```

    If the command isn't found, add cmux to your PATH:

    ```bash theme={null}
    export PATH="/Applications/cmux.app/Contents/MacOS:$PATH"
    ```
  </Step>

  <Step title="Configure Claude Code hooks">
    Create or edit `~/.claude/hooks.json` to wire Claude lifecycle events to cmux:

    ```json theme={null}
    {
      "session-start": "cmux claude-hook session-start",
      "notification": "cmux claude-hook notification",
      "stop": "cmux claude-hook stop"
    }
    ```

    <Note>
      Claude Code will pipe event metadata as JSON to stdin when calling these hooks.
    </Note>
  </Step>

  <Step title="Start a Claude Code session">
    Launch Claude Code from inside cmux:

    ```bash theme={null}
    claude
    ```

    When Claude needs input, you'll see:

    * A blue notification ring around the workspace tab
    * The sidebar badge updates with pending notifications
    * Status indicator shows "Needs input" with a bell icon
  </Step>
</Steps>

## Session tracking

cmux tracks Claude Code sessions using the `ClaudeHookSessionStore` mechanism. This associates each Claude session ID with a specific cmux workspace and surface.

### How session tracking works

1. When `session-start` fires, cmux stores the mapping:
   ```
   session_id → workspace_id + surface_id + cwd
   ```

2. On subsequent `notification` events, cmux looks up the session ID and routes notifications to the correct workspace/surface — even if you've switched tabs or windows.

3. When `stop` fires, cmux removes the session from the store and clears the status indicator.

Session state is persisted to `~/.cmuxterm/claude-hook-sessions.json` and pruned after 7 days of inactivity.

<Tip>
  You can override session routing with explicit flags:

  ```bash theme={null}
  cmux claude-hook notification --workspace 2 --surface 1
  ```
</Tip>

## Command reference

### session-start

Triggered when a Claude Code session begins. Sets the workspace status to "Running" with a bolt icon.

```bash theme={null}
cmux claude-hook session-start [--workspace <id>] [--surface <id>]
```

Reads from stdin:

```json theme={null}
{
  "session_id": "abc123",
  "cwd": "/path/to/project"
}
```

### notification

Triggered when Claude needs attention. Creates a notification and updates the workspace status to "Needs input" with a bell icon.

```bash theme={null}
cmux claude-hook notification [--workspace <id>] [--surface <id>]
```

Reads from stdin:

```json theme={null}
{
  "session_id": "abc123",
  "notification": {
    "title": "Waiting for input",
    "body": "Claude is waiting for your response"
  }
}
```

### stop

Triggered when a Claude Code session ends. Clears the workspace status indicator and removes the session from the store.

```bash theme={null}
cmux claude-hook stop [--workspace <id>] [--surface <id>]
```

Reads from stdin:

```json theme={null}
{
  "session_id": "abc123"
}
```

## Advanced usage

### Multiple workspaces

If you run Claude Code in multiple workspaces, cmux automatically routes notifications to the correct workspace based on session ID.

### Custom hook scripts

You can wrap the `cmux claude-hook` command in a custom script for logging or additional automation:

```bash ~/.claude/my-hook.sh theme={null}
#!/bin/bash
log_event() {
  echo "[$(date)] $1" >> ~/.claude/hook.log
}

log_event "Claude hook: $1"
cmux claude-hook "$@"
```

Then update `hooks.json`:

```json theme={null}
{
  "notification": "~/.claude/my-hook.sh notification"
}
```

### Environment variables

* `CMUX_SOCKET_PATH`: Override the default socket path (default: `/tmp/cmux.sock`)
* `CMUX_WORKSPACE_ID`: Pre-set the target workspace ID
* `CMUX_SURFACE_ID`: Pre-set the target surface ID
* `CMUX_CLAUDE_HOOK_STATE_PATH`: Override session store location (default: `~/.cmuxterm/claude-hook-sessions.json`)

## Troubleshooting

<AccordionGroup>
  <Accordion title="Notifications not appearing">
    Check that cmux is running and the socket is accessible:

    ```bash theme={null}
    ls -l /tmp/cmux.sock
    ```

    Test the connection manually:

    ```bash theme={null}
    cmux ping
    ```

    If the socket doesn't exist, launch cmux and try again.
  </Accordion>

  <Accordion title="Notifications appear in wrong workspace">
    The session store may have stale mappings. Clear it:

    ```bash theme={null}
    rm ~/.cmuxterm/claude-hook-sessions.json
    ```

    Or explicitly target a workspace:

    ```bash theme={null}
    export CMUX_WORKSPACE_ID=workspace:1
    ```
  </Accordion>

  <Accordion title="Hooks not firing">
    Verify your hooks.json syntax:

    ```bash theme={null}
    cat ~/.claude/hooks.json | jq .
    ```

    Check Claude Code hook execution with a test script:

    ```bash theme={null}
    echo '{"session_id":"test"}' | cmux claude-hook notification
    ```
  </Accordion>
</AccordionGroup>

## Related commands

* [`cmux notify`](/cli/notifications) - Send custom notifications to any workspace
* [`cmux list-notifications`](/cli/notifications) - View all pending notifications
* [CLI Reference](/automation/cli-reference) - Complete command reference
