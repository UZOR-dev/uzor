# uzor-window-mobile

The thin mobile layer over `uzor-framework`. A phone app is the same
`uzor_framework::App` as on the desktop, run by the framework's native host
(winit); this crate adds only what a phone needs and keeps platform pieces out of
uzor's core.

| Piece | What it does |
|---|---|
| `entry!(name_run, make)` | `android_main` for Android and `extern "C" fn name_run()` for the Swift host, both running the app with phone defaults (lean render backends) |
| lifecycle | handled by the framework host: surfaces dropped / remade around Android background, painting paused on iOS, app state kept; windows read `Occluded` |
| `bridge` | safe area and soft keyboard height from the native wrapper (C for Swift, JNI for Kotlin), into `VisualView::safe_area()` / `keyboard()` and CSS `env()` |
| `haptics` | `haptic(Haptic::Light)`; the iOS wrapper performs it, Android is a no-op for now |
| `templates/` | iOS host (XcodeGen `project.yml`, 5-line `main.swift`, `Bridge.swift`), Android host (Gradle, Kotlin `GameActivity`, `UzorBridge`), build scripts, the app skeleton |
| `scripts/new-app.sh` | `new-app.sh my_app [dest] [--title MyApp] [--package com.example.my_app] [--uzor-path DIR]` |

Try the template app on the desktop:
`cargo run -p uzor-window-mobile --example mobile_hello`.

Features: `android-game-activity` (text input through GameTextInput; the
templates use it), `android-native-activity`, `all-backends` (every render
backend instead of vello-gpu + tiny-skia).
