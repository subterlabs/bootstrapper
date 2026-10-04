#!/bin/bash

set -euo pipefail
cd "$(dirname "$0")"

APP_NAME="Subter Launcher"
EXECUTABLE="SubterLauncher"
BUNDLE_ID="org.subter.launcher"
VERSION="$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)"
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
OUT="$TARGET_DIR/macos"
APP="$OUT/$APP_NAME.app"

rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
lipo -create \
    "$TARGET_DIR/aarch64-apple-darwin/release/syntax_bootstrapper" \
    "$TARGET_DIR/x86_64-apple-darwin/release/syntax_bootstrapper" \
    -output "$APP/Contents/MacOS/$EXECUTABLE"

# App icon from the Windows .ico
ICONSET="$OUT/AppIcon.iconset"
rm -rf "$ICONSET"
mkdir -p "$ICONSET"
sips -s format png assets/Bootstrapper.ico --out "$OUT/icon.png" >/dev/null
for size in 16 32 128 256; do
    sips -z $size $size "$OUT/icon.png" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    [ $double -le 256 ] && sips -z $double $double "$OUT/icon.png" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"
rm -rf "$ICONSET" "$OUT/icon.png"

cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key>
	<string>$APP_NAME</string>
	<key>CFBundleDisplayName</key>
	<string>$APP_NAME</string>
	<key>CFBundleIdentifier</key>
	<string>$BUNDLE_ID</string>
	<key>CFBundleExecutable</key>
	<string>$EXECUTABLE</string>
	<key>CFBundleIconFile</key>
	<string>AppIcon</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleVersion</key>
	<string>$VERSION</string>
	<key>CFBundleShortVersionString</key>
	<string>$VERSION</string>
	<key>LSMinimumSystemVersion</key>
	<string>11.0</string>
	<key>LSUIElement</key>
	<true/>
	<key>CFBundleURLTypes</key>
	<array>
		<dict>
			<key>CFBundleURLName</key>
			<string>Subter Player</string>
			<key>CFBundleURLSchemes</key>
			<array>
				<string>subter-player</string>
			</array>
		</dict>
	</array>
</dict>
</plist>
EOF

xattr -cr "$APP"
codesign --force --sign - "$APP"

rm -f "$OUT/SubterMacLauncher.zip"
ditto -c -k --norsrc --noextattr --keepParent "$APP" "$OUT/SubterMacLauncher.zip"

CHECK="$(mktemp -d)"
ditto -x -k "$OUT/SubterMacLauncher.zip" "$CHECK"
codesign --verify --strict --deep "$CHECK/$APP_NAME.app"
rm -rf "$CHECK"
echo "Built $APP and $OUT/SubterMacLauncher.zip"
