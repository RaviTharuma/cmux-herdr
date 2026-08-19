> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Custom agent integrations

> Build custom integrations using the cmux CLI and socket API

Integrate any AI coding agent, CI/CD system, or custom workflow with cmux using the CLI and socket API. This guide shows you how to build custom integrations from scratch.

## Architecture

cmux exposes two interfaces for external integrations:

1. **CLI**: Shell commands like `cmux notify`, `cmux new-workspace`, `cmux send`
2. **Socket API**: JSON-RPC protocol over a Unix domain socket at `/tmp/cmux.sock`

Most integrations use the CLI, which is simpler and more portable. The socket API is useful for high-performance scenarios or when you need fine-grained control.

## Using the CLI

The `cmux` CLI binary is bundled with the app at `/Applications/cmux.app/Contents/MacOS/cmux`.

### Environment variables

cmux sets these variables in every terminal session:

* `CMUX_WORKSPACE_ID`: Current workspace ID (e.g., `workspace:1`)
* `CMUX_SURFACE_ID`: Current surface/tab ID (e.g., `surface:2`)
* `CMUX_SOCKET_PATH`: Path to the control socket (default: `/tmp/cmux.sock`)

Your integration scripts can read these to target the correct workspace.

### Common CLI patterns

<CodeGroup>
  ```bash Send notification theme={null}
  # Notify the current workspace
  cmux notify --title "Agent" --subtitle "Ready" --body "Waiting for input"
  ```

  ```bash Create workspace theme={null}
  # Create a new workspace with a specific working directory
  cmux new-workspace --cwd ~/projects/myapp
  ```

  ```bash Send text to terminal theme={null}
  # Send text to the current surface
  cmux send "npm run build\n"

  # Send to a specific workspace and surface
  cmux send --workspace 2 --surface 1 "git status\n"
  ```

  ```bash Read terminal output theme={null}
  # Read the current terminal screen
  cmux read-screen

  # Read with scrollback (last 50 lines)
  cmux read-screen --lines 50 --scrollback
  ```

  ```bash Control workspaces theme={null}
  # List all workspaces
  cmux list-workspaces

  # Select a workspace
  cmux select-workspace --workspace 3

  # Close a workspace
  cmux close-workspace --workspace 2
  ```
</CodeGroup>

### Example: Simple agent hook

Here's a minimal agent integration that sends a notification when a task completes:

```bash ~/bin/agent-notify theme={null}
#!/bin/bash
# Usage: agent-notify <title> <message>

TITLE="${1:-Agent}"
MESSAGE="${2:-Task complete}"

cmux notify \
  --title "$TITLE" \
  --subtitle "$(date +'%H:%M:%S')" \
  --body "$MESSAGE"
```

Use it in your agent workflow:

```bash theme={null}
my-agent-command && agent-notify "Success" "Task completed" || agent-notify "Error" "Task failed"
```

## Using the socket API

The socket API uses a JSON-RPC-like protocol. Each request is a JSON object sent over a Unix socket, followed by a newline.

### Connection

<Steps>
  <Step title="Connect to the socket">
    Open a connection to `/tmp/cmux.sock` (or the path in `$CMUX_SOCKET_PATH`).

    <Note>
      The socket is owned by your user and only accepts connections from the same user for security.
    </Note>
  </Step>

  <Step title="Send a request">
    Send a JSON object with `id`, `method`, and `params` fields, terminated by a newline:

    ```json theme={null}
    {"id":"req-1","method":"workspace.list","params":{}}
    ```
  </Step>

  <Step title="Read the response">
    The server sends a JSON response:

    ```json theme={null}
    {"ok":true,"result":{"workspaces":[{"id":"...","ref":"workspace:1","title":"main"}]}}
    ```

    Or an error:

    ```json theme={null}
    {"ok":false,"error":{"code":"invalid_params","message":"Missing workspace_id"}}
    ```
  </Step>
</Steps>

### Request format

```json theme={null}
{
  "id": "unique-request-id",
  "method": "workspace.create",
  "params": {
    "cwd": "/path/to/project"
  }
}
```

* `id`: Any unique string (used to match requests to responses)
* `method`: API method name (see below)
* `params`: Method-specific parameters (optional)

### Response format

**Success:**

```json theme={null}
{
  "ok": true,
  "result": {
    "workspace_id": "abc-123",
    "workspace_ref": "workspace:1"
  }
}
```

**Error:**

```json theme={null}
{
  "ok": false,
  "error": {
    "code": "not_found",
    "message": "Workspace not found"
  }
}
```

### Available methods

<AccordionGroup>
  <Accordion title="Workspace methods">
    * `workspace.list` - List all workspaces
    * `workspace.create` - Create a new workspace (params: `cwd`)
    * `workspace.select` - Select a workspace (params: `workspace_id`)
    * `workspace.close` - Close a workspace (params: `workspace_id`)
    * `workspace.rename` - Rename a workspace (params: `workspace_id`, `title`)
    * `workspace.current` - Get current workspace ID
  </Accordion>

  <Accordion title="Surface methods">
    * `surface.list` - List surfaces in a workspace (params: `workspace_id`)
    * `surface.create` - Create a new surface/tab (params: `workspace_id`, optional `type`, `url`)
    * `surface.focus` - Focus a surface (params: `workspace_id`, `surface_id`)
    * `surface.close` - Close a surface (params: `workspace_id`, `surface_id`)
    * `surface.send_text` - Send text to a surface (params: `workspace_id`, `surface_id`, `text`)
    * `surface.send_key` - Send key to a surface (params: `workspace_id`, `surface_id`, `key`)
    * `surface.read_text` - Read terminal text (params: `workspace_id`, `surface_id`, optional `lines`, `scrollback`)
  </Accordion>

  <Accordion title="Pane methods">
    * `pane.list` - List panes in a workspace (params: `workspace_id`)
    * `pane.create` - Create a new pane (params: `workspace_id`, `direction`)
    * `pane.focus` - Focus a pane (params: `workspace_id`, `pane_id`)
  </Accordion>

  <Accordion title="Window methods">
    * `window.list` - List all windows
    * `window.current` - Get current window ID
    * `window.focus` - Focus a window (params: `window_id`)
  </Accordion>

  <Accordion title="System methods">
    * `system.capabilities` - Get API capabilities
    * `system.identify` - Get current context (workspace, surface, pane)
  </Accordion>
</AccordionGroup>

### Example: Python integration

Here's a Python class for interacting with the cmux socket:

```python ~/lib/cmux_client.py theme={null}
import json
import socket
import uuid

class CmuxClient:
    def __init__(self, socket_path="/tmp/cmux.sock"):
        self.socket_path = socket_path
        self.sock = None

    def connect(self):
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.connect(self.socket_path)

    def close(self):
        if self.sock:
            self.sock.close()

    def request(self, method, params=None):
        req = {
            "id": str(uuid.uuid4()),
            "method": method,
            "params": params or {}
        }
        message = json.dumps(req) + "\n"
        self.sock.sendall(message.encode("utf-8"))

        # Read response (simplified - real impl should handle buffering)
        response_data = b""
        while b"\n" not in response_data:
            chunk = self.sock.recv(4096)
            if not chunk:
                raise ConnectionError("Socket closed")
            response_data += chunk

        response = json.loads(response_data.decode("utf-8"))
        if not response.get("ok"):
            error = response.get("error", {})
            raise Exception(f"{error.get('code')}: {error.get('message')}")
        return response.get("result", {})

    # Convenience methods
    def list_workspaces(self):
        return self.request("workspace.list")

    def create_workspace(self, cwd):
        return self.request("workspace.create", {"cwd": cwd})

    def send_text(self, workspace_id, surface_id, text):
        return self.request("surface.send_text", {
            "workspace_id": workspace_id,
            "surface_id": surface_id,
            "text": text
        })

    def read_screen(self, workspace_id, surface_id, lines=None):
        params = {
            "workspace_id": workspace_id,
            "surface_id": surface_id
        }
        if lines:
            params["lines"] = lines
            params["scrollback"] = True
        return self.request("surface.read_text", params)
```

Usage:

```python theme={null}
from cmux_client import CmuxClient

client = CmuxClient()
client.connect()

# List workspaces
result = client.list_workspaces()
for ws in result["workspaces"]:
    print(f"{ws['ref']}: {ws['title']}")

# Create a new workspace
result = client.create_workspace("/home/user/project")
ws_id = result["workspace_ref"]

# Send a command
client.send_text(ws_id, "surface:1", "ls -la\n")

client.close()
```

## Building an agent integration

Here's a complete example of integrating a custom AI agent with cmux:

<Steps>
  <Step title="Create a hook script">
    ```bash ~/.myagent/cmux-hook.sh theme={null}
    #!/bin/bash
    # MyAgent → cmux integration

    EVENT="$1"
    shift

    case "$EVENT" in
      start)
        cmux set-status myagent "Running" --icon "bolt.fill" --color "#00FF00"
        ;;
      waiting)
        cmux notify --title "MyAgent" --subtitle "Input needed" --body "$*"
        cmux set-status myagent "Waiting" --icon "bell.fill" --color "#FF9500"
        ;;
      complete)
        cmux notify --title "MyAgent" --subtitle "Complete" --body "$*"
        cmux clear-status myagent
        ;;
      error)
        cmux notify --title "MyAgent" --subtitle "Error" --body "$*"
        cmux clear-status myagent
        ;;
    esac
    ```

    Make it executable:

    ```bash theme={null}
    chmod +x ~/.myagent/cmux-hook.sh
    ```
  </Step>

  <Step title="Call hooks from your agent">
    In your agent code:

    ```python theme={null}
    import subprocess

    def notify_cmux(event, message=""):
        subprocess.run([
            os.path.expanduser("~/.myagent/cmux-hook.sh"),
            event,
            message
        ])

    # Agent lifecycle
    notify_cmux("start")
    try:
        result = run_agent_task()
        notify_cmux("complete", f"Generated {result.files_changed} changes")
    except NeedsInputError as e:
        notify_cmux("waiting", str(e))
    except Exception as e:
        notify_cmux("error", str(e))
    ```
  </Step>

  <Step title="Test the integration">
    Run your agent from inside a cmux workspace:

    ```bash theme={null}
    myagent "implement feature X"
    ```

    You should see:

    * Status indicator updates on the workspace tab
    * Notifications when the agent needs input
    * Blue notification rings on the sidebar
  </Step>
</Steps>

## Best practices

<CardGroup cols={2}>
  <Card title="Use environment variables" icon="code">
    Always read `CMUX_WORKSPACE_ID` and `CMUX_SURFACE_ID` to target the correct workspace instead of hardcoding IDs.
  </Card>

  <Card title="Handle missing socket gracefully" icon="circle-exclamation">
    Check if cmux is running before sending commands. Fail gracefully if the socket doesn't exist.
  </Card>

  <Card title="Sanitize notification content" icon="shield">
    Strip newlines and control characters from notification text to avoid breaking the protocol.
  </Card>

  <Card title="Use refs instead of UUIDs" icon="hashtag">
    Workspace/surface refs like `workspace:1` are more stable and human-readable than UUIDs.
  </Card>
</CardGroup>

## Security considerations

* The socket at `/tmp/cmux.sock` is owned by your user and only accepts connections from the same UID
* cmux verifies socket ownership before connecting to prevent fake-socket attacks
* No authentication is required for local socket connections (assumes filesystem permissions provide sufficient security)
* If you need stricter access control, you can enable password-protected socket access (see `--password` flag in the CLI docs)

## Troubleshooting

<AccordionGroup>
  <Accordion title="Socket not found">
    Ensure cmux is running:

    ```bash theme={null}
    ps aux | grep cmux
    ls -l /tmp/cmux.sock
    ```

    If the app isn't running, launch it and wait for the socket to appear.
  </Accordion>

  <Accordion title="Permission denied">
    Check socket ownership:

    ```bash theme={null}
    ls -l /tmp/cmux.sock
    ```

    The socket must be owned by your user. If it's owned by another user, quit cmux and restart it.
  </Accordion>

  <Accordion title="JSON parse errors">
    Ensure your JSON is valid and terminated with a newline:

    ```bash theme={null}
    echo '{"id":"1","method":"ping","params":{}}' | nc -U /tmp/cmux.sock
    ```

    Use a JSON linter if you're constructing requests manually.
  </Accordion>

  <Accordion title="Command not found">
    Add cmux to your PATH:

    ```bash theme={null}
    export PATH="/Applications/cmux.app/Contents/MacOS:$PATH"
    ```

    Add this to your shell config (`~/.zshrc` or `~/.bashrc`) to persist it.
  </Accordion>
</AccordionGroup>

## Further reading

* [CLI Reference](/automation/cli-reference) - Complete CLI command reference
* [Socket API](/automation/socket-api) - JSON-RPC protocol documentation
* [Claude Code integration](/integrations/claude-code) - Example of a full hook-based integration
