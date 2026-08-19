# Security policy

## Supported versions

| Version | Supported |
|---|---|
| Latest tag on `main` (currently 0.3.x) | Yes |
| Older tags | Best-effort only |

This project is a local CLI. It has **no cloud service, no accounts, and no
API keys of its own**. Security issues are still welcome: a bug in socket
handling, path handling, or subprocess quoting can matter on a shared Mac.

## What this project never needs

Do **not** put any of the following in issues, PRs, or the git tree:

- API keys, tokens, passwords, private keys
- `.env` files
- Real `HERDR_SOCKET_PATH` dumps from your machine
- Output of `cmux tree` / `herdr pane list` from a live personal session
- Employer or client workspace names, home paths, or hostnames

Association cache files under `$XDG_STATE_HOME/cmux-herdr/` stay on **your**
disk. They are not uploaded anywhere.

## Reporting a vulnerability

Please use GitHub's private advisory form:

**https://github.com/RaviTharuma/cmux-herdr/security/advisories/new**

Include:

1. What the issue is (one paragraph)
2. How to reproduce it against this repo's tests or a local install
3. Impact (for example: writes pills to the wrong cmux workspace)

You should get an acknowledgement. Fixes ship as a patch release when possible.

Do **not** open a public issue for an exploitable bug until a fix is tagged.

## Maintainer notes (history)

`docs/live-env-snapshot.txt` was a local `cmux`/`herdr` dump committed early
in the repo. It contained hostnames, home-relative paths, and workspace
titles. It is **removed from `main` as of 0.3.4**. Older commits still have
the blob (this project does not force-push `main`). There were **no API keys**
in that file.

If you clone an old tag and still see that file, delete it locally and do
not copy it into a fork.

An earlier helper workflow auto-squash-merged branches named `cursor/*`.
That workflow is removed: on a public repository it could merge untrusted
PRs. CI now runs tests only.
