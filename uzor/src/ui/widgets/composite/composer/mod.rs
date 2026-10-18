//! Message composer: rounded box with a text area, `+` attach on the left,
//! mic and a round send button on the right. Draw returns the hit rects.
//! Metrics and colours follow the HQ web composer (`#box.one`).

use crate::input::core::coordinator::LayerId;
use crate::input::{InputCoordinator, Sense, WidgetKind};
use crate::render::{RenderContext, TextAlign};
use crate::ui::css_text::{fill_text_css, INTER};
use crate::types::{Rect, WidgetId};
use crate::ui::widgets::atomic::session_pill::path_round_rect;

/// Plain view data.
#[derive(Clone, Debug)]
pub struct ComposerView<'a> {
    /// Current text.
    pub text: &'a str,
    /// Placeholder when empty.
    pub placeholder: &'a str,
    /// Hovered part, if any.
    pub hovered: Option<ComposerPart>,
    /// Caret (char index) when the text line has focus.
    pub cursor: Option<usize>,
}

/// Clickable parts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComposerPart {
    /// `+` attach.
    Attach,
    /// Voice input.
    Mic,
    /// Send.
    Send,
    /// The text area.
    Text,
}

/// Rects of the parts after layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComposerHits {
    /// Whole box.
    pub frame: Rect,
    /// Text line.
    pub text: Rect,
    /// `+`.
    pub attach: Rect,
    /// Mic.
    pub mic: Rect,
    /// Send.
    pub send: Rect,
}

/// Look of the composer. [`ComposerStyle::default`] is the HQ web composer
/// (`#box.one` in hatchery-hq-web style.css): 44px pill, 7px padding, 28px
/// round buttons, 8px gaps, Inter 14.5px.
#[derive(Clone, Debug)]
pub struct ComposerStyle {
    /// Box height (one-line mode).
    pub height: f64,
    /// Max box width.
    pub max_width: f64,
    /// Min side margin.
    pub side: f64,
    /// Space under the box.
    pub bottom: f64,
    /// Corner radius.
    pub radius: f64,
    /// Inner padding (inside the 1px border).
    pub pad: f64,
    /// Gap between parts.
    pub gap: f64,
    /// Button diameter.
    pub btn: f64,
    /// Box fill / border / focused border.
    pub bg: &'static str,
    /// Border.
    pub border: &'static str,
    /// Border while the text has focus.
    pub border_focus: &'static str,
    /// Text font.
    pub font: &'static str,
    /// Text size in px (matches `font`).
    pub font_px: f64,
    /// Text colour.
    pub text: &'static str,
    /// Placeholder colour.
    pub placeholder: &'static str,
    /// Caret colour.
    pub caret: &'static str,
    /// Round grey button fill / hover / glyph.
    pub btn_bg: &'static str,
    /// Grey button hover.
    pub btn_hover: &'static str,
    /// Grey button glyph.
    pub btn_fg: &'static str,
    /// Send fill / hover.
    pub send_bg: &'static str,
    /// Send hover.
    pub send_hover: &'static str,
}

impl Default for ComposerStyle {
    fn default() -> Self {
        Self {
            height: 44.0,
            max_width: 780.0,
            side: 16.0,
            bottom: 16.0,
            radius: 22.0,
            pad: 7.0,
            gap: 8.0,
            btn: 28.0,
            bg: "#1f1f1f",
            border: "#353535",
            border_focus: "#404040",
            font: "14.5px Inter, sans-serif",
            font_px: 14.5,
            text: "#f5f5f5",
            placeholder: "#5e5e66",
            caret: "#f5f5f5",
            btn_bg: "#282828",
            btn_hover: "#323232",
            btn_fg: "#f5f5f5",
            send_bg: "#116dd2",
            send_hover: "#1884fc",
        }
    }
}

/// Height of the default composer box.
pub const COMPOSER_HEIGHT: f64 = 44.0;

/// Lay out a composer at the bottom of `area` (HQ: `#composer` padding
/// `6px max(16px, (100% - 780px)/2) 16px`).
pub fn composer_layout_styled(area: Rect, st: &ComposerStyle) -> ComposerHits {
    let side = st.side.max((area.width - st.max_width) / 2.0);
    let w = (area.width - 2.0 * side).max(120.0);
    let frame = Rect::new(area.x + side, area.y + area.height - st.bottom - st.height, w, st.height);
    let inner = 1.0 + st.pad;
    let by = frame.y + inner;
    let attach = Rect::new(frame.x + inner, by, st.btn, st.btn);
    let send = Rect::new(frame.x + frame.width - inner - st.btn, by, st.btn, st.btn);
    let mic = Rect::new(send.x - st.gap - st.btn, by, st.btn, st.btn);
    let tx = attach.x + st.btn + st.gap;
    ComposerHits { frame, text: Rect::new(tx, frame.y + 1.0, (mic.x - st.gap - tx).max(10.0), st.height - 2.0), attach, mic, send }
}

/// [`composer_layout_styled`] with the default style.
pub fn composer_layout(area: Rect) -> ComposerHits {
    composer_layout_styled(area, &ComposerStyle::default())
}

fn round_btn(ctx: &mut dyn RenderContext, r: Rect, fill: &str) {
    ctx.begin_path();
    ctx.arc(r.x + r.width / 2.0, r.y + r.height / 2.0, r.width / 2.0, 0.0, std::f64::consts::TAU);
    ctx.set_fill_color(fill);
    ctx.fill();
}

/// Maps a 24-unit SVG viewBox onto a `size` square centred in `r`.
struct Svg {
    x: f64,
    y: f64,
    k: f64,
}

impl Svg {
    fn new(r: Rect, size: f64) -> Self {
        // Browsers snap an inline <svg> box to whole pixels.
        Self { x: (r.x + (r.width - size) / 2.0).round(), y: (r.y + (r.height - size) / 2.0).round(), k: size / 24.0 }
    }
    fn m(&self, ctx: &mut dyn RenderContext, x: f64, y: f64) {
        ctx.move_to(self.x + x * self.k, self.y + y * self.k);
    }
    fn l(&self, ctx: &mut dyn RenderContext, x: f64, y: f64) {
        ctx.line_to(self.x + x * self.k, self.y + y * self.k);
    }
}

/// Paint the composer. Returns the caret x of every char boundary of
/// `view.text` (feed it to the text-field store for hit-testing).
pub fn draw_composer_styled(ctx: &mut dyn RenderContext, hits: &ComposerHits, view: &ComposerView<'_>, st: &ComposerStyle) -> Vec<f64> {
    let f = hits.frame;
    // Border as an outer fill + inner fill (crisp 1px on every backend).
    path_round_rect(ctx, f, st.radius);
    ctx.set_fill_color(if view.cursor.is_some() { st.border_focus } else { st.border });
    ctx.fill();
    path_round_rect(ctx, Rect::new(f.x + 1.0, f.y + 1.0, f.width - 2.0, f.height - 2.0), st.radius - 1.0);
    ctx.set_fill_color(st.bg);
    ctx.fill();

    ctx.set_font(st.font);
    let cy = f.y + f.height / 2.0;
    let (t, c) = if view.text.is_empty() { (view.placeholder, st.placeholder) } else { (view.text, st.text) };
    ctx.set_fill_color(c);
    // textarea line box: 3px padding, line-height 1.45, centred in the box.
    let lh = st.font_px * 1.45;
    fill_text_css(ctx, t, hits.text.x, cy - lh / 2.0, st.font_px, lh, INTER, TextAlign::Left);
    let mut positions = vec![hits.text.x];
    let mut acc = String::new();
    for ch in view.text.chars() {
        acc.push(ch);
        positions.push(hits.text.x + ctx.measure_text(&acc));
    }
    if let Some(c) = view.cursor {
        let x = positions.get(c).copied().unwrap_or(*positions.last().unwrap_or(&hits.text.x));
        ctx.set_fill_color(st.caret);
        // Pixel-snapped caret (thin sub-pixel fills trip tiny-skia 0.11).
        ctx.fill_rect(x.round(), (cy - 9.0).round(), 1.0, 18.0);
    }
    let hov = |p| view.hovered == Some(p);
    round_btn(ctx, hits.attach, if hov(ComposerPart::Attach) { st.btn_hover } else { st.btn_bg });
    round_btn(ctx, hits.mic, if hov(ComposerPart::Mic) { st.btn_hover } else { st.btn_bg });
    round_btn(ctx, hits.send, if hov(ComposerPart::Send) { st.send_hover } else { st.send_bg });
    ctx.set_line_cap("round");
    ctx.set_line_join("round");
    // + : <path d="M12 5v14M5 12h14"/> 18px, stroke 2
    let g = Svg::new(hits.attach, 18.0);
    ctx.set_stroke_color(st.btn_fg);
    ctx.set_stroke_width(2.0 * g.k);
    ctx.begin_path();
    g.m(ctx, 12.0, 5.0); g.l(ctx, 12.0, 19.0);
    g.m(ctx, 5.0, 12.0); g.l(ctx, 19.0, 12.0);
    ctx.stroke();
    // mic: <rect x=9 y=3 w=6 h=11 rx=3/><path d="M5 11a7 7 0 0 0 14 0M12 18v3"/> 17px
    let g = Svg::new(hits.mic, 17.0);
    ctx.set_stroke_width(2.0 * g.k);
    path_round_rect(ctx, Rect::new(g.x + 9.0 * g.k, g.y + 3.0 * g.k, 6.0 * g.k, 11.0 * g.k), 3.0 * g.k);
    ctx.stroke();
    ctx.begin_path();
    ctx.arc(g.x + 12.0 * g.k, g.y + 11.0 * g.k, 7.0 * g.k, 0.0, std::f64::consts::PI);
    ctx.stroke();
    ctx.begin_path();
    g.m(ctx, 12.0, 18.0); g.l(ctx, 12.0, 21.0);
    ctx.stroke();
    // send: <path d="M12 19V5M6 11l6-6 6 6"/> 17px, stroke 2.4, white
    let g = Svg::new(hits.send, 17.0);
    ctx.set_stroke_color("#ffffff");
    ctx.set_stroke_width(2.4 * g.k);
    ctx.begin_path();
    g.m(ctx, 12.0, 19.0); g.l(ctx, 12.0, 5.0);
    g.m(ctx, 6.0, 11.0); g.l(ctx, 12.0, 5.0); g.l(ctx, 18.0, 11.0);
    ctx.stroke();
    ctx.set_line_cap("butt");
    ctx.set_line_join("miter");
    positions
}

/// [`draw_composer_styled`] with the default style.
pub fn draw_composer(ctx: &mut dyn RenderContext, hits: &ComposerHits, view: &ComposerView<'_>) -> Vec<f64> {
    draw_composer_styled(ctx, hits, view, &ComposerStyle::default())
}

/// Declare the composer's click rects under `id_prefix` + `-attach|-mic|-send|-text`.
pub fn register_composer(coord: &mut InputCoordinator, id_prefix: &str, hits: &ComposerHits, layer: &LayerId) {
    for (s, r) in [("attach", hits.attach), ("mic", hits.mic), ("send", hits.send), ("text", hits.text)] {
        coord.register_atomic(WidgetId::from(format!("{id_prefix}-{s}")), WidgetKind::Button, r, Sense::CLICK, layer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layout_bottom_centred() {
        // HQ web at 1100x680: main column x=259 w=841 -> #box 289.5,620 780x44.
        let h = composer_layout(Rect::new(259.0, 0.0, 841.0, 680.0));
        assert_eq!((h.frame.x, h.frame.y, h.frame.width, h.frame.height), (289.5, 620.0, 780.0, 44.0));
        assert_eq!((h.attach.x, h.attach.y), (297.5, 628.0));
        assert_eq!(h.mic.x, 997.5);
        assert_eq!(h.send.x, 1033.5);
        assert_eq!(h.text.x, 333.5);
    }
}
