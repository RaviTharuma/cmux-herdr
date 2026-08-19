> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# OpenCode integration

> Configure OpenCode to send notifications through cmux

Integrate OpenCode with cmux to receive visual notifications when the agent needs your attention. Similar to Claude Code integration, OpenCode can trigger notifications that appear as blue rings on workspace tabs.

## How it works

OpenCode uses a hook system to notify external tools when certain events occur. cmux provides a `notify` command that OpenCode hooks can call to create notifications.

## Setup

<Steps>
  <Step title="Verify cmux CLI is available">
    The `cmux` CLI is installed with the app. Verify it's in your PATH:

    ```bash theme={null}
    which cmux
    cmux --version
    ```

    If not found, add cmux to your PATH:

    ```bash theme={null}
    export PATH="/Applications/cmux.app/Contents/MacOS:$PATH"
    ```
  </Step>

  <Step title="Configure OpenCode hooks">
    OpenCode supports custom hooks in its configuration. Add a notification hook that calls cmux:

    Create `~/.opencode/hooks.sh`:

    ```bash theme={null}
    #!/bin/bash
    # OpenCode → cmux notification bridge

    case "$1" in
      agent-waiting)
        cmux notify \
          --title "OpenCode" \
          --subtitle "Waiting for input" \
          --body "$2"
        ;;
      agent-complete)
        cmux notify \
          --title "OpenCode" \
          --subtitle "Task complete" \
          --body "$2"
        ;;
      agent-error)
        cmux notify \
          --title "OpenCode" \
          --subtitle "Error" \
          --body "$2"
        ;;
    esac
    ```

    Make it executable:

    ```bash theme={null}
    chmod +x ~/.opencode/hooks.sh
    ```
  </Step>

  <Step title="Configure OpenCode to use the hook">
    Edit your OpenCode config (typically `~/.opencode/config.json`):

    ```json theme={null}
    {
      "hooks": {
        "onAgentWaiting": "~/.opencode/hooks.sh agent-waiting",
        "onComplete": "~/.opencode/hooks.sh agent-complete",
        "onError": "~/.opencode/hooks.sh agent-error"
      }
    }
    ```
  </Step>

  <Step title="Test the integration">
    Run an OpenCode command and verify that notifications appear in cmux:

    ```bash theme={null}
    opencode "add a hello world function"
    ```

    You should see a notification in the cmux sidebar when OpenCode is waiting.
  </Step>
</Steps>

## Using the notify command

The `cmux notify` command sends a notification to the current workspace (or a specific workspace if targeted).

### Basic usage

```bash theme={null}
cmux notify --title "Agent" --subtitle "Status" --body "Message text"
```

### Options

* `--title`: Notification title (default: "Notification")
* `--subtitle`: Short summary text
* `--body`: Full notification message
* `--workspace <id>`: Target a specific workspace by ID or index
* `--surface <id>`: Target a specific surface/tab within the workspace

<Note>
  If `--workspace` and `--surface` are omitted, cmux routes the notification to the workspace where the command was executed (via `CMUX_WORKSPACE_ID` and `CMUX_SURFACE_ID` environment variables).
</Note>

### Examples

<CodeGroup>
  ```bash Current workspace theme={null}
  # Send to the current workspace
  cmux notify --title "Build" --subtitle "Success" --body "Build completed in 2.3s"
  ```

  ```bash Specific workspace theme={null}
  # Send to workspace 2
  cmux notify --workspace 2 --title "Tests" --body "All tests passed"
  ```

  ```bash Specific surface theme={null}
  # Send to a specific tab/surface
  cmux notify --workspace 1 --surface 3 --title "Deploy" --body "Deployment started"
  ```
</CodeGroup>

## Environment variables

cmux automatically sets these variables in each terminal session:

* `CMUX_WORKSPACE_ID`: The current workspace ID
* `CMUX_SURFACE_ID`: The current surface (tab) ID
* `CMUX_SOCKET_PATH`: Path to the cmux control socket

You can use these in your hook scripts to ensure notifications route to the correct workspace.

## Advanced integration

### Progress indicators

For long-running operations, you can combine notifications with progress indicators:

```bash theme={null}
# Start operation
cmux set-progress 0.0 --label "Building"

# Update progress
cmux set-progress 0.5 --label "Testing"

# Complete
cmux set-progress 1.0 --label "Done"
cmux clear-progress

# Notify on completion
cmux notify --title "Build" --body "Build complete"
```

### Status badges

Add persistent status badges to workspace tabs:

```bash theme={null}
# Show "Running" status
cmux set-status agent_status "Running" --icon "bolt.fill" --color "#4C8DFF"

# Clear status when done
cmux clear-status agent_status
```

### Custom notification scripts

Create reusable notification scripts for common events:

```bash ~/bin/notify-success theme={null}
#!/bin/bash
cmux notify \
  --title "${1:-Success}" \
  --subtitle "$(date +'%H:%M:%S')" \
  --body "${2:-Operation completed}"
```

```bash ~/bin/notify-error theme={null}
#!/bin/bash
cmux notify \
  --title "${1:-Error}" \
  --subtitle "$(date +'%H:%M:%S')" \
  --body "${2:-Operation failed}"
```

Use in your workflows:

```bash theme={null}
my-long-task && notify-success "Build" "Build succeeded" || notify-error "Build" "Build failed"
```

## Viewing notifications

Use the cmux keyboard shortcuts to navigate notifications:

* `Cmd+I`: Open the notifications panel
* `Cmd+Shift+U`: Jump to the latest unread notification

Or use the CLI:

```bash theme={null}
# List all notifications
cmux list-notifications

# List as JSON
cmux list-notifications --json

# Clear all notifications
cmux clear-notifications
```

## Troubleshooting

<AccordionGroup>
  <Accordion title="Notifications not appearing">
    Verify the socket connection:

    ```bash theme={null}
    cmux ping
    ```

    Check the socket path:

    ```bash theme={null}
    echo $CMUX_SOCKET_PATH
    ls -l $CMUX_SOCKET_PATH
    ```

    If the socket doesn't exist, cmux may not be running.
  </Accordion>

  <Accordion title="Permission denied">
    The socket file must be owned by your user. Check:

    ```bash theme={null}
    ls -l /tmp/cmux.sock
    ```

    If it's owned by another user, quit cmux and restart it.
  </Accordion>

  <Accordion title="Wrong workspace receiving notifications">
    Verify the environment variables are set:

    ```bash theme={null}
    echo $CMUX_WORKSPACE_ID
    echo $CMUX_SURFACE_ID
    ```

    You can override them explicitly:

    ```bash theme={null}
    export CMUX_WORKSPACE_ID=workspace:2
    cmux notify --title "Test" --body "Message"
    ```
  </Accordion>
</AccordionGroup>

## Related commands

* [`cmux notify`](/cli/notifications) - Send notifications to workspaces
* [`cmux list-notifications`](/cli/notifications) - View pending notifications
* [CLI Reference](/automation/cli-reference) - Complete command reference including status and progress commands
