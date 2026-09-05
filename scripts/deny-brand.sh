#!/usr/bin/env bash
# Scan the worktree without dumping matching content into CI logs.
set -euo pipefail

for tool in git rg; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    printf 'deny-brand: required tool missing: %s\n' "$tool" >&2
    exit 2
  fi
done

cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
git rev-parse --is-inside-work-tree >/dev/null

# Split spellings keep the guard itself subject to the same scan as ship code.
pattern='bridge(mind|space|voice|agent|mcp|shot|swarm|bench)|cog''nito|price_[[:alnum:]]+|agent[[:space:]-]+super[[:space:]-]+app'
file_list=$(mktemp)
trap 'rm -f -- "$file_list"' EXIT
git ls-files --cached --others --exclude-standard -z > "$file_list"
failed=0

check_text() {
  local status=0
  rg --no-config --text --ignore-case --quiet -- "$pattern" || status=$?
  case "$status" in
    0) return 0 ;;
    1) return 1 ;;
    *) printf 'deny-brand: scanner error\n' >&2; exit 2 ;;
  esac
}

while IFS= read -r -d '' file; do
  case "$file" in
    CLEANROOM.md|docs-src/references.md|DESIGN.md) continue ;;
  esac
  # A deleted tracked file is no longer part of the working tree.
  if [[ ! -e "$file" && ! -L "$file" ]]; then
    continue
  fi
  if check_text <<< "$file"; then
    printf 'deny-brand: forbidden file name: %q\n' "$file" >&2
    failed=1
  fi
  if [[ -L "$file" ]]; then
    # Check the link itself, never read data outside the repository through it.
    link_target=$(readlink -- "$file")
    if check_text <<< "$link_target"; then
      printf 'deny-brand: forbidden symlink target: %q\n' "$file" >&2
      failed=1
    fi
  elif [[ -f "$file" ]]; then
    if [[ ! -r "$file" ]]; then
      printf 'deny-brand: unreadable file: %q\n' "$file" >&2
      exit 2
    fi
    if check_text < "$file"; then
      printf 'deny-brand: forbidden content in: %q\n' "$file" >&2
      failed=1
    fi
  else
    printf 'deny-brand: cannot scan entry: %q\n' "$file" >&2
    exit 2
  fi
done < "$file_list"

if [[ "$failed" -ne 0 ]]; then
  printf 'deny-brand: failed; see CLEANROOM.md for policy\n' >&2
  exit 1
fi
printf 'deny-brand: clean\n'
