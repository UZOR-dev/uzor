//! Image draw — bilinear-filtered blit of a registered `ImageData`
//! into a destination rect with optional source crop.
//!
//! Premul source-over, honours clip stack (including rounded clips
//! via `ClipStack::pixel_coverage`). Sampling: bilinear when scaled,
//! 1:1 fast-path when dest size matches src size and transform is
//! identity.

use uzor_urx_core::math::{Affine, Point, Rect};
use uzor_urx_core::scene::ImageId;

use crate::clip::{transform_axis_aligned, ClipStack};
use crate::image_reg::{lookup_image, ImageDataArc};
use crate::pixmap::Pixmap;

pub(crate) fn draw_image_aa(
    pixmap:   &mut Pixmap,
    clip:     &ClipStack,
    src_id:   ImageId,
    src_rect: Option<Rect>,
    dest:     Rect,
    transform: &Affine,
    alpha:    f32,
) -> bool {
    let img = match lookup_image(src_id) {
        Some(d) => d,
        None => {
            metrics::counter!(
                uzor_urx_core::metrics_keys::KEY_RENDER_PRIMITIVES,
                "kind" => "image_id_unknown",
            ).increment(1);
            return false;
        }
    };

    let dst_screen = transform_axis_aligned(*transform, dest);
    let cur_clip = clip.current();
    let visible = dst_screen.intersect(cur_clip);
    if visible.width() <= 0.0 || visible.height() <= 0.0 { return true; }

    let src_box = src_rect.unwrap_or_else(|| Rect::new(0.0, 0.0, img.width as f64, img.height as f64));
    if src_box.width() <= 0.0 || src_box.height() <= 0.0 { return true; }

    blit_with_filter(pixmap, clip, &img, src_box, dst_screen, visible, alpha);
    true
}

/// Rasterize one screen-space textured triangle. Bilinear sample at the
/// barycentric UV, premul source-over, clip coverage, straight `alpha`.
/// Degenerate triangles (near-zero barycentric denom) are skipped.
pub(crate) fn draw_image_tri(
    pixmap: &mut Pixmap,
    clip:   &ClipStack,
    src_id: ImageId,
    p:      [Point; 3],
    uv:     [(f64, f64); 3],
    alpha:  f32,
) -> bool {
    let img = match lookup_image(src_id) {
        Some(d) => d,
        None => {
            metrics::counter!(
                uzor_urx_core::metrics_keys::KEY_RENDER_PRIMITIVES,
                "kind" => "image_id_unknown",
            ).increment(1);
            return false;
        }
    };

    let denom = (p[1].y - p[2].y) * (p[0].x - p[2].x) + (p[2].x - p[1].x) * (p[0].y - p[2].y);
    if !denom.is_finite() || denom.abs() < 1e-8 {
        return true;
    }

    let min_x = p[0].x.min(p[1].x).min(p[2].x);
    let max_x = p[0].x.max(p[1].x).max(p[2].x);
    let min_y = p[0].y.min(p[1].y).min(p[2].y);
    let max_y = p[0].y.max(p[1].y).max(p[2].y);
    let bounds = Rect::new(min_x, min_y, max_x, max_y);
    let visible = bounds.intersect(clip.current());
    if visible.width() <= 0.0 || visible.height() <= 0.0 {
        return true;
    }

    let w = pixmap.width() as i64;
    let h = pixmap.height() as i64;
    let ix0 = (visible.x0.floor() as i64).max(0);
    let iy0 = (visible.y0.floor() as i64).max(0);
    let ix1 = (visible.x1.ceil() as i64).min(w);
    let iy1 = (visible.y1.ceil() as i64).min(h);
    if ix0 >= ix1 || iy0 >= iy1 {
        return true;
    }

    let alpha_u8 = straight_alpha_u8(alpha);
    let use_mask = !clip.all_rect();
    let img_w = img.width as f64;
    let img_h = img.height as f64;

    for py in iy0..iy1 {
        let cy = py as f64 + 0.5;
        for px in ix0..ix1 {
            let cx = px as f64 + 0.5;
            let w0 = ((p[1].y - p[2].y) * (cx - p[2].x) + (p[2].x - p[1].x) * (cy - p[2].y)) / denom;
            let w1 = ((p[2].y - p[0].y) * (cx - p[2].x) + (p[0].x - p[2].x) * (cy - p[2].y)) / denom;
            let w2 = 1.0 - w0 - w1;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let u = w0 * uv[0].0 + w1 * uv[1].0 + w2 * uv[2].0;
            let v = w0 * uv[0].1 + w1 * uv[1].1 + w2 * uv[2].1;
            let sample = sample_bilinear(&img, u * img_w, v * img_h);
            if sample[3] == 0 {
                continue;
            }
            let mut cov: u32 = 255;
            if use_mask {
                cov = u32::from(clip.pixel_coverage(px, py));
                if cov == 0 {
                    continue;
                }
            }
            if alpha_u8 != 255 {
                cov = (cov * u32::from(alpha_u8) + 127) / 255;
            }
            if cov == 0 {
                continue;
            }
            let src = if cov == 255 { sample } else { scale_premul(sample, cov) };
            pixmap.blend_pixel(px as u32, py as u32, src);
        }
    }
    true
}

fn blit_with_filter(
    pixmap:   &mut Pixmap,
    clip:     &ClipStack,
    img:      &ImageDataArc,
    src_box:  Rect,
    dst_box:  Rect,
    visible:  Rect,
    alpha:    f32,
) {
    let w  = pixmap.width()  as i64;
    let h  = pixmap.height() as i64;
    let ix0 = (visible.x0.floor() as i64).max(0);
    let iy0 = (visible.y0.floor() as i64).max(0);
    let ix1 = (visible.x1.ceil()  as i64).min(w);
    let iy1 = (visible.y1.ceil()  as i64).min(h);
    if ix0 >= ix1 || iy0 >= iy1 { return; }

    let dst_w = (dst_box.x1 - dst_box.x0).max(1e-9);
    let dst_h = (dst_box.y1 - dst_box.y0).max(1e-9);
    let scale_x = src_box.width()  / dst_w;
    let scale_y = src_box.height() / dst_h;
    let unscaled = (scale_x - 1.0).abs() < 1e-4 && (scale_y - 1.0).abs() < 1e-4;
    let use_mask = !clip.all_rect();
    let alpha_u8 = straight_alpha_u8(alpha);

    let img_w = img.width as i64;
    let img_h = img.height as i64;

    for py in iy0 .. iy1 {
        let cy = py as f64 + 0.5;
        let src_y = src_box.y0 + (cy - dst_box.y0) * scale_y;
        for px in ix0 .. ix1 {
            let cx = px as f64 + 0.5;
            let src_x = src_box.x0 + (cx - dst_box.x0) * scale_x;
            let sample = if unscaled {
                let sx = src_x.floor() as i64;
                let sy = src_y.floor() as i64;
                if sx < 0 || sx >= img_w || sy < 0 || sy >= img_h { continue; }
                let idx = ((sy * img_w + sx) * 4) as usize;
                [img.bytes[idx], img.bytes[idx+1], img.bytes[idx+2], img.bytes[idx+3]]
            } else {
                sample_bilinear(img, src_x, src_y)
            };
            if sample[3] == 0 { continue; }
            let mut cov: u32 = 255;
            if use_mask {
                cov = u32::from(clip.pixel_coverage(px, py));
                if cov == 0 { continue; }
            }
            if alpha_u8 != 255 {
                cov = (cov * u32::from(alpha_u8) + 127) / 255;
            }
            if cov == 0 { continue; }
            let src = if cov == 255 { sample } else { scale_premul(sample, cov) };
            pixmap.blend_pixel(px as u32, py as u32, src);
        }
    }
}

fn straight_alpha_u8(alpha: f32) -> u8 {
    if !alpha.is_finite() {
        return 0;
    }
    (alpha.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn scale_premul(src: [u8; 4], factor: u32) -> [u8; 4] {
    [
        ((u32::from(src[0]) * factor + 127) / 255) as u8,
        ((u32::from(src[1]) * factor + 127) / 255) as u8,
        ((u32::from(src[2]) * factor + 127) / 255) as u8,
        ((u32::from(src[3]) * factor + 127) / 255) as u8,
    ]
}

#[inline]
fn sample_bilinear(img: &ImageDataArc, x: f64, y: f64) -> [u8; 4] {
    // Pixel-centre alignment: sample at (x - 0.5, y - 0.5) so integer
    // x lands between texels not on a texel centre.
    let fx = x - 0.5;
    let fy = y - 0.5;
    let x0 = fx.floor() as i64;
    let y0 = fy.floor() as i64;
    let tx = fx - x0 as f64;
    let ty = fy - y0 as f64;

    let img_w = img.width as i64;
    let img_h = img.height as i64;
    let s = |sx: i64, sy: i64| -> [f64; 4] {
        let cx = sx.clamp(0, img_w - 1) as usize;
        let cy = sy.clamp(0, img_h - 1) as usize;
        let idx = (cy * img.width as usize + cx) * 4;
        [
            img.bytes[idx]   as f64,
            img.bytes[idx+1] as f64,
            img.bytes[idx+2] as f64,
            img.bytes[idx+3] as f64,
        ]
    };
    let p00 = s(x0,     y0    );
    let p10 = s(x0 + 1, y0    );
    let p01 = s(x0,     y0 + 1);
    let p11 = s(x0 + 1, y0 + 1);

    let w00 = (1.0 - tx) * (1.0 - ty);
    let w10 = tx         * (1.0 - ty);
    let w01 = (1.0 - tx) * ty;
    let w11 = tx         * ty;
    let mut out = [0.0f64; 4];
    for i in 0..4 {
        out[i] = p00[i] * w00 + p10[i] * w10 + p01[i] * w01 + p11[i] * w11;
    }
    [
        out[0].round().clamp(0.0, 255.0) as u8,
        out[1].round().clamp(0.0, 255.0) as u8,
        out[2].round().clamp(0.0, 255.0) as u8,
        out[3].round().clamp(0.0, 255.0) as u8,
    ]
}
