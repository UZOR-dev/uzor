//! Regression shots for the icons-merge: every drawing-tool mark whose
//! markup was replaced, and every icon added, rendered at 24px and 48px
//! through `draw_svg_icon`. Each shot is non-empty and thin-pen-guarded.

use uzor::render::{draw_svg_icon, Painter, ShapeHelpers};
use uzor::ui::assets::icons::{chart, ui};
use uzor_render_tiny_skia::TinySkiaCpuRenderContext;

fn root_stroke_width(svg: &str) -> Option<f64> {
    let tag_end = svg.find('>')?;
    let tag = &svg[..tag_end];
    let attr_start = tag.find("stroke-width=\"")? + "stroke-width=\"".len();
    let attr_end = tag[attr_start..].find('"')?;
    tag[attr_start..attr_start + attr_end].parse::<f64>().ok()
}

fn assert_root_stroke_width_is_thin(name: &str, svg: &str) {
    if let Some(w) = root_stroke_width(svg) {
        assert!(
            w <= 1.5,
            "{name}: root stroke-width {w} exceeds the 1.5 thin-icon ceiling"
        );
    }
}

fn render_is_non_empty(svg: &str, size: f64) -> bool {
    let dpr = 2.0;
    let px = (size * dpr).round().max(1.0) as u32;
    let mut ctx = TinySkiaCpuRenderContext::new(px, px, dpr);
    ctx.scale(dpr, dpr);
    ctx.set_fill_color("#131722");
    ctx.fill_rect(0.0, 0.0, size, size);
    let before = ctx.pixels().to_vec();
    draw_svg_icon(&mut ctx, svg, 0.0, 0.0, size, size, "#d1d4dc");
    ctx.pixels() != before.as_slice()
}

fn shots() -> Vec<(&'static str, &'static str)> {
    let mut out = Vec::with_capacity(chart::MERGE_SHOTS.len() + ui::MERGE_ADDED.len());
    out.extend(chart::MERGE_SHOTS.iter().copied());
    out.extend(ui::MERGE_ADDED.iter().copied());
    out
}

#[test]
fn merge_shots_render_non_empty_and_stay_thin() {
    let shots = shots();
    assert_eq!(shots.len(), 30, "27 replaced drawing-tool marks + 3 added ui icons");
    for (name, svg) in &shots {
        assert!(svg.starts_with("<svg") && svg.trim_end().ends_with("</svg>"), "{name} is one svg");
        assert_root_stroke_width_is_thin(name, svg);
        for size in [24.0, 48.0] {
            assert!(
                render_is_non_empty(svg, size),
                "{name} at {size}px left the pixmap unchanged"
            );
        }
    }
}
