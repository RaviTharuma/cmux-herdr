# [#](#title)Workspace Groups

# [#](https://cmux-docs-release.vercel.app/docs/workspace-groups#title)Workspace Groups

Workspace groups let you nest workspaces into collapsible named sections in the sidebar. Each group has an implicit anchor workspace, a customizable + button for spawning new workspaces inside it, and right-click actions for renaming, pinning, ungrouping, and editing its configuration.

## [#](https://cmux-docs-release.vercel.app/docs/workspace-groups#concepts)Concepts

### [#](https://cmux-docs-release.vercel.app/docs/workspace-groups#anchor-workspace)Anchor workspace

Every group is owned by exactly one workspace called the anchor. The group header in the sidebar is the anchor's representation — there is no separate row for it. Clicking the header name area focuses the anchor's panels; clicking the chevron toggles collapse.

A group's anchor is always a brand new workspace at creation time; grouping a selection or creating a group never promotes one of your existing workspaces into the anchor. (Closing an anchor later does promote the group's first remaining member, see below.) The anchor's working directory is inherited from the first selected workspace when grouping a selection, or from the active workspace when creating via the CLI without --cwd.

Closing the anchor workspace closes only that workspace and promotes the group's first remaining member (the earliest one still in the group's order) to be the new anchor, so the group and its other members stay intact (the promoted member then shows the group name as the header). When the anchor is the group's only workspace, the group is removed. To flatten a group into ungrouped workspaces use Ungroup; to close every workspace in a group use Delete Group.

### [#](https://cmux-docs-release.vercel.app/docs/workspace-groups#group-identity)Group identity

A group has a name, an icon (an SF Symbol, default folder.fill), and an optional custom color. These are independent of the anchor workspace's own customizations. The anchor's color and icon are seeded from the group on creation, but they can diverge afterwards.

### [#](https://cmux-docs-release.vercel.app/docs/workspace-groups#pinning)Pinning

Groups can be pinned independently of individual workspace pins. Pinned top-level rows, whether individual workspaces or groups, stay above unpinned rows. Within each tier, groups and workspaces keep the order you drag them into.

The sidebar lays out top-level rows top to bottom:

1.  Pinned top-level rows (workspaces and groups).
2.  Unpinned top-level rows (workspaces and groups).

## [#](https://cmux-docs-release.vercel.app/docs/workspace-groups#creating-a-group)Creating a group

A group is created from a keyboard shortcut or a sidebar context menu. Empty groups insert a fresh anchor workspace as the group header. Selection-based groups insert that anchor above the selection and move the selected workspaces into the group. Once a group exists, you manage it and add workspaces to it from the group header (see Managing a group below).

### [#](https://cmux-docs-release.vercel.app/docs/workspace-groups#from-the-keyboard)From keyboard shortcuts

Press ⌃⌘G to create a new empty workspace group. Select two or more workspaces in the sidebar, then press ⌘⇧G to group the selection. Both paths create a fresh anchor workspace and auto-name the group Group 1, Group 2, and so on. Rename it anytime via the header context menu.

⌘⇧G collides with React Grab's default. The group handler only consumes the chord when there is an explicit sidebar multi-selection of at least two workspaces, so React Grab still fires in single-selection and in browser or terminal contexts. Rebind it in Settings → Keyboard if you would rather the two not share a key.

Single-tab groups are not created from the shortcut. Use the workspace context menu New Group from Workspace entry for that.

### [#](https://cmux-docs-release.vercel.app/docs/workspace-groups#from-a-workspace-context-menu)From a workspace context menu

Right-click any workspace in the sidebar and choose New Empty Workspace Group, New Group from Workspace, or New Group from Selection when multiple workspaces are selected. The blank area below the sidebar list also offers New Empty Workspace Group. These paths use the same auto-naming behavior as the shortcuts.

## [#](https://cmux-docs-release.vercel.app/docs/workspace-groups#managing-a-group)Managing a group

Once a group exists, its header context menu and the + button on the header let you manage the group and add workspaces to it. Neither creates a new group.

### [#](https://cmux-docs-release.vercel.app/docs/workspace-groups#from-the-group-header-context-menu)From the group header context menu

Right-click an existing group header for Rename Group…, Pin Group / Unpin Group, Edit Group Config… (which opens ~/.config/cmux/cmux.json), Open Workspace Groups Docs, Ungroup Workspaces, and Delete Group. Ungroup Workspaces keeps the workspaces and removes only the group container. Delete Group closes the group header workspace and every workspace inside the group; if the group contains child workspaces, it prompts for confirmation first.

### [#](https://cmux-docs-release.vercel.app/docs/workspace-groups#from-the-plus-button)From the + button on a group header

Hover over a group header to reveal a trailing + button. Click it to create a new workspace in the group at the anchor working directory. Right-click it for New Workspace in Group, Edit Group Config…, and Open Workspace Groups Docs.

Pressing ⌘N while the active workspace is a group anchor or member also creates the new workspace inside that group. The default placement is After current: from a regular group member the new workspace lands right after the active member, and from the anchor or header it lands at the top of the group.

## [#](https://cmux-docs-release.vercel.app/docs/workspace-groups#cli)CLI

All group operations are scriptable with the cmux workspace-group subcommands. The hyphenated form ships first; once the broader cmux workspace command namespace lands, cmux workspace group becomes the canonical form, with the hyphenated form kept as an alias forever.

### [#](https://cmux-docs-release.vercel.app/docs/workspace-groups#subcommands)Subcommands

```
cmux workspace-group list [--json]
cmux workspace-group create --name "manaflow" [--cwd ~/projects/manaflow] [--from <id>,<id>]
cmux workspace-group ungroup <group-id>
cmux workspace-group delete  <group-id>
cmux workspace-group delete  <group-id> --close-workspaces
cmux workspace-group rename <group-id> --name "new name"
cmux workspace-group collapse <group-id>
cmux workspace-group expand <group-id>
cmux workspace-group pin <group-id>
cmux workspace-group unpin <group-id>
cmux workspace-group add --group <group-id> --workspace <workspace-id>
cmux workspace-group remove --workspace <workspace-id>
cmux workspace-group set-anchor --group <group-id> --workspace <workspace-id>
cmux workspace-group new-workspace <group-id> [--placement afterCurrent|top|end]
cmux workspace-group set-color <group-id> --hex "#7A4FD8"
cmux workspace-group set-icon  <group-id> --symbol ladybug.fill
cmux workspace-group move <group-id> (--to-index <n> | --before <group-id> | --after <group-id>)
cmux workspace-group focus <group-id>
```

create returns a group handle (workspace\_group:N by default). Omitting --from creates an anchor-only group; existing workspaces are never inferred from selection or caller context. Pass --json for the full structured payload.

delete dissolves the group and keeps its workspaces by default. Pass --close-workspaces only to close every member workspace and terminate its processes. The response reports the operation and affected count. Pass set-color or set-icon an empty value to clear the group's color or icon.

### [#](https://cmux-docs-release.vercel.app/docs/workspace-groups#examples)Examples

Group two explicitly chosen workspaces under a name:

```
cmux workspace-group create --name manaflow --from workspace:1,workspace:2
```

Spin up a new workspace inside an existing group, for example wired to a worktree script:

```
cmux workspace-group new-workspace workspace_group:1
```

List groups in the focused window:

```
cmux workspace-group list
```

## [#](https://cmux-docs-release.vercel.app/docs/workspace-groups#configuration)Configuration

Per-group configuration lives under the workspaceGroups key in ~/.config/cmux/cmux.json, keyed by the anchor workspace's working directory. See the configuration reference for the supported keys, including the global new-workspace placement and per-directory color, icon, placement, and context-menu actions.

[workspaceGroups configuration reference](https://cmux-docs-release.vercel.app/docs/configuration#schema-workspaceGroups)

## [#](https://cmux-docs-release.vercel.app/docs/workspace-groups#persistence)Persistence

Group name, anchor, pin state, collapse state, color, and icon are saved alongside your workspaces and restored across launches. Group membership is stored on each workspace.

[Concepts](https://cmux-docs-release.vercel.app/docs/concepts) [Configuration](https://cmux-docs-release.vercel.app/docs/configuration)

Canonical: https://cmux-docs-release.vercel.app/docs/workspace-groups
