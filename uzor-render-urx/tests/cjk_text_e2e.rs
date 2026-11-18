//! End-to-end CJK text on the URX glyph-id path: cosmic-text shaping
//! (with the uzor-fonts `cjk` face registered) → `UrxRenderContext`
//! `GlyphRun` emission → `uzor-urx-cpu` raster. The us General tab's
//! language names (中文 / 日本語 / 한국어, plus العربية / हिन्दी) must
//! render with their own faces — never tofu, never a panic.
//!
//! Regression for the CJK panic that killed the us text pass when the
//! URX path first met a real CJK run: the shaper now loads the opt-in
//! CJK face (and the bundled Arabic/Devanagari faces), and
//! `uzor-urx-glyph` degrades a bad glyph to a skipped instance instead
//! of aborting the run.

use uzor::render::{Painter, TextBaseline, TextRenderer};
use uzor_render_urx::UrxRenderContext;
use uzor_urx_core::scene::DrawCommand;

#[test]
fn cjk_shapes_with_the_cjk_face_not_tofu() {
    let segs = uzor::shaper::shape_glyph_runs("中文日本語한국어", "13px Roboto");
    assert!(!segs.is_empty(), "CJK text shapes");
    let cjk = segs
        .iter()
        .filter_map(|s| uzor::shaper::font_family_for(s.font))
        .find(|f| f.contains("CJK"));
    assert!(cjk.is_some(), "a CJK face is registered with the shaper, got {:?}",
        segs.iter().filter_map(|s| uzor::shaper::font_family_for(s.font)).collect::<Vec<_>>());
    // Every shaped glyph id is a real glyph of that face, not .notdef.
    for seg in &segs {
        for g in &seg.glyphs {
            assert_ne!(g.glyph_id, 0, "no .notdef in a CJK run");
        }
    }
}

#[test]
fn language_names_render_through_glyph_runs_on_cpu() {
    let mut ctx = UrxRenderContext::new(1.0);
    ctx.begin_frame(420, 140);
    ctx.set_fill_color("#e6e6eb");
    ctx.set_font("13px Roboto");
    ctx.set_text_baseline(TextBaseline::Alphabetic);
    ctx.fill_text("中文 日本語 한국어", 10.0, 30.0);
    ctx.fill_text("العربية हिन्दी Русский", 10.0, 70.0);
    ctx.fill_text("English Español Türkçe", 10.0, 110.0);
    let scene = ctx.take_scene();

    let runs: Vec<&DrawCommand> = scene
        .commands
        .iter()
        .filter(|c| matches!(c, DrawCommand::GlyphRun { .. }))
        .collect();
    assert!(runs.len() >= 3, "the language names emit GlyphRuns, got {}", runs.len());
    // The CJK run carries its own text companion field.
    let has_cjk_run = runs.iter().any(|c| match c {
        DrawCommand::GlyphRun { text: Some(t), .. } => t.contains('中'),
        _ => false,
    });
    assert!(has_cjk_run, "a GlyphRun carries the CJK text");

    let mut pm = uzor_urx_cpu::pixmap::Pixmap::new(420, 140);
    uzor_urx_cpu::backend::CpuBackend::new()
        .render(&scene, &mut pm)
        .expect("urx cpu render of the language names");
    let ink = pm.pixels().chunks_exact(4).filter(|c| c[3] > 0).count();
    assert!(ink > 500, "the language names leave ink, got {ink} px");
}
