//! Text far off the surface (a chart label at a runaway coordinate, a NaN from a
//! degenerate scale) is skipped, never an integer overflow.

#[allow(unused_imports)]
use uzor::render::*;
use uzor_render_tiny_skia::TinySkiaCpuRenderContext;

#[test]
fn text_at_a_far_off_or_non_finite_pen_is_skipped() {
    let mut ctx = TinySkiaCpuRenderContext::new(64, 32, 1.0);
    ctx.set_fill_color("#ffffff");
    ctx.set_font("12px sans-serif");
    for (x, y) in [(1.0e12, 10.0), (-1.0e12, 10.0), (10.0, 1.0e12), (f64::NAN, 10.0), (10.0, f64::INFINITY)] {
        ctx.fill_text("Price 64250", x, y);
    }
    assert!(ctx.pixmap().data().chunks_exact(4).all(|p| p[3] == 0), "nothing lands on the surface");
    // Text in range still draws.
    ctx.fill_text("Price", 2.0, 20.0);
    assert!(ctx.pixmap().data().chunks_exact(4).any(|p| p[3] > 0));
}
