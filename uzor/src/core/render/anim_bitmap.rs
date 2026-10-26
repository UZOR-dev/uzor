//! Bitmap side channel for anim ops 22–25.
//!
//! Triangle draws in one run share a user-space patch. The patch is one
//! `draw_image_rgba`, so the backend CTM (including shear) and clip apply
//! once. A missing slot is skipped.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

use super::anim_script::{op, AnimScriptError};
use super::context::RenderContext;

/// Straight-alpha RGBA owned by the script host. `gen` bumps on replace so a
/// blur cache does not reuse pixels from the previous contents of the slot.
#[derive(Clone, Debug, Default)]
pub struct AnimBitmap {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
    pub gen: u64,
}

/// Replay `ops`. `bitmaps` is indexed by the slot id the script published.
pub fn replay_anim_ops_bitmaps(
    ctx: &mut dyn RenderContext,
    ops: &[f64],
    bitmaps: &[AnimBitmap],
) -> Result<(), AnimScriptError> {
    let mut i = 0;
    let mut depth = 0usize;
    let mut blur = 0.0f64;
    let mut blur_stack: Vec<f64> = Vec::new();
    let mut res = Ok(());
    while i < ops.len() {
        let code = ops[i];
        if code == op::DRAW_TRI as f64 {
            match take_tris(ops, i) {
                Ok((slot, tris, next)) => {
                    if let Some(slot) = slot {
                        paint_tris(ctx, bitmaps, slot, &tris);
                    }
                    i = next;
                    continue;
                }
                Err(at) => {
                    res = Err(AnimScriptError::BadOp(at));
                    break;
                }
            }
        }
        let Some(n) = (code >= 0.0 && code < 256.0 && code.fract() == 0.0)
            .then(|| code as u8)
            .and_then(op::arity)
        else {
            res = Err(AnimScriptError::BadOp(i));
            break;
        };
        if i + 1 + n > ops.len() {
            res = Err(AnimScriptError::BadOp(i));
            break;
        }
        let a = &ops[i + 1..i + 1 + n];
        match code as u8 {
            op::SAVE => {
                depth += 1;
                blur_stack.push(blur);
                ctx.save();
            }
            op::RESTORE => {
                if depth > 0 {
                    depth -= 1;
                    ctx.restore();
                }
                if let Some(prev) = blur_stack.pop() {
                    blur = prev;
                }
            }
            op::TRANSLATE => ctx.translate(a[0], a[1]),
            op::ROTATE => ctx.rotate(a[0]),
            op::SCALE => ctx.scale(a[0], a[1]),
            op::BEGIN_PATH => ctx.begin_path(),
            op::MOVE_TO => ctx.move_to(a[0], a[1]),
            op::LINE_TO => ctx.line_to(a[0], a[1]),
            op::QUAD_TO => ctx.quadratic_curve_to(a[0], a[1], a[2], a[3]),
            op::BEZIER_TO => ctx.bezier_curve_to(a[0], a[1], a[2], a[3], a[4], a[5]),
            op::ARC => ctx.arc(a[0], a[1], a[2], a[3], a[4]),
            op::ELLIPSE => ctx.ellipse(a[0], a[1], a[2], a[3], a[4], a[5], a[6]),
            op::CLOSE_PATH => ctx.close_path(),
            op::FILL => ctx.fill(),
            op::STROKE => ctx.stroke(),
            op::FILL_COLOR | op::STROKE_COLOR => {
                let hex = rgba_hex(a[0] as u32);
                let s = core::str::from_utf8(&hex).unwrap_or("#000000ff");
                if code as u8 == op::FILL_COLOR {
                    ctx.set_fill_color(s);
                } else {
                    ctx.set_stroke_color(s);
                }
            }
            op::LINE_WIDTH => ctx.set_stroke_width(a[0]),
            op::GLOBAL_ALPHA => ctx.set_global_alpha(a[0]),
            op::FILL_RECT => ctx.fill_rect(a[0], a[1], a[2], a[3]),
            op::STROKE_RECT => ctx.stroke_rect(a[0], a[1], a[2], a[3]),
            op::CLIP => ctx.clip(),
            op::TRANSFORM => ctx.transform(a[0], a[1], a[2], a[3], a[4], a[5]),
            op::FILTER => blur = a[0],
            op::DRAW_IMAGE => blit_image(
                ctx, bitmaps, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8], blur,
            ),
            op::DRAW_TRI => {}
            _ => unreachable!(),
        }
        i += 1 + n;
    }
    for _ in 0..depth {
        ctx.restore();
    }
    res
}

fn rgba_hex(c: u32) -> [u8; 9] {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = [b'#'; 9];
    for i in 0..8 {
        s[1 + i] = HEX[((c >> (28 - 4 * i)) & 0xf) as usize];
    }
    s
}

fn slot_index(v: f64) -> Option<usize> {
    if v < 0.0 || v.fract() != 0.0 || v > u32::MAX as f64 {
        None
    } else {
        Some(v as usize)
    }
}

/// Consecutive `DRAW_TRI` that share one slot. A different slot stops the run
/// without consuming it.
fn take_tris(ops: &[f64], mut i: usize) -> Result<(Option<usize>, Vec<[f64; 12]>, usize), usize> {
    let mut slot: Option<usize> = None;
    let mut tris = Vec::new();
    let start = i;
    while i < ops.len() && ops[i] == op::DRAW_TRI as f64 {
        if i + 1 + 13 > ops.len() {
            return Err(i);
        }
        let a = &ops[i + 1..i + 1 + 13];
        let this = slot_index(a[0]);
        if tris.is_empty() {
            slot = this;
        } else if this != slot {
            break;
        }
        if this.is_some() {
            let mut t = [0.0; 12];
            t.copy_from_slice(&a[1..13]);
            tris.push(t);
        }
        i += 1 + 13;
    }
    if i == start {
        return Err(start);
    }
    Ok((slot, tris, i))
}

fn paint_tris(ctx: &mut dyn RenderContext, bitmaps: &[AnimBitmap], slot: usize, tris: &[[f64; 12]]) {
    if tris.is_empty() {
        return;
    }
    let Some(bm) = bitmaps.get(slot) else {
        return;
    };
    let need = bm.w as usize * bm.h as usize * 4;
    if bm.w == 0 || bm.h == 0 || bm.rgba.len() < need {
        return;
    }
    let mut minx = f64::INFINITY;
    let mut miny = f64::INFINITY;
    let mut maxx = f64::NEG_INFINITY;
    let mut maxy = f64::NEG_INFINITY;
    for t in tris {
        for k in 0..3 {
            let x = t[6 + k * 2];
            let y = t[7 + k * 2];
            minx = minx.min(x);
            miny = miny.min(y);
            maxx = maxx.max(x);
            maxy = maxy.max(y);
        }
    }
    if !minx.is_finite() || !miny.is_finite() {
        return;
    }
    let x0 = minx.floor();
    let y0 = miny.floor();
    let pw = (maxx.ceil() - x0).ceil() as i32;
    let ph = (maxy.ceil() - y0).ceil() as i32;
    if pw <= 0 || ph <= 0 || pw > 1600 || ph > 1600 || pw as i64 * ph as i64 > 1_600_000 {
        return;
    }
    let mut patch = vec![0u8; (pw as usize) * (ph as usize) * 4];
    raster_tris(&bm.rgba, bm.w as i32, bm.h as i32, tris, &mut patch, pw, ph, x0, y0);
    blit_rgba(ctx, &patch, pw as u32, ph as u32, x0, y0, pw as f64, ph as f64);
}

fn raster_tris(
    src: &[u8],
    sw: i32,
    sh: i32,
    tris: &[[f64; 12]],
    patch: &mut [u8],
    pw: i32,
    ph: i32,
    origin_x: f64,
    origin_y: f64,
) {
    for t in tris {
        let sx0 = t[0];
        let sy0 = t[1];
        let sx1 = t[2];
        let sy1 = t[3];
        let sx2 = t[4];
        let sy2 = t[5];
        let ax = t[6] - origin_x;
        let ay = t[7] - origin_y;
        let bx = t[8] - origin_x;
        let by = t[9] - origin_y;
        let cx = t[10] - origin_x;
        let cy = t[11] - origin_y;
        let den = (by - cy) * (ax - cx) + (cx - bx) * (ay - cy);
        if den.abs() < 1.0e-8 {
            continue;
        }
        let minx = ax.min(bx).min(cx).floor().max(0.0) as i32;
        let miny = ay.min(by).min(cy).floor().max(0.0) as i32;
        let maxx = ax.max(bx).max(cx).ceil().min((pw - 1) as f64) as i32;
        let maxy = ay.max(by).max(cy).ceil().min((ph - 1) as f64) as i32;
        for y in miny..=maxy {
            for x in minx..=maxx {
                let px = x as f64 + 0.5;
                let py = y as f64 + 0.5;
                let w0 = ((by - cy) * (px - cx) + (cx - bx) * (py - cy)) / den;
                let w1 = ((cy - ay) * (px - cx) + (ax - cx) * (py - cy)) / den;
                let w2 = 1.0 - w0 - w1;
                if w0 < -0.001 || w1 < -0.001 || w2 < -0.001 {
                    continue;
                }
                let su = w0 * sx0 + w1 * sx1 + w2 * sx2;
                let sv = w0 * sy0 + w1 * sy1 + w2 * sy2;
                let (r, g, b, a) = bilinear(src, sw, sh, su, sv);
                let i = ((y as usize) * (pw as usize) + (x as usize)) * 4;
                source_over(patch, i, r, g, b, a);
            }
        }
    }
}

fn bilinear(src: &[u8], sw: i32, sh: i32, x: f64, y: f64) -> (u8, u8, u8, u8) {
    if !(x.is_finite() && y.is_finite()) || sw <= 0 || sh <= 0 {
        return (0, 0, 0, 0);
    }
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let tx = (x - x0 as f64) as f32;
    let ty = (y - y0 as f64) as f32;
    let c00 = sample(src, sw, sh, x0, y0);
    let c10 = sample(src, sw, sh, x0 + 1, y0);
    let c01 = sample(src, sw, sh, x0, y0 + 1);
    let c11 = sample(src, sw, sh, x0 + 1, y0 + 1);
    let mut out = [0u8; 4];
    for c in 0..4 {
        let a = c00[c] as f32;
        let b = c10[c] as f32;
        let d = c01[c] as f32;
        let e = c11[c] as f32;
        let v = a + (b - a) * tx;
        let w = d + (e - d) * tx;
        out[c] = (v + (w - v) * ty).round().clamp(0.0, 255.0) as u8;
    }
    (out[0], out[1], out[2], out[3])
}

fn sample(src: &[u8], sw: i32, sh: i32, x: i32, y: i32) -> [u8; 4] {
    if x < 0 || y < 0 || x >= sw || y >= sh {
        return [0, 0, 0, 0];
    }
    let i = ((y as usize) * (sw as usize) + (x as usize)) * 4;
    if i + 4 > src.len() {
        return [0, 0, 0, 0];
    }
    [src[i], src[i + 1], src[i + 2], src[i + 3]]
}

fn source_over(px: &mut [u8], i: usize, sr: u8, sg: u8, sb: u8, sa: u8) {
    if sa == 0 || i + 4 > px.len() {
        return;
    }
    if sa == 255 {
        px[i] = sr;
        px[i + 1] = sg;
        px[i + 2] = sb;
        px[i + 3] = 255;
        return;
    }
    let sa = sa as u32;
    let da = px[i + 3] as u32;
    let inv = 255 - sa;
    let out_a = sa + da * inv / 255;
    if out_a == 0 {
        return;
    }
    px[i] = ((sr as u32 * sa + px[i] as u32 * da * inv / 255) / out_a) as u8;
    px[i + 1] = ((sg as u32 * sa + px[i + 1] as u32 * da * inv / 255) / out_a) as u8;
    px[i + 2] = ((sb as u32 * sa + px[i + 2] as u32 * da * inv / 255) / out_a) as u8;
    px[i + 3] = out_a as u8;
}

fn blit_image(
    ctx: &mut dyn RenderContext,
    bitmaps: &[AnimBitmap],
    slot: f64,
    sx: f64,
    sy: f64,
    sw: f64,
    sh: f64,
    dx: f64,
    dy: f64,
    dw: f64,
    dh: f64,
    blur: f64,
) {
    let Some(slot) = slot_index(slot) else {
        return;
    };
    let Some(bm) = bitmaps.get(slot) else {
        return;
    };
    let need = bm.w as usize * bm.h as usize * 4;
    if bm.w == 0 || bm.h == 0 || bm.rgba.len() < need || sw <= 0.0 || sh <= 0.0 {
        return;
    }
    let radius = blur.round().clamp(0.0, 8.0) as u32;
    if radius == 0 {
        blit_source(ctx, &bm.rgba, bm.w, bm.h, sx, sy, sw, sh, dx, dy, dw, dh);
        return;
    }
    with_blur(slot as u32, bm.gen, radius, &bm.rgba, bm.w, bm.h, |bytes| {
        blit_source(ctx, bytes, bm.w, bm.h, sx, sy, sw, sh, dx, dy, dw, dh);
    });
}

fn blit_source(
    ctx: &mut dyn RenderContext,
    src: &[u8],
    iw: u32,
    ih: u32,
    sx: f64,
    sy: f64,
    sw: f64,
    sh: f64,
    dx: f64,
    dy: f64,
    dw: f64,
    dh: f64,
) {
    let full = sx.abs() < 0.5
        && sy.abs() < 0.5
        && (sw - iw as f64).abs() < 0.5
        && (sh - ih as f64).abs() < 0.5;
    if full {
        blit_rgba(ctx, src, iw, ih, dx, dy, dw, dh);
        return;
    }
    let Some((buf, rw, rh)) = extract(src, iw, ih, sx, sy, sw, sh) else {
        return;
    };
    blit_rgba(ctx, &buf, rw, rh, dx, dy, dw, dh);
}

fn blit_rgba(
    ctx: &mut dyn RenderContext,
    data: &[u8],
    w: u32,
    h: u32,
    dx: f64,
    dy: f64,
    dw: f64,
    dh: f64,
) {
    if w == 0 || h == 0 || data.len() < (w as usize) * (h as usize) * 4 {
        return;
    }
    let adw = dw.abs();
    let adh = dh.abs();
    if adw < 1.0e-6 || adh < 1.0e-6 {
        return;
    }
    let flip_x = dw < 0.0;
    let flip_y = dh < 0.0;
    if flip_x || flip_y {
        ctx.save();
        ctx.translate(dx, dy);
        if flip_x {
            ctx.scale(-1.0, 1.0);
        }
        if flip_y {
            ctx.scale(1.0, -1.0);
        }
        if let Some(painter) = ctx.image_painter() {
            painter.draw_image_rgba(data, w, h, 0.0, 0.0, adw, adh);
        }
        ctx.restore();
        return;
    }
    if let Some(painter) = ctx.image_painter() {
        painter.draw_image_rgba(data, w, h, dx, dy, adw, adh);
    }
}

fn extract(src: &[u8], iw: u32, ih: u32, sx: f64, sy: f64, sw: f64, sh: f64) -> Option<(Vec<u8>, u32, u32)> {
    let x0 = sx.round() as i32;
    let y0 = sy.round() as i32;
    let rw = sw.round() as i32;
    let rh = sh.round() as i32;
    if rw <= 0 || rh <= 0 {
        return None;
    }
    let x1 = (x0 + rw).min(iw as i32).max(0);
    let y1 = (y0 + rh).min(ih as i32).max(0);
    let x0 = x0.max(0);
    let y0 = y0.max(0);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let ow = (x1 - x0) as u32;
    let oh = (y1 - y0) as u32;
    let mut out = vec![0u8; (ow * oh * 4) as usize];
    for row in 0..oh {
        let s = ((y0 as u32 + row) * iw + x0 as u32) as usize * 4;
        let d = (row * ow) as usize * 4;
        let n = (ow * 4) as usize;
        if s + n > src.len() || d + n > out.len() {
            return None;
        }
        out[d..d + n].copy_from_slice(&src[s..s + n]);
    }
    Some((out, ow, oh))
}

struct BlurCache {
    map: HashMap<(u32, u64, u32), Vec<u8>>,
    order: VecDeque<(u32, u64, u32)>,
}

fn with_blur<R>(
    slot: u32,
    gen: u64,
    radius: u32,
    src: &[u8],
    w: u32,
    h: u32,
    f: impl FnOnce(&[u8]) -> R,
) -> R {
    thread_local! {
        static CACHE: RefCell<BlurCache> = RefCell::new(BlurCache {
            map: HashMap::new(),
            order: VecDeque::new(),
        });
    }
    CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        let key = (slot, gen, radius);
        if !cache.map.contains_key(&key) {
            while cache.order.len() >= 6 {
                if let Some(old) = cache.order.pop_front() {
                    cache.map.remove(&old);
                }
            }
            cache.map.insert(key, box_blur(src, w, h, radius));
            cache.order.push_back(key);
        }
        let bytes = cache.map.get(&key).expect("blur just inserted");
        f(bytes)
    })
}

fn box_blur(src: &[u8], w: u32, h: u32, radius: u32) -> Vec<u8> {
    let w = w as usize;
    let h = h as usize;
    let r = radius as usize;
    if w == 0 || h == 0 || src.len() < w * h * 4 || r == 0 {
        return src.to_vec();
    }
    let mut tmp = vec![0u8; w * h * 4];
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0u32; 4];
            let mut n = 0u32;
            let x0 = x.saturating_sub(r);
            let x1 = (x + r).min(w - 1);
            for xx in x0..=x1 {
                let i = (y * w + xx) * 4;
                for c in 0..4 {
                    acc[c] += src[i + c] as u32;
                }
                n += 1;
            }
            let o = (y * w + x) * 4;
            for c in 0..4 {
                tmp[o + c] = (acc[c] / n) as u8;
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0u32; 4];
            let mut n = 0u32;
            let y0 = y.saturating_sub(r);
            let y1 = (y + r).min(h - 1);
            for yy in y0..=y1 {
                let i = (yy * w + x) * 4;
                for c in 0..4 {
                    acc[c] += tmp[i + c] as u32;
                }
                n += 1;
            }
            let o = (y * w + x) * 4;
            for c in 0..4 {
                out[o + c] = (acc[c] / n) as u8;
            }
        }
    }
    out
}
