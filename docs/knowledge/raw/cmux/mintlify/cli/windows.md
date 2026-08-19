> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Window Commands

> Manage cmux windows with CLI commands

Manage cmux windows using the command-line interface. Windows contain workspaces and can be controlled programmatically.

## list-windows

List all open windows with their IDs, indexes, and workspace counts.

```bash theme={null}
cmux list-windows
```

**Flags:**

<ParamField path="--json" type="boolean">
  Output results in JSON format
</ParamField>

<ParamField path="--id-format" type="string">
  Control ID output format: `refs`, `uuids`, or `both` (default: `refs`)
</ParamField>

**Output (text):**

```
* 1: window:1 selected_workspace=workspace:1 workspaces=3
  2: window:2 selected_workspace=workspace:4 workspaces=2
```

**Output (JSON):**

```json theme={null}
[
  {
    "index": 1,
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "key": true,
    "workspace_count": 3,
    "selected_workspace_id": "550e8400-e29b-41d4-a716-446655440001"
  }
]
```

## current-window

Get the ID of the currently focused window.

```bash theme={null}
cmux current-window
```

**Flags:**

<ParamField path="--json" type="boolean">
  Output results in JSON format
</ParamField>

**Output (text):**

```
window:1
```

**Output (JSON):**

```json theme={null}
{
  "window_id": "window:1"
}
```

## new-window

Create a new window.

```bash theme={null}
cmux new-window
```

**Output:**

```
OK window:2
```

## focus-window

Focus a specific window by ID, ref, or index.

```bash theme={null}
cmux focus-window --window <id|ref|index>
```

**Flags:**

<ParamField path="--window" type="string" required>
  Window ID (UUID), ref (window:1), or index (1)
</ParamField>

**Examples:**

<CodeGroup>
  ```bash By Ref theme={null}
  cmux focus-window --window window:1
  ```

  ```bash By UUID theme={null}
  cmux focus-window --window 550e8400-e29b-41d4-a716-446655440000
  ```

  ```bash By Index theme={null}
  cmux focus-window --window 1
  ```
</CodeGroup>

**Output:**

```
OK
```

## close-window

Close a specific window by ID, ref, or index.

```bash theme={null}
cmux close-window --window <id|ref|index>
```

**Flags:**

<ParamField path="--window" type="string" required>
  Window ID (UUID), ref (window:1), or index (1)
</ParamField>

**Examples:**

<CodeGroup>
  ```bash By Ref theme={null}
  cmux close-window --window window:1
  ```

  ```bash By UUID theme={null}
  cmux close-window --window 550e8400-e29b-41d4-a716-446655440000
  ```
</CodeGroup>

**Output:**

```
OK
```

## Global Flags

These flags can be used with any window command:

<ParamField path="--socket" type="string">
  Path to the cmux socket (default: `/tmp/cmux.sock` or `$CMUX_SOCKET_PATH`)
</ParamField>

<ParamField path="--password" type="string">
  Socket authentication password (or use `$CMUX_SOCKET_PASSWORD`)
</ParamField>
