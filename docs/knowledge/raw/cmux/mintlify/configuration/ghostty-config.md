> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Ghostty configuration

> How cmux reads and applies your Ghostty configuration for themes, fonts, and colors

cmux is **Ghostty compatible** — it reads your existing `~/.config/ghostty/config` for themes, fonts, and colors. Terminal rendering is powered by libghostty.

## Configuration file locations

cmux searches for Ghostty configuration files in the following order (matching Ghostty's macOS defaults):

1. `~/.config/ghostty/config`
2. `~/.config/ghostty/config.ghostty`
3. `~/Library/Application Support/com.mitchellh.ghostty/config`
4. `~/Library/Application Support/com.mitchellh.ghostty/config.ghostty`

The first file found is loaded. Subsequent files are ignored.

<Note>
  cmux loads configuration at startup and when you reload via **⌘⇧,** (Cmd+Shift+Comma) or **cmux → Reload Configuration**.
</Note>

## Supported configuration options

cmux implements a subset of Ghostty's configuration parser focused on visual appearance:

### Fonts

```ini theme={null}
font-family = "JetBrains Mono"
font-size = 14
```

* **font-family** — Font name (default: `Menlo`)
* **font-size** — Point size (default: `12`)

### Theme

```ini theme={null}
theme = "dark:monokai,light:solarized light"
```

* **theme** — Theme name or conditional theme selection

See [Themes](/configuration/themes) for detailed theme configuration.

### Colors

You can override individual colors directly in the config:

<CodeGroup>
  ```ini Basic colors theme={null}
  background = #272822
  foreground = #fdfff1
  cursor-color = #c0c1b5
  cursor-text = #8d8e82
  ```

  ```ini Selection colors theme={null}
  selection-background = #57584f
  selection-foreground = #fdfff1
  ```

  ```ini Palette colors (0-15) theme={null}
  palette = 0=#272822
  palette = 1=#f92672
  palette = 2=#a6e22e
  palette = 3=#f4bf75
  palette = 4=#66d9ef
  palette = 5=#ae81ff
  palette = 6=#a1efe4
  palette = 7=#f8f8f2
  palette = 8=#75715e
  palette = 9=#f92672
  palette = 10=#a6e22e
  palette = 11=#f4bf75
  palette = 12=#66d9ef
  palette = 13=#ae81ff
  palette = 14=#a1efe4
  palette = 15=#f9f8f5
  ```
</CodeGroup>

Color values must be in hex format: `#RRGGBB`

### Transparency

```ini theme={null}
background-opacity = 0.95
```

* **background-opacity** — 0.0 (fully transparent) to 1.0 (opaque)
* Default: `1.0`

### Split pane appearance

```ini theme={null}
unfocused-split-opacity = 0.7
unfocused-split-fill = #000000
split-divider-color = #444444
```

* **unfocused-split-opacity** — Opacity of unfocused panes (0.15–1.0, default: 0.7)
* **unfocused-split-fill** — Overlay color for unfocused panes (default: background color)
* **split-divider-color** — Color of dividers between panes (auto-computed if not set)

### Scrollback

```ini theme={null}
scrolling-limit = 10000
```

* **scrollback-limit** — Number of lines to keep in scrollback (default: 10000)

### Working directory

```ini theme={null}
working-directory = ~/projects
```

* **working-directory** — Default directory for new terminal sessions

## Configuration reloading

Reload your configuration without restarting:

* **⌘⇧,** (Cmd+Shift+Comma)
* Menu: **cmux → Reload Configuration**

This re-reads your Ghostty config files and applies changes to all active terminals.

<Note>
  Theme changes apply immediately. Font changes may require creating a new surface or workspace to take full effect.
</Note>

## Example configuration

Here's a complete example combining common settings:

```ini ~/.config/ghostty/config theme={null}
# Font
font-family = "JetBrains Mono"
font-size = 14

# Theme with light/dark variants
theme = dark:monokai,light:solarized light

# Transparency
background-opacity = 0.95

# Split pane appearance
unfocused-split-opacity = 0.8
split-divider-color = #444444

# Scrollback
scrolling-limit = 50000

# Working directory
working-directory = ~/projects
```

## Environment variables

cmux sets the following Ghostty-compatible environment variables:

* `TERM=xterm-ghostty`
* `TERM_PROGRAM=ghostty`
* `GHOSTTY_RESOURCES_DIR` — Path to Ghostty resources (themes, terminfo, etc.)

These are inherited by all terminal sessions.

## Relationship to Ghostty

cmux uses **libghostty** for terminal rendering but is a separate application. Key differences:

* cmux adds vertical tabs, notifications, and workspace management
* cmux uses its own bundle ID (`com.cmuxterm.app`) and socket path (`/tmp/cmux.sock`)
* Not all Ghostty config options are supported (see list above)
* cmux-specific features are configured via in-app settings (⌘,)

<Note>
  You can run cmux and Ghostty side-by-side. They share the same config files but operate independently.
</Note>

## Troubleshooting

<Accordion title="Theme not loading">
  1. Check that your theme file exists in `~/.config/ghostty/themes/` or the Ghostty bundle
  2. Verify the theme name matches exactly (case-sensitive)
  3. Try reloading configuration with ⌘⇧,
  4. Check the theme syntax — see [Themes](/configuration/themes)
</Accordion>

<Accordion title="Font not applying">
  1. Verify the font is installed on your system
  2. Use the exact font family name (check Font Book.app)
  3. Reload configuration with ⌘⇧,
  4. Create a new workspace to see font changes
</Accordion>

<Accordion title="Colors look wrong">
  1. Check for conflicting color definitions in your config
  2. Verify hex colors are in `#RRGGBB` format
  3. If using a theme, individual color overrides take precedence
  4. Try commenting out custom colors to test theme defaults
</Accordion>

<Accordion title="Configuration not reloading">
  1. Verify file path: `~/.config/ghostty/config` expands correctly
  2. Check file permissions (should be readable)
  3. Look for syntax errors (invalid keys are silently ignored)
  4. Try restarting cmux as a last resort
</Accordion>
