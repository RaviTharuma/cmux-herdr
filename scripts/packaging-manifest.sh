#!/usr/bin/env bash
# Shared by contributor install/uninstall; Bash 3.2 compatible.
packaging_fail() { printf 'cmux-herdr packaging: %s\n' "$*" >&2; exit 1; }
packaging_safe_parents() {
  local p="${1%/*}"
  while [[ "$p" != / && -n "$p" ]]; do
    [[ ! -L "$p" ]] || return 1
    p="${p%/*}"
  done
}
packaging_hash() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum < "$1" | cut -d ' ' -f 1
  else shasum -a 256 < "$1" | cut -d ' ' -f 1; fi
}
packaging_allowed() {
  case "$1" in
    "$HOME/.local/bin/cmux-herdr"|"$HOME/.agents/skills/cmux-herdr/"*|"$HOME/.pi/agent/skills/cmux-herdr/"*) ;;
    *) return 1 ;;
  esac
  case "$1" in *$'\t'*|*$'\n'*|*/../*|*/./*) return 1 ;; esac
}
packaging_matches() {
  packaging_safe_parents "$3" || return 1
  case "$1" in
    link) [[ -L "$3" && $(readlink "$3") == "$2" ]] ;;
    file) [[ -f "$3" && ! -L "$3" && $(packaging_hash "$3") == "$2" ]] ;;
    *) return 1 ;;
  esac
}
# Prune only empty skill ancestors of a file this operation owned.
packaging_prune() {
  local p="${1%/*}"
  while [[ "$p" == "$HOME/.agents/skills/cmux-herdr" || "$p" == "$HOME/.agents/skills/cmux-herdr/"* || "$p" == "$HOME/.pi/agent/skills/cmux-herdr" || "$p" == "$HOME/.pi/agent/skills/cmux-herdr/"* ]]; do
    rmdir "$p" 2>/dev/null || break
    p="${p%/*}"
  done
}
packaging_cleanup() {
  local status=$?
  trap - EXIT
  if [[ -n "${pending_dest:-}" ]]; then
    if [[ -e "$pending_backup/original" || -L "$pending_backup/original" ]]; then
      mv -f "$pending_backup/original" "$pending_dest" || {
        printf 'cmux-herdr packaging: rollback failed; original retained at %s\n' "$pending_backup/original" >&2
        exit 1
      }
    else
      rm -f "$pending_dest" || status=1
      packaging_prune "$pending_dest"
    fi
  fi
  if [[ -n "${pending_backup:-}" ]]; then
    rm -f "$pending_backup/original"
    rmdir "$pending_backup" || status=1
  fi
  [[ -z "${manifest_temp:-}" ]] || rm -f "$manifest_temp"
  rmdir "$STATE/lock" || status=1
  exit "$status"
}
packaging_begin() {
  local dest="$1"
  pending_backup=$(mktemp -d "${dest%/*}/.cmux-herdr-rollback.XXXXXX")
  if [[ -e "$dest" || -L "$dest" ]]; then
    cp -pP "$dest" "$pending_backup/original"
  fi
  pending_dest="$dest"
}
packaging_commit() {
  packaging_save
  pending_dest=''
  rm -f "$pending_backup/original"
  rmdir "$pending_backup"
  pending_backup=''
}
packaging_init() {
  case "${XDG_STATE_HOME:-}" in /*) STATE="$XDG_STATE_HOME" ;; *) STATE="$HOME/.local/state" ;; esac
  STATE="$STATE/cmux-herdr/packaging"
  MANIFEST="$STATE/manifest.tsv"
  packaging_safe_parents "$MANIFEST" || packaging_fail 'symlinked state directory'
  [[ ! -L "$MANIFEST" ]] || packaging_fail 'symlinked manifest'
  [[ ! -e "$MANIFEST" || -f "$MANIFEST" ]] || packaging_fail 'manifest is not a regular file'
  mkdir -p "$STATE"
  mkdir "$STATE/lock" 2>/dev/null || packaging_fail "packaging lock exists: $STATE/lock; after confirming no installer is running, remove that empty directory and retry"
  pending_dest='' pending_backup='' manifest_temp=''
  trap packaging_cleanup EXIT
  # Sentinel keeps arrays nonempty under macOS Bash 3.2 nounset semantics.
  kinds=('') values=('') destinations=('')
  local kind value dest
  if [[ -f "$MANIFEST" ]]; then
    while IFS=$'\t' read -r kind value dest; do
      packaging_allowed "$dest" || packaging_fail 'invalid manifest destination'
      case "$kind" in file|link) ;; *) packaging_fail 'invalid manifest type' ;; esac
      kinds+=("$kind"); values+=("$value"); destinations+=("$dest")
    done < "$MANIFEST"
  fi
}
packaging_save() {
  local i
  manifest_temp=$(mktemp "$STATE/manifest.XXXXXX")
  for ((i=1; i<${#destinations[@]}; i++)); do
    [[ -n "${destinations[i]}" ]] || continue
    printf '%s\t%s\t%s\n' "${kinds[i]}" "${values[i]}" "${destinations[i]}" >> "$manifest_temp"
  done
  mv -f "$manifest_temp" "$MANIFEST"
  manifest_temp=''
}
packaging_install() {
  local kind="$1" source="$2" dest="$3" value i index=-1 tmp
  packaging_allowed "$dest" || packaging_fail "invalid destination: $dest"
  packaging_safe_parents "$dest" || { echo "preserved symlinked parent: $dest"; return; }
  for ((i=1; i<${#destinations[@]}; i++)); do
    [[ "${destinations[i]}" != "$dest" ]] || index=$i
  done
  if [[ -e "$dest" || -L "$dest" ]]; then
    if [[ "$index" -lt 0 ]] || ! packaging_matches "${kinds[index]}" "${values[index]}" "$dest"; then
      echo "preserved unowned or modified: $dest"; return
    fi
  fi
  mkdir -p "${dest%/*}"
  packaging_begin "$dest"
  if [[ "$kind" == link ]]; then
    value="$source"
    [[ "$value" != *$'\t'* && "$value" != *$'\n'* ]] || packaging_fail 'invalid symlink target'
    [[ ! -L "$dest" ]] || rm "$dest"
    ln -s "$source" "$dest"
  else
    value=$(packaging_hash "$source")
    tmp=$(mktemp "${dest%/*}/.cmux-herdr.XXXXXX")
    cp "$source" "$tmp"
    mv -f "$tmp" "$dest"
  fi
  if [[ "$index" -lt 0 ]]; then index=${#destinations[@]}; fi
  kinds[index]="$kind"; values[index]="$value"; destinations[index]="$dest"
  packaging_commit
}
