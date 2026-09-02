# cmux-herdr

**Ein cmux-Plugin für Herdr.**

Live-Status-Pills in der cmux-Sidebar, Herdr-Tab- und Pane-Spiegelung,
und eine CLI, die verschachtelte Herdr-Agenten als vollwertige cmux-Flächen
behandelt.

```bash
git clone --branch v0.4.0 --depth 1 \
  https://github.com/RaviTharuma/cmux-herdr.git
cd cmux-herdr
./scripts/install.sh
```

Das GitHub-Projekt ist öffentlich:
**https://github.com/RaviTharuma/cmux-herdr**

Die technische Hauptsprache des Repos ist **Englisch** (README, Issues, PRs),
damit andere mitmachen können. Diese Seite ist der deutsche Produktüberblick.
Vollständige Befehle und Flags: [README.md](../../README.md).

## Was ist das Plugin?

Zwei Programme stecken ineinander:

1. **cmux** — die äußere macOS-App mit Fenstern, Workspaces, Tabs.
2. **Herdr** — der innere Terminal-Mux für KI-Agenten (Tabs, Panes, Status).

Ohne Plugin sieht cmux oft nur **eine** Fläche namens `herdr`. **cmux-herdr**
ist das Plugin dazwischen:

- Agent-Status von Herdr → farbige Status-Pills in der cmux-Sidebar
- Herdr-Tabs und -Panes → echte cmux-Tabs und Splits (`mirror`)
- Eine CLI (`cmux-herdr`), um beide Schichten zu steuern
- Optionale Custom-Sidebar, Agent-Skill und LaunchAgent

Es ist **kein** Bestandteil von `cmux.app`. Du installierst das Plugin selbst.
Es braucht **kein Compiling**: reines Python 3.10+ (Standardbibliothek), kein
`pip`, kein `npm`, kein Xcode-Build.

Aktuelle Version: **v0.4.0**.

## Installation

```bash
git clone --branch v0.4.0 --depth 1 \
  https://github.com/RaviTharuma/cmux-herdr.git
cd cmux-herdr
./scripts/install.sh
cmux-herdr --version
cmux-herdr doctor
```

Das legt an:

| Artefakt | Pfad |
|---|---|
| CLI | `~/.local/bin/cmux-herdr` |
| Sidebar | `~/.config/cmux/sidebars/herdr.swift` |
| Agent-Skill | `~/.agents/skills/cmux-herdr/` |

Schnellstart in einer Herdr-Pane *in* cmux:

```bash
cmux-herdr status
cmux-herdr tree
cmux-herdr sync
cmux-herdr watch --tmux-parity
```

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
Nein. `mirror` erzeugt zusätzliche cmux-Ansichten der laufenden Herdr-Sitzung.
Die echten PTYs bleiben bei Herdr.

## Aufbau des Repos (kurz)

| Ordner | Inhalt |
|---|---|
| `bin/cmux-herdr` | Das Kommandozeilenprogramm des Plugins |
| `bridge/` | Die Logik (Python-Module) plus Unit-Tests |
| `tests/` | Tests mit nachgemachtem `herdr`/`cmux` |
| `scripts/` | Installieren, Deinstallieren, Test-Skript |
| `sidebars/` | Optionale cmux-Sidebar `herdr` |
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
