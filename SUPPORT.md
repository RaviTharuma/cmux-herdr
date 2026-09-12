# Support and issue reporting

This page is the **issue reporting guide** for cmux-herdr. Please read it
before opening an issue.

## Where to get help

| Need | Where |
|---|---|
| Plugin bug, docs mistake, or enhancement | [Issues here](https://github.com/RaviTharuma/cmux-herdr/issues/new/choose) |
| How to contribute / open a PR | [CONTRIBUTING.md](CONTRIBUTING.md) |
| Security vulnerability | [SECURITY.md](SECURITY.md) (private advisory only) |
| Code of Conduct incident | [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) |
| Herdr itself | [herdrdev/herdr](https://github.com/herdrdev/herdr) |
| cmux.app / native sidebars / ssh-tmux | [manaflow-ai/cmux](https://github.com/manaflow-ai/cmux) |
| Native nested Herdr topology | [cmux#8737](https://github.com/manaflow-ai/cmux/issues/8737) |

Blank issues are disabled. Use a template, or one of the contact links above.

## Before you open an issue

1. Confirm the problem is in **this plugin**, not in cmux or Herdr.
2. Run `cmux-herdr doctor` and note the versions (`cmux-herdr --version`,
   `herdr --version`, `cmux --version` when available).
3. Redact personal paths, hostnames, customer names, and tokens.
4. Search existing issues for a duplicate.
5. Prefer the matching template: **Plugin bug**, **Plugin enhancement**, or
   **Docs**.

## What a good bug report includes

- What you ran and what happened (one or two sentences)
- What you expected
- Numbered reproduction steps
- Versions and OS (macOS for product use; Linux is tests/doctor only)
- Redacted `cmux-herdr doctor` output when relevant

## What not to paste

Never put these in issues, PRs, screenshots, or the git tree:

- API keys, tokens, passwords, private keys, `.env` files
- Live `cmux tree` / `herdr pane list` dumps from a personal session
- Real `HERDR_SOCKET_PATH` values or employer/client workspace names

See [SECURITY.md](SECURITY.md) and [PRIVACY.md](PRIVACY.md).

## Response expectations

This is a small, volunteer-maintained project. Maintainers triage issues when
they can. A reply that points you upstream ("file this on cmux") is a valid
outcome. Security reports get priority acknowledgement when possible.

There is no paid support channel and no SLA.
