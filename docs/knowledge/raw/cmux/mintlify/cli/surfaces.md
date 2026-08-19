> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Surface Commands

> Manage terminal and browser surfaces (panels) in cmux

Surfaces are individual terminal or browser panels within panes. Each pane can contain one or more surfaces stacked vertically.

## list-panels

List all surfaces in the current or specified workspace (legacy alias for surface listing).

```bash theme={null}
cmux list-panels [--workspace <id|ref|index>]
```

**Flags:**

<ParamField path="--workspace" type="string">
  Workspace to list surfaces from (defaults to `$CMUX_WORKSPACE_ID`)
</ParamField>

<ParamField path="--json" type="boolean">
  Output results in JSON format
</ParamField>

<ParamField path="--id-format" type="string">
  Control ID output format: `refs`, `uuids`, or `both`
</ParamField>

**Output (text):**

```
* surface:1  terminal  [focused]  "~/projects/app"
  surface:2  browser
  surface:3  terminal  "npm run dev"
```

**Output (JSON):**

```json theme={null}
{
  "surfaces": [
    {
      "ref": "surface:1",
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "type": "terminal",
      "title": "~/projects/app",
      "focused": true,
      "index": 1
    }
  ]
}
```

## new-surface

Create a new surface in the current or specified pane.

```bash theme={null}
cmux new-surface [--workspace <id|ref|index>] [--pane <id|ref|index>] [--type <terminal|browser>] [--url <url>]
```

**Flags:**

<ParamField path="--workspace" type="string">
  Workspace context (defaults to `$CMUX_WORKSPACE_ID`)
</ParamField>

<ParamField path="--pane" type="string">
  Pane to add surface to (defaults to current pane)
</ParamField>

<ParamField path="--type" type="string">
  Surface type: `terminal` or `browser`
</ParamField>

<ParamField path="--url" type="string">
  URL to open (for browser surfaces)
</ParamField>

**Examples:**

<CodeGroup>
  ```bash Terminal in Current Pane theme={null}
  cmux new-surface
  ```

  ```bash Browser Surface theme={null}
  cmux new-surface --type browser --url https://example.com
  ```

  ```bash In Specific Pane theme={null}
  cmux new-surface --pane pane:2 --type terminal
  ```
</CodeGroup>

**Output:**

```
OK surface=surface:4 pane=pane:1 workspace=workspace:1
```

## close-surface

Close a specific surface.

```bash theme={null}
cmux close-surface [--workspace <id|ref|index>] [--surface <id|ref|index>]
```

**Flags:**

<ParamField path="--workspace" type="string">
  Workspace context (defaults to `$CMUX_WORKSPACE_ID`)
</ParamField>

<ParamField path="--surface" type="string">
  Surface to close (defaults to `$CMUX_SURFACE_ID`)
</ParamField>

<ParamField path="--panel" type="string">
  Legacy alias for `--surface`
</ParamField>

**Examples:**

<CodeGroup>
  ```bash Current Surface theme={null}
  cmux close-surface
  ```

  ```bash Specific Surface theme={null}
  cmux close-surface --surface surface:3
  ```

  ```bash By Index theme={null}
  cmux close-surface --surface 2
  ```
</CodeGroup>

**Output:**

```
OK
```

## focus-panel

Focus a specific surface (panel).

```bash theme={null}
cmux focus-panel --panel <id|ref|index> [--workspace <id|ref|index>]
```

**Flags:**

<ParamField path="--panel" type="string" required>
  Surface to focus (ID, ref, or index)
</ParamField>

<ParamField path="--workspace" type="string">
  Workspace context (defaults to `$CMUX_WORKSPACE_ID`)
</ParamField>

**Examples:**

<CodeGroup>
  ```bash By Ref theme={null}
  cmux focus-panel --panel surface:2
  ```

  ```bash By Index theme={null}
  cmux focus-panel --panel 1
  ```

  ```bash With Workspace theme={null}
  cmux focus-panel --panel 2 --workspace workspace:1
  ```
</CodeGroup>

**Output:**

```
OK
```

## new-split

Create a new split pane with a surface.

```bash theme={null}
cmux new-split <direction> [--workspace <id|ref|index>] [--surface <id|ref|index>]
```

**Arguments:**

<ParamField path="direction" type="string" required>
  Split direction: `left`, `right`, `up`, or `down`
</ParamField>

**Flags:**

<ParamField path="--workspace" type="string">
  Workspace context (defaults to `$CMUX_WORKSPACE_ID`)
</ParamField>

<ParamField path="--surface" type="string">
  Reference surface to split from (defaults to `$CMUX_SURFACE_ID`)
</ParamField>

<ParamField path="--panel" type="string">
  Legacy alias for `--surface`
</ParamField>

**Examples:**

<CodeGroup>
  ```bash Right Split theme={null}
  cmux new-split right
  ```

  ```bash Down Split theme={null}
  cmux new-split down
  ```

  ```bash Split Specific Surface theme={null}
  cmux new-split left --surface surface:2
  ```
</CodeGroup>

**Output:**

```
OK
```

## move-surface

Move a surface to a different pane, workspace, or window.

```bash theme={null}
cmux move-surface --surface <id|ref|index> [--pane <id>] [--workspace <id>] [--window <id>] [--index <number>] [--focus <true|false>]
```

**Flags:**

<ParamField path="--surface" type="string" required>
  Surface to move (ID, ref, or index)
</ParamField>

<ParamField path="--pane" type="string">
  Destination pane (ID, ref, or index)
</ParamField>

<ParamField path="--workspace" type="string">
  Destination workspace (ID, ref, or index)
</ParamField>

<ParamField path="--window" type="string">
  Destination window (ID, ref, or index)
</ParamField>

<ParamField path="--before" type="string">
  Place before this surface
</ParamField>

<ParamField path="--after" type="string">
  Place after this surface
</ParamField>

<ParamField path="--index" type="number">
  Set specific index position
</ParamField>

<ParamField path="--focus" type="boolean">
  Whether to focus the surface after moving
</ParamField>

**Examples:**

<CodeGroup>
  ```bash To Different Pane theme={null}
  cmux move-surface --surface surface:1 --pane pane:2
  ```

  ```bash To Different Workspace theme={null}
  cmux move-surface --surface 1 --workspace 2 --focus true
  ```

  ```bash With Index theme={null}
  cmux move-surface --surface surface:3 --pane pane:1 --index 0
  ```
</CodeGroup>

**Output:**

```
OK surface=surface:1 pane=pane:2 workspace=workspace:1 window=window:1
```

## reorder-surface

Reorder a surface within its pane.

```bash theme={null}
cmux reorder-surface --surface <id|ref|index> [--before <id>] [--after <id>] [--index <number>]
```

**Flags:**

<ParamField path="--surface" type="string" required>
  Surface to reorder (ID, ref, or index)
</ParamField>

<ParamField path="--workspace" type="string">
  Workspace context
</ParamField>

<ParamField path="--before" type="string">
  Place before this surface
</ParamField>

<ParamField path="--after" type="string">
  Place after this surface
</ParamField>

<ParamField path="--index" type="number">
  Set specific index position
</ParamField>

**Examples:**

<CodeGroup>
  ```bash Before Another theme={null}
  cmux reorder-surface --surface 2 --before 1
  ```

  ```bash To Index theme={null}
  cmux reorder-surface --surface surface:3 --index 0
  ```
</CodeGroup>

**Output:**

```
OK surface=surface:2 pane=pane:1 workspace=workspace:1
```

## surface-health

Check the health status of all surfaces in a workspace.

```bash theme={null}
cmux surface-health [--workspace <id|ref|index>]
```

**Flags:**

<ParamField path="--workspace" type="string">
  Workspace to check (defaults to `$CMUX_WORKSPACE_ID`)
</ParamField>

<ParamField path="--json" type="boolean">
  Output results in JSON format
</ParamField>

**Output (text):**

```
surface:1  type=terminal in_window=true
surface:2  type=browser in_window=true
surface:3  type=terminal in_window=false
```

**Output (JSON):**

```json theme={null}
{
  "surfaces": [
    {
      "ref": "surface:1",
      "type": "terminal",
      "in_window": true
    }
  ]
}
```

## trigger-flash

Trigger a visual flash on a surface (for debugging).

```bash theme={null}
cmux trigger-flash [--workspace <id>] [--surface <id>]
```

**Flags:**

<ParamField path="--workspace" type="string">
  Workspace context (defaults to `$CMUX_WORKSPACE_ID`)
</ParamField>

<ParamField path="--surface" type="string">
  Surface to flash (defaults to `$CMUX_SURFACE_ID`)
</ParamField>

**Output:**

```
OK
```

## refresh-surfaces

Refresh all surfaces (legacy command).

```bash theme={null}
cmux refresh-surfaces
```

**Output:**

```
OK
```

## drag-surface-to-split

Drag a surface to create a split in a specific direction (legacy command).

```bash theme={null}
cmux drag-surface-to-split --surface <id|index> <direction>
```

**Flags:**

<ParamField path="--surface" type="string" required>
  Surface to drag (ID or index)
</ParamField>

<ParamField path="--panel" type="string">
  Legacy alias for `--surface`
</ParamField>

**Arguments:**

<ParamField path="direction" type="string" required>
  Split direction: `left`, `right`, `up`, or `down`
</ParamField>

**Example:**

```bash theme={null}
cmux drag-surface-to-split --surface 1 right
```

**Output:**

```
OK
```

## Global Flags

These flags can be used with most surface commands:

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
