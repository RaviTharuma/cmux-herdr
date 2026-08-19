# Maintaining this repository (first public project)

This page is for the **owner** of `RaviTharuma/cmux-herdr`. Contributors should
read [CONTRIBUTING.md](../CONTRIBUTING.md) instead.

The GitHub repository is already **public**:
https://github.com/RaviTharuma/cmux-herdr

Public means anyone can clone, fork, open issues, and send pull requests.
It does **not** mean GitHub magically knows the license or the topics until
those files and settings exist. This repo now ships the files. A few clicks
in GitHub Settings are still yours.

## One-time GitHub Settings

Open **https://github.com/RaviTharuma/cmux-herdr/settings** and:

1. **General → Features**
   - Keep **Issues** on.
   - Turn **Wikis** off unless you actually use one (an empty wiki looks
     unfinished).
   - Discussions are optional (the cmux poll already lives upstream).
2. **General → Social preview / description**
   - Description (suggested): `macOS plugin that mirrors Herdr tabs, panes, and agent status into cmux.`
   - Topics (suggested): `cmux`, `herdr`, `tmux`, `macos`, `cli`, `python`
3. **Code security and analysis**
   - Enable **Secret scanning** and **Push protection** (public repos get this;
     turn the toggles on if they are off).
   - Enable **Private vulnerability reporting**.
4. **Branches → Branch protection** (optional but recommended for `main`)
   - Require the `test` GitHub Actions check before merge.
   - Do **not** allow force-pushes to `main`.

You can also set description and topics from a machine with `gh`:

```bash
gh repo edit RaviTharuma/cmux-herdr \
  --description "macOS plugin that mirrors Herdr tabs, panes, and agent status into cmux." \
  --add-topic cmux --add-topic herdr --add-topic tmux \
  --add-topic macos --add-topic cli --add-topic python \
  --enable-secret-scanning --enable-secret-scanning-push-protection
```

## What each GitHub object is

| Object | Purpose |
|---|---|
| **Issue** | A bug, idea, or question. Not a place to paste secrets. |
| **Pull request** | A proposed change from a branch. CI must stay green. |
| **Tag** (`v0.3.4`) | An immutable pointer to a commit. Installers clone a tag. |
| **Release** | A GitHub page built on a tag, with notes from `CHANGELOG.md`. |
| **LICENSE** | Tells other people they may use, copy, and modify the code (MIT). |
| **CODEOWNERS** | Asks GitHub to request a review from `@RaviTharuma` on every PR. |
| **Actions** | CI. This repo runs `./scripts/test.sh` on Python 3.10–3.13. |

## Cutting a release

Follow [RELEASE.md](../RELEASE.md). Short version:

1. `VERSION` and `CHANGELOG.md` match.
2. Merge to `main`.
3. `git tag -a vX.Y.Z` and push the tag.
4. Create the GitHub Release from that tag in the UI (or `gh release create`).

## Secrets and personal data

- Never commit `.env`, tokens, or live `cmux tree` dumps.
- `docs/live-env-snapshot.txt` was removed; do not restore it.
- The old `auto-squash-merge` workflow is gone on purpose. Do not add a
  workflow that merges PRs based only on a `cursor/*` branch name — on a
  public repo that is a gift to anyone who opens a similarly named PR.
- Git history of `main` still contains the old snapshot blob. Purging it
  would require rewriting `main` (force-push). Do that only if you accept
  breaking every existing clone, and follow
  [GitHub's sensitive-data removal guide](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/removing-sensitive-data-from-a-repository).

## Talking to the community

- Answer issues. Even “thanks, this is upstream — please file it on cmux”
  is enough.
- Use issue templates so people file plugin bugs here and Herdr/cmux bugs
  there.
- You do not have to accept every PR. “Not in scope for the plugin; see
  OPEN.md” is a valid close.

## Related repos you do not own

| Repo | Relation |
|---|---|
| [manaflow-ai/cmux](https://github.com/manaflow-ai/cmux) | Outer app |
| [herdrdev/herdr](https://github.com/herdrdev/herdr) | Inner mux |
| [RaviTharuma/cmux](https://github.com/RaviTharuma/cmux) | Your fork for native PRs |

Keep native Swift work on the cmux fork. This repository stays the
**userspace plugin**.
