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
    composer_layout_lines_styled(area, 1, st)
}

/// Most text lines the composer grows to (more scroll inside the field).
pub const COMPOSER_MAX_LINES: usize = 8;

/// Text lines `text` needs (newlines + 1), clamped to [`COMPOSER_MAX_LINES`].
pub fn composer_lines(text: &str) -> usize {
    (text.matches('\n').count() + 1).min(COMPOSER_MAX_LINES)
}

/// A composer grown upward for `lines` text lines (auto-grow, textarea
/// style); the buttons stay on the bottom row.
pub fn composer_layout_lines_styled(area: Rect, lines: usize, st: &ComposerStyle) -> ComposerHits {
    let lh = st.font_px * 1.45;
    let h = st.height + (lines.clamp(1, COMPOSER_MAX_LINES) - 1) as f64 * lh;
    let side = st.side.max((area.width - st.max_width) / 2.0);
    let w = (area.width - 2.0 * side).max(120.0);
    let frame = Rect::new(area.x + side, area.y + area.height - st.bottom - h, w, h);
    let inner = 1.0 + st.pad;
    let by = frame.y + h - inner - st.btn;
    let attach = Rect::new(frame.x + inner, by, st.btn, st.btn);
    let send = Rect::new(frame.x + frame.width - inner - st.btn, by, st.btn, st.btn);
    let mic = Rect::new(send.x - st.gap - st.btn, by, st.btn, st.btn);
    let tx = attach.x + st.btn + st.gap;
    ComposerHits { frame, text: Rect::new(tx, frame.y + 1.0, (mic.x - st.gap - tx).max(10.0), h - 2.0), attach, mic, send }
}

/// [`composer_layout_lines_styled`] with the default style.
pub fn composer_layout_lines(area: Rect, lines: usize) -> ComposerHits {
    composer_layout_lines_styled(area, lines, &ComposerStyle::default())
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

    let top = f.y + (st.height - st.font_px * 1.45) / 2.0;
    let positions = draw_text_lines(ctx, hits.text.x, top, view, st, (f64::NEG_INFINITY, f64::INFINITY));
    draw_buttons(ctx, hits, view, false, st);
    positions
}

/// The text (or placeholder) as stacked lines from `top`, and the caret.
/// Returns the caret x of every char boundary (x restarts after `\n`).
/// `band`: visible y range; lines not fitting it are skipped (glyph runs
/// ignore clips on some backends).
fn draw_text_lines(ctx: &mut dyn RenderContext, x0: f64, top: f64, view: &ComposerView<'_>, st: &ComposerStyle, band: (f64, f64)) -> Vec<f64> {
    ctx.set_font(st.font);
    let (t, c) = if view.text.is_empty() { (view.placeholder, st.placeholder) } else { (view.text, st.text) };
    ctx.set_fill_color(c);
    let lh = st.font_px * 1.45;
    let fits = |y: f64| y >= band.0 - lh * 0.25 && y + lh <= band.1 + lh * 0.25;
    for (i, l) in t.split('\n').enumerate() {
        let y = top + i as f64 * lh;
        if fits(y) {
            fill_text_css(ctx, l, x0, y, st.font_px, lh, INTER, TextAlign::Left);
        }
    }
    let mut positions = vec![x0];
    let mut line_of = vec![0usize];
    let mut acc = String::new();
    let mut line = 0;
    for ch in view.text.chars() {
        if ch == '\n' {
            acc.clear();
            line += 1;
            positions.push(x0);
        } else {
            acc.push(ch);
            positions.push(x0 + ctx.measure_text(&acc));
        }
        line_of.push(line);
    }
    if let Some(c) = view.cursor {
        let i = c.min(positions.len() - 1);
        let cy = top + line_of[i] as f64 * lh + lh / 2.0;
        if !fits(cy - lh / 2.0) {
            return positions;
        }
        ctx.set_fill_color(st.caret);
        // Pixel-snapped caret (thin sub-pixel fills trip tiny-skia 0.11).
        ctx.fill_rect(positions[i].round(), (cy - 9.0).round(), 1.0, 18.0);
    }
    positions
}

/// `+`, mic and send.
fn draw_buttons(ctx: &mut dyn RenderContext, hits: &ComposerHits, view: &ComposerView<'_>, voice: bool, st: &ComposerStyle) {
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
    if voice {
        // Voice chat (empty composer): purple disc, waveform bars.
        round_btn(ctx, hits.send, if hov(ComposerPart::Send) { VOICE_HOVER } else { VOICE_BG });
        let g = Svg::new(hits.send, 17.0);
        ctx.set_stroke_color("#ffffff");
        ctx.set_stroke_width(2.2 * g.k);
        ctx.begin_path();
        for (x, h) in [(5.0, 4.0), (9.0, 10.0), (12.0, 14.0), (15.0, 8.0), (19.0, 4.0)] {
            g.m(ctx, x, 12.0 - h / 2.0);
            g.l(ctx, x, 12.0 + h / 2.0);
        }
        ctx.stroke();
    } else {
        // send: <path d="M12 19V5M6 11l6-6 6 6"/> 17px, stroke 2.4, white
        let g = Svg::new(hits.send, 17.0);
        ctx.set_stroke_color("#ffffff");
        ctx.set_stroke_width(2.4 * g.k);
        ctx.begin_path();
        g.m(ctx, 12.0, 19.0); g.l(ctx, 12.0, 5.0);
        g.m(ctx, 6.0, 11.0); g.l(ctx, 12.0, 5.0); g.l(ctx, 18.0, 11.0);
        ctx.stroke();
    }
    ctx.set_line_cap("butt");
    ctx.set_line_join("miter");
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

#[cfg(test)]
mod grow_tests {
    use super::*;

    #[test]
    fn composer_grows_upward_with_lines_and_keeps_buttons_at_the_bottom() {
        let area = Rect::new(0.0, 0.0, 800.0, 600.0);
        let one = composer_layout(area);
        let three = composer_layout_lines(area, composer_lines("a\nb\nc"));
        assert_eq!(composer_lines("x"), 1);
        assert_eq!(composer_lines(&"\n".repeat(40)), COMPOSER_MAX_LINES);
        assert!(three.frame.height > one.frame.height);
        assert_eq!(three.frame.y + three.frame.height, one.frame.y + one.frame.height);
        assert_eq!(three.send.y + three.send.height, one.send.y + one.send.height);
    }
}


// ---------------------------------------------------------------------------
// Rich composer: attachments inside the bubble, auto-grow, inner scroll,
// voice-chat button while empty.
// ---------------------------------------------------------------------------

/// Voice-chat button fill (empty composer).
pub const VOICE_BG: &str = "#7c4dff";
/// Voice-chat button hover fill.
pub const VOICE_HOVER: &str = "#9068ff";

/// One attachment shown inside the composer.
#[derive(Clone, Copy, Debug)]
pub enum ComposerAttach<'a> {
    /// An image: a rounded square thumbnail (cover-cropped).
    Image {
        /// RGBA8 pixels.
        rgba: &'a [u8],
        /// Width, px.
        w: u32,
        /// Height, px.
        h: u32,
    },
    /// Any other file: a chip with its name.
    File {
        /// File name.
        name: &'a str,
    },
}

/// View of the rich composer.
#[derive(Clone, Debug)]
pub struct RichComposerView<'a> {
    /// Text, placeholder, hover, caret.
    pub base: ComposerView<'a>,
    /// Attachments, left to right.
    pub attach: &'a [ComposerAttach<'a>],
    /// Attachment under the pointer (shows its ×).
    pub hovered_item: Option<usize>,
    /// The × under the pointer.
    pub hovered_remove: Option<usize>,
    /// Inner scroll offset of the body (attachments + text).
    pub scroll: f64,
}

impl RichComposerView<'_> {
    /// Nothing to send: the right button is voice chat.
    pub fn is_empty(&self) -> bool {
        self.base.text.trim().is_empty() && self.attach.is_empty()
    }
}

/// Layout of the rich composer.
#[derive(Clone, Debug, PartialEq)]
pub struct RichComposerLayout {
    /// Frame, buttons and the text rect (`text` spans the body).
    pub hits: ComposerHits,
    /// Attachment rects (content space already offset by the scroll).
    pub items: Vec<Rect>,
    /// × rects, one per attachment.
    pub remove: Vec<Rect>,
    /// The scrolling body viewport (attachments + text).
    pub body: Rect,
    /// Body content height (scroll extent).
    pub content_h: f64,
    /// Top of the first text line (scroll applied).
    pub text_top: f64,
    /// Tall mode (attachments or several lines): buttons on their own row.
    pub tall: bool,
}

/// Thumbnail edge.
pub const THUMB: f64 = 64.0;
const CHIP_H: f64 = 40.0;

fn chip_w(ctx: &mut dyn RenderContext, name: &str) -> f64 {
    ctx.set_font("13px Inter, sans-serif");
    (ctx.measure_text(name).min(200.0) + 44.0).ceil()
}

/// Lays the rich composer out at the bottom of `area`, at most `max_h` tall
/// (beyond that the body scrolls by `view.scroll`).
pub fn composer_layout_rich(ctx: &mut dyn RenderContext, area: Rect, view: &RichComposerView<'_>, max_h: f64, st: &ComposerStyle) -> RichComposerLayout {
    let lines = view.base.text.matches('\n').count() + 1;
    let lh = st.font_px * 1.45;
    if view.attach.is_empty() && lines == 1 {
        let hits = composer_layout_styled(area, st);
        let top = hits.frame.y + (st.height - lh) / 2.0;
        return RichComposerLayout { hits, items: vec![], remove: vec![], body: hits.text, content_h: hits.text.height, text_top: top, tall: false };
    }
    let side = st.side.max((area.width - st.max_width) / 2.0);
    let w = (area.width - 2.0 * side).max(160.0);
    let pad = 12.0;
    let inner_w = w - 2.0 * pad;
    // Attachments wrap in rows.
    let mut rows_h = 0.0;
    let mut rel: Vec<Rect> = Vec::new();
    let (mut x, mut y, mut row_h) = (0.0, 0.0, 0.0f64);
    for a in view.attach {
        let (iw, ih) = match a {
            ComposerAttach::Image { .. } => (THUMB, THUMB),
            ComposerAttach::File { name } => (chip_w(ctx, name), CHIP_H),
        };
        if x > 0.0 && x + iw > inner_w {
            x = 0.0;
            y += row_h + 8.0;
            row_h = 0.0;
        }
        rel.push(Rect::new(x, y, iw, ih));
        x += iw + 8.0;
        row_h = row_h.max(ih);
    }
    if !rel.is_empty() {
        rows_h = y + row_h + 10.0;
    }
    let text_h = lines as f64 * lh;
    let content_h = rows_h + text_h;
    let btn_row = st.btn + 2.0 * (1.0 + st.pad);
    let natural = pad + content_h + 4.0 + btn_row;
    let h = natural.min(max_h.max(st.height + btn_row));
    let frame = Rect::new(area.x + side, area.y + area.height - st.bottom - h, w, h);
    let body = Rect::new(frame.x + pad, frame.y + pad, inner_w, (h - pad - 4.0 - btn_row).max(lh));
    let off = view.scroll.clamp(0.0, (content_h - body.height).max(0.0));
    let items: Vec<Rect> = rel.iter().map(|r| Rect::new(body.x + r.x, body.y + r.y - off, r.width, r.height)).collect();
    let remove = items.iter().map(|r| Rect::new(r.x + r.width - 22.0, r.y + 4.0, 18.0, 18.0)).collect();
    let inner = 1.0 + st.pad;
    let by = frame.y + h - inner - st.btn;
    let attach = Rect::new(frame.x + inner, by, st.btn, st.btn);
    let send = Rect::new(frame.x + frame.width - inner - st.btn, by, st.btn, st.btn);
    let mic = Rect::new(send.x - st.gap - st.btn, by, st.btn, st.btn);
    let text_top = body.y + rows_h - off;
    // The editable text rect: the text lines, clamped to the visible body.
    let tt = text_top.max(body.y);
    let text = Rect::new(body.x, tt, body.width, (body.y + body.height - tt).max(1.0));
    let hits = ComposerHits { frame, text, attach, mic, send };
    RichComposerLayout { hits, items, remove, body, content_h, text_top, tall: true }
}

/// Paints the rich composer; returns the caret x per char boundary.
pub fn draw_composer_rich(ctx: &mut dyn RenderContext, l: &RichComposerLayout, view: &RichComposerView<'_>, st: &ComposerStyle) -> Vec<f64> {
    let f = l.hits.frame;
    let r = if l.tall { 18.0 } else { st.radius };
    path_round_rect(ctx, f, r);
    ctx.set_fill_color(if view.base.cursor.is_some() { st.border_focus } else { st.border });
    ctx.fill();
    path_round_rect(ctx, Rect::new(f.x + 1.0, f.y + 1.0, f.width - 2.0, f.height - 2.0), r - 1.0);
    ctx.set_fill_color(st.bg);
    ctx.fill();
    ctx.save();
    ctx.begin_path();
    ctx.rect(l.body.x - 2.0, l.body.y, l.body.width + 4.0, l.body.height);
    ctx.clip();
    // Text first, under the plain body clip (thumbnail clips below may replace it).
    let x0 = if l.tall { l.body.x } else { l.hits.text.x };
    let positions = draw_text_lines(ctx, x0, l.text_top, &view.base, st, (l.body.y, l.body.y + l.body.height));
    for (i, (a, ir)) in view.attach.iter().zip(&l.items).enumerate() {
        match a {
            ComposerAttach::Image { rgba, w, h } => {
                // Clip replaces (does not intersect) on some backends: clip to the
                // visible part of the thumbnail inside the body ourselves.
                let top = ir.y.max(l.body.y);
                let bot = (ir.y + ir.height).min(l.body.y + l.body.height);
                if bot <= top {
                    continue;
                }
                let (iw, ih) = ((*w).max(1) as f64, (*h).max(1) as f64);
                let k = (ir.width / iw).max(ir.height / ih);
                ctx.save();
                ctx.begin_path();
                ctx.rounded_rect(ir.x, top, ir.width, bot - top, 12.0);
                ctx.clip();
                ctx.set_fill_color("#111111");
                ctx.fill_rect(ir.x, ir.y, ir.width, ir.height);
                if let Some(ip) = ctx.image_painter() {
                    ip.draw_image_rgba(rgba, *w, *h, ir.x + (ir.width - iw * k) / 2.0, ir.y + (ir.height - ih * k) / 2.0, iw * k, ih * k);
                }
                ctx.restore();
                // Some backends drop the whole clip stack on restore: re-clip the body.
                ctx.begin_path();
                ctx.rect(l.body.x - 2.0, l.body.y, l.body.width + 4.0, l.body.height);
                ctx.clip();
                if top == ir.y && bot == ir.y + ir.height {
                ctx.set_stroke_color("#3a3a3a");
                    ctx.set_stroke_width(1.0);
                    ctx.begin_path();
                    ctx.rounded_rect(ir.x + 0.5, ir.y + 0.5, ir.width - 1.0, ir.height - 1.0, 12.0);
                    ctx.stroke();
                }
            }
            ComposerAttach::File { name } => {
                if ir.y + ir.height <= l.body.y || ir.y >= l.body.y + l.body.height {
                    continue;
                }
                path_round_rect(ctx, *ir, 10.0);
                ctx.set_fill_color("#2a2a2a");
                ctx.fill();
                // document glyph
                ctx.set_fill_color("#58a6ff");
                ctx.fill_rect(ir.x + 12.0, ir.y + 12.0, 12.0, 16.0);
                ctx.set_font("13px Inter, sans-serif");
                ctx.set_fill_color(st.text);
                let mut t = name.to_string();
                while ctx.measure_text(&t) > 200.0 && t.chars().count() > 2 {
                    t.pop();
                }
                fill_text_css(ctx, &t, ir.x + 32.0, ir.y + (ir.height - 19.0) / 2.0, 13.0, 19.0, INTER, TextAlign::Left);
            }
        }
        if view.hovered_item == Some(i) || view.hovered_remove == Some(i) {
            let x = l.remove[i];
            ctx.begin_path();
            ctx.arc(x.x + 9.0, x.y + 9.0, 9.0, 0.0, std::f64::consts::TAU);
            ctx.set_fill_color(if view.hovered_remove == Some(i) { "#000000" } else { "rgba(0,0,0,0.75)" });
            ctx.fill();
            ctx.set_stroke_color("#ffffff");
            ctx.set_stroke_width(1.6);
            ctx.begin_path();
            ctx.move_to(x.x + 6.0, x.y + 6.0);
            ctx.line_to(x.x + 12.0, x.y + 12.0);
            ctx.move_to(x.x + 12.0, x.y + 6.0);
            ctx.line_to(x.x + 6.0, x.y + 12.0);
            ctx.stroke();
        }
    }
    ctx.restore();
    draw_buttons(ctx, &l.hits, &view.base, view.is_empty(), st);
    positions
}

#[cfg(test)]
mod rich_tests {
    use super::*;
    use crate::testing::NullRenderContext as NullRender;

    #[test]
    fn attachments_grow_the_bubble_and_cap_at_max_height() {
        let mut ctx = NullRender;
        let area = Rect::new(0.0, 0.0, 800.0, 600.0);
        let st = ComposerStyle::default();
        let img = [0u8; 4];
        let one = [ComposerAttach::Image { rgba: &img, w: 1, h: 1 }];
        let v = |a: &'static [ComposerAttach<'static>], t: &'static str| RichComposerView { base: ComposerView { text: t, placeholder: "", hovered: None, cursor: None }, attach: a, hovered_item: None, hovered_remove: None, scroll: 0.0 };
        let empty = composer_layout_rich(&mut ctx, area, &v(&[], ""), 300.0, &st);
        assert!(!empty.tall && v(&[], "").is_empty());
        let leaked: &'static [ComposerAttach<'static>] = Box::leak(Box::new([ComposerAttach::Image { rgba: Box::leak(Box::new([0u8; 4])), w: 1, h: 1 }]));
        let _ = one;
        let tall = composer_layout_rich(&mut ctx, area, &v(leaked, "hi"), 300.0, &st);
        assert!(tall.tall && tall.hits.frame.height > empty.hits.frame.height);
        assert_eq!(tall.items.len(), 1);
        assert!(tall.text_top >= tall.items[0].y + THUMB);
        let long: &'static str = Box::leak("x\n".repeat(40).into_boxed_str());
        let capped = composer_layout_rich(&mut ctx, area, &v(leaked, long), 300.0, &st);
        assert!(capped.hits.frame.height <= 300.0 + 0.5);
        assert!(capped.content_h > capped.body.height, "body scrolls");
        assert!(!v(leaked, "").is_empty());
    }
}
