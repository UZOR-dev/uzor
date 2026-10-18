//! CSS line-box text placement for canvas-style backends.
//!
//! A browser centres a font's ascent+descent inside the line box
//! (`line-height`) and draws on the alphabetic baseline. Canvas `middle`
//! centres the em box instead, which lands a pixel or so off. These helpers
//! reproduce the CSS position so uzor text lines up with HTML layouts.

use crate::render::{RenderContext, TextAlign, TextBaseline};

/// Vertical font metrics (hhea / typo, in em units).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontMetrics {
    /// Ascent / unitsPerEm.
    pub ascent: f64,
    /// Descent / unitsPerEm (positive).
    pub descent: f64,
}

/// Inter 4 (ascender 1984, descender -494, 2048 upem).
pub const INTER: FontMetrics = FontMetrics { ascent: 1984.0 / 2048.0, descent: 494.0 / 2048.0 };

/// Baseline offset from the top of a CSS line box.
pub fn baseline(px: f64, line_height: f64, m: FontMetrics) -> f64 {
    (line_height - (m.ascent + m.descent) * px) / 2.0 + m.ascent * px
}

/// `fill_text` the way CSS lays out one line whose box starts at `line_top`.
#[allow(clippy::too_many_arguments)]
pub fn fill_text_css(ctx: &mut dyn RenderContext, s: &str, x: f64, line_top: f64, px: f64, line_height: f64, m: FontMetrics, align: TextAlign) {
    ctx.set_text_align(align);
    ctx.set_text_baseline(TextBaseline::Alphabetic);
    ctx.fill_text(s, x, line_top + baseline(px, line_height, m));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inter_14_21() {
        let b = baseline(14.0, 21.0, INTER);
        assert!((b - 15.59).abs() < 0.02, "{b}");
    }
}
