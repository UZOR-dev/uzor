//! Tray widgets, live: the gallery window plus a real tray icon. Hover the icon (Windows,
//! macOS) or pick "Show widget" in its menu (Linux) for the glass flyout; on Windows the
//! taskbar thumbnail of the window is the picked widget too. The tray icon carries a
//! status dot. Picks, language and theme on the page drive the tray.
//!
//! ```sh
//! cargo run -p uzor-examples --release --features native --bin tray-widgets
//! ```

use uzor::ui::tray::TrayWidget;
use uzor_examples::page_app::PageApp;
use uzor_examples::tray_widgets::{demo_logo, TrayPage};
use uzor_framework::host::native::{
    run_native, NativeOptions, TraySpec, TrayWidgetConfig, TrayWidgetHandle,
};

fn main() -> Result<(), uzor_framework::FrameworkError> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let mut page = TrayPage::default();
    // UZOR_TRAY_SHOW=1 opens the flyout at start (no tray host to hover; screenshots).
    // It may name a widget and "light" / "en": UZOR_TRAY_SHOW=agents,light.
    let show = std::env::var("UZOR_TRAY_SHOW").ok();
    for word in show.iter().flat_map(|s| s.split(',')) {
        match word {
            "light" => page.g.dark = false,
            "en" => page.g.ru = false,
            k => {
                if let Some(w) = TrayWidget::from_key(k) {
                    page.g.pick = w;
                }
            }
        }
    }
    let handle = TrayWidgetHandle::new(TrayWidgetConfig {
        widget: Some(page.g.pick),
        dark: page.g.dark,
        model: page.g.model(),
        labels: page.g.labels(),
        menu_label: Some("Показать виджет".into()),
        ..TrayWidgetConfig::default()
    });
    let feed = handle.clone();
    page.on_change = Some(Box::new(move |g| {
        feed.update(|c| {
            c.widget = Some(g.pick);
            c.model = g.model();
            c.labels = g.labels();
            c.dark = g.dark;
        });
    }));
    if show.is_some() {
        handle.show();
    }
    let app = PageApp::new(page);
    let cfg = app.runtime_config();
    // The host appends the "show widget" entry itself (menu_label).
    let tray = TraySpec { icon: Some(demo_logo(32, true)), tooltip: Some("uzor — tray widgets".into()), items: Vec::new() };
    let opts = NativeOptions { tray: Some(tray), tray_widget: Some(handle), ..NativeOptions::default() };
    run_native(app, cfg, opts)
}
