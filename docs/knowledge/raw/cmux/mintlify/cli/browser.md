> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Browser Commands

> Control embedded browser surfaces with CLI automation

Control browser surfaces in cmux for web automation. All browser commands use the `browser` namespace or legacy aliases.

## browser identify

Get information about a browser surface including URL and title.

```bash theme={null}
cmux browser [surface] identify
```

**Example:**

```bash theme={null}
cmux browser surface:1 identify
```

**Output (JSON with --json):**

```json theme={null}
{
  "browser": {
    "surface": "surface:1",
    "url": "https://example.com",
    "title": "Example Domain"
  }
}
```

## browser open / open-split

Open a URL in a new or existing browser surface.

```bash theme={null}
cmux browser open [--workspace <id>] [--window <id>] [url]
cmux browser open-split [--workspace <id>] [url]
```

**Flags:**

<ParamField path="--workspace" type="string">
  Workspace context (defaults to `$CMUX_WORKSPACE_ID`)
</ParamField>

<ParamField path="--window" type="string">
  Window context for routing
</ParamField>

**Examples:**

<CodeGroup>
  ```bash Open URL theme={null}
  cmux browser open https://github.com
  ```

  ```bash Open in Workspace theme={null}
  cmux browser open --workspace workspace:2 https://docs.example.com
  ```

  ```bash Navigate Existing Surface theme={null}
  cmux browser surface:1 open https://new-url.com
  ```
</CodeGroup>

**Output:**

```
OK surface=surface:2 pane=pane:1 placement=split
```

## browser navigate / goto

Navigate a browser surface to a URL.

```bash theme={null}
cmux browser <surface> navigate <url> [--snapshot-after]
```

**Flags:**

<ParamField path="--snapshot-after" type="boolean">
  Capture page snapshot after navigation completes
</ParamField>

**Examples:**

<CodeGroup>
  ```bash Navigate theme={null}
  cmux browser surface:1 navigate https://example.com
  ```

  ```bash With Snapshot theme={null}
  cmux browser surface:1 navigate https://example.com --snapshot-after
  ```
</CodeGroup>

**Output:**

```
OK
```

## browser back / forward / reload

Navigate browser history or reload the page.

```bash theme={null}
cmux browser <surface> back [--snapshot-after]
cmux browser <surface> forward [--snapshot-after]
cmux browser <surface> reload [--snapshot-after]
```

**Examples:**

<CodeGroup>
  ```bash Back theme={null}
  cmux browser surface:1 back
  ```

  ```bash Forward theme={null}
  cmux browser surface:1 forward
  ```

  ```bash Reload theme={null}
  cmux browser surface:1 reload
  ```
</CodeGroup>

## browser url / get-url

Get the current URL of a browser surface.

```bash theme={null}
cmux browser <surface> url
cmux browser <surface> get-url
```

**Example:**

```bash theme={null}
cmux browser surface:1 url
```

**Output:**

```
https://example.com
```

## browser snapshot

Capture a text snapshot of the page DOM.

```bash theme={null}
cmux browser <surface> snapshot [--selector <css>] [--max-depth <n>] [--interactive] [--cursor] [--compact]
```

**Flags:**

<ParamField path="--selector" type="string">
  CSS selector to snapshot (defaults to entire page)
</ParamField>

<ParamField path="--max-depth" type="number">
  Maximum DOM depth to traverse
</ParamField>

<ParamField path="--interactive" type="boolean">
  Include interactive elements only
</ParamField>

<ParamField path="-i" type="boolean">
  Alias for `--interactive`
</ParamField>

<ParamField path="--cursor" type="boolean">
  Include cursor position indicators
</ParamField>

<ParamField path="--compact" type="boolean">
  Use compact output format
</ParamField>

**Examples:**

<CodeGroup>
  ```bash Full Page theme={null}
  cmux browser surface:1 snapshot
  ```

  ```bash Interactive Elements theme={null}
  cmux browser surface:1 snapshot --interactive
  ```

  ```bash Specific Element theme={null}
  cmux browser surface:1 snapshot --selector "#main-content"
  ```

  ```bash With Depth Limit theme={null}
  cmux browser surface:1 snapshot --max-depth 3 --compact
  ```
</CodeGroup>

## browser eval

Evaluate JavaScript in the page context.

```bash theme={null}
cmux browser <surface> eval [--script <js>] <javascript>
```

**Examples:**

<CodeGroup>
  ```bash Simple Expression theme={null}
  cmux browser surface:1 eval "document.title"
  ```

  ```bash With Flag theme={null}
  cmux browser surface:1 eval --script "window.location.href"
  ```

  ```bash Complex Script theme={null}
  cmux browser surface:1 eval "Array.from(document.querySelectorAll('a')).map(a => a.href)"
  ```
</CodeGroup>

**Output:**
Returns the evaluated value as text or JSON.

## browser wait

Wait for a condition to be met on the page.

```bash theme={null}
cmux browser <surface> wait [--selector <css>] [--text <text>] [--url-contains <text>] [--load-state <state>] [--function <js>] [--timeout <seconds>]
```

**Flags:**

<ParamField path="--selector" type="string">
  CSS selector to wait for
</ParamField>

<ParamField path="--text" type="string">
  Wait for element containing text
</ParamField>

<ParamField path="--url-contains" type="string">
  Wait for URL to contain text
</ParamField>

<ParamField path="--url" type="string">
  Alias for `--url-contains`
</ParamField>

<ParamField path="--load-state" type="string">
  Wait for load state: `load`, `domcontentloaded`, `networkidle`
</ParamField>

<ParamField path="--function" type="string">
  JavaScript function that returns truthy when condition is met
</ParamField>

<ParamField path="--timeout" type="number">
  Timeout in seconds (default varies by condition)
</ParamField>

<ParamField path="--timeout-ms" type="number">
  Timeout in milliseconds
</ParamField>

**Examples:**

<CodeGroup>
  ```bash Wait for Selector theme={null}
  cmux browser surface:1 wait --selector ".loaded"
  ```

  ```bash Wait for Text theme={null}
  cmux browser surface:1 wait --selector "h1" --text "Welcome"
  ```

  ```bash Wait for URL theme={null}
  cmux browser surface:1 wait --url-contains "/dashboard"
  ```

  ```bash Wait for Load State theme={null}
  cmux browser surface:1 wait --load-state networkidle
  ```

  ```bash Wait for Custom Function theme={null}
  cmux browser surface:1 wait --function "() => document.readyState === 'complete'"
  ```

  ```bash With Timeout theme={null}
  cmux browser surface:1 wait --selector ".content" --timeout 10
  ```
</CodeGroup>

## browser click / dblclick / hover

Interact with page elements.

```bash theme={null}
cmux browser <surface> click [--selector <css>] <selector> [--snapshot-after]
cmux browser <surface> dblclick [--selector <css>] <selector> [--snapshot-after]
cmux browser <surface> hover [--selector <css>] <selector> [--snapshot-after]
```

**Examples:**

<CodeGroup>
  ```bash Click theme={null}
  cmux browser surface:1 click "button#submit"
  ```

  ```bash Double Click theme={null}
  cmux browser surface:1 dblclick ".editable"
  ```

  ```bash Hover theme={null}
  cmux browser surface:1 hover "nav .dropdown"
  ```

  ```bash With Snapshot theme={null}
  cmux browser surface:1 click ".load-more" --snapshot-after
  ```
</CodeGroup>

## browser type / fill

Type text into input fields.

```bash theme={null}
cmux browser <surface> type [--selector <css>] <selector> [--text <text>] <text> [--snapshot-after]
cmux browser <surface> fill [--selector <css>] <selector> [--text <text>] <text> [--snapshot-after]
```

**Note:** `type` requires text, `fill` allows empty string to clear.

**Examples:**

<CodeGroup>
  ```bash Type Text theme={null}
  cmux browser surface:1 type "input#search" "query text"
  ```

  ```bash Fill with Flag theme={null}
  cmux browser surface:1 fill --selector "input[name='email']" --text "user@example.com"
  ```

  ```bash Clear Field theme={null}
  cmux browser surface:1 fill "input#name" ""
  ```
</CodeGroup>

## browser press / key / keydown / keyup

Simulate keyboard input.

```bash theme={null}
cmux browser <surface> press [--key <key>] <key> [--snapshot-after]
cmux browser <surface> key [--key <key>] <key>
cmux browser <surface> keydown <key>
cmux browser <surface> keyup <key>
```

**Examples:**

<CodeGroup>
  ```bash Press Enter theme={null}
  cmux browser surface:1 press Enter
  ```

  ```bash Press Escape theme={null}
  cmux browser surface:1 key Escape
  ```

  ```bash Key Down/Up theme={null}
  cmux browser surface:1 keydown Shift
  cmux browser surface:1 keyup Shift
  ```
</CodeGroup>

## browser check / uncheck

Toggle checkboxes.

```bash theme={null}
cmux browser <surface> check <selector> [--snapshot-after]
cmux browser <surface> uncheck <selector> [--snapshot-after]
```

**Examples:**

```bash theme={null}
cmux browser surface:1 check "input[type='checkbox']#terms"
cmux browser surface:1 uncheck "#newsletter"
```

## browser select

Select a dropdown option.

```bash theme={null}
cmux browser <surface> select [--selector <css>] <selector> [--value <value>] <value> [--snapshot-after]
```

**Example:**

```bash theme={null}
cmux browser surface:1 select "select#country" "US"
```

## browser scroll

Scroll the page or an element.

```bash theme={null}
cmux browser <surface> scroll [--selector <css>] [--dx <pixels>] [--dy <pixels>] [dy] [--snapshot-after]
```

**Flags:**

<ParamField path="--selector" type="string">
  Element to scroll (defaults to page)
</ParamField>

<ParamField path="--dx" type="number">
  Horizontal scroll amount in pixels
</ParamField>

<ParamField path="--dy" type="number">
  Vertical scroll amount in pixels (can be positional)
</ParamField>

**Examples:**

<CodeGroup>
  ```bash Scroll Down theme={null}
  cmux browser surface:1 scroll --dy 500
  ```

  ```bash Positional theme={null}
  cmux browser surface:1 scroll 300
  ```

  ```bash Scroll Element theme={null}
  cmux browser surface:1 scroll --selector ".scrollable" --dy 200
  ```
</CodeGroup>

## browser scrollintoview / scroll-into-view

Scroll an element into view.

```bash theme={null}
cmux browser <surface> scrollintoview <selector> [--snapshot-after]
```

**Example:**

```bash theme={null}
cmux browser surface:1 scrollintoview "#footer"
```

## browser focus

Focus a page element.

```bash theme={null}
cmux browser <surface> focus <selector> [--snapshot-after]
```

**Example:**

```bash theme={null}
cmux browser surface:1 focus "input#email"
```

## browser focus-webview

Focus the browser's webview (give keyboard control to the page).

```bash theme={null}
cmux browser <surface> focus-webview
```

## browser is-webview-focused

Check if the browser's webview has focus.

```bash theme={null}
cmux browser <surface> is-webview-focused
```

**Output:**

```
true
```

## browser screenshot

Capture a PNG screenshot of the page.

```bash theme={null}
cmux browser <surface> screenshot [--out <path>]
```

**Flags:**

<ParamField path="--out" type="string">
  Output file path for PNG screenshot
</ParamField>

**Examples:**

<CodeGroup>
  ```bash To File theme={null}
  cmux browser surface:1 screenshot --out page.png
  ```

  ```bash JSON (base64) theme={null}
  cmux browser surface:1 screenshot --json
  ```
</CodeGroup>

**Output:**

```
OK page.png
```

## browser get

Extract data from page elements.

```bash theme={null}
cmux browser <surface> get <subcommand> [options]
```

**Subcommands:**

### get url

```bash theme={null}
cmux browser surface:1 get url
```

### get title

```bash theme={null}
cmux browser surface:1 get title
```

### get text

```bash theme={null}
cmux browser surface:1 get text [--selector] <selector>
```

### get html

```bash theme={null}
cmux browser surface:1 get html [--selector] <selector>
```

### get value

```bash theme={null}
cmux browser surface:1 get value [--selector] <selector>
```

### get attr

```bash theme={null}
cmux browser surface:1 get attr [--selector] <selector> --attr <name>
```

**Example:**

```bash theme={null}
cmux browser surface:1 get attr "a.link" --attr href
```

### get count

```bash theme={null}
cmux browser surface:1 get count [--selector] <selector>
```

### get box

```bash theme={null}
cmux browser surface:1 get box [--selector] <selector>
```

Returns bounding box coordinates.

### get styles

```bash theme={null}
cmux browser surface:1 get styles [--selector] <selector> [--property <name>]
```

**Examples:**

<CodeGroup>
  ```bash Get Text theme={null}
  cmux browser surface:1 get text "h1"
  ```

  ```bash Get Attribute theme={null}
  cmux browser surface:1 get attr "img" --attr src
  ```

  ```bash Get Styles theme={null}
  cmux browser surface:1 get styles ".button" --property background-color
  ```

  ```bash Count Elements theme={null}
  cmux browser surface:1 get count "li.item"
  ```
</CodeGroup>

## browser is

Check element state.

```bash theme={null}
cmux browser <surface> is <subcommand> [--selector] <selector>
```

**Subcommands:**

* `visible` - Check if element is visible
* `enabled` - Check if element is enabled
* `checked` - Check if checkbox/radio is checked

**Examples:**

<CodeGroup>
  ```bash Is Visible theme={null}
  cmux browser surface:1 is visible ".modal"
  ```

  ```bash Is Enabled theme={null}
  cmux browser surface:1 is enabled "button#submit"
  ```

  ```bash Is Checked theme={null}
  cmux browser surface:1 is checked "input#terms"
  ```
</CodeGroup>

**Output:**

```
true
```

## browser find

Find elements using various locator strategies.

```bash theme={null}
cmux browser <surface> find <locator> [options]
```

**Locators:**

### find role

```bash theme={null}
cmux browser surface:1 find role <role> [--name <text>] [--exact]
```

**Example:**

```bash theme={null}
cmux browser surface:1 find role button --name "Submit"
```

### find text / label / placeholder / alt / title / testid

```bash theme={null}
cmux browser surface:1 find text <text> [--exact]
cmux browser surface:1 find label <text> [--exact]
cmux browser surface:1 find placeholder <text> [--exact]
```

**Example:**

```bash theme={null}
cmux browser surface:1 find text "Click here" --exact
```

### find first / last

```bash theme={null}
cmux browser surface:1 find first [--selector] <selector>
cmux browser surface:1 find last [--selector] <selector>
```

### find nth

```bash theme={null}
cmux browser surface:1 find nth [--index] <index> [--selector] <selector>
```

**Example:**

```bash theme={null}
cmux browser surface:1 find nth --index 2 --selector "li"
```

## browser frame

Switch to an iframe context.

```bash theme={null}
cmux browser <surface> frame <selector>
cmux browser <surface> frame main
```

**Examples:**

<CodeGroup>
  ```bash Select Frame theme={null}
  cmux browser surface:1 frame "iframe#content"
  ```

  ```bash Back to Main theme={null}
  cmux browser surface:1 frame main
  ```
</CodeGroup>

## browser dialog

Handle JavaScript dialogs (alert/confirm/prompt).

```bash theme={null}
cmux browser <surface> dialog accept [text]
cmux browser <surface> dialog dismiss
```

**Examples:**

<CodeGroup>
  ```bash Accept Alert theme={null}
  cmux browser surface:1 dialog accept
  ```

  ```bash Accept Prompt with Input theme={null}
  cmux browser surface:1 dialog accept "My input text"
  ```

  ```bash Dismiss theme={null}
  cmux browser surface:1 dialog dismiss
  ```
</CodeGroup>

## browser download

Wait for a download to complete.

```bash theme={null}
cmux browser <surface> download [wait] [--path <path>] [--timeout <seconds>] [--timeout-ms <ms>]
```

**Flags:**

<ParamField path="--path" type="string">
  Expected download path
</ParamField>

<ParamField path="--timeout" type="number">
  Timeout in seconds
</ParamField>

<ParamField path="--timeout-ms" type="number">
  Timeout in milliseconds
</ParamField>

**Example:**

```bash theme={null}
cmux browser surface:1 download --path ~/Downloads/file.pdf --timeout 30
```

## browser cookies

Manage browser cookies.

```bash theme={null}
cmux browser <surface> cookies <get|set|clear> [options]
```

### cookies get

```bash theme={null}
cmux browser surface:1 cookies get [--name <name>] [--all]
```

### cookies set

```bash theme={null}
cmux browser surface:1 cookies set <name> <value> [--domain <domain>] [--path <path>] [--secure] [--expires <timestamp>]
```

### cookies clear

```bash theme={null}
cmux browser surface:1 cookies clear [--name <name>]
```

**Examples:**

<CodeGroup>
  ```bash Get All Cookies theme={null}
  cmux browser surface:1 cookies get --all
  ```

  ```bash Set Cookie theme={null}
  cmux browser surface:1 cookies set session_id abc123 --domain example.com
  ```

  ```bash Clear Cookie theme={null}
  cmux browser surface:1 cookies clear --name session_id
  ```
</CodeGroup>

## browser storage

Manage localStorage and sessionStorage.

```bash theme={null}
cmux browser <surface> storage <local|session> <get|set|clear> [key] [value]
```

**Examples:**

<CodeGroup>
  ```bash Get Item theme={null}
  cmux browser surface:1 storage local get myKey
  ```

  ```bash Set Item theme={null}
  cmux browser surface:1 storage local set myKey myValue
  ```

  ```bash Clear Storage theme={null}
  cmux browser surface:1 storage session clear
  ```
</CodeGroup>

## browser tab

Manage browser tabs (within the surface).

```bash theme={null}
cmux browser <surface> tab <list|new|switch|close> [options]
```

### tab list

```bash theme={null}
cmux browser surface:1 tab list
```

### tab new

```bash theme={null}
cmux browser surface:1 tab new [url]
```

### tab switch

```bash theme={null}
cmux browser surface:1 tab switch <index|surface_id>
```

### tab close

```bash theme={null}
cmux browser surface:1 tab close <index|surface_id>
```

## browser console

View or clear console messages.

```bash theme={null}
cmux browser <surface> console [list|clear]
```

**Examples:**

<CodeGroup>
  ```bash List Console theme={null}
  cmux browser surface:1 console list
  ```

  ```bash Clear Console theme={null}
  cmux browser surface:1 console clear
  ```
</CodeGroup>

**Output:**

```
[log] Page loaded
[warn] Deprecated API used
[error] Failed to fetch resource
```

## browser errors

View or clear page errors.

```bash theme={null}
cmux browser <surface> errors [list|clear]
```

**Example:**

```bash theme={null}
cmux browser surface:1 errors list
```

## browser highlight

Highlight an element on the page (visual debugging).

```bash theme={null}
cmux browser <surface> highlight [--selector] <selector>
```

**Example:**

```bash theme={null}
cmux browser surface:1 highlight ".submit-button"
```

## browser state

Save or load browser state.

```bash theme={null}
cmux browser <surface> state save <path>
cmux browser <surface> state load <path>
```

**Examples:**

<CodeGroup>
  ```bash Save State theme={null}
  cmux browser surface:1 state save ~/browser-state.json
  ```

  ```bash Load State theme={null}
  cmux browser surface:1 state load ~/browser-state.json
  ```
</CodeGroup>

## browser addinitscript / addscript / addstyle

Inject scripts or styles into pages.

```bash theme={null}
cmux browser <surface> addinitscript [--script <js>] <javascript>
cmux browser <surface> addscript [--script <js>] <javascript>
cmux browser <surface> addstyle [--css <css>] <css>
```

**Examples:**

<CodeGroup>
  ```bash Add Init Script theme={null}
  cmux browser surface:1 addinitscript "console.log('Page loading')"
  ```

  ```bash Add Style theme={null}
  cmux browser surface:1 addstyle "body { background: #f0f0f0; }"
  ```
</CodeGroup>

## browser viewport

Set the viewport size.

```bash theme={null}
cmux browser <surface> viewport <width> <height>
```

**Example:**

```bash theme={null}
cmux browser surface:1 viewport 1920 1080
```

## browser geolocation / geo

Set geolocation coordinates.

```bash theme={null}
cmux browser <surface> geolocation <latitude> <longitude>
```

**Example:**

```bash theme={null}
cmux browser surface:1 geolocation 37.7749 -122.4194
```

## browser offline

Simulate offline mode.

```bash theme={null}
cmux browser <surface> offline <true|false>
```

**Example:**

```bash theme={null}
cmux browser surface:1 offline true
```

## browser trace

Start or stop tracing.

```bash theme={null}
cmux browser <surface> trace start [path]
cmux browser <surface> trace stop [path]
```

**Example:**

```bash theme={null}
cmux browser surface:1 trace start ~/trace.json
cmux browser surface:1 trace stop
```

## browser network

Manage network routes and requests.

```bash theme={null}
cmux browser <surface> network route <pattern> [--abort] [--body <text>]
cmux browser <surface> network unroute <pattern>
cmux browser <surface> network requests
```

**Examples:**

<CodeGroup>
  ```bash Route Pattern theme={null}
  cmux browser surface:1 network route "**/api/**" --body '{"mocked":true}'
  ```

  ```bash Abort Pattern theme={null}
  cmux browser surface:1 network route "**/analytics/**" --abort
  ```

  ```bash Unroute theme={null}
  cmux browser surface:1 network unroute "**/api/**"
  ```

  ```bash List Requests theme={null}
  cmux browser surface:1 network requests
  ```
</CodeGroup>

## browser screencast

Start or stop video recording.

```bash theme={null}
cmux browser <surface> screencast start
cmux browser <surface> screencast stop
```

## browser input

Low-level input simulation.

```bash theme={null}
cmux browser <surface> input mouse [args]
cmux browser <surface> input keyboard [args]
cmux browser <surface> input touch [args]
```

## Legacy Aliases

These legacy commands are aliased to the `browser` namespace:

* `open-browser` → `browser open`
* `navigate` → `browser navigate`
* `browser-back` → `browser back`
* `browser-forward` → `browser forward`
* `browser-reload` → `browser reload`
* `get-url` → `browser get-url`
* `focus-webview` → `browser focus-webview`
* `is-webview-focused` → `browser is-webview-focused`

**Note:** `--panel` is accepted as a legacy alias for `--surface` in all browser commands.

## Global Flags

<ParamField path="--socket" type="string">
  Path to the cmux socket (default: `/tmp/cmux.sock` or `$CMUX_SOCKET_PATH`)
</ParamField>

<ParamField path="--password" type="string">
  Socket authentication password (or use `$CMUX_SOCKET_PASSWORD`)
</ParamField>

<ParamField path="--json" type="boolean">
  Output results in JSON format
</ParamField>

<ParamField path="--id-format" type="string">
  Control ID output format: `refs`, `uuids`, or `both`
</ParamField>
