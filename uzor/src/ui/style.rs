//! Declarative style layer (DOM-like roadmap item 1): a small CSS subset that widgets and
//! apps resolve instead of hard-coding colours and metrics.
//!
//! ```
//! use uzor::ui::style::{Sheet, Node, State};
//! let sheet = Sheet::parse(":root{--acc:#3b82f6} .btn{padding:5px 12px;border-radius:8px}
//!     .btn:hover{background:#26262b} #send{background:var(--acc)}").unwrap();
//! let s = sheet.resolve(&Node::new("button").id("send").class("btn").state(State::HOVER), None);
//! assert_eq!(s.background.as_deref(), Some("#3b82f6")); // id beats :hover
//! assert_eq!(s.padding, Some([5.0, 12.0, 5.0, 12.0]));
//! ```
//!
//! Supported: compound selectors (`kind`, `#id`, `.class`, `:hover`, `:active`,
//! `:focus-visible`, `:disabled`, `*`), selector lists, `:root` custom properties with
//! `var(--name[, fallback])`, and cascade by specificity then source order. Inherited
//! properties (`color`, `font-*`, `line-height`) come from the parent's resolved style.

use std::collections::HashMap;

/// Interaction state flags a selector can test.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State(u8);

impl State {
    pub const NONE: State = State(0);
    pub const HOVER: State = State(1);
    pub const ACTIVE: State = State(2);
    pub const FOCUS_VISIBLE: State = State(4);
    pub const DISABLED: State = State(8);
    /// `:checked` (a switch, checkbox or radio that is on).
    pub const CHECKED: State = State(16);
    /// `:focus` (the element holds keyboard focus, however it got it).
    pub const FOCUS: State = State(32);
    pub fn contains(self, o: State) -> bool {
        self.0 & o.0 == o.0
    }
}

impl std::ops::BitOr for State {
    type Output = State;
    fn bitor(self, o: State) -> State {
        State(self.0 | o.0)
    }
}

/// What a selector matches against: a widget's kind, id, classes and state.
#[derive(Clone, Debug, Default)]
pub struct Node<'a> {
    pub kind: &'a str,
    pub id: Option<&'a str>,
    pub classes: Vec<&'a str>,
    pub state: State,
}

impl<'a> Node<'a> {
    pub fn new(kind: &'a str) -> Self {
        Node { kind, ..Default::default() }
    }
    /// A node from a selector-like spec: `span.tq-pill.ok#x:hover`.
    pub fn parse(spec: &'a str) -> Self {
        let head = spec.find(['#', '.', ':']).unwrap_or(spec.len());
        let mut n = Node::new(&spec[..head]);
        let mut rest = &spec[head..];
        while !rest.is_empty() {
            let (sig, body) = rest.split_at(1);
            let end = body.find(['#', '.', ':']).unwrap_or(body.len());
            let name = &body[..end];
            match sig {
                "#" => n.id = Some(name),
                "." => n.classes.push(name),
                _ => {
                    n.state = n.state
                        | match name {
                            "hover" => State::HOVER,
                            "active" => State::ACTIVE,
                            "focus-visible" => State::FOCUS_VISIBLE,
                            "disabled" => State::DISABLED,
                            "checked" => State::CHECKED,
                            "focus" => State::FOCUS,
                            _ => State::NONE,
                        }
                }
            }
            rest = &body[end..];
        }
        n
    }
    pub fn id(mut self, id: &'a str) -> Self {
        self.id = Some(id);
        self
    }
    pub fn class(mut self, c: &'a str) -> Self {
        self.classes.push(c);
        self
    }
    pub fn state(mut self, s: State) -> Self {
        self.state = self.state | s;
        self
    }
}

/// A resolved style. `None` = not set (the widget's default applies).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    pub color: Option<String>,
    pub background: Option<String>,
    pub border_color: Option<String>,
    pub border_width: Option<f64>,
    /// top, right, bottom, left
    pub padding: Option<[f64; 4]>,
    pub margin: Option<[f64; 4]>,
    pub border_radius: Option<f64>,
    pub font_size: Option<f64>,
    pub font_weight: Option<u32>,
    /// px (a unitless multiplier resolves against the font size).
    pub line_height: Option<f64>,
    pub font_family: Option<String>,
    pub opacity: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    /// `gap` (both axes) / `column-gap`.
    pub gap: Option<f64>,
    /// `row-gap` (the first value of a two-value `gap`); `None` = same as `gap`.
    pub row_gap: Option<f64>,
    pub min_width: Option<f64>,
    pub max_width: Option<f64>,
    pub min_height: Option<f64>,
    pub max_height: Option<f64>,
    /// `block` (default), `flex`, `grid`, `none`.
    pub display: Option<String>,
    /// `row` (default) / `column`.
    pub flex_direction: Option<String>,
    pub flex_wrap: Option<String>,
    pub justify_content: Option<String>,
    pub align_items: Option<String>,
    pub align_self: Option<String>,
    pub flex_grow: Option<f64>,
    pub flex_shrink: Option<f64>,
    /// `None` = `auto`.
    pub flex_basis: Option<f64>,
    /// Raw `grid-template-columns` (parsed by `ui::flow`).
    pub grid_template_columns: Option<String>,
    pub box_shadow: Option<String>,
    pub cursor: Option<String>,
    pub text_align: Option<String>,
    /// `white-space` (inherited): `normal` lets [`crate::ui::dom`] text wrap; unset or
    /// `nowrap` keeps one line (ellipsized when it does not fit).
    pub white_space: Option<String>,
    /// `line-clamp` / `-webkit-line-clamp`: wrapping text keeps at most this many lines,
    /// the last one ends in an ellipsis.
    pub line_clamp: Option<u16>,
    /// Custom properties (`--name: value`) set on the element or inherited, as in CSS.
    pub custom: std::collections::BTreeMap<String, String>,
    /// `transition` entries (see [`crate::ui::motion`]).
    pub transitions: Vec<super::motion::Transition>,
    /// `animation` entries naming `@keyframes` (see [`crate::ui::motion`]).
    pub animations: Vec<super::motion::Animation>,
    /// `position`: `static` (unset), `relative`, `absolute`. An absolute box leaves the
    /// flow and is placed against its nearest positioned ancestor's padding box
    /// (see [`crate::ui::flow`]); a relative one is shifted by its insets.
    pub position: Option<String>,
    /// `top` (px), with `right` / `bottom` / `left` the insets of a positioned box.
    pub top: Option<f64>,
    pub right: Option<f64>,
    pub bottom: Option<f64>,
    pub left: Option<f64>,
    /// `z-index` of a positioned box: higher paints (and hit-tests) on top.
    pub z_index: Option<i32>,
    /// Per-corner radii `[horizontal, vertical]` in the order top-left, top-right,
    /// bottom-right, bottom-left, set by a multi-value or elliptical (`a / b`)
    /// `border-radius` or the `border-*-*-radius` longhands. `None`: every corner is
    /// [`Style::border_radius`]. Read both through [`Style::radii`].
    pub border_radii: Option<[[f64; 2]; 4]>,
    /// Per-side border widths (top, right, bottom, left), set by a multi-value
    /// `border-width` or the `border-<side>[-width]` longhands. `None`: every side is
    /// [`Style::border_width`]. Read both through [`Style::border_edges`].
    pub border_widths: Option<[f64; 4]>,
    /// Per-side border colours (top, right, bottom, left); `None` sides and a `None`
    /// list fall back to [`Style::border_color`]. Read through [`Style::border_side_color`].
    pub border_colors: Option<[Option<String>; 4]>,
    /// `font-style` (inherited): `italic` / `oblique` slant the font.
    pub font_style: Option<String>,
    /// `overflow-wrap` (inherited): `anywhere` / `break-word` let inline runs
    /// ([`crate::ui::dom::El::runs`]) break inside a word that does not fit a line.
    pub overflow_wrap: Option<String>,
    /// `shape-rendering` (inherited, as in SVG): `geometricPrecision` draws an element's
    /// [`crate::ui::dom::El::svg`] icon exactly (no pixel-grid snapping of its origin or
    /// strokes); unset / `auto` / `crispEdges` snap for crispness.
    pub shape_rendering: Option<String>,
    /// `width: N%`: a fraction (0.5 for `50%`) of the containing block's width, used when
    /// [`Style::width`] is unset. See [`crate::ui::flow`].
    pub width_pct: Option<f64>,
    /// `height: N%`: a fraction of the containing block's height when that height is
    /// definite; otherwise the height is `auto`.
    pub height_pct: Option<f64>,
    /// `max-width: N%`: a fraction of the containing block's width.
    pub max_width_pct: Option<f64>,
    /// `min-width: N%`: a fraction of the containing block's width.
    pub min_width_pct: Option<f64>,
    /// `aspect-ratio` as width / height (`16 / 9` → 1.777…): a box with an auto height
    /// takes `width / ratio`; with only a height, an auto width takes `height × ratio`.
    pub aspect_ratio: Option<f64>,
    /// `border-style` (`solid` when unset): `dashed` and `dotted` stroke the border with a
    /// dash pattern, `none` / `hidden` draw no border.
    pub border_style: Option<String>,
}

impl Style {
    /// A custom property of the element (`--name`), e.g. a widget-specific knob.
    pub fn custom(&self, name: &str) -> Option<&str> {
        self.custom.get(name).map(String::as_str)
    }
    /// The custom property for a [`styled!`] field: `pad_x` reads `--pad-x`. One shared body,
    /// so each styled field costs a call rather than its own name rewrite.
    #[doc(hidden)]
    #[inline(never)]
    pub fn field_css(&self, field: &str) -> Option<&str> {
        self.custom(&format!("--{}", field.replace('_', "-")))
    }
    /// The colour, or `fallback` when unset.
    pub fn color_or<'a>(&'a self, fallback: &'a str) -> &'a str {
        self.color.as_deref().unwrap_or(fallback)
    }
    /// The background, or `fallback` when unset.
    pub fn background_or<'a>(&'a self, fallback: &'a str) -> &'a str {
        self.background.as_deref().unwrap_or(fallback)
    }
    /// Padding (top, right, bottom, left), zero when unset.
    pub fn pad(&self) -> [f64; 4] {
        self.padding.unwrap_or_default()
    }
    /// Margin (top, right, bottom, left), zero when unset.
    pub fn mar(&self) -> [f64; 4] {
        self.margin.unwrap_or_default()
    }
    /// Font size in px (16 when unset, the CSS initial value).
    pub fn px(&self) -> f64 {
        self.font_size.unwrap_or(16.0)
    }
    /// Line height in px (`normal` ≈ 1.2 × font size when unset).
    pub fn lh(&self) -> f64 {
        self.line_height.unwrap_or(self.px() * 1.2)
    }
    /// Font weight (400 when unset).
    pub fn weight(&self) -> u32 {
        self.font_weight.unwrap_or(400)
    }
    /// Border widths (top, right, bottom, left): the per-side widths, else
    /// `border-width` on every side, else zero.
    pub fn border_edges(&self) -> [f64; 4] {
        self.border_widths.unwrap_or([self.border_width.unwrap_or(0.0); 4])
    }
    /// The border colour of side `i` (0 top, 1 right, 2 bottom, 3 left).
    pub fn border_side_color(&self, i: usize) -> Option<&str> {
        self.border_colors.as_ref().and_then(|c| c.get(i)?.as_deref()).or(self.border_color.as_deref())
    }
    /// Corner radii `[horizontal, vertical]` (top-left, top-right, bottom-right,
    /// bottom-left): the per-corner radii, else `border-radius` on every corner.
    pub fn radii(&self) -> [[f64; 2]; 4] {
        self.border_radii.unwrap_or_else(|| {
            let r = self.border_radius.unwrap_or(0.0);
            [[r, r]; 4]
        })
    }
    /// `position` is `relative`, `absolute`, `fixed` or `sticky` (a containing block
    /// for absolutely positioned descendants).
    pub fn positioned(&self) -> bool {
        matches!(self.position.as_deref(), Some("relative" | "absolute" | "fixed" | "sticky"))
    }
    /// `position: absolute` (or `fixed`): out of the flow.
    pub fn out_of_flow(&self) -> bool {
        matches!(self.position.as_deref(), Some("absolute" | "fixed"))
    }
    /// `position: fixed`: placed against the viewport and left where it is when the
    /// tree scrolls ([`crate::ui::dom::Dom::moved`]).
    pub fn fixed(&self) -> bool {
        self.position.as_deref() == Some("fixed")
    }
    /// The border is drawn (`border-style` is not `none` / `hidden`).
    pub fn border_drawn(&self) -> bool {
        !matches!(self.border_style.as_deref(), Some("none" | "hidden"))
    }
    /// The dash pattern of the border for a stroke `w` wide: `dashed` 3w on 3w off (CSS
    /// does not fix the ratio; this is what Chromium draws for thin borders), `dotted`
    /// w on w off, otherwise solid (`None`).
    pub fn border_dash(&self, w: f64) -> Option<[f64; 2]> {
        let w = w.max(1.0);
        match self.border_style.as_deref() {
            Some("dashed") => Some([3.0 * w, 3.0 * w]),
            Some("dotted") => Some([w, w]),
            _ => None,
        }
    }
}

/// A string with `'static` lifetime for the `&'static str` fields of widget style
/// structs. Each distinct value is leaked once and reused, so a stable sheet costs a
/// bounded amount of memory.
pub fn intern(s: &str) -> &'static str {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};
    static POOL: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    let mut pool = POOL.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    if let Some(v) = pool.get(s) {
        return v;
    }
    let v: &'static str = Box::leak(s.to_string().into_boxed_str());
    pool.insert(v);
    v
}

/// A parse error with the offending text.
#[derive(Clone, Debug, PartialEq)]
pub struct StyleError(pub String);

impl std::fmt::Display for StyleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "style: {}", self.0)
    }
}

impl std::error::Error for StyleError {}

/// A complex selector: compounds joined by descendant (` `) or child (`>`) combinators,
/// the last compound is the subject.
#[derive(Clone, Debug, PartialEq)]
struct Selector {
    parts: Vec<Compound>,
    /// `child[i]`: the combinator between `parts[i]` and `parts[i + 1]` is `>`.
    child: Vec<bool>,
}

impl Selector {
    fn parse(s: &str) -> Result<Selector, StyleError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(StyleError("empty selector".into()));
        }
        let spaced = s.replace('>', " > ");
        let mut parts = Vec::new();
        let mut child = Vec::new();
        let mut pending_child = false;
        for tok in spaced.split_whitespace() {
            if tok == ">" {
                if parts.is_empty() || pending_child {
                    return Err(StyleError(format!("unsupported selector `{s}`")));
                }
                pending_child = true;
                continue;
            }
            if !parts.is_empty() {
                child.push(pending_child);
            }
            pending_child = false;
            parts.push(Compound::parse(tok)?);
        }
        if pending_child || parts.is_empty() {
            return Err(StyleError(format!("unsupported selector `{s}`")));
        }
        Ok(Selector { parts, child })
    }
    /// CSS specificity (ids, classes + pseudo-classes, kinds), summed over the compounds.
    fn specificity(&self) -> (u32, u32, u32) {
        self.parts.iter().map(Compound::specificity).fold((0, 0, 0), |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2))
    }
    /// A one-compound selector matching `n` (a complex one needs [`Selector::matches_in`]).
    fn matches(&self, n: &Node<'_>) -> bool {
        self.parts.len() == 1 && self.parts[0].matches(n)
    }
    /// Match `n` whose ancestors are `up` (root first, parent last).
    fn matches_in(&self, n: &Node<'_>, up: &[Node<'_>]) -> bool {
        let last = self.parts.len() - 1;
        self.parts[last].matches(n) && Self::rest(&self.parts[..last], &self.child, up)
    }
    /// `parts` (the compounds left of a matched element) against the ancestors `up`;
    /// `child[parts.len() - 1]` joins the last of `parts` to the element matched before.
    fn rest(parts: &[Compound], child: &[bool], up: &[Node<'_>]) -> bool {
        let Some((c, before)) = parts.split_last() else { return true };
        if child[parts.len() - 1] {
            // `>`: the parent must match.
            match up.split_last() {
                Some((p, above)) => c.matches(p) && Self::rest(before, child, above),
                None => false,
            }
        } else {
            // descendant: any ancestor, nearest first, with backtracking.
            (0..up.len()).rev().any(|i| c.matches(&up[i]) && Self::rest(before, child, &up[..i]))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Compound {
    kind: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    state: State,
}

impl Compound {
    fn parse(s: &str) -> Result<Compound, StyleError> {
        let mut sel = Compound { kind: None, id: None, classes: vec![], state: State::NONE };
        let mut rest = s;
        let head_end = rest.find(['#', '.', ':']).unwrap_or(rest.len());
        let head = &rest[..head_end];
        if !head.is_empty() && head != "*" {
            sel.kind = Some(head.to_string());
        }
        rest = &rest[head_end..];
        while !rest.is_empty() {
            let (sig, body) = rest.split_at(1);
            let end = body.find(['#', '.', ':']).unwrap_or(body.len());
            let name = &body[..end];
            if name.is_empty() {
                return Err(StyleError(format!("empty name in `{s}`")));
            }
            match sig {
                "#" => sel.id = Some(name.to_string()),
                "." => sel.classes.push(name.to_string()),
                _ => {
                    sel.state = sel.state
                        | match name {
                            "hover" => State::HOVER,
                            "active" => State::ACTIVE,
                            "focus-visible" => State::FOCUS_VISIBLE,
                            "disabled" => State::DISABLED,
                            "checked" => State::CHECKED,
                            "focus" => State::FOCUS,
                            o => return Err(StyleError(format!("unsupported pseudo-class `:{o}`"))),
                        }
                }
            }
            rest = &body[end..];
        }
        Ok(sel)
    }
    /// CSS specificity (ids, classes + pseudo-classes, kinds).
    fn specificity(&self) -> (u32, u32, u32) {
        (self.id.is_some() as u32, self.classes.len() as u32 + self.state.0.count_ones(), self.kind.is_some() as u32)
    }
    fn matches(&self, n: &Node<'_>) -> bool {
        self.kind.as_deref().is_none_or(|k| k == n.kind)
            && self.id.as_deref().is_none_or(|i| n.id == Some(i))
            && self.classes.iter().all(|c| n.classes.contains(&c.as_str()))
            && n.state.contains(self.state)
    }
}

#[derive(Clone, Debug)]
struct Rule {
    sel: Selector,
    src: String,
    order: usize,
    decls: Vec<(String, String)>,
}

/// One applied declaration, as reported by [`Sheet::resolve_traced`].
#[derive(Clone, Debug, PartialEq)]
pub struct Applied {
    pub property: String,
    pub value: String,
    pub selector: String,
}

/// Parse an inline `style="..."` declaration list into a [`Style`] (no cascade, no vars).
pub fn inline(decls: &str) -> Style {
    let mut s = Style::default();
    for d in decls.split(';') {
        if let Some((k, v)) = d.split_once(':') {
            apply(&mut s, &k.trim().to_ascii_lowercase(), v.trim());
        }
    }
    s
}

/// A parsed style sheet.
#[derive(Debug, Default)]
pub struct Sheet {
    rules: Vec<Rule>,
    vars: HashMap<String, String>,
    keyframes: HashMap<String, super::motion::Keyframes>,
    cache: std::sync::Mutex<HashMap<String, Style>>,
}

impl Clone for Sheet {
    fn clone(&self) -> Self {
        Sheet { rules: self.rules.clone(), vars: self.vars.clone(), keyframes: self.keyframes.clone(), cache: Default::default() }
    }
}

impl Sheet {
    /// Parse a CSS subset. `/* comments */` are skipped.
    pub fn parse(css: &str) -> Result<Sheet, StyleError> {
        let mut src = String::with_capacity(css.len());
        let mut rest = css;
        while let Some(i) = rest.find("/*") {
            src.push_str(&rest[..i]);
            rest = rest[i..].find("*/").map(|j| &rest[i + j + 2..]).ok_or_else(|| StyleError("unclosed comment".into()))?;
        }
        src.push_str(rest);
        let mut sheet = Sheet::default();
        let mut rest = src.as_str();
        while let Some(open) = rest.find('{') {
            if let Some(name) = rest[..open].trim().strip_prefix("@keyframes") {
                // Nested block: find the brace that closes it.
                let mut depth = 0;
                let close = rest[open..]
                    .char_indices()
                    .find(|&(_, c)| {
                        depth += (c == '{') as i32 - (c == '}') as i32;
                        depth == 0
                    })
                    .map(|(i, _)| open + i)
                    .ok_or_else(|| StyleError("missing `}`".into()))?;
                let kf = super::motion::Keyframes::parse(&rest[open + 1..close]).ok_or_else(|| StyleError(format!("bad @keyframes `{}`", name.trim())))?;
                sheet.keyframes.insert(name.trim().to_string(), kf);
                rest = &rest[close + 1..];
                continue;
            }
            let close = rest[open..].find('}').map(|c| open + c).ok_or_else(|| StyleError("missing `}`".into()))?;
            let sels = &rest[..open];
            let decls: Vec<(String, String)> = rest[open + 1..close]
                .split(';')
                .filter(|d| !d.trim().is_empty())
                .map(|d| {
                    let (k, v) = d.split_once(':').ok_or_else(|| StyleError(format!("bad declaration `{}`", d.trim())))?;
                    Ok((k.trim().to_ascii_lowercase(), v.trim().to_string()))
                })
                .collect::<Result<_, StyleError>>()?;
            for s in sels.split(',') {
                if s.trim() == ":root" {
                    for (k, v) in &decls {
                        if k.starts_with("--") {
                            sheet.vars.insert(k.clone(), v.clone());
                        }
                    }
                    continue;
                }
                let order = sheet.rules.len();
                sheet.rules.push(Rule { sel: Selector::parse(s)?, src: s.trim().to_string(), order, decls: decls.clone() });
            }
            rest = &rest[close + 1..];
        }
        if !rest.trim().is_empty() {
            return Err(StyleError(format!("trailing text `{}`", rest.trim())));
        }
        Ok(sheet)
    }

    /// The `@keyframes` rule called `name`.
    pub fn keyframes(&self, name: &str) -> Option<&super::motion::Keyframes> {
        self.keyframes.get(name)
    }

    /// A custom property (`--name`) with `var()` references resolved.
    pub fn var(&self, name: &str) -> Option<String> {
        self.vars.get(name).map(|v| self.subst(v, 0))
    }

    /// [`Sheet::var`] interned for `&'static str` style fields; `fallback` when unset.
    pub fn var_static(&self, name: &str, fallback: &'static str) -> &'static str {
        self.var(name).map(|v| intern(&v)).unwrap_or(fallback)
    }

    /// The style of a parentless element given as a spec (`span.tq-pill.ok`), cached.
    pub fn el(&self, spec: &str) -> Style {
        if let Some(s) = self.cache.lock().unwrap_or_else(|e| e.into_inner()).get(spec) {
            return s.clone();
        }
        let s = self.resolve(&Node::parse(spec), None);
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).insert(spec.to_string(), s.clone());
        s
    }

    /// [`Sheet::resolve`] for a registered widget: with the `inspector` feature the
    /// matched declarations are kept under `widget` for the inspector panel.
    pub fn resolve_for(&self, widget: &str, n: &Node<'_>, parent: Option<&Style>) -> Style {
        #[cfg(feature = "inspector")]
        {
            let (s, tr) = self.resolve_traced(n, parent);
            trace::record(widget, tr);
            s
        }
        #[cfg(not(feature = "inspector"))]
        {
            let _ = widget;
            self.resolve(n, parent)
        }
    }

    /// [`Sheet::el`] for a registered widget (see [`Sheet::resolve_for`]).
    pub fn el_for(&self, widget: &str, spec: &str) -> Style {
        #[cfg(feature = "inspector")]
        trace::record(widget, self.resolve_traced(&Node::parse(spec), None).1);
        #[cfg(not(feature = "inspector"))]
        let _ = widget;
        self.el(spec)
    }

    fn subst(&self, v: &str, depth: u32) -> String {
        self.subst_in(v, depth, None)
    }

    fn subst_in(&self, v: &str, depth: u32, local: Option<&std::collections::BTreeMap<String, String>>) -> String {
        let Some(i) = v.find("var(") else { return v.to_string() };
        let Some(j) = v[i..].find(')').map(|j| i + j) else { return v.to_string() };
        let inner = &v[i + 4..j];
        let (name, fb) = inner.split_once(',').map(|(a, b)| (a.trim(), Some(b.trim()))).unwrap_or((inner.trim(), None));
        let found = local.and_then(|l| l.get(name)).or_else(|| self.vars.get(name));
        let val = if depth > 8 { None } else { found.map(|x| self.subst_in(x, depth + 1, local)) };
        let val = val.or(fb.map(str::to_string)).unwrap_or_default();
        format!("{}{}{}", &v[..i], val, self.subst_in(&v[j + 1..], depth, local))
    }

    /// Cascade every matching rule (specificity, then source order) over the inherited
    /// properties of `parent`.
    pub fn resolve(&self, n: &Node<'_>, parent: Option<&Style>) -> Style {
        self.resolve_traced(n, parent).0
    }

    /// [`Sheet::resolve`] plus, per applied declaration in cascade order, the property, its
    /// substituted value and the selector of the rule it came from (what an inspector shows).
    pub fn resolve_traced(&self, n: &Node<'_>, parent: Option<&Style>) -> (Style, Vec<Applied>) {
        self.resolve_traced_in(n, &[], parent)
    }

    /// [`Sheet::resolve`] for an element whose ancestors are `up` (root first, parent last),
    /// so descendant (`.a .b`) and child (`.a > .b`) selectors can match. Without
    /// ancestors (`resolve`, `el`) only one-compound selectors apply.
    pub fn resolve_in(&self, n: &Node<'_>, up: &[Node<'_>], parent: Option<&Style>) -> Style {
        self.resolve_traced_in(n, up, parent).0
    }

    /// The sheet has a selector with a combinator (descendant / child), so callers that
    /// do not track ancestors may want to.
    pub fn has_complex(&self) -> bool {
        self.rules.iter().any(|r| r.sel.parts.len() > 1)
    }

    /// [`Sheet::resolve_traced`] with ancestors (see [`Sheet::resolve_in`]).
    pub fn resolve_traced_in(&self, n: &Node<'_>, up: &[Node<'_>], parent: Option<&Style>) -> (Style, Vec<Applied>) {
        let mut trace = Vec::new();
        let mut s = Style::default();
        if let Some(p) = parent {
            s.color = p.color.clone();
            s.font_size = p.font_size;
            s.font_weight = p.font_weight;
            s.font_family = p.font_family.clone();
            s.line_height = p.line_height;
            s.text_align = p.text_align.clone();
            s.white_space = p.white_space.clone();
            s.font_style = p.font_style.clone();
            s.overflow_wrap = p.overflow_wrap.clone();
            s.shape_rendering = p.shape_rendering.clone();
            s.custom = p.custom.clone();
        }
        let mut hits: Vec<&Rule> = self.rules.iter().filter(|r| if up.is_empty() { r.sel.matches(n) } else { r.sel.matches_in(n, up) }).collect();
        crate::sort::sort_by_key(&mut hits, |r| (r.sel.specificity(), r.order));
        // Custom properties first (they cascade, then every var() sees the element's values).
        for r in &hits {
            for (k, v) in &r.decls {
                if k.starts_with("--") {
                    s.custom.insert(k.clone(), v.clone());
                }
            }
        }
        let local = std::mem::take(&mut s.custom);
        // Inserts, not `collect`: a bulk-built map pulls in a whole stable sort of `(String, String)`.
        for (k, v) in &local {
            s.custom.insert(k.clone(), self.subst_in(v, 0, Some(&local)));
        }
        let mut lh_raw: Option<String> = None;
        for r in hits {
            for (k, v) in &r.decls {
                let v = self.subst_in(v, 0, Some(&s.custom));
                if k.starts_with("--") {
                    trace.push(Applied { property: k.clone(), value: v, selector: r.src.clone() });
                    continue;
                }
                if k == "line-height" {
                    lh_raw = Some(v.clone());
                }
                apply(&mut s, k, &v);
                trace.push(Applied { property: k.clone(), value: v, selector: r.src.clone() });
            }
        }
        // A unitless line-height multiplies the final font size.
        if let Some(raw) = lh_raw {
            if let Ok(m) = raw.trim().parse::<f64>() {
                s.line_height = Some(m * s.font_size.unwrap_or(16.0));
            }
        }
        (s, trace)
    }
}

/// The `border-style` keywords the `border` shorthands recognise.
const BORDER_STYLES: [&str; 9] = ["solid", "dashed", "dotted", "none", "hidden", "double", "groove", "ridge", "inset"];

/// A length that may be a percentage: `(px, fraction)`, at most one set (`auto` → neither).
fn len(v: &str) -> (Option<f64>, Option<f64>) {
    match v.trim().strip_suffix('%') {
        Some(p) => (None, p.trim().parse::<f64>().ok().map(|p| p / 100.0)),
        None => (px(v), None),
    }
}

/// `aspect-ratio`: `w / h`, a single number, or `auto` (optionally with a ratio, which wins).
fn ratio(v: &str) -> Option<f64> {
    let v = v.trim().trim_start_matches("auto").trim();
    let r = match v.split_once('/') {
        Some((w, h)) => w.trim().parse::<f64>().ok()? / h.trim().parse::<f64>().ok()?,
        None => v.parse().ok()?,
    };
    (r.is_finite() && r > 0.0).then_some(r)
}

fn px(v: &str) -> Option<f64> {
    let v = v.trim();
    v.strip_suffix("px").unwrap_or(v).trim().parse().ok()
}

fn edges(v: &str) -> Option<[f64; 4]> {
    let n: Vec<f64> = v.split_whitespace().map(px).collect::<Option<_>>()?;
    Some(match n[..] {
        [a] => [a, a, a, a],
        [a, b] => [a, b, a, b],
        [a, b, c] => [a, b, c, b],
        [a, b, c, d] => [a, b, c, d],
        _ => return None,
    })
}

/// Values split at top-level whitespace (a colour function's inner spaces stay).
fn split_values(v: &str) -> Vec<String> {
    let (mut out, mut cur, mut depth) = (Vec::new(), String::new(), 0i32);
    for ch in v.trim().chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if ch.is_whitespace() && depth <= 0 {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// One to four values expanded to top, right, bottom, left (as `padding`).
fn edges4<T: Clone>(n: &[T]) -> Option<[T; 4]> {
    Some(match n {
        [a] => [a.clone(), a.clone(), a.clone(), a.clone()],
        [a, b] => [a.clone(), b.clone(), a.clone(), b.clone()],
        [a, b, c] => [a.clone(), b.clone(), c.clone(), b.clone()],
        [a, b, c, d] => [a.clone(), b.clone(), c.clone(), d.clone()],
        _ => return None,
    })
}

/// `border-radius: 1-4 values [/ 1-4 values]` as `[h, v]` per corner (top-left,
/// top-right, bottom-right, bottom-left; the value order is the same as `padding`'s
/// with corners for sides).
fn radius_list(v: &str) -> Option<[[f64; 2]; 4]> {
    let (h, vv) = match v.split_once('/') {
        Some((a, b)) => (a, Some(b)),
        None => (v, None),
    };
    let side = |t: &str| -> Option<[f64; 4]> {
        let n: Vec<f64> = t.split_whitespace().map(px).collect::<Option<_>>()?;
        edges4(&n)
    };
    let hs = side(h)?;
    let vs = match vv {
        Some(t) => side(t)?,
        None => hs,
    };
    Some([[hs[0], vs[0]], [hs[1], vs[1]], [hs[2], vs[2]], [hs[3], vs[3]]])
}

/// Apply one declaration (`k` lower-case) to `s`.
pub(crate) fn apply_decl(s: &mut Style, k: &str, v: &str) {
    apply(s, k, v)
}

fn apply(s: &mut Style, k: &str, v: &str) {
    match k {
        "color" => s.color = Some(v.into()),
        "background" | "background-color" => s.background = Some(v.into()),
        "border-color" => {
            let parts = split_values(v);
            if parts.len() <= 1 {
                s.border_color = Some(v.into());
                s.border_colors = None;
            } else if let Some(e) = edges4(&parts) {
                s.border_color = Some(e[0].to_string());
                s.border_colors = Some(e.map(|c| Some(c.to_string())));
            }
        }
        "border-width" => {
            if let Some(e) = edges(v) {
                if e.iter().all(|w| *w == e[0]) {
                    s.border_width = Some(e[0]);
                    s.border_widths = None;
                } else {
                    s.border_width = None;
                    s.border_widths = Some(e);
                }
            } else {
                s.border_width = None;
                s.border_widths = None;
            }
        }
        "border" => {
            // `1px solid #333`
            for part in split_values(v) {
                if let Some(w) = px(&part).filter(|_| part.ends_with("px") || part == "0") {
                    s.border_width = Some(w);
                    s.border_widths = None;
                } else if BORDER_STYLES.contains(&part.as_str()) {
                    s.border_style = Some(part);
                } else {
                    s.border_color = Some(part);
                    s.border_colors = None;
                }
            }
        }
        "border-top" | "border-right" | "border-bottom" | "border-left" | "border-top-width" | "border-right-width" | "border-bottom-width"
        | "border-left-width" | "border-top-color" | "border-right-color" | "border-bottom-color" | "border-left-color" => {
            let mut it = k["border-".len()..].splitn(2, '-');
            let side = it.next().unwrap_or_default();
            let what = it.next();
            let Some(i) = ["top", "right", "bottom", "left"].iter().position(|x| *x == side) else { return };
            let set_w = |s: &mut Style, w: f64| {
                let mut e = s.border_edges();
                e[i] = w;
                s.border_widths = Some(e);
            };
            let set_c = |s: &mut Style, c: String| {
                let base = s.border_color.clone();
                s.border_colors.get_or_insert_with(|| [base.clone(), base.clone(), base.clone(), base])[i] = Some(c);
            };
            match what {
                Some("width") => {
                    if let Some(w) = px(v) {
                        set_w(s, w)
                    }
                }
                Some(_) => set_c(s, v.trim().to_string()),
                None => {
                    for part in split_values(v) {
                        if let Some(w) = px(&part).filter(|_| part.ends_with("px") || part == "0") {
                            set_w(s, w);
                        } else if part == "none" {
                            set_w(s, 0.0);
                        } else if BORDER_STYLES.contains(&part.as_str()) {
                            s.border_style = Some(part);
                        } else {
                            set_c(s, part);
                        }
                    }
                }
            }
        }
        "border-style" => s.border_style = Some(v.trim().into()),
        "aspect-ratio" => s.aspect_ratio = ratio(v),
        "padding" => s.padding = edges(v),
        "margin" => s.margin = edges(v),
        _ if k.starts_with("padding-") || k.starts_with("margin-") => {
            let (box_, side) = k.split_once('-').unwrap_or_default();
            let i = ["top", "right", "bottom", "left"].iter().position(|x| *x == side);
            let e = if box_ == "padding" { &mut s.padding } else { &mut s.margin };
            if let (Some(i), Some(n)) = (i, px(v)) {
                e.get_or_insert([0.0; 4])[i] = n;
            }
        }
        "border-radius" => match radius_list(v) {
            Some(r) if r.iter().all(|c| c[0] == r[0][0] && c[1] == r[0][0]) => {
                s.border_radius = Some(r[0][0]);
                s.border_radii = None;
            }
            Some(r) => {
                s.border_radius = Some(r[0][0]);
                s.border_radii = Some(r);
            }
            None => {
                s.border_radius = None;
                s.border_radii = None;
            }
        },
        "border-top-left-radius" | "border-top-right-radius" | "border-bottom-right-radius" | "border-bottom-left-radius" => {
            let i = ["border-top-left-radius", "border-top-right-radius", "border-bottom-right-radius", "border-bottom-left-radius"]
                .iter()
                .position(|x| *x == k)
                .unwrap_or(0);
            let n: Option<Vec<f64>> = v.split_whitespace().map(px).collect();
            let c = match n.as_deref() {
                Some([a]) => [*a, *a],
                Some([a, b]) => [*a, *b],
                _ => return,
            };
            let mut r = s.radii();
            r[i] = c;
            s.border_radii = Some(r);
        }
        "position" => s.position = Some(v.trim().into()),
        "top" => s.top = px(v),
        "right" => s.right = px(v),
        "bottom" => s.bottom = px(v),
        "left" => s.left = px(v),
        "inset" => {
            let e = edges(v);
            [s.top, s.right, s.bottom, s.left] = e.map_or([None; 4], |e| e.map(Some));
        }
        "z-index" => s.z_index = v.trim().parse().ok(),
        "font-style" => s.font_style = Some(v.trim().into()),
        "overflow-wrap" | "word-wrap" => s.overflow_wrap = Some(v.trim().into()),
        "shape-rendering" => s.shape_rendering = Some(v.trim().into()),
        "font-size" => s.font_size = px(v),
        "font-weight" => {
            s.font_weight = match v {
                "normal" => Some(400),
                "bold" => Some(700),
                n => n.parse().ok(),
            }
        }
        "line-height" => s.line_height = if v.ends_with("px") { px(v) } else { None },
        "font-family" => s.font_family = Some(v.into()),
        "opacity" => s.opacity = v.parse().ok(),
        "width" => (s.width, s.width_pct) = len(v),
        "height" => (s.height, s.height_pct) = len(v),
        "gap" => {
            let parts: Vec<f64> = v.split_whitespace().filter_map(px).collect();
            match parts[..] {
                [g] => {
                    s.gap = Some(g);
                    s.row_gap = Some(g);
                }
                [r, c] => {
                    s.row_gap = Some(r);
                    s.gap = Some(c);
                }
                _ => {}
            }
        }
        "column-gap" => s.gap = px(v),
        "row-gap" => s.row_gap = px(v),
        "min-width" => (s.min_width, s.min_width_pct) = len(v),
        "max-width" => (s.max_width, s.max_width_pct) = len(v),
        "min-height" => s.min_height = px(v),
        "max-height" => s.max_height = px(v),
        "display" => s.display = Some(v.into()),
        "flex-direction" => s.flex_direction = Some(v.into()),
        "flex-wrap" => s.flex_wrap = Some(v.into()),
        "justify-content" => s.justify_content = Some(v.into()),
        "align-items" => s.align_items = Some(v.into()),
        "align-self" => s.align_self = Some(v.into()),
        "flex-grow" => s.flex_grow = v.parse().ok(),
        "flex-shrink" => s.flex_shrink = v.parse().ok(),
        "flex-basis" => s.flex_basis = px(v),
        "flex" => {
            let p: Vec<&str> = v.split_whitespace().collect();
            let (g, sh, b) = match p[..] {
                ["none"] => (Some(0.0), Some(0.0), None),
                ["auto"] => (Some(1.0), Some(1.0), None),
                [g] if g.parse::<f64>().is_ok() => (g.parse().ok(), Some(1.0), Some(0.0)),
                [b] => (Some(1.0), Some(1.0), px(b)),
                [g, sh] => (g.parse().ok(), sh.parse().ok(), Some(0.0)),
                [g, sh, b] => (g.parse().ok(), sh.parse().ok(), if b == "auto" { None } else { px(b) }),
                _ => (None, None, None),
            };
            s.flex_grow = g;
            s.flex_shrink = sh;
            s.flex_basis = b;
        }
        "grid-template-columns" => s.grid_template_columns = Some(v.into()),
        "box-shadow" => s.box_shadow = Some(v.into()),
        "cursor" => s.cursor = Some(v.into()),
        "text-align" => s.text_align = Some(v.into()),
        "white-space" => s.white_space = Some(v.into()),
        "line-clamp" | "-webkit-line-clamp" => s.line_clamp = v.parse::<f64>().ok().filter(|n| *n >= 1.0).map(|n| n.min(999.0) as u16),
        "transition" => s.transitions = super::motion::parse_transitions(v),
        "animation" => s.animations = super::motion::parse_animations(v),
        _ => {}
    }
}

/// Per-widget traces of matched declarations, read by the inspector.
#[cfg(feature = "inspector")]
pub mod trace {
    use super::Applied;
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static TRACES: RefCell<HashMap<String, Vec<Applied>>> = RefCell::new(HashMap::new());
    }

    /// Keep `tr` as the style trace of `widget` (replacing the previous frame's).
    pub fn record(widget: &str, tr: Vec<Applied>) {
        TRACES.with(|t| {
            t.borrow_mut().insert(widget.to_string(), tr);
        });
    }

    /// The last recorded trace of `widget`.
    pub fn of(widget: &str) -> Option<Vec<Applied>> {
        TRACES.with(|t| t.borrow().get(widget).cloned())
    }

    /// Forget every trace (an app that re-registers its widgets each frame rebuilds them).
    pub fn clear() {
        TRACES.with(|t| t.borrow_mut().clear());
    }
}

/// A widget style struct that a style sheet can configure. Each field is reachable as a
/// custom property named after it (`them_bg` → `--them-bg`); common fields also answer
/// to the standard CSS property (`bg` ← `background`, `radius` ← `border-radius`, ...).
pub trait Styled {
    /// Override the fields the resolved style sets; untouched fields keep their value.
    fn apply_style(&mut self, s: &Style);

    /// `self` overridden by the element `spec` of `sheet`.
    fn styled(mut self, sheet: &Sheet, spec: &str) -> Self
    where
        Self: Sized,
    {
        self.apply_style(&sheet.el(spec));
        self
    }

    /// `Default` overridden by the element `spec` of `sheet`.
    fn from_sheet(sheet: &Sheet, spec: &str) -> Self
    where
        Self: Default + Sized,
    {
        let mut v = Self::default();
        v.apply_style(&sheet.el(spec));
        v
    }
}

/// Field conversion used by [`styled!`].
/// One [`styled!`] field: standard properties `aliases` in order, then the
/// field's own `--custom` property; the last one present and parsable wins.
#[doc(hidden)]
#[inline(never)]
pub fn set_field<T: StyleField>(dst: &mut T, s: &Style, aliases: &[&str], field: &str) {
    for a in aliases {
        if let Some(v) = standard(s, a).and_then(|v| T::from_css(&v)) {
            *dst = v;
        }
    }
    if let Some(v) = s.field_css(field).and_then(T::from_css) {
        *dst = v;
    }
}

pub trait StyleField: Sized {
    fn from_css(v: &str) -> Option<Self>;
}

impl StyleField for &'static str {
    fn from_css(v: &str) -> Option<Self> {
        Some(intern(v.trim()))
    }
}

impl StyleField for Option<&'static str> {
    fn from_css(v: &str) -> Option<Self> {
        Some(if v.trim() == "none" { None } else { Some(intern(v.trim())) })
    }
}

impl StyleField for f64 {
    fn from_css(v: &str) -> Option<Self> {
        px(v)
    }
}

impl StyleField for u32 {
    fn from_css(v: &str) -> Option<Self> {
        match v.trim() {
            "normal" => Some(400),
            "bold" => Some(700),
            n => n.parse().ok(),
        }
    }
}

impl StyleField for bool {
    fn from_css(v: &str) -> Option<Self> {
        match v.trim() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        }
    }
}

/// The CSS text of a standard property in a resolved [`Style`].
pub fn standard(s: &Style, prop: &str) -> Option<String> {
    let n = |v: Option<f64>| v.map(|x| x.to_string());
    match prop {
        "color" => s.color.clone(),
        "background" => s.background.clone(),
        "border-color" => s.border_color.clone(),
        "border-width" => n(s.border_width),
        "border-radius" => n(s.border_radius),
        "font-size" => n(s.font_size),
        "font-weight" => s.font_weight.map(|w| w.to_string()),
        "line-height" => n(s.line_height),
        "font-family" => s.font_family.clone(),
        "width" => n(s.width),
        "height" => n(s.height),
        "gap" => n(s.gap),
        "padding-top" => n(s.padding.map(|p| p[0])),
        "padding-right" => n(s.padding.map(|p| p[1])),
        "padding-bottom" => n(s.padding.map(|p| p[2])),
        "padding-left" => n(s.padding.map(|p| p[3])),
        "box-shadow" => s.box_shadow.clone(),
        _ => None,
    }
}

/// Implement [`Styled`] for a style struct: `styled!(T { field [= "css-prop", ...], ... })`.
/// The custom property `--field-name` always applies, after (and so over) the aliases.
#[macro_export]
macro_rules! styled {
    ($t:ty { $($f:ident $(= $($alias:literal),+)?),* $(,)? }) => {
        impl $crate::ui::style::Styled for $t {
            fn apply_style(&mut self, s: &$crate::ui::style::Style) {
                $(
                    $crate::ui::style::set_field(&mut self.$f, s, &[$($($alias),+)?], stringify!($f));
                )*
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    const CSS: &str = r#"
        :root { --txt: #f5f5f5; --acc: #3b82f6; --btn-bg: var(--el, #1f1f1f) }
        /* the base */
        button { color: var(--txt); font-size: 13px; line-height: 1.5; padding: 5px 14px }
        .seg button, .x { gap: 1px }
    "#;

    #[test]
    fn descendant_and_child_selectors() {
        let sh = Sheet::parse(CSS).unwrap();
        let seg = [Node::parse("div.seg")];
        // `.seg button` needs the ancestor; without one only `button` applies.
        assert_eq!(sh.resolve_in(&Node::new("button"), &seg, None).gap, Some(1.0));
        assert_eq!(sh.resolve(&Node::new("button"), None).gap, None);
        let sh = Sheet::parse(
            ".a > .b { width: 10px } .a .b { height: 5px } .a .c > .b { width: 20px } .x { width: 3px } .bad >  { }",
        );
        assert!(sh.is_err(), "a dangling combinator is rejected");
        let sh = Sheet::parse(".a > .b { width: 10px } .a .b { height: 5px } .a .c > .b { width: 20px }").unwrap();
        assert!(sh.has_complex());
        let b = Node::parse("div.b");
        // child of .a
        let s = sh.resolve_in(&b, &[Node::parse("div.a")], None);
        assert_eq!((s.width, s.height), (Some(10.0), Some(5.0)));
        // grandchild of .a through .c: descendant + the more specific `.a .c > .b`
        let s = sh.resolve_in(&b, &[Node::parse("div.a"), Node::parse("div.c")], None);
        assert_eq!((s.width, s.height), (Some(20.0), Some(5.0)));
        // grandchild through a plain div: descendant only
        let s = sh.resolve_in(&b, &[Node::parse("div.a"), Node::parse("div")], None);
        assert_eq!((s.width, s.height), (None, Some(5.0)));
        // backtracking: `.a .c > .b` with .c the parent and .a further up
        let s = sh.resolve_in(&b, &[Node::parse("div.a"), Node::parse("div"), Node::parse("div.c")], None);
        assert_eq!(s.width, Some(20.0));
    }

    #[test]
    fn percent_lengths_aspect_ratio_and_border_style() {
        let s = inline("width: 50%; height: 25%; max-width: 80%; aspect-ratio: 16 / 9; border: 1px dashed #333");
        assert_eq!((s.width, s.width_pct, s.height_pct, s.max_width_pct), (None, Some(0.5), Some(0.25), Some(0.8)));
        assert!((s.aspect_ratio.unwrap() - 16.0 / 9.0).abs() < 1e-9);
        assert_eq!((s.border_style.as_deref(), s.border_color.as_deref(), s.border_width), (Some("dashed"), Some("#333"), Some(1.0)));
        assert_eq!(s.border_dash(1.0), Some([3.0, 3.0]));
        assert!(!inline("border-style: none").border_drawn());
        assert_eq!(inline("aspect-ratio: 1.5").aspect_ratio, Some(1.5));
        assert_eq!(inline("aspect-ratio: auto").aspect_ratio, None);
    }

    #[test]
    fn cascade_specificity_then_order() {
        let sh = Sheet::parse(
            "button{background:#111} .on{background:#222} button{background:#333} #send{background:var(--acc)} :root{--acc:#3b82f6}",
        )
        .unwrap();
        assert_eq!(sh.resolve(&Node::new("button"), None).background.as_deref(), Some("#333"));
        assert_eq!(sh.resolve(&Node::new("button").class("on"), None).background.as_deref(), Some("#222"));
        assert_eq!(sh.resolve(&Node::new("button").class("on").id("send"), None).background.as_deref(), Some("#3b82f6"));
    }

    #[test]
    fn state_precedence() {
        let sh = Sheet::parse(".b{color:#aaa} .b:hover{color:#bbb} .b:focus-visible{color:#ccc} .b:disabled{opacity:0.5}").unwrap();
        let n = |s| Node::new("button").class("b").state(s);
        assert_eq!(sh.resolve(&n(State::NONE), None).color.as_deref(), Some("#aaa"));
        assert_eq!(sh.resolve(&n(State::HOVER), None).color.as_deref(), Some("#bbb"));
        // equal specificity: the later rule wins
        assert_eq!(sh.resolve(&n(State::HOVER | State::FOCUS_VISIBLE), None).color.as_deref(), Some("#ccc"));
        assert_eq!(sh.resolve(&n(State::DISABLED), None).opacity, Some(0.5));
        assert!(Selector::parse("a:nth-child").is_err());
    }

    #[test]
    fn variables_fallbacks_and_inheritance() {
        let sh = Sheet::parse(
            ":root{--txt:#f5f5f5;--btn-bg:var(--el, #1f1f1f)} .card{color:var(--txt);font-size:14px;line-height:1.5;background:var(--btn-bg)} .pill{font-size:11.5px;padding:0 8px}",
        )
        .unwrap();
        assert_eq!(sh.var("--btn-bg").as_deref(), Some("#1f1f1f"));
        let card = sh.resolve(&Node::new("div").class("card"), None);
        assert_eq!((card.color.as_deref(), card.line_height, card.background.as_deref()), (Some("#f5f5f5"), Some(21.0), Some("#1f1f1f")));
        let pill = sh.resolve(&Node::new("span").class("pill"), Some(&card));
        // colour and line-height inherit (a computed px line-height, as in CSS); background does not
        assert_eq!((pill.color.as_deref(), pill.font_size, pill.line_height, pill.background), (Some("#f5f5f5"), Some(11.5), Some(21.0), None));
        assert_eq!(pill.padding, Some([0.0, 8.0, 0.0, 8.0]));
    }

    #[test]
    fn trace_reports_rule_sources_in_cascade_order() {
        let sh = Sheet::parse(":root{--a:#111} #x{color:var(--a)} .b, button{color:#222;gap:2px}").unwrap();
        let (s, tr) = sh.resolve_traced(&Node::new("button").class("b").id("x"), None);
        assert_eq!(s.color.as_deref(), Some("#111"));
        let sel: Vec<_> = tr.iter().map(|a| (a.property.as_str(), a.selector.as_str())).collect();
        assert_eq!(sel, [("color", "button"), ("gap", "button"), ("color", ".b"), ("gap", ".b"), ("color", "#x")]);
        assert_eq!(tr.last().unwrap().value, "#111");
    }

    #[test]
    fn custom_properties_cascade_inherit_and_feed_var() {
        let sh = Sheet::parse(":root{--acc:#3b82f6} .t{--me-bg:var(--acc);--pad:4px} .t.dim{--pad:2px;padding:var(--pad)} .c{color:var(--me-bg)}").unwrap();
        let t = sh.resolve(&Node::parse("div.t.dim"), None);
        assert_eq!((t.custom("--me-bg"), t.custom("--pad"), t.padding), (Some("#3b82f6"), Some("2px"), Some([2.0; 4])));
        let c = sh.resolve(&Node::parse("span.c"), Some(&t));
        assert_eq!(c.color.as_deref(), Some("#3b82f6"), "inherited custom property feeds var()");
        assert_eq!(sh.el("div.t").custom("--pad"), Some("4px"));
        assert_eq!(sh.var_static("--acc", "#000"), "#3b82f6");
        assert_eq!(sh.var_static("--nope", "#000"), "#000");
    }

    #[derive(Default, Debug, PartialEq)]
    struct Demo {
        bg: &'static str,
        border: Option<&'static str>,
        radius: f64,
        hdr_weight: u32,
        chevron_on_hover: bool,
    }
    crate::styled!(Demo { bg = "background", border = "border-color", radius = "border-radius", hdr_weight, chevron_on_hover });

    #[test]
    fn styled_structs_read_standard_and_custom_properties() {
        let sh = Sheet::parse(".pill{background:#111;border-radius:9px;--hdr-weight:600;--chevron-on-hover:true} .pill.flat{--border:none} .pill.edge{border:1px solid #222;--bg:#333}").unwrap();
        let d = Demo::from_sheet(&sh, "div.pill");
        assert_eq!(d, Demo { bg: "#111", border: None, radius: 9.0, hdr_weight: 600, chevron_on_hover: true });
        let e = Demo::from_sheet(&sh, "div.pill.edge");
        assert_eq!((e.bg, e.border), ("#333", Some("#222")), "the custom property wins over the alias");
        assert_eq!(Demo::from_sheet(&sh, "div.pill.flat").border, None);
        let n = Node::parse("button.a.b#x:hover");
        assert_eq!((n.kind, n.id, n.classes.clone(), n.state), ("button", Some("x"), vec!["a", "b"], State::HOVER));
    }

    #[test]
    fn dialog_card_style_reads_the_sheet() {
        let sh = Sheet::parse(":root{--el:#1b1b1b} .dlg{background:var(--el);border:1px solid #2c2c2c;border-radius:14px;box-shadow:0 30px 80px rgba(0,0,0,0.8)} .flat{box-shadow:none;--border:none}").unwrap();
        let base = crate::ui::widgets::composite::dialog::CardStyle { bg: "a", border: None, radius: 0.0, shadow: None };
        let c = base.styled(&sh, "div.dlg");
        assert_eq!(c, crate::ui::widgets::composite::dialog::CardStyle { bg: "#1b1b1b", border: Some("#2c2c2c"), radius: 14.0, shadow: Some(crate::ui::widgets::composite::dialog::Shadow { dy: 30.0, blur: 80.0, color: "rgba(0,0,0,0.8)" }) });
        let f = base.styled(&sh, "div.dlg.flat");
        assert_eq!((f.shadow, f.border), (None, None));
    }
    #[test]
    fn border_shorthand_and_comments() {
        let sh = Sheet::parse("/* x */ .c{border:1px solid #1b1b1b;border-radius:12px;margin:0 0 8px}").unwrap();
        let s = sh.resolve(&Node::new("div").class("c"), None);
        assert_eq!((s.border_width, s.border_color.as_deref(), s.border_radius, s.margin), (Some(1.0), Some("#1b1b1b"), Some(12.0), Some([0.0, 0.0, 8.0, 0.0])));
        assert!(Sheet::parse(".a{color:#fff").is_err());
        assert!(Sheet::parse(".a{color}").is_err());
    }

    #[test]
    fn margin_and_padding_longhands_set_one_side() {
        let sh = Sheet::parse(".a{margin-top:18px}.b{padding:4px;padding-right:50px;margin-left:3px}").unwrap();
        let a = sh.resolve(&Node::new("div").class("a"), None);
        assert_eq!(a.margin, Some([18.0, 0.0, 0.0, 0.0]));
        let b = sh.resolve(&Node::new("div").class("b"), None);
        assert_eq!((b.padding, b.margin), (Some([4.0, 50.0, 4.0, 4.0]), Some([0.0, 0.0, 0.0, 3.0])));
    }

    #[test]
    fn corners_sides_and_positions_parse() {
        let s = inline("border-radius: 1px 2px 3px / 4px 5px; border-top-right-radius: 7px 8px");
        assert_eq!(s.radii(), [[1.0, 4.0], [7.0, 8.0], [3.0, 4.0], [2.0, 5.0]]);
        assert_eq!(s.border_radius, Some(1.0), "the first corner stays readable as the radius");
        // A later uniform radius resets the corners.
        assert_eq!(inline("border-radius: 2px 0; border-radius: 6px").border_radii, None);
        let b = inline("border: 1px solid rgba(0, 0, 0, 0.5); border-bottom-width: 3px; border-right: none");
        assert_eq!(b.border_color.as_deref(), Some("rgba(0, 0, 0, 0.5)"));
        assert_eq!(b.border_edges(), [1.0, 0.0, 3.0, 1.0]);
        let c = inline("border-color: red green; border-width: 2px");
        assert_eq!((c.border_side_color(0), c.border_side_color(1)), (Some("red"), Some("green")));
        assert_eq!((c.border_width, c.border_widths), (Some(2.0), None));
        let p = inline("position: absolute; inset: 1px 2px; z-index: 3; font-style: italic; overflow-wrap: anywhere; shape-rendering: geometricPrecision");
        assert!(p.positioned() && p.out_of_flow());
        assert_eq!([p.top, p.right, p.bottom, p.left], [Some(1.0), Some(2.0), Some(1.0), Some(2.0)]);
        assert_eq!((p.z_index, p.font_style.as_deref()), (Some(3), Some("italic")));
        // font-style, overflow-wrap and shape-rendering inherit.
        let sh = Sheet::parse("div { font-style: italic; overflow-wrap: anywhere; shape-rendering: geometricPrecision }").unwrap();
        let parent = sh.el("div");
        let kid = sh.resolve(&Node::parse("span"), Some(&parent));
        assert_eq!((kid.font_style, kid.overflow_wrap, kid.shape_rendering), (parent.font_style.clone(), parent.overflow_wrap.clone(), parent.shape_rendering.clone()));
    }
}
