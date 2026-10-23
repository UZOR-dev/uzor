//! Image viewer (lightbox): a full-window dimmed overlay with the selected
//! image centred, an "alt · N / M" caption, a thumbnail strip when the image
//! belongs to a series, and round download / close buttons in the top-right
//! corner (left of any window controls the host reserves there).
//!
//! Pure geometry + paint + hit-test; the host owns the open state, the
//! selection and what download / close do. [`key_nav`] maps arrow / Escape
//! keys to the next selection.

mod tokens;

use crate::input::KeyCode;
use crate::render::RenderContext;
use crate::types::Rect;

/// One image of the series. `rgba` is `w`×`h` RGBA8 (premultiplied as the
/// backend expects for `draw_image_rgba`).
#[derive(Clone, Copy, Debug)]
pub struct ViewerImage<'a> {
    /// Pixels.
    pub rgba: &'a [u8],
    /// Width, px.
    pub w: u32,
    /// Height, px.
    pub h: u32,
    /// Alt text / name shown in the caption.
    pub alt: &'a str,
}

/// Look and geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewerStyle {
    /// Full-window dim.
    pub backdrop: &'static str,
    /// Image corner radius.
    pub radius: f64,
    /// Margin kept around the image (each side), fraction of the viewport.
    pub margin_frac: f64,
    /// Caption colour / size.
    pub caption: &'static str,
    /// Caption font size.
    pub caption_px: f64,
    /// Thumbnail size (square box) and gap.
    pub thumb: f64,
    /// Gap between thumbnails.
    pub thumb_gap: f64,
    /// Thumbnail strip bottom inset.
    pub thumb_bottom: f64,
    /// Selected thumbnail ring.
    pub thumb_ring: &'static str,
    /// Unselected thumbnail dim (drawn over the thumb).
    pub thumb_dim: &'static str,
    /// Round button diameter.
    pub btn: f64,
    /// Round button gap.
    pub btn_gap: f64,
    /// Round button fill / hover fill / icon colour.
    pub btn_bg: &'static str,
    /// Hover fill.
    pub btn_hover: &'static str,
    /// Icon colour.
    pub btn_icon: &'static str,
}

impl Default for ViewerStyle {
    fn default() -> Self {
        Self {
            backdrop: tokens::BACKDROP,
            radius: 10.0,
            margin_frac: 0.1,
            caption: tokens::CAPTION,
            caption_px: 13.0,
            thumb: 46.0,
            thumb_gap: 10.0,
            thumb_bottom: 22.0,
            thumb_ring: tokens::THUMB_RING,
            thumb_dim: tokens::THUMB_DIM,
            btn: 30.0,
            btn_gap: 8.0,
            btn_bg: tokens::BTN_BG,
            btn_hover: tokens::BTN_HOVER,
            btn_icon: tokens::BTN_ICON,
        }
    }
}

/// Solved rects.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewerLayout {
    /// The selected image.
    pub image: Rect,
    /// Caption baseline box (centred text inside).
    pub caption: Rect,
    /// One rect per image; empty for a single image.
    pub thumbs: Vec<Rect>,
    /// Download button.
    pub download: Rect,
    /// Close button.
    pub close: Rect,
}

/// What a point hits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewerHit {
    /// Close (X).
    Close,
    /// Download.
    Download,
    /// Thumbnail `i`.
    Thumb(usize),
    /// The image itself.
    Image,
    /// The dimmed backdrop (click = close).
    Backdrop,
}

/// Lay the viewer out in `viewport`. `img` = selected image size, `n` = series
/// length, `reserve_right` = width kept free at the top-right for window
/// controls (the round buttons sit left of it), `top` = their row height.
pub fn layout(viewport: Rect, img: (u32, u32), n: usize, reserve_right: f64, top: f64, st: &ViewerStyle) -> ViewerLayout {
    let strip_h = if n > 1 { st.thumb + st.thumb_bottom + 12.0 } else { 0.0 };
    let cap_h = st.caption_px * 1.5 + 12.0;
    let mx = viewport.width * st.margin_frac;
    let avail_w = (viewport.width - 2.0 * mx).max(1.0);
    let top_m = (top + 16.0).max(viewport.height * st.margin_frac * 0.6);
    let avail_h = (viewport.height - top_m - strip_h - cap_h - 24.0).max(1.0);
    let (iw, ih) = (img.0.max(1) as f64, img.1.max(1) as f64);
    let k = (avail_w / iw).min(avail_h / ih);
    let (w, h) = ((iw * k).round(), (ih * k).round());
    let x = (viewport.x + (viewport.width - w) / 2.0).round();
    let y = (viewport.y + top_m + (avail_h - h) / 2.0).round();
    let image = Rect::new(x, y, w, h);
    let caption = Rect::new(viewport.x, y + h + 12.0, viewport.width, st.caption_px * 1.5);
    let mut thumbs = Vec::new();
    if n > 1 {
        let total = n as f64 * st.thumb + (n as f64 - 1.0) * st.thumb_gap;
        let tx = (viewport.x + (viewport.width - total) / 2.0).round();
        let ty = viewport.y + viewport.height - st.thumb_bottom - st.thumb;
        for i in 0..n {
            thumbs.push(Rect::new(tx + i as f64 * (st.thumb + st.thumb_gap), ty, st.thumb, st.thumb));
        }
    }
    let by = viewport.y + ((top - st.btn) / 2.0).max(1.0);
    let close = Rect::new(viewport.x + viewport.width - reserve_right - 12.0 - st.btn, by, st.btn, st.btn);
    let download = Rect::new(close.x - st.btn_gap - st.btn, by, st.btn, st.btn);
    ViewerLayout { image, caption, thumbs, download, close }
}

fn inside(r: Rect, p: (f64, f64)) -> bool {
    p.0 >= r.x && p.0 < r.x + r.width && p.1 >= r.y && p.1 < r.y + r.height
}

fn in_circle(r: Rect, p: (f64, f64)) -> bool {
    let (cx, cy, rr) = (r.x + r.width / 2.0, r.y + r.height / 2.0, r.width / 2.0);
    (p.0 - cx).powi(2) + (p.1 - cy).powi(2) <= rr * rr
}

/// The part under `p` (anything inside the viewport is at least `Backdrop`).
pub fn hit(l: &ViewerLayout, p: (f64, f64)) -> ViewerHit {
    if in_circle(l.close, p) {
        return ViewerHit::Close;
    }
    if in_circle(l.download, p) {
        return ViewerHit::Download;
    }
    if let Some(i) = l.thumbs.iter().position(|r| inside(*r, p)) {
        return ViewerHit::Thumb(i);
    }
    if inside(l.image, p) {
        return ViewerHit::Image;
    }
    ViewerHit::Backdrop
}

/// Keyboard: Left / Right move the selection (clamped), Escape closes
/// (`None`). Other keys keep the selection.
pub fn key_nav(sel: usize, n: usize, key: KeyCode) -> Option<usize> {
    match key {
        KeyCode::Escape => None,
        KeyCode::ArrowLeft => Some(sel.saturating_sub(1)),
        KeyCode::ArrowRight => Some((sel + 1).min(n.saturating_sub(1))),
        KeyCode::Home => Some(0),
        KeyCode::End => Some(n.saturating_sub(1)),
        _ => Some(sel),
    }
}

/// "alt · N / M" (just "alt" for a single image).
pub fn caption_text(alt: &str, sel: usize, n: usize) -> String {
    if n > 1 {
        if alt.is_empty() { format!("{} / {}", sel + 1, n) } else { format!("{alt} · {} / {}", sel + 1, n) }
    } else {
        alt.to_string()
    }
}

fn draw_img(ctx: &mut dyn RenderContext, r: Rect, im: &ViewerImage<'_>, radius: f64) {
    ctx.save();
    ctx.begin_path();
    ctx.rounded_rect(r.x, r.y, r.width, r.height, radius);
    ctx.clip();
    if let Some(ip) = ctx.image_painter() {
        ip.draw_image_rgba(im.rgba, im.w, im.h, r.x, r.y, r.width, r.height);
    }
    ctx.restore();
}

/// Cover-fit `img` into the square `r` (thumbnail crop): the source sub-rect
/// is not expressible through `draw_image_rgba`, so the image is drawn larger
/// and clipped to `r`.
fn draw_thumb(ctx: &mut dyn RenderContext, r: Rect, im: &ViewerImage<'_>, radius: f64) {
    let (iw, ih) = (im.w.max(1) as f64, im.h.max(1) as f64);
    let k = (r.width / iw).max(r.height / ih);
    let (w, h) = (iw * k, ih * k);
    ctx.save();
    ctx.begin_path();
    ctx.rounded_rect(r.x, r.y, r.width, r.height, radius);
    ctx.clip();
    if let Some(ip) = ctx.image_painter() {
        ip.draw_image_rgba(im.rgba, im.w, im.h, r.x + (r.width - w) / 2.0, r.y + (r.height - h) / 2.0, w, h);
    }
    ctx.restore();
}

fn round_btn(ctx: &mut dyn RenderContext, r: Rect, hot: bool, st: &ViewerStyle) {
    ctx.set_fill_color(if hot { st.btn_hover } else { st.btn_bg });
    ctx.begin_path();
    ctx.arc(r.x + r.width / 2.0, r.y + r.height / 2.0, r.width / 2.0, 0.0, std::f64::consts::TAU);
    ctx.fill();
    ctx.set_stroke_color(st.btn_icon);
    ctx.set_stroke_width(1.5);
}

/// Paint the viewer over `viewport`.
pub fn draw(ctx: &mut dyn RenderContext, viewport: Rect, l: &ViewerLayout, images: &[ViewerImage<'_>], sel: usize, hover: Option<ViewerHit>, font: &str, st: &ViewerStyle) {
    ctx.set_fill_color(st.backdrop);
    ctx.fill_rect(viewport.x, viewport.y, viewport.width, viewport.height);
    let n = images.len();
    if let Some(im) = images.get(sel) {
        draw_img(ctx, l.image, im, st.radius);
        ctx.set_font(font);
        ctx.set_fill_color(st.caption);
        ctx.set_text_align(crate::render::TextAlign::Center);
        ctx.set_text_baseline(crate::render::TextBaseline::Middle);
        ctx.fill_text(&caption_text(im.alt, sel, n), l.caption.x + l.caption.width / 2.0, l.caption.y + l.caption.height / 2.0);
    }
    for (i, (r, im)) in l.thumbs.iter().zip(images).enumerate() {
        draw_thumb(ctx, *r, im, 6.0);
        if i == sel {
            ctx.set_stroke_color(st.thumb_ring);
            ctx.set_stroke_width(2.0);
            ctx.begin_path();
            ctx.rounded_rect(r.x - 2.0, r.y - 2.0, r.width + 4.0, r.height + 4.0, 8.0);
            ctx.stroke();
        } else if hover != Some(ViewerHit::Thumb(i)) {
            ctx.set_fill_color(st.thumb_dim);
            ctx.begin_path();
            ctx.rounded_rect(r.x, r.y, r.width, r.height, 6.0);
            ctx.fill();
        }
    }
    // Download: tray + down arrow.
    let d = l.download;
    round_btn(ctx, d, hover == Some(ViewerHit::Download), st);
    let (cx, cy) = (d.x + d.width / 2.0, d.y + d.height / 2.0);
    ctx.begin_path();
    ctx.move_to(cx, cy - 7.0);
    ctx.line_to(cx, cy + 3.0);
    ctx.move_to(cx - 4.0, cy - 1.0);
    ctx.line_to(cx, cy + 3.0);
    ctx.line_to(cx + 4.0, cy - 1.0);
    ctx.move_to(cx - 7.0, cy + 2.0);
    ctx.line_to(cx - 7.0, cy + 7.0);
    ctx.line_to(cx + 7.0, cy + 7.0);
    ctx.line_to(cx + 7.0, cy + 2.0);
    ctx.stroke();
    // Close: X.
    let c = l.close;
    round_btn(ctx, c, hover == Some(ViewerHit::Close), st);
    let (cx, cy) = (c.x + c.width / 2.0, c.y + c.height / 2.0);
    ctx.begin_path();
    ctx.move_to(cx - 6.0, cy - 6.0);
    ctx.line_to(cx + 6.0, cy + 6.0);
    ctx.move_to(cx + 6.0, cy - 6.0);
    ctx.line_to(cx - 6.0, cy + 6.0);
    ctx.stroke();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vp() -> Rect {
        Rect::new(0.0, 0.0, 1280.0, 800.0)
    }

    #[test]
    fn series_layout_centres_image_and_thumbs() {
        let st = ViewerStyle::default();
        let l = layout(vp(), (1920, 1200), 4, 138.0, 32.0, &st);
        assert!((l.image.x + l.image.width / 2.0 - 640.0).abs() <= 1.0);
        assert_eq!(l.thumbs.len(), 4);
        let total = l.thumbs[3].x + l.thumbs[3].width - l.thumbs[0].x;
        assert!((l.thumbs[0].x + total / 2.0 - 640.0).abs() <= 1.0);
        assert!(l.image.y + l.image.height < l.thumbs[0].y);
        // Buttons left of the reserved controls, close right of download.
        assert!(l.close.x + l.close.width <= 1280.0 - 138.0);
        assert!(l.download.x < l.close.x);
    }

    #[test]
    fn single_image_has_no_strip() {
        let l = layout(vp(), (400, 300), 1, 0.0, 32.0, &ViewerStyle::default());
        assert!(l.thumbs.is_empty());
        assert_eq!(caption_text("a.png", 0, 1), "a.png");
        assert_eq!(caption_text("a.png", 1, 6), "a.png · 2 / 6");
    }

    #[test]
    fn hits_and_keys() {
        let l = layout(vp(), (1920, 1200), 3, 138.0, 32.0, &ViewerStyle::default());
        let c = (l.close.x + l.close.width / 2.0, l.close.y + l.close.height / 2.0);
        assert_eq!(hit(&l, c), ViewerHit::Close);
        let t = (l.thumbs[2].x + 5.0, l.thumbs[2].y + 5.0);
        assert_eq!(hit(&l, t), ViewerHit::Thumb(2));
        assert_eq!(hit(&l, (l.image.x + 3.0, l.image.y + 3.0)), ViewerHit::Image);
        assert_eq!(hit(&l, (2.0, 400.0)), ViewerHit::Backdrop);
        assert_eq!(key_nav(1, 3, KeyCode::ArrowRight), Some(2));
        assert_eq!(key_nav(2, 3, KeyCode::ArrowRight), Some(2));
        assert_eq!(key_nav(0, 3, KeyCode::ArrowLeft), Some(0));
        assert_eq!(key_nav(1, 3, KeyCode::Escape), None);
    }
}
