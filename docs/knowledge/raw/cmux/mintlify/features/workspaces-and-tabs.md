> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Workspaces and tabs

> Organize terminal sessions with workspaces, vertical tabs, and metadata tracking

Workspaces are cmux's primary organizational unit. Each workspace appears as a vertical tab in the sidebar and can contain multiple surfaces (terminals and browsers) arranged in split panes.

## Workspaces

Each workspace is an independent environment that tracks:

* Working directory
* Git branch and dirty state
* Listening ports
* Process title
* Custom color and icon
* Status metadata and logs
* Progress indicators

<Note>
  Workspaces are the equivalent of "windows" in traditional terminals but appear as vertical tabs in the sidebar instead of separate OS windows.
</Note>

### Creating workspaces

Create a new workspace:

* Press <kbd>⌘</kbd> <kbd>N</kbd>
* Use the CLI: `cmux new-workspace`
* Click the "+" button in the sidebar

New workspaces inherit the current working directory unless you specify a different path:

```bash theme={null}
cmux new-workspace --cwd ~/projects/myapp
```

### Workspace metadata

The sidebar displays rich metadata for each workspace:

* **Git branch**: Automatically detected from the working directory
* **Dirty indicator**: Shows when you have uncommitted changes
* **Pull request**: Linked PR number and status (open/merged/closed)
* **Working directory**: Current directory path
* **Listening ports**: Ports bound by processes in this workspace
* **Latest notification**: Most recent notification text from agents

<Info>
  Workspace metadata updates automatically as you work. Git status, ports, and directory changes are detected in real-time.
</Info>

### Custom titles and colors

Personalize workspaces with custom titles and colors:

```bash theme={null}
# Rename workspace
cmux rename-workspace "Frontend Dev"
```

<Note>
  Workspace colors are set through the in-app settings, not via CLI.
</Note>

Or use keyboard shortcuts:

* <kbd>⌘</kbd> <kbd>⇧</kbd> <kbd>R</kbd> to rename the current workspace

### Pinned workspaces

Pin important workspaces to keep them at the top of the sidebar using the workspace context menu in the sidebar. Pinned workspaces stay in position even when new notifications arrive in other workspaces.

## Surfaces (tabs within a workspace)

Each workspace contains one or more surfaces. A surface is either a terminal or browser pane.

### Creating surfaces

* Press <kbd>⌘</kbd> <kbd>T</kbd> to create a new terminal surface
* Press <kbd>⌘</kbd> <kbd>⇧</kbd> <kbd>L</kbd> to create a new browser surface

Surfaces appear as horizontal tabs within the active pane.

### Navigating surfaces

| Shortcut                                 | Action               |
| ---------------------------------------- | -------------------- |
| <kbd>⌘</kbd> <kbd>⇧</kbd> <kbd>]</kbd>   | Next surface         |
| <kbd>⌘</kbd> <kbd>⇧</kbd> <kbd>\[</kbd>  | Previous surface     |
| <kbd>⌃</kbd> <kbd>Tab</kbd>              | Next surface         |
| <kbd>⌃</kbd> <kbd>⇧</kbd> <kbd>Tab</kbd> | Previous surface     |
| <kbd>⌃</kbd> <kbd>1</kbd>–<kbd>8</kbd>   | Jump to surface 1–8  |
| <kbd>⌃</kbd> <kbd>9</kbd>                | Jump to last surface |
| <kbd>⌘</kbd> <kbd>W</kbd>                | Close surface        |

<Warning>
  Closing the last surface in a workspace does not close the workspace. cmux automatically creates a new terminal surface to keep the workspace alive.
</Warning>

## Workspace navigation

### Switching workspaces

| Shortcut                                | Action                 |
| --------------------------------------- | ---------------------- |
| <kbd>⌘</kbd> <kbd>1</kbd>–<kbd>8</kbd>  | Jump to workspace 1–8  |
| <kbd>⌘</kbd> <kbd>9</kbd>               | Jump to last workspace |
| <kbd>⌃</kbd> <kbd>⌘</kbd> <kbd>]</kbd>  | Next workspace         |
| <kbd>⌃</kbd> <kbd>⌘</kbd> <kbd>\[</kbd> | Previous workspace     |

Click any workspace tab in the sidebar to switch to it.

### Closing workspaces

Close a workspace:

* Press <kbd>⌘</kbd> <kbd>⇧</kbd> <kbd>W</kbd>
* Use the CLI: `cmux close-workspace --workspace <id>`
* Click the close button (×) on the workspace tab

If the workspace has running processes, cmux prompts for confirmation before closing.

## Session restore

cmux automatically saves workspace state on quit and restores it on relaunch:

**Restored state:**

* Workspace layout and split pane structure
* Working directories
* Terminal scrollback (best effort, up to 5000 lines)
* Browser URLs and navigation history
* Custom titles, colors, and pinned state
* Metadata (git branch, ports, etc.)

**Not restored:**

* Live process state (tmux/vim sessions don't resume)
* Active SSH connections
* Running builds or long-running commands

<Info>
  Terminal processes restart in the same working directory but don't resume mid-execution. Use tmux or screen if you need persistent sessions across app restarts.
</Info>

## Sidebar

The vertical sidebar displays all workspaces and can be toggled with <kbd>⌘</kbd> <kbd>B</kbd>.

Sidebar features:

* Drag tabs to reorder workspaces
* Context menu on each tab for quick actions
* Visual indicators for notifications, git state, and activity
* Auto-reorder workspaces on notification (optional)

## Working directory tracking

cmux automatically detects directory changes in terminal sessions and updates:

* The workspace's current directory
* Git branch detection in the new directory
* Relative path display in the sidebar

Directory tracking works by monitoring shell integration sequences (OSC 7) and process working directories.

## Command palette

Access the command palette with <kbd>⌘</kbd> <kbd>⇧</kbd> <kbd>P</kbd> to quickly:

* Switch between workspaces across all windows
* Access update actions
* Execute common workspace operations

The command palette provides fuzzy search across all available commands and workspace tabs.
