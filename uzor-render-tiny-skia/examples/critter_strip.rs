//! Critter motion strip: renders critter frames through the real
//! [`uzor::ui::widgets::atomic::critter::draw_critter_motion`] path.
//!
//! ```text
//! cargo run --release -p uzor-render-tiny-skia --example critter_strip -- <out-dir> [frames] [seconds]
//! ```
//!
//! Writes `working.png`, `attention.png` and `thinking.png` (one row per seed, `frames`
//! frames evenly over `seconds`, defaults 16 over 3 s) plus one PNG per working frame.

use std::path::PathBuf;

use uzor::render::{Painter, ShapeHelpers};
use uzor::types::Rect;
use uzor::ui::widgets::atomic::critter::{draw_critter_motion, CritterMotion, CritterState, CritterView};
use uzor_render_tiny_skia::TinySkiaCpuRenderContext;

const CELL: f64 = 72.0;

fn view(phase: f64) -> CritterView {
    CritterView { shape: 12, color: "#8b5cf6", eye_y: 30.0, off: false, phase }
}

fn row(ctx: &mut TinySkiaCpuRenderContext, y: f64, state: CritterState, phase: f64, frames: usize, secs: f64, t0: f64) {
    for i in 0..frames {
        let t = t0 + secs * i as f64 / frames as f64;
        let r = Rect::new(i as f64 * CELL + 10.0, y + 10.0, CELL - 20.0, CELL - 20.0);
        draw_critter_motion(ctx, r, &view(phase), &CritterMotion { state, t, look_at: None, reduced: false });
    }
}

fn main() {
    let mut a = std::env::args().skip(1);
    let out = PathBuf::from(a.next().expect("out dir"));
    let frames: usize = a.next().and_then(|s| s.parse().ok()).unwrap_or(16);
    let secs: f64 = a.next().and_then(|s| s.parse().ok()).unwrap_or(3.0);
    std::fs::create_dir_all(&out).expect("mkdir");
    let seeds = [0.0, 1.7, 3.1, 4.9, 6.3];
    for (name, state) in [("working", CritterState::Working), ("attention", CritterState::Attention), ("thinking", CritterState::Thinking)] {
        let rows = seeds.len() * 2;
        let mut ctx = TinySkiaCpuRenderContext::new((CELL * frames as f64) as u32, (CELL * rows as f64) as u32, 1.0);
        ctx.set_fill_color("#141414");
        ctx.fill_rect(0.0, 0.0, CELL * frames as f64, CELL * rows as f64);
        // Each seed for two consecutive 3 s windows.
        for (k, ph) in seeds.iter().enumerate() {
            for w in 0..2 {
                row(&mut ctx, CELL * (k * 2 + w) as f64, state, *ph, frames, secs, secs * w as f64);
            }
        }
        ctx.pixmap().save_png(out.join(format!("{name}.png"))).expect("png");
    }
    for i in 0..frames {
        let mut ctx = TinySkiaCpuRenderContext::new(96, 96, 1.0);
        ctx.set_fill_color("#141414");
        ctx.fill_rect(0.0, 0.0, 96.0, 96.0);
        let t = secs * i as f64 / frames as f64;
        draw_critter_motion(&mut ctx, Rect::new(16.0, 16.0, 64.0, 64.0), &view(0.0), &CritterMotion { state: CritterState::Working, t, look_at: None, reduced: false });
        ctx.pixmap().save_png(out.join(format!("working-{i:02}.png"))).expect("png");
    }
}
