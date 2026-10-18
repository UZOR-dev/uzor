//! Remote frame view: a raster surface streamed from elsewhere (an embedded
//! browser, a remote screen) shown in a rect, plus the pointer mapping back
//! to source pixels (app.js `wireCanvas` `pos()`).

use crate::types::Rect;
use crate::render::RenderContext;

/// Height for `width` that keeps the `vw`×`vh` aspect.
pub fn aspect_height(width: f64, vw: f64, vh: f64) -> f64 {
    if vw <= 0.0 { 0.0 } else { width * vh / vw }
}

/// Map a window point inside `r` to source pixel `(x, y)` of a `vw`×`vh`
/// frame, rounded and clamped to `[0, vw-1]`×`[0, vh-1]`.
pub fn to_source(r: Rect, vw: f64, vh: f64, p: (f64, f64)) -> (f64, f64) {
    let x = ((p.0 - r.x) * vw / r.width.max(1.0)).round().clamp(0.0, (vw - 1.0).max(0.0));
    let y = ((p.1 - r.y) * vh / r.height.max(1.0)).round().clamp(0.0, (vh - 1.0).max(0.0));
    (x, y)
}

/// Paint an RGBA frame stretched into `r` over a `bg` fill. Returns `false`
/// when there is no frame or the backend has no image capability (only the
/// background is painted then).
pub fn draw_frame(ctx: &mut dyn RenderContext, r: Rect, rgba: &[u8], w: u32, h: u32, bg: &str) -> bool {
    ctx.set_fill_color(bg);
    ctx.fill_rect(r.x, r.y, r.width, r.height);
    if w == 0 || h == 0 || rgba.len() < (w as usize) * (h as usize) * 4 {
        return false;
    }
    match ctx.image_painter() {
        Some(ip) => {
            ip.draw_image_rgba(rgba, w, h, r.x, r.y, r.width, r.height);
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_and_clamps() {
        let r = Rect::new(10.0, 20.0, 640.0, 360.0);
        assert_eq!(to_source(r, 1280.0, 720.0, (10.0, 20.0)), (0.0, 0.0));
        assert_eq!(to_source(r, 1280.0, 720.0, (330.0, 200.0)), (640.0, 360.0));
        assert_eq!(to_source(r, 1280.0, 720.0, (9999.0, -5.0)), (1279.0, 0.0));
        assert_eq!(aspect_height(400.0, 1280.0, 720.0), 225.0);
    }
}
