# [#](#title)Keyboard Shortcuts

# [#](https://cmux-docs-release.vercel.app/docs/keyboard-shortcuts#title)Keyboard Shortcuts

Default cmux keyboard shortcuts. Every cmux-owned shortcut can be changed in Settings or ~/.config/cmux/cmux.json, including two-step chords.

## [#](https://cmux-docs-release.vercel.app/docs/keyboard-shortcuts#shortcut-chords)Shortcut chords

cmux supports two-step shortcut chords in `~/.config/cmux/cmux.json`. For the full configuration schema, see the [configuration docs](https://cmux-docs-release.vercel.app/docs/configuration).

Settings can edit shortcuts directly. Use cmux.json when you want an exact tmux-style prefix binding, keep shortcuts in dotfiles, or unbind an action with null, an empty string, "none", "clear", "unbound", or "disabled".

cmux.json

```
{
  "shortcuts": {
    "bindings": {
      "newSurface": ["ctrl+b", "c"],
      "showNotifications": ["ctrl+b", "i"],
      "toggleSidebar": "cmd+b",
      "toggleFileExplorer": "cmd+opt+b",
      "splitRight": "",
      "commandPalettePrevious": null
    }
  }
}
```

-   Use a plain string for a one-step shortcut.
-   Use a two-item array for a chord. The first item is the prefix stroke, the second is the key that follows it.
-   Each item uses the same syntax as regular bindings, for example cmd+b, ctrl+b, shift+/, or ctrl+1.

[App](https://cmux-docs-release.vercel.app/docs/keyboard-shortcuts#app)·[Workspaces](https://cmux-docs-release.vercel.app/docs/keyboard-shortcuts#workspaces)·[Surfaces](https://cmux-docs-release.vercel.app/docs/keyboard-shortcuts#surfaces)·[Split Panes](https://cmux-docs-release.vercel.app/docs/keyboard-shortcuts#split-panes)·[Canvas](https://cmux-docs-release.vercel.app/docs/keyboard-shortcuts#canvas)·[Browser](https://cmux-docs-release.vercel.app/docs/keyboard-shortcuts#browser)·[Diff Viewer](https://cmux-docs-release.vercel.app/docs/keyboard-shortcuts#diff-viewer)·[Find](https://cmux-docs-release.vercel.app/docs/keyboard-shortcuts#find)·[Notifications](https://cmux-docs-release.vercel.app/docs/keyboard-shortcuts#notifications)

App

Application, window, and top-level command shortcuts.

Settings

⌘+,

Reload configuration

⌘+⇧+,

Show/hide all cmux windowssystem-wide hotkey

⌃+⌥+⌘+.

Global searchwhen cmux is active

⌥+⌘+F

Command palette

⌘+⇧+P

Command palette next resultwhen the command palette is open

⌃+N

Command palette previous resultwhen the command palette is open

⌃+P

New window

⌘+⇧+N

Close window

⌃+⌘+W

Toggle full screen

⌃+⌘+F

Send feedbackunbound by default

Reopen previous session

⌘+⇧+O

Quit cmux

⌘+Q

Workspaces

Workspaces live in the sidebar. Each workspace has its own set of panes and surfaces.

Toggle left sidebar

⌘+B

Toggle right sidebar

⌘+⌥+B

New workspace

⌘+N

New browser workspacelike New Workspace, but the first surface is a browser pane with the address bar focused

⌥+⌘+N

Save current workspace layout as a template

⌃+⌘+S

Open folder

⌘+O

Go to workspaceworkspace switcher

⌘+P

Next workspace

⌃+⌘+\]

Previous workspace

⌃+⌘+\[

Move workspace up

⌃+⌥+⌘+\[

Move workspace down

⌃+⌥+⌘+\]

Focus backFocus Back/Forward use Cmd+\[ and Cmd+\] outside browser panes; browser Back/Forward use the same defaults inside browser panes. Unbind Focus Back/Forward to let terminal shortcuts handle those keys.

⌘+\[

Focus forwardFocus Back/Forward use Cmd+\[ and Cmd+\] outside browser panes; browser Back/Forward use the same defaults inside browser panes. Unbind Focus Back/Forward to let terminal shortcuts handle those keys.

⌘+\]

Select workspace 1…9

⌘+1…9

Rename workspace

⌘+⇧+R

Edit workspace description

⌥+⌘+E

Mark workspace as done

⌘+;

Cycle workspace status one lane forward

⌘+⇧+;

Toggle the highlighted checklist itemApplies in the focused todo pane or checklist popover.

⌘+↩

New empty workspace group

⌃+⌘+G

Group selected workspaces

⌘+⇧+G

Collapse or expand focused workspace group

⌃+⌘+.

Toggle right-sidebar focus

⌘+⇧+E

Navigate focused sidebar rowsIn Files, H/L collapse and expand folders. Search starts with /.

J / K/⌃+N / P/H / L

Open selected file or toggle folderfocused file explorer

↩

Open selected file or toggle folderFinder-style alias for the focused file explorer

⌘+↓

Close workspace

⌘+⇧+W

Reopen closed workspaceunbound by default

Surfaces

Surfaces are tabs inside a pane.

New surface

⌘+T

Next surface

⌘+⇧+\]

Previous surface

⌘+⇧+\[

Reorder surface left

⌥+⌘+⇧+\[

Reorder surface right

⌥+⌘+⇧+\]

Move surface to previous pane

⌃+⌘+⇧+\[

Move surface to next pane

⌃+⌘+⇧+\]

Move surface to pane on left

⌥+⌘+⇧+←

Move surface to pane on right

⌥+⌘+⇧+→

Move surface to pane above

⌥+⌘+⇧+↑

Move surface to pane below

⌥+⌘+⇧+↓

Select surface 1…9

⌃+1…9

Rename tab

⌘+R

Close tab

⌘+W

Close other tabs in pane

⌥+⌘+T

Reopen last closed

⌘+⇧+T

Toggle terminal copy mode

⌘+⇧+M

Clear screen (keep scrollback)

⌘+⇧+K

Simulator: Homefocused Simulator

⌘+⇧+H

Simulator: rotate leftfocused Simulator

⌘+←

Simulator: rotate rightfocused Simulator

⌘+→

Simulator: toggle appearancefocused Simulator

⌘+⇧+A

Simulator: toggle software keyboardfocused Simulator

⌘+K

Switch focus between terminal and TextBox input

⌘+⇧+A

Cycle TextBox submit action

⇧+Tab

Attach file to TextBox input

⌥+⌘+⇧+A

Send Ctrl-F to terminalunbound by default; forwards Ctrl-F to the focused terminal (Claude Code: invoke twice to force-stop hung background agents)

Save file previewfocused text preview

⌘+S

Split Panes

Focus pane left

⌥+⌘+←

Focus pane right

⌥+⌘+→

Focus pane up

⌥+⌘+↑

Focus pane down

⌥+⌘+↓

Focus previous pane (cycle)unbound by default; a Ghostty goto\_split:previous keybind also cycles panes while Focus Back does not claim the same keys

Focus next pane (cycle)unbound by default; a Ghostty goto\_split:next keybind also cycles panes while Focus Forward does not claim the same keys

Split right

⌘+D

Split down

⌘+⇧+D

Split browser right

⌥+⌘+D

Split browser down

⌥+⌘+⇧+D

Toggle pane zoom

⌘+⇧+↩

Increase font size for every terminal in the selected workspace

⌃+⌘+\=

Decrease font size for every terminal in the selected workspace

⌃+⌘+\-

Reset font size for every terminal in the selected workspace

⌃+⌘+0

Equalize split sizes

⌃+⌘+⇧+\=

Canvas

A freeform 2D layout mode where panes float on an infinite, pannable canvas. Alignment and distribution commands are available from the command palette.

Toggle canvas layout

⌃+⌘+C

Reveal focused pane

⌃+⌘+R

Toggle overview zoom

⌃+⌘+O

Zoom in

⌥+⌘+\=

Zoom out

⌥+⌘+\-

Actual size

⌘+0

Tidy panes into a grid

⌃+⌘+T

Browser

Open browser

⌘+⇧+L

Focus address bar

⌘+L

Back

⌘+\[

Forward

⌘+\]

Reload pagefocused browser

⌘+R

Hard refresh pagefocused browser

⌘+⇧+R

Zoom in

⌘+\=

Zoom out

⌘+\-

Actual size

⌘+0

Markdown viewer: zoom infocused markdown viewer

⌘+\=

Markdown viewer: zoom outfocused markdown viewer

⌘+\-

Markdown viewer: actual sizefocused markdown viewer

⌘+0

Toggle browser developer tools

⌥+⌘+I

Show browser JavaScript console

⌥+⌘+C

Enter browser focus modeGives the focused web page first claim on shortcuts. Press Esc twice to exit.

⌥+⌘+↩

Toggle browser design modeSelect and visually edit elements in the focused browser

⌃+⌥+⌘+D

Toggle React Grabfocused browser, or the only browser pane when a terminal is focused

⌘+⇧+G

Diff Viewer

Open diff viewer

⌃+⌘+⇧+D

Scroll viewer down one smooth stepfocused diff or Markdown viewer

J

Scroll viewer up one smooth stepfocused diff or Markdown viewer

K

Scroll viewer down half a pagefocused diff or Markdown viewer

⌃+D

Scroll viewer up half a pagefocused diff or Markdown viewer

⌃+U

Scroll viewer down one smooth step (Emacs)focused diff or Markdown viewer

⌃+N

Scroll viewer up one smooth step (Emacs)focused diff or Markdown viewer

⌃+P

Scroll diff to bottomfocused diff viewer

⇧+G

Scroll diff to topfocused diff viewer

G+G

Open diff file searchfocused diff viewer

/

Jump to next diff filefocused diff viewer

\]+F

Jump to previous diff filefocused diff viewer

\[+F

Find

Find

⌘+F

Find in directory

⌘+⇧+F

Find next

⌘+G

Find previous

⌥+⌘+G

Hide find bar

⌥+⌘+⇧+F

Use selection for find

⌘+E

Notifications

Show notifications

⌘+I

Jump to latest unread

⌘+⇧+U

Toggle current item unread state

⌥+⌘+U

Mark current item as oldest unread and jump to the next latest unread

⌃+⌘+U

Flash focused panel

⌘+⇧+H

[Dock](https://cmux-docs-release.vercel.app/docs/dock) [CLI Reference](https://cmux-docs-release.vercel.app/docs/api)

Canonical: https://cmux-docs-release.vercel.app/docs/keyboard-shortcuts
