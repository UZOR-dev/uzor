//! Render the tray widgets to PNG shots: `tray-widgets-frames <out-dir> [scale]`.
//! Writes every widget alone in the flyout (glass over a wallpaper) and the thumbnail
//! (opaque) looks, dark and light, Russian, plus the whole gallery page.

use uzor::render::{Painter, ShapeHelpers};
use uzor::ui::dom;
use uzor::ui::style::State;
use uzor::ui::tray::{self, TrayLook, TrayWidget};
use uzor::Rect;
use uzor_examples::tray_widgets::TrayGallery;
use uzor_render_tiny_skia::TinySkiaCpuRenderContext;

fn save(ctx: &TinySkiaCpuRenderContext, out: &std::path::Path, name: &str) {
    let p = out.join(format!("{name}.png"));
    ctx.pixmap().save_png(&p).expect("png");
    eprintln!("{}", p.display());
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = std::path::PathBuf::from(args.next().unwrap_or_else(|| "tray-widgets".into()));
    let scale: f64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(2.0);
    std::fs::create_dir_all(&out).expect("out dir");
    let sheet = TrayGallery::sheet();
    let t = 0.8;
    for dark in [true, false] {
        let g = TrayGallery { dark, ..Default::default() };
        let theme = if dark { "dark" } else { "light" };
        // Each widget, glass over the wallpaper (a 40 px margin), and opaque.
        for w in TrayWidget::ALL {
            let (cw, ch) = w.size();
            for (look, name) in [(TrayLook::flyout(dark), "flyout"), (TrayLook::solid(dark), "thumb")] {
                let (pw, ph) = (cw + 80.0, ch + 80.0);
                let mut ctx = TinySkiaCpuRenderContext::new((pw * scale) as u32, (ph * scale) as u32, scale);
                ctx.scale(scale, scale);
                if look.glass {
                    g.backdrop(&mut ctx, Rect::new(0.0, 0.0, pw, ph));
                } else {
                    ctx.set_fill_color(if dark { "#101012" } else { "#e9e9ee" });
                    ctx.fill_rect(0.0, 0.0, pw, ph);
                }
                tray::render(&mut ctx, &sheet, w, &g.model(), &g.labels(), &look, 40.0, 40.0, t);
                save(&ctx, &out, &format!("{}-{name}-{theme}-ru", w.key()));
            }
        }
        // The gallery page.
        let (pw, ph) = (1400.0, 760.0);
        let mut ctx = TinySkiaCpuRenderContext::new((pw * scale) as u32, (ph * scale) as u32, scale);
        ctx.scale(scale, scale);
        g.backdrop(&mut ctx, Rect::new(0.0, 0.0, pw, ph));
        let d = dom::lay_out_in(&mut ctx, &sheet, &g.tree(pw), 0.0, 0.0, pw, Some(Rect::new(0.0, 0.0, pw, ph)), &|_| State::NONE);
        dom::paint_slots(&mut ctx, &d, &mut |ctx, k, r| g.paint_slot(ctx, k, r, t));
        save(&ctx, &out, &format!("gallery-{theme}-ru"));
    }
}
