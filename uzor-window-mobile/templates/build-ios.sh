#!/bin/bash
# Build the iOS host of __APP_TITLE__ (macOS with Xcode, rustup targets
# aarch64-apple-ios and aarch64-apple-ios-sim, and xcodegen).
#   ./build-ios.sh            device + simulator libraries, xcframework, project, simulator build
#   ./build-ios.sh --open     ... and open the project in Xcode
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release --lib --target aarch64-apple-ios
cargo build --release --lib --target aarch64-apple-ios-sim
rm -rf ios/Rust/lib__APP__.xcframework
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/lib__APP__.a \
  -library target/aarch64-apple-ios-sim/release/lib__APP__.a \
  -output ios/Rust/lib__APP__.xcframework
cd ios
xcodegen generate
xcodebuild -project __APP_TITLE__.xcodeproj -scheme __APP_TITLE__ -sdk iphonesimulator -configuration Debug build | tail -3
if [ "${1:-}" = "--open" ]; then open __APP_TITLE__.xcodeproj; fi
