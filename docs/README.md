# Documentation index

This folder is the long-form documentation for **cmux-herdr**, a user-installed
plugin (not a compiled app).

| Doc | Audience | What it is |
|---|---|---|
| [../README.md](../README.md) | Everyone | Install, quick start, CLI |
| [ARCHITECTURE.md](ARCHITECTURE.md) | New contributors | Layout, no-build model, macOS vs Linux |
| [../CONTRIBUTING.md](../CONTRIBUTING.md) | Contributors | How to test and open a PR |
| [MAINTAINING.md](MAINTAINING.md) | Repo owner | First-time GitHub open-source checklist |
| [de/README.md](de/README.md) | Deutsch | Projektüberblick ohne Fachjargon |
| [de/GITHUB.md](de/GITHUB.md) | Deutsch | Issues, PRs, Releases, Secrets |
| [PLUGIN_DESIGN.md](PLUGIN_DESIGN.md) | Contributors | Why the plugin exists and how it talks to cmux/Herdr |
| [../mapping/concept-map.md](../mapping/concept-map.md) | Contributors | cmux ↔ Herdr ↔ tmux vocabulary |
| [../OPEN.md](../OPEN.md) | Contributors | What the plugin does **not** claim |
| [../RELEASE.md](../RELEASE.md) | Maintainers | How to cut a tag |
| [upstream/README.md](upstream/README.md) | Native-track readers | Design notes for **cmux**, not this CLI |

There is **no compile step**. `python3 -m py_compile` inside `./scripts/test.sh`
only checks that the Python sources parse.
