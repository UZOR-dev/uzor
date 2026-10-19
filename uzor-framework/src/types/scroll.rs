//! Scroll container model: wheel lines, trackpad pixels (1:1 + fling inertia
//! through [`uzor::animation::math::decay::Decay`]), a draggable scrollbar
//! thumb, and optional stick-to-end (chat threads).
//!
//! Pure state; [`crate::widgets::scroll_area`] wires it to a panel (hit
//! registration, wheel routing, thumb drag, paint).

use uzor::ui::animation::math::decay::Decay;
use uzor::Rect;

/// Look and feel.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollStyle {
    /// Pixels per wheel line (winit `LineDelta`).
    pub line_px: f64,
    /// Pixels per framework scroll unit for precise (pixel) deltas; the input
    /// engine normalises pixels to lines at this rate.
    pub pixel_unit: f64,
    /// Per-frame friction of the fling (0..1).
    pub friction: f64,
    /// Fling starts above this release speed, px/s.
    pub fling_min: f64,
    /// A precise gesture is over when no delta arrived for this long, ms.
    pub release_ms: f64,
    /// Scrollbar width and edge inset.
    pub bar_w: f64,
    /// Inset from the right edge.
    pub bar_inset: f64,
    /// Minimum thumb length.
    pub min_thumb: f64,
    /// Thumb colour idle / hover-or-drag.
    pub thumb: &'static str,
    /// Hovered / dragged thumb.
    pub thumb_hot: &'static str,
    /// Synthesise a fling after a precise gesture. Off in the browser: like DOM scrolling,
    /// the offset follows the deltas 1:1 and momentum comes from the OS / browser's own
    /// kinetic events (a synthetic fling on top made a wheel notch drift on after the hand
    /// stopped — the "scroll lags" report). On native it only follows touchpad-sized deltas.
    pub fling: bool,
    /// A precise delta larger than this (px) is a wheel notch, not a touchpad sample: no fling.
    pub fling_max_step: f64,
}

impl Default for ScrollStyle {
    fn default() -> Self {
        Self {
            line_px: 40.0,
            pixel_unit: 20.0,
            friction: 0.95,
            fling_min: 120.0,
            release_ms: 70.0,
            bar_w: 6.0,
            bar_inset: 3.0,
            min_thumb: 24.0,
            thumb: "rgba(255,255,255,0.16)",
            thumb_hot: "rgba(255,255,255,0.32)",
            fling: !cfg!(target_arch = "wasm32"),
            fling_max_step: 40.0,
        }
    }
}

/// One scrollable's state. Offset 0 = top.
#[derive(Clone, Debug, Default)]
pub struct ScrollView {
    /// Pixels scrolled from the top.
    pub offset: f64,
    /// Keep the end in view while content grows (chat threads); cleared when
    /// the user scrolls away from the end, set again on reaching it.
    pub follow_end: bool,
    content: f64,
    view: f64,
    /// (time ms, offset) samples of the current precise gesture.
    samples: Vec<(f64, f64)>,
    last_precise_ms: Option<f64>,
    fling: Option<(Decay, f64, f64)>,
    drag: Option<(f64, f64)>,
}

impl ScrollView {
    /// A view that starts at the end and follows it.
    pub fn at_end() -> Self {
        Self { follow_end: true, ..Self::default() }
    }

    /// Largest offset.
    pub fn max(&self) -> f64 {
        (self.content - self.view).max(0.0)
    }

    /// Content taller than the viewport.
    pub fn scrollable(&self) -> bool {
        self.content > self.view + 0.5
    }

    /// At (or within half a pixel of) the end.
    pub fn at_bottom(&self) -> bool {
        self.offset >= self.max() - 0.5
    }

    /// At the top.
    pub fn at_top(&self) -> bool {
        self.offset <= 0.5
    }

    /// Set the extents for this frame; clamps, and keeps the end when following.
    pub fn set_extent(&mut self, content: f64, view: f64) {
        self.content = content.max(0.0);
        self.view = view.max(0.0);
        if self.follow_end && self.drag.is_none() && self.fling.is_none() {
            self.offset = self.max();
        }
        self.offset = self.offset.clamp(0.0, self.max());
    }

    fn moved(&mut self) {
        self.offset = self.offset.clamp(0.0, self.max());
        self.follow_end = self.at_bottom();
    }

    /// Room left to scroll towards `dy`'s direction (positive dy = towards the top).
    pub fn can_scroll(&self, dy: f64) -> bool {
        self.scrollable() && ((dy > 0.0 && !self.at_top()) || (dy < 0.0 && !self.at_bottom()))
    }

    /// Feed one frame's wheel delta in framework units (positive = towards
    /// the top, winit's sign). `precise` = pixel deltas. Returns whether the
    /// offset moved.
    pub fn wheel(&mut self, dy: f64, precise: bool, now_ms: f64, st: &ScrollStyle) -> bool {
        if dy == 0.0 || !self.scrollable() {
            return false;
        }
        let before = self.offset;
        self.fling = None;
        if precise {
            self.offset -= dy * st.pixel_unit;
            if st.fling && (dy * st.pixel_unit).abs() <= st.fling_max_step {
                self.last_precise_ms = Some(now_ms);
                self.samples.push((now_ms, self.offset));
                self.samples.retain(|(t, _)| now_ms - t <= 100.0);
            } else {
                self.last_precise_ms = None;
                self.samples.clear();
            }
        } else {
            self.offset -= dy * st.line_px;
            self.samples.clear();
        }
        self.moved();
        (self.offset - before).abs() > 0.01
    }

    /// Advance inertia. Returns `true` while animating (request a frame).
    pub fn tick(&mut self, now_ms: f64, st: &ScrollStyle) -> bool {
        if let Some(t) = self.last_precise_ms {
            if now_ms - t >= st.release_ms {
                self.last_precise_ms = None;
                let v = match (self.samples.first(), self.samples.last()) {
                    (Some(a), Some(b)) if b.0 > a.0 => (b.1 - a.1) / ((b.0 - a.0) / 1000.0),
                    _ => 0.0,
                };
                self.samples.clear();
                if v.abs() >= st.fling_min {
                    let d = Decay::new(v).friction(st.friction).rest_threshold(5.0);
                    self.fling = Some((d, now_ms, self.offset));
                }
            } else {
                return true;
            }
        }
        let Some((d, t0, o0)) = self.fling else { return false };
        let t = (now_ms - t0) / 1000.0;
        let (pos, _v) = d.evaluate(t);
        self.offset = o0 + pos;
        let edge = self.offset <= 0.0 || self.offset >= self.max();
        self.moved();
        if d.is_at_rest(t) || edge {
            self.fling = None;
            return false;
        }
        true
    }

    /// Animating (fling or a gesture that may still fling).
    pub fn animating(&self) -> bool {
        self.fling.is_some() || self.last_precise_ms.is_some()
    }

    /// Jump (programmatic).
    pub fn scroll_to(&mut self, offset: f64) {
        self.fling = None;
        self.offset = offset;
        self.moved();
    }

    /// Keep the content rect `[y, y+h)` (content coordinates) visible.
    pub fn ensure_visible(&mut self, y: f64, h: f64) {
        if y < self.offset {
            self.scroll_to(y);
        } else if y + h > self.offset + self.view {
            self.scroll_to(y + h - self.view);
        }
    }

    /// The scrollbar track for a viewport.
    pub fn track(&self, viewport: Rect, st: &ScrollStyle) -> Rect {
        Rect::new(viewport.x + viewport.width - st.bar_w - st.bar_inset, viewport.y + 2.0, st.bar_w, (viewport.height - 4.0).max(0.0))
    }

    /// The thumb inside `track` (`None` when not scrollable).
    pub fn thumb(&self, track: Rect, st: &ScrollStyle) -> Option<Rect> {
        if !self.scrollable() || track.height <= 0.0 {
            return None;
        }
        let len = (track.height * self.view / self.content).clamp(st.min_thumb.min(track.height), track.height);
        let y = track.y + (track.height - len) * (self.offset / self.max().max(1e-9));
        Some(Rect::new(track.x, y, track.width, len))
    }

    /// Start a thumb drag at pointer `y`.
    pub fn begin_drag(&mut self, y: f64) {
        self.fling = None;
        self.drag = Some((y, self.offset));
    }

    /// Thumb drag in progress.
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Pointer at `y` while dragging.
    pub fn drag_to(&mut self, y: f64, track: Rect, st: &ScrollStyle) {
        let Some((y0, o0)) = self.drag else { return };
        let Some(th) = self.thumb(track, st) else { return };
        let room = (track.height - th.height).max(1.0);
        self.offset = o0 + (y - y0) * self.max() / room;
        self.moved();
    }

    /// End the thumb drag.
    pub fn end_drag(&mut self) {
        self.drag = None;
    }

    /// A press on the track outside the thumb pages towards the pointer.
    pub fn page_towards(&mut self, y: f64, track: Rect, st: &ScrollStyle) {
        if let Some(th) = self.thumb(track, st) {
            let page = self.view * 0.9;
            if y < th.y {
                self.scroll_to(self.offset - page);
            } else if y >= th.y + th.height {
                self.scroll_to(self.offset + page);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sv() -> (ScrollView, ScrollStyle) {
        let mut s = ScrollView::default();
        s.set_extent(1000.0, 200.0);
        (s, ScrollStyle::default())
    }

    #[test]
    fn line_wheel_moves_and_clamps() {
        let (mut s, st) = sv();
        assert!(s.wheel(-1.0, false, 0.0, &st));
        assert_eq!(s.offset, 40.0);
        s.wheel(100.0, false, 1.0, &st);
        assert_eq!(s.offset, 0.0);
        s.wheel(-100.0, false, 2.0, &st);
        assert_eq!(s.offset, 800.0);
        assert!(s.follow_end && s.at_bottom());
    }

    #[test]
    fn precise_flick_flings_and_comes_to_rest() {
        let (mut s, mut st) = sv();
        st.fling = true;
        for i in 0..5 {
            s.wheel(-1.0, true, i as f64 * 10.0, &st); // 20 px / 10 ms = 2000 px/s
        }
        let at_release = s.offset;
        assert_eq!(at_release, 100.0);
        assert!(s.tick(60.0, &st), "gesture still open");
        assert!(s.tick(120.0, &st), "fling started");
        let mut t = 120.0;
        while s.tick(t, &st) {
            t += 16.0;
            assert!(t < 10_000.0);
        }
        assert!(s.offset > at_release + 50.0, "inertia carried it on: {}", s.offset);
        assert!(s.offset <= s.max());
    }

    #[test]
    fn wheel_notches_and_no_fling_style_follow_deltas_exactly() {
        let (mut s, mut st) = sv();
        st.fling = true;
        // 100 px notches (browser wheel, pixel mode) never fling.
        s.wheel(-5.0, true, 0.0, &st);
        s.wheel(-5.0, true, 16.0, &st);
        assert_eq!(s.offset, 200.0);
        assert!(!s.animating());
        assert!(!s.tick(200.0, &st));
        assert_eq!(s.offset, 200.0);
        // fling off: touchpad samples move 1:1 and stop with the hand.
        st.fling = false;
        for i in 0..5 {
            s.wheel(-1.0, true, 300.0 + i as f64 * 10.0, &st);
        }
        assert_eq!(s.offset, 300.0);
        assert!(!s.tick(500.0, &st));
        assert_eq!(s.offset, 300.0);
    }

    #[test]
    fn slow_precise_scroll_does_not_fling() {
        let (mut s, st) = sv();
        s.wheel(-0.05, true, 0.0, &st);
        s.wheel(-0.05, true, 90.0, &st);
        let o = s.offset;
        s.tick(200.0, &st);
        assert!(!s.tick(300.0, &st));
        assert_eq!(s.offset, o);
    }

    #[test]
    fn thumb_drag_maps_track_to_content() {
        let (mut s, st) = sv();
        let tr = Rect::new(0.0, 0.0, 6.0, 200.0);
        let th = s.thumb(tr, &st).unwrap();
        assert_eq!(th.height, 40.0);
        s.begin_drag(10.0);
        s.drag_to(10.0 + 80.0, tr, &st); // half of the 160 px room
        assert!((s.offset - 400.0).abs() < 0.01);
        s.drag_to(1000.0, tr, &st);
        assert_eq!(s.offset, 800.0);
        s.end_drag();
        s.page_towards(1.0, tr, &st);
        assert!((s.offset - 620.0).abs() < 0.01);
    }

    #[test]
    fn follow_end_sticks_while_content_grows_until_scrolled_away() {
        let mut s = ScrollView::at_end();
        let st = ScrollStyle::default();
        s.set_extent(500.0, 200.0);
        assert_eq!(s.offset, 300.0);
        s.set_extent(700.0, 200.0);
        assert_eq!(s.offset, 500.0);
        s.wheel(1.0, false, 0.0, &st);
        assert!(!s.follow_end);
        s.set_extent(900.0, 200.0);
        assert_eq!(s.offset, 460.0);
    }
}
