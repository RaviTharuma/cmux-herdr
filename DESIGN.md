# Design

Herdr inside **native cmux chrome**: sidebar status pills, tab/pane mirroring, right-rail sessions/feed/dock. Design rationale: [docs/PLUGIN_DESIGN.md](docs/PLUGIN_DESIGN.md), [docs/RIGHT_RAIL.md](docs/RIGHT_RAIL.md).
- **Native first**: inherit the cmux sidebar chrome and the Ghostty/terminal theme (colours, fonts, spacing); no custom palette.
- Status is shown as compact pills; colour carries state only together with a text/glyph (never colour alone).
- Must stay legible in both light and dark cmux themes; test at narrow sidebar widths.
