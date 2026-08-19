# Sources and scrape log

Captured 2026-08-19. Tool status is recorded so later agents know what was
actually fetched versus what was unavailable.

## Tool status

| Tool | Result |
|---|---|
| GitHub MCP | Used. Listed repos/dirs; `search_code` for Herdr CLI sources. |
| `curl` + official `llms.txt` indexes | Primary scrape path. |
| WebFetch / WebSearch | Used for docs homepages, DeepWiki overviews, Mintlify pages. |
| Context7 MCP | **Quota exceeded** (`Monthly quota exceeded`). No library IDs resolved. |
| Exa MCP | **Free rate limit**. Not used after the first failed calls. |
| Tavily MCP | **needsAuth**. Not used. |
| Firecrawl CLI | **Not installed** in this VM; npm global install skipped after Exa/Context7 were already down. Official `llms.txt` + GitHub raw files covered the same docs. |
| DeepWiki | HTML is a Next.js app. Page trees extracted from `/herdrdev/herdr/N-…` and `/manaflow-ai/cmux/N-…` links; readable text extracted from `self.__next_f` flight payloads into `raw/deepwiki/*-extracted.md`. Indexed dates: Herdr 2026-08-09, cmux 2026-07-14. |

## Canonical URLs

### Herdr

- Site: https://herdr.dev/
- Docs hub: https://herdr.dev/docs/
- Agent index: https://herdr.dev/llms.txt
- Full concatenated docs: https://herdr.dev/llms-full.txt
- Small / preview indexes: https://herdr.dev/llms-small.txt , https://herdr.dev/llms-preview.txt
- Agent onboarding: https://herdr.dev/agent-guide.md
- Installers: https://herdr.dev/install.sh , https://herdr.dev/install.ps1
- GitHub: https://github.com/herdrdev/herdr (also redirected from `ogulcancelik/herdr`)
- Skill: https://raw.githubusercontent.com/herdrdev/herdr/master/skills/herdr/SKILL.md
- DeepWiki: https://deepwiki.com/herdrdev/herdr
- Marketplace / awesome list (ecosystem, not core): https://github.com/yigitkonur/awesome-herdr

Official Herdr docs pages (HTML; source MDX saved under `raw/herdr/docs-master/`):

- https://herdr.dev/docs/quick-start/
- https://herdr.dev/docs/install/
- https://herdr.dev/docs/concepts/
- https://herdr.dev/docs/keyboard/
- https://herdr.dev/docs/agents/
- https://herdr.dev/docs/agent-automation/
- https://herdr.dev/docs/agent-skill/
- https://herdr.dev/docs/cli-reference/
- https://herdr.dev/docs/socket-api/
- https://herdr.dev/docs/configuration/
- https://herdr.dev/docs/config-reference/
- https://herdr.dev/docs/session-state/
- https://herdr.dev/docs/how-to-work/
- https://herdr.dev/docs/persistence-remote/
- https://herdr.dev/docs/integrations/
- https://herdr.dev/docs/plugins/
- https://herdr.dev/docs/marketplace/
- https://herdr.dev/docs/troubleshooting/
- https://herdr.dev/docs/windows-beta/

### cmux

- Site: https://cmux.com/
- Docs: https://cmux.com/docs/getting-started (and siblings)
- Agent index: https://cmux.com/llms.txt (every public page has `.md` / `.txt` variants)
- GitHub: https://github.com/manaflow-ai/cmux
- Mintlify (older automation docs): https://manaflow-ai-cmux.mintlify.app/ and https://mintlify.com/manaflow-ai/cmux/llms.txt
- DeepWiki: https://deepwiki.com/manaflow-ai/cmux
- Homebrew tap: `manaflow-ai/cmux`

Every `https://cmux.com/*.md` URL listed in `raw/indexes/cmux-llms.txt` was
downloaded into `raw/cmux/site/` (115 files, 0 failures). That includes docs,
blog posts, compare pages (including `cmux-vs-herdr`), agent landing pages,
pricing/enterprise, iOS, Linux, legal, and community pages.

GitHub files saved under `raw/cmux/github-docs/`:

- `cli-contract.md` (authoritative CLI compatibility contract)
- `custom-sidebars.md`, `events.md`, `notifications.md`, `configuration.md`
- `agent-hooks.md`, `agent-session-tracking-spec.md`
- `remote-tmux-reconcile-design.md`, `vault.md`, `workspace-groups.md`
- `dock.md`, `feed.md`

Mintlify pages saved under `raw/cmux/mintlify/`. Treat `cmux.com` +
`cli-contract.md` as newer than Mintlify when they disagree (socket path,
command coverage).

## Fetch failures / non-docs

- `https://herdr.dev/sitemap.xml` and `https://cmux.com/sitemap.xml` returned HTTP 500.
- `https://cmux.com/docs/tui.md` and `https://cmux.com/docs/custom-sidebars.md` returned HTTP 404 (TUI is linked from getting-started; custom sidebars live in GitHub `docs/custom-sidebars.md`).
- DeepWiki child pages are not static HTML; content is in the SPA payload. The extracted text is in `raw/deepwiki/`.
- Context7 / Exa / Tavily / Firecrawl could not be used (quota, auth, or missing CLI).
- `soheilhy/cmux` DeepWiki is the Go connection multiplexer — **unrelated**. Ignored except as a name collision.
- Herdr GitHub search also returns many plugins (`herdr-browser`, `herdr-sidebar`, etc.). Those are ecosystem, not core product docs.

## License note

Herdr's GitHub README/LICENSE present **Apache-2.0**. cmux.com's compare page
claims Herdr is AGPL. This corpus records both claims; the GitHub LICENSE of
`herdrdev/herdr` is the repo authority until Herdr says otherwise.

cmux is **GPL** (site + README).
