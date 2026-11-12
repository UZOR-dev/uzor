//! Render the paint-slot demo to PNG shots: `dom-slots-frames <out-dir> [scale]`.
//! Writes the page at rest (dark, Russian), with the sparkline hovered and the dial
//! dragged and focused, and the light theme in English.

use uzor::render::Painter;
use uzor::ui::dom::SlotInput;
use uzor_examples::dom_slots::SlotsDemo;
use uzor_render_tiny_skia::TinySkiaCpuRenderContext;

const W: f64 = 720.0;
const H: f64 = 290.0;

fn shot(out: &std::path::Path, name: &str, scale: f64, g: &SlotsDemo, script: impl Fn(&mut uzor::ui::dom::SlotRegistry, &uzor::ui::dom::Dom)) {
    let mut reg = g.registry();
    // Lay out once to learn where the slots are, feed the input, then paint.
    let mut probe = TinySkiaCpuRenderContext::new(1, 1, 1.0);
    let d = g.render(&mut probe, &mut reg, W, H);
    script(&mut reg, &d);
    let mut ctx = TinySkiaCpuRenderContext::new((W * scale) as u32, (H * scale) as u32, scale);
    ctx.scale(scale, scale);
    g.render(&mut ctx, &mut reg, W, H);
    let p = out.join(format!("{name}.png"));
    ctx.pixmap().save_png(&p).expect("png");
    eprintln!("{}", p.display());
}

fn slot(d: &uzor::ui::dom::Dom, key: &str) -> uzor::Rect {
    d.slots().find(|(k, _)| *k == key).map(|(_, r)| r).expect("slot")
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = std::path::PathBuf::from(args.next().unwrap_or_else(|| "dom-slots".into()));
    let scale: f64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(2.0);
    std::fs::create_dir_all(&out).expect("out dir");
    let g = SlotsDemo::default();
    shot(&out, "01-rest-dark-ru", scale, &g, |_, _| {});
    shot(&out, "02-hover-drag-focus-dark-ru", scale, &g, |reg, d| {
        let s = slot(d, "spark");
        reg.input("spark", s, &SlotInput { pointer: Some((s.x + s.width * 0.4, s.y + 50.0)), ..Default::default() });
        let k = slot(d, "dial");
        let (x, y) = (k.x + k.width / 2.0, k.y + k.height / 2.0);
        reg.input("dial", k, &SlotInput { pointer: Some((x, y)), pressed: Some((x, y)), focused: true, ..Default::default() });
        reg.input("dial", k, &SlotInput { pointer: Some((x, y - 37.5)), pressed: Some((x, y - 37.5)), focused: true, ..Default::default() });
    });
    let light = SlotsDemo { ru: false, dark: false };
    shot(&out, "03-rest-light-en", scale, &light, |_, _| {});
}
