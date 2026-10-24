//! The open list of a `<select>`: a framework-agnostic model and painter.
//!
//! [`SelectState`] is the browser listbox behaviour: the highlighted row, type-to-search
//! (`query`), the first visible row of a [`SelectStyle::rows`] window, and the keys a
//! native select answers (arrows, Page Up/Down, Home/End, Enter picks, Escape closes,
//! Backspace edits the search). [`place`] anchors the list under its trigger and keeps it
//! inside a container (a modal card, the window): it flips above when there is no room
//! below and shifts left / up instead of running past the container edge. [`draw_list`]
//! paints it from a CSS-driven [`SelectStyle`] (`div.select-list { --row-h: 30px; ... }`).
//!
//! Both the kit builder (`uzor_framework::widgets::kit::select`) and apps that host the
//! list in their own overlay use these, so every select behaves the same.

use crate::input::keyboard::KeyCode;
use crate::layout::{place_anchored, Side};
use crate::render::{RenderContext, TextAlign};
use crate::types::Rect;
use crate::ui::css_box::ellipsize;
use crate::ui::css_text::{fill_text_css, INTER};
use crate::ui::widgets::composite::dialog::{draw_card, fill_round_rect, CardStyle, Shadow};

mod tokens;

/// The look of an open list, set from CSS (`Styled`): standard `background`,
/// `border-color`, `border-radius`, `box-shadow`, `color`, `font-size`, plus the custom
/// properties named after the fields (`--row-h`, `--row-hi`, `--check`, ...).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectStyle {
    pub bg: &'static str,
    pub border: Option<&'static str>,
    pub radius: f64,
    pub shadow: Option<Shadow>,
    /// Option text.
    pub color: &'static str,
    /// Placeholder / "No matches".
    pub sub: &'static str,
    pub px: f64,
    pub row_h: f64,
    pub row_radius: f64,
    /// Hovered row fill.
    pub row_hover: &'static str,
    /// Highlighted (keyboard) row fill.
    pub row_hi: &'static str,
    /// Tick on the chosen row.
    pub check: &'static str,
    pub search_bg: &'static str,
    /// Rows shown at once; more scroll.
    pub rows: f64,
    /// Minimum list width (a narrow trigger still gets a readable list).
    pub min_w: f64,
    /// Minimum width of a searchable list.
    pub search_min_w: f64,
    /// Scroll thumb.
    pub thumb: &'static str,
    /// Space between trigger and list.
    pub gap: f64,
}

crate::styled!(SelectStyle {
    bg = "background",
    border = "border-color",
    radius = "border-radius",
    shadow = "box-shadow",
    color = "color",
    px = "font-size",
    sub,
    row_h,
    row_radius,
    row_hover,
    row_hi,
    check,
    search_bg,
    rows,
    min_w,
    search_min_w,
    thumb,
    gap,
});

const PAD: f64 = 4.0;
const SEARCH_H: f64 = 34.0;

impl SelectStyle {
    /// Rows in the window.
    pub fn window(&self) -> usize {
        (self.rows.max(1.0)) as usize
    }
    /// This style with as many rows as fit a container `h` tall (at least 3), so a list
    /// never runs past a short dialog: it scrolls instead.
    pub fn fit(mut self, h: f64, searchable: bool) -> Self {
        let room = h - 8.0 - 2.0 * PAD - if searchable { SEARCH_H } else { 0.0 };
        self.rows = self.rows.min((room / self.row_h).floor().max(3.0));
        self
    }
    /// Size of the list for `n` matching options under a trigger `anchor_w` wide.
    pub fn list_size(&self, n: usize, searchable: bool, anchor_w: f64) -> (f64, f64) {
        let w = anchor_w.max(if searchable { self.search_min_w } else { self.min_w });
        (w, 2.0 * PAD + if searchable { SEARCH_H } else { 0.0 } + self.row_h * n.clamp(1, self.window()) as f64)
    }
}

/// `value` / `label` match the search `query` (case-insensitive; `_` and `/` match spaces,
/// so `new york` finds `America/New_York`).
pub fn matches(value: &str, label: &str, query: &str) -> bool {
    let norm = |s: &str| s.to_lowercase().replace(['_', '/'], " ");
    let q = norm(query.trim());
    q.is_empty() || norm(label).contains(&q) || norm(value).contains(&q)
}

/// Indices of the `(value, label)` options matching `query`.
pub fn filter<V: AsRef<str>, L: AsRef<str>>(opts: &[(V, L)], query: &str) -> Vec<usize> {
    opts.iter().enumerate().filter(|(_, (v, l))| matches(v.as_ref(), l.as_ref(), query)).map(|(i, _)| i).collect()
}

/// What a key did to an open list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectKey {
    /// Highlight or search changed: repaint.
    Changed,
    /// Pick this row of the filtered list (Enter).
    Pick(usize),
    /// Close without picking (Escape).
    Close,
    /// Not a list key: let it through.
    Ignored,
}

/// An open list: highlighted row (index into the filtered list), search text, first
/// visible row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectState {
    pub hi: usize,
    pub query: String,
    pub top: usize,
}

impl SelectState {
    /// Opened on option `chosen` (highlighted and scrolled into a `window`-row view).
    pub fn open_at(chosen: Option<usize>, window: usize) -> Self {
        let hi = chosen.unwrap_or(0);
        Self { hi, query: String::new(), top: scroll_to(0, hi, window) }
    }

    /// Apply a key to a list of `n` matching rows. `text` is the key's logical text,
    /// `command` = Ctrl or Meta held (shortcuts are not search text).
    pub fn key(&mut self, code: KeyCode, text: Option<&str>, command: bool, n: usize, searchable: bool, window: usize) -> SelectKey {
        let last = n.saturating_sub(1);
        match code {
            KeyCode::ArrowDown => self.hi = (self.hi + 1).min(last),
            KeyCode::ArrowUp => self.hi = self.hi.saturating_sub(1),
            KeyCode::PageDown => self.hi = (self.hi + window).min(last),
            KeyCode::PageUp => self.hi = self.hi.saturating_sub(window),
            KeyCode::Home => self.hi = 0,
            KeyCode::End => self.hi = last,
            KeyCode::Enter => return if n > 0 { SelectKey::Pick(self.hi.min(last)) } else { SelectKey::Close },
            KeyCode::Escape => return SelectKey::Close,
            KeyCode::Backspace if searchable => {
                self.query.pop();
                self.hi = 0;
            }
            _ => match text {
                Some(t) if searchable && !command && !t.is_empty() && !t.chars().any(char::is_control) && !(t == " " && self.query.is_empty()) => {
                    self.query.push_str(t);
                    self.hi = 0;
                }
                _ => return SelectKey::Ignored,
            },
        }
        self.top = scroll_to(self.top, self.hi, window).min(n.saturating_sub(window));
        SelectKey::Changed
    }

    /// Wheel `dy` (positive = down) over a list of `n` rows: scroll the window by 3.
    pub fn wheel(&mut self, dy: f64, n: usize, window: usize) -> bool {
        if dy == 0.0 || n <= window {
            return false;
        }
        let max = n - window;
        let top = if dy > 0.0 { (self.top + 3).min(max) } else { self.top.saturating_sub(3) };
        let moved = top != self.top;
        self.top = top;
        moved
    }
}

/// Keep `hi` visible: the first row of a `window`-row view starting at `top`.
pub fn scroll_to(top: usize, hi: usize, window: usize) -> usize {
    if hi < top {
        hi
    } else if hi >= top + window {
        hi + 1 - window
    } else {
        top
    }
}

/// Where a `w`×`h` list opens for a trigger `anchor`: under it (left-aligned), flipped
/// above when it does not fit below and above has more room, then shifted to stay inside
/// `within` (the modal card or window that contains the trigger).
pub fn place(anchor: Rect, w: f64, h: f64, within: Rect, gap: f64) -> Rect {
    let w = w.min(within.width - 8.0).max(0.0);
    place_anchored(anchor, w, h, within, Side::Below, gap, 4.0, 0.0)
}

/// The taller of the two places a list can open for `anchor` inside `within` (below or
/// above it, less the `gap` and the 4px margin): feed it to [`SelectStyle::fit`] so the
/// list never covers its own trigger.
pub fn room(anchor: Rect, within: Rect, gap: f64) -> f64 {
    let below = within.y + within.height - 4.0 - (anchor.y + anchor.height + gap);
    let above = anchor.y - gap - (within.y + 4.0);
    below.max(above) + 8.0
}

/// Paint the list in `b`: an optional search line showing `query`, then the rows of the
/// window from `st.top` (`rows`: label, chosen), the highlighted one filled, the chosen
/// one ticked. `hover(i)` = row `i` is under the pointer. Returns `(row index, rect)` of
/// the painted rows.
pub fn draw_list(ctx: &mut dyn RenderContext, b: Rect, rows: &[(&str, bool)], st: &SelectState, query: Option<&str>, hover: &dyn Fn(usize) -> bool, s: &SelectStyle) -> Vec<(usize, Rect)> {
    draw_card(ctx, b, &CardStyle { bg: s.bg, border: s.border, radius: s.radius, shadow: s.shadow });
    let font = format!("400 {}px Inter, sans-serif", s.px);
    let lh = (s.px * 1.5).round();
    let mut y = b.y + PAD;
    if let Some(q) = query {
        let r = Rect::new(b.x + 6.0, y + 2.0, b.width - 12.0, SEARCH_H - 6.0);
        fill_round_rect(ctx, r, s.row_radius, s.search_bg);
        magnifier(ctx, r.x + 8.0, r.y + r.height / 2.0, s.sub);
        let (t, col) = if q.is_empty() { ("Type to search", s.sub) } else { (q, s.color) };
        ctx.set_font(&font);
        let t = ellipsize(ctx, t, r.width - 40.0);
        ctx.set_fill_color(col);
        fill_text_css(ctx, &t, r.x + 28.0, r.y + (r.height - lh) / 2.0, s.px, lh, INTER, TextAlign::Left);
        if !q.is_empty() {
            let cx = r.x + 28.0 + ctx.measure_text(&t) + 1.0;
            ctx.set_fill_color(s.color);
            ctx.fill_rect(cx, r.y + 6.0, 1.0, r.height - 12.0);
        }
        y += SEARCH_H;
    }
    let win = s.window();
    let list_y = y;
    if rows.is_empty() {
        ctx.set_font(&font);
        ctx.set_fill_color(s.sub);
        fill_text_css(ctx, "No matches", b.x + 14.0, y + (s.row_h - lh) / 2.0, s.px, lh, INTER, TextAlign::Left);
    }
    let mut out = Vec::new();
    for (i, (label, on)) in rows.iter().enumerate().skip(st.top).take(win) {
        let r = Rect::new(b.x + PAD, y, b.width - 2.0 * PAD, s.row_h);
        if i == st.hi {
            fill_round_rect(ctx, r, s.row_radius, s.row_hi);
        } else if hover(i) {
            fill_round_rect(ctx, r, s.row_radius, s.row_hover);
        }
        if *on {
            tick(ctx, r.x + 8.0, r.y + r.height / 2.0, s.check);
        }
        ctx.set_font(&font);
        let t = ellipsize(ctx, label, r.width - 40.0);
        ctx.set_fill_color(s.color);
        fill_text_css(ctx, &t, r.x + 28.0, r.y + (r.height - lh) / 2.0, s.px, lh, INTER, TextAlign::Left);
        out.push((i, r));
        y += s.row_h;
    }
    let n = rows.len();
    if n > win {
        let lh = s.row_h * win as f64;
        let th = (lh * win as f64 / n as f64).max(18.0);
        let ty = list_y + (lh - th) * st.top as f64 / (n - win) as f64;
        fill_round_rect(ctx, Rect::new(b.x + b.width - 6.0, ty, 3.0, th), 1.5, s.thumb);
    }
    out
}

fn tick(ctx: &mut dyn RenderContext, x: f64, cy: f64, color: &str) {
    ctx.set_stroke_color(color);
    ctx.set_stroke_width(2.0);
    ctx.set_line_cap("round");
    ctx.set_line_join("round");
    ctx.begin_path();
    ctx.move_to(x + 1.0, cy);
    ctx.line_to(x + 5.0, cy + 4.0);
    ctx.line_to(x + 12.0, cy - 4.0);
    ctx.stroke();
    ctx.set_line_cap("butt");
}

fn magnifier(ctx: &mut dyn RenderContext, x: f64, cy: f64, color: &str) {
    ctx.set_stroke_color(color);
    ctx.set_stroke_width(1.5);
    ctx.set_line_cap("round");
    ctx.begin_path();
    ctx.arc(x + 5.5, cy - 1.0, 4.5, 0.0, std::f64::consts::TAU);
    ctx.stroke();
    ctx.begin_path();
    ctx.move_to(x + 9.0, cy + 2.5);
    ctx.line_to(x + 12.0, cy + 5.5);
    ctx.stroke();
    ctx.set_line_cap("butt");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::style::{Sheet, Styled};

    fn zones() -> Vec<(String, String)> {
        ["Asia/Yekaterinburg", "Europe/Berlin", "America/New_York"].iter().map(|z| (z.to_string(), z.to_string())).collect()
    }

    #[test]
    fn filter_matches_like_a_person_types() {
        let o = zones();
        assert_eq!(filter(&o, "new york"), vec![2]);
        assert_eq!(filter(&o, "BER"), vec![1]);
        assert_eq!(filter(&o, "").len(), 3);
    }

    #[test]
    fn keys_move_search_pick_and_close() {
        let mut s = SelectState::open_at(Some(12), 10);
        assert_eq!(s.top, 3);
        assert_eq!(s.key(KeyCode::Home, None, false, 30, true, 10), SelectKey::Changed);
        assert_eq!((s.hi, s.top), (0, 0));
        s.key(KeyCode::PageDown, None, false, 30, true, 10);
        assert_eq!((s.hi, s.top), (10, 1));
        s.key(KeyCode::End, None, false, 30, true, 10);
        assert_eq!((s.hi, s.top), (29, 20));
        s.key(KeyCode::ArrowDown, None, false, 30, true, 10);
        assert_eq!(s.hi, 29);
        // Typing filters (and restarts the highlight); a shortcut or a leading space does not.
        assert_eq!(s.key(KeyCode::B, Some("b"), false, 30, true, 10), SelectKey::Changed);
        assert_eq!((s.query.as_str(), s.hi, s.top), ("b", 0, 0));
        assert_eq!(s.key(KeyCode::C, Some("c"), true, 3, true, 10), SelectKey::Ignored);
        assert_eq!(s.key(KeyCode::Space, Some(" "), false, 3, false, 10), SelectKey::Ignored);
        s.key(KeyCode::Backspace, None, false, 30, true, 10);
        assert_eq!(s.query, "");
        s.key(KeyCode::ArrowDown, None, false, 3, true, 10);
        assert_eq!(s.key(KeyCode::Enter, None, false, 3, true, 10), SelectKey::Pick(1));
        assert_eq!(s.key(KeyCode::Escape, None, false, 3, true, 10), SelectKey::Close);
        assert_eq!(SelectState::default().key(KeyCode::Enter, None, false, 0, true, 10), SelectKey::Close);
    }

    #[test]
    fn wheel_scrolls_the_window() {
        let mut s = SelectState::default();
        assert!(s.wheel(1.0, 12, 10));
        assert_eq!(s.top, 2);
        assert!(!s.wheel(1.0, 12, 10));
        assert!(!s.wheel(1.0, 5, 10));
    }

    #[test]
    fn list_stays_inside_its_container() {
        let modal = Rect::new(100.0, 100.0, 600.0, 400.0);
        // A trigger near the modal's right edge: the list shifts left to stay inside.
        let r = place(Rect::new(600.0, 150.0, 90.0, 28.0), 280.0, 200.0, modal, 4.0);
        assert!(r.x + r.width <= 696.0 + 0.01 && r.y == 182.0, "{r:?}");
        // Near the bottom: it flips above the trigger.
        let r = place(Rect::new(300.0, 440.0, 120.0, 28.0), 160.0, 200.0, modal, 4.0);
        assert_eq!(r.y, 440.0 - 4.0 - 200.0);
        // Taller than the room on both sides: shifted to stay within the container.
        let r = place(Rect::new(300.0, 300.0, 120.0, 28.0), 160.0, 380.0, modal, 4.0);
        assert!(r.y >= 104.0 && r.y + r.height <= 496.0 + 0.01, "{r:?}");
        // A long list in a short container shows fewer rows.
        let s = SelectStyle::default().fit(300.0, true);
        assert_eq!(s.window(), 8);
        assert!(s.list_size(30, true, 90.0).1 <= 292.0);
    }

    #[test]
    fn style_from_css() {
        let sh = Sheet::parse("div.select-list{background:black;border-radius:12px;--row-h:28px;--rows:8;--check:green;font-size:14px}").unwrap();
        let s = SelectStyle::from_sheet(&sh, "div.select-list");
        assert_eq!((s.bg, s.radius, s.row_h, s.window(), s.check, s.px), ("black", 12.0, 28.0, 8, "green", 14.0));
        assert_eq!(s.list_size(30, true, 100.0), (280.0, 8.0 + 34.0 + 28.0 * 8.0));
    }
}
