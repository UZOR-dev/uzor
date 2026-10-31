//! The shared look of the `uzor-examples` pages: the domkit catalog plus a few page
//! classes, themed by `--k-*` variables on the page root, the same look as
//! [`crate::dom_gallery`] and the `uzor-framework` examples.

use uzor::ui::dom::El;
use uzor::ui::domkit as k;
use uzor::ui::style::Sheet;
use uzor::Rect;

/// Page classes on top of the catalog.
pub const PAGE_CSS: &str = r#"
div.p-page { display: flex; flex-direction: column; gap: 10px; padding: 14px; background: var(--k-bg); overflow: hidden }
div.p-sec { color: var(--k-muted); font-size: 11px; font-weight: 600; height: 14px; flex-shrink: 0 }
div.p-row { display: flex; align-items: center; gap: 10px; flex-shrink: 0 }
div.p-grow { flex-grow: 1 }
div.p-muted { color: var(--k-muted) }
div.p-h1 { font-size: 16px; font-weight: 600 }
div.p-card { display: flex; flex-direction: column; gap: 8px; padding: 12px; border-radius: 10px; background: var(--k-panel);
             border: 1px solid var(--k-line); flex-shrink: 0 }
div.p-grid2 { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; flex-shrink: 0 }
span.p-icon { width: 16px; height: 16px; color: var(--k-muted); flex-shrink: 0 }
span.p-icon.accent { color: var(--k-accent) }
"#;

/// The catalog + [`PAGE_CSS`] + `extra`.
pub fn sheet(extra: &str) -> Sheet {
    k::sheet_with(&format!("{PAGE_CSS}\n{extra}")).expect("example css")
}

/// The themed root (`div.k-root`) of a page `h` tall.
pub fn root(dark: bool, h: f64) -> El {
    let vars = k::theme_vars(dark);
    let sep = if vars.is_empty() { "" } else { "; " };
    El::new("div.k-root").css(format!("{vars}{sep}height: {h}px"))
}

/// A small icon; `accent` tints it.
pub fn ico(body: &str, accent: bool) -> El {
    El::new(if accent { "span.p-icon.accent" } else { "span.p-icon" }).svg(k::icon(body))
}

/// The leaf area below the 28px dock header the kernel paints.
pub fn leaf_body(panel: Rect) -> Rect {
    const HEADER: f64 = 28.0;
    Rect::new(panel.x, panel.y + HEADER, panel.width.max(1.0), (panel.height - HEADER).max(1.0))
}
