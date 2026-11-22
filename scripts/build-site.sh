#!/usr/bin/env bash
# Puts the website together: site/ with the pictures from docs/images and the
# app icon, in _site or the folder given. The Website workflow publishes the
# result and the Lint workflow checks its links. Preview it with
#   bash scripts/build-site.sh && python3 -m http.server -d _site
set -euo pipefail

cd "$(dirname "$0")/.."
out="${1:-_site}"
if [ -z "$out" ] || [ "$out" = "/" ] || [ "$out" = "." ]; then
  echo "refusing to replace '$out'" >&2
  exit 1
fi

rm -rf "$out"
mkdir -p "$out/images"
cp -r site/. "$out/"
cp docs/images/demo.gif docs/images/karaoke.gif docs/images/settings.png docs/images/social-preview.png \
  docs/images/layout-caption.png docs/images/layout-focus.png docs/images/layout-context.png "$out/images/"
cp apps/desktop/design/app-icon.svg "$out/images/"
cp apps/desktop/src-tauri/icons/128x128@2x.png "$out/images/app-icon.png"
