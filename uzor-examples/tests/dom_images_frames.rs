//! Reference renders of raster images in the DOM on the CPU (tiny-skia): `object-fit`
//! places the picture, `border-radius` clips it, an image the backend lacks leaves the
//! box background. Probes pixels at places computed from the laid-out tree. Set
//! `UZOR_SHOTS_DIR` to also write the render as PNG (never committed).
#![cfg(feature = "png")]

use uzor::Rect;
use uzor_examples::dom_images::ImagesDemo;
use uzor_render_tiny_skia::TinySkiaCpuRenderContext;

const W: f64 = 1040.0;
const H: f64 = 260.0;

fn px(ctx: &TinySkiaCpuRenderContext, x: f64, y: f64) -> [u8; 3] {
    let i = ((y.floor() as usize) * ctx.width() as usize + x.floor() as usize) * 4;
    let p = ctx.pixels();
    [p[i], p[i + 1], p[i + 2]]
}

#[test]
fn object_fit_radius_and_the_pending_placeholder() {
    let g = ImagesDemo::default();
    let mut ctx = TinySkiaCpuRenderContext::new(W as u32, H as u32, 1.0);
    let d = g.render(&mut ctx, W, H);
    if let Ok(dir) = std::env::var("UZOR_SHOTS_DIR") {
        std::fs::create_dir_all(&dir).expect("shots dir");
        ctx.pixmap().save_png(std::path::Path::new(&dir).join("dom-images-dark-ru.png")).expect("png");
    }
    let panel = [0x1f, 0x21, 0x25];
    let hover = [0x2c, 0x2f, 0x36];
    let r = |id: &str| -> Rect { d.rect(id).unwrap() };
    // Avatar: round. The centre is the light disc, a corner of its box is the card.
    let a = r("avatar");
    assert_eq!(px(&ctx, a.x + 32.0, a.y + 32.0), [210, 210, 210], "avatar centre");
    assert_eq!(px(&ctx, a.x + 2.0, a.y + 2.0), panel, "outside the circle");
    // Cover: the picture reaches every edge (cropped top and bottom), corners rounded.
    let c = r("cover");
    let left = px(&ctx, c.x + 10.0, c.y + 60.0)[0];
    let right = px(&ctx, c.x + c.width - 10.0, c.y + 60.0)[0];
    assert!(left < right && left >= 60, "ramp left {left} right {right}");
    assert_ne!(px(&ctx, c.x + 10.0, c.y + 2.0), hover, "cover fills the top edge");
    assert_eq!(px(&ctx, c.x + 0.5, c.y + 0.5), panel, "rounded corner");
    // Contain: letterboxed left and right (box background), the picture in the middle.
    let k = r("contain");
    assert_eq!(px(&ctx, k.x + 3.0, k.y + 60.0), hover, "letterbox");
    assert_ne!(px(&ctx, k.x + k.width / 2.0, k.y + 60.0), hover, "picture in the middle");
    // Not loaded: the box background.
    let p = r("pending");
    assert_eq!(px(&ctx, p.x + p.width / 2.0, p.y + p.height / 2.0), hover, "placeholder");
}
