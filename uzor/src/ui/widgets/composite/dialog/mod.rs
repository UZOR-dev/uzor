//! CSS-faithful dialog primitives and a dialog stack (modal over modal).
//!
//! Each layer gets its own scrim; only the top layer takes input. Values are plain
//! CSS numbers so a host can match a web reference pixel for pixel.

mod tokens;

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

impl crate::ui::style::StyleField for Option<Shadow> {
    /// `box-shadow: <x> <dy> <blur> <color>` (x is ignored), or `none`.
    fn from_css(v: &str) -> Option<Self> {
        let v = v.trim();
        if v == "none" {
            return Some(None);
        }
        let mut nums = Vec::new();
        let mut rest = v;
        while let Some((head, tail)) = rest.split_once(' ') {
            match head.trim_end_matches("px").parse::<f64>() {
                Ok(n) => {
                    nums.push(n);
                    rest = tail.trim_start();
                }
                Err(_) => break,
            }
        }
        match nums[..] {
            [_, dy, blur] | [_, dy, blur, _] => Some(Some(Shadow { dy, blur, color: crate::ui::style::intern(rest) })),
            _ => None,
        }
    }
}

crate::styled!(CardStyle { bg = "background", border = "border-color", radius = "border-radius", shadow = "box-shadow" });

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
    fill_circle(ctx, x + d / 2.0, r.y + 3.0 + d / 2.0, d / 2.0, tokens::WHITE);
}

/// Fill a circle of radius `rad` centred on `(cx, cy)` (dots, round buttons).
pub fn fill_circle(ctx: &mut dyn RenderContext, cx: f64, cy: f64, rad: f64, color: &str) {
    ctx.begin_path();
    ctx.arc(cx, cy, rad, 0.0, std::f64::consts::TAU);
    ctx.set_fill_color(color);
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

/// Size a text button for `label` at `(x, y)`, paint it, and return its rect (the app
/// registers the hit).
pub fn text_button_at(ctx: &mut dyn RenderContext, x: f64, y: f64, label: &str, s: &TextButtonStyle, hovered: bool) -> Rect {
    let (w, h) = s.size(ctx, label);
    let r = Rect::new(x, y, w, h);
    draw_text_button(ctx, r, label, s, hovered);
    r
}

/// Single-line field text with placeholder and caret, Inter `px` on a `lh` line
/// vertically centred in `r`. Returns per-char x positions for the text store.
#[allow(clippy::too_many_arguments)]
pub fn draw_field_text(ctx: &mut dyn RenderContext, r: Rect, text: &str, placeholder: &str, caret: Option<usize>, px: f64, color: &str, ph_color: &str) -> Vec<f64> {
    draw_field_text_in(ctx, r, text, placeholder, caret, px, color, ph_color, "Inter, sans-serif")
}

/// [`draw_field_text`] in the CSS font-family list `family` (a page whose inputs keep the
/// browser's control font).
#[allow(clippy::too_many_arguments)]
pub fn draw_field_text_in(ctx: &mut dyn RenderContext, r: Rect, text: &str, placeholder: &str, caret: Option<usize>, px: f64, color: &str, ph_color: &str, family: &str) -> Vec<f64> {
    draw_field_text_ime(ctx, r, text, placeholder, caret, None, px, color, ph_color, family)
}

/// [`draw_field_text_in`] with an IME composition: `preedit` is the char
/// range of `text` (see `TextFieldState::display`) underlined like the
/// browser's composition text.
#[allow(clippy::too_many_arguments)]
pub fn draw_field_text_ime(ctx: &mut dyn RenderContext, r: Rect, text: &str, placeholder: &str, caret: Option<usize>, preedit: Option<(usize, usize)>, px: f64, color: &str, ph_color: &str, family: &str) -> Vec<f64> {
    let lh = (px * 1.5).round();
    let top = r.y + (r.height - lh) / 2.0;
    ctx.set_font(&format!("400 {px}px {family}"));
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
    if let Some((Some(&x0), Some(&x1))) = preedit.map(|(a, b)| (pos.get(a), pos.get(b))) {
        ctx.set_fill_color(color);
        ctx.fill_rect(x0.round(), (top + (lh + px * 1.2) / 2.0).round() - 1.0, (x1 - x0).round(), 1.0);
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

/// The `✕` close glyph as two strokes centred on `(cx, cy)` (text fonts often
/// lack U+2715; this matches its 14px rendering: ~8px arms, 1.4px stroke).
pub fn draw_close_glyph(ctx: &mut dyn RenderContext, cx: f64, cy: f64, size: f64, color: &str) {
    let h = size * 0.29;
    ctx.set_stroke_color(color);
    ctx.set_stroke_width(size * 0.1);
    ctx.set_line_cap("round");
    ctx.begin_path();
    ctx.move_to(cx - h, cy - h);
    ctx.line_to(cx + h, cy + h);
    ctx.move_to(cx + h, cy - h);
    ctx.line_to(cx - h, cy + h);
    ctx.stroke();
    ctx.set_line_cap("butt");
}

/// Backdrop of a close button.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CloseBg<'a> {
    /// Glyph only (a hover-less `.round` or an icon button at rest).
    None,
    /// A circle filling the button (`border-radius: 50%`).
    Circle(&'a str),
    /// A rounded rect with this radius.
    Round(&'a str, f64),
}

/// A close button in `r`: its backdrop, then the `✕` glyph (`size` px) centred in it.
pub fn draw_close_button(ctx: &mut dyn RenderContext, r: Rect, bg: CloseBg<'_>, size: f64, color: &str) {
    let (cx, cy) = (r.x + r.width / 2.0, r.y + r.height / 2.0);
    match bg {
        CloseBg::None => {}
        CloseBg::Circle(c) => fill_circle(ctx, cx, cy, r.width / 2.0, c),
        CloseBg::Round(c, rad) => fill_round_rect(ctx, r, rad, c),
    }
    draw_close_glyph(ctx, cx, cy, size, color);
}
