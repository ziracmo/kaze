#!/usr/bin/env bash
# Packs target/release/Kaze.app (see bundle-macos.sh) into a drag-to-install disk image.
# Usage: scripts/dmg-macos.sh <output.dmg>
set -euo pipefail
cd "$(dirname "$0")/.."

OUT=${1:-target/release/Kaze.dmg}
STAGE=target/release/dmg
rm -rf "$STAGE" "$OUT"
mkdir -p "$STAGE" "$(dirname "$OUT")"
cp -R target/release/Kaze.app "$STAGE/"
ln -s /Applications "$STAGE/Applications"
hdiutil create -volname Kaze -srcfolder "$STAGE" -ov -format UDZO "$OUT"
echo "Built $OUT"
