//! Shimmer text: a muted label with a light band sweeping across it (a "thinking"
//! status line). The sweep is [`SHIMMER`](crate::ui::motion::SHIMMER); reduced motion
//! paints the plain label.

mod tokens;
use crate::render::RenderContext;
use crate::ui::motion::{Loop, SHIMMER};

/// Look of a shimmer label.
#[derive(Clone, Copy, Debug)]
pub struct ShimmerStyle {
    /// CSS font (`400 13px Inter`).
    pub font: &'static str,
    /// Base (muted) colour.
    pub color: &'static str,
    /// Highlight colour.
    pub highlight: &'static str,
    /// Highlight band width in px.
    pub band: f64,
    /// Sweep.
    pub sweep: Loop,
}

impl Default for ShimmerStyle {
    fn default() -> Self {
        Self { font: "400 13px Inter", color: tokens::BASE, highlight: tokens::HIGHLIGHT, band: 56.0, sweep: SHIMMER }
    }
}

/// Paint `text` with its baseline-free top-left at (`x`, `y`) (`textBaseline = top`)
/// at time `t`; returns its width. The band is three nested clips of rising opacity,
/// so the backend needs only rect clips and plain fills.
pub fn draw_shimmer_text(ctx: &mut dyn RenderContext, text: &str, x: f64, y: f64, h: f64, t: f64, reduced: bool, st: &ShimmerStyle) -> f64 {
    ctx.set_font(st.font);
    ctx.set_text_baseline(crate::render::TextBaseline::Top);
    ctx.set_text_align(crate::render::TextAlign::Left);
    let w = ctx.measure_text(text);
    ctx.set_fill_color(st.color);
    ctx.fill_text(text, x, y);
    if !reduced {
        let c = x + w * st.sweep.at(t, false);
        for k in [1.0, 0.6, 0.3] {
            let bw = st.band * k;
            ctx.save();
            ctx.begin_path();
            ctx.rect(c - bw / 2.0, y - 2.0, bw, h + 4.0);
            ctx.clip();
            ctx.set_fill_color_alpha(st.highlight, 0.35);
            ctx.fill_text(text, x, y);
            ctx.restore();
        }
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sweeps_across_and_holds_when_reduced() {
        assert_eq!(SHIMMER.at(0.0, false), -0.25);
        assert!((SHIMMER.at(1.1, false) - 0.5).abs() < 1e-9);
        let mut ctx = crate::testing::RecordingRenderContext::with_char_width(8.0);
        assert_eq!(draw_shimmer_text(&mut ctx, "abcd", 0.0, 0.0, 16.0, 1.0, true, &ShimmerStyle::default()), 32.0);
    }
}
