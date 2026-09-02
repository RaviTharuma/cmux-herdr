#!/usr/bin/env bash
# Reverse cmux-herdr install.
set -euo pipefail

LOCAL_BIN="${HOME}/.local/bin"
TARGET="${LOCAL_BIN}/cmux-herdr"
SIDEBAR_JS_DST="${HOME}/.config/cmux/sidebars/herdr.js"
SIDEBAR_SWIFT_DST="${HOME}/.config/cmux/sidebars/herdr.swift"

echo "cmux-herdr plugin uninstall"

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

for sidebar in "${SIDEBAR_JS_DST}" "${SIDEBAR_SWIFT_DST}"; do
  if [[ -f "${sidebar}" ]]; then
    rm -f "${sidebar}"
    echo "  removed ${sidebar}"
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
