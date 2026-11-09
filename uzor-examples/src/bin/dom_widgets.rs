//! The dashboard widgets, live: space switcher, tiles, agents, a task tree, rings, a
//! task card, a mini board, activity, mini panels and glass overlays (RU/EN button).
//!
//! ```sh
//! cargo run -p uzor-examples --release --features native --bin dom-widgets
//! ```

use uzor_examples::page_app::PageApp;
use uzor_framework::host::native::{run_native, NativeOptions};

fn main() -> Result<(), uzor_framework::FrameworkError> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let app = PageApp::new(uzor_examples::dom_widgets::WidgetsPage::default());
    let cfg = app.runtime_config();
    run_native(app, cfg, NativeOptions::default())
}
