#!/bin/sh
# Builds Hollow Hours for Android (arm64 + older 32-bit phones) with today's tools:
#   1. the game as a shared library (cargo-ndk)
#   2. miniquad's own Java activity, compiled to classes.dex (javac + d8)
#   3. packed, aligned and signed with the Android SDK's aapt2, zipalign and apksigner
# Needs ANDROID_HOME (with build-tools and a platform), an NDK (ANDROID_NDK_HOME), a JDK and `cargo install cargo-ndk`.
# HH_KEYSTORE=path signs with your own key (otherwise a new debug key is made).
set -e
cd "$(dirname "$0")/.."
PKG=io.github.jdj212223_ctrl.hollowhours
SDK=${ANDROID_HOME:-$ANDROID_SDK_ROOT}
BT=$(ls -d "$SDK"/build-tools/* | sort -V | tail -1)
PLATFORM=$(ls -d "$SDK"/platforms/android-* | sort -V | tail -1)
echo "build-tools: $BT  platform: $PLATFORM"
OUT=target/apk
rm -rf "$OUT"
mkdir -p "$OUT/java/main" "$OUT/java/quad_native" "$OUT/classes" "$OUT/lib/arm64-v8a" "$OUT/lib/armeabi-v7a"

# 1. The game.
cargo ndk -t arm64-v8a -t armeabi-v7a -P 26 build --release --example hollowhours_android
cp target/aarch64-linux-android/release/examples/libhollowhours_android.so "$OUT/lib/arm64-v8a/libhollowhours.so"
cp target/armv7-linux-androideabi/release/examples/libhollowhours_android.so "$OUT/lib/armeabi-v7a/libhollowhours.so"

# 2. miniquad's Java activity (from the miniquad version this game uses).
MQ=$(dirname "$(cargo metadata --format-version 1 | python3 -c "import json,sys; print([p['manifest_path'] for p in json.load(sys.stdin)['packages'] if p['name']=='miniquad'][0])")")
sed -e "s/TARGET_PACKAGE_NAME/$PKG/" -e 's/LIBRARY_NAME/hollowhours/' -e '/\/\/%/d' "$MQ/java/MainActivity.java" > "$OUT/java/main/MainActivity.java"
cp "$MQ/java/QuadNative.java" "$OUT/java/quad_native/"
javac --release 11 -cp "$PLATFORM/android.jar" -d "$OUT/classes" "$OUT/java/quad_native/QuadNative.java" "$OUT/java/main/MainActivity.java"
"$BT/d8" --release --lib "$PLATFORM/android.jar" --min-api 26 --output "$OUT" $(find "$OUT/classes" -name '*.class')

# 3. Pack, align, sign.
"$BT/aapt2" compile --dir android/res -o "$OUT/res.zip"
"$BT/aapt2" link -o "$OUT/base.apk" -I "$PLATFORM/android.jar" --manifest android/AndroidManifest.xml "$OUT/res.zip" \
  --min-sdk-version 26 --target-sdk-version 34 --version-code 22 --version-name 2.2
(cd "$OUT" && zip -q base.apk classes.dex && zip -qr base.apk lib)
"$BT/zipalign" -f -p 4 "$OUT/base.apk" "$OUT/aligned.apk"
KS=${HH_KEYSTORE:-$OUT/debug.keystore}
[ -f "$KS" ] || keytool -genkeypair -keystore "$KS" -storepass android -keypass android -alias hh -keyalg RSA -keysize 2048 \
  -validity 10000 -dname "CN=Hollow Hours" >/dev/null 2>&1
"$BT/apksigner" sign --ks "$KS" --ks-pass pass:android --key-pass pass:android --out Hollow-Hours-Android.apk "$OUT/aligned.apk"
"$BT/apksigner" verify Hollow-Hours-Android.apk
echo "built Hollow-Hours-Android.apk"
