> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Browser

> Built-in WebKit browser with scriptable API for agent interaction

cmux includes a WKWebView-based browser that can be split alongside terminals. The browser has a scriptable API ported from [agent-browser](https://github.com/vercel-labs/agent-browser) for AI agent interaction.

## Opening the browser

Create a browser surface:

* Press <kbd>⌘</kbd> <kbd>⇧</kbd> <kbd>L</kbd> to open a browser in a split
* Use the CLI: `cmux new-pane --type browser`
* Click the browser split button in the UI

The browser opens in a new split pane next to your terminal.

## Browser features

### Navigation

| Shortcut                   | Action            |
| -------------------------- | ----------------- |
| <kbd>⌘</kbd> <kbd>L</kbd>  | Focus address bar |
| <kbd>⌘</kbd> <kbd>\[</kbd> | Back              |
| <kbd>⌘</kbd> <kbd>]</kbd>  | Forward           |
| <kbd>⌘</kbd> <kbd>R</kbd>  | Reload page       |

The address bar supports:

* Direct URL entry (http/https)
* Search queries (uses configured search engine)
* History suggestions
* Autocomplete from browser history

### Search engines

Configure your preferred search engine in Settings → Browser:

* Google (default)
* DuckDuckGo
* Bing
* Kagi

When you type a non-URL in the address bar, cmux searches using the selected engine.

<Note>
  Search suggestions are fetched from the search engine's API and displayed as you type. You can disable this in Settings → Browser → Search Suggestions.
</Note>

### Developer tools

Access WebKit developer tools for debugging:

| Shortcut                               | Action                  |
| -------------------------------------- | ----------------------- |
| <kbd>⌥</kbd> <kbd>⌘</kbd> <kbd>I</kbd> | Toggle Developer Tools  |
| <kbd>⌥</kbd> <kbd>⌘</kbd> <kbd>C</kbd> | Show JavaScript Console |

Developer tools open in a split pane below the browser, similar to Safari's inspector.

<Info>
  Developer tools shortcuts follow Safari defaults and are customizable in Settings → Keyboard Shortcuts.
</Info>

## Browser theme

The browser adapts to Ghostty's terminal theme:

* Background color matches the terminal background
* Supports transparent/translucent backgrounds
* Light/dark mode based on system appearance or custom override

Change the browser theme mode in Settings → Browser → Theme:

* **System**: Follow macOS appearance
* **Light**: Force light mode
* **Dark**: Force dark mode

Forced dark mode uses WKWebView's content appearance API to render pages in dark mode even if the page doesn't provide a dark theme.

## Security and privacy

### Insecure HTTP blocking

By default, cmux blocks insecure HTTP URLs except for localhost and common development hosts:

* `localhost`
* `127.0.0.1`
* `::1`
* `0.0.0.0`
* `*.localtest.me`

Add additional hosts to the allowlist in Settings → Browser → Insecure HTTP Allowlist.

When you navigate to a blocked HTTP URL, cmux prompts:

* "Allow Once" (one-time bypass)
* "Always Allow" (add to persistent allowlist)
* "Cancel"

<Warning>
  Insecure HTTP connections are vulnerable to man-in-the-middle attacks. Only allow hosts you trust.
</Warning>

### Cookie and session sharing

All browser panels share a WKProcessPool, meaning:

* Cookies are shared across all browser tabs
* You stay logged in when you split or create new browser surfaces
* Session storage is shared within the same origin

Clear cookies and site data via WKWebView's data store (not yet exposed in UI).

### Opening external links

By default, cmux intercepts terminal link clicks and opens them in the built-in browser:

* **Terminal links** (Cmd+Click on URLs): Open in cmux browser
* **Sidebar PR links**: Open in cmux browser
* **`open` command interception**: Redirect `open <url>` to cmux browser

Disable these behaviors in Settings → Browser if you prefer using your default external browser.

#### Host whitelist

Restrict which hosts open in cmux browser vs. external browser using the host whitelist (Settings → Browser → Host Whitelist):

* Empty whitelist = all hosts allowed
* Add specific hosts (e.g., `localhost`, `github.com`)
* Supports wildcards (e.g., `*.vercel.app`)

Hosts not in the whitelist open externally in your default browser.

## Browser history

cmux maintains a local browser history store:

* Persisted to `~/Library/Application Support/com.cmuxterm.app/browser_history.json`
* Stores URL, title, visit count, and last visited timestamp
* Powers address bar autocomplete and suggestions
* Automatically deduplicates URLs (ignores trailing slashes, www prefix)

History is shared across all browser surfaces in the app.

### Clear history

Clear browser history in Settings → Browser → Clear History. This removes all stored URLs and resets autocomplete suggestions.

## Scriptable API

The browser includes a scriptable API for AI agent interaction, ported from [agent-browser](https://github.com/vercel-labs/agent-browser):

### Accessibility tree snapshot

Capture the accessibility tree for page structure:

```bash theme={null}
cmux browser snapshot
```

Returns a JSON representation of the page's accessibility tree, useful for agents to understand page layout without parsing raw HTML.

### Element interaction

Agents can interact with page elements:

```bash theme={null}
# Click an element by reference
cmux browser click --ref <element-ref>

# Fill a form field
cmux browser fill --ref <element-ref> --value "text"

# Evaluate JavaScript
cmux browser eval "document.title"
```

These commands use element references from the accessibility tree snapshot.

<Info>
  The scriptable API is designed for programmatic control by agents. For interactive browsing, use the standard keyboard shortcuts and mouse.
</Info>

## Page zoom

Zoom in/out on web pages:

* Browser zoom is separate from terminal font zoom
* Zoom level is saved per browser panel and restored on session reload
* Defaults to 100% (1.0x)

## Navigation history

Each browser panel maintains its own navigation history:

* Back/forward buttons in the UI
* Keyboard shortcuts <kbd>⌘</kbd> <kbd>\[</kbd> / <kbd>⌘</kbd> <kbd>]</kbd>
* History is saved in session state and restored on app relaunch

Navigation history includes:

* Back history (pages you visited before current)
* Forward history (pages you can go forward to)
* Current URL

Closing and reopening the browser preserves navigation state.

## Telemetry hooks

The browser injects JavaScript hooks to capture:

* Console messages (log, info, warn, error, debug)
* Unhandled errors and promise rejections
* Dialog calls (alert, confirm, prompt)

These hooks are used internally for debugging and agent telemetry. They don't send data externally.

<Note>
  Telemetry hooks are installed via content scripts and don't interfere with normal page execution.
</Note>
