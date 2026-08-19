> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Themes

> Configure and customize terminal color schemes using Ghostty themes

cmux supports Ghostty's theme system, allowing you to use any Ghostty-compatible theme file.

## Setting a theme

Add a `theme` line to your `~/.config/ghostty/config`:

```ini theme={null}
theme = monokai
```

Reload configuration with **⌘⇧,** to apply changes.

## Conditional themes (light/dark)

Specify different themes for light and dark mode:

```ini theme={null}
theme = dark:monokai,light:solarized light
```

The syntax is:

```
theme = dark:<dark-theme>,light:<light-theme>
```

cmux automatically switches themes when macOS appearance changes or when you change the app theme in Settings.

<Note>
  You can also specify a single theme without prefixes as a fallback:

  ```ini theme={null}
  theme = tokyo-night,dark:tokyo-night-storm,light:tokyo-night-day
  ```

  The unprefixed theme is used if neither dark nor light variant matches.
</Note>

## Theme file locations

cmux searches for theme files in the following order:

1. `$GHOSTTY_RESOURCES_DIR/themes/<theme-name>` (if set)
2. App bundle: `cmux.app/Contents/Resources/ghostty/themes/<theme-name>`
3. `$XDG_DATA_DIRS/ghostty/themes/<theme-name>` (colon-separated paths)
4. `/Applications/Ghostty.app/Contents/Resources/ghostty/themes/<theme-name>`
5. `~/.config/ghostty/themes/<theme-name>`
6. `~/Library/Application Support/com.mitchellh.ghostty/themes/<theme-name>`

<Accordion title="Theme name resolution">
  cmux supports Ghostty's theme naming conventions:

  * `monokai` → searches for file named `monokai`
  * `Monokai Pro` → searches for `Monokai Pro`
  * `builtin monokai` → strips "builtin" prefix → searches for `monokai`
  * `monokai (builtin)` → strips "(builtin)" suffix → searches for `monokai`

  Theme name resolution is case-sensitive.
</Accordion>

## Theme file format

Theme files use the same syntax as Ghostty config files:

<CodeGroup>
  ```ini Monokai example theme={null}
  # Basic colors
  background = #272822
  foreground = #fdfff1
  cursor-color = #c0c1b5
  cursor-text = #8d8e82
  selection-background = #57584f
  selection-foreground = #fdfff1

  # 16-color palette
  palette = 0=#272822   # black
  palette = 1=#f92672   # red
  palette = 2=#a6e22e   # green
  palette = 3=#f4bf75   # yellow
  palette = 4=#66d9ef   # blue
  palette = 5=#ae81ff   # magenta
  palette = 6=#a1efe4   # cyan
  palette = 7=#f8f8f2   # white
  palette = 8=#75715e   # bright black
  palette = 9=#f92672   # bright red
  palette = 10=#a6e22e  # bright green
  palette = 11=#f4bf75  # bright yellow
  palette = 12=#66d9ef  # bright blue
  palette = 13=#ae81ff  # bright magenta
  palette = 14=#a1efe4  # bright cyan
  palette = 15=#f9f8f5  # bright white
  ```

  ```ini Solarized Light example theme={null}
  background = #fdf6e3
  foreground = #657b83
  cursor-color = #657b83
  cursor-text = #fdf6e3
  selection-background = #eee8d5
  selection-foreground = #657b83

  palette = 0=#073642
  palette = 1=#dc322f
  palette = 2=#859900
  palette = 3=#b58900
  palette = 4=#268bd2
  palette = 5=#d33682
  palette = 6=#2aa198
  palette = 7=#eee8d5
  palette = 8=#002b36
  palette = 9=#cb4b16
  palette = 10=#586e75
  palette = 11=#657b83
  palette = 12=#839496
  palette = 13=#6c71c4
  palette = 14=#93a1a1
  palette = 15=#fdf6e3
  ```
</CodeGroup>

### Required fields

* `background` — Terminal background color
* `foreground` — Default text color
* `palette` entries (0–15) — ANSI color palette

### Optional fields

* `cursor-color` — Cursor fill color (defaults to foreground)
* `cursor-text` — Text color under cursor (defaults to background)
* `selection-background` — Background color for selected text
* `selection-foreground` — Text color for selected text
* `background-opacity` — Transparency (0.0–1.0)

## Creating custom themes

1. Create `~/.config/ghostty/themes/` directory:
   ```bash theme={null}
   mkdir -p ~/.config/ghostty/themes
   ```

2. Create a theme file (e.g., `my-theme`):
   ```bash theme={null}
   touch ~/.config/ghostty/themes/my-theme
   ```

3. Edit the file with your colors:
   ```ini ~/.config/ghostty/themes/my-theme theme={null}
   background = #1e1e2e
   foreground = #cdd6f4
   cursor-color = #f5e0dc

   palette = 0=#45475a
   palette = 1=#f38ba8
   # ... add all 16 palette entries
   ```

4. Reference it in your config:
   ```ini ~/.config/ghostty/config theme={null}
   theme = my-theme
   ```

5. Reload configuration: **⌘⇧,**

<Note>
  You can also copy existing themes from Ghostty's theme directory as starting points.
</Note>

## Compatibility aliases

cmux includes compatibility aliases for popular themes:

* `solarized light` → tries `iTerm2 Solarized Light` as fallback
* `solarized dark` → tries `iTerm2 Solarized Dark` as fallback

This improves compatibility with theme collections that use different naming conventions.

## Overriding theme colors

You can override specific colors from a theme in your main config file:

```ini ~/.config/ghostty/config theme={null}
theme = monokai

# Override background with transparency
background = #272822
background-opacity = 0.95

# Override cursor colors
cursor-color = #ff6188
cursor-text = #2d2a2e
```

Config file color definitions take precedence over theme file values.

## Built-in themes

If you have Ghostty installed at `/Applications/Ghostty.app`, cmux can access its bundled themes:

* 3024 Day / Night
* Alabaster
* Argonaut
* Ashes
* Atom One Dark / Light
* Breeze
* Builtin Tango variants
* Catppuccin (Frappe, Latte, Macchiato, Mocha)
* Dracula
* Flexoki (Dark, Light)
* Gruvbox (Dark, Light)
* Nord
* One Half (Dark, Light)
* Tokyo Night (Day, Moon, Night, Storm)
* ... and many more

Check `/Applications/Ghostty.app/Contents/Resources/ghostty/themes/` for the complete list.

## Terminal color testing

Test your theme with this snippet:

```bash theme={null}
# 16 ANSI colors
for i in {0..15}; do
  printf "\e[48;5;${i}m  \e[0m"
  [ $((($i + 1) % 8)) -eq 0 ] && echo
done
echo

# 256 color palette preview
for i in {16..255}; do
  printf "\e[48;5;${i}m  \e[0m"
  [ $((($i + 1) % 16)) -eq 0 ] && echo
done
echo
```

This displays all palette colors in your current terminal.

## Troubleshooting

<Accordion title="Theme not found">
  1. Verify the theme file exists:
     ```bash theme={null}
     ls ~/.config/ghostty/themes/
     ```
  2. Check the theme name matches exactly (case-sensitive)
  3. Try the full path:
     ```ini theme={null}
     theme = ~/.config/ghostty/themes/my-theme
     ```
  4. Look for the theme in Ghostty bundle:
     ```bash theme={null}
     ls /Applications/Ghostty.app/Contents/Resources/ghostty/themes/
     ```
</Accordion>

<Accordion title="Colors not applying">
  1. Verify all palette entries (0–15) are defined
  2. Check hex format: must be `#RRGGBB` (6 digits)
  3. Reload config with ⌘⇧,
  4. Check for conflicting color definitions in main config
  5. Try a known working theme (e.g., `monokai`) to isolate the issue
</Accordion>

<Accordion title="Light/dark switching not working">
  1. Verify syntax: `theme = dark:theme1,light:theme2`
  2. Check both theme files exist
  3. Test by changing app theme in Settings (⌘,)
  4. Try using `theme = system` in a theme file to delegate to main config
</Accordion>

<Accordion title="Transparency not working">
  Background transparency requires:

  1. `background-opacity` set to less than 1.0
  2. macOS compositing enabled (should be default)
  3. No background image set in the theme

  If still not working, try:

  ```ini theme={null}
  background-opacity = 0.95
  background = #000000
  ```
</Accordion>

## Resources

* [Ghostty theme documentation](https://ghostty.org/docs/themes)
* [iTerm2 Color Schemes](https://iterm2colorschemes.com/) — Many are Ghostty-compatible
* [Gogh](https://gogh-co.github.io/Gogh/) — Terminal color scheme collection
