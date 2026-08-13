# Concept map: cmux ↔ herdr ↔ tmux

| Concept | cmux (outer) | herdr (inner) | tmux (legacy analogy) |
|---------|--------------|---------------|------------------------|
| App / server | `cmux.app` process | `herdr` server | `tmux` server |
| Socket | `$CMUX_SOCKET_PATH` | `$HERDR_SOCKET_PATH` | `$TMUX` / server socket |
| Top container | window (`window:N`) | (session) | session |
| Mid container | workspace (`workspace:N`, UUID) | workspace (`w2`) | (often session or window group) |
| Tab | tab / surface row (`tab:N`, UUID) | tab (`w2:t11`) | window |
| Split unit | pane (`pane:N`, UUID) | pane (`w2:p2B`) | pane |
| Terminal leaf | surface (`surface:N`, terminal) | terminal in pane (`terminal_id`) | pane tty |
| Agent | (detection / shims; not first-class tree) | `agent` + `agent_status` on pane | (none native) |
| Focus env | `$CMUX_SURFACE_ID`, `$CMUX_WORKSPACE_ID`, `$CMUX_TAB_ID` | `$HERDR_PANE_ID`, `$HERDR_TAB_ID`, `$HERDR_WORKSPACE_ID` | `$TMUX_PANE` |
| Nested flag | bundle id / socket present | `$HERDR_ENV=1` | — |
| Status to UI | `cmux set-status` / `set-progress` | `agent_status` field; `pane report-agent` | — |
| CLI | `cmux` | `herdr` | `tmux` |
| Bridge keys | `herdr:<pane_id>` status pills | pane_id source | — |

## Env vars (nested case)

```
# herdr (inner)
HERDR_ENV=1
HERDR_PANE_ID=w2:p34
HERDR_TAB_ID=w2:t17
HERDR_WORKSPACE_ID=w2
HERDR_SOCKET_PATH=~/.config/herdr/herdr.sock

# cmux (outer)
CMUX_SURFACE_ID=<uuid>
CMUX_TAB_ID=<uuid>
CMUX_WORKSPACE_ID=<uuid>   # may be stale when nested — prefer cmux-herdr resolve
CMUX_SOCKET_PATH=~/.local/state/cmux/cmux-501.sock
CMUX_BUNDLE_ID=com.cmuxterm.app
```

## Command cheat sheet

| Intent | Prefer |
|--------|--------|
| Outer tree | `cmux tree` |
| Inner tree | `cmux-herdr tree` / `herdr pane list` |
| Diagnose install | `cmux-herdr doctor` |
| Mirror agents to sidebar | `cmux-herdr sync` / `watch` |
| Mirror tabs/panes into cmux | `cmux-herdr mirror` / `watch --mirror` |
| Follow one inner pane | `cmux-herdr attach-pane <pane_id>` |
| Split agent | `herdr pane split --current` / `cmux-herdr split` |
| Focus inner workspace | `herdr workspace focus` / `cmux-herdr focus-workspace` |
| Focus inner tab | `herdr tab focus` / `cmux-herdr focus-tab` |
| Focus agent pane | `herdr agent focus` / `cmux-herdr focus-pane` / `focus-agent` |
| Read pane / agent output | `cmux-herdr read-pane` / `read-agent` |
| Outer workspace select | `cmux` workspace APIs / custom sidebar button |
