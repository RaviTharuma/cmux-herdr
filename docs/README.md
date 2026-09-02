# Documentation index

Product docs for **cmux-herdr**, the cmux plugin for Herdr.

| Doc | Audience | What it is |
|---|---|---|
| [../README.md](../README.md) | Everyone | Plugin landing page: install, features, commands |
| [de/README.md](de/README.md) | Deutsch | Produktüberblick |
| [de/GITHUB.md](de/GITHUB.md) | Deutsch | Issues, PRs, Releases, Secrets |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Contributors | Layout, no-build model, macOS vs Linux |
| [PLUGIN_DESIGN.md](PLUGIN_DESIGN.md) | Contributors | How the plugin talks to cmux and Herdr |
| [../CONTRIBUTING.md](../CONTRIBUTING.md) | Contributors | How to test and open a PR |
| [MAINTAINING.md](MAINTAINING.md) | Repo owner | GitHub open-source checklist |
| [../mapping/concept-map.md](../mapping/concept-map.md) | Contributors | cmux ↔ Herdr ↔ tmux vocabulary |
| [../OPEN.md](../OPEN.md) | Contributors | What the plugin does not claim |
| [../RELEASE.md](../RELEASE.md) | Maintainers | How to cut a tag |
| [upstream/README.md](upstream/README.md) | Native-track readers | Design notes for **cmux**, not this plugin |

There is **no compile step**. `python3 -m py_compile` inside `./scripts/test.sh`
only checks that the Python sources parse.
