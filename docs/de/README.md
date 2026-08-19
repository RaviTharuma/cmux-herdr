# cmux-herdr — Überblick (Deutsch)

Das GitHub-Projekt ist öffentlich:
**https://github.com/RaviTharuma/cmux-herdr**

Die technische Hauptsprache des Repos ist **Englisch** (README, Issues, PRs),
damit andere mitmachen können. Diese Seite erklärt das Projekt auf Deutsch.

## Was ist das?

Zwei Programme stecken ineinander:

1. **cmux** — die äußere macOS-App mit Fenstern, Workspaces, Tabs.
2. **Herdr** — der innere Terminal-Mux für KI-Agenten (Tabs, Panes, Status).

Wenn Herdr *in* einem cmux-Terminal läuft, sieht cmux oft nur **eine** Fläche.
Dieses Plugin (`cmux-herdr`) ist die Brücke dazwischen:

- Agent-Status von Herdr → farbige Status-Pills in cmux
- Herdr-Tabs/Panes → echte cmux-Tabs/Splits (`mirror`)
- Eine CLI, um Herdr zu steuern, ohne dass cmux das nativ können muss

Es ist **kein** Teil von cmux.app. Du installierst es selbst. Es braucht **kein
Compiling**: reines Python (Standardbibliothek), kein `pip`, kein `npm`, kein
Xcode-Build. `./scripts/test.sh` prüft nur, ob die `.py`-Dateien gültiges
Python sind, und führt Unit-Tests aus.

## Was du zum Benutzen brauchst

- macOS
- `cmux` und `herdr` im `PATH`
- Python 3.10 oder neuer

Zum **Testen der Quellen** reicht Linux mit Python — so läuft auch GitHub
Actions. Die echten Spiegel-Befehle brauchen aber die macOS-Apps.

## Installation

```bash
git clone https://github.com/RaviTharuma/cmux-herdr.git
cd cmux-herdr
./scripts/install.sh
cmux-herdr --version
cmux-herdr doctor
```

Details und alle Befehle: [README.md](../../README.md) (Englisch).

## Aufbau des Repos (kurz)

| Ordner | Inhalt |
|---|---|
| `bin/cmux-herdr` | Das Kommandozeilenprogramm |
| `bridge/` | Die Logik (Python-Module) plus Unit-Tests |
| `tests/` | Tests mit nachgemachtem `herdr`/`cmux` |
| `scripts/` | Installieren, Deinstallieren, Test-Skript |
| `sidebars/` | Optionale cmux-Sidebar |
| `docs/` | Diese Dokumentation |
| `LICENSE` | MIT — andere dürfen den Code nutzen und verändern |

Mehr: [ARCHITECTURE.md](../ARCHITECTURE.md).

## GitHub auf Deutsch

Wenn du GitHub zum ersten Mal als Maintainer nutzt: **[GITHUB.md](GITHUB.md)**.

## Lizenz und Privates

- Lizenz: MIT ([LICENSE](../../LICENSE)).
- Keine API-Keys, keine Konten, keine Telemetrie.
- Zustandsdateien liegen nur lokal unter `~/.local/state/cmux-herdr/`.
- Ein frühes Dump deiner lokalen cmux/Herdr-Sitzung wurde aus `main`
  entfernt. Lade so etwas nie wieder hoch.
