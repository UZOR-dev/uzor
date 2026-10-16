//! Shrink-to-fit with a hard minimum font size: text is never set below the
//! floor; if it still does not fit, the overflow is reported.

use crate::model::{Paragraph, StyledRun};
use crate::shape::LineShaper;

use super::exclusion::Exclusion;
use super::wrap::{layout_around, WrappedParagraph};

/// Result of [`fit_with_font_floor`].
#[derive(Debug, Clone, PartialEq)]
pub struct FitResult {
    /// Scale applied to every run's font size (and fixed line height).
    pub scale: f64,
    /// Smallest font size actually used, px.
    pub smallest_size_px: f64,
    /// `true` when the floor stopped the shrinking.
    pub hit_floor: bool,
    /// `true` when the text still does not fit at the final scale.
    pub overflowed: bool,
    pub layout: WrappedParagraph,
}

/// Lay `paragraph` into a `frame_width` × `max_height` frame around
/// `exclusions`, shrinking all font sizes together in steps of
/// `step` (e.g. `0.02` = 2 %) until it fits, never letting any run go below
/// `min_size_px`.
pub fn fit_with_font_floor(
    paragraph: &Paragraph<'_>,
    shaper: &dyn LineShaper,
    frame_width: f64,
    max_height: f64,
    exclusions: &[Exclusion],
    min_size_px: f64,
    step: f64,
) -> FitResult {
    let step = if step > 0.0 { step } else { 0.02 };
    let smallest = paragraph.runs.iter().map(|r| r.font.size_px).fold(f64::INFINITY, f64::min);
    let min_scale = if smallest.is_finite() && smallest > 0.0 { (min_size_px / smallest).min(1.0) } else { 1.0 };
    let mut k = 0u32;
    loop {
        let raw = 1.0 - step * f64::from(k);
        let scale = raw.max(min_scale);
        let runs: Vec<StyledRun<'_>> = paragraph
            .runs
            .iter()
            .map(|r| {
                let mut r = *r;
                r.font.size_px *= scale;
                r
            })
            .collect();
        let mut p = *paragraph;
        p.runs = &runs;
        p.line_height = paragraph.line_height.map(|h| h * scale);
        let layout = layout_around(&p, shaper, frame_width, 0.0, exclusions, Some(max_height));
        let at_floor = scale <= min_scale + 1e-12;
        if !layout.overflowed || at_floor {
            return FitResult {
                scale,
                smallest_size_px: if smallest.is_finite() { smallest * scale } else { 0.0 },
                hit_floor: at_floor && scale < 1.0,
                overflowed: layout.overflowed,
                layout,
            };
        }
        k += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FontSpec;
    use crate::shape::CosmicShaper;
    use uzor::fonts::FontFamily;

    fn para<'a>(runs: &'a [StyledRun<'a>]) -> Paragraph<'a> {
        Paragraph {
            runs,
            inline_boxes: &[],
            align: Default::default(),
            line_height: Some(24.0),
            max_width: 0.0,
            break_strategy: Default::default(),
            hyphenation: Default::default(),
            max_consecutive_hyphens: None,
            line_break_params: Default::default(),
            protrusion: None,
        }
    }

    const T: &str = "Три девицы под окном пряли поздно вечерком. Кабы я была царица, говорит одна девица.";

    #[test]
    fn fits_without_shrinking_when_room() {
        let runs = [StyledRun::new(T, FontSpec::new(FontFamily::Roboto, 20.0))];
        let r = fit_with_font_floor(&para(&runs), &CosmicShaper::headless(), 400.0, 1000.0, &[], 12.0, 0.05);
        assert_eq!(r.scale, 1.0);
        assert!(!r.overflowed && !r.hit_floor);
    }

    #[test]
    fn shrinks_but_never_below_floor_and_reports_overflow() {
        let runs = [StyledRun::new(T, FontSpec::new(FontFamily::Roboto, 20.0))];
        let r = fit_with_font_floor(&para(&runs), &CosmicShaper::headless(), 120.0, 30.0, &[], 16.0, 0.05);
        assert!(r.hit_floor);
        assert!(r.overflowed);
        assert!((r.smallest_size_px - 16.0).abs() < 1e-9);
    }

    #[test]
    fn shrinks_until_it_fits() {
        let runs = [StyledRun::new(T, FontSpec::new(FontFamily::Roboto, 20.0))];
        let full = fit_with_font_floor(&para(&runs), &CosmicShaper::headless(), 300.0, 10_000.0, &[], 1.0, 0.05);
        let h = full.layout.bottom - 24.0; // one line less than natural
        let r = fit_with_font_floor(&para(&runs), &CosmicShaper::headless(), 300.0, h, &[], 8.0, 0.05);
        assert!(!r.overflowed);
        assert!(r.scale < 1.0 && r.smallest_size_px >= 8.0);
    }
}
