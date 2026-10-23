//! CSS box model for one-line labelled controls (buttons, tabs, pills).
//!
//! The DOM sizes a `<button>` from its content: the measured label width
//! plus padding and border (`width:auto`), clamped by `min-width` /
//! `max-width`; its height is the line box (`line-height`) plus padding and
//! border. A label that does not fit the clamped content box is ellipsized
//! (`text-overflow:ellipsis`) and clipped (`overflow:hidden`), so text never
//! falls out of its control. Paint code that hardcoded widths and offsets
//! per screen let long or wide labels run past the box; [`TextBox`] gives
//! every control the same rule.
//!
//! [`segmented`] builds the pill tab strip (`.tabs{display:flex;gap;padding;
//! border-radius:999px}` with `button.on` filled) on top of it.

use crate::render::{RenderContext, TextAlign};
use crate::types::Rect;
use crate::ui::css_text::{fill_text_css, FontMetrics, INTER};
use crate::ui::widgets::composite::dialog::fill_round_rect;

/// Box edges (CSS order: top, right, bottom, left).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Edges {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Edges {
    /// `padding: v h`.
    pub const fn vh(v: f64, h: f64) -> Self {
        Self { top: v, right: h, bottom: v, left: h }
    }
    /// The same on every side.
    pub const fn all(a: f64) -> Self {
        Self { top: a, right: a, bottom: a, left: a }
    }
    pub fn horizontal(&self) -> f64 {
        self.left + self.right
    }
    pub fn vertical(&self) -> f64 {
        self.top + self.bottom
    }
}

/// A one-line text control's box: font, line box, padding, border, width clamp.
#[derive(Clone, Debug, PartialEq)]
pub struct TextBox {
    /// Font family list (`Inter, sans-serif`).
    pub family: &'static str,
    pub weight: u32,
    pub px: f64,
    /// CSS `line-height` in px (e.g. `1.5 * px` for an inherited `1.5`).
    pub line_height: f64,
    pub metrics: FontMetrics,
    pub padding: Edges,
    /// Border width on every side.
    pub border: f64,
    pub min_width: f64,
    pub max_width: Option<f64>,
}

impl TextBox {
    /// Inter at `px` / `weight` with a `1.5` line height and no padding.
    pub fn inter(weight: u32, px: f64) -> Self {
        Self {
            family: "Inter, sans-serif",
            weight,
            px,
            line_height: (px * 1.5 * 100.0).round() / 100.0,
            metrics: INTER,
            padding: Edges::default(),
            border: 0.0,
            min_width: 0.0,
            max_width: None,
        }
    }
    pub fn padding(mut self, p: Edges) -> Self {
        self.padding = p;
        self
    }
    pub fn line_height(mut self, lh: f64) -> Self {
        self.line_height = lh;
        self
    }
    pub fn border(mut self, b: f64) -> Self {
        self.border = b;
        self
    }
    pub fn min_width(mut self, w: f64) -> Self {
        self.min_width = w;
        self
    }
    pub fn max_width(mut self, w: f64) -> Self {
        self.max_width = Some(w);
        self
    }

    /// The canvas font string.
    pub fn font(&self) -> String {
        format!("{} {}px {}", self.weight, self.px, self.family)
    }

    fn chrome_w(&self) -> f64 {
        self.padding.horizontal() + 2.0 * self.border
    }

    /// Border-box height: line box + padding + border.
    pub fn height(&self) -> f64 {
        self.line_height + self.padding.vertical() + 2.0 * self.border
    }

    /// Border-box width for `label` (`width:auto`), clamped by min / max.
    pub fn width(&self, ctx: &mut dyn RenderContext, label: &str) -> f64 {
        ctx.set_font(&self.font());
        let w = ctx.measure_text(label) + self.chrome_w();
        let w = w.max(self.min_width);
        match self.max_width {
            Some(m) => w.min(m.max(self.min_width)),
            None => w,
        }
    }

    /// Border-box size for `label`.
    pub fn size(&self, ctx: &mut dyn RenderContext, label: &str) -> (f64, f64) {
        (self.width(ctx, label), self.height())
    }

    /// The content box of a border box `r`.
    pub fn content(&self, r: Rect) -> Rect {
        let l = self.padding.left + self.border;
        let t = self.padding.top + self.border;
        Rect::new(r.x + l, r.y + t, (r.width - self.chrome_w()).max(0.0), (r.height - self.padding.vertical() - 2.0 * self.border).max(0.0))
    }

    /// The text actually drawn in a content box `max` wide: `label`, or its
    /// longest prefix + `…` that fits.
    pub fn fit(&self, ctx: &mut dyn RenderContext, label: &str, max: f64) -> String {
        ctx.set_font(&self.font());
        ellipsize(ctx, label, max)
    }

    /// Paint `label` inside the border box `r`: vertically centred line box
    /// (flex `align-items:center`), ellipsized to the content width and
    /// clipped to the content box. Returns the drawn string.
    pub fn paint_label(&self, ctx: &mut dyn RenderContext, label: &str, r: Rect, color: &str, align: TextAlign) -> String {
        let c = self.content(r);
        let s = self.fit(ctx, label, c.width + 0.5);
        let x = match align {
            TextAlign::Center => c.x + c.width / 2.0,
            TextAlign::Right => c.x + c.width,
            _ => c.x,
        };
        let top = c.y + (c.height - self.line_height) / 2.0;
        ctx.save();
        // Clip horizontally to the content box (a glyph's overhang is
        // `overflow:hidden`'s too); vertically to the border box, as the DOM
        // lets descenders sit in the padding.
        ctx.clip_rect(c.x - 0.5, r.y, c.width + 1.0, r.height);
        ctx.set_fill_color(color);
        fill_text_css(ctx, &s, x, top, self.px, self.line_height, self.metrics, align);
        ctx.restore();
        s
    }
}

/// One-line ellipsis with the current font (`text-overflow:ellipsis`).
/// Whitespace runs collapse to one space first, like `white-space:nowrap`.
pub fn ellipsize(ctx: &mut dyn RenderContext, s: &str, max: f64) -> String {
    let flat: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if ctx.measure_text(&flat) <= max {
        return flat;
    }
    let chars: Vec<char> = flat.chars().collect();
    let fits = |ctx: &mut dyn RenderContext, n: usize| ctx.measure_text(&format!("{}…", chars[..n].iter().collect::<String>())) <= max;
    let (mut lo, mut hi) = (0usize, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if fits(ctx, mid) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    if lo == 0 && !fits(ctx, 0) {
        return String::new();
    }
    format!("{}…", chars[..lo].iter().collect::<String>().trim_end())
}

/// Pill tab strip (segmented control).
pub mod segmented {
    use super::*;

    /// Style: the track and its buttons.
    #[derive(Clone, Debug, PartialEq)]
    pub struct Style {
        pub button: TextBox,
        pub gap: f64,
        pub track_padding: f64,
        pub radius: f64,
        pub track_bg: &'static str,
        pub on_bg: &'static str,
        pub hover_bg: Option<&'static str>,
        pub fg: &'static str,
        pub fg_on: &'static str,
    }

    impl Style {
        /// `.tabs{gap:4px;background:el;border-radius:999px;padding:3px}`,
        /// `button{font-size:13px;padding:5px 14px}` with an inherited 1.5
        /// line height, `.on{background:el2;color:txt}`.
        pub fn pill(track_bg: &'static str, on_bg: &'static str, fg: &'static str, fg_on: &'static str) -> Self {
            Self {
                button: TextBox::inter(400, 13.0).padding(Edges::vh(5.0, 14.0)),
                gap: 4.0,
                track_padding: 3.0,
                radius: 999.0,
                track_bg,
                on_bg,
                hover_bg: None,
                fg,
                fg_on,
            }
        }
    }

    /// Border boxes of the track and of each button, laid out from `(x, y)`.
    pub fn layout(ctx: &mut dyn RenderContext, labels: &[&str], x: f64, y: f64, st: &Style) -> (Rect, Vec<Rect>) {
        let h = st.button.height();
        let mut bx = x + st.track_padding;
        let mut out = Vec::with_capacity(labels.len());
        for (i, l) in labels.iter().enumerate() {
            if i > 0 {
                bx += st.gap;
            }
            let w = st.button.width(ctx, l);
            out.push(Rect::new(bx, y + st.track_padding, w, h));
            bx += w;
        }
        let track = Rect::new(x, y, bx + st.track_padding - x, h + 2.0 * st.track_padding);
        (track, out)
    }

    /// Paint the strip; `on` is the selected index, `hover` the hovered one.
    pub fn paint(ctx: &mut dyn RenderContext, labels: &[&str], x: f64, y: f64, st: &Style, on: Option<usize>, hover: Option<usize>) -> (Rect, Vec<Rect>) {
        let (track, rs) = layout(ctx, labels, x, y, st);
        let rad = |r: Rect| st.radius.min(r.height / 2.0);
        fill_round_rect(ctx, track, rad(track), st.track_bg);
        for (i, (l, r)) in labels.iter().zip(&rs).enumerate() {
            let sel = on == Some(i);
            if sel {
                fill_round_rect(ctx, *r, rad(*r), st.on_bg);
            } else if let (Some(bg), true) = (st.hover_bg, hover == Some(i)) {
                fill_round_rect(ctx, *r, rad(*r), bg);
            }
            st.button.paint_label(ctx, l, *r, if sel || hover == Some(i) { st.fg_on } else { st.fg }, TextAlign::Center);
        }
        (track, rs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{DrawOp, RecordingRenderContext};

    /// A label sizes its button: content width = measured text, plus padding.
    #[test]
    fn text_fits_its_button() {
        let mut ctx = RecordingRenderContext::with_char_width(7.0);
        let b = TextBox::inter(400, 13.0).padding(Edges::vh(5.0, 14.0));
        let (w, h) = b.size(&mut ctx, "Limits");
        assert_eq!(w, 6.0 * 7.0 + 28.0);
        assert_eq!(h, 19.5 + 10.0);
        let r = Rect::new(10.0, 20.0, w, h);
        let drawn = b.paint_label(&mut ctx, "Limits", r, "#fff", TextAlign::Center);
        assert_eq!(drawn, "Limits", "an intrinsic box never truncates its own label");
        let (_, x, y) = ctx.fill_texts()[0];
        assert_eq!(x, r.x + r.width / 2.0, "centred in the content box");
        // Baseline of a 13/19.5 line box starting 5px under the top.
        let base = 20.0 + 5.0 + crate::ui::css_text::baseline(13.0, 19.5, INTER);
        assert!((y - base).abs() < 1e-9, "{y} vs {base}");
    }

    /// A clamped box ellipsizes and clips; the drawn text never exceeds the content box.
    #[test]
    fn long_label_is_ellipsized_and_clipped() {
        let mut ctx = RecordingRenderContext::with_char_width(7.0);
        let b = TextBox::inter(400, 13.0).padding(Edges::vh(5.0, 10.0)).max_width(100.0);
        let label = "Tasks and resources and more";
        let w = b.width(&mut ctx, label);
        assert_eq!(w, 100.0);
        let r = Rect::new(0.0, 0.0, w, b.height());
        let drawn = b.paint_label(&mut ctx, label, r, "#fff", TextAlign::Left);
        assert!(drawn.ends_with('…'), "{drawn}");
        assert!(drawn.chars().count() as f64 * 7.0 <= b.content(r).width + 0.5, "{drawn}");
        assert!(ctx.ops.iter().any(|o| matches!(o, DrawOp::Clip | DrawOp::Rect { .. })), "label is clipped");
        assert_eq!(ctx.save_depth(), 0);
    }

    /// min-width wins over a short label; the label is centred, not stretched.
    #[test]
    fn min_width_and_border() {
        let mut ctx = RecordingRenderContext::with_char_width(7.0);
        let b = TextBox::inter(500, 12.0).padding(Edges::vh(2.0, 8.0)).border(1.0).min_width(60.0);
        assert_eq!(b.width(&mut ctx, "ok"), 60.0);
        assert_eq!(b.height(), 18.0 + 4.0 + 2.0);
        let c = b.content(Rect::new(0.0, 0.0, 60.0, 24.0));
        assert_eq!((c.x, c.y, c.width, c.height), (9.0, 3.0, 42.0, 18.0));
    }

    /// The pill strip matches the DOM metrics of `.tq-tabs`.
    #[test]
    fn segmented_pill_metrics() {
        let mut ctx = RecordingRenderContext::with_char_width(7.0);
        let st = segmented::Style::pill("#1f1f1f", "#202020", "#acacac", "#f5f5f5");
        let (track, rs) = segmented::layout(&mut ctx, &["Board", "Limits"], 100.0, 50.0, &st);
        assert_eq!(rs[0], Rect::new(103.0, 53.0, 35.0 + 28.0, 29.5));
        assert_eq!(rs[1].x, 103.0 + 63.0 + 4.0);
        assert_eq!(track.height, 35.5);
        assert_eq!(track.width, 3.0 + 63.0 + 4.0 + 70.0 + 3.0);
    }
}
