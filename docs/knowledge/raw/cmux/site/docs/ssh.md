# [#](#title)SSH

# [#](https://cmux-docs-release.vercel.app/docs/ssh#title)SSH

cmux ssh creates a workspace for a remote machine. Browser panes route through the remote network, files drag-and-drop via scp, coding agents send notifications to your local sidebar, and sessions reconnect on drops.

## [#](https://cmux-docs-release.vercel.app/docs/ssh#usage)Usage

```
cmux ssh user@remote
cmux ssh user@remote --name "dev server"
cmux ssh user@remote --command 'omp "investigate auth"'
cmux ssh user@remote -p 2222
cmux ssh user@remote -i ~/.ssh/id_ed25519
cmux ssh user@remote --transport mosh
cmux mosh user@remote
cmux mosh-tmux user@remote
cmux mosh-tmux user@remote --session agent-main
```

cmux ssh reads your ~/.ssh/config for host aliases, identity files, and proxy settings. All flags mirror their ssh equivalents.

## [#](https://cmux-docs-release.vercel.app/docs/ssh#flags-title)Flags

| Flag | Description |
| --- | --- |
| `--name` | Set the workspace title |
| `--command` | Run command text once in the initial remote terminal after shell startup |
| `-p, --port` | SSH port (default 22) |
| `-i, --identity` | Path to identity file |
| `-o, --ssh-option` | Pass arbitrary SSH options (e.g. -o StrictHostKeyChecking=no) |
| `--transport` | Interactive terminal transport: ssh (default) or mosh |
| `--no-focus` | Create the workspace without switching to it |

## [#](https://cmux-docs-release.vercel.app/docs/ssh#mosh-transport)Mosh terminal transport

Use cmux ssh user@remote --transport mosh when the interactive terminal should survive Wi-Fi, VPN, hotspot, and sleep/wake network changes. Mosh carries only the terminal PTY; SSH remains the management lane for remote metadata, daemon bootstrap and control, browser proxying, uploads, and reconnect actions.

cmux requires Mosh 1.4 or later locally and checks for remote mosh-server before launch. If the client is missing or incompatible, the server is missing, or the remote probe fails, cmux explains the reason and continues over SSH.

Create a Mosh workspace that creates or attaches to a named tmux session (default: main). The tmux profile persists across reconnect and app session restore. This is a roaming tmux terminal, not the native mirror from cmux ssh-tmux. --session <name> selects the session; all other flags match cmux mosh. Without Mosh, the same profile runs over SSH.

## [#](https://cmux-docs-release.vercel.app/docs/ssh#ssh-deep-links)SSH deep links

Use cmux SSH deep links when a website or tool wants to offer an Open in cmux button. The link opens cmux, shows a confirmation prompt, then runs the equivalent cmux ssh command after the user confirms.

```
cmux://ssh?host=dev.example.com
cmux://ssh?host=dev.example.com&user=alice&port=2222&title=GPU%20box
cmux://ssh?host=workspace123.vm-ssh.freestyle.sh&user=workspace123%2Csession-token
cmux://ssh?host=dev.example.com&host-key-policy=accept-new&no-focus=true
```

Use the cmux.com fallback URL for website buttons. It opens the native link and shows the download if cmux is not installed.

```
https://cmux.com/deeplink/ssh?host=workspace123.vm-ssh.freestyle.sh&user=workspace123%2Csession-token&title=Freestyle
```

Prompt and rules buttons use the same fallback shape. Commas, colons, and literal plus signs are preserved when they are URL encoded:

```
https://cmux.com/deeplink/prompt?text=Review%20this%20branch
https://cmux.com/deeplink/rules?name=freestyle&text=Prefer%20commas,%20colons:%20and%20small%20PRs
```

Use the SVG icon for dashboard buttons, or the PNG logo when a raster image is required:

```
https://cmux.com/cmux-icon.svg
https://cmux.com/logo.png
```

Build the fallback URL with URLSearchParams so titles, host aliases, and user names are encoded safely:

```
const params = new URLSearchParams({
  host: "workspace123.vm-ssh.freestyle.sh",
  user: "workspace123,session-token",
  title: "Freestyle",
});

const href = "https://cmux.com/deeplink/ssh?" + params.toString();
```

| Parameter | Meaning |
| --- | --- |
| `host` | Required SSH host or ~/.ssh/config alias. |
| `user` | Optional SSH user. cmux combines it with host as user@host. |
| `port` | Optional SSH port, 1 through 65535. |
| `title` / `name` | Optional workspace title. Use only one of title or name. |
| `connect-timeout` | Optional ConnectTimeout value in seconds, 1 through 600. |
| `server-alive-interval` | Optional ServerAliveInterval value in seconds, 1 through 3600. |
| `server-alive-count-max` | Optional ServerAliveCountMax value, 1 through 100. |
| `host-key-policy` | Optional StrictHostKeyChecking policy: accept-new, ask, strict, or yes. |
| `no-focus` | Optional boolean. true creates the workspace without switching to it. |

Use cmux:// for the stable app, cmux-nightly:// for Nightly, and cmux-dev:// for Debug or tagged dev builds.

External links cannot pass identity files, raw ssh options, commands, ProxyCommand, or forwarding rules. Put keys, ProxyJump, HostName, and advanced options in ~/.ssh/config instead. cmux displays the command preview and requires the user to trust the SSH target before connecting.

## [#](https://cmux-docs-release.vercel.app/docs/ssh#browser-title)Browser panes

Browser panes in a remote workspace route all HTTP and WebSocket traffic through the remote machine's network. Type localhost:3000 and you're looking at the dev server running on the remote box. No -L flags, no manual port forwarding. Each remote workspace gets an isolated cookie store so sessions are scoped per-connection.

## [#](https://cmux-docs-release.vercel.app/docs/ssh#drag-drop-title)Drag and drop

Drag an image or file into a remote terminal and cmux uploads it via scp through the existing SSH connection. cmux detects the foreground SSH process by TTY and routes the upload through ControlMaster multiplexing.

## [#](https://cmux-docs-release.vercel.app/docs/ssh#notifications-title)Notifications

Processes on the remote machine can run cmux commands that execute on your local instance. When a coding agent calls cmux notify on the remote box, the notification appears in your local sidebar. The blue ring lights up on the workspace tab. Cmd+Shift+U jumps to it. Notification spam from flaky connections is suppressed with a per-host cooldown.

## [#](https://cmux-docs-release.vercel.app/docs/ssh#agents-title)Coding agents over SSH

cmux claude-teams and cmux omo both work inside SSH sessions. The Go relay daemon on the remote host handles the same tmux-compat translation that the local Swift CLI does. Teammate agents spawn as native cmux splits on your local machine while computation runs on the remote box.

```
# Inside an SSH session:
cmux claude-teams
cmux omo
```

## [#](https://cmux-docs-release.vercel.app/docs/ssh#reconnect-title)Reconnect

When the connection drops, cmux reconnects with exponential backoff (3s, 6s, 12s, up to 60s). The remote session persists and cmux reattaches on reconnect, resizing with smallest-screen-wins semantics. Default keepalive options (ServerAliveInterval=20, ServerAliveCountMax=2) are injected unless your config already sets them.

## [#](https://cmux-docs-release.vercel.app/docs/ssh#daemon-title)Relay daemon

On first connect, cmux probes the remote host (uname -s, uname -m) and uploads a versioned cmuxd-remote binary. The binary speaks JSON-RPC over stdio and handles three things:

| Feature | How |
| --- | --- |
| Browser traffic proxying | SOCKS5 and HTTP CONNECT over the daemon's stdio channel |
| CLI relay | Reverse TCP tunnel with HMAC-SHA256 auth so remote processes can call cmux commands locally |
| Session management | Persists sessions across reconnects, coordinates PTY resize across multiple attachments |

The daemon binary is stored at ~/.cmux/bin/cmuxd-remote/<version>/<os>-<arch>/cmuxd-remote on the remote host and verified against a SHA-256 manifest embedded in the app.

[Notifications](https://cmux-docs-release.vercel.app/docs/notifications) [iOS App](https://cmux-docs-release.vercel.app/docs/ios)

Canonical: https://cmux-docs-release.vercel.app/docs/ssh
