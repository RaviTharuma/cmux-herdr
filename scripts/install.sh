#!/usr/bin/env bash
# Contributor install only; never adopt artifacts merely because their names match.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/scripts/packaging-manifest.sh"
packaging_init
[[ -x "$ROOT/bin/cmux-herdr" && -x "$ROOT/bin/cmux-herdr-fetch" ]] || packaging_fail 'missing launcher/bootstrap'
"$ROOT/bin/cmux-herdr-fetch"
packaging_install link "$ROOT/bin/cmux-herdr" "$HOME/.local/bin/cmux-herdr"
for dest in "$HOME/.agents/skills/cmux-herdr" "$HOME/.pi/agent/skills/cmux-herdr"; do
  if [[ -e "$dest" || -L "$dest" ]]; then
    owned=0
    for recorded in "${destinations[@]}"; do
      case "$recorded" in "$dest/"*) owned=1 ;; esac
    done
    if [[ "$owned" == 0 ]]; then
      echo "preserved unowned skill directory: $dest"
      continue
    fi
  fi
  # Enumerate only regular source files, without following source symlinks.
  while IFS= read -r -d '' file; do
    packaging_install file "$file" "$dest/${file#"$ROOT/agent-skill/"}"
  done < <(find "$ROOT/agent-skill" -type f -print0)
done
packaging_save
echo 'cmux-herdr contributor install complete; unowned or modified files preserved.'
