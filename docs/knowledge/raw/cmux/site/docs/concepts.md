# [#](#title)Concepts

# [#](https://cmux-docs-release.vercel.app/docs/concepts#title)Concepts

cmux organizes your terminals in a four-level hierarchy. Understanding these levels helps when using the socket API, CLI, and keyboard shortcuts.

## [#](https://cmux-docs-release.vercel.app/docs/concepts#hierarchy)Hierarchy

```
Window
  └── Workspace (sidebar entry)
        └── Pane (split region)
              └── Surface (tab within pane)
                    └── Panel (terminal or browser content)
```

### [#](https://cmux-docs-release.vercel.app/docs/concepts#window-title)Window

A macOS window. Open multiple windows with ⌘⇧N. Each window has its own sidebar with independent workspaces.

### [#](https://cmux-docs-release.vercel.app/docs/concepts#workspace-title)Workspace

A sidebar entry. Each workspace contains one or more split panes. Workspaces are what you see listed in the left sidebar.

In the UI and keyboard shortcuts, workspaces are often called "tabs" since they behave like tabs in the sidebar. The socket API and environment variables use the term "workspace".

| Context | Term used |
| --- | --- |
| Sidebar UI | Tab |
| Keyboard shortcuts | Workspace or tab |
| Socket API | `workspace` |
| Environment variable | `CMUX_WORKSPACE_ID` |

**Shortcuts: ⌘N (new), ⌘1–⌘9 (jump), ⌘⇧W (close), ⌃⌘\[ / ⌃⌘\] (prev/next)**

### [#](https://cmux-docs-release.vercel.app/docs/concepts#pane-title)Pane

A split region within a workspace. Created by splitting with ⌘D (right) or ⌘⇧D (down). Navigate between panes with ⌥⌘ + arrow keys.

Each pane can hold multiple surfaces (tabs within the pane).

### [#](https://cmux-docs-release.vercel.app/docs/concepts#surface-title)Surface

A tab within a pane. Each pane has its own tab bar and can hold multiple surfaces. Created with ⌘T, navigated with ⌘\[ / ⌘\] or ⌃1–⌃9.

Surfaces are the individual terminal or browser sessions you interact with. Each surface has its own CMUX\_SURFACE\_ID environment variable.

### [#](https://cmux-docs-release.vercel.app/docs/concepts#panel-title)Panel

The content inside a surface. Currently two types:

-   **Terminal: a Ghostty terminal session**
-   **Browser: an embedded web view**

Panel is mostly an internal concept. In the socket API and CLI, you interact with surfaces rather than panels directly.

## [#](https://cmux-docs-release.vercel.app/docs/concepts#workspace-groups)Workspace Groups

Group related workspaces into collapsible named sections in the sidebar. Each group is owned by an anchor workspace and can be pinned, renamed, and given its own icon and color. New workspaces can be spawned directly inside a group from its header.

[Read the full Workspace Groups guide](https://cmux-docs-release.vercel.app/docs/workspace-groups)

## [#](https://cmux-docs-release.vercel.app/docs/concepts#visual-example)Visual example

```
┌──────────────────────────────────────────────────────┐
│ ┌──────────┐ ┌─────────────────────────────────────┐ │
│ │ Sidebar  │ │ Workspace "dev"                     │ │
│ │          │ │                                     │ │
│ │          │ │ ┌───────────────┬─────────────────┐ │ │
│ │ > dev    │ │ │ Pane 1        │ Pane 2          │ │ │
│ │   server │ │ │ [S1] [S2]     │ [S1]            │ │ │
│ │   logs   │ │ │               │                 │ │ │
│ │          │ │ │  Terminal     │  Terminal       │ │ │
│ │          │ │ │               │                 │ │ │
│ │          │ │ └───────────────┴─────────────────┘ │ │
│ └──────────┘ └─────────────────────────────────────┘ │
└──────────────────────────────────────────────────────┘
```

In this example:

-   The window contains a sidebar with three workspaces (dev, server, logs)
-   Workspace "dev" is selected, showing two panes side by side
-   Pane 1 has two surfaces (\[S1\] and \[S2\] in the tab bar), with S1 active
-   Pane 2 has one surface
-   Each surface contains a panel (a terminal in this case)

## [#](https://cmux-docs-release.vercel.app/docs/concepts#summary)Summary

| Level | What it is | Created by | Identified by |
| --- | --- | --- | --- |
| Window | macOS window | `⌘⇧N` | — |
| Workspace | Sidebar entry | `⌘N` | `CMUX_WORKSPACE_ID` |
| Pane | Split region | `⌘D` / `⌘⇧D` | Pane ID (socket API) |
| Surface | Tab within pane | `⌘T` | `CMUX_SURFACE_ID` |
| Panel | Terminal or browser | Automatic | Panel ID (internal) |

[cmux TUI](https://cmux-docs-release.vercel.app/docs/tui) [Workspace Groups](https://cmux-docs-release.vercel.app/docs/workspace-groups)

Canonical: https://cmux-docs-release.vercel.app/docs/concepts
