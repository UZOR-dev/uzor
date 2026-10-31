//! Render the DOM widget gallery to PNG frames: `dom-gallery-frames <out-dir> [fps]`.
//! A scripted walk-through (switch, typing with caret and selection, the select
//! popover, a hovered message, scrolling a virtualized chat, the fixed side panel).
//! Join the frames with e.g. `ffmpeg -framerate 30 -i out/%04d.png gallery.mp4`.

use uzor::render::{Painter, RenderContext, ShapeHelpers};
use uzor::ui::dom::{self, Dom};
use uzor::ui::domkit::VirtualList;
use uzor::ui::style::State;
use uzor::Rect;
use uzor_examples::dom_gallery::{DomGallery, MESSAGES};
use uzor_render_tiny_skia::TinySkiaCpuRenderContext;

const W: f64 = 1280.0;
const H: f64 = 720.0;

fn main() {
    let mut args = std::env::args().skip(1);
    let out = std::path::PathBuf::from(args.next().unwrap_or_else(|| "dom-gallery".into()));
    let fps: f64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(30.0);
    std::fs::create_dir_all(&out).expect("out dir");
    let sheet = DomGallery::sheet();
    let mut list = VirtualList::new(MESSAGES.len() * 50, 64.0);
    list.gap = 6.0;
    let mut g = DomGallery::default();
    let mut anchor: Option<Rect> = None;
    let total = (16.0 * fps) as usize;
    let typed = "lavr_the_flame";
    for f in 0..total {
        let t = f as f64 / fps;
        g.t = t;
        let mut hover: Option<String> = None;
        let mut focus: Option<&str> = None;
        // The script.
        g.compact = t > 1.2;
        let sw = ((t - 1.2) / 0.2).clamp(0.0, 1.0);
        if (1.0..1.5).contains(&t) {
            hover = Some("compact".into());
        }
        g.agree = t > 2.0;
        if (2.2..5.0).contains(&t) {
            focus = Some("nick");
            let n = (((t - 2.2) / 0.12) as usize).min(typed.len());
            g.nick.value = typed[..n].to_string();
            let len = g.nick.value.chars().count();
            g.nick.caret = Some(len);
            g.nick.caret_on = (t * 2.0).fract() < 0.6 || n < typed.len();
            g.nick.anchor = (t > 4.2).then_some(len.saturating_sub(5));
        } else {
            g.nick.caret = None;
            g.nick.anchor = None;
        }
        g.open = (5.2..7.4).contains(&t);
        if (6.0..7.4).contains(&t) {
            hover = Some("level:opt:2".into());
        }
        if t >= 7.4 {
            g.level = 2;
        }
        if (5.0..5.2).contains(&t) {
            hover = Some("level".into());
        }
        g.card_open = !(8.0..9.0).contains(&t);
        g.chip = if t > 8.5 { 1 } else { 0 };
        g.hover_msg = (7.8..9.5).contains(&t).then_some(1);
        if (9.0..9.5).contains(&t) {
            hover = Some("m1:reply".into());
        }
        g.scroll = if t > 9.5 { ((t - 9.5) / 3.0).min(1.0) * 4000.0 } else { 0.0 };
        g.side = t > 12.8;
        if t > 13.6 {
            focus = Some("side:topic");
        }
        let state = |id: &str| {
            let mut s = State::NONE;
            if hover.as_deref() == Some(id) {
                s = s | State::HOVER;
            }
            if focus == Some(id) {
                s = s | State::FOCUS;
            }
            s
        };
        let mut ctx = TinySkiaCpuRenderContext::new(W as u32, H as u32, 1.0);
        ctx.set_fill_color("#17181b");
        ctx.fill_rect(0.0, 0.0, W, H);
        // Lay out; learn row heights; re-lay out once if they changed.
        let mut d: Dom;
        let mut tries = 0;
        loop {
            let mut tree = g.tree(&list, W, H, anchor);
            if g.compact {
                set_switch_pos(&mut tree, "compact", sw);
            }
            d = dom::lay_out_in(&mut ctx, &sheet, &tree, 0.0, 0.0, W, Some(Rect::new(0.0, 0.0, W, H)), &state);
            let win = list.window(g.scroll, 500.0, 200.0);
            tries += 1;
            if !list.measure(&d, win, |i| format!("m{i}")) || tries > 2 {
                break;
            }
        }
        anchor = DomGallery::trigger_rect(&d).or(anchor);
        // Clip the chat box like a scroll viewport.
        // A kit-drawn piece embedded in the DOM through a slot, painted in tree order.
        dom::paint_slots(&mut ctx, &d, &mut |ctx, key, r| {
            if key == "@kit-chart" {
                paint_chart(ctx, r, t);
            }
        });
        ctx.pixmap().save_png(out.join(format!("{f:04}.png"))).expect("png");
    }
    eprintln!("{total} frames in {}", out.display());
}

fn set_switch_pos(e: &mut uzor::ui::dom::El, id: &str, pos: f64) {
    if e.id.as_deref() == Some(id) {
        if let Some(uzor::ui::dom::Control::Switch { pos: p, .. }) = &mut e.control {
            *p = pos;
        }
    }
    for c in &mut e.children {
        set_switch_pos(c, id, pos);
    }
}

/// Stand-in for a kit widget drawn straight on the render context inside a slot.
fn paint_chart(ctx: &mut dyn RenderContext, r: Rect, t: f64) {
    let n = 24;
    let bw = r.width / n as f64;
    ctx.set_fill_color("#3b82f6");
    for i in 0..n {
        let v = 0.5 + 0.45 * ((i as f64 * 0.5 + t * 2.0).sin());
        let h = (r.height - 8.0) * v;
        ctx.fill_rect(r.x + i as f64 * bw + 2.0, r.y + r.height - 4.0 - h, bw - 4.0, h);
    }
}
