#!/usr/bin/env bash
set -euo pipefail

SDK_DIR="/opt/android-sdk"
BUILD_TOOLS="$SDK_DIR/build-tools/35.0.0"
PLATFORM="$SDK_DIR/platforms/android-36/android.jar"
APP_DIR="chronos-app/app"
BUILD_DIR="target/apk_build"
DIST_DIR="dist"

mkdir -p "$BUILD_DIR/gen" "$BUILD_DIR/classes" "$BUILD_DIR/dex" "$BUILD_DIR/apk/lib/arm64-v8a" "$DIST_DIR"

echo "==> 1. Compiling resources with aapt2..."
"$BUILD_TOOLS/aapt2" compile --dir "$APP_DIR/src/main/res" -o "$BUILD_DIR/res.zip"

echo "==> 2. Linking APK resources and generating R.java..."
"$BUILD_TOOLS/aapt2" link -o "$BUILD_DIR/unaligned_res.apk" \
    -I "$PLATFORM" \
    --manifest "$APP_DIR/src/main/AndroidManifest.xml" \
    --auto-add-overlay \
    --java "$BUILD_DIR/gen" \
    "$BUILD_DIR/res.zip"

echo "==> 3. Compiling Java sources with javac (targeting Java 17)..."
javac --release 17 -cp "$PLATFORM" \
    -d "$BUILD_DIR/classes" \
    $(find "$APP_DIR/src/main/java" -name "*.java") \
    $(find "$BUILD_DIR/gen" -name "*.java" 2>/dev/null || true)

echo "==> 4. Dexing bytecode with d8..."
"$BUILD_TOOLS/d8" \
    --lib "$PLATFORM" \
    --output "$BUILD_DIR/dex" \
    $(find "$BUILD_DIR/classes" -name "*.class")

echo "==> 5. Assembling unaligned APK..."
cp "$BUILD_DIR/unaligned_res.apk" "$BUILD_DIR/unaligned.apk"
cd "$BUILD_DIR/dex"
zip -u "../unaligned.apk" classes.dex
cd - > /dev/null

# Include native arm64-v8a library
cp "target/aarch64-linux-android/release/libchronos_app.so" "$BUILD_DIR/apk/lib/arm64-v8a/"
cd "$BUILD_DIR/apk"
zip -u -r "../unaligned.apk" lib/
cd - > /dev/null

echo "==> 6. Aligning APK with zipalign..."
"$BUILD_TOOLS/zipalign" -p -f 4 "$BUILD_DIR/unaligned.apk" "$DIST_DIR/chronos-app.apk"

echo "==> 7. Generating debug keystore if not present..."
KEYSTORE="$BUILD_DIR/debug.keystore"
if [ ! -f "$KEYSTORE" ]; then
    keytool -genkeypair -v -keystore "$KEYSTORE" \
        -storepass android -keypass android \
        -alias androiddebugkey -dname "CN=Android Debug,O=Android,C=US" \
        -keyalg RSA -keysize 2048 -validity 10000
fi

echo "==> 8. Signing APK with apksigner..."
"$BUILD_TOOLS/apksigner" sign \
    --ks "$KEYSTORE" \
    --ks-pass pass:android \
    --key-pass pass:android \
    --ks-key-alias androiddebugkey \
    "$DIST_DIR/chronos-app.apk"

echo "==> Build complete: $DIST_DIR/chronos-app.apk"
"$BUILD_TOOLS/apksigner" verify --verbose "$DIST_DIR/chronos-app.apk"
