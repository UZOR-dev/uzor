//! Reference renders of the paint-slot demo on the CPU (tiny-skia): the registered
//! painters draw into their slots, in the sheet's colours, and their input state
//! (hover, drag, focus) shows. Probes pixels at places computed from the laid-out
//! tree rather than diffing a stored image, so font hinting does not make it flaky.
//! Set `UZOR_SHOTS_DIR` to also write the renders as PNG (never committed).
#![cfg(feature = "png")]

use std::f64::consts::PI;

use uzor::ui::dom::{Dom, SlotInput, SlotRegistry};
use uzor::Rect;
use uzor_examples::dom_slots::SlotsDemo;
use uzor_render_tiny_skia::TinySkiaCpuRenderContext;

const W: f64 = 720.0;
const H: f64 = 290.0;

struct Shot {
    ctx: TinySkiaCpuRenderContext,
    dom: Dom,
}

impl Shot {
    fn px(&self, x: f64, y: f64) -> [u8; 3] {
        let (x, y) = (x.floor() as usize, y.floor() as usize);
        let i = (y * self.ctx.width() as usize + x) * 4;
        let p = self.ctx.pixels();
        [p[i], p[i + 1], p[i + 2]]
    }
    fn lum(&self, x: f64, y: f64) -> u8 {
        let [r, g, b] = self.px(x, y);
        ((r as u32 * 3 + g as u32 * 6 + b as u32) / 10) as u8
    }
    fn slot(&self, key: &str) -> Rect {
        self.dom.slots().find(|(k, _)| *k == key).map(|(_, r)| r).expect("slot")
    }
}

fn render(g: &SlotsDemo, name: &str, script: impl Fn(&mut SlotRegistry, &Dom)) -> Shot {
    let mut reg = g.registry();
    let mut ctx = TinySkiaCpuRenderContext::new(W as u32, H as u32, 1.0);
    let d = g.render(&mut ctx, &mut reg, W, H);
    script(&mut reg, &d);
    let mut ctx = TinySkiaCpuRenderContext::new(W as u32, H as u32, 1.0);
    let dom = g.render(&mut ctx, &mut reg, W, H);
    if let Ok(dir) = std::env::var("UZOR_SHOTS_DIR") {
        std::fs::create_dir_all(&dir).expect("shots dir");
        ctx.pixmap().save_png(std::path::Path::new(&dir).join(format!("{name}.png"))).expect("png");
    }
    Shot { ctx, dom }
}

/// Where the dial's arc passes at fraction `t` of its sweep, and the radius.
fn dial_at(r: Rect, t: f64, extra: f64) -> (f64, f64) {
    let (cx, cy) = (r.x + r.width / 2.0, r.y + r.height / 2.0);
    let rad = r.width.min(r.height) / 2.0 - 8.0 + extra;
    let a = 0.75 * PI + 1.5 * PI * t;
    (cx + rad * a.cos(), cy + rad * a.sin())
}

/// The sparkline's crosshair column for day `i` of 30, and the plot's bottom.
fn spark_col(r: Rect, i: usize) -> (f64, f64) {
    let (x0, pw) = (r.x + 4.0, r.width - 8.0);
    ((x0 + pw * i as f64 / 29.0).round() + 0.5, r.y + r.height - 6.0)
}

fn hover_drag_focus(reg: &mut SlotRegistry, d: &Dom) {
    let s = d.slots().find(|(k, _)| *k == "spark").unwrap().1;
    // Day 12 of 30.
    let x = s.x + 4.0 + (s.width - 8.0) * 12.0 / 29.0;
    reg.input("spark", s, &SlotInput { pointer: Some((x, s.y + 50.0)), ..Default::default() });
    let k = d.slots().find(|(k, _)| *k == "dial").unwrap().1;
    let (x, y) = (k.x + k.width / 2.0, k.y + k.height / 2.0);
    reg.input("dial", k, &SlotInput { pointer: Some((x, y)), pressed: Some((x, y)), focused: true, ..Default::default() });
    reg.input("dial", k, &SlotInput { pointer: Some((x, y - 37.5)), pressed: Some((x, y - 37.5)), focused: true, ..Default::default() });
}

#[test]
fn painters_draw_their_slots_in_the_sheet_colours() {
    let rest = render(&SlotsDemo::default(), "dom-slots-rest-dark-ru", |_, _| {});
    let k = rest.slot("dial");
    assert_eq!((k.width, k.height), (170.0, 120.0), "dial content box");
    // The value arc (40 %) is the text colour, the rest of the track the line colour.
    let (x, y) = dial_at(k, 0.2, 0.0);
    assert!(rest.lum(x, y) > 200, "value arc {:?}", rest.px(x, y));
    let (x, y) = dial_at(k, 0.8, 0.0);
    assert_eq!(rest.px(x, y), [0x33, 0x36, 0x3c], "track");
    // Inside the slot, away from the drawing: the card's panel shows through.
    assert_eq!(rest.px(k.x + 2.0, k.y + 2.0), [0x1f, 0x21, 0x25], "panel under the slot");
    // No focus ring at rest.
    let (x, y) = dial_at(k, 0.5, 7.0);
    assert_eq!(rest.px(x, y), [0x1f, 0x21, 0x25], "no focus ring");
    // The sparkline: no crosshair at rest.
    let s = rest.slot("spark");
    let (cx, cy) = spark_col(s, 12);
    assert!(rest.lum(cx, cy) < 70, "no crosshair {:?}", rest.px(cx, cy));
}

#[test]
fn hover_drag_and_focus_show_in_the_render() {
    let shot = render(&SlotsDemo::default(), "dom-slots-hover-drag-focus-dark-ru", hover_drag_focus);
    let s = shot.slot("spark");
    let (cx, cy) = spark_col(s, 12);
    assert!(shot.lum(cx, cy) > 110, "crosshair {:?}", shot.px(cx, cy));
    let k = shot.slot("dial");
    // Dragged up 37.5 px: 40 % + 25 % = 65 %, so 60 % of the sweep is lit now.
    let (x, y) = dial_at(k, 0.6, 0.0);
    assert!(shot.lum(x, y) > 200, "value arc after the drag {:?}", shot.px(x, y));
    let (x, y) = dial_at(k, 0.75, 0.0);
    assert_eq!(shot.px(x, y), [0x33, 0x36, 0x3c], "track past the value");
    // Focused: the ring above the dial.
    let (x, y) = dial_at(k, 0.5, 7.0);
    assert!(shot.lum(x, y) > 150, "focus ring {:?}", shot.px(x, y));
}

#[test]
fn the_light_theme_comes_from_the_sheet() {
    let shot = render(&SlotsDemo { ru: false, dark: false }, "dom-slots-rest-light-en", |_, _| {});
    let k = shot.slot("dial");
    assert_eq!(shot.px(k.x + 2.0, k.y + 2.0), [0xff, 0xff, 0xff], "light panel");
    let (x, y) = dial_at(k, 0.2, 0.0);
    assert!(shot.lum(x, y) < 70, "dark value arc on light {:?}", shot.px(x, y));
    let (x, y) = dial_at(k, 0.8, 0.0);
    assert_eq!(shot.px(x, y), [0xdf, 0xe1, 0xe6], "light track");
}
