//! `box-shadow` in the DOM painter honours its x offset (it used to keep only dy).

use uzor::ui::dom::{render, El};
use uzor::ui::style::{Sheet, State};
use uzor_render_tiny_skia::TinySkiaCpuRenderContext;

fn shot(css: &str) -> TinySkiaCpuRenderContext {
    let sheet = Sheet::parse("div.root { position: relative; width: 100px; height: 60px; background: #ffffff } div.b { position: absolute; left: 30px; top: 20px; width: 40px; height: 20px; background: #0000ff }").unwrap();
    let mut ctx = TinySkiaCpuRenderContext::new(100, 60, 1.0);
    let tree = El::new("div.root").child(El::new("div.b").css(css));
    render(&mut ctx, &sheet, &tree, 0.0, 0.0, 100.0, &|_| State::NONE);
    ctx
}

fn px(ctx: &TinySkiaCpuRenderContext, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * 100 + x) * 4) as usize;
    let p = ctx.pixels();
    [p[i], p[i + 1], p[i + 2]]
}

const WHITE: [u8; 3] = [255, 255, 255];

#[test]
fn a_hard_shadow_falls_right_and_down_by_its_offsets() {
    let ctx = shot("box-shadow: 6px 4px 0 #000000");
    // The box spans x 30..70, y 20..40; the shadow 36..76, 24..44.
    assert_eq!(px(&ctx, 50, 30), [0, 0, 255], "the box itself");
    assert_eq!(px(&ctx, 73, 30), [0, 0, 0], "right of the box: shadow");
    assert_eq!(px(&ctx, 50, 42), [0, 0, 0], "below the box: shadow");
    assert_eq!(px(&ctx, 33, 42), WHITE, "below-left, before the x offset: clear");
    assert_eq!(px(&ctx, 27, 30), WHITE, "left of the box: clear");
}

#[test]
fn a_negative_x_offset_falls_left() {
    let ctx = shot("box-shadow: -6px 0 0 #000000");
    assert_eq!(px(&ctx, 26, 30), [0, 0, 0], "left of the box: shadow");
    assert_eq!(px(&ctx, 73, 30), WHITE, "right of the box: clear");
}

#[test]
fn without_an_x_offset_nothing_changes() {
    let ctx = shot("box-shadow: 0 4px 0 #000000");
    assert_eq!(px(&ctx, 50, 42), [0, 0, 0]);
    assert_eq!(px(&ctx, 73, 30), WHITE);
    assert_eq!(px(&ctx, 27, 30), WHITE);
}
