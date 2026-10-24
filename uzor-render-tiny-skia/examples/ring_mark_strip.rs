//! Ring mark motion strip: frames through the real
//! [`uzor::ui::widgets::atomic::ring_mark::draw_ring_mark`] path.
//!
//! ```text
//! cargo run --release -p uzor-render-tiny-skia --example ring_mark_strip -- <out-dir> [frames] [step-s]
//! ```
//!
//! Writes `ring_mark.png`: per mode (idle, loading, busy) a row at 96 px, then rows at
//! 32 and 16 px, light and dark; `frames` frames `step-s` apart (defaults 14, mode slot / 8).
//! Prints the poses too (mode, t, quaternion, star, ring, core) for comparing with other
//! implementations.

use std::path::PathBuf;

use uzor::render::{Painter, ShapeHelpers};
use uzor::types::Rect;
use uzor::ui::widgets::atomic::ring_mark::{draw_ring_mark, ring_mark_pose, RingMarkMode, RingMarkStyle};
use uzor_render_tiny_skia::TinySkiaCpuRenderContext;

const STYLE: RingMarkStyle = RingMarkStyle {
    sides: 6,
    radius: 27.0,
    width: 7.5,
    points: 4,
    star_outer: 16.5,
    star_inner: 5.94,
    ring_color: "#15171c",
    star_color: "#f5a524",
    background: "#ffffff",
    camera: 150.0,
};

fn main() {
    let mut a = std::env::args().skip(1);
    let out = PathBuf::from(a.next().expect("out dir"));
    let frames: usize = a.next().and_then(|s| s.parse().ok()).unwrap_or(14);
    let step: Option<f64> = a.next().and_then(|s| s.parse().ok());
    std::fs::create_dir_all(&out).expect("mkdir");
    let modes = [(RingMarkMode::Idle, 1.0), (RingMarkMode::Loading, 1.6), (RingMarkMode::Busy, 1.1)];
    let sizes = [96.0, 32.0, 16.0];
    let cell = 112.0;
    let row_h: f64 = sizes.iter().map(|s| s + 16.0).sum::<f64>() * 2.0;
    let (w, h) = (cell * frames as f64, row_h * modes.len() as f64);
    let mut ctx = TinySkiaCpuRenderContext::new(w as u32, h as u32, 1.0);
    ctx.set_fill_color("#ffffff");
    ctx.fill_rect(0.0, 0.0, w, h);
    for (m, (mode, t0)) in modes.iter().enumerate() {
        let dt = step.unwrap_or(mode.cycle().slot / 8.0);
        let mut y = row_h * m as f64;
        for dark in [false, true] {
            let style = if dark {
                RingMarkStyle { ring_color: "#f4f2ee", background: "#0f1115", ..STYLE }
            } else {
                STYLE
            };
            let band: f64 = sizes.iter().map(|s| s + 16.0).sum();
            ctx.set_fill_color(style.background);
            ctx.fill_rect(0.0, y, w, band);
            for &s in &sizes {
                for i in 0..frames {
                    let t = t0 + dt * i as f64;
                    let p = ring_mark_pose(*mode, t, 7, &style);
                    if s == 96.0 && !dark {
                        println!("{mode:?} {t:.4} {:.5} {:.5} {:.5} {:.5} {:.5} {:.5} {:.5}", p.turn.w, p.turn.x, p.turn.y, p.turn.z, p.star, p.ring, p.core);
                    }
                    let x = cell * i as f64 + (cell - s) / 2.0;
                    draw_ring_mark(&mut ctx, Rect::new(x.round(), y + 8.0, s, s), &style, &p);
                }
                y += s + 16.0;
            }
        }
    }
    ctx.pixmap().save_png(out.join("ring_mark.png")).expect("png");
}
