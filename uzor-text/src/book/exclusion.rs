//! Exclusion zones: rectangles (images) that text flows around.

/// Per-side clearance between an image and the text around it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Gaps {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Gaps {
    pub fn uniform(v: f64) -> Self {
        Self { top: v, right: v, bottom: v, left: v }
    }
}

/// How text treats an exclusion inside the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WrapMode {
    /// Text runs on the side with more room (left or right of the image).
    #[default]
    Auto,
    /// Text runs only to the LEFT of the image.
    TextLeft,
    /// Text runs only to the RIGHT of the image.
    TextRight,
    /// The image takes its own band: no text beside it, text above and below.
    Band,
}

/// A rectangle in frame coordinates (`x`, `y` from the frame's top-left).
/// It may extend past the frame (bleed); only the part inside the frame
/// affects text.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Exclusion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub gaps: Gaps,
    pub mode: WrapMode,
}

impl Exclusion {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height, gaps: Gaps::default(), mode: WrapMode::Auto }
    }
    pub fn with_gaps(mut self, gaps: Gaps) -> Self {
        self.gaps = gaps;
        self
    }
    pub fn with_mode(mut self, mode: WrapMode) -> Self {
        self.mode = mode;
        self
    }
    /// Rectangle with an aspect-preserving height for `width`
    /// (`source_w` × `source_h` is the image's pixel size).
    pub fn keep_aspect(x: f64, y: f64, width: f64, source_w: f64, source_h: f64) -> Self {
        let h = if source_w > 0.0 { width * source_h / source_w } else { 0.0 };
        Self::new(x, y, width, h)
    }
    /// The rectangle grown by its gaps.
    pub fn outer(&self) -> (f64, f64, f64, f64) {
        (
            self.x - self.gaps.left,
            self.y - self.gaps.top,
            self.x + self.width + self.gaps.right,
            self.y + self.height + self.gaps.bottom,
        )
    }
}

/// The widest horizontal span `(x, width)` of a `frame_width`-wide frame
/// that is free of every exclusion over the band `[top, bottom)`.
/// `None` when nothing is free (the band is blocked; move down).
/// Ties pick the leftmost span, so the result is deterministic.
pub fn available_span(frame_width: f64, top: f64, bottom: f64, exclusions: &[Exclusion]) -> Option<(f64, f64)> {
    let mut free: Vec<(f64, f64)> = vec![(0.0, frame_width)];
    for ex in exclusions {
        let (l, t, r, b) = ex.outer();
        if b <= top || t >= bottom || ex.width <= 0.0 || ex.height <= 0.0 {
            continue;
        }
        let (cut_l, cut_r) = match ex.mode {
            WrapMode::Band => (f64::NEG_INFINITY, f64::INFINITY),
            WrapMode::TextLeft => (l, f64::INFINITY),
            WrapMode::TextRight => (f64::NEG_INFINITY, r),
            WrapMode::Auto => (l, r),
        };
        let mut next = Vec::with_capacity(free.len() + 1);
        for (a, z) in free {
            if cut_l > a {
                next.push((a, z.min(cut_l)));
            }
            if cut_r < z {
                next.push((a.max(cut_r), z));
            }
        }
        free = next.into_iter().filter(|(a, z)| z - a > 1e-9).collect();
    }
    let mut best: Option<(f64, f64)> = None;
    for (a, z) in free {
        let w = z - a;
        if best.is_none_or(|(_, bw)| w > bw + 1e-9) {
            best = Some((a, w));
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_exclusion_gives_full_width() {
        assert_eq!(available_span(300.0, 0.0, 20.0, &[]), Some((0.0, 300.0)));
    }

    #[test]
    fn right_image_with_gap_leaves_left_span() {
        let ex = [Exclusion::new(200.0, 0.0, 100.0, 50.0).with_gaps(Gaps { left: 12.0, ..Gaps::default() })];
        assert_eq!(available_span(300.0, 0.0, 20.0, &ex), Some((0.0, 188.0)));
        // Below the image (and its zero bottom gap) text is full width again.
        assert_eq!(available_span(300.0, 50.0, 70.0, &ex), Some((0.0, 300.0)));
    }

    #[test]
    fn bottom_gap_keeps_text_away_until_cleared() {
        let ex = [Exclusion::new(0.0, 0.0, 100.0, 50.0).with_gaps(Gaps { right: 10.0, bottom: 8.0, ..Gaps::default() })];
        assert_eq!(available_span(300.0, 52.0, 60.0, &ex), Some((110.0, 190.0)));
        assert_eq!(available_span(300.0, 58.0, 70.0, &ex), Some((0.0, 300.0)));
    }

    #[test]
    fn band_blocks_the_whole_width() {
        let ex = [Exclusion::new(100.0, 10.0, 50.0, 30.0).with_mode(WrapMode::Band)];
        assert_eq!(available_span(300.0, 0.0, 20.0, &ex), None);
        assert_eq!(available_span(300.0, 0.0, 10.0, &ex), Some((0.0, 300.0)));
    }

    #[test]
    fn forced_side_and_auto_pick_wider() {
        let ex = Exclusion::new(100.0, 0.0, 50.0, 30.0);
        assert_eq!(available_span(300.0, 0.0, 10.0, &[ex]), Some((150.0, 150.0)));
        assert_eq!(available_span(300.0, 0.0, 10.0, &[ex.with_mode(WrapMode::TextLeft)]), Some((0.0, 100.0)));
        assert_eq!(available_span(300.0, 0.0, 10.0, &[ex.with_mode(WrapMode::TextRight)]), Some((150.0, 150.0)));
    }

    #[test]
    fn bleed_image_past_frame_edge_is_clipped_to_frame() {
        let ex = [Exclusion::new(250.0, 0.0, 200.0, 40.0)];
        assert_eq!(available_span(300.0, 0.0, 10.0, &ex), Some((0.0, 250.0)));
    }

    #[test]
    fn keep_aspect_scales_height() {
        let ex = Exclusion::keep_aspect(0.0, 0.0, 100.0, 400.0, 300.0);
        assert!((ex.height - 75.0).abs() < 1e-9);
    }
}
