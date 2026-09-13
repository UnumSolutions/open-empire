#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo build --locked -p empire-client
app="target/Open Empire.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/debug/empire-client "$app/Contents/MacOS/OpenEmpire"
cat > "$app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Open Empire</string>
<key>CFBundleDisplayName</key><string>Open Empire</string>
<key>CFBundleIdentifier</key><string>org.openempire.prototype</string>
<key>CFBundleVersion</key><string>1</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>CFBundleExecutable</key><string>OpenEmpire</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>NSHighResolutionCapable</key><true/>
<key>LSMinimumSystemVersion</key><string>12.0</string>
</dict></plist>
PLIST
printf 'Unsigned development app: %s\n' "$app"
