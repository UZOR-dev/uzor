//! __APP_TITLE__ entry points: `android_main` for Android and `__APP___run` for the
//! Swift host (and the desktop).
pub mod app;

uzor_window_mobile::entry!(__APP___run, || (app::MobileApp::new(), app::MobileApp::config()));
