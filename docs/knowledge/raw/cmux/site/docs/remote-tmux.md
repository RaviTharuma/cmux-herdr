# [#](#title)Remote tmux

# [#](https://cmux-docs-release.vercel.app/docs/remote-tmux#title)Remote tmux

Remote tmux mirrors a tmux session running on a remote machine into cmux over SSH, using tmux control mode (tmux -CC). Instead of a plain SSH terminal, the remote session is reprojected into cmux's native UI — workspaces, tabs, splits, scrollback, and copy — and cmux drives the real tmux server behind it. It is a Beta Features flag and does not change local terminals.

Beta. This feature is opt-in and still stabilizing — see the limitations below.

For network roaming, use cmux mosh-tmux. It opens a first-class Mosh terminal attached to a named tmux session; it does not replace the native workspace/tab/split mirror provided by SSH tmux control mode.

## [#](https://cmux-docs-release.vercel.app/docs/remote-tmux#mapping)How tmux maps to cmux

cmux and tmux nest their layouts in opposite directions. In cmux, a pane holds a row of tabs (each tab is one terminal). In tmux, a window holds a split of panes. Remote tmux bridges the two by projecting tmux's structure onto cmux's:

| tmux | cmux |
| --- | --- |
| `session` | A dedicated workspace in the sidebar. |
| `window` | A tab in that workspace. |
| `pane` | A pane in a native split inside that tab. |

The new capability is panes inside a tab: a cmux tab used to hold a single terminal, but a tmux window with several panes is now rendered as a real cmux split layout within one tab — each remote pane is a native cmux terminal pane with the usual chrome. The bridge is two-way: splitting or closing a pane in that tab runs tmux split-window, and reordering the tabs reorders the tmux windows with swap-window.

## [#](https://cmux-docs-release.vercel.app/docs/remote-tmux#requirements)Requirements

A reachable SSH host (cmux reads your ~/.ssh/config) with tmux 3.2 or newer installed. Control mode (tmux -CC) is a standard tmux feature, so no special build is needed; cmux checks the remote tmux version before attaching and fails clearly on older servers. cmux attaches over an SSH ControlMaster connection, and the remote tmux server keeps running when you detach.

## [#](https://cmux-docs-release.vercel.app/docs/remote-tmux#enable)Enable it

Open Settings → Beta Features and turn on Remote tmux. The toggle is off by default, so nothing changes for local terminals until you opt in.

## [#](https://cmux-docs-release.vercel.app/docs/remote-tmux#attach)Attaching

Run cmux ssh-tmux <destination> in a terminal — an ~/.ssh/config alias or user@host. cmux mirrors that host's tmux sessions into the current window's sidebar: each session becomes a workspace, each window a tab, and multi-pane windows become in-tab splits.

Pass --new-window to open the mirror in a dedicated new window instead of the current window.

Hosts that authenticate non-interactively (ssh-agent, or a key in ~/.ssh/config) attach with no prompt. If the host needs interactive authentication (a password, host-key confirmation, or MFA), cmux runs ssh inline in your terminal so you can authenticate, then mirrors into the current window. Accepts --port, --identity, and --no-focus.

```
cmux ssh-tmux dev@example.com
cmux ssh-tmux my-ssh-alias --port 2222 --identity ~/.ssh/id_ed25519
cmux ssh-tmux dev@example.com --new-window
```

The remote.tmux.\* socket commands (below) give finer control. remote.tmux.mirror is the mirror entry point; it accepts window/caller routing and can optionally select the mirrored workspace.

### [#](https://cmux-docs-release.vercel.app/docs/remote-tmux#permission-denied)If you get "Permission denied"

The destination is handed to ssh as-is, so an alias must carry everything ssh needs to log in — most commonly User (without it, ssh tries your local username) and IdentityFile:

```
Host my-ssh-alias
    HostName 203.0.113.10
    User dev
    IdentityFile ~/.ssh/id_ed25519
```

Or pass the user explicitly: cmux ssh-tmux dev@my-ssh-alias. If plain ssh <destination> doesn't log you in, cmux ssh-tmux <destination> won't either.

## [#](https://cmux-docs-release.vercel.app/docs/remote-tmux#how-it-works)How it works

cmux spawns ssh … tmux -CC attach and parses the control-mode stream itself rather than relying on a built-in tmux viewer, so the protocol and %begin/%end command correlation are fully owned by cmux. Each remote pane renders into a dedicated terminal surface fed by tmux %output, and your input — keystrokes and mouse — is forwarded with tmux send-keys. The remote tmux server owns pane sizing and reflow; cmux keeps its surfaces in lock-step with it and never reflows locally.

## [#](https://cmux-docs-release.vercel.app/docs/remote-tmux#behavior)What it supports

-   Sizing: the remote client is resized to the rendered grid (refresh-client -C), so TUIs aren't stuck at tmux's default 80×24.
-   Splits: splitting or closing a pane in a mirrored window is propagated to tmux with split-window, so the cmux layout and the tmux layout stay in sync. Programmatic splits (cmux new-split or surface.split over the socket) report accepted with no surface id — the new pane arrives asynchronously once tmux confirms the layout change. Requests carrying options the routed split cannot honor (startup command, working directory, divider position, left/up placement) are rejected up front, before the remote session is mutated.
-   Reordering: drag-reordering the mirrored tabs reorders the tmux windows with swap-window.
-   Working directory: the remote pane's current folder is tracked and shown on the tab.
-   Paste & drop: pasted text and dropped images/files go through tmux paste-buffer -p, so a remote app (e.g. a coding agent) receives a real bracketed paste — an image arrives as \[Image #N\], not a local path.
-   Mouse: clicks, scroll, and drag reach the remote app when it has mouse mode on. Hold Shift while dragging for a native cmux text selection/copy (the same as any terminal running a mouse-mode app).
-   Unicode-correct output: multi-byte characters are preserved even when tmux splits them across updates, so box-drawing and other wide content render cleanly.

## [#](https://cmux-docs-release.vercel.app/docs/remote-tmux#socket-commands)Socket commands

The feature is also driven by socket commands (gated on the Beta Features flag). host is an SSH destination or ~/.ssh/config alias; session is a tmux session name; mirror also accepts standard window/workspace/surface/pane routing.

| Method | Params | Description |
| --- | --- | --- |
| `remote.tmux.sessions` | `host`, `port?`, `identity_file?` | List the tmux sessions on a host. |
| `remote.tmux.attach` | `host`, `session`, `create?` | Attach a control client to a session (create attaches-or-creates). |
| `remote.tmux.mirror` | `host`, `port?`, `identity_file?`, `activate?` | Mirror every session on a host into the resolved window, each as its own workspace (windows become tabs); returns the ssh command to run for interactive auth when the host needs it. |
| `remote.tmux.detach` | `host`, `session` | Detach the control client; the remote session keeps running. |
| `remote.tmux.state` | `host`, `session` | Report the control client's observed state (diagnostics). |

A dash-prefixed host or identity file is rejected at the trust boundary as a defense against SSH option injection.

```
{ "method": "remote.tmux.mirror", "params": { "host": "dev.example.com" } }
```

## [#](https://cmux-docs-release.vercel.app/docs/remote-tmux#limitations)Limitations

-   Transient SSH drops reconnect automatically: if the connection blips mid-session the mirror freezes and cmux retries with capped exponential backoff, re-seeding the panes when it resumes. The mirror only closes for good when the remote session itself ends, or when you close the mirror window or quit cmux — and it isn't restored on relaunch (re-attach with cmux ssh-tmux).
-   Only single-line paste/drop (such as a file or image path) is delivered as a real bracketed paste via paste-buffer -p; multi-line text is sent as plain keystrokes, so a remote app won't treat it as one paste.
-   Live working-directory updates use control-mode subscriptions, which is why cmux requires tmux 3.2 or newer before it opens the mirror.
-   Primary-screen scrollback isn't re-wrapped on resize — cmux leaves reflow to tmux, so older lines can stay at the previous width until the app repaints. Full-screen TUIs, which redraw on resize, are unaffected.

[iOS App](https://cmux-docs-release.vercel.app/docs/ios) [Claude Code Teams](https://cmux-docs-release.vercel.app/docs/agent-integrations/claude-code-teams)

Canonical: https://cmux-docs-release.vercel.app/docs/remote-tmux
