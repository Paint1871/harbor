#!/usr/bin/env bash
# Download each vendor's own logo into Harbor's runtime icon directory.
#
# Harbor ships original marks for every engine and does NOT redistribute vendor
# trademarks. This script fetches the vendors' artwork onto *your* machine, into
# a directory outside the repository, where Harbor picks it up at runtime. The
# files are never committed. Whether your use of a vendor's mark is permitted is
# between you and that vendor's brand guidelines.
#
# Engines with no verified logo source keep Harbor's own drawn mark.
set -euo pipefail

case "$(uname -s)" in
  Darwin) dir="$HOME/Library/Application Support/harbor/engine-icons" ;;
  Linux)  dir="${XDG_DATA_HOME:-$HOME/.local/share}/harbor/engine-icons" ;;
  *)      dir="${APPDATA:-$HOME}/harbor/engine-icons" ;;
esac
mkdir -p "$dir"

# engine-id|url  — each source was checked to return that vendor's own mark.
sources=(
  "claude-code|https://claude.com/favicon.svg"
  "codex|https://developers.openai.com/favicon.svg"
  "cursor|https://cursor.com/favicon.svg"
  "opencode|https://opencode.ai/favicon.svg"
  "copilot|https://github.githubassets.com/images/modules/site/copilot/copilot.png"
  "gemini|https://raw.githubusercontent.com/google-gemini/gemini-cli/main/packages/vscode-ide-companion/assets/icon.png"
  "aider|https://aider.chat/assets/icons/favicon-32x32.png"
  "grok-build|https://x.ai/favicon.ico"
  "kimi-code|https://www.kimi.com/favicon.ico"
  "muse-code|https://www.meta.ai/favicon.ico"
  "amp|https://ampcode.com/favicon.ico"
  "factory|https://factory.ai/favicon.ico"
  "droid|https://app.factory.ai/favicon.ico"
  "antigravity|https://antigravity.google/favicon.ico"
)

# Some vendor edges refuse a bare curl; identify as a browser rather than
# being blocked by bot protection.
agent="Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"

ok=0
skipped=0
for entry in "${sources[@]}"; do
  id="${entry%%|*}"
  url="${entry#*|}"
  tmp="$(mktemp)"
  if ! curl -fsSL --compressed --max-time 20 -A "$agent" -o "$tmp" "$url"; then
    printf '  skip  %-14s download failed\n' "$id"
    rm -f "$tmp"
    skipped=$((skipped + 1))
    continue
  fi
  # A site that answers 200 with an HTML shell is not a logo.
  kind="$(file -b --mime-type "$tmp")"
  case "$kind" in
    image/svg+xml|image/svg) ext=svg ;;
    image/png) ext=png ;;
    image/webp) ext=webp ;;
    image/x-icon|image/vnd.microsoft.icon)
      if command -v sips >/dev/null 2>&1; then
        sips -s format png "$tmp" --out "$tmp.png" >/dev/null 2>&1 || true
      fi
      if [ -s "$tmp.png" ]; then
        mv "$tmp.png" "$tmp"
        ext=png
      else
        printf '  skip  %-14s .ico needs conversion (install sips or convert by hand)\n' "$id"
        rm -f "$tmp"
        skipped=$((skipped + 1))
        continue
      fi
      ;;
    *)
      printf '  skip  %-14s served %s, not an image\n' "$id" "$kind"
      rm -f "$tmp"
      skipped=$((skipped + 1))
      continue
      ;;
  esac
  rm -f "$dir/$id".svg "$dir/$id".png "$dir/$id".webp
  mv "$tmp" "$dir/$id.$ext"
  printf '  ok    %-14s %s\n' "$id" "$id.$ext"
  ok=$((ok + 1))
done

cat > "$dir/PROVENANCE.md" <<EOF
# Engine logo provenance

Downloaded by \`scripts/fetch-engine-logos.sh\` on $(date -u +%Y-%m-%dT%H:%M:%SZ).

Each file is the vendor's own artwork, fetched from the vendor's own domain, and
is that vendor's trademark. These files are not part of the Harbor repository
and are not redistributed with Harbor. Delete this directory to fall back to
Harbor's original marks.

$(for entry in "${sources[@]}"; do printf -- '- %s: %s\n' "${entry%%|*}" "${entry#*|}"; done)

Notes on two of these: muse-code is Meta's CLI (it updates from api.meta.ai), so
the Meta AI mark stands in for it; no Muse-specific public logo was found. The
gemini entry is the Gemini CLI's own product icon from Google's repository
rather than the generic Gemini sparkle.
EOF

printf '\n%d installed, %d skipped\n%s\n' "$ok" "$skipped" "$dir"
