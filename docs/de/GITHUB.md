# GitHub für das cmux-herdr Plugin (Deutsch)

Kurzer Leitfaden für das öffentliche Plugin-Repo.
Die Oberfläche von GitHub ist auf Englisch; die Begriffe unten reichen zum
Arbeiten.

## Das Repo ist schon öffentlich

Adresse: https://github.com/RaviTharuma/cmux-herdr

**Öffentlich** heißt: jede Person darf klonen, forken, Issues öffnen und
Pull Requests schicken. Du musst niemanden extra einladen.

Was GitHub *nicht* von allein setzt (einmal unter Settings klicken):

- Topics / Beschreibung (Vorschlag in [MAINTAINING.md](../MAINTAINING.md))
- Secret scanning / Push protection / private Vulnerability reports
- Wiki aus, wenn du keins nutzt

## Die wichtigsten Wörter

| Begriff | Bedeutung |
|---|---|
| **Repository (Repo)** | Der Projektordner auf GitHub. |
| **Clone** | Eine lokale Kopie (`git clone …`). |
| **Fork** | Eine Kopie unter *einem anderen* GitHub-Konto. |
| **Branch** | Eine Arbeitslinie. `main` ist die stabile Linie. |
| **Commit** | Ein gespeicherter Schnappschuss mit Nachricht. |
| **Push** | Commits zum GitHub-Remote schicken. |
| **Issue** | Fehler, Idee, Frage. Keine Secrets hineinkopieren. |
| **Pull Request (PR)** | Vorschlag, einen Branch nach `main` zu mergen. |
| **Review** | Jemand schaut sich den PR an. |
| **Merge** | Der PR landet auf `main`. Hier üblich: squash-merge. |
| **Tag** | Namensschild an einem Commit, z.B. `v0.3.4`. |
| **Release** | Eine GitHub-Seite zu einem Tag, mit Changelog. |
| **Actions / CI** | GitHub führt `./scripts/test.sh` automatisch aus. |
| **LICENSE** | MIT: Nutzung, Kopie, Änderung erlaubt, ohne Gewähr. |

## Typischer Ablauf einer Änderung

1. Branch von `main` (zum Beispiel `cursor/fix-typo-….`).
2. Code oder Docs ändern, `./scripts/test.sh` ausführen.
3. Commit + Push.
4. Pull Request gegen `main` öffnen.
5. Warten, bis der CI-Haken grün ist.
6. Squash-merge.

## Issues vs. die anderen Projekte

| Problem liegt in … | Wohin |
|---|---|
| `cmux-herdr` CLI, Installer, diese Docs | Issue **hier** |
| Herdr selbst | https://github.com/herdrdev/herdr |
| cmux.app / natives tmux | https://github.com/manaflow-ai/cmux |

Issue-Vorlagen fragen das ab, damit nicht alles hier landet.

## Secrets — die eine harte Regel

Nie in Git, Issues oder Screenshots:

- API-Keys, Tokens, Passwörter, private SSH-Keys
- `.env`-Dateien
- Live-Dumps von `cmux tree` mit echten Pfaden und Kundennamen

Wenn etwas doch rutscht: Token **sofort rotieren**, Datei aus dem Tree
löschen. History auf `main` umzuschreiben (force-push) ist ein letzter
Schritt und bricht alle Klone — siehe GitHubs Anleitung „Removing sensitive
data from a repository“.

## Community-Dateien in diesem Repo

| Datei | Zweck |
|---|---|
| `README.md` | Erste Seite für Besucher |
| `LICENSE` | MIT |
| `CODE_OF_CONDUCT.md` | Umgangsregeln |
| `CONTRIBUTING.md` | Wie man mitmacht |
| `SECURITY.md` | Wie man Sicherheitslücken *privat* meldet |
| `.github/ISSUE_TEMPLATE/` | Formulare für Bugs / Features / Docs |
| `.github/PULL_REQUEST_TEMPLATE.md` | Checkliste an jedem PR |
| `.github/workflows/ci.yml` | Tests auf GitHub |

Mehr Einstellungen: [MAINTAINING.md](../MAINTAINING.md) (Englisch).
