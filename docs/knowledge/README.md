# Herdr + cmux knowledge corpus

Captured **2026-08-19** for this plugin (`cmux-herdr`). Official Herdr and cmux
documentation, websites, GitHub docs/skills, Mintlify pages, DeepWiki trees, and
CLI/socket inventories live here so agents do not have to re-scrape the web.

## Start here

| File | What it is |
|---|---|
| [HERDR.md](HERDR.md) | Synthesized Herdr model, CLI, socket API, agents, plugins, restore |
| [CMUX.md](CMUX.md) | Synthesized cmux model, CLI, socket API, sidebars, remote tmux, skills |
| [SYNTHESIS.md](SYNTHESIS.md) | How the two products nest, conflict, and map onto this plugin |
| [INVENTORIES.md](INVENTORIES.md) | Exhaustive command/method/config-key lists extracted from sources |
| [SOURCES.md](SOURCES.md) | Every URL and GitHub path that was fetched, plus tool failures |
| [raw/](raw/) | Verbatim scrapes (MDX, markdown, JSON, DeepWiki extracts, indexes) |

## What was read

- **Herdr official docs** (stable `v0.8.0` + `master`): every English page under
  `docs/next/website/src/content/docs/`, plus `llms.txt` / `llms-full.txt`,
  `agent-guide.md`, `skills/herdr/SKILL.md`, `config-reference.json`, README,
  AGENTS.md, CHANGELOG, CONTRIBUTING.
- **cmux official site**: all 115 `.md` variants listed in `https://cmux.com/llms.txt`
  (docs, blog, compare, agents, legal, iOS, etc.).
- **cmux GitHub**: `docs/cli-contract.md` and related design docs, plus public
  skills (`cmux`, `cmux-custom-sidebar`, `cmux-workspace`, `cmux-browser`,
  `cmux-socket-policy`, `cmux-keyboard-shortcuts`).
- **Mintlify mirror**: `https://mintlify.com/manaflow-ai/cmux/llms.txt` and the
  automation/CLI/socket pages (older than `cmux.com/docs`, kept for gaps).
- **DeepWiki**: full page trees for `herdrdev/herdr` (48 pages) and
  `manaflow-ai/cmux` (81 pages), plus extracted Next.js flight text.

Japanese / Chinese Herdr translations were listed but not duplicated; English
is the canonical control-surface language.

## How to use this in `cmux-herdr`

Prefer `HERDR.md` + `CMUX.md` + `SYNTHESIS.md` for design work. When a flag,
method name, or status enum must be exact, open the matching file under `raw/`
or the inventories. Do not invent Herdr keybindings, config keys, or cmux socket
methods — the inventories and official pages are the authority.
