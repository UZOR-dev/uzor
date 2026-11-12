//! `background-clip: padding-box` / `content-box` in the DOM painter: a translucent
//! border shows what is behind the box, not the box's own background.

use uzor::ui::dom::{render, El};
use uzor::ui::style::{Sheet, State};
use uzor_render_tiny_skia::TinySkiaCpuRenderContext;

fn shot(css: &str) -> TinySkiaCpuRenderContext {
    let sheet = Sheet::parse(
        "div.root { position: relative; width: 100px; height: 60px; background: #ffffff }
         div.b { position: absolute; left: 20px; top: 10px; width: 60px; height: 40px; padding: 6px; background: #0000ff;
                 border: 4px solid rgba(255,0,0,0.5) }",
    )
    .unwrap();
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

fn near(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 2)
}

// The box: x 20..80, y 10..50; border 4 px; padding 6 px.
const BORDER: (u32, u32) = (22, 30);
const PADDING: (u32, u32) = (27, 30);
const CONTENT: (u32, u32) = (50, 30);
const PINK: [u8; 3] = [255, 127, 127]; // half red over white
const PURPLE: [u8; 3] = [127, 0, 127]; // half red over blue
const BLUE: [u8; 3] = [0, 0, 255];
const WHITE: [u8; 3] = [255, 255, 255];

#[test]
fn by_default_the_background_runs_under_the_border() {
    let c = shot("");
    assert!(near(px(&c, BORDER.0, BORDER.1), PURPLE), "{:?}", px(&c, BORDER.0, BORDER.1));
    assert_eq!(px(&c, PADDING.0, PADDING.1), BLUE);
}

#[test]
fn padding_box_keeps_the_background_off_the_border() {
    let c = shot("background-clip: padding-box");
    assert!(near(px(&c, BORDER.0, BORDER.1), PINK), "{:?}", px(&c, BORDER.0, BORDER.1));
    assert_eq!(px(&c, PADDING.0, PADDING.1), BLUE);
    assert_eq!(px(&c, CONTENT.0, CONTENT.1), BLUE);
}

#[test]
fn content_box_keeps_it_off_the_padding_too() {
    let c = shot("background-clip: content-box");
    assert!(near(px(&c, BORDER.0, BORDER.1), PINK));
    assert_eq!(px(&c, PADDING.0, PADDING.1), WHITE);
    assert_eq!(px(&c, CONTENT.0, CONTENT.1), BLUE);
}

#[test]
fn rounded_corners_follow_the_inner_radius() {
    // radius 10, border 4: the padding box (x 24.., y 14..) has corner radius 6
    // around (30, 20). (25, 15) is inside the padding box's square corner but 7 px
    // from the centre: border over the page (pink), not the blue fill.
    let c = shot("background-clip: padding-box; border-radius: 10px");
    let p = px(&c, 25, 15);
    assert!(p[0] > 230 && p[2] < 160, "border over white, no fill: {p:?}");
    assert_eq!(px(&c, 30, 20), BLUE, "inside it");
}
