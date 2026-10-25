//! Paint path through `uzor-render-canvas2d` (design §2.4, §5.5).
//!
//! `uzor-render-hub` does not build for `wasm32-unknown-unknown` (verdict in
//! `host/native/mod.rs`). Canvas2d is immediate-mode: it has no retained
//! cache, so [`FrameRequest::invalidations`] are not forwarded. The kernel
//! still decides which frames are due; this function only draws the ones it
//! is given.

use uzor_render_canvas2d::Canvas2dRenderContext;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use super::WebHostError;
use crate::runtime::Runtime;
use crate::types::frame::FrameRequest;
use crate::{App, Spec};

/// Paint one [`FrameRequest`] into `canvas` and present it (the 2D context
/// draws directly; there is no separate submit).
pub fn paint_frame<S, A>(
    runtime: &mut Runtime<S, A>,
    canvas: &HtmlCanvasElement,
    req: &FrameRequest,
) -> Result<(), WebHostError>
where
    S: Spec,
    A: App<Spec = S>,
{
    let dpr = if req.dpr.is_finite() && req.dpr > 0.0 {
        req.dpr
    } else {
        1.0
    };
    let (css_w, css_h) = sync_backing_store(canvas, dpr);
    // An opaque background (alpha 0xff, or 0 = unset, painted opaque) gets an opaque
    // context: only there does the browser draw LCD (subpixel) text, the crisp text every
    // DOM page gets; on a transparent canvas text is greyscale-antialiased and looks soft.
    let a = req.background & 0xff;
    let ctx = context_2d(canvas, a == 0 || a == 0xff)?;
    let _ = ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
    // The 2D context keeps its state between frames: a leaked alpha, blend or
    // shadow from the last frame must not reach the background fill.
    ctx.set_global_alpha(1.0);
    let _ = ctx.set_global_composite_operation("source-over");
    ctx.set_shadow_blur(0.0);
    ctx.set_shadow_color("rgba(0,0,0,0)");
    ctx.set_fill_style_str(&css_rgba(req.background));
    ctx.fill_rect(0.0, 0.0, css_w, css_h);

    let mut rctx = Canvas2dRenderContext::new(ctx, dpr);
    if req.regions.is_empty() {
        runtime.paint(req, &mut rctx);
    } else {
        for region in &req.regions {
            let mut region_req = req.clone();
            region_req.regions = smallvec::smallvec![*region];
            runtime.paint(&region_req, &mut rctx);
        }
    }
    Ok(())
}

/// The canvas' 2D context. Its attributes are fixed by the first `getContext` call on the
/// element; later calls return that same context whatever they ask for.
fn context_2d(canvas: &HtmlCanvasElement, opaque: bool) -> Result<CanvasRenderingContext2d, WebHostError> {
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"alpha".into(), &(!opaque).into());
    let raw = canvas
        .get_context_with_context_options("2d", &opts)
        .map_err(|e| WebHostError::new(format!("getContext(\"2d\"): {e:?}")))?
        .ok_or_else(|| WebHostError::new("getContext(\"2d\") returned null"))?;
    raw.dyn_into::<CanvasRenderingContext2d>()
        .map_err(|_| WebHostError::new("getContext(\"2d\") was not a 2d context"))
}

/// CSS pixel size of the element, after the bitmap is sized to physical pixels.
fn sync_backing_store(canvas: &HtmlCanvasElement, dpr: f64) -> (f64, f64) {
    let css_w = (canvas.client_width().max(1)) as f64;
    let css_h = (canvas.client_height().max(1)) as f64;
    let phys_w = (css_w * dpr).round().max(1.0) as u32;
    let phys_h = (css_h * dpr).round().max(1.0) as u32;
    if canvas.width() != phys_w {
        canvas.set_width(phys_w);
    }
    if canvas.height() != phys_h {
        canvas.set_height(phys_h);
    }
    (css_w, css_h)
}

/// `FrameRequest.background` is `0xRRGGBBAA`. Alpha 0 means "unset" and is
/// painted opaque, matching the native hub path.
fn css_rgba(rgba: u32) -> String {
    let r = (rgba >> 24) & 0xff;
    let g = (rgba >> 16) & 0xff;
    let b = (rgba >> 8) & 0xff;
    let a = rgba & 0xff;
    let af = if a == 0 { 1.0 } else { a as f64 / 255.0 };
    format!("rgba({r},{g},{b},{af})")
}
