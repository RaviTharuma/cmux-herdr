#!/usr/bin/env bash
# Install LaunchAgent that runs `cmux-herdr watch` in the background.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LABEL="com.cmux-herdr.watch"
PLIST_SRC="${ROOT}/scripts/${LABEL}.plist"
PLIST_DST="${HOME}/Library/LaunchAgents/${LABEL}.plist"
LOG_DIR="${HOME}/Library/Logs"
CLI="${HOME}/.local/bin/cmux-herdr"
UID_NUM="$(id -u)"
DOMAIN="gui/${UID_NUM}"

if [[ ! -f "${PLIST_SRC}" ]]; then
  echo "error: missing ${PLIST_SRC}" >&2
  exit 1
fi

if [[ ! -x "${CLI}" && ! -L "${CLI}" ]]; then
  echo "error: ${CLI} not found. Run ./scripts/install.sh first." >&2
  exit 1
fi

mkdir -p "${HOME}/Library/LaunchAgents" "${LOG_DIR}"

# Rewrite user-specific paths from the sample template without requiring Python.
ESCAPED_HOME="$(printf '%s' "${HOME}" | sed 's/[\\&|]/\\&/g')"
TEMP_PLIST="$(mktemp "${PLIST_DST}.XXXXXX")"
trap 'rm -f "${TEMP_PLIST}"' EXIT HUP INT TERM
sed "s|/Users/PLACEHOLDER|${ESCAPED_HOME}|g" "${PLIST_SRC}" > "${TEMP_PLIST}"
chmod 600 "${TEMP_PLIST}"
mv -f "${TEMP_PLIST}" "${PLIST_DST}"
trap - EXIT HUP INT TERM
echo "  wrote ${PLIST_DST}"

# Boot out this plist only. Tests and alternate HOME installs must not evict a
# same-label agent loaded from another path in the real user domain.
launchctl bootout "${DOMAIN}" "${PLIST_DST}" 2>/dev/null || \
  launchctl unload "${PLIST_DST}" 2>/dev/null || true

if launchctl bootstrap "${DOMAIN}" "${PLIST_DST}" 2>/dev/null; then
  launchctl enable "${DOMAIN}/${LABEL}" 2>/dev/null || true
  echo "  loaded: ${DOMAIN}/${LABEL} (bootstrap)"
elif launchctl load "${PLIST_DST}" 2>/dev/null; then
  echo "  loaded: ${PLIST_DST} (load)"
else
  echo "error: failed to load LaunchAgent. Try:" >&2
  echo "  launchctl bootstrap ${DOMAIN} ${PLIST_DST}" >&2
  exit 1
fi

echo "cmux-herdr watch service installed"
echo "  label : ${LABEL}"
echo "  plist : ${PLIST_DST}"
echo "  logs  : ${LOG_DIR}/cmux-herdr-watch.{out,err}.log"
echo "  stop  : ./scripts/uninstall-watch-service.sh"
echo
echo "Note: watch needs Herdr + cmux context. If pills do not update, run"
echo "  cmux-herdr status"
echo "from a nested Herdr pane and check the err log."
