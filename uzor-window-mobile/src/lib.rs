//! The thin mobile layer over `uzor-framework`.
//!
//! An uzor app on a phone is the same `App` as on the desktop, run by the
//! framework's native host (winit). This crate holds what only a phone needs and
//! keeps the platform pieces out of uzor's core:
//!
//! - **Entry** ([`entry!`]): one macro gives the app its Android `android_main` and
//!   the `extern "C" fn <name>_run()` the Swift host calls; both run
//!   [`run`] with phone defaults (lean render backends, the activity handed to
//!   winit).
//! - **Lifecycle**: background / foreground is handled by the framework host
//!   (surfaces dropped and remade on Android, painting paused on iOS, app state
//!   kept); windows read `Occluded(true / false)`. Nothing to wire.
//! - **Insets and keyboard** ([`bridge`]): the safe area (winit reports it on iOS;
//!   Android `WindowInsets` comes from the Kotlin host) and the soft keyboard
//!   height (iOS `keyboardWillChangeFrame`, Android `WindowInsets.ime`) arrive
//!   through C / JNI functions the host templates call. Hooks read
//!   `VisualView::safe_area()` / `keyboard()`; CSS reads
//!   `env(safe-area-inset-*)` / `env(keyboard-inset-height)`.
//! - **Keyboard show / hide**: a focused text field already asks the host for the
//!   IME (`WindowCommand::SetImeAllowed`), which winit turns into
//!   `becomeFirstResponder` (iOS) / `showSoftInput` (Android).
//! - **Haptics** ([`haptics`]): a call the app makes; the host wrapper performs it
//!   (stub until a wrapper registers a performer).
//! - **Host templates** (`templates/` in this crate's source): an XcodeGen iOS host
//!   with a five-line `main.swift`, a Kotlin `GameActivity` host, build scripts,
//!   and `scripts/new-app.sh <name>` that stamps a new app out of them.

pub mod bridge;
pub mod haptics;

pub use uzor_framework::host::native::NativeOptions;
pub use uzor_framework::types::window::Insets;

/// Phone defaults on top of the app's own options: the activity handed to winit on
/// Android. The app keeps everything else (`RuntimeConfig` comes from it).
#[derive(Default)]
pub struct MobileOptions {
    /// Extra native options (render preference, dev control).
    pub native: NativeOptions,
}

/// Run `app` on the phone's native host. On Android pass the activity
/// `android_main` received; on iOS call this from the `<name>_run` entry the
/// Swift `main` calls (it never returns while the app lives).
#[cfg(target_os = "android")]
pub fn run<S, A>(
    app: A,
    cfg: uzor_framework::RuntimeConfig,
    activity: uzor_framework::host::native::android::AndroidApp,
    opts: MobileOptions,
) -> Result<(), uzor_framework::FrameworkError>
where
    S: uzor_framework::Spec,
    A: uzor_framework::App<Spec = S>,
{
    let native = NativeOptions { android_app: Some(activity), ..opts.native };
    uzor_framework::host::native::run_native(app, cfg, native)
}

/// Run `app` on the native host (iOS, and the desktop for development).
#[cfg(not(target_os = "android"))]
pub fn run<S, A>(app: A, cfg: uzor_framework::RuntimeConfig, opts: MobileOptions) -> Result<(), uzor_framework::FrameworkError>
where
    S: uzor_framework::Spec,
    A: uzor_framework::App<Spec = S>,
{
    uzor_framework::host::native::run_native(app, cfg, opts.native)
}

/// The app's mobile entry points, from one place:
///
/// ```ignore
/// uzor_window_mobile::entry!(myapp_run, || (MyApp::new(), MyApp::config()));
/// ```
///
/// expands to `android_main` (Android, the symbol `android-activity` looks for) and
/// `#[no_mangle] extern "C" fn myapp_run() -> i32` (what the Swift host's `main`
/// calls; also usable on the desktop). The closure builds the app and its
/// `RuntimeConfig`; errors are logged and turned into a non-zero return.
#[macro_export]
macro_rules! entry {
    ($run:ident, $make:expr) => {
        #[cfg(target_os = "android")]
        #[no_mangle]
        fn android_main(activity: $crate::__android::AndroidApp) {
            let (app, cfg) = ($make)();
            if let Err(e) = $crate::run(app, cfg, activity, $crate::MobileOptions::default()) {
                $crate::__log_error(stringify!($run), &e);
            }
        }

        #[no_mangle]
        pub extern "C" fn $run() -> i32 {
            #[cfg(target_os = "android")]
            {
                // Android enters through android_main; the C entry is for iOS / desktop.
                return 1;
            }
            #[cfg(not(target_os = "android"))]
            {
                let (app, cfg) = ($make)();
                match $crate::run(app, cfg, $crate::MobileOptions::default()) {
                    Ok(()) => 0,
                    Err(e) => {
                        $crate::__log_error(stringify!($run), &e);
                        1
                    }
                }
            }
        }
    };
}

#[doc(hidden)]
#[cfg(target_os = "android")]
pub mod __android {
    pub use uzor_framework::host::native::android::AndroidApp;
}

#[doc(hidden)]
pub fn __log_error(entry: &str, e: &dyn std::fmt::Display) {
    log::error!("{entry}: {e}");
    eprintln!("{entry}: {e}");
}
