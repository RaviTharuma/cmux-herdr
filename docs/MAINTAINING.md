# Maintaining this repository (first public project)

This page is for the **owner** of `RaviTharuma/cmux-herdr`. Contributors should
read [CONTRIBUTING.md](../CONTRIBUTING.md) instead.

The GitHub repository is **public**:
https://github.com/RaviTharuma/cmux-herdr

Public means anyone can clone, fork, open issues, and send pull requests.
License is MIT. CI runs on every push to `main` and every pull request.
Pushing a `vX.Y.Z` tag publishes a GitHub Release automatically.

Repository **administration** (topics, wiki toggle, secret-scanning org
toggles) is not writable with the automation token this project uses.
Public GitHub.com repos still get secret scanning from GitHub itself.
Default issue labels (`bug`, `enhancement`, `documentation`, …) are already
present.

## What each GitHub object is

| Object | Purpose |
|---|---|
| **Issue** | A bug, idea, or question. Not a place to paste secrets. |
| **Pull request** | A proposed change from a branch. CI must stay green. |
| **Tag** (`v0.3.4`) | An immutable pointer to a commit. Installers clone a tag. |
| **Release** | A GitHub page built on a tag, with notes from `CHANGELOG.md`. |
| **LICENSE** | Tells other people they may use, copy, and modify the code (MIT). |
| **CODEOWNERS** | Asks GitHub to request a review from `@RaviTharuma` on every PR. |
| **Actions** | CI on Python 3.10–3.13; tag `vX.Y.Z` publishes a GitHub Release. |

## Cutting a release

Follow [RELEASE.md](../RELEASE.md). Short version:

1. `VERSION` and `CHANGELOG.md` match.
2. Merge to `main`.
3. `git tag -a vX.Y.Z` and `git push origin vX.Y.Z`.
4. The `release` GitHub Action publishes the GitHub Release.

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
