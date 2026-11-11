#!/bin/bash
# Build the Android host of __APP_TITLE__.
# Needs: ANDROID_NDK_HOME (r26+), rustup targets aarch64-linux-android (+ x86_64-linux-android
# for the emulator), and for the APK the Android SDK + JDK 17 (Gradle wrapper or `gradle`).
#   ./build-android.sh          release .so for arm64 + x86_64, copied into jniLibs, debug APK
#   ./build-android.sh --so     only the libraries
set -euo pipefail
cd "$(dirname "$0")"
: "${ANDROID_NDK_HOME:?set ANDROID_NDK_HOME to the NDK}"
API=26
BIN=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$(uname -s | tr A-Z a-z)-x86_64/bin
build() { # rust target, android abi, clang prefix
  local T=$1 ABI=$2 CL=$3
  local UT=$(echo "$T" | tr a-z- A-Z_)
  env "CC_${T//-/_}=$BIN/${CL}${API}-clang" "AR_${T//-/_}=$BIN/llvm-ar" \
      "CARGO_TARGET_${UT}_LINKER=$BIN/${CL}${API}-clang" \
      cargo build --release --lib --target "$T"
  mkdir -p android/app/src/main/jniLibs/$ABI
  cp target/$T/release/lib__APP__.so android/app/src/main/jniLibs/$ABI/
}
build aarch64-linux-android arm64-v8a aarch64-linux-android
build x86_64-linux-android x86_64 x86_64-linux-android
ls -la android/app/src/main/jniLibs/*/
[ "${1:-}" = "--so" ] && exit 0
cd android
if [ -x ./gradlew ]; then ./gradlew assembleDebug; else gradle assembleDebug; fi
ls -la app/build/outputs/apk/debug/
