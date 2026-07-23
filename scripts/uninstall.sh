#!/usr/bin/env bash
# Reverse cmux-herdr install.
set -euo pipefail

LOCAL_BIN="${HOME}/.local/bin"
TARGET="${LOCAL_BIN}/cmux-herdr"
SIDEBAR_DST="${HOME}/.config/cmux/sidebars/herdr.swift"

echo "cmux-herdr uninstall"

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

if [[ -f "${SIDEBAR_DST}" ]]; then
  rm -f "${SIDEBAR_DST}"
  echo "  removed ${SIDEBAR_DST}"
fi

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
