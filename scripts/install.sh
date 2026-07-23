#!/usr/bin/env bash
# Install cmux-herdr into the user environment (no root, no cmux upstream PR).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_SRC="${ROOT}/bin/cmux-herdr"
LOCAL_BIN="${HOME}/.local/bin"
TARGET="${LOCAL_BIN}/cmux-herdr"
SIDEBAR_SRC="${ROOT}/sidebars/herdr.swift"
SIDEBAR_DST_DIR="${HOME}/.config/cmux/sidebars"
SIDEBAR_DST="${SIDEBAR_DST_DIR}/herdr.swift"
SKILL_SRC="${ROOT}/agent-skill"

echo "cmux-herdr install"
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

if [[ -f "${SIDEBAR_SRC}" ]]; then
  mkdir -p "${SIDEBAR_DST_DIR}"
  cp "${SIDEBAR_SRC}" "${SIDEBAR_DST}"
  echo "  sidebar: ${SIDEBAR_DST}"
else
  echo "  sidebar: (skipped, source missing)"
fi

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
echo "  2. Inside a herdr pane nested in cmux, run:"
echo "       cmux-herdr status"
echo "       cmux-herdr sync"
echo "       cmux-herdr watch          # background mirror every 3s"
echo "  3. Enable custom sidebars in cmux Settings → Beta features,"
echo "     then: cmux sidebar reload && cmux sidebar validate herdr"
echo "  4. Agents: skill installed as cmux-herdr (if skill dirs present)"
echo
echo "Done."
