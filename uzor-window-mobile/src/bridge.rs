//! What the native wrapper tells the app: safe-area insets and the soft keyboard
//! height, in logical px (points on iOS, dp on Android). Callable from any thread;
//! the framework host echoes `WindowInput::Insets` on its next loop turn.
//!
//! Swift (see `templates/ios/Sources/Bridge.swift`) calls the C functions; Kotlin
//! (`templates/android/.../UzorBridge.kt`) calls the JNI ones, which is why the
//! Kotlin class lives at the fixed name `dev.uzor.mobile.UzorBridge`.

use crate::Insets;

/// Push the safe area (overrides what winit reports; on Android only the wrapper
/// knows it).
pub fn set_safe_area(insets: Insets) {
    uzor_framework::platform::set_safe_area(insets);
}

/// Push the soft keyboard height over the bottom of the window (0 = hidden).
pub fn set_keyboard(height: f64) {
    uzor_framework::platform::set_keyboard(height);
}

/// The values last pushed: (safe area if the wrapper set one, keyboard height).
pub fn current() -> (Option<Insets>, f64) {
    uzor_framework::platform::current()
}

fn finite(v: f64) -> f64 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

/// C: `void uzor_mobile_set_safe_area(double top, double right, double bottom, double left)`.
#[no_mangle]
pub extern "C" fn uzor_mobile_set_safe_area(top: f64, right: f64, bottom: f64, left: f64) {
    set_safe_area(Insets::new(finite(top), finite(right), finite(bottom), finite(left)));
}

/// C: `void uzor_mobile_set_keyboard(double height)`.
#[no_mangle]
pub extern "C" fn uzor_mobile_set_keyboard(height: f64) {
    set_keyboard(finite(height));
}

/// JNI: `external fun setSafeArea(top: Float, right: Float, bottom: Float, left: Float)`
/// on `dev.uzor.mobile.UzorBridge` (an `object`, so the receiver is the instance).
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_dev_uzor_mobile_UzorBridge_setSafeArea(
    _env: *mut core::ffi::c_void,
    _this: *mut core::ffi::c_void,
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
) {
    uzor_mobile_set_safe_area(top.into(), right.into(), bottom.into(), left.into());
}

/// JNI: `external fun setKeyboard(height: Float)` on `dev.uzor.mobile.UzorBridge`.
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_dev_uzor_mobile_UzorBridge_setKeyboard(
    _env: *mut core::ffi::c_void,
    _this: *mut core::ffi::c_void,
    height: f32,
) {
    uzor_mobile_set_keyboard(height.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_entry_points_reach_the_framework() {
        uzor_mobile_set_safe_area(47.0, 0.0, 34.0, f64::NAN);
        uzor_mobile_set_keyboard(-3.0);
        assert_eq!(current(), (Some(Insets::new(47.0, 0.0, 34.0, 0.0)), 0.0));
        uzor_mobile_set_keyboard(291.0);
        assert_eq!(current().1, 291.0);
    }
}
