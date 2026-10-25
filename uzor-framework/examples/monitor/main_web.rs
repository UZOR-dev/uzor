//! Web entry for the HQ observe monitor.
//!
//! One canvas, four dock panels. The page owns the bearer stream and calls
//! [`push`] with each observe frame. This crate does not open a socket.
//!
//! ```text
//! cargo build -p uzor-framework --example monitor_web --release \
//!     --target wasm32-unknown-unknown --features monitor
//! ```

#[path = "mod.rs"]
mod monitor;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Browser start. Calls [`uzor_framework::host::start_web`].
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn start_web() {
    let _ = uzor_framework::host::start_web(
        monitor::CANVAS_ID,
        monitor::MonitorApp::new(),
        monitor::MonitorApp::runtime_config(),
    );
}

/// Store one observe JSON frame and ask the dock to repaint.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn push(json: &str) {
    monitor::push_snap(json);
}
