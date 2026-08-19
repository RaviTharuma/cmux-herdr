# [#](#title)CLI Reference

# [#](https://cmux-docs-release.vercel.app/docs/api#title)CLI Reference

cmux provides both a CLI tool and a Unix socket for programmatic control. Every command is available through both interfaces.

## [#](https://cmux-docs-release.vercel.app/docs/api#socket)Socket

| Build | Path |
| --- | --- |
| Release | `/tmp/cmux.sock` |
| Debug | `/tmp/cmux-debug.sock` |
| Tagged debug build | `/tmp/cmux-debug-<tag>.sock` |

Override with the CMUX\_SOCKET\_PATH environment variable. Send one newline-terminated JSON request per call:

```
{"id":"req-1","method":"workspace.list","params":{}}
// Response:
{"id":"req-1","ok":true,"result":{"workspaces":[...]}}
```

JSON socket requests must use method and params. Legacy v1 JSON payloads such as `{"command":"..."}` are not supported.

## [#](https://cmux-docs-release.vercel.app/docs/api#access-modes)Access modes

| Mode | Description | How to enable |
| --- | --- | --- |
| **Off** | Socket disabled | Settings UI or CMUX\_SOCKET\_MODE=off |
| **cmux processes only** | Only processes spawned inside cmux terminals can connect. | Default mode in Settings UI |
| **allowAll** | Allow any local process to connect (no ancestry check). | Environment override only: CMUX\_SOCKET\_MODE=allowAll |

On shared machines, use Off or cmux processes only.

## [#](https://cmux-docs-release.vercel.app/docs/api#cli-options)CLI options

| Flag | Description |
| --- | --- |
| `--socket PATH` | Custom socket path |
| `--json` | Output in JSON format |
| `--window ID` | Target a specific window |
| `--workspace ID` | Target a specific workspace |
| `--surface ID` | Target a specific surface |
| `--id-format refs|uuids|both` | Control identifier format in JSON output |

## [#](https://cmux-docs-release.vercel.app/docs/api#workspace-commands)Workspace commands

#### list-workspaces

List all open workspaces.

CLI

```
cmux list-workspaces
cmux list-workspaces --json
```

Socket

```
{"id":"ws-list","method":"workspace.list","params":{}}
```

#### new-workspace

Create a new workspace.

CLI

```
cmux new-workspace
```

Socket

```
{"id":"ws-new","method":"workspace.create","params":{}}
```

#### select-workspace

Switch to a specific workspace.

CLI

```
cmux select-workspace --workspace <id>
```

Socket

```
{"id":"ws-select","method":"workspace.select","params":{"workspace_id":"<id>"}}
```

#### current-workspace

Get the currently active workspace.

CLI

```
cmux current-workspace
cmux current-workspace --json
```

Socket

```
{"id":"ws-current","method":"workspace.current","params":{}}
```

#### close-workspace

Close a workspace.

CLI

```
cmux close-workspace --workspace <id>
```

Socket

```
{"id":"ws-close","method":"workspace.close","params":{"workspace_id":"<id>"}}
```

## [#](https://cmux-docs-release.vercel.app/docs/api#split-commands)Split commands

#### new-split

Create a new split pane. Directions: left, right, up, down.

CLI

```
cmux new-split right
cmux new-split down
```

Socket

```
{"id":"split-new","method":"surface.split","params":{"direction":"right"}}
```

#### list-panels

List all surfaces in the current workspace.

CLI

```
cmux list-panels
cmux list-panels --json
```

Socket

```
{"id":"surface-list","method":"surface.list","params":{}}
```

#### list-pane-surfaces

List surfaces in the focused pane.

CLI

```
cmux list-pane-surfaces
cmux list-pane-surfaces --json
```

Socket

```
{"id":"pane-surfaces","method":"pane.surfaces","params":{}}
```

#### focus-panel

Focus a specific surface.

CLI

```
cmux focus-panel --panel <id>
```

Socket

```
{"id":"surface-focus","method":"surface.focus","params":{"surface_id":"<id>"}}
```

## [#](https://cmux-docs-release.vercel.app/docs/api#input-commands)Input commands

#### send

Send text input to the focused terminal.

CLI

```
cmux send "echo hello"
cmux send "ls -la\n"
```

Socket

```
{"id":"send-text","method":"surface.send_text","params":{"text":"echo hello\n"}}
```

#### send-key

Send a key press. Keys: enter, tab, escape, backspace, delete, up, down, left, right.

CLI

```
cmux send-key enter
```

Socket

```
{"id":"send-key","method":"surface.send_key","params":{"key":"enter"}}
```

#### send --surface

Send text to a specific surface.

CLI

```
cmux send --surface <id> "command"
```

Socket

```
{"id":"send-surface","method":"surface.send_text","params":{"surface_id":"<id>","text":"command"}}
```

#### send-key --surface

Send a key press to a specific surface.

CLI

```
cmux send-key --surface <id> enter
```

Socket

```
{"id":"send-key-surface","method":"surface.send_key","params":{"surface_id":"<id>","key":"enter"}}
```

## [#](https://cmux-docs-release.vercel.app/docs/api#notification-commands)Notification commands

#### notify

Send a notification.

CLI

```
cmux notify --title "Title" --body "Body"
cmux notify --title "T" --subtitle "S" --body "B"
```

Socket

```
{"id":"notify","method":"notification.create","params":{"title":"Title","subtitle":"S","body":"Body"}}
```

#### list-notifications

List all notifications.

CLI

```
cmux list-notifications
cmux list-notifications --json
```

Socket

```
{"id":"notif-list","method":"notification.list","params":{}}
```

#### clear-notifications

Clear all notifications.

CLI

```
cmux clear-notifications
```

Socket

```
{"id":"notif-clear","method":"notification.clear","params":{}}
```

## [#](https://cmux-docs-release.vercel.app/docs/api#sidebar-metadata)Sidebar metadata commands

Set status pills, progress bars, and log entries in the sidebar for any workspace. Useful for build scripts, CI integrations, and AI coding agents that want to surface state at a glance.

#### set-status

Set a sidebar status pill. Use a unique key so different tools can manage their own entries.

CLI

```
cmux set-status build "compiling" --icon hammer --color "#ff9500" --priority 80
cmux set-status deploy "v1.2.3" --workspace workspace:2
```

Socket

```
set_status build compiling --icon=hammer --color=#ff9500 --priority=80 --tab=<workspace-uuid>
```

#### clear-status

Remove a sidebar status entry by key.

CLI

```
cmux clear-status build
```

Socket

```
clear_status build --tab=<workspace-uuid>
```

#### list-status

List all sidebar status entries for a workspace.

CLI

```
cmux list-status
```

Socket

```
list_status --tab=<workspace-uuid>
```

#### set-progress

Set a progress bar in the sidebar (0.0 to 1.0).

CLI

```
cmux set-progress 0.5 --label "Building..."
cmux set-progress 1.0 --label "Done"
```

Socket

```
set_progress 0.5 --label=Building... --tab=<workspace-uuid>
```

#### clear-progress

Clear the sidebar progress bar.

CLI

```
cmux clear-progress
```

Socket

```
clear_progress --tab=<workspace-uuid>
```

#### log

Append a log entry to the sidebar. Levels: info, progress, success, warning, error.

CLI

```
cmux log "Build started"
cmux log --level error --source build "Compilation failed"
cmux log --level success -- "All 42 tests passed"
```

Socket

```
log --level=error --source=build --tab=<workspace-uuid> -- Compilation failed
```

#### clear-log

Clear all sidebar log entries.

CLI

```
cmux clear-log
```

Socket

```
clear_log --tab=<workspace-uuid>
```

#### list-log

List sidebar log entries.

CLI

```
cmux list-log
cmux list-log --limit 5
```

Socket

```
list_log --limit=5 --tab=<workspace-uuid>
```

#### sidebar-state

Dump all sidebar metadata (cwd, git branch, ports, status, progress, logs).

CLI

```
cmux sidebar-state
cmux sidebar-state --workspace workspace:2
```

Socket

```
sidebar_state --tab=<workspace-uuid>
```

## [#](https://cmux-docs-release.vercel.app/docs/api#utility-commands)Utility commands

#### ping

Check if cmux is running and responsive.

CLI

```
cmux ping
```

Socket

```
{"id":"ping","method":"system.ping","params":{}}
// Response: {"id":"ping","ok":true,"result":{"pong":true}}
```

#### capabilities

List available socket methods and current access mode.

CLI

```
cmux capabilities
cmux capabilities --json
```

Socket

```
{"id":"caps","method":"system.capabilities","params":{}}
```

#### identify

Show focused window/workspace/pane/surface context.

CLI

```
cmux identify
cmux identify --json
```

Socket

```
{"id":"identify","method":"system.identify","params":{}}
```

## [#](https://cmux-docs-release.vercel.app/docs/api#env-variables)Environment variables

| Variable | Description |
| --- | --- |
| `CMUX_SOCKET_PATH` | Override the socket path used by CLI and integrations |
| `CMUX_SOCKET_ENABLE` | Force-enable/disable socket (1/0, true/false, on/off) |
| `CMUX_SOCKET_MODE` | Override access mode (cmuxOnly, allowAll, off). Also accepts cmux-only/cmux\_only and allow-all/allow\_all |
| `CMUX_WORKSPACE_ID` | Auto-set: current workspace ID |
| `CMUX_SURFACE_ID` | Auto-set: current surface ID |
| `TERM_PROGRAM` | Set to ghostty |
| `TERM` | Set to xterm-ghostty |

Legacy CMUX\_SOCKET\_MODE values full and notifications are still accepted for compatibility.

## [#](https://cmux-docs-release.vercel.app/docs/api#detecting-cmux)Detecting cmux

bash

```
# Prefer explicit socket path if set
SOCK="${CMUX_SOCKET_PATH:-/tmp/cmux.sock}"
[ -S "$SOCK" ] && echo "Socket available"

# Check for the CLI
command -v cmux &>/dev/null && echo "cmux available"

# In cmux-managed terminals these are auto-set
[ -n "${CMUX_WORKSPACE_ID:-}" ] && [ -n "${CMUX_SURFACE_ID:-}" ] && echo "Inside cmux surface"

# Distinguish from regular Ghostty
[ "$TERM_PROGRAM" = "ghostty" ] && [ -n "${CMUX_WORKSPACE_ID:-}" ] && echo "In cmux"
```

## [#](https://cmux-docs-release.vercel.app/docs/api#examples)Examples

### [#](https://cmux-docs-release.vercel.app/docs/api#python-client)Python client

python

```
import json
import os
import socket

SOCKET_PATH = os.environ.get("CMUX_SOCKET_PATH", "/tmp/cmux.sock")

def rpc(method, params=None, req_id=1):
    payload = {"id": req_id, "method": method, "params": params or {}}
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
        sock.connect(SOCKET_PATH)
        sock.sendall(json.dumps(payload).encode("utf-8") + b"\n")
        return json.loads(sock.recv(65536).decode("utf-8"))

# List workspaces
print(rpc("workspace.list", req_id="ws"))

# Send notification
print(rpc(
    "notification.create",
    {"title": "Hello", "body": "From Python!"},
    req_id="notify"
))
```

### [#](https://cmux-docs-release.vercel.app/docs/api#shell-script)Shell script

bash

```
#!/bin/bash
SOCK="${CMUX_SOCKET_PATH:-/tmp/cmux.sock}"

cmux_cmd() {
    printf "%s\n" "$1" | nc -U "$SOCK"
}

cmux_cmd '{"id":"ws","method":"workspace.list","params":{}}'
cmux_cmd '{"id":"notify","method":"notification.create","params":{"title":"Done","body":"Task complete"}}'
```

### [#](https://cmux-docs-release.vercel.app/docs/api#build-script-notification)Build script with notification

bash

```
#!/bin/bash
npm run build
if [ $? -eq 0 ]; then
    cmux notify --title "✓ Build Success" --body "Ready to deploy"
else
    cmux notify --title "✗ Build Failed" --body "Check the logs"
fi
```

[Keyboard Shortcuts](https://cmux-docs-release.vercel.app/docs/keyboard-shortcuts) [Browser Automation](https://cmux-docs-release.vercel.app/docs/browser-automation)

Canonical: https://cmux-docs-release.vercel.app/docs/api
