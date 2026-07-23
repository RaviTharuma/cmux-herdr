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

# Rewrite user-specific paths from the sample template.
python3 - "${PLIST_SRC}" "${PLIST_DST}" "${HOME}" <<'PY'
import sys
from pathlib import Path

src, dst, home = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]
text = src.read_text()
text = text.replace("/Users/PLACEHOLDER", home)
# Drop XML comment block noise is fine; keep as-is.
dst.write_text(text)
print(f"  wrote {dst}")
PY

# Prefer modern bootstrap; fall back to load.
if launchctl print "${DOMAIN}/${LABEL}" >/dev/null 2>&1; then
  launchctl bootout "${DOMAIN}/${LABEL}" 2>/dev/null || \
    launchctl unload "${PLIST_DST}" 2>/dev/null || true
fi

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
