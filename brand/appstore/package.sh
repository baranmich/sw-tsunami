#!/usr/bin/env bash
# Fork: build the Mac App Store package: "<Name>.app", sandboxed, signed for the App Store,
# wrapped in a signed installer package ready to upload to App Store Connect.
#
#   brand/appstore/package.sh [--skip-build]      → $DIST/<slug>-<version>-appstore.pkg
#
# Needs (env):
#   APPLE_TEAM_ID           Team ID (10 characters)
#   MAS_APP_IDENTITY        signing identity for the app: "Apple Distribution: …" (or its SHA-1)
#   MAS_INSTALLER_IDENTITY  signing identity for the .pkg: "3rd Party Mac Developer Installer: …"
#                           (shown as "Mac Installer Distribution" in the developer portal)
#   MAS_PROFILE             path to the Mac App Store provisioning profile for brand.toml's bundle id
#   MACOS_KEYCHAIN          keychain holding the identities (optional)
#   BUILD_NUMBER            CFBundleVersion; must grow with every upload (default: git commit count)
# Optional upload (App Store Connect API key): ASC_KEY_ID, ASC_ISSUER_ID and the key file in
# ~/.appstoreconnect/private_keys/AuthKey_<ASC_KEY_ID>.p8 (or ~/private_keys). Without them the
# .pkg is left for Transporter.app.
set -euo pipefail
# shellcheck source=../../packaging/env.sh
. "$(dirname "${BASH_SOURCE[0]}")/../../packaging/env.sh"
# shellcheck source=../brand.sh
. "$ROOT/brand/brand.sh"
HERE="$ROOT/brand/appstore"

SKIP_BUILD=0
case "${1:-}" in
  --skip-build) SKIP_BUILD=1 ;;
  "") ;;
  *) echo "usage: $0 [--skip-build]" >&2; exit 2 ;;
esac

for v in APPLE_TEAM_ID MAS_APP_IDENTITY MAS_INSTALLER_IDENTITY MAS_PROFILE; do
  [ -n "${!v:-}" ] || { echo "error: $v is not set (see the header of $0)" >&2; exit 1; }
done
[ -f "$MAS_PROFILE" ] || { echo "error: MAS_PROFILE $MAS_PROFILE not found" >&2; exit 1; }

export MACOSX_DEPLOYMENT_TARGET=11.0
SHORT_VERSION="${VERSION%%-*}"
BUILD_NUMBER="${BUILD_NUMBER:-$(git -C "$ROOT" rev-list --count HEAD 2>/dev/null || echo 1)}"
WORK="$CARGO_TARGET_DIR/appstore-package"
APP="$WORK/$BRAND_NAME.app"
PKG="$DIST/$BRAND_SLUG-$VERSION-appstore.pkg"
TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)
kc=()
[ -n "${MACOS_KEYCHAIN:-}" ] && kc=(--keychain "$MACOS_KEYCHAIN")

echo "==> $BRAND_NAME $VERSION ($BUILD_NUMBER) for the Mac App Store"

if [ "$SKIP_BUILD" = 0 ]; then
  args=()
  for t in "${TARGETS[@]}"; do args+=(--target "$t"); done
  (cd "$ROOT" && cargo build --release --locked -p wordcraft "${args[@]}")
fi

rm -rf "$WORK"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
inputs=()
for t in "${TARGETS[@]}"; do inputs+=("$CARGO_TARGET_DIR/$t/release/wordcraft"); done
lipo -create -output "$APP/Contents/MacOS/$BRAND_NAME" "${inputs[@]}"
cp "$ROOT/brand/icon/icon.icns" "$APP/Contents/Resources/$BRAND_NAME.icns"
copy_font_licences "$APP/Contents/Resources"
cp "$MAS_PROFILE" "$APP/Contents/embedded.provisionprofile"

# Info.plist: the DMG build's template with the brand, plus the App Store build number.
plist_value() { printf '%s' "$1" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g' -e 's/[\/&]/\\&/g'; }
sed -e "s/@VERSION@/$VERSION/g" -e "s/@SHORT_VERSION@/$SHORT_VERSION/g" \
  -e "s/@BUILD_SHA@/${WORDCRAFT_BUILD_SHA:-unknown}/g" \
  -e "s/@NAME@/$(plist_value "$BRAND_NAME")/g" -e "s/@BUNDLE_ID@/$(plist_value "$BRAND_BUNDLE_ID")/g" \
  -e "s/@COPYRIGHT@/$(plist_value "$BRAND_COPYRIGHT")/g" \
  "$ROOT/packaging/macos/Info.plist.in" >"$APP/Contents/Info.plist"
plutil -replace CFBundleVersion -string "$BUILD_NUMBER" "$APP/Contents/Info.plist"
plutil -lint "$APP/Contents/Info.plist"
printf 'APPL????' >"$APP/Contents/PkgInfo"

# Entitlements: the sandbox set, plus the app and team identifiers the profile is for.
ENT="$WORK/entitlements.plist"
cp "$HERE/entitlements.plist" "$ENT"
plutil -insert com.apple.application-identifier -string "$APPLE_TEAM_ID.$BRAND_BUNDLE_ID" "$ENT"
plutil -insert com.apple.developer.team-identifier -string "$APPLE_TEAM_ID" "$ENT"

xattr -cr "$APP"
codesign --force --sign "$MAS_APP_IDENTITY" ${kc[@]+"${kc[@]}"} --options runtime --entitlements "$ENT" "$APP/Contents/MacOS/$BRAND_NAME"
codesign --force --sign "$MAS_APP_IDENTITY" ${kc[@]+"${kc[@]}"} --options runtime --entitlements "$ENT" "$APP"
codesign --verify --strict --deep --verbose=2 "$APP"
codesign -d --entitlements - "$APP" | grep -q com.apple.security.app-sandbox || { echo "error: the app isn't sandboxed" >&2; exit 1; }

rm -f "$PKG"
productbuild --component "$APP" /Applications --sign "$MAS_INSTALLER_IDENTITY" ${kc[@]+"${kc[@]}"} "$PKG"
pkgutil --check-signature "$PKG"
echo "==> $PKG"

if [ -n "${ASC_KEY_ID:-}" ] && [ -n "${ASC_ISSUER_ID:-}" ]; then
  echo "==> validating and uploading to App Store Connect"
  xcrun altool --validate-app -f "$PKG" -t macos --apiKey "$ASC_KEY_ID" --apiIssuer "$ASC_ISSUER_ID"
  xcrun altool --upload-app -f "$PKG" -t macos --apiKey "$ASC_KEY_ID" --apiIssuer "$ASC_ISSUER_ID"
else
  echo "==> not uploaded (no ASC_KEY_ID / ASC_ISSUER_ID): upload $PKG with Transporter.app"
fi
