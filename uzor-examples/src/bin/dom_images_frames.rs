//! Render the raster-image demo to PNG shots: `dom-images-frames <out-dir> [scale]`.
//! Writes the page dark in Russian and light in English.

use uzor::render::Painter;
use uzor_examples::dom_images::ImagesDemo;
use uzor_render_tiny_skia::TinySkiaCpuRenderContext;

fn main() {
    let mut args = std::env::args().skip(1);
    let out = std::path::PathBuf::from(args.next().unwrap_or_else(|| "dom-images".into()));
    let scale: f64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(2.0);
    std::fs::create_dir_all(&out).expect("out dir");
    let (w, h) = (1040.0, 260.0);
    for (name, g) in [("01-images-dark-ru", ImagesDemo::default()), ("02-images-light-en", ImagesDemo { ru: false, dark: false })] {
        let mut ctx = TinySkiaCpuRenderContext::new((w * scale) as u32, (h * scale) as u32, scale);
        ctx.scale(scale, scale);
        g.render(&mut ctx, w, h);
        let p = out.join(format!("{name}.png"));
        ctx.pixmap().save_png(&p).expect("png");
        eprintln!("{}", p.display());
    }
}
