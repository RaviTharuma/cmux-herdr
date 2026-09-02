# cmux-herdr

**Ein cmux-Plugin für Herdr.**

cmux ist die offizielle Oberfläche von Herdr. Herdr ist die Engine.
Native cmux-Chrome — Maus, Reorderable, Tabs und Panes — kein
eingerahmtes Herdr-Fenster.

```bash
cmux sidebar plugin install https://github.com/RaviTharuma/cmux-herdr.git
cmux-herdr doctor
cmux-herdr watch
```

Das GitHub-Projekt ist öffentlich:
**https://github.com/RaviTharuma/cmux-herdr**

Die technische Hauptsprache des Repos ist **Englisch** (README, Issues, PRs),
damit andere mitmachen können. Diese Seite ist der deutsche Produktüberblick.
Vollständige Befehle und Flags: [README.md](../../README.md).

## Was ist das Plugin?

Zwei Programme stecken ineinander:

1. **cmux** — die offizielle GUI: Fenster, Workspaces, Tabs, Panes.
2. **Herdr** — die Engine für KI-Agenten (Tabs, Panes, Status).

Ohne Plugin sieht cmux oft nur **eine** Fläche namens `herdr`. **cmux-herdr**
macht cmux zur Herdr-Oberfläche:

- Agent-Status von Herdr → Status-Pills in der cmux-Sidebar
- Herdr-Tabs und -Panes → echte cmux-Tabs und Splits (`watch`)
- Eine CLI (`cmux-herdr`), um die Engine zu steuern
- Agent-Skill, LaunchAgent, Status-Pills und `watch`

Es ist **kein** Bestandteil von `cmux.app`. Du installierst das Plugin selbst.
Es braucht **kein Compiling**: reines Python 3.10+ (Standardbibliothek), kein
`pip`, kein `npm`, kein Cargo-Binary.

Aktuelle Version: **v0.6.1**.

## Installation

Offizielle Installation ist der cmux-Plugin-Manager plus die `cmux-herdr`-CLI.
Native Herdr-Chrome ist parent cmux (#8736 / #10045). Dieses Plugin kopiert
keine Custom-Sidebar nach `~/.config/cmux/sidebars/`.

```bash
cmux sidebar plugin install https://github.com/RaviTharuma/cmux-herdr.git
cmux sidebar plugin use cmux-herdr
cmux sidebar plugin update cmux-herdr
cmux sidebar plugin remove cmux-herdr
```

`./scripts/install.sh` ist nur für Mitwirkende (CLI-Symlink + Skill, siehe
CONTRIBUTING). Es kopiert keine Sidebar-Dateien.

Schnellstart in einer Herdr-Pane *in* cmux:

```bash
cmux-herdr doctor
cmux-herdr watch
```

`watch` reicht. Du bleibst in der cmux-Oberfläche. `sidebars/herdr.js` und
`herdr.swift` bleiben im Repo als experimentelle Reste, nicht als Default.

## Was du zum Benutzen brauchst

- macOS
- `cmux` und `herdr` im `PATH`
- Python 3.10 oder neuer
- Herdr 0.8+

Zum **Testen der Quellen** reicht Linux mit Python — so läuft auch GitHub
Actions. Die echten Spiegel-Befehle brauchen die macOS-Apps.

## FAQ

**Ist das offiziell in cmux eingebaut?**
Nein. Es ist ein installierbares Plugin. Du behältst es, wenn du cmux
aktualisierst.

**Braucht es einen cmux-PR?**
Nein. Installieren und loslegen.

**Ersetzt es natives ssh-tmux?**
Nein. `watch` erzeugt zusätzliche cmux-Ansichten der laufenden Herdr-Sitzung.
Die echten PTYs bleiben bei Herdr.

## Aufbau des Repos (kurz)

| Ordner | Inhalt |
|---|---|
| `bin/cmux-herdr` | Das Kommandozeilenprogramm des Plugins |
| `bridge/` | Die Logik (Python-Module) plus Unit-Tests |
| `tests/` | Tests mit nachgemachtem `herdr`/`cmux` |
| `cmux-plugin.toml` | Manifest für den offiziellen Plugin-Manager |
| `bin/cmux-herdr-sidebar` | Sidebar-TUI |
| `scripts/` | Entwickler-Install, Deinstallieren, Test-Skript |
| `sidebars/` | Experimentelle Custom-Sidebar `herdr` (JS + Swift, kein Default) |
| `agent-skill/` | Skill für Coding-Agenten |
| `docs/` | Diese Dokumentation |
| `LICENSE` | MIT — andere dürfen den Code nutzen und verändern |

Mehr: [ARCHITECTURE.md](../ARCHITECTURE.md).

## GitHub auf Deutsch

Wenn du GitHub zum ersten Mal als Maintainer nutzt: **[GITHUB.md](GITHUB.md)**.

## Lizenz und Privates

- Lizenz: MIT ([LICENSE](../../LICENSE)).
- Keine API-Keys, keine Konten, keine Telemetrie.
- Zustandsdateien liegen nur lokal unter `~/.local/state/cmux-herdr/`.
- Ein frühes Dump einer lokalen cmux/Herdr-Sitzung wurde aus `main`
  entfernt. Lade so etwas nie wieder hoch.
