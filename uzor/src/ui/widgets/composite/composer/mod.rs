//! Message composer: rounded box with a text area, `+` attach on the left,
//! mic and a round send button on the right. Draw returns the hit rects.

use crate::input::core::coordinator::LayerId;
use crate::input::{InputCoordinator, Sense, WidgetKind};
use crate::render::{RenderContext, TextAlign, TextBaseline};
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

/// Height of the composer box.
pub const COMPOSER_HEIGHT: f64 = 96.0;

/// Lay out a composer centred at the bottom of `area`, max 760 wide.
pub fn composer_layout(area: Rect) -> ComposerHits {
    let w = (area.width - 32.0).min(760.0).max(120.0);
    let frame = Rect::new(area.x + (area.width - w) / 2.0, area.y + area.height - COMPOSER_HEIGHT - 16.0, w, COMPOSER_HEIGHT);
    let by = frame.y + frame.height - 10.0 - 32.0;
    ComposerHits {
        frame,
        text: Rect::new(frame.x + 16.0, frame.y + 12.0, frame.width - 32.0, 28.0),
        attach: Rect::new(frame.x + 10.0, by, 32.0, 32.0),
        mic: Rect::new(frame.x + frame.width - 10.0 - 32.0 - 8.0 - 32.0, by, 32.0, 32.0),
        send: Rect::new(frame.x + frame.width - 10.0 - 32.0, by, 32.0, 32.0),
    }
}

fn round_btn(ctx: &mut dyn RenderContext, r: Rect, fill: &str) {
    ctx.begin_path();
    ctx.arc(r.x + r.width / 2.0, r.y + r.height / 2.0, r.width / 2.0, 0.0, std::f64::consts::TAU);
    ctx.set_fill_color(fill);
    ctx.fill();
}

/// Paint the composer (light theme). Returns the caret x of every char
/// boundary of `view.text` (feed it to the text-field store for hit-testing).
pub fn draw_composer(ctx: &mut dyn RenderContext, hits: &ComposerHits, view: &ComposerView<'_>) -> Vec<f64> {
    path_round_rect(ctx, hits.frame, 24.0);
    ctx.set_fill_color("#ffffff");
    ctx.fill();
    ctx.set_stroke_color("#e4e4e7");
    ctx.set_stroke_width(1.0);
    ctx.stroke();
    ctx.set_font("15px sans-serif");
    ctx.set_text_align(TextAlign::Left);
    ctx.set_text_baseline(TextBaseline::Middle);
    let (t, c) = if view.text.is_empty() { (view.placeholder, "#9a9aa2") } else { (view.text, "#18181b") };
    ctx.set_fill_color(c);
    ctx.fill_text(t, hits.text.x, hits.text.y + hits.text.height / 2.0);
    let mut positions = vec![hits.text.x];
    let mut acc = String::new();
    for ch in view.text.chars() {
        acc.push(ch);
        positions.push(hits.text.x + ctx.measure_text(&acc));
    }
    if let Some(c) = view.cursor {
        let x = positions.get(c).copied().unwrap_or(*positions.last().unwrap_or(&hits.text.x));
        ctx.set_fill_color("#1d77f2");
        // Pixel-snapped 2px caret (thin sub-pixel fills trip tiny-skia 0.11).
        ctx.fill_rect(x.round(), (hits.text.y + 5.0).round(), 2.0, (hits.text.height - 10.0).round());
    }
    let hov = |p| view.hovered == Some(p);
    round_btn(ctx, hits.attach, if hov(ComposerPart::Attach) { "#e9e9ec" } else { "#f2f2f4" });
    round_btn(ctx, hits.mic, if hov(ComposerPart::Mic) { "#e9e9ec" } else { "#f2f2f4" });
    round_btn(ctx, hits.send, if hov(ComposerPart::Send) { "#3388ff" } else { "#1d77f2" });
    // + glyph
    let (ax, ay) = (hits.attach.x + 16.0, hits.attach.y + 16.0);
    ctx.set_stroke_color("#52525b");
    ctx.set_stroke_width(2.0);
    ctx.set_line_cap("round");
    ctx.begin_path();
    ctx.move_to(ax, ay - 6.0); ctx.line_to(ax, ay + 6.0);
    ctx.move_to(ax - 6.0, ay); ctx.line_to(ax + 6.0, ay);
    ctx.stroke();
    // mic glyph
    let (mx, my) = (hits.mic.x + 16.0, hits.mic.y + 16.0);
    path_round_rect(ctx, Rect::new(mx - 3.0, my - 8.0, 6.0, 10.0), 3.0);
    ctx.stroke();
    ctx.begin_path();
    ctx.arc(mx, my - 1.0, 6.0, 0.0, std::f64::consts::PI);
    ctx.move_to(mx, my + 5.0); ctx.line_to(mx, my + 8.0);
    ctx.stroke();
    // send arrow
    let (sx, sy) = (hits.send.x + 16.0, hits.send.y + 16.0);
    ctx.set_stroke_color("#ffffff");
    ctx.set_stroke_width(2.4);
    ctx.set_line_join("round");
    ctx.begin_path();
    ctx.move_to(sx, sy + 6.0); ctx.line_to(sx, sy - 6.0);
    ctx.move_to(sx - 5.0, sy - 1.0); ctx.line_to(sx, sy - 6.0); ctx.line_to(sx + 5.0, sy - 1.0);
    ctx.stroke();
    ctx.set_line_cap("butt");
    positions
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
        let h = composer_layout(Rect::new(0.0, 0.0, 1000.0, 800.0));
        assert_eq!(h.frame.width, 760.0);
        assert!(h.send.x > h.mic.x && h.mic.x > h.attach.x);
        assert_eq!(h.frame.y + h.frame.height, 784.0);
    }
}
