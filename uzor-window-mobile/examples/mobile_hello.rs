//! The host template's app (`templates/rust/src/app.rs`) on the desktop, in a
//! phone-shaped window: `cargo run -p uzor-window-mobile --example mobile_hello`.
//! The same file is what `scripts/new-app.sh` stamps into a new app.
#[path = "../templates/rust/src/app.rs"]
mod app;

uzor_window_mobile::entry!(mobile_hello_run, || (app::MobileApp::new(), app::MobileApp::config()));

fn main() {
    std::process::exit(mobile_hello_run());
}
