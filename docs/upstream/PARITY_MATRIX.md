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

This matrix distinguishes the current nested stopgap from the proposed native adapter. “Native” means provider-owned virtual descendants under a cmux host terminal, not conversion into Bonsplit panes.

## Topology and presentation

| Capability | Herdr source/API | cmux today | Plugin stopgap | Native target | First release |
|---|---|---|---|---|---|
| Provider health/version | `ping` → version, protocol, capabilities | No Herdr awareness | CLI health check | Direct handshake with compatibility result | Yes |
| Complete initial topology | `session.snapshot` | Outer surface only | Merge CLI list output | Atomic snapshot: workspace → tab → pane → agent | Yes |
| Incremental topology | `events.subscribe` | None | Poll every N seconds | Long-lived event stream plus resync | Yes |
| Workspace rows | snapshot/workspace events | cmux workspace only | Not representable | Virtual nested workspace rows | Yes |
| Tab rows/order | snapshot/tab events | Surface tabs are cmux-owned | Text-only tree | Virtual provider tab rows preserving order | Yes |
| Pane rows/layout | panes + `layouts` | Bonsplit cmux panes | Text-only tree | Virtual pane rows; optional layout hints | Rows yes; graphical layout later |
| Agent rows | `agents`, pane agent fields | Agent detection tied to cmux surface | Status pills | Agent child/decoration on provider pane | Yes |
| Focus | focused IDs/events | Outer focus only | Separate CLI helper | Inner focus reflected and forwarded | Yes |
| Titles/labels | workspace/tab/pane/agent fields | Native titles only | Flattened pill text | Provider labels with clear provenance | Yes |
| Working/idle/blocked/done | `AgentStatus`, status events | cmux agent/status facilities | Color-coded pills | Native status decoration retaining raw value | Yes |
| Metadata/state labels | pane/agent metadata | Not nested | Mostly dropped | Bounded typed metadata for UI | Selected fields only |
| Read output | `pane.read`, `agent.read` | Native surfaces only | CLI command | Provider action/read API | Later |
| Unread/attention synthesis | events/status | Outer workspace semantics | Approximate | Explicit policy; never silently clear outer state | Later |

## Actions

| Action | Herdr method | Native policy | First release |
|---|---|---|---|
| Focus workspace/tab/pane/agent | `*.focus` / `agent.focus` | Allowed after current-binding check | Yes |
| Rename workspace/tab/pane/agent | `*.rename` | User-initiated only; capability-gated | Yes |
| Send text/keys/input | `pane.send_*`, agent methods | Never derived from display text; explicit target | Later/guarded |
| Prompt/wait agent | `agent.prompt`, `agent.wait` | Explicit UI/API with timeout limits | Later |
| Split pane | `pane.split` | Provider-owned split; result refreshes snapshot | Later |
| Move/swap/resize/layout | pane/layout methods | Provider-owned; not Bonsplit mutation | Later |
| Close inner node | `*.close` | Confirmation; cannot close host implicitly | Later |
| Stop server | `server.stop` | Excluded from nested adapter | No |
| Reload server/config/integrations/plugins | server/integration/plugin methods | Excluded; outside topology scope | No |

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

## Definition of native parity for v1

Native v1 is complete when health, initial tree, event-driven updates, focus, labels, agent status, collision-free IDs, safe disconnect/reconnect, additive control-socket reads, and restore revalidation work. Full parity does **not** require every Herdr mutation, graphical reproduction of its split layout, or ownership of its PTYs.
