#!/usr/bin/env bash
# Contributor/dev install: CLI symlink + agent skill only.
# The product install path is: cmux sidebar plugin install <this-repo.git>
# No root required.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_SRC="${ROOT}/bin/cmux-herdr"
LOCAL_BIN="${HOME}/.local/bin"
TARGET="${LOCAL_BIN}/cmux-herdr"
SKILL_SRC="${ROOT}/agent-skill"

echo "cmux-herdr contributor install (not the product install path)"
echo "  repo: ${ROOT}"

if [[ ! -f "${BIN_SRC}" ]]; then
  echo "error: missing ${BIN_SRC}" >&2
  exit 1
fi
chmod +x "${BIN_SRC}"
chmod +x "${ROOT}/bridge/cmux_herdr_bridge.py" 2>/dev/null || true

mkdir -p "${LOCAL_BIN}"
# Prefer symlink so edits in the repo are live; fall back to copy.
if ln -sfn "${BIN_SRC}" "${TARGET}" 2>/dev/null; then
  echo "  cli:  ${TARGET} -> ${BIN_SRC}"
else
  cp "${BIN_SRC}" "${TARGET}"
  chmod +x "${TARGET}"
  echo "  cli:  ${TARGET} (copied)"
fi

# Ensure bridge package is importable when invoked via symlink.
# bin/cmux-herdr already adds repo root via Path(__file__).resolve().parent.parent
# which works for symlinks. Also drop a thin path hint file if needed later.

# Sidebar files are NOT installed. `sidebars/herdr.js` / `herdr.swift` are a
# legacy contrib fallback, not the product; the plugin manager mounts the
# sidebar from its own checkout.

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
echo "       cmux-herdr status"
echo "       cmux-herdr sync"
echo "       cmux-herdr watch"
echo "  3. Product install (users, and to mount the sidebar):"
echo "       cmux sidebar plugin install https://github.com/RaviTharuma/cmux-herdr.git"
echo "       cmux sidebar plugin use cmux-herdr"
echo "       cmux sidebar plugin update cmux-herdr"
echo "  4. Agents: skill installed as cmux-herdr (if skill dirs present)"
echo
echo "Contributor install done (CLI + skill; no sidebar files copied)."
