//! The DOM gallery, live: chat blocks, markdown, a virtualized chat, a kit chart in a slot.
//!
//! ```sh
//! cargo run -p uzor-examples --release --features native --bin dom-gallery
//! ```

use uzor_examples::page_app::PageApp;
use uzor_framework::host::native::{run_native, NativeOptions};

fn main() -> Result<(), uzor_framework::FrameworkError> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let app = PageApp::new(uzor_examples::dom_gallery::DomGalleryPage::default());
    let cfg = app.runtime_config();
    run_native(app, cfg, NativeOptions::default())
}
