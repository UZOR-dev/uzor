//! Flex and grid track solvers (DOM-like roadmap item 2): the arithmetic behind
//! `display:flex` / `display:grid` so screens stop hand-computing x offsets.
//!
//! All functions are pure and return `(offset, size)` pairs along one axis.
//!
//! ```
//! use uzor::ui::flow::{grid_tracks, Track};
//! // grid-template-columns: 34px 1fr 76px 190px 64px; gap: 10px
//! let cols = grid_tracks(600.0, &[Track::Px(34.0), Track::Fr(1.0), Track::Px(76.0), Track::Px(190.0), Track::Px(64.0)], 10.0);
//! assert_eq!(cols[1], (44.0, 196.0));
//! ```

/// `justify-content` along the main axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Justify {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
}

/// One flex item: `flex: grow shrink basis` with min/max clamps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlexItem {
    pub basis: f64,
    pub grow: f64,
    pub shrink: f64,
    pub min: f64,
    pub max: f64,
}

impl FlexItem {
    /// A fixed-size item (`flex: none`).
    pub fn fixed(w: f64) -> Self {
        FlexItem { basis: w, grow: 0.0, shrink: 0.0, min: w, max: w }
    }
    /// `flex: 1` (basis 0, grows to fill).
    pub fn fill() -> Self {
        FlexItem { basis: 0.0, grow: 1.0, shrink: 1.0, min: 0.0, max: f64::INFINITY }
    }
    /// Content-sized item that may shrink (`flex: 0 1 auto`).
    pub fn auto(w: f64) -> Self {
        FlexItem { basis: w, grow: 0.0, shrink: 1.0, min: 0.0, max: f64::INFINITY }
    }
}

/// Lay items out on one line of `avail` pixels (a single-line flex container).
pub fn flex_row(avail: f64, items: &[FlexItem], gap: f64, justify: Justify) -> Vec<(f64, f64)> {
    if items.is_empty() {
        return vec![];
    }
    let gaps = gap * (items.len() - 1) as f64;
    let n = items.len();
    // CSS "resolve flexible lengths" (css-flexbox-1 §9.7).
    let hyp: Vec<f64> = items.iter().map(|i| i.basis.clamp(i.min, i.max)).collect();
    let grow = hyp.iter().sum::<f64>() + gaps < avail;
    let mut size: Vec<f64> = items.iter().map(|i| i.basis).collect();
    let mut frozen: Vec<bool> = (0..n)
        .map(|k| {
            let it = &items[k];
            let factor = if grow { it.grow } else { it.shrink };
            factor == 0.0 || (grow && it.basis > hyp[k]) || (!grow && it.basis < hyp[k])
        })
        .collect();
    for k in 0..n {
        if frozen[k] {
            size[k] = hyp[k];
        }
    }
    for _ in 0..=n {
        if frozen.iter().all(|f| *f) {
            break;
        }
        let used: f64 = (0..n).map(|k| if frozen[k] { size[k] } else { items[k].basis }).sum();
        let free = avail - gaps - used;
        let weight = |k: usize| if grow { items[k].grow } else { items[k].shrink * items[k].basis };
        let total: f64 = (0..n).filter(|&k| !frozen[k]).map(weight).sum();
        let mut viol = vec![0.0; n];
        for k in (0..n).filter(|&k| !frozen[k]) {
            let want = if total > 0.0 { items[k].basis + free * weight(k) / total } else { items[k].basis };
            let got = want.clamp(items[k].min, items[k].max);
            viol[k] = got - want;
            size[k] = got;
        }
        let tv: f64 = viol.iter().sum();
        let open: Vec<usize> = (0..n).filter(|&k| !frozen[k]).collect();
        for k in open {
            if tv.abs() < 1e-9 || (tv > 0.0 && viol[k] > 0.0) || (tv < 0.0 && viol[k] < 0.0) {
                frozen[k] = true;
            }
        }
    }
    let used = size.iter().sum::<f64>() + gaps;
    let rest = (avail - used).max(0.0);
    let (mut x, step) = match justify {
        Justify::Start => (0.0, gap),
        Justify::End => (rest, gap),
        Justify::Center => (rest / 2.0, gap),
        Justify::SpaceBetween if items.len() > 1 => (0.0, gap + rest / (items.len() - 1) as f64),
        Justify::SpaceBetween => (0.0, gap),
    };
    size.iter()
        .map(|&w| {
            let r = (x, w);
            x += w + step;
            r
        })
        .collect()
}

/// `flex-wrap: wrap`: place fixed-size boxes left to right, breaking lines at `avail`.
/// Returns `(x, y)` per box and the total height.
pub fn flex_wrap(avail: f64, sizes: &[(f64, f64)], gap_x: f64, gap_y: f64) -> (Vec<(f64, f64)>, f64) {
    let (mut x, mut y, mut line_h) = (0.0, 0.0, 0.0f64);
    let mut out = Vec::with_capacity(sizes.len());
    for &(w, h) in sizes {
        if x > 0.0 && x + w > avail + 1e-9 {
            y += line_h + gap_y;
            x = 0.0;
            line_h = 0.0;
        }
        out.push((x, y));
        x += w + gap_x;
        line_h = line_h.max(h);
    }
    (out, if sizes.is_empty() { 0.0 } else { y + line_h })
}

/// A grid track size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Track {
    Px(f64),
    Fr(f64),
    /// `minmax(min, Nfr)`
    MinMax(f64, f64),
}

/// `grid-template-columns` resolution: fixed tracks first, the rest shared by `fr`,
/// never below a `minmax` minimum.
pub fn grid_tracks(avail: f64, tracks: &[Track], gap: f64) -> Vec<(f64, f64)> {
    let fr: f64 = tracks.iter().map(|t| match *t { Track::Px(_) => 0.0, Track::Fr(f) | Track::MinMax(_, f) => f }).sum();
    // Each fr track gets max(min, its fr share of the space left after fixed tracks and gaps).
    let fixed: f64 = tracks.iter().filter_map(|t| if let Track::Px(w) = t { Some(*w) } else { None }).sum::<f64>()
        + gap * tracks.len().saturating_sub(1) as f64;
    let flex_space = (avail - fixed).max(0.0);
    let mut x = 0.0;
    tracks
        .iter()
        .map(|t| {
            let w = match *t {
                Track::Px(w) => w,
                Track::Fr(f) => flex_space * f / fr.max(1e-9),
                Track::MinMax(m, f) => (flex_space * f / fr.max(1e-9)).max(m),
            };
            let r = (x, w);
            x += w + gap;
            r
        })
        .collect()
}

/// `repeat(auto-fill, minmax(min, 1fr))`: column count and column width.
pub fn auto_fill(avail: f64, min: f64, gap: f64) -> (usize, f64) {
    let n = (((avail + gap) / (min + gap)).floor() as usize).max(1);
    (n, ((avail - gap * (n - 1) as f64) / n as f64).max(0.0))
}

// ---------------------------------------------------------------------------------------
// Box tree layout: block, flex (row / column, wrap, justify, align) and grid, with
// intrinsic (min-/max-content) sizing. `display:flex` needs the `flow-flex` feature and
// `display:grid` the `flow-grid` feature (both default); without one, such a box lays out as a
// block, so an app pulls in only the solvers it uses. The track functions above are always there. Boxes are border-box (`box-sizing: border-box`).

use crate::ui::style::Style;
use crate::Rect;

/// Text (or any measured) content: given a width limit (`None` = unlimited) return the
/// laid-out `(width, height)`. `Some(0.0)` asks for the min-content width (every break taken).
pub type Measure<'a> = &'a dyn Fn(Option<f64>) -> (f64, f64);

/// One box of a layout tree.
pub struct LBox<'a> {
    pub style: Style,
    pub measure: Option<Measure<'a>>,
    pub children: Vec<LBox<'a>>,
}

impl<'a> LBox<'a> {
    pub fn new(style: Style) -> Self {
        LBox { style, measure: None, children: Vec::new() }
    }
    /// Shorthand: a box from inline CSS (`"display:flex;gap:8px"`).
    pub fn css(decls: &str) -> Self {
        LBox::new(crate::ui::style::inline(decls))
    }
    pub fn child(mut self, c: LBox<'a>) -> Self {
        self.children.push(c);
        self
    }
    pub fn text(mut self, m: Measure<'a>) -> Self {
        self.measure = Some(m);
        self
    }
}

/// A laid-out box: its border-box rect and its children, in tree order.
#[derive(Clone, Debug, PartialEq)]
pub struct Laid {
    pub rect: Rect,
    pub children: Vec<Laid>,
}

impl Laid {
    /// Every rect in pre-order (root first), the order the goldens dump.
    pub fn flatten(&self) -> Vec<Rect> {
        let mut v = vec![self.rect];
        for c in &self.children {
            v.extend(c.flatten());
        }
        v
    }
}

fn bw(s: &Style) -> f64 {
    s.border_width.unwrap_or(0.0)
}
fn hz(s: &Style) -> f64 {
    let p = s.pad();
    p[1] + p[3] + 2.0 * bw(s)
}
fn vt(s: &Style) -> f64 {
    let p = s.pad();
    p[0] + p[2] + 2.0 * bw(s)
}
fn mh(s: &Style) -> f64 {
    let m = s.mar();
    m[1] + m[3]
}
#[cfg(any(feature = "flow-flex", feature = "flow-grid"))]
fn mv(s: &Style) -> f64 {
    let m = s.mar();
    m[0] + m[2]
}
fn clamp_w(s: &Style, w: f64) -> f64 {
    w.min(s.max_width.unwrap_or(f64::INFINITY)).max(s.min_width.unwrap_or(0.0))
}
fn clamp_h(s: &Style, h: f64) -> f64 {
    h.min(s.max_height.unwrap_or(f64::INFINITY)).max(s.min_height.unwrap_or(0.0))
}
#[cfg(feature = "flow-flex")]
fn is(s: &Option<String>, v: &str) -> bool {
    s.as_deref() == Some(v)
}
fn display(s: &Style) -> &str {
    s.display.as_deref().unwrap_or("block")
}
#[cfg(feature = "flow-flex")]
fn is_col(s: &Style) -> bool {
    matches!(s.flex_direction.as_deref(), Some("column") | Some("column-reverse"))
}
#[cfg(any(feature = "flow-flex", feature = "flow-grid"))]
fn row_gap(s: &Style) -> f64 {
    s.row_gap.or(s.gap).unwrap_or(0.0)
}
#[cfg(any(feature = "flow-flex", feature = "flow-grid"))]
fn col_gap(s: &Style) -> f64 {
    s.gap.unwrap_or(0.0)
}

/// Parse `grid-template-columns` for a content width.
pub fn parse_tracks(spec: &str, avail: f64, gap: f64) -> Vec<Track> {
    fn one(t: &str) -> Option<Track> {
        let t = t.trim();
        if let Some(f) = t.strip_suffix("fr") {
            return f.trim().parse().ok().map(Track::Fr);
        }
        if let Some(inner) = t.strip_prefix("minmax(").and_then(|r| r.strip_suffix(')')) {
            let (a, b) = inner.split_once(',')?;
            let m = a.trim().trim_end_matches("px").parse().ok()?;
            let f = b.trim().strip_suffix("fr")?.trim().parse().ok()?;
            return Some(Track::MinMax(m, f));
        }
        t.trim_end_matches("px").parse().ok().map(Track::Px)
    }
    // split on top-level spaces (not inside parentheses)
    let mut parts = Vec::new();
    let (mut depth, mut cur) = (0, String::new());
    for ch in spec.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if ch.is_whitespace() && depth == 0 {
            if !cur.is_empty() {
                parts.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        parts.push(cur);
    }
    let mut out = Vec::new();
    for p in parts {
        if let Some(inner) = p.strip_prefix("repeat(").and_then(|r| r.strip_suffix(')')) {
            let Some((n, t)) = inner.split_once(',') else { continue };
            let Some(t) = one(t) else { continue };
            let count = match n.trim() {
                "auto-fill" | "auto-fit" => {
                    let min = match t {
                        Track::Px(w) | Track::MinMax(w, _) => w,
                        Track::Fr(_) => 0.0,
                    };
                    auto_fill(avail, min, gap).0
                }
                k => k.parse().unwrap_or(1),
            };
            out.extend(std::iter::repeat_n(t, count));
        } else if let Some(t) = one(&p) {
            out.push(t);
        }
    }
    out
}

/// The max-content border-box width of a box (its width when nothing wraps).
pub fn max_content(b: &LBox<'_>) -> f64 {
    intrinsic(b, false)
}

/// The min-content border-box width of a box (every soft wrap taken).
pub fn min_content(b: &LBox<'_>) -> f64 {
    intrinsic(b, true)
}

fn intrinsic(b: &LBox<'_>, min: bool) -> f64 {
    let s = &b.style;
    if let Some(w) = s.width {
        return clamp_w(s, w);
    }
    let kids = b.children.iter().map(|c| intrinsic(c, min) + mh(&c.style));
    let content = if let Some(m) = b.measure {
        m(if min { Some(0.0) } else { None }).0
    } else {
        match display(s) {
            #[cfg(feature = "flow-flex")]
            "flex" if !is_col(s) => {
                let n = b.children.len();
                let gaps = col_gap(s) * n.saturating_sub(1) as f64;
                if min && is(&s.flex_wrap, "wrap") {
                    kids.fold(0.0, f64::max)
                } else {
                    kids.sum::<f64>() + gaps
                }
            }
            #[cfg(feature = "flow-grid")]
            "grid" => {
                let tracks = parse_tracks(s.grid_template_columns.as_deref().unwrap_or("1fr"), 0.0, col_gap(s));
                let n = tracks.len().max(1);
                let widest = b.children.iter().map(|c| intrinsic(c, min) + mh(&c.style)).fold(0.0, f64::max);
                let mut sum = col_gap(s) * (n - 1) as f64;
                for t in &tracks {
                    sum += match *t {
                        Track::Px(w) => w,
                        Track::MinMax(m, _) => m.max(widest),
                        Track::Fr(_) => widest,
                    };
                }
                sum
            }
            _ => kids.fold(0.0, f64::max),
        }
    };
    clamp_w(s, content + hz(s))
}

/// Lay out `b` at `(x, y)` in a containing block `avail` wide (its border box fills it
/// unless `width` says otherwise). Coordinates are absolute.
pub fn layout(b: &LBox<'_>, x: f64, y: f64, avail: f64) -> Laid {
    let w = clamp_w(&b.style, b.style.width.unwrap_or(avail - mh(&b.style)));
    lay(b, x + b.style.mar()[3], y + b.style.mar()[0], w, None)
}

#[cfg(any(feature = "flow-flex", feature = "flow-grid"))]
/// Height of `b`'s border box at border-box width `w`.
fn height_at(b: &LBox<'_>, w: f64) -> f64 {
    lay(b, 0.0, 0.0, w, None).rect.height
}

#[cfg(feature = "flow-flex")]
fn justify_of(s: &Style) -> Justify {
    match s.justify_content.as_deref() {
        Some("flex-end") | Some("end") => Justify::End,
        Some("center") => Justify::Center,
        Some("space-between") => Justify::SpaceBetween,
        _ => Justify::Start,
    }
}

#[cfg(any(feature = "flow-flex", feature = "flow-grid"))]
fn align_of<'s>(parent: &'s Style, child: &'s Style) -> &'s str {
    match child.align_self.as_deref() {
        Some(a) if a != "auto" => a,
        _ => parent.align_items.as_deref().unwrap_or("stretch"),
    }
}

/// Lay out with a known border-box width `w`; `forced_h` = a height imposed by the
/// parent (stretch, flex resolution).
fn lay(b: &LBox<'_>, x: f64, y: f64, w: f64, forced_h: Option<f64>) -> Laid {
    let s = &b.style;
    let p = s.pad();
    let bw = bw(s);
    let (cx, cy) = (x + p[3] + bw, y + p[0] + bw);
    let cw = (w - hz(s)).max(0.0);
    let def_h = forced_h.or(s.height.map(|h| clamp_h(s, h)));
    #[cfg_attr(not(feature = "flow-flex"), allow(unused_variables))]
    let ch_def = def_h.map(|h| (h - vt(s)).max(0.0));
    let (children, content_h) = if let Some(m) = b.measure {
        (Vec::new(), m(Some(cw)).1)
    } else {
        match display(s) {
            #[cfg(feature = "flow-flex")]
            "flex" if is_col(s) => flex_col(b, cx, cy, cw, ch_def),
            #[cfg(feature = "flow-flex")]
            "flex" => flex_rowbox(b, cx, cy, cw, ch_def),
            #[cfg(feature = "flow-grid")]
            "grid" => grid_box(b, cx, cy, cw),
            _ => block(b, cx, cy, cw),
        }
    };
    let h = def_h.unwrap_or_else(|| clamp_h(s, content_h + vt(s)));
    Laid { rect: Rect::new(x, y, w, h), children }
}

fn block(b: &LBox<'_>, cx: f64, cy: f64, cw: f64) -> (Vec<Laid>, f64) {
    let mut yy = 0.0;
    let mut prev_mb: Option<f64> = None;
    let mut out = Vec::new();
    for c in &b.children {
        let m = c.style.mar();
        // Adjacent sibling margins collapse (the larger one wins).
        let top = match prev_mb {
            Some(pm) => pm.max(m[0]) - pm,
            None => m[0],
        };
        yy += top;
        let w = clamp_w(&c.style, c.style.width.unwrap_or(cw - m[1] - m[3]));
        let l = lay(c, cx + m[3], cy + yy, w, None);
        yy += l.rect.height + m[2];
        prev_mb = Some(m[2]);
        out.push(l);
    }
    (out, yy)
}

#[cfg(feature = "flow-flex")]
/// A flex item's hypothetical main size and its clamps (outer = margins included).
fn item(c: &LBox<'_>, main_col: bool, cross_w: f64) -> FlexItem {
    let s = &c.style;
    let (basis, min, max) = if main_col {
        let w = s.width.unwrap_or(cross_w);
        let content = height_at(c, w);
        let basis = s.flex_basis.or(s.height).unwrap_or(content);
        (basis, s.min_height.unwrap_or(if s.height.is_some() { 0.0 } else { content.min(basis) }), s.max_height.unwrap_or(f64::INFINITY))
    } else {
        let mc = min_content(c);
        let basis = s.flex_basis.or(s.width).unwrap_or_else(|| max_content(c));
        // `min-width: auto` = min-content, capped by a specified width.
        let auto_min = if let Some(w) = s.width { mc.min(w) } else { mc };
        (basis, s.min_width.unwrap_or(auto_min), s.max_width.unwrap_or(f64::INFINITY))
    };
    let m = if main_col { mv(s) } else { mh(s) };
    FlexItem { basis: basis + m, grow: s.flex_grow.unwrap_or(0.0), shrink: s.flex_shrink.unwrap_or(1.0), min: min + m, max: max + m }
}

#[cfg(feature = "flow-flex")]
fn flex_rowbox(b: &LBox<'_>, cx: f64, cy: f64, cw: f64, ch: Option<f64>) -> (Vec<Laid>, f64) {
    let s = &b.style;
    let wrap = is(&s.flex_wrap, "wrap");
    let items: Vec<FlexItem> = b.children.iter().map(|c| item(c, false, cw)).collect();
    // Break into lines by hypothetical outer sizes.
    let mut lines: Vec<Vec<usize>> = vec![vec![]];
    let mut used = 0.0;
    for (i, it) in items.iter().enumerate() {
        let hyp = it.basis.clamp(it.min, it.max);
        let cur = lines.last_mut().unwrap();
        if wrap && !cur.is_empty() && used + col_gap(s) + hyp > cw + 1e-9 {
            lines.push(vec![i]);
            used = hyp;
        } else {
            used += if cur.is_empty() { hyp } else { col_gap(s) + hyp };
            cur.push(i);
        }
    }
    let mut out: Vec<Option<Laid>> = (0..b.children.len()).map(|_| None).collect();
    lines.retain(|l| !l.is_empty());
    // Main sizes and natural cross sizes per line.
    let solved: Vec<(Vec<(f64, f64)>, Vec<(f64, f64)>, f64)> = lines
        .iter()
        .map(|line| {
            let its: Vec<FlexItem> = line.iter().map(|&i| items[i]).collect();
            let main = flex_row(cw, &its, col_gap(s), justify_of(s));
            let sizes: Vec<(f64, f64)> = line
                .iter()
                .zip(&main)
                .map(|(&i, &(_, ow))| {
                    let c = &b.children[i];
                    let w = ow - mh(&c.style);
                    (w, c.style.height.unwrap_or_else(|| height_at(c, w)))
                })
                .collect();
            let lh = line.iter().zip(&sizes).map(|(&i, &(_, h))| h + mv(&b.children[i].style)).fold(0.0, f64::max);
            (main, sizes, lh)
        })
        .collect();
    // A definite cross size: one line takes all of it; several lines share the
    // leftover (`align-content: normal` stretches lines).
    let natural: f64 = solved.iter().map(|l| l.2).sum::<f64>() + row_gap(s) * solved.len().saturating_sub(1) as f64;
    let extra = match ch {
        Some(h) if solved.len() == 1 => h - natural,
        Some(h) if h > natural => h - natural,
        _ => 0.0,
    } / solved.len().max(1) as f64;
    let mut yy = 0.0;
    for (li, (line, (main, sizes, lh))) in lines.iter().zip(&solved).enumerate() {
        if li > 0 {
            yy += row_gap(s);
        }
        let line_h = lh + extra;
        for ((&i, &(ox, _)), &(w, h)) in line.iter().zip(main).zip(sizes) {
            let c = &b.children[i];
            let m = c.style.mar();
            let (hh, dy) = match align_of(s, &c.style) {
                "stretch" if c.style.height.is_none() => (clamp_h(&c.style, line_h - mv(&c.style)), 0.0),
                "center" => (h, (line_h - h - mv(&c.style)) / 2.0),
                "flex-end" | "end" => (h, line_h - h - mv(&c.style)),
                _ => (h, 0.0),
            };
            out[i] = Some(lay(c, cx + ox + m[3], cy + yy + m[0] + dy, w, Some(hh)));
        }
        yy += line_h;
    }
    (out.into_iter().flatten().collect(), ch.unwrap_or(yy))
}

#[cfg(feature = "flow-flex")]
fn flex_col(b: &LBox<'_>, cx: f64, cy: f64, cw: f64, ch: Option<f64>) -> (Vec<Laid>, f64) {
    let s = &b.style;
    // Cross (width) first: stretch fills, otherwise fit-content.
    let widths: Vec<f64> = b
        .children
        .iter()
        .map(|c| {
            let avail = cw - mh(&c.style);
            let w = match (c.style.width, align_of(s, &c.style)) {
                (Some(w), _) => w,
                (None, "stretch") => avail,
                (None, _) => max_content(c).min(avail),
            };
            clamp_w(&c.style, w)
        })
        .collect();
    let items: Vec<FlexItem> = b.children.iter().zip(&widths).map(|(c, &w)| item(c, true, w)).collect();
    let main = match ch {
        Some(h) => flex_row(h, &items, row_gap(s), justify_of(s)),
        None => flex_row(f64::NEG_INFINITY.max(items.iter().map(|i| i.basis.clamp(i.min, i.max)).sum::<f64>() + row_gap(s) * items.len().saturating_sub(1) as f64), &items, row_gap(s), Justify::Start),
    };
    let mut out = Vec::new();
    let mut end: f64 = 0.0;
    for ((c, &(oy, oh)), &w) in b.children.iter().zip(&main).zip(&widths) {
        let m = c.style.mar();
        let dx = match align_of(s, &c.style) {
            "center" => (cw - w - mh(&c.style)) / 2.0,
            "flex-end" | "end" => cw - w - mh(&c.style),
            _ => 0.0,
        };
        let l = lay(c, cx + m[3] + dx, cy + oy + m[0], w, Some(oh - mv(&c.style)));
        end = end.max(oy + oh);
        out.push(l);
    }
    (out, ch.unwrap_or(end))
}

#[cfg(feature = "flow-grid")]
fn grid_box(b: &LBox<'_>, cx: f64, cy: f64, cw: f64) -> (Vec<Laid>, f64) {
    let s = &b.style;
    let tracks = parse_tracks(s.grid_template_columns.as_deref().unwrap_or("1fr"), cw, col_gap(s));
    let cols = grid_tracks(cw, &tracks, col_gap(s));
    let n = cols.len().max(1);
    let mut out = Vec::new();
    let mut yy = 0.0;
    for (ri, row) in b.children.chunks(n).enumerate() {
        if ri > 0 {
            yy += row_gap(s);
        }
        let hs: Vec<f64> = row
            .iter()
            .zip(&cols)
            .map(|(c, &(_, tw))| c.style.height.unwrap_or_else(|| height_at(c, tw - mh(&c.style))) + mv(&c.style))
            .collect();
        let rh = hs.iter().copied().fold(0.0, f64::max);
        for ((c, &(tx, tw)), &h) in row.iter().zip(&cols).zip(&hs) {
            let m = c.style.mar();
            let w = c.style.width.unwrap_or(tw - mh(&c.style));
            let hh = match align_of(s, &c.style) {
                "stretch" if c.style.height.is_none() => rh - mv(&c.style),
                _ => h - mv(&c.style),
            };
            let dy = match align_of(s, &c.style) {
                "center" => (rh - h) / 2.0,
                "end" | "flex-end" => rh - h,
                _ => 0.0,
            };
            out.push(lay(c, cx + tx + m[3], cy + yy + m[0] + dy, w, Some(hh)));
        }
        yy += rh;
    }
    (out, yy)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(p, q)| (p.0 - q.0).abs() < 1e-6 && (p.1 - q.1).abs() < 1e-6)
    }

    #[test]
    fn flex_grow_and_justify() {
        let r = flex_row(300.0, &[FlexItem::fixed(50.0), FlexItem::fill(), FlexItem::fixed(70.0)], 10.0, Justify::Start);
        assert!(close(&r, &[(0.0, 50.0), (60.0, 160.0), (230.0, 70.0)]));
        let r = flex_row(100.0, &[FlexItem::fixed(20.0), FlexItem::fixed(20.0)], 0.0, Justify::End);
        assert!(close(&r, &[(60.0, 20.0), (80.0, 20.0)]));
        let r = flex_row(100.0, &[FlexItem::fixed(20.0), FlexItem::fixed(20.0)], 0.0, Justify::Center);
        assert!(close(&r, &[(30.0, 20.0), (50.0, 20.0)]));
        let r = flex_row(100.0, &[FlexItem::fixed(20.0), FlexItem::fixed(20.0), FlexItem::fixed(20.0)], 0.0, Justify::SpaceBetween);
        assert!(close(&r, &[(0.0, 20.0), (40.0, 20.0), (80.0, 20.0)]));
    }

    #[test]
    fn flex_shrink_respects_min_and_max() {
        // two auto items overflow by 50: shrink proportional to basis
        let r = flex_row(150.0, &[FlexItem::auto(100.0), FlexItem::auto(100.0)], 0.0, Justify::Start);
        assert!(close(&r, &[(0.0, 75.0), (75.0, 75.0)]));
        // one item with a min freezes, the other absorbs the rest
        let a = FlexItem { min: 90.0, ..FlexItem::auto(100.0) };
        let r = flex_row(150.0, &[a, FlexItem::auto(100.0)], 0.0, Justify::Start);
        assert!(close(&r, &[(0.0, 90.0), (90.0, 60.0)]));
        // max clamps growth; the leftover goes to the other grower
        let capped = FlexItem { max: 40.0, ..FlexItem::fill() };
        let r = flex_row(200.0, &[capped, FlexItem::fill()], 0.0, Justify::Start);
        assert!(close(&r, &[(0.0, 40.0), (40.0, 160.0)]));
    }

    #[test]
    fn wrap_breaks_lines() {
        let (pos, h) = flex_wrap(100.0, &[(40.0, 24.0), (40.0, 24.0), (40.0, 24.0)], 5.0, 5.0);
        assert_eq!(pos, vec![(0.0, 0.0), (45.0, 0.0), (0.0, 29.0)]);
        assert_eq!(h, 53.0);
        // an oversize first box still sits on line one
        assert_eq!(flex_wrap(10.0, &[(40.0, 20.0)], 5.0, 5.0), (vec![(0.0, 0.0)], 20.0));
    }

    #[test]
    fn grid_fr_and_minmax() {
        let r = grid_tracks(310.0, &[Track::Fr(2.0), Track::Fr(1.0)], 10.0);
        assert!(close(&r, &[(0.0, 200.0), (210.0, 100.0)]));
        let r = grid_tracks(100.0, &[Track::MinMax(140.0, 1.0), Track::Px(50.0)], 0.0);
        assert!(close(&r, &[(0.0, 140.0), (140.0, 50.0)]));
        assert_eq!(auto_fill(800.0, 380.0, 10.0), (2, 395.0));
        assert_eq!(auto_fill(300.0, 380.0, 10.0), (1, 300.0));
    }

    #[cfg(all(feature = "flow-flex", feature = "flow-grid"))]
    fn words<'a>(text: &'a str) -> impl Fn(Option<f64>) -> (f64, f64) + 'a {
        // 7px per char, 10px per space, 20px lines: a deterministic text measurer.
        move |limit| {
            let ws: Vec<f64> = text.split(' ').map(|w| w.len() as f64 * 7.0).collect();
            let (mut lines, mut cur, mut widest) = (1.0, 0.0f64, 0.0f64);
            for (i, w) in ws.iter().enumerate() {
                let add = if i == 0 || cur == 0.0 { *w } else { cur + 10.0 + w };
                if cur > 0.0 && limit.is_some_and(|l| add > l) {
                    lines += 1.0;
                    widest = widest.max(cur);
                    cur = *w;
                } else {
                    cur = add;
                }
            }
            (widest.max(cur), lines * 20.0)
        }
    }

    #[test]
    #[cfg(all(feature = "flow-flex", feature = "flow-grid"))]
    fn text_intrinsic_sizes_and_shrink_to_min_content() {
        let a = words("aaaa bb cccccc");
        let t = LBox::css("").text(&a);
        assert_eq!((min_content(&t), max_content(&t)), (42.0, 28.0 + 10.0 + 14.0 + 10.0 + 42.0));
        // A 70px row: the text item shrinks but not below its min-content (42), it wraps.
        let b = words("x");
        let row = LBox::css("display:flex;width:70px").child(LBox::css("").text(&a)).child(LBox::css("").text(&b));
        let l = layout(&row, 0.0, 0.0, 500.0);
        assert_eq!(l.children[0].rect.width, 63.0);
        assert_eq!(l.children[0].rect.height, 40.0);
        assert_eq!(l.rect.height, 40.0, "the row is as tall as its wrapped text");
    }

    #[test]
    #[cfg(all(feature = "flow-flex", feature = "flow-grid"))]
    fn golden_layouts_match_chrome() {
        let raw = include_str!("../../tests/golden/flow.json");
        let cases: serde_json::Value = serde_json::from_str(raw).unwrap();
        fn build(v: &serde_json::Value) -> LBox<'static> {
            let mut b = LBox::css(v["style"].as_str().unwrap_or(""));
            for c in v["children"].as_array().into_iter().flatten() {
                b.children.push(build(c));
            }
            b
        }
        let mut bad = Vec::new();
        for case in cases.as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let root = build(&case["tree"]);
            let got = layout(&root, 0.0, 0.0, case["viewport"].as_f64().unwrap()).flatten();
            let want: Vec<Vec<f64>> = serde_json::from_value(case["chrome"].clone()).unwrap();
            assert_eq!(got.len(), want.len(), "{name}: box count");
            for (i, (g, w)) in got.iter().zip(&want).enumerate() {
                let d = [g.x - w[0], g.y - w[1], g.width - w[2], g.height - w[3]];
                if d.iter().any(|x| x.abs() > 0.5) {
                    bad.push(format!("{name} box {i}: uzor {:?} chrome {:?}", (g.x, g.y, g.width, g.height), w));
                }
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }
}
