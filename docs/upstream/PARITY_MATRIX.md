> **Canonical source of truth is GitHub**, not this draft.
>
> - Community poll: https://github.com/manaflow-ai/cmux/discussions/10106
> - Full nested topology: https://github.com/manaflow-ai/cmux/issues/8737
> - Nested topology v1: https://github.com/manaflow-ai/cmux/pull/10045
> - Hidden compat MVP: https://github.com/manaflow-ai/cmux/pull/8736
> - Update GitHub first when trackers move; then sync this file if still useful.
>
> This file is a paste-ready / design package kept for dual-path history. Prefer the live issue/PR over local text if they diverge.

# cmux ↔ Herdr native parity matrix

This matrix distinguishes three things: the plugin stopgap, native **sidebar** nested topology (#10045), and native **window mirror** (PR7 / ssh-tmux parity). Canonical tmux mapping: [TMUX_PARITY.md](./TMUX_PARITY.md).

## Topology and presentation

| Capability | Herdr source/API | cmux ssh-tmux | Plugin (`--tmux-parity`) | Native sidebar (#10045) | Native mirror (PR7) |
|---|---|---|---|---|---|
| Provider health/version | `ping` | handshake | CLI health | Direct handshake | Same connection |
| Complete initial topology | `session.snapshot` | tmux layout fetch | `pane/tab list` + layouts | Atomic snapshot | Snapshot → window mirrors |
| Incremental topology | `events.subscribe` | `%layout-change` / `%output` | poll + optional socket wait | Event stream + resync | Events + resync |
| Workspace rows | snapshot | session list | not a cmux workspace | Virtual nested rows | Host surface only |
| Tab rows/order | tab numbers | one cmux tab per tmux window | `move-tab` | Virtual rows | Real cmux tabs, Herdr order |
| Pane rows/layout | layouts + rects | Bonsplit from layout tree | layout-driven splits + `set-ratio` | Virtual rows; graphical later | Bonsplit from Herdr tree |
| Agent rows | `agents` | n/a | status pills | Agent child/decoration | Decoration on pane chrome |
| Focus | focused IDs | `%window-pane-changed` | `--focus` + reverse click | `nested.node.focus` | tmux-style project + send |
| Titles/labels | tab/pane fields | window title | tab-root title | Provider labels | Window title rule |
| Read output | `pane.read` | `%output` → surface | `attach-pane` poll | Later | Push into `TerminalPanel` |
| Send input | `pane.send_text` / `pane.send_keys` | `send-keys` | cbreak → `pane send-text` | Later/guarded | Ghostty → `pane.send_*` |
| Split pane | `pane.split` | `split-window` | `cmux split` from layout | Later | User split → provider |
| Resize/layout | `pane.resize` | `resize-pane` + claim | SIGWINCH + impose `set-ratio` | Later | `RemoteHerdrImpose` + divider drag |
| Close inner node | `*.close` | kill-pane | `--prune` | Later | Reconcile teardown |
| Zoom | pane zoom flag | base vs visible layout | keep mapped viewers | n/a | Same as tmux |

## Actions

| Action | Herdr method | Native sidebar (#10045) | Native mirror (PR7) |
|---|---|---|---|
| Focus workspace/tab/agent | `workspace.focus` / `tab.focus` / `agent.focus` | Yes | Yes (plus Bonsplit selection) |
| Focus neighboring pane | `pane.focus_direction` (no `pane.focus` method) | Yes | Yes |
| Rename | `*.rename` | User-initiated | Tab title from provider |
| Send text/keys | `pane.send_*` | Later/guarded | **Yes** |
| Split pane | `pane.split` | Later | **Yes** |
| Move/swap/resize/layout | pane/layout methods | Later | **Yes** |
| Close inner node | `*.close` | Later | **Yes** (confirm if busy) |
| Stop server | `server.stop` | Excluded | Excluded |

## Definition of native tmux-parity (PR7)

Native Herdr has **tmux parity** when each Herdr tab is a real cmux tab, each pane is a real Bonsplit + Ghostty surface, output/input/split/resize/focus/zoom/prune match `RemoteTmuxWindowMirror`, and the #10045 sidebar remains the session navigator.

Sidebar-only v1 (#10045) is **not** tmux parity. It is a prerequisite (socket, IDs, attach, restore).

## Lifecycle, identity, and API parity

| Concern | cmux native behavior required | Herdr fact / gap | Target |
|---|---|---|---|
| Parent binding | One attachment record owned by host stable surface identity | Environment identifies current Herdr pane but is not sufficient proof | Explicit binding; no heuristic-only permanent attachment |
| Node identity | Compound provider identity, node kind, raw node ID | IDs are opaque strings and may repeat across server lifetimes | Versioned `NestedNodeID` |
| Provider instance identity | Detect path reuse/restart | `ping` has version/protocol but no documented server UUID | cmux connection generation initially; request upstream server instance UUID for durable auto-reattach |
| Capability negotiation | Expose read/actions independently | protocol 17 has two server feature booleans, not method inventory | Tested protocol table; unknown versions snapshot-only or rejected |
| Resync | Snapshot after stream gaps or malformed ordering | Event stream has no documented resumable cursor | Mark stale, reconnect, fetch full snapshot |
| Restore | Persist attachment intent, not live inner nodes | Herdr owns session/process persistence | Revalidate then fresh snapshot |
| Multiple providers | Isolated trees/actions | Raw IDs can overlap | Attachment-scoped stores |
| Public tree | Optional nested descendants with parent reference | N/A | Additive `system.tree` option and capability |
| Public actions | Structured nested target only | N/A | No delimiter-parsed target strings |
| Mobile/remote cmux | Do not imply availability | Provider socket is local to Mac host | Omit or read-only projection in first release |

## Security parity

| Threat | Existing relevant behavior | Required nested-provider behavior |
|---|---|---|
| Unauthorized local socket access | Herdr API socket is created mode `0600` | Verify socket type, owner UID, restrictive mode, and path identity before/after connect |
| Untrusted terminal environment | cmux already distinguishes socket authorization/capability paths | Descriptor is a hint, not authority; require user opt-in and scoped binding |
| Command injection | Plugin shells out to CLIs | Native adapter uses a Unix socket and typed JSON; no shell |
| Resource exhaustion | cmux control socket has preauthorization/read policies | Bound line bytes, snapshot bytes, node counts, depth, metadata, event rates, and reconnects |
| Confused deputy | cmux socket uses scoped capability concepts | Every action resolves host surface + provider generation + typed node ID atomically |
| Stale endpoint/path reuse | Socket path alone is mutable | Pin file identity during connection; require provider instance identity for unattended restore |
| Malicious display fields | Existing UI receives external titles/output | Sanitize/control-character strip, truncate, and never auto-open cwd/URL/metadata |
| Destructive lifecycle coupling | Closing native pane normally owns its surface | Detach on host close; do not stop provider or close children implicitly |

## Degradation rules

1. **No socket / refused connection:** show the normal terminal; nested tree absent or disconnected.
2. **Unsafe ownership or mode:** reject attachment and explain the local security check.
3. **Unknown protocol:** do not guess method shapes. Offer normal terminal and plugin fallback; read-only mode only if a tested compatibility range permits it.
4. **Missing action capability:** omit/disable that action; never emulate it with keystrokes.
5. **Malformed or oversized snapshot/event:** disconnect adapter, retain last state as visibly stale for a short bounded period, then remove it.
6. **Event gap/reconnect:** discard incremental assumptions and fetch `session.snapshot`.
7. **Restore without provider identity proof:** leave disconnected and require manual reattach.

## Definition of native parity

- **Sidebar v1 (#10045):** health, tree, events, focus, labels, agent status, compound IDs, restore revalidation. Complete as a navigator; **not** ssh-tmux.
- **Mirror PR7:** Bonsplit + Ghostty ownership of Herdr panes, layout imposition, input/output, split/resize/zoom/prune — copy `RemoteTmuxWindowMirror`. This is tmux parity.
- Plugin `--tmux-parity` is the userspace stand-in until PR7 lands.
