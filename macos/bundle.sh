#!/bin/sh
# Builds Hollow Hours.app (game + launcher + icon + the hollowhours:// link). macos/bundle.sh [--install] copies it to /Applications.
set -e
cd "$(dirname "$0")/.."
command -v cargo >/dev/null || PATH="$(brew --prefix rustup 2>/dev/null)/bin:$PATH"
# UNIVERSAL=1: one app for Apple Silicon AND Intel Macs (both builds glued together with lipo).
if [ "$UNIVERSAL" = 1 ]; then
  rustup target add aarch64-apple-darwin x86_64-apple-darwin
  cargo build --release --target aarch64-apple-darwin
  cargo build --release --target x86_64-apple-darwin
  mkdir -p target/universal
  for b in hollow-hours hh-mcp; do
    lipo -create "target/aarch64-apple-darwin/release/$b" "target/x86_64-apple-darwin/release/$b" -output "target/universal/$b"
  done
  BIN=target/universal
else
  cargo build --release
  BIN=target/release
fi
[ -f macos/HollowHours.icns ] || { swift macos/make-icon.swift macos/HollowHours.iconset && iconutil -c icns macos/HollowHours.iconset -o macos/HollowHours.icns; }
APP="Hollow Hours.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
if [ "$UNIVERSAL" = 1 ]; then
  xcrun swiftc -O -target arm64-apple-macos12 macos/Launcher.swift -o target/launcher-arm64
  xcrun swiftc -O -target x86_64-apple-macos12 macos/Launcher.swift -o target/launcher-x86_64
  lipo -create target/launcher-arm64 target/launcher-x86_64 -output "$APP/Contents/MacOS/Hollow Hours"
else
  xcrun swiftc -O macos/Launcher.swift -o "$APP/Contents/MacOS/Hollow Hours"
fi
cp "$BIN/hollow-hours" "$APP/Contents/MacOS/hollow-hours-game"
cp "$BIN/hh-mcp" "$APP/Contents/MacOS/hh-mcp"
cp macos/Info.plist "$APP/Contents/Info.plist"
cp macos/HollowHours.icns "$APP/Contents/Resources/HollowHours.icns"
codesign --force --deep --sign - "$APP" 2>/dev/null || true
echo "built $(pwd)/$APP"
if [ "$1" = "--install" ]; then
  rm -rf "/Applications/$APP"; cp -R "$APP" /Applications/
  /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "/Applications/$APP"
  echo "installed /Applications/$APP (hollowhours:// links open it)"
fi
