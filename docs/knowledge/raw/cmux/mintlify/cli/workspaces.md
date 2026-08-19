> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Workspace Commands

> Manage cmux workspaces (vertical tabs) with CLI commands

Manage workspaces (vertical tabs) in cmux. Each workspace contains one or more panes with surfaces.

## list-workspaces

List all workspaces in the current or specified window.

```bash theme={null}
cmux list-workspaces
```

**Flags:**

<ParamField path="--json" type="boolean">
  Output results in JSON format
</ParamField>

<ParamField path="--id-format" type="string">
  Control ID output format: `refs`, `uuids`, or `both` (default: `refs`)
</ParamField>

<ParamField path="--window" type="string">
  Window ID, ref, or index to list workspaces from
</ParamField>

**Output (text):**

```
* workspace:1  My Project  [selected]
  workspace:2  Backend
  workspace:3  Frontend
```

**Output (JSON):**

```json theme={null}
{
  "workspaces": [
    {
      "ref": "workspace:1",
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "index": 1,
      "title": "My Project",
      "selected": true
    }
  ]
}
```

## new-workspace

Create a new workspace with optional working directory and command.

```bash theme={null}
cmux new-workspace [--cwd <path>] [--command <text>]
```

**Flags:**

<ParamField path="--cwd" type="string">
  Working directory for the new workspace (absolute or relative path)
</ParamField>

<ParamField path="--command" type="string">
  Command to run after workspace creation (automatically appends `\n`)
</ParamField>

**Examples:**

<CodeGroup>
  ```bash Basic theme={null}
  cmux new-workspace
  ```

  ```bash With Directory theme={null}
  cmux new-workspace --cwd ~/projects/myapp
  ```

  ```bash With Command theme={null}
  cmux new-workspace --cwd ~/projects/myapp --command "npm run dev"
  ```
</CodeGroup>

**Output:**

```
OK workspace:4
```

## select-workspace

Switch to a specific workspace by ID, ref, or index.

```bash theme={null}
cmux select-workspace --workspace <id|ref|index>
```

**Flags:**

<ParamField path="--workspace" type="string" required>
  Workspace ID (UUID), ref (workspace:1), or index (1)
</ParamField>

**Examples:**

<CodeGroup>
  ```bash By Ref theme={null}
  cmux select-workspace --workspace workspace:2
  ```

  ```bash By Index theme={null}
  cmux select-workspace --workspace 2
  ```

  ```bash By UUID theme={null}
  cmux select-workspace --workspace 550e8400-e29b-41d4-a716-446655440000
  ```
</CodeGroup>

**Output (text):**

```
OK workspace=workspace:2
```

**Output (JSON with --json):**

```json theme={null}
{
  "workspace_ref": "workspace:2",
  "workspace_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

## close-workspace

Close a specific workspace by ID, ref, or index.

```bash theme={null}
cmux close-workspace --workspace <id|ref|index>
```

**Flags:**

<ParamField path="--workspace" type="string" required>
  Workspace ID (UUID), ref (workspace:1), or index (1)
</ParamField>

**Examples:**

<CodeGroup>
  ```bash By Ref theme={null}
  cmux close-workspace --workspace workspace:3
  ```

  ```bash By Index theme={null}
  cmux close-workspace --workspace 3
  ```
</CodeGroup>

**Output:**

```
OK workspace=workspace:3
```

## rename-workspace

Rename a workspace (also aliased as `rename-window`).

```bash theme={null}
cmux rename-workspace [--workspace <id|ref|index>] [--] <title>
```

**Flags:**

<ParamField path="--workspace" type="string">
  Workspace to rename (defaults to current workspace from `$CMUX_WORKSPACE_ID`)
</ParamField>

**Examples:**

<CodeGroup>
  ```bash Current Workspace theme={null}
  cmux rename-workspace My New Title
  ```

  ```bash Specific Workspace theme={null}
  cmux rename-workspace --workspace workspace:2 Backend API
  ```

  ```bash With Separator theme={null}
  cmux rename-workspace --workspace 1 -- My Project
  ```
</CodeGroup>

**Output:**

```
OK workspace=workspace:1
```

## current-workspace

Get the ID of the currently selected workspace.

```bash theme={null}
cmux current-workspace
```

**Output (text):**

```
workspace:1
```

**Output (JSON with --json):**

```json theme={null}
{
  "workspace_id": "workspace:1"
}
```

## move-workspace-to-window

Move a workspace to a different window.

```bash theme={null}
cmux move-workspace-to-window --workspace <id|ref|index> --window <id|ref|index>
```

**Flags:**

<ParamField path="--workspace" type="string" required>
  Workspace to move (ID, ref, or index)
</ParamField>

<ParamField path="--window" type="string" required>
  Destination window (ID, ref, or index)
</ParamField>

**Examples:**

```bash theme={null}
cmux move-workspace-to-window --workspace workspace:3 --window window:2
```

**Output:**

```
OK workspace=workspace:3 window=window:2
```

## reorder-workspace

Reorder a workspace within its window.

```bash theme={null}
cmux reorder-workspace --workspace <id|ref|index> [--before <id|ref|index>] [--after <id|ref|index>] [--index <number>]
```

**Flags:**

<ParamField path="--workspace" type="string" required>
  Workspace to reorder (ID, ref, or index)
</ParamField>

<ParamField path="--window" type="string">
  Window context (if omitted, uses current window)
</ParamField>

<ParamField path="--before" type="string">
  Place before this workspace (ID, ref, or index)
</ParamField>

<ParamField path="--after" type="string">
  Place after this workspace (ID, ref, or index)
</ParamField>

<ParamField path="--index" type="number">
  Set specific index position (0-based)
</ParamField>

**Examples:**

<CodeGroup>
  ```bash Before Another theme={null}
  cmux reorder-workspace --workspace 3 --before 1
  ```

  ```bash After Another theme={null}
  cmux reorder-workspace --workspace workspace:2 --after workspace:1
  ```

  ```bash To Index theme={null}
  cmux reorder-workspace --workspace 3 --index 0
  ```
</CodeGroup>

**Output:**

```
OK workspace=workspace:3 window=window:1 index=0
```

## workspace-action

Perform various workspace actions (next, previous, last, rename).

```bash theme={null}
cmux workspace-action --action <action> [--workspace <id|ref|index>] [--title <text>]
```

**Flags:**

<ParamField path="--action" type="string" required>
  Action: `next`, `previous`, `last`, or `rename`
</ParamField>

<ParamField path="--workspace" type="string">
  Target workspace (defaults to current workspace)
</ParamField>

<ParamField path="--title" type="string">
  New title (for `rename` action only)
</ParamField>

**Examples:**

<CodeGroup>
  ```bash Next Workspace theme={null}
  cmux workspace-action --action next
  ```

  ```bash Previous Workspace theme={null}
  cmux workspace-action --action previous
  ```

  ```bash Rename theme={null}
  cmux workspace-action --action rename --title "New Name"
  ```

  ```bash Rename with Trailing Title theme={null}
  cmux workspace-action --action rename My New Title
  ```
</CodeGroup>

**Output:**

```
OK action=next workspace=workspace:2 window=window:1
```

## Global Flags

These flags can be used with most workspace commands:

<ParamField path="--socket" type="string">
  Path to the cmux socket (default: `/tmp/cmux.sock` or `$CMUX_SOCKET_PATH`)
</ParamField>

<ParamField path="--password" type="string">
  Socket authentication password (or use `$CMUX_SOCKET_PASSWORD`)
</ParamField>

<ParamField path="--json" type="boolean">
  Output results in JSON format
</ParamField>

<ParamField path="--id-format" type="string">
  Control ID output format: `refs`, `uuids`, or `both`
</ParamField>
