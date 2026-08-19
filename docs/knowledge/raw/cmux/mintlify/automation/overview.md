> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Automation overview

> Control cmux programmatically via CLI and socket API

cmux provides powerful automation capabilities through its CLI tool and JSON-RPC socket protocol. You can control windows, workspaces, panes, surfaces, send text, read screen output, manage notifications, and automate browser interactions.

## Quick start

Open a directory in cmux:

```bash theme={null}
cmux ~/projects/my-app
```

Create a new workspace and run a command:

```bash theme={null}
cmux new-workspace --cwd ~/projects --command "npm run dev"
```

Send text to the current terminal:

```bash theme={null}
echo "git status" | cmux send
```

Read the terminal screen:

```bash theme={null}
cmux read-screen --scrollback
```

## Use cases

<AccordionGroup>
  <Accordion title="AI coding agents">
    cmux is designed for AI coding agents. The CLI and socket API enable agents to:

    * Create workspaces for different tasks
    * Send commands and read output
    * Receive notifications when long-running processes complete
    * Automate browser testing and web scraping
    * Monitor multiple terminals simultaneously
  </Accordion>

  <Accordion title="Build automation">
    Automate development workflows:

    * Start dev servers in dedicated workspaces
    * Run tests and collect results
    * Monitor logs across multiple services
    * Trigger notifications on build completion
  </Accordion>

  <Accordion title="DevOps workflows">
    Manage infrastructure and deployments:

    * SSH to multiple servers in split panes
    * Monitor deployment status
    * Automate kubectl commands
    * Stream logs from multiple pods
  </Accordion>

  <Accordion title="Browser automation">
    Control embedded browser surfaces:

    * Navigate to URLs and wait for page load
    * Click elements, fill forms, take screenshots
    * Extract text, HTML, or element properties
    * Run JavaScript and get results
  </Accordion>
</AccordionGroup>

## Architecture

cmux exposes two automation interfaces:

### CLI tool

The `cmux` command-line tool provides a convenient interface for automation:

* **Simple commands**: `cmux new-workspace`, `cmux send`, `cmux read-screen`
* **JSON output**: Add `--json` for machine-readable responses
* **Environment integration**: Commands automatically target the current workspace/surface
* **Cross-platform**: Works from any terminal or script

See [CLI reference](/automation/cli-reference) for all available commands.

### Socket API

The underlying JSON-RPC socket protocol enables real-time control:

* **Event-driven**: Subscribe to notifications, status changes, and errors
* **Low latency**: Direct socket communication with no subprocess overhead
* **Batch operations**: Send multiple commands in sequence
* **Custom clients**: Build your own automation tools in any language

See [Socket API](/automation/socket-api) for protocol details.

## Environment variables

cmux automatically sets environment variables in terminals that identify the workspace and surface:

* `CMUX_WORKSPACE_ID`: UUID of the current workspace
* `CMUX_SURFACE_ID`: UUID of the current terminal surface
* `CMUX_SOCKET_PATH`: Path to the control socket (default: `/tmp/cmux.sock`)

These allow commands like `cmux send` to automatically target the correct context without explicit flags.

See [Environment variables](/automation/environment-variables) for details.

## Authentication modes

cmux supports multiple socket authentication modes:

<CardGroup cols={2}>
  <Card title="cmux processes only" icon="lock">
    Default mode. Only processes started inside cmux terminals can connect.
  </Card>

  <Card title="Automation mode" icon="robot">
    Allow external local automation clients from the same macOS user (no ancestry check).
  </Card>

  <Card title="Password mode" icon="key">
    Require socket authentication with a password. Use `--password` flag or `CMUX_SOCKET_PASSWORD` env var.
  </Card>

  <Card title="Full open access" icon="unlock">
    Allow any local process and user to connect with no auth. Unsafe - use only for testing.
  </Card>
</CardGroup>

Configure the socket mode in Settings or via the `CMUX_SOCKET_MODE` environment variable.

## Handle formats

Most commands accept multiple handle formats for identifying resources:

* **UUIDs**: `a1b2c3d4-e5f6-7890-abcd-ef1234567890`
* **Refs**: `window:1`, `workspace:2`, `pane:3`, `surface:4`, `tab:1`
* **Indexes**: `0`, `1`, `2` (0-based, depends on context)

By default, commands output refs. Use `--id-format uuids` or `--id-format both` to include UUIDs.

## Next steps

<CardGroup cols={2}>
  <Card title="CLI reference" icon="terminal" href="/automation/cli-reference">
    Complete list of CLI commands and flags
  </Card>

  <Card title="Socket API" icon="plug" href="/automation/socket-api">
    JSON-RPC protocol specification
  </Card>

  <Card title="Environment variables" icon="gear" href="/automation/environment-variables">
    Auto-set context variables
  </Card>
</CardGroup>
