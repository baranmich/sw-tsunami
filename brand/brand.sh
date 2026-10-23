# shellcheck shell=bash
# Source this to get brand.toml's values as shell variables (used by the packaging scripts):
#   BRAND_NAME BRAND_FAMILY BRAND_PRODUCT BRAND_VENDOR BRAND_COPYRIGHT BRAND_WEBSITE
#   BRAND_BUNDLE_ID BRAND_DATA_DIR BRAND_SLUG
# Plain awk, so it runs with the stock macOS tools (no Python TOML parser needed).

_BRAND_TOML="${BRAND_TOML:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/brand.toml}"
[ -f "$_BRAND_TOML" ] || { echo "error: $_BRAND_TOML not found" >&2; return 1 2>/dev/null || exit 1; }

# brand_value <section> <key>: the quoted string value of `key = "…"` inside `[section]`.
brand_value() {
  awk -v want_sec="$1" -v want_key="$2" '
    /^[[:space:]]*\[/ { gsub(/[][[:space:]]/, ""); sec = $0; next }
    sec == want_sec {
      line = $0
      sub(/^[[:space:]]*/, "", line)
      if (index(line, want_key) != 1) next
      rest = substr(line, length(want_key) + 1)
      if (rest !~ /^[[:space:]]*=/) next
      sub(/^[[:space:]]*=[[:space:]]*"/, "", rest)
      sub(/"[[:space:]]*(#.*)?$/, "", rest)
      print rest
      exit
    }' "$_BRAND_TOML"
}

BRAND_FAMILY="$(brand_value brand family)"
BRAND_PRODUCT="$(brand_value brand product)"
BRAND_NAME="$(brand_value brand name)"
[ -n "$BRAND_NAME" ] || BRAND_NAME="$BRAND_FAMILY $BRAND_PRODUCT"
BRAND_VENDOR="$(brand_value brand vendor)"
BRAND_COPYRIGHT="$(brand_value brand copyright)"
BRAND_WEBSITE="$(brand_value brand website)"
BRAND_BUNDLE_ID="$(brand_value identity bundle_id)"
BRAND_DATA_DIR="$(brand_value identity data_dir)"
BRAND_SLUG="$(brand_value identity slug)"
export BRAND_NAME BRAND_FAMILY BRAND_PRODUCT BRAND_VENDOR BRAND_COPYRIGHT BRAND_WEBSITE \
  BRAND_BUNDLE_ID BRAND_DATA_DIR BRAND_SLUG

if [ -z "$BRAND_BUNDLE_ID" ] || [ -z "$BRAND_SLUG" ] || [ -z "${BRAND_NAME// /}" ]; then
  echo "error: brand.toml needs a name (family + product), identity.bundle_id and identity.slug" >&2
  return 1 2>/dev/null || exit 1
fi
