//! Emoji fallback for canvas text, the way a browser page gets it for free: every font
//! string the renderer sets gets the colour-emoji families appended (the platform ones,
//! then the bundled [`FAMILY`]), so `👍` inside `500 14px Inter, sans-serif` paints in
//! colour instead of tofu. When the platform has no colour-emoji font, [`ensure_font`]
//! loads a small subset (see `uzor-fonts/fonts/README-emoji-subset.md`) as a separate asset
//! — never inside the wasm — and repaints when it arrives.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Family name the bundled subset registers under.
pub const FAMILY: &str = "UzorEmoji";

/// Appended to every font family list.
pub const TAIL: &str = ", \"Apple Color Emoji\", \"Segoe UI Emoji\", \"Noto Color Emoji\", UzorEmoji";

/// `font` with the emoji fallback families (unchanged when it already names one).
pub fn with_fallback(font: &str) -> String {
    if font.contains("Emoji") {
        font.to_string()
    } else {
        format!("{font}{TAIL}")
    }
}

/// Whether the platform paints colour emoji: draws 😀 with the platform families only and
/// looks for a non-grey pixel.
fn platform_has_color_emoji() -> bool {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return true };
    let Ok(el) = doc.create_element("canvas") else { return true };
    let Ok(c) = el.dyn_into::<web_sys::HtmlCanvasElement>() else { return true };
    c.set_width(24);
    c.set_height(24);
    let Some(ctx) = c.get_context("2d").ok().flatten().and_then(|x| x.dyn_into::<web_sys::CanvasRenderingContext2d>().ok()) else { return true };
    ctx.set_font("20px \"Apple Color Emoji\", \"Segoe UI Emoji\", \"Noto Color Emoji\"");
    ctx.set_text_baseline("top");
    let _ = ctx.fill_text("\u{1F600}", 0.0, 0.0);
    let Ok(img) = ctx.get_image_data(0.0, 0.0, 24.0, 24.0) else { return true };
    img.data().0.chunks(4).any(|p| p[3] > 0 && (p[0].abs_diff(p[1]) > 24 || p[1].abs_diff(p[2]) > 24))
}

/// Load the bundled emoji subset from `url` (a woff2 next to the page) when the platform
/// lacks colour emoji; once it is in `document.fonts`, a `resize` event makes the host
/// repaint. Returns whether a load was started.
pub fn ensure_font(url: &str) -> bool {
    if platform_has_color_emoji() {
        return false;
    }
    let Some(w) = web_sys::window() else { return false };
    let Ok(ctor) = js_sys::Reflect::get(&w, &JsValue::from_str("FontFace")) else { return false };
    let Ok(ctor) = ctor.dyn_into::<js_sys::Function>() else { return false };
    let args = js_sys::Array::of2(&JsValue::from_str(FAMILY), &JsValue::from_str(&format!("url({url})")));
    let Ok(face) = js_sys::Reflect::construct(&ctor, &args) else { return false };
    let Ok(load) = js_sys::Reflect::get(&face, &JsValue::from_str("load")).and_then(|f| f.dyn_into::<js_sys::Function>().map_err(JsValue::from)) else { return false };
    let Ok(p) = load.call0(&face).and_then(|p| p.dyn_into::<js_sys::Promise>().map_err(JsValue::from)) else { return false };
    let done = Closure::once(move |face: JsValue| {
        let Some(w) = web_sys::window() else { return };
        if let Some(doc) = w.document() {
            if let Ok(fonts) = js_sys::Reflect::get(&doc, &JsValue::from_str("fonts")) {
                if let Ok(add) = js_sys::Reflect::get(&fonts, &JsValue::from_str("add")).and_then(|f| f.dyn_into::<js_sys::Function>().map_err(JsValue::from)) {
                    let _ = add.call1(&fonts, &face);
                }
            }
        }
        if let Ok(ev) = web_sys::Event::new("resize") {
            let _ = w.dispatch_event(&ev);
        }
    });
    let _ = p.then(&done);
    done.forget();
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn fallback_appended_once() {
        let f = super::with_fallback("500 14px Inter, sans-serif");
        assert!(f.starts_with("500 14px Inter, sans-serif, ") && f.ends_with("UzorEmoji"));
        assert_eq!(super::with_fallback(&f), f);
    }
}
