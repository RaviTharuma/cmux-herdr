# Architecture

Root summary — the detailed design lives in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), [docs/PLUGIN_DESIGN.md](docs/PLUGIN_DESIGN.md) and [docs/RIGHT_RAIL.md](docs/RIGHT_RAIL.md).

```
Herdr (sessions, panes, agents) ◀──api.rs / bridge.rs──▶ cmux-herdr engine
                                                          ├─ mirror.rs / layout.rs   tab & pane mirroring
                                                          ├─ live.rs / pump.rs       live status → cmux pills
                                                          ├─ handoff.rs / impose.rs  attach & ownership
                                                          └─ host.rs / control.rs    cmux host contract
                                                                 ▼
                                 cmux sidebar + right rail (sidebars/herdr.swift|.js): sessions · feed · dock
```

Packaging: `bin/` launchers, `scripts/install*.sh`, optional LaunchAgent watch service, `agent-skill/` for agents.
