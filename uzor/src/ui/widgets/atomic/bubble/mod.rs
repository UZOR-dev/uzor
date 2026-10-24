//! Chat message bubble: wrapped text in a rounded box, own messages right
//! (accent), others left (neutral), optional sender line above.

mod tokens;

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
            mine_bg: tokens::MINE_BG,
            mine_fg: tokens::MINE_FG,
            other_bg: tokens::OTHER_BG,
            other_fg: tokens::OTHER_FG,
            from_fg: tokens::FROM_FG,
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
        for (wi, word) in para.split(' ').enumerate() {
            // Break opportunities inside a word: after a hyphen between
            // letters (UAX #14 HY, as browsers do: "one-task-|model").
            for (si, seg) in hyphen_segments(word).into_iter().enumerate() {
                let sep = if si == 0 && wi > 0 && !line.is_empty() { " " } else { "" };
                let cand = format!("{line}{sep}{seg}");
                if ctx.measure_text(&cand) <= max_w || line.is_empty() {
                    line = cand;
                } else {
                    out.push(std::mem::take(&mut line));
                    line = seg.to_string();
                }
            }
        }
        out.push(line);
    }
    out
}

/// `word` split after each hyphen that sits between two alphanumerics.
pub(crate) fn hyphen_segments(word: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let cs: Vec<(usize, char)> = word.char_indices().collect();
    for i in 1..cs.len().saturating_sub(1) {
        let (bi, c) = cs[i];
        if c == '-' && cs[i - 1].1.is_alphanumeric() && cs[i + 1].1.is_alphanumeric() {
            out.push(&word[start..bi + 1]);
            start = bi + 1;
        }
    }
    out.push(&word[start..]);
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

#[cfg(test)]
mod wrap_tests {
    use super::*;
    use crate::testing::RecordingRenderContext;

    #[test]
    fn wraps_at_spaces_and_after_hyphens_like_css() {
        let mut ctx = RecordingRenderContext::with_char_width(1.0);
        assert_eq!(wrap_lines(&mut ctx, "canary one-task-model", 16.0), vec!["canary one-task-", "model"]);
        assert_eq!(wrap_lines(&mut ctx, "a b", 10.0), vec!["a b"]);
        // No break at a slash; a long word stays whole (the caller clips it).
        assert_eq!(wrap_lines(&mut ctx, "x prompt/tools/sub", 8.0), vec!["x", "prompt/tools/sub"]);
        // A leading or trailing hyphen is not a break opportunity.
        assert_eq!(hyphen_segments("-a-b-"), vec!["-a-", "b-"]);
        assert_eq!(wrap_lines(&mut ctx, "l1\nl2", 80.0), vec!["l1", "l2"]);
    }
}
