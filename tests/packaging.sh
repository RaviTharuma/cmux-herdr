#!/usr/bin/env bash
# Offline only: no host CLI, service manager, or user config is touched.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
# macOS temporary roots may traverse /var -> /private/var. Resolve only the
# fixture root; deliberately redirected paths created below must stay symlinks.
TMP=$(cd "$TMP" && pwd -P)
export HOME="$TMP/home" XDG_STATE_HOME="$TMP/state"
mkdir -p "$HOME/.local/bin" "$TMP/repo/scripts" "$TMP/repo/bin" "$TMP/repo/agent-skill"
cp "$ROOT/scripts/install.sh" "$ROOT/scripts/uninstall.sh" "$TMP/repo/scripts/"
if [[ -f "$ROOT/scripts/packaging-manifest.sh" ]]; then cp "$ROOT/scripts/packaging-manifest.sh" "$TMP/repo/scripts/"; fi
printf '#!/bin/sh\nexit 0\n' > "$TMP/repo/bin/cmux-herdr-fetch"
printf '#!/bin/sh\necho invoked >> "$HOME/host-called"\n' > "$TMP/repo/bin/cmux-herdr"
chmod +x "$TMP/repo/bin/"*
printf 'skill original\n' > "$TMP/repo/agent-skill/SKILL.md"
# Exercise the zero-entry manifest before any artifact has ever been installed.
# Invoke the selected Bash so this also covers Bash 3.2's nounset array behavior.
HOME="$TMP/empty-home" XDG_STATE_HOME="$TMP/empty-state" bash -eu -o pipefail -c '
  source "$1"
  packaging_init
  packaging_save
  [[ -f "$MANIFEST" && ! -s "$MANIFEST" ]]
' _ "$ROOT/scripts/packaging-manifest.sh"
HOME="$TMP/empty-home" XDG_STATE_HOME="$TMP/empty-state" bash "$TMP/repo/scripts/uninstall.sh"
printf 'foreign binary\n' > "$HOME/.local/bin/cmux-herdr"
bash "$TMP/repo/scripts/install.sh" > "$TMP/install.log" 2>&1 || true
[[ $(cat "$HOME/.local/bin/cmux-herdr") == 'foreign binary' ]]
rm "$HOME/.local/bin/cmux-herdr"
bash "$TMP/repo/scripts/install.sh"
[[ -L "$HOME/.local/bin/cmux-herdr" ]]
printf 'user edit\n' > "$HOME/.agents/skills/cmux-herdr/SKILL.md"
printf 'new upstream\n' > "$TMP/repo/agent-skill/SKILL.md"
bash "$TMP/repo/scripts/install.sh" || true
[[ $(cat "$HOME/.agents/skills/cmux-herdr/SKILL.md") == 'user edit' ]]
[[ $(cat "$HOME/.pi/agent/skills/cmux-herdr/SKILL.md") == 'new upstream' ]]
mkdir -p "$HOME/.config/cmux/sidebars"
printf 'foreign sidebar\n' > "$HOME/.config/cmux/sidebars/herdr.js"
printf 'foreign extra\n' > "$HOME/.pi/agent/skills/cmux-herdr/extra"
rm "$HOME/.local/bin/cmux-herdr"
ln -s "$TMP/foreign" "$HOME/.local/bin/cmux-herdr"
bash "$TMP/repo/scripts/uninstall.sh"
[[ $(readlink "$HOME/.local/bin/cmux-herdr") == "$TMP/foreign" ]]
[[ $(cat "$HOME/.agents/skills/cmux-herdr/SKILL.md") == 'user edit' ]]
[[ -f "$HOME/.pi/agent/skills/cmux-herdr/extra" ]]
[[ ! -e "$HOME/.pi/agent/skills/cmux-herdr/SKILL.md" ]]
[[ -f "$HOME/.config/cmux/sidebars/herdr.js" && ! -e "$HOME/host-called" ]]
# A redirected skill directory must not expose the external file to install/uninstall.
rm -rf "$HOME/.pi/agent/skills/cmux-herdr"
mkdir "$TMP/external"
printf 'outside\n' > "$TMP/external/SKILL.md"
ln -s "$TMP/external" "$HOME/.pi/agent/skills/cmux-herdr"
bash "$TMP/repo/scripts/install.sh" || true
bash "$TMP/repo/scripts/uninstall.sh"
[[ $(cat "$TMP/external/SKILL.md") == outside ]]
# Recover the original symlink; only that exact owned target may be removed.
rm "$HOME/.local/bin/cmux-herdr"
ln -s "$TMP/repo/bin/cmux-herdr" "$HOME/.local/bin/cmux-herdr"
bash "$TMP/repo/scripts/uninstall.sh"
[[ ! -L "$HOME/.local/bin/cmux-herdr" ]]
[[ -x "$ROOT/scripts/install.sh" && -x "$ROOT/scripts/uninstall.sh" ]]
# A late destination failure must not lose ownership of earlier successful writes.
export HOME="$TMP/partial-home" XDG_STATE_HOME="$TMP/partial-state"
mkdir -p "$HOME/.pi"
printf 'blocked parent\n' > "$HOME/.pi/agent"
if bash "$TMP/repo/scripts/install.sh"; then exit 1; fi
[[ -L "$HOME/.local/bin/cmux-herdr" ]]
rm "$HOME/.pi/agent"
bash "$TMP/repo/scripts/install.sh"
bash "$TMP/repo/scripts/uninstall.sh"
[[ ! -L "$HOME/.local/bin/cmux-herdr" ]]
[[ ! -f "$HOME/.agents/skills/cmux-herdr/SKILL.md" ]]
# Empty manifest and stale lock recovery are deterministic on Bash 3.2 as well.
bash "$TMP/repo/scripts/uninstall.sh"
mkdir "$XDG_STATE_HOME/cmux-herdr/packaging/lock"
if bash "$TMP/repo/scripts/install.sh"; then exit 1; fi
rmdir "$XDG_STATE_HOME/cmux-herdr/packaging/lock"
bash "$TMP/repo/scripts/uninstall.sh"
export HOME="$TMP/foreign-home" XDG_STATE_HOME="$TMP/foreign-state"
mkdir -p "$HOME/.agents/skills/cmux-herdr"
printf 'foreign directory\n' > "$HOME/.agents/skills/cmux-herdr/README"
bash "$TMP/repo/scripts/install.sh"
[[ ! -e "$HOME/.agents/skills/cmux-herdr/SKILL.md" ]]
bash "$TMP/repo/scripts/uninstall.sh"
[[ -f "$HOME/.agents/skills/cmux-herdr/README" ]]
# A stale manifest temporary name must not strand an unowned launcher.
export HOME="$TMP/stale-home" XDG_STATE_HOME="$TMP/stale-state"
mkdir -p "$XDG_STATE_HOME/cmux-herdr/packaging/manifest.new"
printf 'foreign sentinel\n' > "$XDG_STATE_HOME/cmux-herdr/packaging/manifest.new/keep"
bash "$TMP/repo/scripts/install.sh"
bash "$TMP/repo/scripts/uninstall.sh"
[[ ! -L "$HOME/.local/bin/cmux-herdr" ]]
[[ -f "$XDG_STATE_HOME/cmux-herdr/packaging/manifest.new/keep" ]]
# Failure publishing the manifest must roll back the filesystem mutation.
mkdir -p "$TMP/fail-bin"
REAL_MV=$(command -v mv)
export REAL_MV
printf '#!/bin/bash\nfor arg; do case "$arg" in */manifest.tsv) exit 71;; esac; done\nexec "$REAL_MV" "$@"\n' > "$TMP/fail-bin/mv"
chmod +x "$TMP/fail-bin/mv"
if PATH="$TMP/fail-bin:$PATH" bash "$TMP/repo/scripts/install.sh"; then exit 1; fi
[[ ! -L "$HOME/.local/bin/cmux-herdr" ]]
bash "$TMP/repo/scripts/install.sh"
if PATH="$TMP/fail-bin:$PATH" bash "$TMP/repo/scripts/uninstall.sh"; then exit 1; fi
[[ -L "$HOME/.local/bin/cmux-herdr" ]]
[[ -f "$HOME/.pi/agent/skills/cmux-herdr/SKILL.md" ]]
bash "$TMP/repo/scripts/uninstall.sh"
# An install/uninstall cycle must allow the next install to restore both skills.
bash "$TMP/repo/scripts/install.sh"
[[ -f "$HOME/.agents/skills/cmux-herdr/SKILL.md" ]]
[[ -f "$HOME/.pi/agent/skills/cmux-herdr/SKILL.md" ]]
bash "$TMP/repo/scripts/uninstall.sh"
echo 'OK: packaging ownership fixtures'
