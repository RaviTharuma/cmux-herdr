# Release checklist

Tag and publish **after** the version-bump PR has merged to `main`.
Do not invent a release script beyond these steps.

Current tagged line: **v0.3.4**. Replace `X.Y.Z` below with the version in
[`VERSION`](VERSION) (no `v` prefix in the file, `v` prefix on the git tag).

## Preconditions

- [ ] Version PR is merged to `main`
- [ ] Working tree on `main` matches the merge commit
- [ ] `VERSION` contains `X.Y.Z` (no `v` prefix)
- [ ] `CHANGELOG.md` has a `## [X.Y.Z]` section dated correctly
- [ ] `./scripts/test.sh` is green locally and on GitHub Actions

## 1. Run tests

```bash
./scripts/test.sh
./bin/cmux-herdr --version   # → cmux-herdr X.Y.Z
```

Optional live smoke (needs Herdr nested in cmux on macOS):

```bash
./bin/cmux-herdr doctor
./bin/cmux-herdr status
./bin/cmux-herdr tree
./bin/cmux-herdr sync
```

## 2. Tag

From the merge commit on `main`:

```bash
git checkout main
git pull origin main
git tag -a vX.Y.Z -m "cmux-herdr vX.Y.Z"
git push origin vX.Y.Z
```

## 3. GitHub Release

In the GitHub UI: **Releases → Draft a new release**, choose tag `vX.Y.Z`,
paste the `[X.Y.Z]` section from `CHANGELOG.md`.

Or from a machine with `gh`:

```bash
gh release create vX.Y.Z --title "vX.Y.Z" --notes-file CHANGELOG.md
```

Prefer pasting only that version's changelog section, not the whole file.

## 4. Install from the tag

```bash
git clone --branch vX.Y.Z --depth 1 \
  https://github.com/RaviTharuma/cmux-herdr.git
cd cmux-herdr
./scripts/install.sh
# optional continuous watch:
./scripts/install-watch-service.sh
cmux-herdr --version
```

Install paths (after `install.sh`):

| Artifact | Path |
|---|---|
| CLI | `~/.local/bin/cmux-herdr` |
| Sidebar | `~/.config/cmux/sidebars/herdr.swift` |
| Agent skill | `~/.agents/skills/cmux-herdr/` (and/or `~/.pi/agent/skills/cmux-herdr/`) |
| LaunchAgent plist | `~/Library/LaunchAgents/com.cmux-herdr.watch.plist` |
| Watch logs | `~/Library/Logs/cmux-herdr-watch.{out,err}.log` |
| Association cache | `~/.local/state/cmux-herdr/` |

## Version bump for later releases

1. Edit `VERSION` (e.g. `0.3.5` or `0.4.0`).
2. Move `[Unreleased]` notes into a new `CHANGELOG.md` section.
3. Confirm `cmux-herdr --version` prints the new value.
4. Open a prep PR; after merge, tag `vX.Y.Z` and publish the GitHub Release.

Do not expand this plugin into [manaflow-ai/cmux#8737](https://github.com/manaflow-ai/cmux/issues/8737).
