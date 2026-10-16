//! Paragraph layout inside a frame with exclusion zones (text wraps around
//! images). Uses the same atom stream (shaping, NBSP, forced `\n` breaks)
//! as [`crate::layout::layout_paragraph`]; only the per-line width varies.

use crate::layout::greedy::{self, Atom};
use crate::layout::GlyphLayout;
use crate::model::Paragraph;
use crate::shape::LineShaper;

use super::exclusion::{available_span, Exclusion};

/// One placed line in frame coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct WrapLine {
    /// Left edge of the free span this line was set into.
    pub x: f64,
    pub y_top: f64,
    pub height: f64,
    pub baseline_y: f64,
    /// Width of the free span.
    pub available_width: f64,
    /// Width actually used by the line's content.
    pub content_width: f64,
    /// `true` when one unbreakable atom was wider than the full frame
    /// (the line overflows; reported, never silently clipped).
    pub overfull: bool,
}

/// Result of [`layout_around`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WrappedParagraph {
    pub lines: Vec<WrapLine>,
    /// Glyphs in frame coordinates; `line_index` indexes `lines`.
    pub glyphs: Vec<GlyphLayout>,
    /// Y just below the last line.
    pub bottom: f64,
    /// `true` when text did not fit above `max_bottom`; the remaining text
    /// was not placed.
    pub overflowed: bool,
}

/// Lay `paragraph` (its `max_width` is ignored) into a `frame_width`-wide
/// frame starting at `start_y`, flowing around `exclusions`. Lines never
/// overlap an exclusion's rect plus its gaps; when a band beside an image is
/// too narrow for the next word, the line moves below the image. Under the
/// image text returns to full width. `max_bottom` (frame height) stops the
/// flow and sets `overflowed`. Deterministic: the same inputs give the same
/// output.
pub fn layout_around(
    paragraph: &Paragraph<'_>,
    shaper: &dyn LineShaper,
    frame_width: f64,
    start_y: f64,
    exclusions: &[Exclusion],
    max_bottom: Option<f64>,
) -> WrappedParagraph {
    let atoms = greedy::build_atom_stream(paragraph, shaper);
    let natural_lh = paragraph.runs.iter().map(|r| r.font.size_px * 1.2).fold(0.0_f64, f64::max).max(1.0);
    let lh = paragraph.line_height.unwrap_or(natural_lh).max(1.0);

    let mut out = WrappedParagraph { bottom: start_y, ..Default::default() };
    let mut i = 0usize;
    let mut y = start_y;
    let mut guard = 0usize;

    while i < atoms.len() {
        guard += 1;
        if guard > 100_000 {
            break;
        }
        // Skip glue at line start.
        while matches!(atoms.get(i), Some(Atom::Text(t)) if t.is_glue) {
            i += 1;
        }
        if i >= atoms.len() {
            break;
        }
        if matches!(atoms[i], Atom::Break) {
            // Forced break on an empty line: an empty line of height lh.
            i += 1;
            y += lh;
            out.bottom = y;
            continue;
        }
        if let Some(mb) = max_bottom {
            if y + lh > mb + 1e-9 {
                out.overflowed = true;
                break;
            }
        }
        let Some((x0, width)) = available_span(frame_width, y, y + lh, exclusions) else {
            y = next_clear_y(y, lh, exclusions).max(y + 1.0);
            continue;
        };
        let first_w = greedy::atom_width(&atoms[i]);
        let full_width_line = width >= frame_width - 1e-9;
        if first_w > width + 1e-9 && !full_width_line {
            y = next_clear_y(y, lh, exclusions).max(y + 1.0);
            continue;
        }

        // Greedy pack into `width`.
        let mut end = i;
        let mut used = 0.0_f64;
        let mut last_fit_end = i;
        let mut last_fit_width = 0.0_f64;
        while end < atoms.len() {
            match &atoms[end] {
                Atom::Break => break,
                Atom::Text(t) if t.is_glue => {
                    used += t.width;
                    end += 1;
                }
                a => {
                    let w = greedy::atom_width(a);
                    if end > i && used + w > width + 1e-9 {
                        break;
                    }
                    used += w;
                    end += 1;
                    last_fit_end = end;
                    last_fit_width = used;
                }
            }
        }
        let line_atoms = &atoms[i..last_fit_end];
        let (mut asc, mut desc) = (0.0_f64, 0.0_f64);
        for a in line_atoms {
            let (aa, dd) = greedy::atom_metrics(a);
            asc = asc.max(aa);
            desc = desc.max(dd);
        }
        let baseline = y + asc + ((lh - (asc + desc)).max(0.0)) / 2.0;
        let line_index = out.lines.len();
        let mut pen = x0;
        for a in line_atoms {
            match a {
                Atom::Text(t) => {
                    let color = paragraph.runs.get(t.run_index).and_then(|r| r.color);
                    if !t.is_glue {
                        for g in &t.glyphs {
                            out.glyphs.push(GlyphLayout {
                                cluster: g.cluster.clone(),
                                run_index: t.run_index,
                                line_index,
                                x: pen + g.x,
                                y: baseline + g.y_offset,
                                advance: g.advance,
                                width: g.width,
                                font: t.shape_font,
                                color,
                            });
                        }
                    }
                    pen += t.width;
                }
                other => pen += greedy::atom_width(other),
            }
        }
        out.lines.push(WrapLine {
            x: x0,
            y_top: y,
            height: lh,
            baseline_y: baseline,
            available_width: width,
            content_width: last_fit_width,
            overfull: last_fit_width > width + 1e-9,
        });
        y += lh;
        out.bottom = y;
        i = last_fit_end;
        // Consume a forced break that ends this line.
        while matches!(atoms.get(i), Some(Atom::Text(t)) if t.is_glue) {
            i += 1;
        }
        if matches!(atoms.get(i), Some(Atom::Break)) {
            i += 1;
        }
    }
    out
}

/// The smallest exclusion bottom (with gap) below `y` among exclusions that
/// touch the band `[y, y+lh)`.
fn next_clear_y(y: f64, lh: f64, exclusions: &[Exclusion]) -> f64 {
    exclusions
        .iter()
        .map(|e| e.outer())
        .filter(|&(_, t, _, b)| b > y && t < y + lh)
        .map(|(_, _, _, b)| b)
        .fold(f64::INFINITY, f64::min)
        .min(y + lh)
        .max(y + 1e-6)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::exclusion::{Gaps, WrapMode};
    use crate::model::{FontSpec, StyledRun};
    use crate::shape::CosmicShaper;
    use uzor::fonts::FontFamily;

    const TEXT: &str = "Жил-был у бабушки серенький козлик. Вот как, вот как, серенький козлик. \
Бабушка козлика очень любила, вот как, вот как, очень любила. Вздумалось козлику в лес погуляти.";

    fn para<'a>(runs: &'a [StyledRun<'a>]) -> Paragraph<'a> {
        Paragraph {
            runs,
            inline_boxes: &[],
            align: Default::default(),
            line_height: Some(20.0),
            max_width: 0.0,
            break_strategy: Default::default(),
            hyphenation: Default::default(),
            max_consecutive_hyphens: None,
            line_break_params: Default::default(),
            protrusion: None,
        }
    }

    fn overlaps(l: &WrapLine, e: &Exclusion) -> bool {
        let (a, t, z, b) = e.outer();
        let lx1 = l.x + l.content_width;
        l.y_top < b && l.y_top + l.height > t && l.x < z && lx1 > a
    }

    #[test]
    fn text_wraps_around_image_and_returns_to_full_width() {
        let runs = [StyledRun::new(TEXT, FontSpec::new(FontFamily::Roboto, 14.0))];
        let shaper = CosmicShaper::headless();
        let ex = [Exclusion::new(180.0, 0.0, 120.0, 60.0).with_gaps(Gaps::uniform(8.0))];
        let out = layout_around(&para(&runs), &shaper, 300.0, 0.0, &ex, None);
        assert!(out.lines.len() > 4);
        for l in &out.lines {
            assert!(!overlaps(l, &ex[0]), "line overlaps image: {l:?}");
            assert!(l.content_width <= l.available_width + 1e-6);
        }
        assert!((out.lines[0].available_width - 172.0).abs() < 1e-6);
        let below = out.lines.iter().find(|l| l.y_top >= 68.0).expect("a line below image");
        assert!((below.available_width - 300.0).abs() < 1e-6);
        assert!(out.glyphs.iter().all(|g| g.x >= -1e-6));
    }

    #[test]
    fn band_image_pushes_text_below() {
        let runs = [StyledRun::new(TEXT, FontSpec::new(FontFamily::Roboto, 14.0))];
        let shaper = CosmicShaper::headless();
        let ex = [Exclusion::new(50.0, 0.0, 100.0, 50.0).with_mode(WrapMode::Band).with_gaps(Gaps { bottom: 10.0, ..Gaps::default() })];
        let out = layout_around(&para(&runs), &shaper, 300.0, 0.0, &ex, None);
        assert!(out.lines[0].y_top >= 60.0 - 1e-6);
    }

    #[test]
    fn overflow_is_reported() {
        let runs = [StyledRun::new(TEXT, FontSpec::new(FontFamily::Roboto, 14.0))];
        let shaper = CosmicShaper::headless();
        let out = layout_around(&para(&runs), &shaper, 200.0, 0.0, &[], Some(40.0));
        assert!(out.overflowed);
        assert_eq!(out.lines.len(), 2);
    }

    #[test]
    fn forced_break_starts_new_line_in_same_paragraph() {
        let runs = [StyledRun::new("Раз\nдва", FontSpec::new(FontFamily::Roboto, 14.0))];
        let shaper = CosmicShaper::headless();
        let out = layout_around(&para(&runs), &shaper, 300.0, 0.0, &[], None);
        assert_eq!(out.lines.len(), 2);
    }

    #[test]
    fn layout_is_deterministic() {
        let runs = [StyledRun::new(TEXT, FontSpec::new(FontFamily::Roboto, 14.0))];
        let shaper = CosmicShaper::headless();
        let ex = [Exclusion::new(0.0, 10.0, 90.0, 70.0).with_gaps(Gaps::uniform(6.0))];
        let a = layout_around(&para(&runs), &shaper, 280.0, 0.0, &ex, None);
        let b = layout_around(&para(&runs), &shaper, 280.0, 0.0, &ex, None);
        assert_eq!(a, b);
    }

    #[test]
    fn bound_dash_never_starts_a_line() {
        let text = crate::book::prep::bind_dashes("Пёс — друг человека, а кот — хозяин дома — так говорят все дети на свете — и это правда");
        let runs = [StyledRun::new(&text, FontSpec::new(FontFamily::Roboto, 14.0))];
        let shaper = CosmicShaper::headless();
        for w in [90.0, 120.0, 150.0, 180.0] {
            let out = layout_around(&para(&runs), &shaper, w, 0.0, &[], None);
            for li in 0..out.lines.len() {
                let first = out.glyphs.iter().find(|g| g.line_index == li).unwrap();
                assert_ne!(first.cluster, "—", "width {w}: line {li} starts with a dash");
            }
        }
    }
}
