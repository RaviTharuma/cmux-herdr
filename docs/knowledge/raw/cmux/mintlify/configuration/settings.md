> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Settings

> Configure cmux behavior, appearance, and integrations through the in-app settings panel

Access cmux settings via **⌘,** (Cmd+Comma) or **cmux → Settings** from the menu bar.

## App

### Appearance

<Accordion title="Theme">
  Choose between light, dark, or system appearance mode.

  * **System** — Follows macOS system appearance
  * **Light** — Always use light mode
  * **Dark** — Always use dark mode
</Accordion>

<Accordion title="App icon">
  Select from multiple app icon variants. Changes apply immediately without relaunch.
</Accordion>

### Workspace behavior

<Accordion title="New workspace placement">
  Controls where new workspaces appear in the sidebar.

  * **End** — Add to the end of the list
  * **After current** — Insert after the active workspace
  * **Start** — Add to the beginning
</Accordion>

<Accordion title="Reorder on notification">
  When enabled, workspaces move to the top when they receive a notification.

  <Note>
    Disable this if you rely on stable workspace positions for keyboard shortcuts (⌘1–⌘8).
  </Note>
</Accordion>

### Sidebar display options

<Accordion title="Show branch + directory in sidebar">
  Display the git branch and working directory for each workspace.
</Accordion>

<Accordion title="Show pull requests in sidebar">
  Display PR/MR status, number, and clickable links.

  **Open sidebar PR links in cmux browser** — When enabled, clicking PR links opens them in the embedded browser instead of your default browser.
</Accordion>

<Accordion title="Show listening ports in sidebar">
  Display detected listening ports for the active workspace. Useful for tracking dev servers.
</Accordion>

<Accordion title="Show latest log in sidebar">
  Display the latest log/status message from `cmux log` commands.
</Accordion>

<Accordion title="Show progress in sidebar">
  Display progress bars set via `cmux set_progress`.
</Accordion>

<Accordion title="Show custom metadata in sidebar">
  Display custom metadata from `report_meta`/`set_status` and `report_meta_block` socket commands.
</Accordion>

<Accordion title="Sidebar branch layout">
  * **Vertical** — Each branch appears on its own line
  * **Inline** — All branches share one line
</Accordion>

### Notifications

<Accordion title="Dock badge">
  Show unread notification count on the app icon in the Dock and Cmd+Tab switcher.
</Accordion>

### Other

<Accordion title="Send anonymous telemetry">
  Share anonymized crash and usage data to help improve cmux. Changes take effect on next launch.
</Accordion>

<Accordion title="Warn before quit">
  Show a confirmation dialog before quitting with ⌘Q. When disabled, ⌘Q quits immediately.
</Accordion>

<Accordion title="Rename selects existing name">
  When enabled, the Command Palette rename action starts with all text selected. When disabled, the caret stays at the end.
</Accordion>

## Workspace colors

<Accordion title="Workspace color indicator">
  Choose how workspace colors appear in the sidebar:

  * **Accent** — Colored accent on the left edge
  * **Underline** — Colored line at the bottom
  * **None** — No color indicator
</Accordion>

<Accordion title="Customize palette">
  Modify the default workspace color palette. The settings panel shows:

  * Built-in color entries with hex values
  * Custom colors added via "Choose Custom Color..." from workspace context menus
  * Reset button to restore defaults

  Each color entry shows:

  * Color picker for visual editing
  * Hex value in monospace font
  * Base color reference for built-in entries
</Accordion>

## Automation

### Socket control mode

Controls access to the local Unix socket at `/tmp/cmux.sock` for programmatic control.

<Accordion title="Off">
  Disable the local control socket entirely.
</Accordion>

<Accordion title="cmux processes only">
  Default. Only processes started inside cmux terminals can send commands.
</Accordion>

<Accordion title="Automation mode">
  Allow external local automation clients from this macOS user (no ancestry check).
</Accordion>

<Accordion title="Password mode">
  Require socket authentication with a password stored in `~/Library/Application Support/cmux/socket-control-password`.

  The settings panel provides:

  * Secure password field for entry
  * **Set/Change** button to save password
  * **Clear** button to remove password (when one exists)
  * Status messages for save/clear operations

  <Note>
    External clients are blocked until a password is configured.
  </Note>
</Accordion>

<Accordion title="Full open access">
  <Warning>
    Makes the control socket world-readable/writable on this Mac and disables auth checks. Use only for local debugging.
  </Warning>

  Requires confirmation before enabling.
</Accordion>

**Environment variable overrides:**

* `CMUX_SOCKET_ENABLE` — `1`/`true`/`yes`/`on` or `0`/`false`/`no`/`off`
* `CMUX_SOCKET_MODE` — `off`, `cmuxOnly`, `automation`, `password`, `allowAll`
* `CMUX_SOCKET_PATH` — Custom socket path (requires `CMUX_ALLOW_SOCKET_OVERRIDE=1` for stable/nightly builds)
* `CMUX_SOCKET_PASSWORD` — Password for password mode (alternative to file storage)

### Claude Code integration

<Accordion title="Enable Claude Code hooks">
  When enabled, cmux wraps the `claude` command to inject session tracking and notification hooks. The sidebar shows Claude session status and notifications.

  Disable if you prefer to manage Claude Code hooks yourself.
</Accordion>

### Port ranges

<Accordion title="Port base and range">
  Each workspace gets `CMUX_PORT` and `CMUX_PORT_END` environment variables with a dedicated port range. New terminals inherit these values.

  * **Port Base** — Starting port (default: 9100)
  * **Port Range Size** — Number of ports per workspace (default: 10)

  Example: With base 9100 and range 10, workspace 1 gets ports 9100–9109, workspace 2 gets 9110–9119, etc.
</Accordion>

## Browser

### Search

<Accordion title="Default search engine">
  Used by the browser address bar when input is not a URL.

  Available engines:

  * Google
  * DuckDuckGo
  * Bing
  * Yahoo
  * Ecosia
</Accordion>

<Accordion title="Show search suggestions">
  Enable/disable search suggestions in the address bar.
</Accordion>

### Appearance

<Accordion title="Browser theme">
  * **System** — Follows app and macOS appearance
  * **Light** — Forces light mode for compatible pages
  * **Dark** — Forces dark mode for compatible pages
</Accordion>

### Link behavior

<Accordion title="Open terminal links in cmux browser">
  When enabled, clicking links in terminal output opens them in the embedded browser. When disabled, links open in your default browser.
</Accordion>

<Accordion title="Intercept open http(s) in terminal">
  When enabled, `open https://...` and `open http://...` commands open in the cmux browser (subject to host whitelist). When disabled, they always use your default browser.
</Accordion>

<Accordion title="Hosts to open in embedded browser">
  Multi-line text field for host patterns. Applies to terminal link clicks and intercepted `open https://...` calls.

  * One host or wildcard per line
  * Examples: `example.com`, `*.internal.example`
  * Leave empty to open all hosts in cmux
  * Only matching hosts open in cmux; others use your default browser
</Accordion>

### Security

<Accordion title="HTTP hosts allowed in embedded browser">
  Controls which HTTP (non-HTTPS) hosts can open in cmux without a warning prompt.

  Default allowlist includes:

  * `localhost`
  * `127.0.0.1`
  * `::1`
  * `0.0.0.0`
  * `*.localtest.me`

  Add one host or wildcard per line. Click **Save** to apply changes.
</Accordion>

### History

<Accordion title="Browsing history">
  View count of saved pages that appear in omnibar suggestions. Click **Clear History...** to remove all entries.
</Accordion>

## Keyboard shortcuts

All keyboard shortcuts are customizable. Click a shortcut value to record a new one. Press **Esc** to cancel recording.

<Note>
  See [Keyboard shortcuts](/features/keyboard-shortcuts) for the full list with default bindings.
</Note>

Available actions:

* Toggle Sidebar
* New Workspace / Surface / Window
* Close Workspace / Window
* Show Notifications / Jump to Latest Unread
* Next/Previous Workspace / Surface
* Rename Workspace / Tab
* Focus Pane (Left/Right/Up/Down)
* Split Right/Down
* Split Browser Right/Down
* Toggle Pane Zoom
* Open Browser / Folder
* Toggle Browser Developer Tools
* Show Browser JavaScript Console
* Flash Focused Panel

## Reset

**Reset All Settings** — Restores all in-app settings to factory defaults, including:

* Appearance and theme
* Socket control mode
* Browser settings
* Keyboard shortcuts
* Sidebar display options
* Workspace colors

<Warning>
  This action cannot be undone. Your Ghostty configuration (`~/.config/ghostty/config`) is not affected.
</Warning>
