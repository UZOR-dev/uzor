# __APP_TITLE__

An uzor app for iOS, Android and the desktop, made by `uzor-window-mobile`'s
`new-app.sh`. All UI and logic is Rust (`src/app.rs`); the hosts are thin.

| Where | What |
|---|---|
| `src/app.rs` | the app (an `uzor_framework::App`, DOM screens) |
| `src/lib.rs` | entry points: `android_main`, `__APP___run` for Swift |
| `src/main.rs` | desktop run: `cargo run` |
| `ios/` | XcodeGen host: `main.swift` (calls `__APP___run`), `Bridge.swift` (keyboard height, haptics) |
| `android/` | Gradle host: `MainActivity` (GameActivity), `dev.uzor.mobile.UzorBridge` (insets, keyboard) |
| `build-ios.sh` | macOS: device + simulator libraries, xcframework, `xcodegen`, simulator build |
| `build-android.sh` | NDK: arm64 + x86_64 `.so` into `jniLibs`, debug APK |

Edge to edge: pad top chrome with `cx.view.safe_area().top`, bottom chrome with
`max(safe_area().bottom, keyboard())`, or in CSS with `env(safe-area-inset-*)` and
`env(keyboard-inset-height)`.
