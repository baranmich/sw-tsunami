#!/usr/bin/env bash
# Render every app icon of the fork from brand/icon/icon.svg.
#
# Needs resvg (brew install resvg / cargo install resvg) and Python 3 with Pillow (for the .icns,
# so this runs on any OS). Outputs are committed; builds and packaging never need these tools.
#
#   brand/icons.sh
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIR="$HERE/icon"
SVG="$DIR/icon.svg"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

command -v resvg >/dev/null || { echo "error: resvg not found (brew install resvg)" >&2; exit 1; }

# The artwork is a full-bleed 512 tile. macOS icons sit on Apple's grid: an 824/1024 body with a
# transparent margin, so widen the viewBox to 512*1024/824.
MAC="$TMP/mac.svg"
sed 's/viewBox="0 0 512 512"/viewBox="-62.136 -62.136 636.272 636.272"/' "$SVG" >"$MAC"
grep -q 'viewBox="-62.136' "$MAC" || { echo "error: expected viewBox=\"0 0 512 512\" in $SVG" >&2; exit 1; }

render() { resvg -w "$2" -h "$2" "$1" "$3" </dev/null; }

render "$SVG" 1024 "$DIR/icon-1024.png"
render "$SVG" 256 "$DIR/icon-256.png"
# Runtime window/Dock icon on macOS (embedded by apps/wordcraft).
render "$MAC" 512 "$DIR/icon-macos-512.png"
render "$MAC" 1024 "$TMP/mac-1024.png"

# macOS .icns (Pillow writes every size from the 1024 master).
python3 -I - "$TMP/mac-1024.png" "$DIR/icon.icns" <<'EOF'
import sys
from PIL import Image
Image.open(sys.argv[1]).convert("RGBA").save(sys.argv[2], format="ICNS")
EOF
echo "icons written to $DIR"
