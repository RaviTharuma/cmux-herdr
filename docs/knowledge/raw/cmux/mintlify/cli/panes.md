> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Pane Commands

> Manage panes (split containers) in cmux workspaces

Panes are containers for surfaces within a workspace. Each workspace has a split layout tree of panes.

## list-panes

List all panes in the current or specified workspace.

```bash theme={null}
cmux list-panes [--workspace <id|ref|index>]
```

**Flags:**

<ParamField path="--workspace" type="string">
  Workspace to list panes from (defaults to `$CMUX_WORKSPACE_ID`)
</ParamField>

<ParamField path="--json" type="boolean">
  Output results in JSON format
</ParamField>

<ParamField path="--id-format" type="string">
  Control ID output format: `refs`, `uuids`, or `both`
</ParamField>

**Output (text):**

```
* pane:1  [2 surfaces]  [focused]
  pane:2  [1 surface]
  pane:3  [3 surfaces]
```

**Output (JSON):**

```json theme={null}
{
  "panes": [
    {
      "ref": "pane:1",
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "index": 1,
      "focused": true,
      "surface_count": 2
    }
  ]
}
```

## new-pane

Create a new pane with a split direction.

```bash theme={null}
cmux new-pane [--workspace <id|ref|index>] [--direction <direction>] [--type <terminal|browser>] [--url <url>]
```

**Flags:**

<ParamField path="--workspace" type="string">
  Workspace context (defaults to `$CMUX_WORKSPACE_ID`)
</ParamField>

<ParamField path="--direction" type="string">
  Split direction: `left`, `right`, `up`, or `down` (default: `right`)
</ParamField>

<ParamField path="--type" type="string">
  Initial surface type: `terminal` or `browser`
</ParamField>

<ParamField path="--url" type="string">
  URL to open (for browser surfaces)
</ParamField>

**Examples:**

<CodeGroup>
  ```bash Default Right Split theme={null}
  cmux new-pane
  ```

  ```bash Down Split theme={null}
  cmux new-pane --direction down
  ```

  ```bash Browser Pane theme={null}
  cmux new-pane --type browser --url https://example.com
  ```

  ```bash In Specific Workspace theme={null}
  cmux new-pane --workspace workspace:2 --direction left
  ```
</CodeGroup>

**Output:**

```
OK surface=surface:5 pane=pane:3 workspace=workspace:1
```

## focus-pane

Focus a specific pane by ID, ref, or index.

```bash theme={null}
cmux focus-pane --pane <id|ref|index> [--workspace <id|ref|index>]
```

**Flags:**

<ParamField path="--pane" type="string" required>
  Pane to focus (ID, ref, or index)
</ParamField>

<ParamField path="--workspace" type="string">
  Workspace context (defaults to `$CMUX_WORKSPACE_ID`)
</ParamField>

**Examples:**

<CodeGroup>
  ```bash By Ref theme={null}
  cmux focus-pane --pane pane:2
  ```

  ```bash By Index theme={null}
  cmux focus-pane --pane 1
  ```

  ```bash Positional Argument theme={null}
  cmux focus-pane pane:2
  ```

  ```bash With Workspace theme={null}
  cmux focus-pane --pane 2 --workspace workspace:1
  ```
</CodeGroup>

**Output:**

```
OK pane=pane:2 workspace=workspace:1
```

## list-pane-surfaces

List all surfaces within a specific pane.

```bash theme={null}
cmux list-pane-surfaces --pane <id|ref|index> [--workspace <id|ref|index>]
```

**Flags:**

<ParamField path="--pane" type="string" required>
  Pane to list surfaces from (ID, ref, or index)
</ParamField>

<ParamField path="--workspace" type="string">
  Workspace context (defaults to `$CMUX_WORKSPACE_ID`)
</ParamField>

<ParamField path="--json" type="boolean">
  Output results in JSON format
</ParamField>

<ParamField path="--id-format" type="string">
  Control ID output format: `refs`, `uuids`, or `both`
</ParamField>

**Examples:**

<CodeGroup>
  ```bash By Ref theme={null}
  cmux list-pane-surfaces --pane pane:1
  ```

  ```bash By Index with JSON theme={null}
  cmux list-pane-surfaces --pane 2 --json
  ```
</CodeGroup>

**Output (text):**

```
* surface:1  ~/projects/app  [selected]
  surface:2  npm run dev
  surface:3  git status
```

**Output (JSON):**

```json theme={null}
{
  "surfaces": [
    {
      "ref": "surface:1",
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "title": "~/projects/app",
      "selected": true,
      "index": 1
    }
  ]
}
```

## tree

Display the pane/surface hierarchy tree for a workspace.

```bash theme={null}
cmux tree [--workspace <id|ref|index>]
```

**Flags:**

<ParamField path="--workspace" type="string">
  Workspace to display tree for (defaults to current workspace)
</ParamField>

<ParamField path="--json" type="boolean">
  Output results in JSON format
</ParamField>

<ParamField path="--id-format" type="string">
  Control ID output format: `refs`, `uuids`, or `both`
</ParamField>

**Example:**

```bash theme={null}
cmux tree
```

**Output:**
Displays a hierarchical tree view of panes and surfaces in the workspace.

## Global Flags

These flags can be used with most pane commands:

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
