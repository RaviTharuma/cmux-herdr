#!/usr/bin/env bash
# Contributor/dev install (CLI symlink + agent skill).
# End users should run: cmux sidebar plugin install <this-repo.git>
# Does not copy custom sidebars. Native Herdr chrome is parent cmux.
# No root required.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_SRC="${ROOT}/bin/cmux-herdr"
FETCH_SRC="${ROOT}/bin/cmux-herdr-fetch"
LOCAL_BIN="${HOME}/.local/bin"
TARGET="${LOCAL_BIN}/cmux-herdr"
SKILL_SRC="${ROOT}/agent-skill"

echo "cmux-herdr plugin install"
echo "  repo: ${ROOT}"

if [[ ! -x "${BIN_SRC}" || ! -x "${FETCH_SRC}" ]]; then
  echo "error: missing executable launcher/bootstrap under ${ROOT}/bin" >&2
  exit 1
fi
"${FETCH_SRC}"

mkdir -p "${LOCAL_BIN}"
# Prefer symlink so edits in the repo are live; fall back to copy.
if ln -sfn "${BIN_SRC}" "${TARGET}" 2>/dev/null; then
  echo "  cli:  ${TARGET} -> ${BIN_SRC}"
else
  cp "${BIN_SRC}" "${TARGET}"
  chmod +x "${TARGET}"
  echo "  cli:  ${TARGET} (copied)"
fi

# The launcher resolves this symlink back to the checkout and executes the
# verified binary under .cmux-herdr/bin.

# Do not copy sidebars/herdr.js or herdr.swift into ~/.config/cmux/sidebars/.
# Those files stay in the repo as experimental leftovers. Uninstall removes
# leftover copies from older installs.
echo "  sidebar: not installed (experimental leftover in repo; native chrome is parent cmux)"

install_skill_dir() {
  local dest="$1"
  mkdir -p "${dest}"
  if [[ -d "${SKILL_SRC}" ]]; then
    cp -R "${SKILL_SRC}/." "${dest}/"
    echo "  skill: ${dest}"
    return 0
  fi
  return 1
}

SKILL_INSTALLED=0
if [[ -d "${HOME}/.agents/skills" ]] || mkdir -p "${HOME}/.agents/skills" 2>/dev/null; then
  if install_skill_dir "${HOME}/.agents/skills/cmux-herdr"; then
    SKILL_INSTALLED=1
  fi
fi
if [[ -d "${HOME}/.pi/agent" ]] || mkdir -p "${HOME}/.pi/agent/skills" 2>/dev/null; then
  if install_skill_dir "${HOME}/.pi/agent/skills/cmux-herdr"; then
    SKILL_INSTALLED=1
  fi
fi
if [[ "${SKILL_INSTALLED}" -eq 0 ]]; then
  echo "  skill: (skipped)"
fi

# Optional: shims note only (no automatic PATH mutation)
if [[ -d "${ROOT}/shims" ]]; then
  echo "  shims: see ${ROOT}/shims/README.md (optional)"
fi

echo
echo "Next steps:"
echo "  1. Ensure ~/.local/bin is on PATH"
echo "  2. Inside a Herdr pane nested in cmux, run:"
echo "       cmux-herdr doctor"
echo "       cmux-herdr watch"
echo "  3. Agents: skill installed as cmux-herdr (if skill dirs present)"
echo
echo "Plugin installed."
