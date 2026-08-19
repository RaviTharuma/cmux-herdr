# [#](#title)Configuration

# [#](https://cmux-docs-release.vercel.app/docs/configuration#title)Configuration

cmux reads terminal configuration from Ghostty config files. cmux-owned app settings also live in ~/.config/cmux/cmux.json, including shortcuts, automation, sidebar behavior, notifications, and browser preferences.

## [#](https://cmux-docs-release.vercel.app/docs/configuration#config-locations)Config file locations

cmux looks for configuration in these locations (in order):

1.  `~/.config/ghostty/config`
2.  `~/Library/Application Support/com.mitchellh.ghostty/config`

Create the config file if it doesn't exist:

```
mkdir -p ~/.config/ghostty
touch ~/.config/ghostty/config
```

## [#](https://cmux-docs-release.vercel.app/docs/configuration#example-config)Example config

~/.config/ghostty/config

```
font-family = SF Mono
font-size = 13
sidebar-font-size = 14
surface-tab-bar-font-size = 11
theme = One Dark
scrollback-limit = 50000000
split-divider-color = #3e4451
working-directory = ~/code
```

## [#](https://cmux-docs-release.vercel.app/docs/configuration#cmux-json)cmux.json

cmux keeps app-owned settings, shortcuts, actions, custom commands, and workspace layouts in `~/.config/cmux/cmux.json`. Terminal rendering still lives in Ghostty config. On launch, if the file is missing, cmux writes a commented template there.

Open cmux Settings, then use the `cmux.json` section to open the canonical file in your preferred text editor.

1.  `~/.config/cmux/cmux.json`
2.  `.cmux/cmux.json` in a project for project-scoped actions and workspace commands

**Precedence:** global `~/.config/cmux/cmux.json` settings override values saved in the Settings window. Legacy `~/.config/cmux/settings.json` and Application Support settings files are read only as fallback for missing settings keys. Project-local `.cmux/cmux.json` can override actions, commands, UI action wiring, and notification hooks, but not global app preferences.

**Reload:** edit the file, then use `Cmd+Shift+,` or `cmux reload-config` to re-read it without restarting the app.

**Migrations:** keep `schemaVersion` at `1` for now. Future cmux versions will use that field for upgrades. If cmux sees a newer schema version, it logs a warning and parses known keys only.

The file accepts JSON with comments and trailing commas. The canonical schema is published at [https://raw.githubusercontent.com/manaflow-ai/cmux/main/web/data/cmux.schema.json](https://raw.githubusercontent.com/manaflow-ai/cmux/main/web/data/cmux.schema.json) and the source lives at [https://github.com/manaflow-ai/cmux/blob/main/web/data/cmux.schema.json](https://github.com/manaflow-ai/cmux/blob/main/web/data/cmux.schema.json).

~/.config/cmux/cmux.json

```
{
  "$schema": "https://raw.githubusercontent.com/manaflow-ai/cmux/main/web/data/cmux.schema.json",
  "schemaVersion": 1,

  // "app": {
  //   "appearance": "dark",
  //   "menuBarOnly": false,
  //   "newWorkspacePlacement": "afterCurrent",
  //   "windowTitleTemplate": "[cmux:{windowToken}] {activeWorkspace}",
  //   "confirmQuit": "always",
  //   "openSupportedFilesInCmux": true,
  //   "workspaceInheritWorkingDirectory": true,
  //   "iMessageMode": true
  // },

  // "terminal": {
  //   "showScrollBar": false,
  //   "copyOnSelect": true,
  //   "autoResumeAgentSessions": true,
  //   "showTextBoxOnNewTerminals": false,
  //   "focusTextBoxOnNewTerminals": false,
  //   "agentHibernation": {
  //     "enabled": false,
  //     "idleSeconds": 5,
  //     "maxLiveTerminals": 12
  //   },
  //   "textBoxMaxLines": 10
  // },

  // "browser": {
  //   "defaultSearchEngine": "kagi",
  //   // For an unlisted provider, set "defaultSearchEngine": "custom" and fill these:
  //   "customSearchEngineName": "My Search",
  //   "customSearchEngineURLTemplate": "https://search.example.com/?q={query}",
  //   "openTerminalLinksInCmuxBrowser": true,
  //   "hostsToOpenInEmbeddedBrowser": ["localhost", "*.internal.example"]
  // },

  // "markdown": {
  //   // Default body font size (points) for newly opened markdown viewers.
  //   // Zoom a viewer live with Cmd-+ / Cmd-- / Cmd-0.
  //   "fontSize": 15,
  //   // Default body font family. Empty keeps the system markdown font stack.
  //   "fontFamily": "",
  //   // Default maximum reading column width, in CSS pixels.
  //   "maxWidth": 980
  // },

  // "fileEditor": {
  //   // Wrap long lines at the editor's right edge instead of scrolling horizontally.
  //   "wordWrap": false
  // },

  // "fileExplorer": {
  //   // Double-click a file: preview (default), defaultEditor (macOS default app), or preferredEditor (the app.preferredEditor command).
  //   "doubleClickAction": "preview"
  // },

  // "automation": {
  //   "suppressSubagentNotifications": true
  // },

  // "workspaceColors": {
  //   "colors": {
  //     "Red": "#C0392B",
  //     "Blue": "#1565C0",
  //     "Neon Mint": "#00F5D4"
  //   }
  // },

  // "workspaceGroups": {
  //   "newWorkspacePlacement": "afterCurrent"
  // },

  // "agentChat": {
  //   "url": "http://127.0.0.1:7739",
  //   "startCommand": "cmux-chat"
  // },

  // "shortcuts": {
  //   "bindings": {
  //     "toggleSidebar": "cmd+b",
  //     "toggleFileExplorer": "cmd+opt+b",
  //     "newTab": ["ctrl+b", "c"],
  //     "commandPalettePrevious": null
  //   }
  // },
}
```

## [#](https://cmux-docs-release.vercel.app/docs/configuration#schema-reference)Schema reference

This reference covers every supported global settings key in `cmux.json`. The embedded browser, terminal, sidebar, notifications, automation, and cmux-owned keyboard shortcuts all live here. Actions and workspace commands are documented on the [custom commands page](https://cmux-docs-release.vercel.app/docs/custom-commands).

### [#](https://cmux-docs-release.vercel.app/docs/configuration#metadata)Metadata

`$schema`

Optional schema URL for editor completion and validation.

Type

`string`

Default

`"https://raw.githubusercontent.com/manaflow-ai/cmux/main/web/data/cmux.schema.json"`

`schemaVersion`

Schema version for forward-compatible migrations. Newer versions are parsed on a best-effort basis.

Type

`integer`

Default

`1`

### [#](https://cmux-docs-release.vercel.app/docs/configuration#schema-app)`app`

General app preferences from Settings > App.

`app.language`

Preferred app language.

Type

`string`

Default

`"system"`

Allowed values

`system, en, ar, bs, zh-Hans, zh-Hant, da, de, es, fr, it, ja, ko, nb, pl, pt-BR, ru, th, tr`

`app.appearance`

App appearance mode.

Type

`string`

Default

`"system"`

Allowed values

`system, light, dark`

`app.appIcon`

Dock and app switcher icon style.

Type

`string`

Default

`"automatic"`

Allowed values

`automatic, light, dark`

`app.windowTitleTemplate`

Optional NSWindow title template. Blank preserves cmux's existing default title behavior, including the current-directory fallback. Supported placeholders: {windowId}, {windowToken}, {activeWorkspace}, {activeDirectory}, {defaultTitle}, {appName}.

Type

`string`

Default

`""`

`app.menuBarOnly`

Hide the Dock icon and app switcher entry while keeping cmux available from the menu bar.

Type

`boolean`

Default

`false`

`app.newWorkspacePlacement`

Where new workspaces are inserted in the sidebar.

Type

`string`

Default

`"afterCurrent"`

Allowed values

`top, afterCurrent, end`

`app.forkConversationDefaultDestination`

Default destination for the tab context menu's primary Fork Conversation action. The submenu still exposes every destination.

Type

`string`

Default

`"right"`

Allowed values

`right, left, top, bottom, newTab, newWorkspace`

`app.workspaceInheritWorkingDirectory`

When true, new workspaces inherit the current workspace working directory. When false, new workspaces use Ghostty's working-directory setting instead.

Type

`boolean`

Default

`true`

`app.minimalMode`

Hide the workspace title bar and move controls into the sidebar.

Type

`boolean`

Default

`false`

`app.keepWorkspaceOpenWhenClosingLastSurface`

When true, closing the last surface keeps the workspace open.

Type

`boolean`

Default

`false`

`app.focusPaneOnFirstClick`

When cmux is inactive, the first click can activate and focus the clicked pane.

Type

`boolean`

Default

`true`

`app.focusHistoryIncludesPanesAndTabs`

When true, Back and Forward include focus changes between panes and tabs. When false, they navigate between workspaces only.

Type

`boolean`

Default

`false`

`app.preferredEditor`

Custom editor command used when Cmd-click file previews are disabled or a file is unsupported. Leave empty to use the default.

Type

`string`

Default

`""`

`app.openSupportedFilesInCmux`

When enabled, Cmd-clicking readable local files opens supported previews in cmux, including text, code, PDFs, images, audio, video, and Quick Look files. Preview headers include an Open With menu based on the user's default and compatible macOS apps for that file.

Type

`boolean`

Default

`true`

`app.openMarkdownInCmuxViewer`

When enabled, Cmd-clicking .md/.markdown/.mkd/.mdx files opens the rendered cmux markdown viewer panel (with live reload) instead of the generic file preview.

Type

`boolean`

Default

`true`

`app.globalFontMagnification`

Scales cmux-owned terminals, tab titles, sidebars, settings, overlays, and app chrome by this percentage. Rendered browser page content is excluded.

Type

`integer`

Default

`100`

`app.reorderOnNotification`

Move workspaces with new notifications toward the top.

Type

`boolean`

Default

`true`

`app.iMessageMode`

Move a workspace to the top and show the submitted message when sending an agent prompt.

Type

`boolean`

Default

`false`

`app.sendAnonymousTelemetry`

Allow anonymous telemetry.

Type

`boolean`

Default

`true`

`app.confirmQuit`

Control when cmux asks for confirmation before quitting. DEV builds always quit immediately regardless of this setting. Legacy app.warnBeforeQuit is still accepted as a boolean fallback.

Type

`string`

Default

`"always"`

Allowed values

`always, dirty-only, never`

`app.warnBeforeQuit`

Legacy boolean fallback for quit confirmation. Use app.confirmQuit for new configs.

Type

`boolean`

Default

`true`

`app.warnBeforeClosingTab`

Show a confirmation before closing a tab.

Type

`boolean`

Default

`true`

`app.warnBeforeClosingTabXButton`

Show a confirmation before closing a tab with the tab close button.

Type

`boolean`

Default

`false`

`app.hideTabCloseButton`

Hide tab close buttons in the pane tab bar.

Type

`boolean`

Default

`false`

`app.renameSelectsExistingName`

Select the current name when opening rename flows.

Type

`boolean`

Default

`true`

`app.commandPaletteSearchesAllSurfaces`

Search every surface in the command palette switcher instead of only the active workspace.

Type

`boolean`

Default

`false`

### [#](https://cmux-docs-release.vercel.app/docs/configuration#schema-terminal)`terminal`

Terminal presentation settings from Settings > Terminal.

`terminal.showScrollBar`

Show the right-edge terminal scroll bar when scrollback is available. cmux automatically suppresses it for alternate-screen style TUI surfaces.

Type

`boolean`

Default

`true`

`terminal.scrollSpeed`

Multiplier applied to terminal scroll wheel and trackpad deltas. Higher values scroll faster; lower values scroll slower.

Type

`number`

Default

`1`

`terminal.sessionContentMaxWidth`

Optional maximum width, in points, for terminal and built-in agent chat content. Set false to use the full pane width.

Type

`boolean | number`

Default

`false`

`terminal.sessionContentAlignment`

Horizontal placement for terminal and built-in agent chat content when sessionContentMaxWidth is enabled.

Type

`string`

Default

`"center"`

Allowed values

`left, center, right`

`terminal.copyOnSelect`

When true, copy selected terminal text to the system clipboard when the selection is committed. When false, cmux does not emit a Ghostty copy-on-select override; Ghostty config and defaults control selection-clipboard behavior.

Type

`boolean`

Default

`false`

`terminal.autoResumeAgentSessions`

Automatically run agent resume commands for restored terminal sessions when cmux reopens after quit. Set false to restore panes while keeping Claude Code, Codex, OpenCode, and other saved agent sessions idle until you resume them manually.

Type

`boolean`

Default

`true`

`terminal.showTextBoxOnNewTerminals`

Show the beta TextBox input by default for newly created workspaces, terminal tabs, and terminal splits.

Type

`boolean`

Default

`false`

`terminal.focusTextBoxOnNewTerminals`

Focus the beta TextBox input by default for newly created workspaces, terminal tabs, and terminal splits. Focusing also shows the TextBox.

Type

`boolean`

Default

`false`

`terminal.agentHibernation`

Routine Agent Hibernation settings. cmux kills idle background agent processes to free RAM and CPU, then resumes them with their saved session when their tab is visited. Routine hibernation requires a restorable coding agent whose lifecycle reports idle, an off-screen terminal, a live-terminal count above the configured limit, and unchanged output through the idle and confirmation windows. Independently, during critical memory pressure cmux may hibernate a bounded batch of safe idle background agents even when enabled is false; visible, running, needs-input, recently changed, and unprotectable agents remain excluded. The placeholder Resume button is a manual fallback.

Type

`object`

Default

`none`

`terminal.rendererRealization`

Reclaim off-screen terminal GPU renderer memory. cmux releases the Metal renderer (IOSurface) of a terminal that has stayed off-screen and idle while keeping its process and terminal state alive, then rebuilds the renderer instantly when the tab is visited again. Non-destructive and on by default.

Type

`object`

Default

`none`

`terminal.textBoxMaxLines`

Maximum number of lines the rich terminal TextBox input can grow to before it scrolls.

Type

`integer`

Default

`10`

`terminal.textBoxDefaultSubmitAction`

Default TextBox submit action ID for new terminal sessions. Use text-entry for plain input or one of the configured action IDs.

Type

`string`

Default

`"text-entry"`

`terminal.textBoxSubmitActions`

Configurable TextBox submit actions shown on the submit button, Shift-Tab cycle, and right-click menu.

Type

`array<object>`

Default

```
[]
```

`terminal.resumeCommands`

Signed command-prefix approvals for restoring non-agent terminal surfaces. cmux writes this list when you approve a surface resume command.

Type

`array<object>`

Default

```
[]
```

`terminal.uploadCommands`

Host-scoped rules that replace the built-in scp for terminal file drops and pastes over SSH. When the ssh destination matches a rule, cmux runs that rule's command once per file instead of scp, and inserts the command's stdout at the cursor (verbatim, with control characters stripped); if the command prints nothing, cmux inserts the shell-escaped remote path it chose instead. Per-file outputs are space-joined. First matching enabled rule wins; no match runs the built-in scp unchanged. A non-zero exit, timeout, or cancel inserts nothing.

Type

`array<object>`

Default

```
[]
```

### [#](https://cmux-docs-release.vercel.app/docs/configuration#schema-notifications)`notifications`

Notification behavior from Settings > Notifications.

`notifications.dockBadge`

Show the unread count in the Dock tile.

Type

`boolean`

Default

`true`

`notifications.showInMenuBar`

Show the menu bar extra.

Type

`boolean`

Default

`true`

`notifications.unreadPaneRing`

Highlight panes with unread notifications.

Type

`boolean`

Default

`true`

`notifications.paneFlash`

Flash the focused pane when requested.

Type

`boolean`

Default

`true`

`notifications.suppressOnlyFocusedSurface`

When enabled, a notification banner is auto-withdrawn only when its surface is the exact focused surface. A banner delivered for a non-focused surface in the currently visible workspace stays up until you focus that surface (or click/dismiss it), instead of being retracted when the workspace becomes visible. Off preserves the legacy workspace-visibility withdraw.

Type

`boolean`

Default

`false`

`notifications.agentPermissionPrompt`

Notify when an agent (e.g. Claude Code) is blocked waiting for your permission to run a tool. On by default, since this is the alert you must act on to unblock the agent.

Type

`boolean`

Default

`true`

`notifications.agentTurnComplete`

When to notify that an agent finished a turn. whenIdle (default) suppresses the notification while the agent still has a running background task or a pending scheduled wakeup, so you are pinged once work truly drains. always notifies on every turn end; never disables it.

Type

`string`

Default

`"whenIdle"`

Allowed values

`whenIdle, always, never`

`notifications.agentIdleReminder`

Notify when an agent has been idle waiting for your input (about 60s after a turn ends). Suppressed while background work from the last turn is still pending, so a running build or watcher does not trigger a false waiting alert.

Type

`boolean`

Default

`true`

`notifications.sound`

Notification sound preset.

Type

`string`

Default

`"default"`

Allowed values

`default, Basso, Blow, Bottle, Frog, Funk, Glass, Hero, Morse, Ping, Pop, Purr, Sosumi, Submarine, Tink, custom_file, none`

`notifications.customSoundFilePath`

Local path to the custom notification sound file.

Type

`string`

Default

`""`

`notifications.command`

Optional shell command to run alongside notification delivery.

Type

`string`

Default

`""`

`notifications.hooksMode`

Controls whether project-local notification hooks append to inherited hooks or replace them.

Type

`string`

Default

`"append"`

Allowed values

`append, replace`

`notifications.hooks`

Composable shell hooks that receive notification policy JSON on stdin and return updated policy JSON on stdout.

Type

`array<object>`

Default

```
[]
```

### [#](https://cmux-docs-release.vercel.app/docs/configuration#schema-sidebar)`sidebar`

Sidebar content and metadata visibility from Settings > Sidebar.

`sidebar.hideAllDetails`

Hide all per-workspace detail rows.

Type

`boolean`

Default

`false`

`sidebar.wrapWorkspaceTitles`

Allow workspace titles in the sidebar to wrap to multiple lines instead of truncating after one line.

Type

`boolean`

Default

`false`

`sidebar.showWorkspaceDescription`

Show custom workspace descriptions in the sidebar.

Type

`boolean`

Default

`true`

`sidebar.beta`

Experimental sidebar features.

Type

`object`

Default

`none`

`sidebar.branchLayout`

Show git branch details stacked vertically or inline.

Type

`string`

Default

`"vertical"`

Allowed values

`vertical, inline`

`sidebar.showNotificationMessage`

Show the latest notification text in the sidebar.

Type

`boolean`

Default

`true`

`sidebar.notificationMessageLineLimit`

Maximum lines shown for the latest notification below each workspace title.

Type

`integer`

Default

`12`

`sidebar.showBranchDirectory`

Show the workspace working directory.

Type

`boolean`

Default

`true`

`sidebar.showPullRequests`

Show pull request metadata in the sidebar.

Type

`boolean`

Default

`true`

`sidebar.watchGitStatus`

Watch repository files for sidebar branch and pull request metadata without polling git.

Type

`boolean`

Default

`true`

`sidebar.makePullRequestsClickable`

Allow sidebar pull request metadata to open links when clicked.

Type

`boolean`

Default

`true`

`sidebar.openPullRequestLinksInCmuxBrowser`

Open sidebar pull request links in the embedded cmux browser.

Type

`boolean`

Default

`true`

`sidebar.openPortLinksInCmuxBrowser`

Open sidebar port links in the embedded cmux browser.

Type

`boolean`

Default

`true`

`sidebar.showSSH`

Show SSH connection details.

Type

`boolean`

Default

`true`

`sidebar.showPorts`

Show listening ports.

Type

`boolean`

Default

`true`

`sidebar.showLog`

Show recent log snippets.

Type

`boolean`

Default

`true`

`sidebar.showProgress`

Show progress indicators.

Type

`boolean`

Default

`true`

`sidebar.showAgentActivity`

Show the loading spinner on workspaces with running coding agents or active loaders.

Type

`boolean`

Default

`true`

`sidebar.loadingSpinnerPosition`

Which side of the workspace row the loading spinner appears on: leading (left, sharing the unread-badge slot) or trailing (right).

Type

`string`

Default

`"leading"`

Allowed values

`leading, trailing`

`sidebar.notificationBadgePosition`

Which side of the workspace row the unread notification badge appears on: leading (left) or trailing (right).

Type

`string`

Default

`"leading"`

Allowed values

`leading, trailing`

`sidebar.showCustomMetadata`

Show custom metadata pills.

Type

`boolean`

Default

`true`

`sidebar.rightMaxWidth`

Maximum width in points for the right sidebar. When omitted, the built-in dynamic cap applies.

Type

`number`

Default

`none`

### [#](https://cmux-docs-release.vercel.app/docs/configuration#schema-workspaceGroups)`workspaceGroups`

Per-cwd customization for sidebar workspace groups. The anchor workspace's cwd is matched against the keys in \`byCwd\`; longest-match wins. Keys containing \`\*\` or \`?\` are matched as fnmatch globs (with \`~\` expanded); other keys are path prefixes.

`workspaceGroups.newWorkspacePlacement`

Global default for where Cmd-N inside a group, the group header + button, and configured group actions place the new workspace: \`afterCurrent\` (after the active in-group workspace, falling back to top), \`top\` (second slot, just after the anchor), or \`end\` (after the last member).

Type

`string`

Default

`"afterCurrent"`

Allowed values

`afterCurrent, top, end`

`workspaceGroups.byCwd`

Map of cwd patterns to group customization. Empty when omitted.

Type

`object`

Default

`none`

### [#](https://cmux-docs-release.vercel.app/docs/configuration#schema-workspaceColors)`workspaceColors`

Workspace tab and badge colors from Settings > Workspace Colors.

`workspaceColors.indicatorStyle`

Active workspace indicator style. Legacy aliases are accepted and normalized.

Type

`string`

Default

`"leftRail"`

Allowed values

`leftRail, solidFill, rail, border, wash, lift, typography, washRail, blueWashColorRail`

`workspaceColors.selectionColor`

Override the selected workspace background color.

Type

`unknown`

Default

`null`

`workspaceColors.notificationBadgeColor`

Override the unread notification badge color.

Type

`unknown`

Default

`null`

`workspaceColors.colors`

Full named workspace color palette. Include built-in entries you want to keep, remove keys to remove colors, and add more named entries to extend the picker.

Type

`object`

Default

```
{
  "Red": "#C0392B",
  "Crimson": "#922B21",
  "Orange": "#A04000",
  "Amber": "#7D6608",
  "Olive": "#4A5C18",
  "Green": "#196F3D",
  "Teal": "#006B6B",
  "Aqua": "#0E6B8C",
  "Blue": "#1565C0",
  "Navy": "#1A5276",
  "Indigo": "#283593",
  "Purple": "#6A1B9A",
  "Magenta": "#AD1457",
  "Rose": "#880E4F",
  "Brown": "#7B3F00",
  "Charcoal": "#3E4B5E"
}
```

`workspaceColors.paletteOverrides`

Legacy workspace color overrides for built-in palette names. Prefer workspaceColors.colors for new configs.

Type

`object`

Default

```
{}
```

`workspaceColors.customColors`

Legacy list of custom workspace colors. Prefer workspaceColors.colors for new configs.

Type

`array<unknown>`

Default

```
[]
```

`workspaceColors.colors` is the full palette. Keep the built-in keys you want, delete keys to remove colors from the picker, and add more named color entries to extend it. Older `paletteOverrides` and `customColors` files still parse during upgrades, but new files should use `colors`.

```
{
  "workspaceColors": {
    "colors": {
      "Red": "#C0392B",
      "Blue": "#1565C0",
      "Neon Mint": "#00F5D4"
    }
  }
}
```

### [#](https://cmux-docs-release.vercel.app/docs/configuration#schema-sidebarAppearance)`sidebarAppearance`

Sidebar tint settings from Settings > Sidebar Appearance.

`sidebarAppearance.matchTerminalBackground`

Use the terminal background instead of the sidebar tint.

Type

`boolean`

Default

`false`

`sidebarAppearance.tintColor`

Base sidebar tint color used when light/dark overrides are not set.

Type

`unknown`

Default

`"#000000"`

`sidebarAppearance.lightModeTintColor`

Sidebar tint override for light appearance.

Type

`unknown`

Default

`null`

`sidebarAppearance.darkModeTintColor`

Sidebar tint override for dark appearance.

Type

`unknown`

Default

`null`

`sidebarAppearance.tintOpacity`

Sidebar tint opacity from 0 to 1. Note: this only controls the sidebar tint, not terminal/window transparency. For terminal background transparency or blur, set \`background-opacity\` and \`background-blur\` in \`~/.config/ghostty/config\` and run \`cmux reload-config\`.

Type

`number`

Default

`0.03`

### [#](https://cmux-docs-release.vercel.app/docs/configuration#schema-automation)`automation`

Socket control and automation settings from Settings > Automation.

`automation.socketControlMode`

Socket control mode. Legacy aliases are accepted and normalized.

Type

`string`

Default

`"cmuxOnly"`

Allowed values

`off, cmuxOnly, automation, password, allowAll, openAccess, fullOpenAccess, notifications, full`

`automation.socketPassword`

Password for password-mode socket access. Use null or an empty string to clear it.

Type

`string | null`

Default

`""`

`automation.claudeCodeIntegration`

Enable cmux integration hooks for Claude Code.

Type

`boolean`

Default

`true`

`automation.claudeBinaryPath`

Custom path to the claude binary.

Type

`string`

Default

`""`

`automation.workspaceAutoNaming`

Opt-in AI auto-naming of workspaces and tabs from agent conversation content. When enabled, cmux summarizes supported agent sessions into short titles using each agent's own binary; manual renames always win.

Type

`boolean`

Default

`false`

`automation.autoNamingAgent`

Which agent generates auto-names for every session. "auto" (default) names each session with its own agent; any agent slug (claude, codex, grok, opencode, pi, omp, …) overrides naming for all sessions, even other agents' sessions. Undriveable or uninstalled agents fall back to the session's own agent, so naming never breaks.

Type

`string`

Default

`"auto"`

`automation.ripgrepBinaryPath`

Custom path to the ripgrep (rg) binary used by project search.

Type

`string`

Default

`""`

`automation.suppressSubagentNotifications`

Suppress visible completion notifications and status mutations from nested Codex or Claude child agents while keeping their events in Feed telemetry.

Type

`boolean`

Default

`true`

`automation.ampIntegration`

Enable cmux integration hooks for Amp. When disabled, the bundled plugin stays inactive without needing to be removed.

Type

`boolean`

Default

`true`

`automation.cursorIntegration`

Enable cmux integration hooks for Cursor.

Type

`boolean`

Default

`true`

`automation.geminiIntegration`

Enable cmux integration hooks for Gemini.

Type

`boolean`

Default

`true`

`automation.kiroIntegration`

Enable cmux integration hooks for Kiro CLI.

Type

`boolean`

Default

`true`

`automation.kiroNotificationLevel`

Controls how many Kiro tool events appear in Feed.

Type

`string`

Default

`"standard"`

Allowed values

`minimal, standard, verbose`

`automation.portBase`

Starting value for workspace CMUX\_PORT assignments.

Type

`integer`

Default

`9100`

`automation.portRange`

Number of ports reserved per workspace.

Type

`integer`

Default

`10`

### [#](https://cmux-docs-release.vercel.app/docs/configuration#schema-ui)`ui`

UI action wiring, including surface tab bar buttons and plus-button behavior. The plus-button context menu shows ui.newWorkspace.contextMenu (or the default items) followed by actions that opt in via newWorkspaceMenu.

`ui.newWorkspace`

New Workspace button behavior and context-menu wiring.

Type

`object`

Default

`none`

### [#](https://cmux-docs-release.vercel.app/docs/configuration#schema-agentChat)`agentChat`

Agent Chat GUI server settings. The built-in New agent chat action opens this URL in a browser workspace, probes /healthz first, and can run a configured start command when the server is down.

`agentChat.url`

Base URL for the machine-local Agent Chat GUI server. cmux probes <url>/healthz before opening the workspace.

Type

`string`

Default

`"http://127.0.0.1:7739"`

`agentChat.startCommand`

Optional shell command to start the Agent Chat server when <url>/healthz is unreachable. cmux runs it detached with the user's SHELL, polls healthz for a short bounded window, then opens the workspace either way.

Type

`string`

Default

`none`

`agentChat.keys`

Keyboard behavior overrides for the Agent Chat web UI.

Type

`object`

Default

`none`

`agentChat.fonts`

Font overrides for the Agent Chat web UI. Omitted values fall back to Ghostty and built-in defaults.

Type

`object`

Default

`none`

### [#](https://cmux-docs-release.vercel.app/docs/configuration#schema-browser)`browser`

Embedded browser settings from Settings > Browser.

`browser.defaultSearchEngine`

Default search engine for non-URL browser address bar queries. Use custom with customSearchEngineURLTemplate for arbitrary providers.

Type

`string`

Default

`"google"`

Allowed values

`google, duckduckgo, bing, kagi, startpage, brave, perplexity, exa, yahoo, ecosia, qwant, mojeek, wikipedia, github, baidu, yandex, custom`

`browser.customSearchEngineName`

Display name used when defaultSearchEngine is custom.

Type

`string`

Default

`""`

`browser.customSearchEngineURLTemplate`

Search URL used when defaultSearchEngine is custom. Include the query placeholder or %s for the encoded query. If omitted, cmux appends q= to the URL.

Type

`string`

Default

`"https://www.google.com/search?q={query}"`

`browser.showSearchSuggestions`

Show omnibar search suggestions.

Type

`boolean`

Default

`true`

`browser.theme`

Embedded browser theme.

Type

`string`

Default

`"system"`

Allowed values

`system, light, dark`

`browser.discardHiddenWebViews`

Allow hidden browser tabs to release page memory and restore when shown again.

Type

`boolean`

Default

`true`

`browser.hiddenWebViewDiscardDelaySeconds`

Seconds a browser tab must stay hidden before cmux frees its page memory.

Type

`number`

Default

`300`

`browser.askWhereToSaveDownloads`

Show a save panel for browser downloads instead of saving directly to Downloads.

Type

`boolean`

Default

`false`

`browser.openTerminalLinksInCmuxBrowser`

Open clicked terminal links in the embedded browser.

Type

`boolean`

Default

`true`

`browser.interceptTerminalOpenCommandInCmuxBrowser`

Intercept terminal open http(s) commands and route them through the embedded browser.

Type

`boolean`

Default

`true`

`browser.hostsToOpenInEmbeddedBrowser`

Allowlist of hosts that should stay inside the embedded browser.

Type

`array<string>`

Default

```
[]
```

`browser.urlsToAlwaysOpenExternally`

Rules that always open matching URLs in the system browser.

Type

`array<string>`

Default

```
[]
```

`browser.insecureHttpHostsAllowedInEmbeddedBrowser`

HTTP hosts allowed in the embedded browser without a warning prompt.

Type

`array<string>`

Default

```
[
  "localhost",
  "*.localhost",
  "127.0.0.1",
  "::1",
  "0.0.0.0",
  "*.localtest.me"
]
```

`browser.showImportHintOnBlankTabs`

Show the browser import hint on blank tabs.

Type

`boolean`

Default

`true`

`browser.reactGrabVersion`

Pinned react-grab version for the browser toolbar helper.

Type

`string`

Default

`"0.1.29"`

### [#](https://cmux-docs-release.vercel.app/docs/configuration#schema-markdown)`markdown`

Built-in markdown viewer settings.

`markdown.fontSize`

Default body font size, in points, for newly opened markdown viewers. Zoom a viewer live with Cmd-+ / Cmd-- / Cmd-0.

Type

`integer`

Default

`15`

`markdown.fontFamily`

Default body font family for newly opened markdown viewers. Leave empty for the system markdown font stack.

Type

`string`

Default

`""`

`markdown.maxWidth`

Default maximum reading column width, in CSS pixels, for newly opened markdown viewers.

Type

`integer`

Default

`980`

### [#](https://cmux-docs-release.vercel.app/docs/configuration#schema-fileEditor)`fileEditor`

Built-in plain-text file editor settings.

`fileEditor.wordWrap`

Wrap long lines at the editor's right edge instead of scrolling horizontally.

Type

`boolean`

Default

`false`

### [#](https://cmux-docs-release.vercel.app/docs/configuration#schema-fileExplorer)`fileExplorer`

Right-sidebar file explorer (file tree) settings.

`fileExplorer.doubleClickAction`

What double-clicking (or pressing Return on a search result for) a file in the file explorer does. \`preview\` opens the built-in cmux file preview (the default and historical behavior). \`defaultEditor\` opens with the macOS default app for the file type. \`preferredEditor\` opens with the \`app.preferredEditor\` command, falling back to the default app when none is set. Only applies to files; directories always expand/collapse, and non-local (remote) file explorers always open the cmux preview.

Type

`string`

Default

`"preview"`

Allowed values

`preview, defaultEditor, preferredEditor`

### [#](https://cmux-docs-release.vercel.app/docs/configuration#schema-shortcuts)`shortcuts`

Keyboard shortcut settings from Settings > Keyboard Shortcuts.

`shortcuts.showModifierHoldHints`

Show shortcut-hint chips while holding Cmd or Control.

Type

`boolean`

Default

`true`

`shortcuts.when`

Optional per-action context predicates (VS Code-style \`when\` clauses), keyed by cmux action id. Each value is a boolean expression over context keys combined with !, &&, ||, and parentheses. Boolean keys: sidebarFocus, browserFocus, markdownFocus, filePreviewTextEditorFocus, simulatorFocus, terminalFocus, commandPaletteVisible, terminalFindVisible, workspaceCanvasLayout. Typed keys support comparisons: the string sidebarMode (files, find, sessions, feed, or dock) and the integers paneCount and workspaceCount. Comparison operators are ==, !=, =~ (regex), <, <=, >, >=, and \`in \[a, b\]\`; an unknown or absent key reads as false. The boolean literals true and false are also accepted; \`key == false\` is the same as \`!key\`. The action's shortcut only fires (and only conflicts with other shortcuts) when the clause holds. Examples: { "selectWorkspaceByNumber": "!sidebarFocus" } selects workspaces with Ctrl+1–9 everywhere except when the right sidebar is focused; { "selectSurfaceByNumber": "sidebarMode == 'find' && paneCount > 1" } scopes a binding to the Find sidebar when the workspace has multiple panes.

Type

`object`

Default

```
{}
```

### [#](https://cmux-docs-release.vercel.app/docs/configuration#shortcuts-bindings)`shortcuts.bindings`

Use a string for a single shortcut, a two-item array for a chord, or `null` to unbind a shortcut in `shortcuts.bindings`. Unbind aliases also include empty string (`""`), `none`, `clear`, `unbound`, and `disabled`. Example chord: `["ctrl+b", "c"]`. Numbered actions use `1` as the stored default and still match digits `1` through `9`.

The defaults below are the same cmux-owned actions listed on the [keyboard shortcuts page](https://cmux-docs-release.vercel.app/docs/keyboard-shortcuts).

#### App

`openSettings`

Settings

Default file value

`cmd+,`

`reloadConfiguration`

Reload configuration

Default file value

`cmd+shift+,`

`showHideAllWindows`

Show/hide all cmux windowssystem-wide hotkey

Default file value

`ctrl+opt+cmd+.`

`globalSearch`

Global searchwhen cmux is active

Default file value

`opt+cmd+f`

`commandPalette`

Command palette

Default file value

`cmd+shift+p`

`commandPaletteNext`

Command palette next resultwhen the command palette is open

Default file value

`ctrl+n`

`commandPalettePrevious`

Command palette previous resultwhen the command palette is open

Default file value

`ctrl+p`

`newWindow`

New window

Default file value

`cmd+shift+n`

`closeWindow`

Close window

Default file value

`ctrl+cmd+w`

`toggleFullScreen`

Toggle full screen

Default file value

`ctrl+cmd+f`

`sendFeedback`

Send feedbackunbound by default

Default file value

`reopenPreviousSession`

Reopen previous session

Default file value

`cmd+shift+o`

`quit`

Quit cmux

Default file value

`cmd+q`

#### Workspaces

`toggleSidebar`

Toggle left sidebar

Default file value

`cmd+b`

`toggleFileExplorer`

Toggle right sidebar

Default file value

`cmd+opt+b`

`newTab`

New workspace

Default file value

`cmd+n`

`newBrowserWorkspace`

New browser workspacelike New Workspace, but the first surface is a browser pane with the address bar focused

Default file value

`opt+cmd+n`

`saveLayoutTemplate`

Save current workspace layout as a template

Default file value

`ctrl+cmd+s`

`openFolder`

Open folder

Default file value

`cmd+o`

`goToWorkspace`

Go to workspaceworkspace switcher

Default file value

`cmd+p`

`nextSidebarTab`

Next workspace

Default file value

`ctrl+cmd+]`

`prevSidebarTab`

Previous workspace

Default file value

`ctrl+cmd+[`

`moveWorkspaceUp`

Move workspace up

Default file value

`ctrl+opt+cmd+[`

`moveWorkspaceDown`

Move workspace down

Default file value

`ctrl+opt+cmd+]`

`focusHistoryBack`

Focus backFocus Back/Forward use Cmd+\[ and Cmd+\] outside browser panes; browser Back/Forward use the same defaults inside browser panes. Unbind Focus Back/Forward to let terminal shortcuts handle those keys.

Default file value

`cmd+[`

`focusHistoryForward`

Focus forwardFocus Back/Forward use Cmd+\[ and Cmd+\] outside browser panes; browser Back/Forward use the same defaults inside browser panes. Unbind Focus Back/Forward to let terminal shortcuts handle those keys.

Default file value

`cmd+]`

`selectWorkspaceByNumber`

Select workspace 1…9

Default file value

`cmd+1`

`renameWorkspace`

Rename workspace

Default file value

`cmd+shift+r`

`editWorkspaceDescription`

Edit workspace description

Default file value

`opt+cmd+e`

`markWorkspaceDone`

Mark workspace as done

Default file value

`cmd+;`

`cycleWorkspaceStatus`

Cycle workspace status one lane forward

Default file value

`cmd+shift+;`

`toggleChecklistItemComplete`

Toggle the highlighted checklist itemApplies in the focused todo pane or checklist popover.

Default file value

`cmd+enter`

`newWorkspaceGroup`

New empty workspace group

Default file value

`ctrl+cmd+g`

`groupSelectedWorkspaces`

Group selected workspaces

Default file value

`cmd+shift+g`

`toggleFocusedWorkspaceGroupCollapsed`

Collapse or expand focused workspace group

Default file value

`ctrl+cmd+.`

`focusRightSidebar`

Toggle right-sidebar focus

Default file value

`cmd+shift+e`

`navigateRightSidebarRows`

Navigate focused sidebar rowsIn Files, H/L collapse and expand folders. Search starts with /.

Default file value

`j / k`

`fileExplorerOpenSelection`

Open selected file or toggle folderfocused file explorer

Default file value

`enter`

`fileExplorerOpenSelectionFinderAlias`

Open selected file or toggle folderFinder-style alias for the focused file explorer

Default file value

`cmd+down`

`closeWorkspace`

Close workspace

Default file value

`cmd+shift+w`

`reopenClosedWorkspace`

Reopen closed workspaceunbound by default

Default file value

#### Surfaces

`newSurface`

New surface

Default file value

`cmd+t`

`nextSurface`

Next surface

Default file value

`cmd+shift+]`

`prevSurface`

Previous surface

Default file value

`cmd+shift+[`

`moveSurfaceLeft`

Reorder surface left

Default file value

`opt+cmd+shift+[`

`moveSurfaceRight`

Reorder surface right

Default file value

`opt+cmd+shift+]`

`moveSurfaceToPreviousPane`

Move surface to previous pane

Default file value

`ctrl+cmd+shift+[`

`moveSurfaceToNextPane`

Move surface to next pane

Default file value

`ctrl+cmd+shift+]`

`moveSurfaceToPaneLeft`

Move surface to pane on left

Default file value

`opt+cmd+shift+left`

`moveSurfaceToPaneRight`

Move surface to pane on right

Default file value

`opt+cmd+shift+right`

`moveSurfaceToPaneUp`

Move surface to pane above

Default file value

`opt+cmd+shift+up`

`moveSurfaceToPaneDown`

Move surface to pane below

Default file value

`opt+cmd+shift+down`

`selectSurfaceByNumber`

Select surface 1…9

Default file value

`ctrl+1`

`renameTab`

Rename tab

Default file value

`cmd+r`

`closeTab`

Close tab

Default file value

`cmd+w`

`closeOtherTabsInPane`

Close other tabs in pane

Default file value

`opt+cmd+t`

`reopenClosedBrowserPanel`

Reopen last closed

Default file value

`cmd+shift+t`

`toggleTerminalCopyMode`

Toggle terminal copy mode

Default file value

`cmd+shift+m`

`clearScreenKeepScrollback`

Clear screen (keep scrollback)

Default file value

`cmd+shift+k`

`simulatorHome`

Simulator: Homefocused Simulator

Default file value

`cmd+shift+h`

`simulatorRotateLeft`

Simulator: rotate leftfocused Simulator

Default file value

`cmd+left`

`simulatorRotateRight`

Simulator: rotate rightfocused Simulator

Default file value

`cmd+right`

`simulatorToggleAppearance`

Simulator: toggle appearancefocused Simulator

Default file value

`cmd+shift+a`

`simulatorToggleSoftwareKeyboard`

Simulator: toggle software keyboardfocused Simulator

Default file value

`cmd+k`

`focusTextBoxInput`

Switch focus between terminal and TextBox input

Default file value

`cmd+shift+a`

`cycleTextBoxSubmitAction`

Cycle TextBox submit action

Default file value

`shift+tab`

`attachTextBoxFile`

Attach file to TextBox input

Default file value

`opt+cmd+shift+a`

`sendCtrlFToTerminal`

Send Ctrl-F to terminalunbound by default; forwards Ctrl-F to the focused terminal (Claude Code: invoke twice to force-stop hung background agents)

Default file value

`saveFilePreview`

Save file previewfocused text preview

Default file value

`cmd+s`

#### Split Panes

`focusLeft`

Focus pane left

Default file value

`opt+cmd+left`

`focusRight`

Focus pane right

Default file value

`opt+cmd+right`

`focusUp`

Focus pane up

Default file value

`opt+cmd+up`

`focusDown`

Focus pane down

Default file value

`opt+cmd+down`

`focusPreviousPane`

Focus previous pane (cycle)unbound by default; a Ghostty goto\_split:previous keybind also cycles panes while Focus Back does not claim the same keys

Default file value

`focusNextPane`

Focus next pane (cycle)unbound by default; a Ghostty goto\_split:next keybind also cycles panes while Focus Forward does not claim the same keys

Default file value

`splitRight`

Split right

Default file value

`cmd+d`

`splitDown`

Split down

Default file value

`cmd+shift+d`

`splitBrowserRight`

Split browser right

Default file value

`opt+cmd+d`

`splitBrowserDown`

Split browser down

Default file value

`opt+cmd+shift+d`

`toggleSplitZoom`

Toggle pane zoom

Default file value

`cmd+shift+enter`

`increaseWorkspaceTerminalFontSize`

Increase font size for every terminal in the selected workspace

Default file value

`ctrl+cmd+=`

`decreaseWorkspaceTerminalFontSize`

Decrease font size for every terminal in the selected workspace

Default file value

`ctrl+cmd+-`

`resetWorkspaceTerminalFontSize`

Reset font size for every terminal in the selected workspace

Default file value

`ctrl+cmd+0`

`equalizeSplits`

Equalize split sizes

Default file value

`ctrl+cmd+shift+=`

#### Canvas

`toggleCanvasLayout`

Toggle canvas layout

Default file value

`ctrl+cmd+c`

`canvasRevealFocusedPane`

Reveal focused pane

Default file value

`ctrl+cmd+r`

`canvasOverview`

Toggle overview zoom

Default file value

`ctrl+cmd+o`

`canvasZoomIn`

Zoom in

Default file value

`opt+cmd+=`

`canvasZoomOut`

Zoom out

Default file value

`opt+cmd+-`

`canvasZoomReset`

Actual size

Default file value

`cmd+0`

`canvasTidy`

Tidy panes into a grid

Default file value

`ctrl+cmd+t`

#### Browser

`openBrowser`

Open browser

Default file value

`cmd+shift+l`

`focusBrowserAddressBar`

Focus address bar

Default file value

`cmd+l`

`browserBack`

Back

Default file value

`cmd+[`

`browserForward`

Forward

Default file value

`cmd+]`

`browserReload`

Reload pagefocused browser

Default file value

`cmd+r`

`browserHardReload`

Hard refresh pagefocused browser

Default file value

`cmd+shift+r`

`browserZoomIn`

Zoom in

Default file value

`cmd+=`

`browserZoomOut`

Zoom out

Default file value

`cmd+-`

`browserZoomReset`

Actual size

Default file value

`cmd+0`

`markdownZoomIn`

Markdown viewer: zoom infocused markdown viewer

Default file value

`cmd+=`

`markdownZoomOut`

Markdown viewer: zoom outfocused markdown viewer

Default file value

`cmd+-`

`markdownZoomReset`

Markdown viewer: actual sizefocused markdown viewer

Default file value

`cmd+0`

`toggleBrowserDeveloperTools`

Toggle browser developer tools

Default file value

`opt+cmd+i`

`showBrowserJavaScriptConsole`

Show browser JavaScript console

Default file value

`opt+cmd+c`

`toggleBrowserFocusMode`

Enter browser focus modeGives the focused web page first claim on shortcuts. Press Esc twice to exit.

Default file value

`opt+cmd+enter`

`toggleBrowserDesignMode`

Toggle browser design modeSelect and visually edit elements in the focused browser

Default file value

`ctrl+opt+cmd+d`

`toggleReactGrab`

Toggle React Grabfocused browser, or the only browser pane when a terminal is focused

Default file value

`cmd+shift+g`

#### Diff Viewer

`openDiffViewer`

Open diff viewer

Default file value

`ctrl+cmd+shift+d`

`diffViewerScrollDown`

Scroll viewer down one smooth stepfocused diff or Markdown viewer

Default file value

`j`

`diffViewerScrollUp`

Scroll viewer up one smooth stepfocused diff or Markdown viewer

Default file value

`k`

`diffViewerScrollHalfPageDown`

Scroll viewer down half a pagefocused diff or Markdown viewer

Default file value

`ctrl+d`

`diffViewerScrollHalfPageUp`

Scroll viewer up half a pagefocused diff or Markdown viewer

Default file value

`ctrl+u`

`diffViewerScrollDownEmacs`

Scroll viewer down one smooth step (Emacs)focused diff or Markdown viewer

Default file value

`ctrl+n`

`diffViewerScrollUpEmacs`

Scroll viewer up one smooth step (Emacs)focused diff or Markdown viewer

Default file value

`ctrl+p`

`diffViewerScrollToBottom`

Scroll diff to bottomfocused diff viewer

Default file value

`shift+g`

`diffViewerScrollToTop`

Scroll diff to topfocused diff viewer

Default file value

`["g", "g"]`

`diffViewerOpenFileSearch`

Open diff file searchfocused diff viewer

Default file value

`/`

`diffViewerNextFile`

Jump to next diff filefocused diff viewer

Default file value

`["]", "f"]`

`diffViewerPreviousFile`

Jump to previous diff filefocused diff viewer

Default file value

`["[", "f"]]`

#### Find

`find`

Find

Default file value

`cmd+f`

`findInDirectory`

Find in directory

Default file value

`cmd+shift+f`

`findNext`

Find next

Default file value

`cmd+g`

`findPrevious`

Find previous

Default file value

`opt+cmd+g`

`hideFind`

Hide find bar

Default file value

`opt+cmd+shift+f`

`useSelectionForFind`

Use selection for find

Default file value

`cmd+e`

#### Notifications

`showNotifications`

Show notifications

Default file value

`cmd+i`

`jumpToUnread`

Jump to latest unread

Default file value

`cmd+shift+u`

`toggleUnread`

Toggle current item unread state

Default file value

`opt+cmd+u`

`markOldestUnreadAndJumpNext`

Mark current item as oldest unread and jump to the next latest unread

Default file value

`ctrl+cmd+u`

`triggerFlash`

Flash focused panel

Default file value

`cmd+shift+h`

### [#](https://cmux-docs-release.vercel.app/docs/configuration#shortcuts-when)`shortcuts.when`

Optional per-action context predicates (VS Code-style when clauses), keyed by cmux action id. A binding only fires — and only conflicts with another binding on the same keystroke — when its clause holds. Omit a clause to keep the action's built-in context. The clause vocabulary:

-   `sidebarFocus`, `browserFocus`, `markdownFocus`, `filePreviewTextEditorFocus`, `simulatorFocus`, `terminalFocus`, `commandPaletteVisible`, `terminalFindVisible`, `workspaceCanvasLayout` — boolean keys. An unknown or absent key reads as false; the literals true and false are also accepted.
-   `sidebarMode` (`files`, `find`, `sessions`, `feed`, `dock`), `paneCount`, `workspaceCount` — typed keys for comparisons: the right sidebar's active mode (a string) plus pane and workspace counts (integers).
-   `!`, `&&`, `||`, `(…)`, `==`, `!=`, `=~`, `<`, `<=`, `>`, `>=`, `in [a, b]` — boolean operators, typed comparisons, regex match, and list membership. Comparisons bind tighter than && and ||.

For example, this makes Ctrl+1–9 select workspaces everywhere except when the right sidebar is focused (leaving Ctrl+1–5 free for the sidebar's mode switcher), and scopes surface selection to the Find sidebar when the workspace has multiple panes:

```
"shortcuts": {
  "bindings": { "selectWorkspaceByNumber": "ctrl+1" },
  "when": {
    "selectWorkspaceByNumber": "!sidebarFocus",
    "selectSurfaceByNumber": "sidebarMode == 'find' && paneCount > 1"
  }
}
```

[Workspace Groups](https://cmux-docs-release.vercel.app/docs/workspace-groups) [TextBox (Beta)](https://cmux-docs-release.vercel.app/docs/textbox)

Canonical: https://cmux-docs-release.vercel.app/docs/configuration
