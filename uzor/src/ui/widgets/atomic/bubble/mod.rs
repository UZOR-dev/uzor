//! Chat message bubble: wrapped text in a rounded box, own messages right
//! (accent), others left (neutral), optional sender line above.

use crate::render::{RenderContext, TextAlign, TextBaseline};
use crate::types::Rect;
use crate::ui::widgets::atomic::session_pill::path_round_rect;

/// Plain view data.
#[derive(Clone, Debug)]
pub struct BubbleView<'a> {
    /// Body text.
    pub body: &'a str,
    /// Sender label shown above others' bubbles.
    pub from: Option<&'a str>,
    /// Own message (right, accent).
    pub mine: bool,
}

/// Colours / metrics.
#[derive(Clone, Debug)]
pub struct BubbleStyle {
    /// Own bubble fill.
    pub mine_bg: &'static str,
    /// Own text.
    pub mine_fg: &'static str,
    /// Others' fill.
    pub other_bg: &'static str,
    /// Others' text.
    pub other_fg: &'static str,
    /// Sender label colour.
    pub from_fg: &'static str,
    /// CSS font.
    pub font: &'static str,
    /// Line height.
    pub line_h: f64,
    /// Max bubble width as share of the column.
    pub max_share: f64,
}

impl Default for BubbleStyle {
    fn default() -> Self {
        Self {
            mine_bg: "#116dd2",
            mine_fg: "#ffffff",
            other_bg: "#f2f2f4",
            other_fg: "#18181b",
            from_fg: "#8e8e96",
            font: "14.5px sans-serif",
            line_h: 21.0,
            max_share: 0.72,
        }
    }
}

/// Greedy word wrap with the current font.
pub fn wrap_lines(ctx: &mut dyn RenderContext, text: &str, max_w: f64) -> Vec<String> {
    let mut out = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split(' ') {
            let cand = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if ctx.measure_text(&cand) <= max_w || line.is_empty() {
                line = cand;
            } else {
                out.push(std::mem::take(&mut line));
                line = word.to_string();
            }
        }
        out.push(line);
    }
    out
}

/// Height the bubble takes in a column `col_w` wide.
pub fn bubble_height(ctx: &mut dyn RenderContext, view: &BubbleView<'_>, col_w: f64, style: &BubbleStyle) -> f64 {
    ctx.set_font(style.font);
    let n = wrap_lines(ctx, view.body, col_w * style.max_share - 28.0).len() as f64;
    let from = if view.from.is_some() && !view.mine { 18.0 } else { 0.0 };
    from + n * style.line_h + 18.0
}

/// Paint at the top of `col` (x/width = message column); returns its height.
pub fn draw_bubble(ctx: &mut dyn RenderContext, col: Rect, view: &BubbleView<'_>, style: &BubbleStyle) -> f64 {
    ctx.set_font(style.font);
    let lines = wrap_lines(ctx, view.body, col.width * style.max_share - 28.0);
    let tw = lines.iter().map(|l| ctx.measure_text(l)).fold(0.0, f64::max);
    let w = tw + 28.0;
    let mut y = col.y;
    ctx.set_text_align(TextAlign::Left);
    ctx.set_text_baseline(TextBaseline::Middle);
    if let (Some(f), false) = (view.from, view.mine) {
        ctx.set_font("12.5px sans-serif");
        ctx.set_fill_color(style.from_fg);
        ctx.fill_text(f, col.x + 4.0, y + 8.0);
        ctx.set_font(style.font);
        y += 18.0;
    }
    let h = lines.len() as f64 * style.line_h + 18.0;
    let x = if view.mine { col.x + col.width - w } else { col.x };
    path_round_rect(ctx, Rect::new(x, y, w, h), 18.0);
    ctx.set_fill_color(if view.mine { style.mine_bg } else { style.other_bg });
    ctx.fill();
    ctx.set_fill_color(if view.mine { style.mine_fg } else { style.other_fg });
    for (i, l) in lines.iter().enumerate() {
        ctx.fill_text(l, x + 14.0, y + 9.0 + style.line_h * (i as f64 + 0.5));
    }
    (y - col.y) + h
}
