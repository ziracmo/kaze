#!/usr/bin/env bash
# Builds Kaze.app (ad-hoc signed) and installs it in /Applications.
# Usage: scripts/bundle-macos.sh              → build + install in /Applications
#        scripts/bundle-macos.sh --no-install → build only (target/release/Kaze.app)
#        scripts/bundle-macos.sh --universal  → Apple silicon + Intel binary (implies --no-install)
set -euo pipefail
cd "$(dirname "$0")/.."

INSTALL=1
UNIVERSAL=0
for arg in "$@"; do
  case "$arg" in
    --no-install) INSTALL=0 ;;
    --universal) UNIVERSAL=1 INSTALL=0 ;;
  esac
done

APP=target/release/Kaze.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

if [[ $UNIVERSAL == 1 ]]; then
  for t in aarch64-apple-darwin x86_64-apple-darwin; do
    cargo build --release --locked -p kaze-app --target "$t"
  done
  lipo -create -output "$APP/Contents/MacOS/Kaze" \
    target/aarch64-apple-darwin/release/Kaze target/x86_64-apple-darwin/release/Kaze
else
  cargo build --release -p kaze-app
  cp target/release/Kaze "$APP/Contents/MacOS/Kaze"
fi

# App icon: every size macOS wants, from the 1024 px master.
ICONSET=target/release/Kaze.iconset
rm -rf "$ICONSET" && mkdir -p "$ICONSET"
SRC=crates/kaze-app/assets/icon/kaze.png
for size in 16 32 128 256 512; do
  sips -z $size $size "$SRC" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  sips -z $((size * 2)) $((size * 2)) "$SRC" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/Kaze.icns"

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Kaze</string>
  <key>CFBundleDisplayName</key><string>Kaze</string>
  <key>CFBundleIdentifier</key><string>dev.kaze</string>
  <key>CFBundleExecutable</key><string>Kaze</string>
  <key>CFBundleIconFile</key><string>Kaze</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# Ad-hoc signature so Gatekeeper lets a locally built app run.
codesign --force --deep --sign - "$APP"

if [[ $INSTALL == 1 ]]; then
  rm -rf /Applications/Kaze.app
  cp -R "$APP" /Applications/
  # Refresh the Dock / Launchpad icon cache for the new bundle.
  touch /Applications/Kaze.app
  echo "Installé : /Applications/Kaze.app (Launchpad, Spotlight, Dock)"
else
  echo "Built $APP"
fi
