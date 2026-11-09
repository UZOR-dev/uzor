//! Render the dashboard widgets to PNG shots: `dom-widgets-frames <out-dir> [scale]`.
//! Writes the catalog (EN and RU), the task tree folded, the mini panels and the glass
//! overlays (the space menu, a popover, a dialog) over the catalog.

use uzor::render::{Painter, ShapeHelpers};
use uzor::ui::dom;
use uzor::ui::style::State;
use uzor::Rect;
use uzor_examples::dom_widgets::{Lang, WidgetsGallery};
use uzor_render_tiny_skia::TinySkiaCpuRenderContext;

fn shot(out: &std::path::Path, name: &str, w: f64, h: f64, scale: f64, tree: &uzor::ui::dom::El, sheet: &uzor::ui::style::Sheet, hover: Option<&str>) {
    let mut ctx = TinySkiaCpuRenderContext::new((w * scale) as u32, (h * scale) as u32, scale);
    ctx.scale(scale, scale);
    ctx.set_fill_color("#141414");
    ctx.fill_rect(0.0, 0.0, w, h);
    let state = |id: &str| if hover == Some(id) { State::HOVER } else { State::NONE };
    let d = dom::lay_out_in(&mut ctx, sheet, tree, 0.0, 0.0, w, Some(Rect::new(0.0, 0.0, w, h)), &state);
    dom::paint_slots(&mut ctx, &d, &mut |_, _, _| {});
    let p = out.join(format!("{name}.png"));
    ctx.pixmap().save_png(&p).expect("png");
    eprintln!("{}", p.display());
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = std::path::PathBuf::from(args.next().unwrap_or_else(|| "dom-widgets".into()));
    let scale: f64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1.0);
    std::fs::create_dir_all(&out).expect("out dir");
    let sheet = WidgetsGallery::sheet();
    let (w, h) = (1280.0, 900.0);
    let mut g = WidgetsGallery::default();
    shot(&out, "01-catalog-en", w, h, scale, &g.tree(w, h), &sheet, None);
    g.lang = Lang::Ru;
    shot(&out, "02-catalog-ru", w, h, scale, &g.tree(w, h), &sheet, None);
    g.folded.insert("w".into());
    g.sel = Some("b".into());
    shot(&out, "03-tree-folded-ru", w, h, scale, &g.tree(w, h), &sheet, Some("tasks:fold:w"));
    g.folded.clear();
    g.sel = None;
    shot(&out, "04-mini-panels-ru", w, 560.0, scale, &g.panels_tree(560.0), &sheet, None);
    g.menu = true;
    shot(&out, "05-glass-space-menu-ru", w, h, scale, &g.tree(w, h), &sheet, Some("space:item:1"));
    g.menu = false;
    g.popover = true;
    shot(&out, "06-glass-popover-ru", w, h, scale, &g.tree(w, h), &sheet, None);
    g.popover = false;
    g.dialog = true;
    shot(&out, "07-glass-dialog-ru", w, h, scale, &g.tree(w, h), &sheet, None);
}
