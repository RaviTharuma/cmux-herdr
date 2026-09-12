# Issue reporting

There is **no support desk** for this plugin. Issues are optional signals.
If you file one, make it **complete enough that any agent (or human) can
reproduce and fix the bug without asking follow-up questions**.

Incomplete reports may be closed with a link back here.

## Where to file

| Kind | Where |
|---|---|
| This plugin (`cmux-herdr` CLI, install scripts, docs, tests) | [Issues here](https://github.com/RaviTharuma/cmux-herdr/issues/new/choose) |
| Security / exploitable bug | [SECURITY.md](SECURITY.md) (private advisory only) |
| Herdr itself | [herdrdev/herdr](https://github.com/herdrdev/herdr) |
| cmux.app / native sidebars / ssh-tmux | [manaflow-ai/cmux](https://github.com/manaflow-ai/cmux) |
| Native nested Herdr topology | [cmux#8737](https://github.com/manaflow-ai/cmux/issues/8737) |

Blank issues are disabled. Use a template.

## Agent-complete bug report checklist

A usable bug issue includes **all** of the following (redact secrets and
personal names; keep structure and IDs that matter for reproduction):

1. **Exact command(s)** that failed, copy-pasted (flags included).
2. **Expected vs actual** — one short paragraph each; include stderr/stdout
   when the CLI printed something.
3. **Numbered reproduction steps** from a clean starting point (install path,
   nested pane or not, first command).
4. **Versions** — `cmux-herdr --version`, `herdr --version`, `cmux --version`
   (or “not on PATH”).
5. **OS** — macOS version for product bugs; Linux only for test/doctor issues.
6. **Redacted `cmux-herdr doctor` output** (required for live CLI bugs).
7. **Host fingerprint presence** — which of these were set (yes/no, not the
   raw values): `CMUX_SURFACE_ID`, `HERDR_SOCKET_PATH`, `HERDR_SERVER_PID`,
   `HERDR_WORKSPACE_ID`, `HERDR_ENV`. Say whether you passed `--workspace`.
8. **Install path** — plugin-manager install, contributor `./scripts/install.sh`,
   or `cargo build` / release binary.
9. **Scope guess** — which area: `doctor` / `watch` / `mirror` / `sync` /
   `attach-pane` / `sessions` / install / docs / tests. Link a file under
   `src/` if you know it.
10. **Repro with fakes?** — Can `./scripts/test.sh` or a hermetic fake from
    `tests/` show it? If yes, describe how. If no, say why it needs live macOS.
11. **Regression?** — Last known good tag/commit if you have one.

## What not to paste

Never put these in issues, PRs, screenshots, or the git tree:

- API keys, tokens, passwords, private keys, `.env` files
- Live unredacted `cmux tree` / `herdr pane list` dumps
- Real socket paths, home directories, employer/client workspace titles

Redact paths to placeholders like `/Users/<you>/…` and workspace titles to
`<workspace>`. Keep pane IDs, command names, and error strings.

See [SECURITY.md](SECURITY.md) and [PRIVACY.md](PRIVACY.md).

## Enhancements and docs

Enhancements still need: problem, concrete proposal, acceptance criteria, and
area. Docs issues need file path + current text + proposed change. Vague
“make it better” issues are not actionable for agents.

## After you file

No SLA and no promise of a human reply. A complete report is what lets an
automated agent open a PR. Scope limits: [DISCLAIMER.md](DISCLAIMER.md),
[OPEN.md](OPEN.md).
