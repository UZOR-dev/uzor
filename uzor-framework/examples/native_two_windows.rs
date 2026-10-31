//! Two native windows driven by one app, each a DOM page (F9 done-condition).
//!
//! ```text
//! cargo run -p uzor-framework --release --example native_two_windows --features native
//! ```

#[path = "two_windows/mod.rs"]
mod two_windows;

use two_windows::TwoWindows;
use uzor_framework::host::native::run_native_default;

fn main() -> Result<(), uzor_framework::FrameworkError> {
    run_native_default(TwoWindows::new(), TwoWindows::config())
}
