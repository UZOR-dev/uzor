//! Chat thread pieces laid out like a CSS messenger column: a message bubble
//! (optional sender header, optional reply quote, styled body runs that wrap with
//! `overflow-wrap: anywhere`), and a centred system line ("x joined").
//!
//! Layout and paint are split: [`layout_message`] measures, [`draw_message`] paints
//! the measured result at a position, so a caller can stack and clip freely.

mod tokens;

use crate::render::{RenderContext, TextAlign};
use crate::types::Rect;
use crate::ui::css_text::{fill_text_css, FontMetrics};
use crate::ui::widgets::atomic::session_pill::path_round_rect;

/// One styled run of body text.
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    /// Text (may contain `\n`).
    pub text: String,
    /// Fill colour; `None` uses the bubble's text colour.
    pub color: Option<String>,
    /// CSS weight.
    pub weight: u32,
}

impl Span {
    /// Plain run.
    pub fn plain(t: impl Into<String>) -> Self {
        Self { text: t.into(), color: None, weight: 400 }
    }
}

/// Split `text` into plain runs and `@nick` runs (nick: `[A-Za-z0-9][A-Za-z0-9._-]{1,63}`,
/// preceded by start, whitespace, `(` or `>`), the mention runs get `color` / `weight`.
pub fn mention_spans(text: &str, color: &str, weight: u32) -> Vec<Span> {
    let cs: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut i = 0;
    let nick_ch = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-');
    while i < cs.len() {
        let pre_ok = i == 0 || cs[i - 1].is_whitespace() || matches!(cs[i - 1], '(' | '>');
        if cs[i] == '@' && pre_ok && i + 1 < cs.len() && cs[i + 1].is_ascii_alphanumeric() {
            let mut j = i + 1;
            while j < cs.len() && nick_ch(cs[j]) && j - i <= 64 {
                j += 1;
            }
            if j - i - 1 >= 2 {
                if !cur.is_empty() {
                    out.push(Span::plain(std::mem::take(&mut cur)));
                }
                out.push(Span { text: cs[i..j].iter().collect(), color: Some(color.to_string()), weight });
                i = j;
                continue;
            }
        }
        cur.push(cs[i]);
        i += 1;
    }
    if !cur.is_empty() {
        out.push(Span::plain(cur));
    }
    out
}

/// A reply quote block above the body.
#[derive(Clone, Debug)]
pub struct Quote<'a> {
    /// Quoted author.
    pub who: &'a str,
    /// Author colour.
    pub who_color: &'a str,
    /// Quoted text (one line, ellipsized).
    pub text: &'a str,
}

/// View data of one message.
#[derive(Clone, Debug)]
pub struct MessageView<'a> {
    /// Own message: right aligned, accent fill.
    pub mine: bool,
    /// Sender header `(name, colour)`.
    pub header: Option<(&'a str, &'a str)>,
    /// Reply quote.
    pub quote: Option<Quote<'a>>,
    /// Body runs.
    pub body: &'a [Span],
    /// Reaction badge on the bubble's top corner (empty = none).
    pub reactions: &'a [ReactionView<'a>],
    /// The pointer rests on the badge: it widens to show who reacted (CSS `.rx:hover .rw`).
    pub rx_hover: bool,
}

/// One reaction key in a message's badge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactionView<'a> {
    /// The emoji (or text) key.
    pub key: &'a str,
    /// How many reacted with it.
    pub count: u32,
    /// The viewer is among them (the badge gets the `mine` border).
    pub mine: bool,
    /// Who reacted, shown while the badge is hovered (`You, peer-b`).
    pub who: &'a str,
}

/// Where a message's reaction badge and its pills sit (for hit-testing).
#[derive(Clone, Debug, PartialEq)]
pub struct BadgeLayout {
    /// The whole badge.
    pub rect: Rect,
    /// One rect per reaction key, in order.
    pub pills: Vec<Rect>,
}

/// Metrics and colours (defaults: the HQ web `.msg` computed style).
#[derive(Clone, Debug)]
pub struct ThreadStyle {
    pub them_bg: &'static str,
    pub me_bg: &'static str,
    pub them_fg: &'static str,
    pub me_fg: &'static str,
    pub px: f64,
    pub lh: f64,
    pub pad_x: f64,
    pub pad_y: f64,
    pub radius: f64,
    pub max_w: f64,
    pub max_share: f64,
    pub hdr_px: f64,
    pub hdr_lh: f64,
    pub hdr_weight: u32,
    pub hdr_gap: f64,
    pub q_px: f64,
    pub q_lh: f64,
    pub q_bg: &'static str,
    pub q_bar: &'static str,
    pub q_fg: &'static str,
    pub q_who_weight: u32,
    pub q_pad_x: f64,
    pub q_pad_y: f64,
    pub q_mt: f64,
    pub q_mb: f64,
    pub sys_px: f64,
    pub sys_lh: f64,
    pub sys_fg: &'static str,
    pub sys_pad_y: f64,
    pub sys_margin: f64,
    pub metrics: FontMetrics,
    pub family: &'static str,
    /// Reaction badge (style.css round 21 `.rx` / `.rpill` / `.rn` / `.rw`): the extra gap above
    /// a message with reactions (`.msg.hasrx{margin-top:16px}`), badge offset above the bubble
    /// top, height, horizontal padding, overhang past the bubble's corner, colours.
    pub rx_gap: f64,
    pub rx_top: f64,
    pub rx_h: f64,
    pub rx_pad: f64,
    pub rx_out: f64,
    pub rx_bg: &'static str,
    pub rx_border_mine: &'static str,
    pub rx_px: f64,
    pub rx_n_px: f64,
    pub rx_n_fg: &'static str,
    pub rx_who_px: f64,
    pub rx_who_fg: &'static str,
}

impl Default for ThreadStyle {
    fn default() -> Self {
        Self {
            them_bg: tokens::THEM_BG,
            me_bg: tokens::ME_BG,
            them_fg: tokens::THEM_FG,
            me_fg: tokens::ME_FG,
            px: 14.0,
            lh: 21.0,
            pad_x: 14.0,
            pad_y: 9.0,
            radius: 18.0,
            max_w: 600.0,
            max_share: 0.82,
            hdr_px: 12.5,
            hdr_lh: 18.75,
            hdr_weight: 600,
            hdr_gap: 2.0,
            q_px: 12.5,
            q_lh: 18.75,
            q_bg: tokens::Q_BG,
            q_bar: tokens::Q_BAR,
            q_fg: tokens::Q_FG,
            q_who_weight: 700,
            q_pad_x: 8.0,
            q_pad_y: 4.0,
            q_mt: 2.0,
            q_mb: 6.0,
            sys_px: 12.0,
            sys_lh: 18.0,
            sys_fg: tokens::SYS_FG,
            sys_pad_y: 2.0,
            sys_margin: 6.0,
            metrics: crate::ui::css_text::INTER,
            family: "Inter, sans-serif",
            rx_gap: 16.0,
            rx_top: 16.0,
            rx_h: 24.0,
            rx_pad: 7.0,
            rx_out: 10.0,
            rx_bg: tokens::RX_BG,
            rx_border_mine: tokens::RX_BORDER_MINE,
            rx_px: 13.0,
            rx_n_px: 12.0,
            rx_n_fg: tokens::RX_N_FG,
            rx_who_px: 12.0,
            rx_who_fg: tokens::RX_WHO_FG,
        }
    }
}

fn font(st: &ThreadStyle, w: u32, px: f64) -> String {
    format!("{w} {px}px {}", st.family)
}

/// One placed piece of a wrapped line.
#[derive(Clone, Debug)]
pub struct Piece {
    pub x: f64,
    pub text: String,
    pub span: usize,
}

/// Measured message.
#[derive(Clone, Debug)]
pub struct MessageLayout {
    /// Bubble size.
    pub w: f64,
    pub h: f64,
    /// Wrapped body lines.
    pub lines: Vec<Vec<Piece>>,
}

/// Greedy wrap of styled runs into `max_w` (spaces kept at line ends are dropped,
/// words longer than a line break anywhere).
pub fn wrap_spans(ctx: &mut dyn RenderContext, spans: &[Span], max_w: f64, st: &ThreadStyle) -> Vec<Vec<Piece>> {
    // tokens: (span, text, is_space, is_newline)
    let mut toks: Vec<(usize, String, bool, bool)> = Vec::new();
    for (si, s) in spans.iter().enumerate() {
        let mut cur = String::new();
        let mut cur_sp = false;
        for ch in s.text.chars() {
            if ch == '\n' {
                if !cur.is_empty() {
                    toks.push((si, std::mem::take(&mut cur), cur_sp, false));
                }
                toks.push((si, String::new(), false, true));
                continue;
            }
            let sp = ch == ' ';
            if !cur.is_empty() && sp != cur_sp {
                toks.push((si, std::mem::take(&mut cur), cur_sp, false));
            }
            cur_sp = sp;
            cur.push(ch);
        }
        if !cur.is_empty() {
            toks.push((si, cur, cur_sp, false));
        }
    }
    let mut lines: Vec<Vec<Piece>> = vec![Vec::new()];
    let mut x = 0.0;
    // Some backends trim spaces when measuring: measure runs with spaces between two
    // fixed glyphs so their advance is counted.
    let measure = |ctx: &mut dyn RenderContext, si: usize, t: &str| {
        ctx.set_font(&font(st, spans[si].weight, st.px));
        if t.starts_with(' ') || t.ends_with(' ') {
            ctx.measure_text(&format!("x{t}x")) - ctx.measure_text("xx")
        } else {
            ctx.measure_text(t)
        }
    };
    for (si, t, sp, nl) in toks {
        if nl {
            lines.push(Vec::new());
            x = 0.0;
            continue;
        }
        let w = measure(ctx, si, &t);
        if sp {
            if x > 0.0 && x + w <= max_w {
                lines.last_mut().unwrap().push(Piece { x, text: t, span: si });
                x += w;
            } else if x > 0.0 {
                lines.push(Vec::new());
                x = 0.0;
            }
            continue;
        }
        if x + w <= max_w {
            lines.last_mut().unwrap().push(Piece { x, text: t, span: si });
            x += w;
            continue;
        }
        if x > 0.0 && w <= max_w {
            // drop trailing spaces of the full line
            if let Some(l) = lines.last_mut() {
                while l.last().is_some_and(|p| p.text.trim().is_empty()) {
                    l.pop();
                }
            }
            lines.push(vec![Piece { x: 0.0, text: t, span: si }]);
            x = w;
            continue;
        }
        // break anywhere
        let mut part = String::new();
        for ch in t.chars() {
            let cand = format!("{part}{ch}");
            if x + measure(ctx, si, &cand) > max_w && !(part.is_empty() && x == 0.0) {
                if !part.is_empty() {
                    let pw = measure(ctx, si, &part);
                    lines.last_mut().unwrap().push(Piece { x, text: std::mem::take(&mut part), span: si });
                    let _ = pw;
                }
                lines.push(Vec::new());
                x = 0.0;
                part.push(ch);
            } else {
                part = cand;
            }
        }
        if !part.is_empty() {
            let pw = measure(ctx, si, &part);
            lines.last_mut().unwrap().push(Piece { x, text: part, span: si });
            x += pw;
        }
    }
    lines
}

fn line_w(ctx: &mut dyn RenderContext, l: &[Piece], spans: &[Span], st: &ThreadStyle) -> f64 {
    l.iter()
        .rev()
        .find(|p| !p.text.trim().is_empty())
        .map(|p| {
            ctx.set_font(&font(st, spans[p.span].weight, st.px));
            p.x + ctx.measure_text(&p.text)
        })
        .unwrap_or(0.0)
}

/// Measure a message in a column `col_w` wide.
pub fn layout_message(ctx: &mut dyn RenderContext, v: &MessageView<'_>, col_w: f64, st: &ThreadStyle) -> MessageLayout {
    let maxw = st.max_w.min(col_w * st.max_share);
    let inner = maxw - 2.0 * st.pad_x;
    let lines = wrap_spans(ctx, v.body, inner, st);
    let mut cw = lines.iter().map(|l| line_w(ctx, l, v.body, st)).fold(0.0, f64::max);
    let mut h = 2.0 * st.pad_y + lines.len() as f64 * st.lh;
    if let Some((name, _)) = v.header {
        ctx.set_font(&font(st, st.hdr_weight, st.hdr_px));
        cw = cw.max(ctx.measure_text(name));
        h += st.hdr_lh + st.hdr_gap;
    }
    if let Some(q) = &v.quote {
        // the quote line is nowrap: its max-content widens the bubble up to the max.
        ctx.set_font(&font(st, 400, st.q_px));
        let tw = ctx.measure_text(q.text);
        ctx.set_font(&font(st, st.q_who_weight, st.q_px));
        let ww = ctx.measure_text(q.who);
        cw = cw.max(tw.max(ww) + 2.0 * st.q_pad_x + 3.0);
        h += st.q_mt + 2.0 * st.q_pad_y + 2.0 * st.q_lh + st.q_mb;
    }
    let w = (cw.ceil() + 2.0 * st.pad_x).min(maxw);
    MessageLayout { w, h, lines }
}

fn ellipsize(ctx: &mut dyn RenderContext, s: &str, max: f64) -> String {
    if ctx.measure_text(s) <= max {
        return s.to_string();
    }
    let mut cs: Vec<char> = s.chars().collect();
    while !cs.is_empty() && ctx.measure_text(&format!("{}…", cs.iter().collect::<String>())) > max {
        cs.pop();
    }
    format!("{}…", cs.iter().collect::<String>())
}

/// Paint a measured message with its bubble's top-left at `(x, y)`.
pub fn draw_message(ctx: &mut dyn RenderContext, x: f64, y: f64, v: &MessageView<'_>, l: &MessageLayout, st: &ThreadStyle) {
    path_round_rect(ctx, Rect::new(x, y, l.w, l.h), st.radius);
    ctx.set_fill_color(if v.mine { st.me_bg } else { st.them_bg });
    ctx.fill();
    let fg = if v.mine { st.me_fg } else { st.them_fg };
    let cx = x + st.pad_x;
    let cw = l.w - 2.0 * st.pad_x;
    let mut yy = y + st.pad_y;
    if let Some((name, col)) = v.header {
        ctx.set_font(&font(st, st.hdr_weight, st.hdr_px));
        ctx.set_fill_color(col);
        fill_text_css(ctx, name, cx, yy, st.hdr_px, st.hdr_lh, st.metrics, TextAlign::Left);
        yy += st.hdr_lh + st.hdr_gap;
    }
    if let Some(q) = &v.quote {
        yy += st.q_mt;
        let qh = 2.0 * st.q_pad_y + 2.0 * st.q_lh;
        let r = Rect::new(cx, yy, cw, qh);
        path_round_rect(ctx, r, 6.0);
        ctx.set_fill_color(st.q_bg);
        ctx.fill();
        ctx.save();
        path_round_rect(ctx, r, 6.0);
        ctx.clip();
        ctx.set_fill_color(st.q_bar);
        ctx.fill_rect(cx, yy, 3.0, qh);
        ctx.restore();
        let tx = cx + 3.0 + st.q_pad_x;
        let tmax = cw - 3.0 - 2.0 * st.q_pad_x;
        ctx.set_font(&font(st, st.q_who_weight, st.q_px));
        let who = ellipsize(ctx, q.who, tmax);
        ctx.set_fill_color(q.who_color);
        fill_text_css(ctx, &who, tx, yy + st.q_pad_y, st.q_px, st.q_lh, st.metrics, TextAlign::Left);
        ctx.set_font(&font(st, 400, st.q_px));
        let t = ellipsize(ctx, q.text, tmax);
        ctx.set_fill_color(st.q_fg);
        fill_text_css(ctx, &t, tx, yy + st.q_pad_y + st.q_lh, st.q_px, st.q_lh, st.metrics, TextAlign::Left);
        yy += qh + st.q_mb;
    }
    for line in &l.lines {
        for p in line {
            let s = &v.body[p.span];
            ctx.set_font(&font(st, s.weight, st.px));
            ctx.set_fill_color(s.color.as_deref().unwrap_or(fg));
            fill_text_css(ctx, &p.text, cx + p.x, yy, st.px, st.lh, st.metrics, TextAlign::Left);
        }
        yy += st.lh;
    }
    draw_badge(ctx, x, y, v, l, st);
}

fn badge_items(ctx: &mut dyn RenderContext, v: &MessageView<'_>, st: &ThreadStyle) -> (f64, Vec<(f64, f64)>, f64) {
    // flex row, gap 1px: per key a pill (padding 0 1px) + its `.rw` names (hover only,
    // margin 0 6px 0 3px), then the total `.rn` (padding 0 2px 0 3px) when it is above 1
    // and the badge is not hovered.
    let mut x = st.rx_pad;
    let mut out = Vec::new();
    for (i, r) in v.reactions.iter().enumerate() {
        if i > 0 {
            x += 1.0;
        }
        ctx.set_font(&font(st, 400, st.rx_px));
        let w = ctx.measure_text(r.key) + 2.0;
        let who_w = if v.rx_hover && !r.who.is_empty() {
            ctx.set_font(&font(st, 400, st.rx_who_px));
            ctx.measure_text(r.who).min(220.0) + 9.0
        } else {
            0.0
        };
        out.push((x, w));
        x += w + who_w;
    }
    let tot: u32 = v.reactions.iter().map(|r| r.count).sum();
    let mut n_w = 0.0;
    if tot > 1 && !v.rx_hover {
        ctx.set_font(&font(st, 600, st.rx_n_px));
        n_w = ctx.measure_text(&tot.to_string()) + 5.0;
        x += 1.0 + n_w;
    }
    (x + st.rx_pad, out, n_w)
}

/// The reaction badge of a message whose bubble's top-left is `(x, y)` (`None` without
/// reactions): own messages hang it off the left corner, others' off the right one.
pub fn reaction_badge(ctx: &mut dyn RenderContext, x: f64, y: f64, v: &MessageView<'_>, l: &MessageLayout, st: &ThreadStyle) -> Option<BadgeLayout> {
    if v.reactions.is_empty() {
        return None;
    }
    let (w, items, _) = badge_items(ctx, v, st);
    let bx = if v.mine { x - st.rx_out } else { x + l.w + st.rx_out - w };
    let by = y - st.rx_top;
    Some(BadgeLayout { rect: Rect::new(bx, by, w, st.rx_h), pills: items.iter().map(|(px, pw)| Rect::new(bx + px, by, *pw, st.rx_h)).collect() })
}

fn draw_badge(ctx: &mut dyn RenderContext, x: f64, y: f64, v: &MessageView<'_>, l: &MessageLayout, st: &ThreadStyle) {
    let Some(b) = reaction_badge(ctx, x, y, v, l, st) else { return };
    let (_, items, n_w) = badge_items(ctx, v, st);
    let r = b.rect;
    let mine = v.reactions.iter().any(|x| x.mine);
    // box-shadow 0 2px 8px #0006, 1px border (transparent unless one is the viewer's).
    ctx.set_shadow(0.0, 2.0, 8.0, tokens::RGBA);
    path_round_rect(ctx, r, r.height / 2.0);
    ctx.set_fill_color(if mine { st.rx_border_mine } else { st.rx_bg });
    ctx.fill();
    ctx.clear_shadow();
    if mine {
        path_round_rect(ctx, Rect::new(r.x + 1.0, r.y + 1.0, r.width - 2.0, r.height - 2.0), r.height / 2.0 - 1.0);
        ctx.set_fill_color(st.rx_bg);
        ctx.fill();
    }
    let lh = st.rx_h - 2.0;
    for (i, (px, pw)) in items.iter().enumerate() {
        let rv = &v.reactions[i];
        ctx.set_font(&font(st, 400, st.rx_px));
        ctx.set_fill_color(tokens::ME_FG);
        fill_text_css(ctx, rv.key, r.x + px + 1.0, r.y + 1.0, st.rx_px, lh, st.metrics, TextAlign::Left);
        if v.rx_hover && !rv.who.is_empty() {
            ctx.set_font(&font(st, 400, st.rx_who_px));
            ctx.set_fill_color(st.rx_who_fg);
            let who = ellipsize(ctx, rv.who, 220.0);
            fill_text_css(ctx, &who, r.x + px + pw + 3.0, r.y + 1.0, st.rx_who_px, lh, st.metrics, TextAlign::Left);
        }
    }
    if n_w > 0.0 {
        let tot: u32 = v.reactions.iter().map(|r| r.count).sum();
        ctx.set_font(&font(st, 600, st.rx_n_px));
        ctx.set_fill_color(st.rx_n_fg);
        fill_text_css(ctx, &tot.to_string(), r.x + r.width - st.rx_pad - n_w + 3.0, r.y + 1.0, st.rx_n_px, lh, st.metrics, TextAlign::Left);
    }
}

/// Height of a system line including its vertical margins.
pub fn sys_height(st: &ThreadStyle) -> f64 {
    2.0 * st.sys_margin + 2.0 * st.sys_pad_y + st.sys_lh
}

/// Centred system line in `col` starting at `y` (margin included); returns its height.
pub fn draw_sys(ctx: &mut dyn RenderContext, col: Rect, y: f64, text: &str, st: &ThreadStyle) -> f64 {
    ctx.set_font(&font(st, 400, st.sys_px));
    ctx.set_fill_color(st.sys_fg);
    fill_text_css(ctx, text, col.x + col.width / 2.0, y + st.sys_margin + st.sys_pad_y, st.sys_px, st.sys_lh, st.metrics, TextAlign::Center);
    sys_height(st)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reaction_badge_hangs_off_the_outer_corner() {
        let mut ctx = crate::testing::RecordingRenderContext::with_char_width(8.0);
        let st = ThreadStyle::default();
        let body = vec![Span::plain("hello")];
        let rx = [ReactionView { key: "+", count: 2, mine: false, who: "a, b" }];
        let mut v = MessageView { mine: false, header: None, quote: None, body: &body, reactions: &rx, rx_hover: false };
        let l = layout_message(&mut ctx, &v, 600.0, &st);
        let b = reaction_badge(&mut ctx, 100.0, 200.0, &v, &l, &st).unwrap();
        // them: right edge 10px past the bubble's right edge, 16px above its top, 24 tall
        assert_eq!(b.rect.x + b.rect.width, 100.0 + l.w + 10.0);
        assert_eq!((b.rect.y, b.rect.height), (184.0, 24.0));
        // pill (8 + 2) + gap 1 + total "2" (8 + 5) + padding 7 * 2
        assert_eq!(b.rect.width, 7.0 + 10.0 + 1.0 + 13.0 + 7.0);
        v.mine = true;
        let b = reaction_badge(&mut ctx, 100.0, 200.0, &v, &l, &st).unwrap();
        assert_eq!(b.rect.x, 90.0);
        // hover: names instead of the total
        v.rx_hover = true;
        let b2 = reaction_badge(&mut ctx, 100.0, 200.0, &v, &l, &st).unwrap();
        assert_eq!(b2.rect.width, 7.0 + 10.0 + (4.0 * 8.0 + 9.0) + 7.0);
        assert!(reaction_badge(&mut ctx, 0.0, 0.0, &MessageView { reactions: &[], ..v }, &l, &st).is_none());
    }

    #[test]
    fn mentions_split() {
        let s = mention_spans("@a1.b x (@cc) e@mail", tokens::C_58A6FF, 500);
        let m: Vec<_> = s.iter().filter(|x| x.color.is_some()).map(|x| x.text.as_str()).collect();
        assert_eq!(m, vec!["@a1.b", "@cc"]);
        assert_eq!(s.iter().map(|x| x.text.as_str()).collect::<String>(), "@a1.b x (@cc) e@mail");
    }
}
