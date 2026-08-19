> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Splits and panes

> Split terminals and browsers horizontally and vertically to work on multiple tasks simultaneously

cmux supports flexible split pane layouts within each workspace, allowing you to view multiple terminals and browsers side by side.

## Creating splits

### Terminal splits

Split the current terminal pane:

| Shortcut                               | Direction                       |
| -------------------------------------- | ------------------------------- |
| <kbd>⌘</kbd> <kbd>D</kbd>              | Split right (vertical divider)  |
| <kbd>⌘</kbd> <kbd>⇧</kbd> <kbd>D</kbd> | Split down (horizontal divider) |

Or use the split buttons in the terminal UI:

* Click the "+" button and select split direction
* Right-click the terminal tab bar for split options

### Browser splits

Create a browser in a split pane:

| Shortcut                                            | Direction           |
| --------------------------------------------------- | ------------------- |
| <kbd>⌥</kbd> <kbd>⌘</kbd> <kbd>D</kbd>              | Split browser right |
| <kbd>⌥</kbd> <kbd>⌘</kbd> <kbd>⇧</kbd> <kbd>D</kbd> | Split browser down  |

This is useful for viewing your dev server alongside the terminal running the build.

<Info>
  Browser splits inherit the Ghostty terminal background color and opacity for a unified appearance.
</Info>

## Navigating between panes

Move focus between split panes using directional shortcuts:

| Shortcut                               | Direction        |
| -------------------------------------- | ---------------- |
| <kbd>⌥</kbd> <kbd>⌘</kbd> <kbd>←</kbd> | Focus pane left  |
| <kbd>⌥</kbd> <kbd>⌘</kbd> <kbd>→</kbd> | Focus pane right |
| <kbd>⌥</kbd> <kbd>⌘</kbd> <kbd>↑</kbd> | Focus pane up    |
| <kbd>⌥</kbd> <kbd>⌘</kbd> <kbd>↓</kbd> | Focus pane down  |

<Note>
  Pane focus is directional and wraps around edges. If you're in the rightmost pane and press <kbd>⌥</kbd> <kbd>⌘</kbd> <kbd>→</kbd>, focus moves to the leftmost pane.
</Note>

## Pane zoom

Temporarily maximize a pane to full screen:

* Press <kbd>⌘</kbd> <kbd>⇧</kbd> <kbd>↩</kbd> to toggle zoom
* The zoomed pane fills the entire workspace
* Press the shortcut again to restore the split layout

Pane zoom is useful when you need to focus on a single terminal or browser without closing other panes.

## Flash focused panel

If you lose track of which pane has focus in a complex split layout:

* Press <kbd>⌘</kbd> <kbd>⇧</kbd> <kbd>H</kbd> to flash the focused panel
* The active pane briefly highlights so you can see where focus is

## Divider manipulation

Adjust split pane sizes by dragging the divider between panes:

1. Hover over the divider between two panes
2. Click and drag to resize
3. Release to set the new size

Divider positions are saved in session state and restored on app relaunch.

## Closing panes

Close a split pane:

* Press <kbd>⌘</kbd> <kbd>W</kbd> to close the active surface
* Click the close button (×) on the tab
* Use the CLI: `cmux close-surface`

When you close a pane:

* The adjacent pane expands to fill the space
* If only one pane remains, it fills the entire workspace
* The last pane in a workspace cannot be closed (cmux creates a new terminal automatically)

<Warning>
  cmux does not allow closing the last pane in a workspace. If you close all surfaces, a new terminal is created automatically to keep the workspace alive.
</Warning>

## Split layouts

cmux supports arbitrarily nested splits:

```
┌─────────────┬──────────┐
│             │          │
│   Terminal  │ Browser  │
│             │          │
├─────────────┴──────────┤
│                        │
│   Terminal (full width)│
│                        │
└────────────────────────┘
```

Create complex layouts by:

1. Starting with a single pane
2. Splitting horizontally or vertically
3. Splitting the new panes again as needed

Each pane can contain multiple surfaces (tabs), so you can have horizontal tabs within each split section.

## Tabs within panes

Each split pane can contain multiple surfaces as tabs:

* Press <kbd>⌘</kbd> <kbd>T</kbd> to create a new tab in the active pane
* Drag tabs between panes to reorganize
* Click tabs to switch between surfaces in the same pane

This gives you flexibility to organize related tasks together while keeping unrelated tasks in separate panes.

## Font size and zoom

Terminal font size is inherited when you create splits:

* Zoom in/out with <kbd>⌘</kbd> <kbd>+</kbd> / <kbd>⌘</kbd> <kbd>-</kbd>
* Reset with <kbd>⌘</kbd> <kbd>0</kbd>
* New splits inherit the current terminal's font size

This ensures consistent text size across related panes even if you zoom the parent terminal before splitting.

## CLI for split management

Create splits programmatically via the socket API:

```bash theme={null}
# Split the active pane right
cmux new-split right

# Split down
cmux new-split down

# Create a browser pane
cmux new-pane --type browser --direction right
```

This is useful for scripting workspace layouts:

```bash theme={null}
#!/bin/bash
# Set up a full-stack dev workspace
cmux new-workspace
cmux new-pane --type browser --direction right
cmux send "npm run dev\n"
```

## Drag and drop

Reorganize panes by dragging tabs:

* Drag a tab to another pane to move it
* Drag a tab to the edge of a pane to create a new split
* Drag a tab out of the window to detach it to a new window (if enabled)

<Info>
  Tab drag-and-drop uses custom UTTypes declared in `Info.plist` for internal transfers. External drops are not supported.
</Info>
