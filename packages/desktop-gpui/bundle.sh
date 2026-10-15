#!/bin/sh
# Builds a release binary or example and wraps it in target/bundles/<app-name>.app,
# because cua_repl cannot attach to a bare binary.
# Usage: bundle.sh <bin|example> <target> <app-name> <bundle-id>
# Prints the bundle path.
set -eu
if [ $# -ne 4 ]; then
  echo "usage: bundle.sh <bin|example> <target> <app-name> <bundle-id>" >&2
  exit 2
fi
kind=$1
target=$2
name=$3
bundle_id=$4
cd "$(dirname "$0")"
case $kind in
  bin) ./cargo.sh build --release --bin "$target"; binary=target/release/$target ;;
  example) ./cargo.sh build --release --example "$target"; binary=target/release/examples/$target ;;
  *) echo "bundle.sh: kind must be bin or example, not $kind" >&2; exit 2 ;;
esac
app=target/bundles/$name.app
mkdir -p "$app/Contents/MacOS"
cp "$binary" "$app/Contents/MacOS/$target"
cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key>
  <string>$bundle_id</string>
  <key>CFBundleName</key>
  <string>$name</string>
  <key>CFBundleExecutable</key>
  <string>$target</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>0.1</string>
  <key>LSMinimumSystemVersion</key>
  <string>14.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
EOF
echo "$PWD/$app"
