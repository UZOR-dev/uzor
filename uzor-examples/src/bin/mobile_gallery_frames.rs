//! Render the mobile shell widgets as 390×844 phone screens (PNG, 2× device pixels):
//! `mobile-gallery-frames <out-dir>`. One file per screen of
//! [`uzor_examples::mobile_gallery::SCREENS`].

use uzor::render::{Painter, ShapeHelpers};
use uzor::ui::dom;
use uzor::ui::media::{self, Edges, MediaEnv};
use uzor::ui::style::State;
use uzor::Rect;
use uzor_examples::mobile_gallery::{MobileGallery, SCREENS};
use uzor_render_tiny_skia::TinySkiaCpuRenderContext;

fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "mobile-gallery".into()));
    std::fs::create_dir_all(&out).expect("out dir");
    let (w, h, scale) = (390.0, 844.0, 2.0);
    for (i, name) in SCREENS.iter().enumerate() {
        let g = MobileGallery::phone(i);
        media::set_current(MediaEnv { keyboard: g.keyboard, ..MediaEnv::touch(w, h, Edges::new(47.0, 0.0, 34.0, 0.0)) });
        let sheet = MobileGallery::sheet();
        let mut ctx = TinySkiaCpuRenderContext::new((w * scale) as u32, (h * scale) as u32, scale);
        ctx.scale(scale, scale);
        ctx.set_fill_color("#121212");
        ctx.fill_rect(0.0, 0.0, w, h);
        let d = dom::lay_out_in(&mut ctx, &sheet, &g.tree(), 0.0, 0.0, w, Some(Rect::new(0.0, 0.0, w, h)), &|_| State::NONE);
        dom::paint(&mut ctx, &d);
        // The soft keyboard is the system's: a plain block where it would be.
        if g.keyboard > 0.0 {
            ctx.set_fill_color("#1d1d1f");
            ctx.fill_rect(0.0, h - g.keyboard, w, g.keyboard);
        }
        ctx.pixmap().save_png(out.join(format!("{name}.png"))).expect("png");
    }
    eprintln!("{} screens in {}", SCREENS.len(), out.display());
}
