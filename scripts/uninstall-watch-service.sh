#!/usr/bin/env bash
# Remove the cmux-herdr watch LaunchAgent if present.
set -euo pipefail

LABEL="com.cmux-herdr.watch"
PLIST_DST="${HOME}/Library/LaunchAgents/${LABEL}.plist"
UID_NUM="$(id -u)"
DOMAIN="gui/${UID_NUM}"

echo "cmux-herdr watch service uninstall"

if launchctl print "${DOMAIN}/${LABEL}" >/dev/null 2>&1; then
  launchctl bootout "${DOMAIN}/${LABEL}" 2>/dev/null || true
  echo "  bootout: ${DOMAIN}/${LABEL}"
elif [[ -f "${PLIST_DST}" ]]; then
  launchctl unload "${PLIST_DST}" 2>/dev/null || true
  echo "  unload: ${PLIST_DST}"
else
  echo "  (not loaded)"
fi

if [[ -f "${PLIST_DST}" ]]; then
  rm -f "${PLIST_DST}"
  echo "  removed: ${PLIST_DST}"
else
  echo "  plist already absent"
fi

echo "done (logs under ~/Library/Logs/cmux-herdr-watch.* left in place)"
