# Release checklist (v0.1.0)

Tag and publish **after** this release PR merges to `main`. Do not invent a release
script beyond these steps.

## Preconditions

- [ ] PR for release prep is merged to `main`
- [ ] Working tree on `main` matches the merge commit
- [ ] `VERSION` contains `0.1.0` (no `v` prefix)
- [ ] `CHANGELOG.md` has a `## [0.1.0]` section dated correctly

## 1. Run tests

```bash
./scripts/test.sh
```

Expect: `OK: all cmux-herdr tests passed (unittest only; no pytest)`.

Optional smoke (needs live Herdr nested in cmux):

```bash
./bin/cmux-herdr --version   # → cmux-herdr 0.1.0
./bin/cmux-herdr status
./bin/cmux-herdr tree
./bin/cmux-herdr sync
```

## 2. Tag

From the merge commit on `main`:

```bash
git checkout main
git pull origin main
git tag -a v0.1.0 -m "cmux-herdr v0.1.0"
git push origin v0.1.0
```

## 3. GitHub Release

```bash
gh release create v0.1.0 \
  --title "v0.1.0" \
  --notes-file CHANGELOG.md
```

Or create the release in the GitHub UI from tag `v0.1.0`, pasting the `0.1.0`
section from `CHANGELOG.md`.

## 4. Install from the tag

```bash
git clone --branch v0.1.0 --depth 1 \
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

## 5. Close tracking

- Close plugin issue [#3](https://github.com/RaviTharuma/cmux-herdr/issues/3) (tagged release)
  after the GitHub Release exists.
- Close [#2](https://github.com/RaviTharuma/cmux-herdr/issues/2) (multi-parent host fingerprint bindings) when that fix is merged (Unreleased / 0.2 prep).
- Do not expand into [manaflow-ai/cmux#8737](https://github.com/manaflow-ai/cmux/issues/8737).

## Version bump for later releases

1. Edit `VERSION` (e.g. `0.2.0`).
2. Add a new `CHANGELOG.md` section.
3. Confirm `cmux-herdr --version` prints the new value.
4. Open a prep PR; after merge, tag `vX.Y.Z` and `gh release create` as above.
