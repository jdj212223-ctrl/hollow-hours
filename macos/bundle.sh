#!/bin/sh
# Builds Hollow Hours.app (game + launcher + icon + the hollowhours:// link). macos/bundle.sh [--install] copies it to /Applications.
set -e
cd "$(dirname "$0")/.."
command -v cargo >/dev/null || PATH="$(brew --prefix rustup 2>/dev/null)/bin:$PATH"
cargo build --release
[ -f macos/HollowHours.icns ] || { swift macos/make-icon.swift macos/HollowHours.iconset && iconutil -c icns macos/HollowHours.iconset -o macos/HollowHours.icns; }
APP="Hollow Hours.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
xcrun swiftc -O macos/Launcher.swift -o "$APP/Contents/MacOS/Hollow Hours"
cp target/release/hollow-hours "$APP/Contents/MacOS/hollow-hours-game"
cp macos/Info.plist "$APP/Contents/Info.plist"
cp macos/HollowHours.icns "$APP/Contents/Resources/HollowHours.icns"
codesign --force --deep --sign - "$APP" 2>/dev/null || true
echo "built $(pwd)/$APP"
if [ "$1" = "--install" ]; then
  rm -rf "/Applications/$APP"; cp -R "$APP" /Applications/
  /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "/Applications/$APP"
  echo "installed /Applications/$APP (hollowhours:// links open it)"
fi
