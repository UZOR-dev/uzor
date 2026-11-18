//! Per-primitive URX-CPU vs tiny-skia raster comparison for the us
//! (user-settings) kit/legacy primitive vocabulary: rounded-rect fills
//! (checkbox box, hover rows), rounded-rect strokes (kit radio ring /
//! border), arc strokes + fills (legacy radio ring / dot), and labels.
//!
//! The us parity gap on URX (3.5k–13.5k px over8 vs legacy) is the sum
//! of these per-primitive divergences: the legacy pass and the DOM kit
//! pass issue DIFFERENT command streams for the same visual, tiny-skia
//! rasterises them near-identically, URX must too. Each case paints the
//! SAME trait call sequence through both adapters and reports the diff;
//! the bounding boxes localise the AA disagreement.

use uzor::render::{Painter, ShapeHelpers, TextBaseline};

const W: u32 = 96;
const H: u32 = 96;

fn urx_paint(f: impl Fn(&mut uzor_render_urx::UrxRenderContext)) -> Vec<u8> {
    let mut ctx = uzor_render_urx::UrxRenderContext::new(1.0);
    ctx.begin_frame(W, H);
    ctx.set_fill_color("#131722");
    ctx.fill_rect(0.0, 0.0, W as f64, H as f64);
    f(&mut ctx);
    let scene = ctx.take_scene();
    let mut pm = uzor_urx_cpu::pixmap::Pixmap::new(W, H);
    uzor_urx_cpu::backend::CpuBackend::new().render(&scene, &mut pm).expect("urx render");
    pm.pixels().to_vec()
}

fn skia_paint(f: impl Fn(&mut uzor_render_tiny_skia::TinySkiaCpuRenderContext)) -> Vec<u8> {
    let mut ctx = uzor_render_tiny_skia::TinySkiaCpuRenderContext::new(W, H, 1.0);
    ctx.set_fill_color("#131722");
    ctx.fill_rect(0.0, 0.0, W as f64, H as f64);
    f(&mut ctx);
    ctx.pixmap().data().to_vec()
}

fn diff(name: &str, a: &[u8], b: &[u8]) -> (usize, usize) {
    let mut over = 0;
    let mut exact = 0;
    let (mut x0, mut y0, mut x1, mut y1) = (W, H, 0u32, 0u32);
    for (i, (p, q)) in a.chunks_exact(4).zip(b.chunks_exact(4)).enumerate() {
        let d = p.iter().zip(q.iter()).map(|(u, v)| u.abs_diff(*v)).max().unwrap_or(0);
        if d > 0 {
            exact += 1;
            let (x, y) = ((i % W as usize) as u32, (i / W as usize) as u32);
            x0 = x0.min(x); y0 = y0.min(y); x1 = x1.max(x); y1 = y1.max(y);
        }
        if d > 8 { over += 1; }
    }
    eprintln!("{name}: {over} over8, {exact} differ, bbox x{x0}..={x1} y{y0}..={y1}");
    (over, exact)
}

/// Kit checkbox box: 16×16 rounded rect, 2px corners (persist_checkbox_row).
#[test]
fn rounded_fill_checkbox_box() {
    let f = |c: &mut dyn uzor::render::RenderContext| {
        c.set_fill_color("#2a2e39");
        c.rounded_rect(20.0, 20.0, 16.0, 16.0, 2.0);
        c.fill();
        c.set_fill_color("#2962ff");
        c.rounded_rect(23.0, 23.0, 10.0, 10.0, 1.0);
        c.fill();
    };
    // The trait-object form keeps ONE call sequence for both adapters.
    let a = urx_paint(|c| f(c));
    let b = skia_paint(|c| f(c));
    let (over, _) = diff("rounded_fill_checkbox_box", &a, &b);
    assert!(over <= 40, "checkbox-box rounded fill diverges: {over} over8");
}

/// Kit radio ring as a rounded-rect STROKE (border 1.5px, radius 7.75).
#[test]
fn rounded_stroke_radio_ring() {
    let f = |c: &mut dyn uzor::render::RenderContext| {
        c.set_stroke_color("#787b86");
        c.set_stroke_width(1.5);
        c.rounded_rect(20.0, 20.0, 15.5, 15.5, 7.75);
        c.stroke();
    };
    let a = urx_paint(|c| f(c));
    let b = skia_paint(|c| f(c));
    let (over, _) = diff("rounded_stroke_radio_ring", &a, &b);
    assert!(over <= 40, "kit ring rounded stroke diverges: {over} over8");
}

/// Legacy radio ring: arc path + stroke (draw_radio_group).
#[test]
fn arc_stroke_legacy_ring() {
    let f = |c: &mut dyn uzor::render::RenderContext| {
        c.set_stroke_color("#787b86");
        c.set_stroke_width(1.5);
        c.begin_path();
        c.arc(27.75, 27.75, 7.0, 0.0, std::f64::consts::TAU);
        c.stroke();
    };
    let a = urx_paint(|c| f(c));
    let b = skia_paint(|c| f(c));
    let (over, _) = diff("arc_stroke_legacy_ring", &a, &b);
    assert!(over <= 40, "legacy ring arc stroke diverges: {over} over8");
}

/// Legacy radio dot: arc path + fill (8px dot).
#[test]
fn arc_fill_legacy_dot() {
    let f = |c: &mut dyn uzor::render::RenderContext| {
        c.set_fill_color("#2962ff");
        c.begin_path();
        c.arc(27.75, 27.75, 4.0, 0.0, std::f64::consts::TAU);
        c.fill();
    };
    let a = urx_paint(|c| f(c));
    let b = skia_paint(|c| f(c));
    let (over, _) = diff("arc_fill_legacy_dot", &a, &b);
    assert!(over <= 40, "legacy dot arc fill diverges: {over} over8");
}

/// Hover row fill: wide rounded rect, 6px corners (us-row:hover).
#[test]
fn rounded_fill_hover_row() {
    let f = |c: &mut dyn uzor::render::RenderContext| {
        c.set_fill_color("#1e222d");
        c.rounded_rect(8.0, 20.0, 80.0, 44.0, 6.0);
        c.fill();
    };
    let a = urx_paint(|c| f(c));
    let b = skia_paint(|c| f(c));
    let (over, _) = diff("rounded_fill_hover_row", &a, &b);
    assert!(over <= 60, "hover-row rounded fill diverges: {over} over8");
}

/// The DOM painter's cubic `path_round_rect` (uzor dialog mod) vs the
/// legacy `fill_rounded_rect` / `stroke_rounded_rect` ShapeHelpers calls,
/// at the SAME geometry. URX must raster the two conventions identically
/// — they now share one draw path (FillRect/StrokeRect + radii); before
/// the unification the cubic path filled scanline while the legacy call
/// used the rounded-clip rect fill, and the AA differed (the strip
/// tab-close background and colmenu row residuals).
fn cubic_round_rect(c: &mut dyn uzor::render::RenderContext, x: f64, y: f64, w: f64, h: f64, rad: f64) {
    let k = rad * (1.0 - 0.5523);
    let (x0, y0, x1, y1) = (x, y, x + w, y + h);
    c.begin_path();
    c.move_to(x0 + rad, y0);
    c.line_to(x1 - rad, y0);
    c.bezier_curve_to(x1 - k, y0, x1, y0 + k, x1, y0 + rad);
    c.line_to(x1, y1 - rad);
    c.bezier_curve_to(x1, y1 - k, x1 - k, y1, x1 - rad, y1);
    c.line_to(x0 + rad, y1);
    c.bezier_curve_to(x0 + k, y1, x0, y1 - k, x0, y1 - rad);
    c.line_to(x0, y0 + rad);
    c.bezier_curve_to(x0, y0 + k, x0 + k, y0, x0 + rad, y0);
    c.close_path();
}

#[test]
fn dom_cubic_round_rect_matches_legacy_radii_commands_on_urx() {
    // Fill pair (tab-close background / colmenu rows / hover fills).
    let dom = |c: &mut dyn uzor::render::RenderContext| {
        c.set_fill_color("#2b2f3a");
        cubic_round_rect(c, 20.0, 20.0, 17.0, 17.0, 6.0);
        c.fill();
    };
    let legacy = |c: &mut dyn uzor::render::RenderContext| {
        c.set_fill_color("#2b2f3a");
        c.fill_rounded_rect(20.0, 20.0, 17.0, 17.0, 6.0);
    };
    let a = urx_paint(|c| dom(c));
    let b = urx_paint(|c| legacy(c));
    let (over, exact) = diff("dom_cubic_vs_legacy_fill_URX", &a, &b);
    assert!(over <= 2, "rounded fills must share one draw path on URX: {over} over8, {exact} differ");

    // Stroke pair (kit border boxes vs legacy stroke_rounded_rect).
    let dom = |c: &mut dyn uzor::render::RenderContext| {
        c.set_stroke_color("#787b86");
        c.set_stroke_width(1.5);
        cubic_round_rect(c, 20.0, 20.0, 40.0, 24.0, 4.0);
        c.stroke();
    };
    let legacy = |c: &mut dyn uzor::render::RenderContext| {
        c.set_stroke_color("#787b86");
        c.set_stroke_width(1.5);
        c.stroke_rounded_rect(20.0, 20.0, 40.0, 24.0, 4.0);
    };
    let a = urx_paint(|c| dom(c));
    let b = urx_paint(|c| legacy(c));
    let (over, exact) = diff("dom_cubic_vs_legacy_stroke_URX", &a, &b);
    assert!(over <= 2, "rounded strokes must share one draw path on URX: {over} over8, {exact} differ");
}

/// Ink-weighted vertical centroid of the frame (background #131722).
fn ink_centroid_y(px: &[u8]) -> f64 {
    let (mut sum, mut n) = (0f64, 0f64);
    for (i, c) in px.chunks_exact(4).enumerate() {
        let d = (c[0].abs_diff(19) as u32 + c[1].abs_diff(23) as u32 + c[2].abs_diff(34) as u32) as f64;
        if d > 24.0 {
            let y = (i / W as usize) as f64;
            sum += y * d;
            n += d;
        }
    }
    if n > 0.0 { sum / n } else { -1.0 }
}

/// THE baseline parity gate: legacy-convention `Middle`/`Top` text must
/// land on the same device ROW under URX and tiny-skia. The mlc compares
/// pair legacy `Middle` (row centre) against DOM `Alphabetic` (css_text
/// baseline); tiny-skia's fontdue ascent made those agree, so URX must
/// use the same ascent. A shifted baseline moves the whole ink band
/// (~1.5 px at 13 px with the old heuristics); hinting noise alone does
/// not. Gate on the ink centroid, not per-pixel counts (the swash-vs-
/// fontdue raster delta is informational, see below).
#[test]
fn middle_and_top_baselines_match_tiny_skia_rows() {
    for baseline in [TextBaseline::Middle, TextBaseline::Top] {
        let f = move |c: &mut dyn uzor::render::RenderContext| {
            c.set_fill_color("#d1d4dc");
            c.set_font("13px Roboto");
            c.set_text_baseline(baseline);
            c.fill_text("English", 20.0, 40.0);
        };
        let a = urx_paint(|c| f(c));
        let b = skia_paint(|c| f(c));
        let (ca, cb) = (ink_centroid_y(&a), ink_centroid_y(&b));
        eprintln!("{baseline:?} ink centroid: urx {ca:.3} skia {cb:.3}");
        assert!(ca > 0.0 && cb > 0.0, "both sides rendered ink");
        assert!((ca - cb).abs() < 0.35, "{baseline:?} baseline row shifted: urx {ca:.3} vs skia {cb:.3}");
    }
}

/// `line_ascent` parses the real hhea metric (scales linearly with px,
/// sane Roboto band) — the value URX's Top/Middle now share with fontdue.
#[test]
fn line_ascent_is_a_real_scaled_hhea_metric() {
    let a13 = uzor::shaper::line_ascent("13px Roboto");
    let a26 = uzor::shaper::line_ascent("26px Roboto");
    assert!(a13 > 11.0 && a13 < 13.0, "Roboto 13px ascent in the ~0.9em band: {a13}");
    assert!((a26 - 2.0 * a13).abs() < 0.01, "ascent scales with px: {a13} -> {a26}");
    let bold = uzor::shaper::line_ascent("bold 13px Roboto");
    assert!(bold > 11.0 && bold < 13.0, "bold ascent sane: {bold}");
}

/// Label text: 13px Roboto, alphabetic baseline (us radio label).
/// INFORMATIONAL, not a gate: URX rasterises glyphs with hinted swash,
/// tiny-skia with fontdue — the same string at the same pen lands ~200 px
/// over8 apart across backends (stem hinting + coverage gamma). That delta
/// CANCELS in the port's parity compares (old and DOM passes render through
/// the SAME backend); it only matters for cross-backend reference shots.
/// The us parity gap on URX is therefore the text BASELINE/face convention
/// (css_text vs legacy Middle) plus the kit/legacy shape streams above,
/// not the glyph rasteriser itself.
#[test]
fn label_text_cross_backend_delta_is_documented() {
    let f = |c: &mut dyn uzor::render::RenderContext| {
        c.set_fill_color("#d1d4dc");
        c.set_font("13px Roboto");
        c.set_text_baseline(TextBaseline::Alphabetic);
        c.fill_text("English", 20.0, 40.0);
    };
    let a = urx_paint(|c| f(c));
    let b = skia_paint(|c| f(c));
    let (over, exact) = diff("label_text (cross-backend, informational)", &a, &b);
    // Sanity only: the word rendered on both sides, and the delta stays in
    // the documented hinting band (a catastrophic font or baseline
    // regression would blow far past it).
    assert!(over > 0 && over < 1000, "text raster delta out of the documented band: {over} over8, {exact} differ");
}
