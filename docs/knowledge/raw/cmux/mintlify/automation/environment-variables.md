> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Environment variables

> Auto-set context variables for cmux automation

cmux automatically sets environment variables in terminals that identify the workspace, surface, and socket path. This enables CLI commands and scripts to automatically target the correct context.

## Auto-set variables

These variables are automatically set in all cmux terminal surfaces:

### CMUX\_WORKSPACE\_ID

UUID of the current workspace.

**Example value:**

```bash theme={null}
a1b2c3d4-e5f6-7890-abcd-ef1234567890
```

**Usage:**

```bash theme={null}
# Commands automatically use current workspace
cmux send git status
cmux list-panels
cmux new-split right

# Explicitly reference from scripts
echo "Current workspace: $CMUX_WORKSPACE_ID"
```

<Note>
  Most CLI commands use `CMUX_WORKSPACE_ID` as the default `--workspace` value when not explicitly provided.
</Note>

### CMUX\_SURFACE\_ID

UUID of the current terminal surface (also called "panel" or "tab").

**Example value:**

```bash theme={null}
b2c3d4e5-f6a7-8901-bcde-f12345678901
```

**Usage:**

```bash theme={null}
# Commands automatically target current surface
cmux send echo hello
cmux read-screen --scrollback
cmux close-surface

# Reference from scripts
echo "Current surface: $CMUX_SURFACE_ID"
```

<Note>
  Commands like `send`, `read-screen`, and `close-surface` use `CMUX_SURFACE_ID` as the default `--surface` value.
</Note>

### CMUX\_TAB\_ID

Optional alias for `CMUX_SURFACE_ID`, used by `tab-action` and `rename-tab` as the default `--tab` value.

**Usage:**

```bash theme={null}
cmux tab-action --action rename --title "My Tab"
cmux rename-tab New Name
```

### CMUX\_SOCKET\_PATH

Path to the control socket for the running cmux instance.

**Default values:**

* Production: `/tmp/cmux.sock`
* Debug build: `/tmp/cmux-debug.sock`
* Nightly: `/tmp/cmux-nightly.sock`
* Staging: `/tmp/cmux-staging.sock`

**Usage:**

```bash theme={null}
# CLI automatically uses correct socket
cmux list-workspaces

# Custom socket clients
echo '{"id":"1","method":"workspace.list","params":{}}' | \
  nc -U "$CMUX_SOCKET_PATH"
```

<Note>
  The `--socket` CLI flag and custom clients should read `CMUX_SOCKET_PATH` to connect to the correct instance.
</Note>

## Configuration variables

These variables configure cmux behavior:

### CMUX\_SOCKET\_MODE

Override the socket authentication mode.

**Values:**

* `off`: Disable socket
* `cmuxonly` / `cmux-only`: Only cmux processes (default)
* `automation`: Allow external automation clients
* `password`: Require password authentication
* `allowall` / `allow-all` / `fullopenaccess`: Full open access (unsafe)

**Example:**

```bash theme={null}
export CMUX_SOCKET_MODE=automation
open -a cmux
```

### CMUX\_SOCKET\_ENABLE

Enable or disable the socket.

**Values:**

* `1`, `true`, `yes`, `on`: Enable
* `0`, `false`, `no`, `off`: Disable

**Example:**

```bash theme={null}
export CMUX_SOCKET_ENABLE=0
open -a cmux  # socket disabled
```

### CMUX\_SOCKET\_PASSWORD

Socket password for authentication (when password mode is enabled).

**Precedence:**

1. `--password` CLI flag (highest)
2. `CMUX_SOCKET_PASSWORD` environment variable
3. Password file: `~/Library/Application Support/cmux/socket-control-password`

**Example:**

```bash theme={null}
export CMUX_SOCKET_PASSWORD="my-secret-password"
cmux --socket /tmp/cmux.sock list-workspaces
```

<Warning>
  Store passwords securely. Avoid hardcoding passwords in scripts. Use the password file for persistent storage.
</Warning>

### CMUX\_ALLOW\_SOCKET\_OVERRIDE

Allow `CMUX_SOCKET_PATH` to override the default socket path.

**Values:**

* `1`, `true`, `yes`, `on`: Allow override

**Default behavior:**

* Debug/staging builds: Always allow override
* Production build: Require explicit `CMUX_ALLOW_SOCKET_OVERRIDE=1`

**Example:**

```bash theme={null}
export CMUX_ALLOW_SOCKET_OVERRIDE=1
export CMUX_SOCKET_PATH=/tmp/my-custom.sock
open -a cmux
```

### CMUX\_TAG

Launch tag for parallel/isolated debug builds.

**Usage:**

```bash theme={null}
export CMUX_TAG=feature-test
open -a "cmux DEV"
# Creates isolated app with:
# - Name: "cmux DEV (feature-test)"
# - Bundle ID: com.cmuxterm.app.debug.feature-test
# - Socket: /tmp/cmux-debug-feature-test.sock
```

<Note>
  Tags are used by the `./scripts/reload.sh --tag <name>` script for parallel development.
</Note>

## Debugging variables

### CMUX\_CLI\_SENTRY\_DISABLED

Disable CLI Sentry diagnostics.

**Values:**

* `1`: Disable Sentry

**Example:**

```bash theme={null}
export CMUX_CLI_SENTRY_DISABLED=1
cmux list-workspaces
```

### CMUX\_CLAUDE\_HOOK\_SENTRY\_DISABLED

Disable Claude hook Sentry diagnostics.

**Values:**

* `1`: Disable Sentry

**Example:**

```bash theme={null}
export CMUX_CLAUDE_HOOK_SENTRY_DISABLED=1
cmux claude-hook session-start
```

### CMUX\_CLAUDE\_HOOK\_STATE\_PATH

Override Claude hook session state file path.

**Default:**

```bash theme={null}
~/.cmuxterm/claude-hook-sessions.json
```

**Example:**

```bash theme={null}
export CMUX_CLAUDE_HOOK_STATE_PATH=~/custom-hook-state.json
```

### CMUXTERM\_CLI\_RESPONSE\_TIMEOUT\_SEC

CLI socket response timeout in seconds.

**Default:**

```bash theme={null}
15.0
```

**Example:**

```bash theme={null}
export CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC=30
cmux browser wait --timeout 25
```

## Shell integration

Add these to your shell profile for enhanced automation:

### Bash/Zsh

```bash ~/.bashrc or ~/.zshrc theme={null}
# Quick workspace info
alias ws='echo "Workspace: $CMUX_WORKSPACE_ID"'
alias surf='echo "Surface: $CMUX_SURFACE_ID"'

# Notify on command completion
notify() {
  "$@"
  local exit_code=$?
  if [ $exit_code -eq 0 ]; then
    cmux notify --title "Command complete" --body "$*"
  else
    cmux notify --title "Command failed" --body "$* (exit $exit_code)"
  fi
  return $exit_code
}

# Example: notify npm run build
```

### Fish

```fish ~/.config/fish/config.fish theme={null}
# Quick workspace info
alias ws='echo "Workspace: $CMUX_WORKSPACE_ID"'
alias surf='echo "Surface: $CMUX_SURFACE_ID"'

# Notify on command completion
function notify
    $argv
    set exit_code $status
    if test $exit_code -eq 0
        cmux notify --title "Command complete" --body "$argv"
    else
        cmux notify --title "Command failed" --body "$argv (exit $exit_code)"
    end
    return $exit_code
end
```

## Script examples

### Multi-workspace build script

```bash theme={null}
#!/bin/bash
# build-all.sh - Build multiple projects in parallel

PROJECTS=("frontend" "backend" "api")

for project in "${PROJECTS[@]}"; do
  ws_id=$(cmux new-workspace --cwd "~/projects/$project" --json | jq -r '.workspace_id')
  cmux send --workspace "$ws_id" "npm run build"
  cmux set-status --workspace "$ws_id" build "Building..." --icon "⚙️"
done

echo "Started builds in ${#PROJECTS[@]} workspaces"
```

### Monitor and notify

```bash theme={null}
#!/bin/bash
# watch-tests.sh - Monitor test output and notify on completion

WORKSPACE=${CMUX_WORKSPACE_ID}

while true; do
  OUTPUT=$(cmux read-screen --workspace "$WORKSPACE" --lines 10)
  
  if echo "$OUTPUT" | grep -q "Tests: .* passed"; then
    cmux notify --workspace "$WORKSPACE" \
      --title "Tests Passed" \
      --body "All tests completed successfully"
    break
  elif echo "$OUTPUT" | grep -q "Tests: .* failed"; then
    cmux notify --workspace "$WORKSPACE" \
      --title "Tests Failed" \
      --body "Some tests failed"
    break
  fi
  
  sleep 2
done
```

### Context-aware automation

```bash theme={null}
#!/bin/bash
# smart-deploy.sh - Deploy based on current workspace

if [ -z "$CMUX_WORKSPACE_ID" ]; then
  echo "Not running in cmux - creating workspace"
  WORKSPACE=$(cmux new-workspace --cwd . --json | jq -r '.workspace_id')
else
  echo "Using current workspace: $CMUX_WORKSPACE_ID"
  WORKSPACE=$CMUX_WORKSPACE_ID
fi

cmux set-progress --workspace "$WORKSPACE" 0.0 --label "Starting deploy"
cmux send --workspace "$WORKSPACE" "npm run deploy"

for i in {1..10}; do
  cmux set-progress --workspace "$WORKSPACE" "0.${i}" --label "Deploying..."
  sleep 1
done

cmux set-progress --workspace "$WORKSPACE" 1.0 --label "Complete"
cmux clear-progress --workspace "$WORKSPACE"
cmux notify --workspace "$WORKSPACE" --title "Deploy Complete"
```

## Best practices

<AccordionGroup>
  <Accordion title="Always check for cmux context">
    Check if `CMUX_WORKSPACE_ID` is set before assuming you're in a cmux terminal:

    ```bash theme={null}
    if [ -z "$CMUX_WORKSPACE_ID" ]; then
      echo "Not running in cmux"
      exit 1
    fi
    ```
  </Accordion>

  <Accordion title="Use --json for machine parsing">
    Always use `--json` flag when parsing CLI output in scripts:

    ```bash theme={null}
    ws_id=$(cmux new-workspace --json | jq -r '.workspace_id')
    ```
  </Accordion>

  <Accordion title="Handle missing variables gracefully">
    Provide fallbacks for missing environment variables:

    ```bash theme={null}
    WORKSPACE=${CMUX_WORKSPACE_ID:-$(cmux current-workspace)}
    SOCKET=${CMUX_SOCKET_PATH:-/tmp/cmux.sock}
    ```
  </Accordion>

  <Accordion title="Use explicit IDs for non-local operations">
    When operating on resources outside the current workspace, always specify explicit IDs:

    ```bash theme={null}
    # Good: explicit workspace
    cmux send --workspace workspace:2 "echo hello"

    # Bad: assumes current workspace
    cmux send "echo hello"  # works locally but not in scripts
    ```
  </Accordion>
</AccordionGroup>
