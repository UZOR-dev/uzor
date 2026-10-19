//! Clips intersect (canvas semantics) and glyph blits honour them.
#[allow(unused_imports)]
use uzor::render::*;
use uzor_render_tiny_skia::TinySkiaCpuRenderContext;

fn ink(ctx: &TinySkiaCpuRenderContext) -> usize {
    ctx.pixmap().data().chunks(4).filter(|p| p[3] != 0).count()
}

#[test]
fn text_outside_the_clip_is_not_drawn() {
    let mut ctx = TinySkiaCpuRenderContext::new(200, 100, 1.0);
    ctx.save();
    ctx.begin_path();
    ctx.rect(0.0, 0.0, 200.0, 10.0);
    ctx.clip();
    ctx.set_font("16px sans-serif");
    ctx.set_fill_color("#ffffff");
    ctx.fill_text("Hello", 10.0, 80.0);
    ctx.restore();
    assert_eq!(ink(&ctx), 0);
    ctx.fill_text("Hello", 10.0, 80.0);
    assert!(ink(&ctx) > 0);
}

#[test]
fn nested_clips_intersect_and_restore_pops() {
    let mut ctx = TinySkiaCpuRenderContext::new(100, 100, 1.0);
    ctx.save();
    ctx.begin_path();
    ctx.rect(0.0, 0.0, 50.0, 100.0);
    ctx.clip();
    ctx.save();
    ctx.begin_path();
    ctx.rect(0.0, 0.0, 100.0, 50.0);
    ctx.clip();
    ctx.set_fill_color("#ffffff");
    ctx.fill_rect(0.0, 0.0, 100.0, 100.0);
    assert_eq!(ink(&ctx), 50 * 50);
    ctx.restore();
    ctx.fill_rect(0.0, 0.0, 100.0, 100.0);
    assert_eq!(ink(&ctx), 50 * 100);
    ctx.restore();
}
