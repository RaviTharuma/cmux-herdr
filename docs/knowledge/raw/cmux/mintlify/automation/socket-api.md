> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Socket API

> JSON-RPC protocol for direct socket communication with cmux

The cmux socket API is a JSON-RPC protocol over a Unix domain socket. It provides programmatic control over cmux without spawning CLI subprocesses.

## Connection

### Socket path

Default socket locations:

* Production: `/tmp/cmux.sock`
* Debug build: `/tmp/cmux-debug.sock`
* Nightly: `/tmp/cmux-nightly.sock`
* Staging: `/tmp/cmux-staging.sock`

Override via `CMUX_SOCKET_PATH` environment variable.

### Security

Socket permissions: `0600` (owner-only) in all modes except `allowAll`.

**Socket ownership validation:**
The socket must be owned by the current user. Connection attempts to sockets owned by other users are rejected to prevent fake-socket attacks.

### Authentication

Depending on the configured socket mode:

1. **Off**: Socket not available
2. **cmux processes only** (default): Only processes with cmux ancestry can connect
3. **Automation mode**: Any local process from the same macOS user
4. **Password mode**: Requires password authentication via `auth` command
5. **Full open access**: Any local process (unsafe)

**Password authentication:**

If password mode is enabled, send an `auth` command before any other requests:

```json theme={null}
auth <password>
```

Password sources (in order of precedence):

1. `--password` CLI flag
2. `CMUX_SOCKET_PASSWORD` environment variable
3. Password file: `~/Library/Application Support/cmux/socket-control-password`

Example:

```bash theme={null}
echo "auth my-secret-password" | nc -U /tmp/cmux.sock
```

## Protocol

### V2 JSON-RPC format

All modern commands use JSON-RPC requests and responses.

**Request:**

```json theme={null}
{
  "id": "<uuid>",
  "method": "<method-name>",
  "params": { ... }
}
```

**Success response:**

```json theme={null}
{
  "ok": true,
  "result": { ... }
}
```

**Error response:**

```json theme={null}
{
  "ok": false,
  "error": {
    "code": "<error-code>",
    "message": "<error-message>"
  }
}
```

### V1 legacy format

Older commands use plain text requests:

```
<command> <args>\n
```

Responses:

* Success: `OK` or data
* Error: `ERROR: <message>`

<Note>
  New integrations should use V2 methods. V1 commands are maintained for backward compatibility.
</Note>

## System methods

### system.capabilities

Get supported API capabilities:

```json theme={null}
{"id":"1","method":"system.capabilities","params":{}}
```

### system.identify

Get current context information:

```json theme={null}
{"id":"2","method":"system.identify","params":{
  "caller": {
    "workspace_id": "<uuid>",
    "surface_id": "<uuid>"
  }
}}
```

**Response:**

```json theme={null}
{
  "ok": true,
  "result": {
    "window_id": "...",
    "window_ref": "window:1",
    "workspace_id": "...",
    "workspace_ref": "workspace:2",
    "focused": {
      "pane_id": "...",
      "pane_ref": "pane:1",
      "surface_id": "...",
      "surface_ref": "surface:3"
    }
  }
}
```

## Window methods

### window\.list

List all windows:

```json theme={null}
{"id":"3","method":"window.list","params":{}}
```

### window\.current

Get current window:

```json theme={null}
{"id":"4","method":"window.current","params":{}}
```

### window\.focus

Focus a window:

```json theme={null}
{"id":"5","method":"window.focus","params":{
  "window_id": "window:1"
}}
```

## Workspace methods

### workspace.list

List workspaces:

```json theme={null}
{"id":"6","method":"workspace.list","params":{
  "window_id": "window:1"  // optional
}}
```

**Response:**

```json theme={null}
{
  "ok": true,
  "result": {
    "workspaces": [
      {
        "id": "...",
        "ref": "workspace:1",
        "index": 0,
        "title": "Project A",
        "selected": true
      }
    ]
  }
}
```

### workspace.create

Create a new workspace:

```json theme={null}
{"id":"7","method":"workspace.create","params":{
  "cwd": "/Users/me/projects/app"
}}
```

**Response:**

```json theme={null}
{
  "ok": true,
  "result": {
    "workspace_id": "...",
    "workspace_ref": "workspace:3"
  }
}
```

### workspace.current

Get current workspace:

```json theme={null}
{"id":"8","method":"workspace.current","params":{}}
```

### workspace.select

Switch to a workspace:

```json theme={null}
{"id":"9","method":"workspace.select","params":{
  "workspace_id": "workspace:2"
}}
```

### workspace.close

Close a workspace:

```json theme={null}
{"id":"10","method":"workspace.close","params":{
  "workspace_id": "workspace:1"
}}
```

### workspace.rename

Rename a workspace:

```json theme={null}
{"id":"11","method":"workspace.rename","params":{
  "workspace_id": "workspace:1",
  "title": "My Project"
}}
```

### workspace.move\_to\_window

Move workspace to another window:

```json theme={null}
{"id":"12","method":"workspace.move_to_window","params":{
  "workspace_id": "workspace:1",
  "window_id": "window:2"
}}
```

### workspace.reorder

Reorder workspace position:

```json theme={null}
{"id":"13","method":"workspace.reorder","params":{
  "workspace_id": "workspace:3",
  "index": 0,  // or use before_workspace_id/after_workspace_id
  "window_id": "window:1"  // optional
}}
```

### workspace.action

Perform workspace action:

```json theme={null}
{"id":"14","method":"workspace.action","params":{
  "workspace_id": "workspace:1",
  "action": "next"  // next|previous|last|select|close|rename
}}
```

## Pane methods

### pane.list

List panes in a workspace:

```json theme={null}
{"id":"15","method":"pane.list","params":{
  "workspace_id": "workspace:1"
}}
```

### pane.create

Create a new pane:

```json theme={null}
{"id":"16","method":"pane.create","params":{
  "workspace_id": "workspace:1",
  "direction": "right",  // left|right|up|down
  "type": "terminal",    // terminal|browser
  "url": "https://example.com"  // for browser panes
}}
```

### pane.focus

Focus a pane:

```json theme={null}
{"id":"17","method":"pane.focus","params":{
  "pane_id": "pane:1",
  "workspace_id": "workspace:1"  // optional
}}
```

### pane.surfaces

List surfaces in a pane:

```json theme={null}
{"id":"18","method":"pane.surfaces","params":{
  "pane_id": "pane:1",
  "workspace_id": "workspace:1"  // optional
}}
```

## Surface methods

### surface.list

List surfaces in a workspace:

```json theme={null}
{"id":"19","method":"surface.list","params":{
  "workspace_id": "workspace:1"
}}
```

### surface.create

Create a new surface in a pane:

```json theme={null}
{"id":"20","method":"surface.create","params":{
  "pane_id": "pane:1",
  "workspace_id": "workspace:1",
  "type": "terminal",  // terminal|browser
  "url": "https://example.com"  // for browser
}}
```

### surface.split

Create a split from a surface:

```json theme={null}
{"id":"21","method":"surface.split","params":{
  "surface_id": "surface:1",
  "workspace_id": "workspace:1",
  "direction": "right"  // left|right|up|down
}}
```

### surface.close

Close a surface:

```json theme={null}
{"id":"22","method":"surface.close","params":{
  "surface_id": "surface:1",
  "workspace_id": "workspace:1"  // optional
}}
```

### surface.focus

Focus a surface:

```json theme={null}
{"id":"23","method":"surface.focus","params":{
  "surface_id": "surface:2",
  "workspace_id": "workspace:1"  // optional
}}
```

### surface.move

Move a surface:

```json theme={null}
{"id":"24","method":"surface.move","params":{
  "surface_id": "surface:1",
  "pane_id": "pane:2",          // optional
  "workspace_id": "workspace:2", // optional
  "window_id": "window:1",      // optional
  "index": 0,                    // optional
  "before_surface_id": "surface:3",  // optional
  "after_surface_id": "surface:4",   // optional
  "focus": true                  // optional
}}
```

### surface.reorder

Reorder surface within pane:

```json theme={null}
{"id":"25","method":"surface.reorder","params":{
  "surface_id": "surface:3",
  "index": 0  // or before_surface_id/after_surface_id
}}
```

### surface.read\_text

Read terminal screen content:

```json theme={null}
{"id":"26","method":"surface.read_text","params":{
  "surface_id": "surface:1",
  "workspace_id": "workspace:1",  // optional
  "scrollback": true,              // optional
  "lines": 100                     // optional
}}
```

**Response:**

```json theme={null}
{
  "ok": true,
  "result": {
    "text": "terminal output here..."
  }
}
```

### surface.send\_text

Send text to terminal:

```json theme={null}
{"id":"27","method":"surface.send_text","params":{
  "surface_id": "surface:1",
  "workspace_id": "workspace:1",  // optional
  "text": "git status\\n"
}}
```

### surface.send\_key

Send key to terminal:

```json theme={null}
{"id":"28","method":"surface.send_key","params":{
  "surface_id": "surface:1",
  "workspace_id": "workspace:1",  // optional
  "key": "ctrl+c"
}}
```

### surface.trigger\_flash

Flash surface visually:

```json theme={null}
{"id":"29","method":"surface.trigger_flash","params":{
  "surface_id": "surface:1",
  "workspace_id": "workspace:1"  // optional
}}
```

### surface.health

Check surface health status:

```json theme={null}
{"id":"30","method":"surface.health","params":{
  "workspace_id": "workspace:1"
}}
```

## Tab methods

### tab.action

Perform tab action:

```json theme={null}
{"id":"31","method":"tab.action","params":{
  "surface_id": "surface:2",  // or tab:2
  "workspace_id": "workspace:1",  // optional
  "action": "next",  // next|previous|close|rename|reload
  "title": "New Title",  // for rename
  "url": "https://example.com"  // for reload on terminal tabs
}}
```

## Browser methods

All browser methods require a `surface_id` parameter identifying the browser surface.

### browser.open\_split

Open URL in new browser split:

```json theme={null}
{"id":"32","method":"browser.open_split","params":{
  "url": "https://example.com",
  "surface_id": "surface:1",      // optional - source surface
  "workspace_id": "workspace:1",  // optional
  "window_id": "window:1"         // optional
}}
```

### browser.navigate

Navigate to URL:

```json theme={null}
{"id":"33","method":"browser.navigate","params":{
  "surface_id": "surface:1",
  "url": "https://google.com",
  "snapshot_after": true  // optional
}}
```

### browser.back / browser.forward / browser.reload

Browser navigation:

```json theme={null}
{"id":"34","method":"browser.back","params":{
  "surface_id": "surface:1",
  "snapshot_after": true  // optional
}}
```

### browser.url.get

Get current URL:

```json theme={null}
{"id":"35","method":"browser.url.get","params":{
  "surface_id": "surface:1"
}}
```

**Response:**

```json theme={null}
{
  "ok": true,
  "result": {
    "url": "https://example.com/page"
  }
}
```

### browser.snapshot

Capture page accessibility tree:

```json theme={null}
{"id":"36","method":"browser.snapshot","params":{
  "surface_id": "surface:1",
  "interactive": true,     // only interactive elements
  "cursor": true,          // show focused element
  "compact": true,         // compact format
  "max_depth": 3,          // tree depth limit
  "selector": ".content"   // scope to selector
}}
```

**Response:**

```json theme={null}
{
  "ok": true,
  "result": {
    "snapshot": "<accessibility tree text>"
  }
}
```

### browser.eval

Execute JavaScript:

```json theme={null}
{"id":"37","method":"browser.eval","params":{
  "surface_id": "surface:1",
  "script": "return document.title"
}}
```

**Response:**

```json theme={null}
{
  "ok": true,
  "result": {
    "value": "Page Title"
  }
}
```

### browser.wait

Wait for page conditions:

```json theme={null}
{"id":"38","method":"browser.wait","params":{
  "surface_id": "surface:1",
  "selector": ".element",          // wait for selector
  "text_contains": "Success",      // wait for text
  "url_contains": "complete",      // wait for URL
  "load_state": "interactive",     // interactive|complete
  "function": "() => true",        // wait for JS function
  "timeout_ms": 5000               // timeout in milliseconds
}}
```

### browser.click / browser.hover / browser.focus

Interact with elements:

```json theme={null}
{"id":"39","method":"browser.click","params":{
  "surface_id": "surface:1",
  "selector": "button.submit",
  "snapshot_after": true  // optional
}}
```

### browser.type

Type text into element:

```json theme={null}
{"id":"40","method":"browser.type","params":{
  "surface_id": "surface:1",
  "selector": "input[name='email']",
  "text": "user@example.com",
  "snapshot_after": true  // optional
}}
```

### browser.fill

Fill input (clears first):

```json theme={null}
{"id":"41","method":"browser.fill","params":{
  "surface_id": "surface:1",
  "selector": "input",
  "text": "new value",
  "snapshot_after": true  // optional
}}
```

### browser.press

Press keyboard key:

```json theme={null}
{"id":"42","method":"browser.press","params":{
  "surface_id": "surface:1",
  "key": "Enter",
  "snapshot_after": true  // optional
}}
```

### browser.select

Select dropdown option:

```json theme={null}
{"id":"43","method":"browser.select","params":{
  "surface_id": "surface:1",
  "selector": "select[name='country']",
  "value": "US",
  "snapshot_after": true  // optional
}}
```

### browser.get.\*

Extract element data:

```json theme={null}
// Get text
{"id":"44","method":"browser.get.text","params":{
  "surface_id": "surface:1",
  "selector": ".title"
}}

// Get HTML
{"id":"45","method":"browser.get.html","params":{
  "surface_id": "surface:1",
  "selector": "#content"
}}

// Get input value
{"id":"46","method":"browser.get.value","params":{
  "surface_id": "surface:1",
  "selector": "input"
}}

// Get attribute
{"id":"47","method":"browser.get.attr","params":{
  "surface_id": "surface:1",
  "selector": "img",
  "attr": "src"
}}

// Get element count
{"id":"48","method":"browser.get.count","params":{
  "surface_id": "surface:1",
  "selector": ".item"
}}

// Get page title
{"id":"49","method":"browser.get.title","params":{
  "surface_id": "surface:1"
}}
```

### browser.is.\*

Check element state:

```json theme={null}
// Check visibility
{"id":"50","method":"browser.is.visible","params":{
  "surface_id": "surface:1",
  "selector": ".modal"
}}

// Check enabled
{"id":"51","method":"browser.is.enabled","params":{
  "surface_id": "surface:1",
  "selector": "button"
}}

// Check checked
{"id":"52","method":"browser.is.checked","params":{
  "surface_id": "surface:1",
  "selector": "input[type='checkbox']"
}}
```

### browser.find.\*

Find elements by locator:

```json theme={null}
// Find by role
{"id":"53","method":"browser.find.role","params":{
  "surface_id": "surface:1",
  "role": "button",
  "name": "Submit",  // optional
  "exact": true      // optional
}}

// Find by text
{"id":"54","method":"browser.find.text","params":{
  "surface_id": "surface:1",
  "text": "Click here",
  "exact": false  // optional
}}

// Find by test ID
{"id":"55","method":"browser.find.testid","params":{
  "surface_id": "surface:1",
  "testid": "login-button"
}}
```

### browser.screenshot

Capture screenshot:

```json theme={null}
{"id":"56","method":"browser.screenshot","params":{
  "surface_id": "surface:1"
}}
```

**Response:**

```json theme={null}
{
  "ok": true,
  "result": {
    "png_base64": "<base64-encoded-png>"
  }
}
```

### browser.focus\_webview

Focus the browser webview:

```json theme={null}
{"id":"57","method":"browser.focus_webview","params":{
  "surface_id": "surface:1"
}}
```

### browser.is\_webview\_focused

Check if webview is focused:

```json theme={null}
{"id":"58","method":"browser.is_webview_focused","params":{
  "surface_id": "surface:1"
}}
```

**Response:**

```json theme={null}
{
  "ok": true,
  "result": {
    "focused": true
  }
}
```

## Example client (Python)

```python theme={null}
import socket
import json
import uuid

class CmuxClient:
    def __init__(self, socket_path="/tmp/cmux.sock"):
        self.socket_path = socket_path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.connect(socket_path)
    
    def request(self, method, params=None):
        req = {
            "id": str(uuid.uuid4()),
            "method": method,
            "params": params or {}
        }
        self.sock.sendall(json.dumps(req).encode() + b"\n")
        
        # Read response until newline
        response = b""
        while b"\n" not in response:
            response += self.sock.recv(8192)
        
        data = json.loads(response.decode().strip())
        if not data.get("ok"):
            error = data.get("error", {})
            raise Exception(f"{error.get('code')}: {error.get('message')}")
        
        return data.get("result", {})
    
    def close(self):
        self.sock.close()

# Usage
client = CmuxClient()

# Create workspace
result = client.request("workspace.create", {
    "cwd": "/Users/me/projects"
})
workspace_id = result["workspace_id"]

# Send command
client.request("surface.send_text", {
    "workspace_id": workspace_id,
    "text": "npm run dev\\n"
})

# Read output
output = client.request("surface.read_text", {
    "workspace_id": workspace_id,
    "scrollback": True
})
print(output["text"])

client.close()
```

## Error codes

Common error codes in error responses:

* `error`: Generic error
* `invalid_params`: Invalid or missing parameters
* `not_found`: Resource not found
* `unauthorized`: Authentication required or failed
* `timeout`: Operation timed out
* `internal_error`: Internal server error
