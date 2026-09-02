#!/usr/bin/env bash
# Reverse the cmux-herdr contributor install (CLI symlink + agent skill).
# Also sweeps legacy ~/.config/cmux/sidebars copies left by older versions.
# The product install is removed with: cmux sidebar plugin remove cmux-herdr
set -euo pipefail

LOCAL_BIN="${HOME}/.local/bin"
TARGET="${LOCAL_BIN}/cmux-herdr"
LEGACY_SIDEBAR_JS="${HOME}/.config/cmux/sidebars/herdr.js"
LEGACY_SIDEBAR_SWIFT="${HOME}/.config/cmux/sidebars/herdr.swift"

echo "cmux-herdr contributor uninstall"

# Clear cmux status pills if CLI still available
if command -v cmux-herdr >/dev/null 2>&1; then
  cmux-herdr clear 2>/dev/null || true
elif [[ -x "${TARGET}" ]]; then
  "${TARGET}" clear 2>/dev/null || true
fi

if [[ -L "${TARGET}" || -f "${TARGET}" ]]; then
  rm -f "${TARGET}"
  echo "  removed ${TARGET}"
else
  echo "  cli not found at ${TARGET}"
fi

for sidebar in "${LEGACY_SIDEBAR_JS}" "${LEGACY_SIDEBAR_SWIFT}"; do
  if [[ -f "${sidebar}" ]]; then
    rm -f "${sidebar}"
    echo "  removed legacy sidebar copy ${sidebar}"
  fi
done

for d in \
  "${HOME}/.agents/skills/cmux-herdr" \
  "${HOME}/.pi/agent/skills/cmux-herdr"
do
  if [[ -d "${d}" ]]; then
    rm -rf "${d}"
    echo "  removed ${d}"
  fi
done

echo "Done. (repo at source location left intact)"
echo "Product install: cmux sidebar plugin remove cmux-herdr"
