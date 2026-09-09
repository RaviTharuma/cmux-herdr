#!/usr/bin/env bash
# Remove only unchanged, manifest-owned files. Never invoke host CLIs or services.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/scripts/packaging-manifest.sh"
packaging_init
for ((i=${#destinations[@]}-1; i>=1; i--)); do
  if packaging_matches "${kinds[i]}" "${values[i]}" "${destinations[i]}"; then
    dest="${destinations[i]}"
    packaging_begin "$dest"
    rm "${destinations[i]}"
    kinds[i]=''; values[i]=''; destinations[i]=''
    packaging_commit
    packaging_prune "$dest"
  else
    echo "preserved unowned or modified: ${destinations[i]}"
  fi
done
packaging_save
echo 'cmux-herdr uninstall complete; repository, user edits and foreign files preserved.'
