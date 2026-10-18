//! CSS-faithful dialog primitives and a dialog stack (modal over modal).
//!
//! Each layer gets its own scrim; only the top layer takes input. Values are plain
//! CSS numbers so a host can match a web reference pixel for pixel.

use crate::render::{RenderContext, TextAlign};
use crate::types::Rect;
use crate::ui::css_text::{fill_text_css, INTER};

/// Ordered dialog layers; the last one is on top.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DialogStack<T> {
    layers: Vec<T>,
}

impl<T: PartialEq> DialogStack<T> {
    /// Empty stack.
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }
    /// Open `d` above the current top.
    pub fn push(&mut self, d: T) {
        self.layers.push(d);
    }
    /// Close the top layer (Escape, scrim click, close button).
    pub fn pop(&mut self) -> Option<T> {
        self.layers.pop()
    }
    /// Swap the top layer's content in place (a dialog that re-renders itself).
    pub fn replace_top(&mut self, d: T) {
        match self.layers.last_mut() {
            Some(t) => *t = d,
            None => self.layers.push(d),
        }
    }
    /// Close everything.
    pub fn clear(&mut self) {
        self.layers.clear();
    }
    /// Top layer.
    pub fn top(&self) -> Option<&T> {
        self.layers.last()
    }
    /// Mutable top layer.
    pub fn top_mut(&mut self) -> Option<&mut T> {
        self.layers.last_mut()
    }
    /// Bottom-to-top layers.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.layers.iter()
    }
    /// Number of open layers.
    pub fn len(&self) -> usize {
        self.layers.len()
    }
    /// No dialog open.
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
    /// Whether `d` is open at any depth.
    pub fn contains(&self, d: &T) -> bool {
        self.layers.contains(d)
    }
}

/// CSS `box-shadow: 0 dy blur color`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    /// Vertical offset.
    pub dy: f64,
    /// Blur radius.
    pub blur: f64,
    /// Colour.
    pub color: &'static str,
}

/// A bordered rounded card (`background`, `border:1px`, `border-radius`, `box-shadow`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CardStyle {
    /// Fill.
    pub bg: &'static str,
    /// 1px border colour; `None` = no border.
    pub border: Option<&'static str>,
    /// Corner radius.
    pub radius: f64,
    /// Drop shadow.
    pub shadow: Option<Shadow>,
}

/// Rounded rect path with cubic corners (matches browser corner curves).
pub fn path_round_rect(ctx: &mut dyn RenderContext, r: Rect, rad: f64) {
    let rad = rad.min(r.width / 2.0).min(r.height / 2.0).max(0.0);
    let k = rad * (1.0 - 0.5523);
    let (x0, y0, x1, y1) = (r.x, r.y, r.x + r.width, r.y + r.height);
    ctx.begin_path();
    ctx.move_to(x0 + rad, y0);
    ctx.line_to(x1 - rad, y0);
    ctx.bezier_curve_to(x1 - k, y0, x1, y0 + k, x1, y0 + rad);
    ctx.line_to(x1, y1 - rad);
    ctx.bezier_curve_to(x1, y1 - k, x1 - k, y1, x1 - rad, y1);
    ctx.line_to(x0 + rad, y1);
    ctx.bezier_curve_to(x0 + k, y1, x0, y1 - k, x0, y1 - rad);
    ctx.line_to(x0, y0 + rad);
    ctx.bezier_curve_to(x0, y0 + k, x0 + k, y0, x0 + rad, y0);
    ctx.close_path();
}

/// Fill a rounded rect.
pub fn fill_round_rect(ctx: &mut dyn RenderContext, r: Rect, rad: f64, color: &str) {
    path_round_rect(ctx, r, rad);
    ctx.set_fill_color(color);
    ctx.fill();
}

/// Full-window scrim (`position:fixed; inset:0; background`).
pub fn draw_scrim(ctx: &mut dyn RenderContext, r: Rect, color: &str) {
    ctx.set_fill_color(color);
    ctx.fill_rect(r.x, r.y, r.width, r.height);
}

/// Paint a card: shadow + border as an outer fill, background as the inner fill.
pub fn draw_card(ctx: &mut dyn RenderContext, r: Rect, s: &CardStyle) {
    if let Some(sh) = s.shadow {
        ctx.set_shadow(0.0, sh.dy, sh.blur, sh.color);
    }
    fill_round_rect(ctx, r, s.radius, s.border.unwrap_or(s.bg));
    ctx.clear_shadow();
    if s.border.is_some() {
        fill_round_rect(ctx, Rect::new(r.x + 1.0, r.y + 1.0, r.width - 2.0, r.height - 2.0), (s.radius - 1.0).max(0.0), s.bg);
    }
}

/// Centre a `w×h` box in `r` (`display:grid; place-items:center`).
pub fn center_box(r: Rect, w: f64, h: f64) -> Rect {
    Rect::new(r.x + (r.width - w) / 2.0, r.y + (r.height - h) / 2.0, w, h)
}

/// iOS-style switch (`.sw`): 38×22 track, 16px knob inset 3, slides 16 when on.
pub fn draw_switch(ctx: &mut dyn RenderContext, r: Rect, on: bool, on_color: &str, off_color: &str) {
    fill_round_rect(ctx, r, r.height / 2.0, if on { on_color } else { off_color });
    let d = r.height - 6.0;
    let x = r.x + 3.0 + if on { r.width - 6.0 - d } else { 0.0 };
    ctx.begin_path();
    ctx.arc(x + d / 2.0, r.y + 3.0 + d / 2.0, d / 2.0, 0.0, std::f64::consts::TAU);
    ctx.set_fill_color("#ffffff");
    ctx.fill();
}

/// A text button with CSS box metrics (`padding`, optional 1px border, radius).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextButtonStyle {
    /// Fill.
    pub bg: &'static str,
    /// Hover fill.
    pub bg_hover: &'static str,
    /// 1px border.
    pub border: Option<&'static str>,
    /// Radius.
    pub radius: f64,
    /// Horizontal padding.
    pub pad_x: f64,
    /// Vertical padding.
    pub pad_y: f64,
    /// Font weight.
    pub weight: u32,
    /// Font size.
    pub px: f64,
    /// Line height.
    pub lh: f64,
    /// Text colour.
    pub color: &'static str,
}

impl TextButtonStyle {
    fn font(&self) -> String {
        format!("{} {}px Inter, sans-serif", self.weight, self.px)
    }
    /// Box size for `label`.
    pub fn size(&self, ctx: &mut dyn RenderContext, label: &str) -> (f64, f64) {
        ctx.set_font(&self.font());
        let b = if self.border.is_some() { 2.0 } else { 0.0 };
        (ctx.measure_text(label) + 2.0 * self.pad_x + b, self.lh + 2.0 * self.pad_y + b)
    }
}

/// Paint a text button in `r`.
pub fn draw_text_button(ctx: &mut dyn RenderContext, r: Rect, label: &str, s: &TextButtonStyle, hovered: bool) {
    let bg = if hovered { s.bg_hover } else { s.bg };
    match s.border {
        Some(b) => draw_card(ctx, r, &CardStyle { bg, border: Some(b), radius: s.radius, shadow: None }),
        None => fill_round_rect(ctx, r, s.radius, bg),
    }
    let b = if s.border.is_some() { 1.0 } else { 0.0 };
    ctx.set_font(&s.font());
    ctx.set_fill_color(s.color);
    fill_text_css(ctx, label, r.x + b + s.pad_x, r.y + b + s.pad_y, s.px, s.lh, INTER, TextAlign::Left);
}

/// Single-line field text with placeholder and caret, Inter `px` on a `lh` line
/// vertically centred in `r`. Returns per-char x positions for the text store.
#[allow(clippy::too_many_arguments)]
pub fn draw_field_text(ctx: &mut dyn RenderContext, r: Rect, text: &str, placeholder: &str, caret: Option<usize>, px: f64, color: &str, ph_color: &str) -> Vec<f64> {
    let lh = (px * 1.5).round();
    let top = r.y + (r.height - lh) / 2.0;
    ctx.set_font(&format!("400 {px}px Inter, sans-serif"));
    if text.is_empty() {
        ctx.set_fill_color(ph_color);
        fill_text_css(ctx, placeholder, r.x, top, px, lh, INTER, TextAlign::Left);
    } else {
        ctx.set_fill_color(color);
        fill_text_css(ctx, text, r.x, top, px, lh, INTER, TextAlign::Left);
    }
    let mut pos = vec![r.x];
    let mut acc = String::new();
    for ch in text.chars() {
        acc.push(ch);
        pos.push(r.x + ctx.measure_text(&acc));
    }
    if let Some(c) = caret {
        let x = pos.get(c).copied().unwrap_or(*pos.last().unwrap_or(&r.x)).round();
        ctx.set_fill_color(color);
        ctx.fill_rect(x, (top + (lh - px * 1.2) / 2.0).round(), 1.0, (px * 1.2).round());
    }
    pos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_orders_layers() {
        let mut s = DialogStack::new();
        s.push(1);
        s.push(2);
        assert_eq!(s.top(), Some(&2));
        s.replace_top(3);
        assert_eq!(s.iter().copied().collect::<Vec<_>>(), vec![1, 3]);
        assert_eq!(s.pop(), Some(3));
        assert!(s.contains(&1) && !s.contains(&3));
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn center_box_is_centred() {
        let r = center_box(Rect::new(0.0, 0.0, 1100.0, 680.0), 900.0, 598.4);
        assert_eq!((r.x, r.y.round()), (100.0, 41.0));
    }
}
