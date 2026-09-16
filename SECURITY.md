# Security policy

## Supported versions

| Version | Supported |
|---|---|
| Latest release tag on `main` (currently **0.7.x**) | Yes |
| Older tags | Best-effort only |

This plugin is a **local** CLI/plugin. It has **no** cmux-herdr cloud service,
accounts, or first-party API keys. Security reports are still welcome: bugs in
socket handling, path handling, subprocess quoting, or release fetch/verify can
matter on a shared Mac.

Privacy posture: [PRIVACY.md](PRIVACY.md). Risk allocation:
[DISCLAIMER.md](DISCLAIMER.md).

## Trust boundary

| Trusts | Does not trust / does not claim |
|---|---|
| Local `herdr` Unix socket + allowlisted RPC | Network services owned by this project |
| Local `cmux` / `herdr` CLIs on `PATH` | That merged PRs or CI green are malware-free |
| Checksum-verified GitHub Release assets | Unsigned third-party mirrors without verification |
| Host fingerprint + one-writer lease | Guaranteed isolation from a compromised Mac user |

## What this project never needs

Do **not** put any of the following in issues, PRs, or the git tree:

- API keys, tokens, passwords, private keys
- `.env` files
- Real `HERDR_SOCKET_PATH` dumps from your machine
- Output of `cmux tree` / `herdr pane list` from a live personal session
- Employer or client workspace names, home paths, or hostnames

Association and rail cache files under `$XDG_STATE_HOME/cmux-herdr/` stay on
**your** disk. They are not uploaded by this plugin.

## Install integrity

Official installs use `bin/cmux-herdr-fetch`: HTTPS download of a target binary
plus `SHA256SUMS`, checksum verify, atomic install. Prefer that path over
copying unsigned binaries. Source builds (`cargo build --release`) are a
fallback for unusual architectures or offline work — you then own dependency
and toolchain trust.

## Reporting a vulnerability

Please use GitHub's private advisory form:

**https://github.com/RaviTharuma/cmux-herdr/security/advisories/new**

Include:

1. What the issue is (one paragraph)
2. How to reproduce it against this repo's tests or a local install
3. Impact (for example: writes pills to the wrong cmux workspace)

You should get an acknowledgement. Fixes ship as a patch release when possible.

Do **not** open a public issue for an exploitable bug until a fix is tagged.

## Supply chain and contributor risk

This is free MIT software. Maintainers do **not** warrant that dependencies,
Actions, release artifacts, or merged PRs are free of compromise or malice.
Users must verify checksums, pin what they run, and isolate secrets. Liability
and risk allocation: [DISCLAIMER.md](DISCLAIMER.md).

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
