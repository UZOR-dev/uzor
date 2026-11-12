//! Long lists on a phone: a virtualized list ([`VirtualList`]) that lays out only the
//! rows in sight (plus a few either side) however many there are, a search bar
//! ([`search_bar`]) on the DOM text input, and the matching that filters a list by
//! what was typed ([`search_filter`]). Same monochrome look as the rest of the kit.

use std::ops::Range;

use super::super::dom::{Control, Edit, El};
use super::{ic, icons};

/// A list of `n` rows of one fixed height, scrolled by `scroll` px, of which only the
/// visible ones (and [`VirtualList::overscan`] more either side) are built and laid out.
///
/// The element ([`VirtualList::el`]) is a `div.m-vlist` column that fills what is left
/// of its parent (`flex-grow: 1; min-height: 0; overflow: hidden`) and scrolls its body
/// by `scroll` (CSS `scrollTop`). Its body is exactly [`VirtualList::content_height`]
/// tall: a spacer for the rows above, the rows, a spacer for the rows below. So the
/// scrollbar, `Placed::scroll_height` and clamping see the whole list.
///
/// Scrolling stays the host's: feed `scroll` from a framework `ScrollView` over the
/// list's rect of the last frame (`Dom::rect(id)`), with
/// [`VirtualList::content_height`] as the content height; the finger, the wheel and the
/// fling then work as on any list. [`VirtualList::reveal`] scrolls a row (the keyboard
/// highlight, a search hit) into view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualList {
    /// Height of every row, px. A row's own box is held to it (`div.m-vrow`).
    pub row_h: f64,
    /// Rows built beyond each edge of the viewport, so a fast fling does not show
    /// empty space for a frame.
    pub overscan: usize,
}

impl VirtualList {
    /// Rows `row_h` px tall, 4 rows of overscan.
    pub fn new(row_h: f64) -> Self {
        Self { row_h: row_h.max(1.0), overscan: 4 }
    }

    /// With `rows` of overscan.
    pub fn overscan(mut self, rows: usize) -> Self {
        self.overscan = rows;
        self
    }

    /// Height of all `n` rows.
    pub fn content_height(&self, n: usize) -> f64 {
        n as f64 * self.row_h
    }

    /// The largest useful scroll offset for `n` rows in a viewport `viewport_h` tall.
    pub fn max_scroll(&self, n: usize, viewport_h: f64) -> f64 {
        (self.content_height(n) - viewport_h).max(0.0)
    }

    /// `scroll` clamped to `0 ..= max_scroll`.
    pub fn clamp(&self, n: usize, scroll: f64, viewport_h: f64) -> f64 {
        if scroll.is_finite() {
            scroll.clamp(0.0, self.max_scroll(n, viewport_h))
        } else {
            0.0
        }
    }

    /// The rows that are (even partly) visible at `scroll` in a viewport `viewport_h`
    /// tall, without overscan.
    pub fn visible(&self, n: usize, scroll: f64, viewport_h: f64) -> Range<usize> {
        if n == 0 || viewport_h <= 0.0 {
            return 0..0;
        }
        let s = self.clamp(n, scroll, viewport_h);
        let first = ((s / self.row_h).floor() as usize).min(n);
        let last = (((s + viewport_h) / self.row_h).ceil() as usize).min(n);
        first..last.max(first)
    }

    /// The rows to build: [`Self::visible`] widened by [`Self::overscan`] on each side.
    pub fn range(&self, n: usize, scroll: f64, viewport_h: f64) -> Range<usize> {
        let v = self.visible(n, scroll, viewport_h);
        if v.is_empty() {
            return v;
        }
        v.start.saturating_sub(self.overscan)..(v.end + self.overscan).min(n)
    }

    /// The scroll offset that shows row `i` whole, moving as little as possible from
    /// `scroll` (unchanged when it is already in view).
    pub fn reveal(&self, n: usize, i: usize, scroll: f64, viewport_h: f64) -> f64 {
        let top = i.min(n.saturating_sub(1)) as f64 * self.row_h;
        let s = if top < scroll {
            top
        } else if top + self.row_h > scroll + viewport_h {
            top + self.row_h - viewport_h
        } else {
            scroll
        };
        self.clamp(n, s, viewport_h)
    }

    /// The list element `id` with `n` rows at `scroll` in a viewport `viewport_h` tall
    /// (the list's height on the last frame; the first frame may pass the screen
    /// height). `row(i)` builds row `i`; it is called only for [`Self::range`]. Give
    /// the rows ids (`{id}:row:{i}` or the item's own key) to make them hit targets.
    /// With `n == 0` and `empty` set, the list shows that line instead.
    pub fn el(&self, id: &str, n: usize, scroll: f64, viewport_h: f64, empty: Option<&str>, mut row: impl FnMut(usize) -> El) -> El {
        let list = El::new("div.m-vlist").id(id);
        if n == 0 {
            return match empty {
                Some(t) => list.child(El::new("div.m-empty").text(t)),
                None => list,
            };
        }
        let s = self.clamp(n, scroll, viewport_h);
        let r = self.range(n, s, viewport_h.max(self.row_h));
        let pad = |h: f64| El::new("div.m-vpad").css(format!("height: {h}px"));
        let rows = r.clone().map(|i| El::new("div.m-vrow").css(format!("height: {}px", self.row_h)).child(row(i)));
        let body = El::new("div.m-vbody")
            .child(pad(r.start as f64 * self.row_h))
            .children(rows)
            .child(pad((n - r.end) as f64 * self.row_h));
        list.scroll_top(s).child(body)
    }
}

/// A search bar: the magnifier, the DOM text input `id` (`edit` carries its value,
/// caret and placeholder; [`Edit::placeholder`]), a clear button `{id}:clear` once
/// something is typed, and with `cancel` a text button `{id}:cancel` after the field
/// (iOS: drops the search and the keyboard). Every part is a DOM control, so the
/// framework's DOM host gives the input its caret, selection, IME and clipboard.
pub fn search_bar(id: &str, edit: Edit, name: &str, cancel: Option<&str>) -> El {
    let has_text = !edit.value.is_empty();
    let field = El::new("div.m-search.m-sfield")
        .child(ic(icons::SEARCH))
        .child(El::new("input.m-sinput").id(id).name(name).control(Control::input(edit)))
        .child_if(has_text, || El::new("button.m-sclear").id(format!("{id}:clear")).tab().name("×").svg(super::icon(icons::CLOSE)));
    El::new("div.m-sbar").child(field).child_if(cancel.is_some(), || {
        let t = cancel.unwrap_or_default();
        El::new("button.m-scancel").id(format!("{id}:cancel")).tab().text(t)
    })
}

fn norm(s: &str) -> String {
    super::super::i18n::case::lower(s).replace(['_', '/', '-', '.', ','], " ")
}

/// How well `label` matches `query`: `None` when it does not, else a rank (lower is
/// better) — 0 the label starts with the query, 1 every query word starts a word of
/// the label, 2 every query word is somewhere in it. Case-insensitive in every script
/// (`rsi` finds `RSI`, `индекс` finds `Индекс`); `_ / - . ,` count as spaces. An empty
/// query matches everything at rank 0.
pub fn search_rank(label: &str, query: &str) -> Option<u8> {
    let q = norm(query);
    let q = q.trim();
    if q.is_empty() {
        return Some(0);
    }
    let l = norm(label);
    let words: Vec<&str> = l.split_whitespace().collect();
    let toks: Vec<&str> = q.split_whitespace().collect();
    if l.trim_start().starts_with(q) {
        return Some(0);
    }
    if toks.iter().all(|t| words.iter().any(|w| w.starts_with(t))) {
        return Some(1);
    }
    if toks.iter().all(|t| l.contains(t)) {
        return Some(2);
    }
    None
}

/// Indices of `labels` that match `query` ([`search_rank`]), best first; equal ranks
/// keep the list's order.
pub fn search_filter<L: AsRef<str>>(labels: &[L], query: &str) -> Vec<usize> {
    let mut hits: Vec<(u8, usize)> = labels.iter().enumerate().filter_map(|(i, l)| search_rank(l.as_ref(), query).map(|r| (r, i))).collect();
    hits.sort_by_key(|h| h.0);
    hits.into_iter().map(|h| h.1).collect()
}
