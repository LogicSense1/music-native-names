#!/bin/zsh
set -euo pipefail
cd "${0:A:h}"
[[ "$(uname -m)" == arm64 ]] || { print -u2 'This release build requires an Apple Silicon Mac.'; exit 1; }
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}"
export MACOSX_DEPLOYMENT_TARGET=14.0
./scripts/test.sh
cargo build --locked --release
version="$(./scripts/version.sh)"
app="$PWD/dist/Music Native Names.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$CARGO_TARGET_DIR/release/music-native-names" "$app/Contents/MacOS/music-native-names"
chmod 755 "$app/Contents/MacOS/music-native-names"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>local.music-native-names.rust</string>
<key>CFBundleName</key><string>Music Native Names</string>
<key>CFBundleDisplayName</key><string>Music Native Names</string>
<key>CFBundleExecutable</key><string>music-native-names</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>$version</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>LSUIElement</key><true/>
<key>NSHighResolutionCapable</key><true/>
<key>LSMinimumSystemVersion</key><string>14.0</string>
</dict></plist>
PLIST
/usr/bin/codesign --force --sign - "$app"
/usr/bin/codesign --verify --strict "$app"
python3 scripts/check_release.py --stamp
print "Built $version: $app"
