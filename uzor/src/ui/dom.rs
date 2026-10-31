//! A small DOM: a tree of elements styled by a [`Sheet`](crate::ui::style::Sheet), laid out
//! with [`flow`](crate::ui::flow) (`display:flex` needs the `flow-flex` feature, on by default)
//! and painted as CSS boxes (background, border, radius,
//! `box-shadow`, text). Text keeps one line, ellipsized when it does not fit, unless
//! `white-space: normal` lets it wrap at spaces and after inner hyphens (as browsers break). Screens describe *what*
//! is on them (`El::new("div.card").child(...)`) and the stylesheet decides *how it looks
//! and where it goes*: no per-screen pixel math. [`render`] returns every element's border
//! box by `id`, so the app hit-tests, registers and focuses by id as in a browser.
//!
//! `opacity` dims an element with its subtree; [`El::svg`] puts an inline icon in an element.
//! Element kinds with built-in content: `select` draws its chevron in the right padding
//! (`--chevron` colour, default the text colour). Everything else is a plain box.
//! `display:none` removes an element; `:hover` / `:focus-visible` / `:disabled` /
//! `:active` rules apply through the state callback given to [`render`].
//!
//! Inline rich text: [`El::runs`] puts styled [`Run`]s (bold, italic, code chips, links)
//! in one text flow that wraps across them; each run is styled by the sheet as an element
//! of its own (`b`, `code`, `a.link`, ...) inheriting the element's font, and a run with an
//! id is a hit of its own, one rect per wrapped piece.
//!
//! `position: absolute` takes an element out of the flow and places it against its nearest
//! positioned ancestor (`top` / `right` / `bottom` / `left`); positioned elements paint and
//! hit-test above the flow, ordered by `z-index`. Corners may differ (`border-radius: 12px
//! 12px 0 0`, elliptical `20px / 10px`, `border-top-left-radius`) and so may border sides
//! (`border-width: 0 0 1px`, `border-left: 3px solid red`). `shape-rendering:
//! geometricPrecision` draws an element's icon exactly, without pixel snapping.
//!
//! ```
//! use uzor::ui::dom::{lay_out, El, Run};
//! use uzor::ui::style::{Sheet, State};
//! fn card(ctx: &mut dyn uzor::render::RenderContext) {
//! let sheet = Sheet::parse(r#"
//!     div.card { position: relative; width: 240px; padding: 8px; white-space: normal;
//!                border-radius: 12px 12px 4px 12px; border-bottom: 2px solid #3b82f6 }
//!     b { font-weight: 700 }
//!     code { font-family: monospace; background: #222; padding: 0 6px; border-radius: 6px }
//!     a.link { color: #60a5fa }
//!     div.badge { position: absolute; top: -6px; right: -6px; width: 16px; height: 16px; z-index: 1 }
//! "#).unwrap();
//! let card = El::new("div.card").id("card")
//!     .runs([Run::text("Run "), Run::new("code", "cargo test"), Run::text(" and see "),
//!            Run::new("a.link", "the docs").id("url:docs"), Run::new("b", " now")])
//!     .child(El::new("div.badge").id("badge"));
//! let dom = lay_out(ctx, &sheet, &card, 0.0, 0.0, 800.0, &|_| State::NONE);
//! // The badge hangs over the card's corner and is the topmost hit there.
//! let (badge, card) = (dom.rect("badge").unwrap(), dom.rect("card").unwrap());
//! assert_eq!((badge.x + badge.width, badge.y), (card.x + card.width + 6.0, card.y - 6.0));
//! // The link run is a hit of its own.
//! assert!(dom.hits().any(|(id, _)| id == "url:docs"));
//! }
//! ```
//!
//! Animation nodes ([`El::anim`]) are boxes like any other (flex sizes them, an `id` makes
//! them hits, document order is their z-order) whose content is painted every frame by a
//! named [`AnimScript`](crate::render::AnimScript) from an
//! [`AnimRegistry`](crate::render::AnimRegistry): typically a canvas-style JS function
//! (`uzor-anim-js`: the page's engine on web, QuickJS on native) with a Rust or baked
//! fallback. [`paint_with`] / [`paint_anim`] paint them; an animation outside the visible
//! rect, fully transparent, or in a paused frame does not run.

use crate::render::{AnimParams, AnimRegistry, RenderContext, TextAlign};
use crate::types::Rect;
use crate::ui::css_box::ellipsize;
use crate::ui::css_text::{fill_text_css, INTER};
use crate::ui::flow::{layout_in, LBox, Laid};
use crate::ui::style::{apply_decl, Node, Sheet, State, Style, StyleField};
use crate::ui::widgets::composite::dialog::{fill_round_rect, path_round_rect, Shadow};

/// One element: `spec` is `kind#id.class.class` as in a selector (`div.card`, `button.pbtn`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct El {
    pub spec: String,
    /// Hit / focus id (also matched by `#id` selectors only if written in `spec`).
    pub id: Option<String>,
    pub text: Option<String>,
    /// Inline declarations (`style="..."`), over the sheet.
    pub css: Option<String>,
    /// A Tab stop (focusable control).
    pub tab: bool,
    /// Accessible name; defaults to the text.
    pub name: Option<String>,
    /// Inline `<svg>` markup drawn into the content box (stroke = the text colour).
    pub svg: Option<String>,
    /// Animated content ([`El::anim`]).
    pub anim: Option<AnimNode>,
    pub children: Vec<El>,
    /// Inline rich text ([`El::runs`]); when set it replaces `text`.
    pub runs: Vec<Run>,
    /// Built-in control content ([`El::control`]): switch, checkbox, spinner, text input.
    pub control: Option<Control>,
    /// An embed slot ([`El::slot`]): the app paints something else (a kit widget, a
    /// canvas) into the element's content box, found with [`Dom::slots`].
    pub slot: Option<String>,
}

/// Built-in control content of an element, painted into its content box after the
/// element's own CSS box (so the track, frame and colours stay in the style sheet).
///
/// | control | paints | CSS hooks |
/// |---|---|---|
/// | [`Control::Switch`] | a round knob in the content box, left (off) to right (on) | `--knob` (`#fff`), `:checked` on the element when on |
/// | [`Control::Checkbox`] | a tick (or a dash when mixed) | `--check` (default `color`), `--check-width` (2px), `:checked` |
/// | [`Control::Spinner`] | a rotating 270° arc | `color`, `--spin-track`, `--spin-width`, `--spin-speed` (turns/s, 1) |
/// | [`Control::Input`] | value or placeholder, selection, caret, IME composition | `color`, `font-*`, `--placeholder`, `--selection`, `--caret`, `:focus` while it has a caret |
#[derive(Clone, Debug, PartialEq)]
pub enum Control {
    /// A switch: `pos` 0 (off) … 1 (on) places the knob (animate it for a slide).
    Switch { on: bool, pos: f64 },
    /// A checkbox tick; `mixed` draws the indeterminate dash.
    Checkbox { on: bool, mixed: bool },
    /// A busy spinner at time `t` (seconds).
    Spinner { t: f64 },
    /// A single-line text input showing the edit state (the app or the framework's text
    /// store owns editing; the DOM lays it out and paints it).
    Input(Box<Edit>),
}

impl Control {
    /// A switch at rest.
    pub fn switch(on: bool) -> Self {
        Control::Switch { on, pos: if on { 1.0 } else { 0.0 } }
    }
    pub fn checkbox(on: bool) -> Self {
        Control::Checkbox { on, mixed: false }
    }
    pub fn spinner(t: f64) -> Self {
        Control::Spinner { t }
    }
    pub fn input(e: Edit) -> Self {
        Control::Input(Box::new(e))
    }
    /// The ARIA role an element with this control exposes.
    pub fn role(&self) -> &'static str {
        match self {
            Control::Switch { .. } => "switch",
            Control::Checkbox { .. } => "checkbox",
            Control::Spinner { .. } => "progressbar",
            Control::Input(_) => "textbox",
        }
    }
    /// On (switch / checkbox).
    pub fn checked(&self) -> Option<bool> {
        match self {
            Control::Switch { on, .. } | Control::Checkbox { on, .. } => Some(*on),
            _ => None,
        }
    }
}

/// The edit state a DOM text input shows ([`Control::Input`]). Indices are chars.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Edit {
    pub value: String,
    /// Shown while `value` is empty and nothing is being composed.
    pub placeholder: String,
    /// The caret; `Some` = focused (the element gets `:focus`).
    pub caret: Option<usize>,
    /// The other end of a selection (a selection is `anchor..caret`).
    pub anchor: Option<usize>,
    /// IME composition shown at the caret, underlined.
    pub preedit: Option<String>,
    /// Bullets instead of the text (passwords).
    pub masked: bool,
    /// The caret is in the visible half of its blink (`false` hides it).
    pub caret_on: bool,
}

impl Edit {
    /// An unfocused input showing `value`.
    pub fn new(value: impl Into<String>) -> Self {
        Edit { value: value.into(), caret_on: true, ..Default::default() }
    }
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }
    /// Focused with the caret at char `at` (the value's length at most).
    pub fn caret(mut self, at: usize) -> Self {
        self.caret = Some(at.min(self.value.chars().count()));
        self
    }
    /// Select `anchor..caret` (call after [`Edit::caret`]).
    pub fn select_from(mut self, anchor: usize) -> Self {
        self.anchor = Some(anchor.min(self.value.chars().count()));
        self
    }
    pub fn preedit(mut self, p: impl Into<String>) -> Self {
        self.preedit = Some(p.into());
        self
    }
    pub fn masked(mut self, on: bool) -> Self {
        self.masked = on;
        self
    }
    pub fn caret_on(mut self, on: bool) -> Self {
        self.caret_on = on;
        self
    }
    /// What is painted: the value (bullets when masked) with the composition at the caret.
    pub fn display(&self) -> String {
        let v: String = if self.masked { "\u{2022}".repeat(self.value.chars().count()) } else { self.value.clone() };
        match (&self.preedit, self.caret) {
            (Some(p), Some(c)) => {
                let at = v.char_indices().nth(c).map_or(v.len(), |(i, _)| i);
                format!("{}{}{}", &v[..at], p, &v[at..])
            }
            _ => v,
        }
    }
    /// The caret in [`Edit::display`] chars (after the composition).
    pub fn display_caret(&self) -> Option<usize> {
        self.caret.map(|c| c + self.preedit.as_ref().map_or(0, |p| p.chars().count()))
    }
    fn shows_placeholder(&self) -> bool {
        self.value.is_empty() && self.preedit.as_deref().is_none_or(str::is_empty)
    }
}

/// One styled run of inline text ([`El::runs`]).
///
/// `spec` styles the run as an element of its own (`span`, `b`, `i`, `code`,
/// `a.link`), resolved by the sheet as a child of the element holding the runs, so it
/// inherits that element's font and colour. A run paints its `background` as a chip
/// behind each of its pieces (with its `border-radius`, its left / right `padding` at its
/// first / last piece, and its vertical padding around a `1.2em` content height), and
/// takes its own `color`, `font-*` and `font-style`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Run {
    pub spec: String,
    pub text: String,
    /// Hit id: each of the run's pieces is a hit ([`Dom::hits`], [`Dom::run_at`]).
    pub id: Option<String>,
    /// Inline declarations over the sheet.
    pub css: Option<String>,
}

impl Run {
    /// A run styled as `spec` (`"code"`, `"a.link"`).
    pub fn new(spec: impl Into<String>, text: impl Into<String>) -> Self {
        Self { spec: spec.into(), text: text.into(), ..Self::default() }
    }
    /// A plain run (a `span`: the element's own font and colour).
    pub fn text(text: impl Into<String>) -> Self {
        Self::new("span", text)
    }
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
    pub fn css(mut self, decls: impl Into<String>) -> Self {
        self.css = Some(decls.into());
        self
    }
}

/// A run's resolved look on a placed element.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedRun {
    pub id: Option<String>,
    pub style: Style,
    pub text: String,
}

/// One piece of a run on one line. Coordinates are relative to the element's content
/// box, so they survive [`Dom::moved`].
#[derive(Clone, Debug, PartialEq)]
pub struct RunPiece {
    /// Index into [`Inline::runs`].
    pub run: usize,
    /// Left edge of the piece's box (a chip's left edge) and its width (text plus padding).
    pub x: f64,
    pub w: f64,
    /// Where the text starts inside the box (a chip's left padding on its first piece).
    pub pad: f64,
    pub text: String,
    /// Char offset of the piece's first char in [`Inline::text`].
    pub start: usize,
    /// X of each char boundary from the text start (`text.chars().count() + 1` values),
    /// for carets and selections.
    pub bounds: Vec<f64>,
}

/// One wrapped line of inline runs.
#[derive(Clone, Debug, PartialEq)]
pub struct RunLine {
    /// Top of the line box, relative to the content box.
    pub y: f64,
    pub height: f64,
    /// Width up to the end of its last non-space piece.
    pub width: f64,
    pub pieces: Vec<RunPiece>,
}

/// Laid-out inline runs of a placed element ([`Placed::inline`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Inline {
    pub runs: Vec<PlacedRun>,
    pub lines: Vec<RunLine>,
    /// Every run's text, concatenated (the element's text content).
    pub text: String,
}

/// What an animation node paints: the script `name` in an [`AnimRegistry`] and its
/// per-instance params (the [`AnimParams`] besides the size and the pointer).
#[derive(Clone, Debug, PartialEq)]
pub struct AnimNode {
    pub name: String,
    pub seed: u32,
    pub args: [f64; crate::render::anim_script::ANIM_ARGS],
    /// Frames per second while it moves; `None`: a still frame (painted, no next frame asked).
    pub fps: Option<f64>,
    /// The script gets the window pointer wherever it is (eyes that follow it), not only
    /// while this node is the topmost thing under it.
    pub track_pointer: bool,
    /// Paint outside the content box (CSS `overflow: visible`); by default the frame is
    /// clipped to the content box, like a `<canvas>`.
    pub overflow: bool,
}

impl AnimNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), seed: 0, args: [0.0; crate::render::anim_script::ANIM_ARGS], fps: Some(30.0), track_pointer: false, overflow: false }
    }
    pub fn seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }
    /// The first `args` (up to [`ANIM_ARGS`](crate::render::anim_script::ANIM_ARGS); the rest 0).
    pub fn args<const N: usize>(mut self, args: [f64; N]) -> Self {
        let n = N.min(self.args.len());
        self.args = Default::default();
        self.args[..n].copy_from_slice(&args[..n]);
        self
    }
    /// `None`: still.
    pub fn fps(mut self, fps: Option<f64>) -> Self {
        self.fps = fps;
        self
    }
    pub fn track_pointer(mut self, on: bool) -> Self {
        self.track_pointer = on;
        self
    }
    pub fn overflow_visible(mut self, on: bool) -> Self {
        self.overflow = on;
        self
    }
}

impl El {
    pub fn new(spec: impl Into<String>) -> Self {
        Self { spec: spec.into(), ..Self::default() }
    }
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
    pub fn text(mut self, t: impl Into<String>) -> Self {
        self.text = Some(t.into());
        self
    }
    pub fn css(mut self, decls: impl Into<String>) -> Self {
        self.css = Some(decls.into());
        self
    }
    /// A focusable control (`id` required to be useful).
    pub fn tab(mut self) -> Self {
        self.tab = true;
        self
    }
    pub fn name(mut self, n: impl Into<String>) -> Self {
        self.name = Some(n.into());
        self
    }
    /// Inline `<svg>` content (an icon), drawn in the content box in `color`.
    pub fn svg(mut self, markup: impl Into<String>) -> Self {
        self.svg = Some(markup.into());
        self
    }
    /// Animated content: `node`'s script paints the content box every frame
    /// (see [`paint_with`]).
    pub fn anim(mut self, node: AnimNode) -> Self {
        self.anim = Some(node);
        self
    }
    pub fn child(mut self, c: El) -> Self {
        self.children.push(c);
        self
    }
    /// Inline rich text: styled runs in one text flow (replacing [`El::text`]). Under
    /// `white-space: normal` / `pre-wrap` the runs wrap together at spaces, a run's chip
    /// padding counting at its ends; `overflow-wrap: anywhere` breaks a word that does
    /// not fit a line. `\n` always starts a line. `line-clamp` keeps the first lines.
    pub fn runs(mut self, runs: impl IntoIterator<Item = Run>) -> Self {
        self.runs.extend(runs);
        self
    }
    pub fn children(mut self, cs: impl IntoIterator<Item = El>) -> Self {
        self.children.extend(cs);
        self
    }
    /// Built-in control content ([`Control`]).
    pub fn control(mut self, c: Control) -> Self {
        self.control = Some(c);
        self
    }
    /// An embed slot named `key`: the element is laid out and painted as a box like any
    /// other, and [`Dom::slots`] gives its content box so the app paints a kit widget
    /// (or anything else) there.
    pub fn slot(mut self, key: impl Into<String>) -> Self {
        self.slot = Some(key.into());
        self
    }
    /// Append `c` when `cond` (conditional markup).
    pub fn child_if(self, cond: bool, c: impl FnOnce() -> El) -> Self {
        if cond {
            self.child(c())
        } else {
            self
        }
    }
}

/// One laid-out element.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub id: Option<String>,
    pub kind: String,
    pub rect: Rect,
    pub style: Style,
    pub text: Option<String>,
    pub tab: bool,
    pub name: Option<String>,
    pub svg: Option<String>,
    pub anim: Option<AnimNode>,
    /// Laid-out inline runs ([`El::runs`]).
    pub inline: Option<Box<Inline>>,
    /// Built-in control content ([`El::control`]).
    pub control: Option<Control>,
    /// Embed slot key ([`El::slot`]).
    pub slot: Option<String>,
    /// In a `position: fixed` subtree: placed against the viewport and not moved by
    /// [`Dom::moved`] (an overlay that does not scroll with the page).
    pub fixed: bool,
    /// The clip from `overflow: hidden` ancestors (their padding boxes intersected),
    /// in the same coordinates as [`Placed::rect`]; paint and hits stay inside it.
    pub clip: Option<Rect>,
    /// A text input's char boundaries: x of each boundary of [`Edit::display`] from the
    /// text start (`chars + 1` values).
    pub bounds: Vec<f64>,
}

impl Default for Placed {
    fn default() -> Self {
        Self {
            id: None,
            kind: "div".into(),
            rect: Rect::default(),
            style: Style::default(),
            text: None,
            tab: false,
            name: None,
            svg: None,
            anim: None,
            inline: None,
            control: None,
            slot: None,
            fixed: false,
            clip: None,
            bounds: Vec::new(),
        }
    }
}

impl Placed {
    /// The ARIA role: a control's ([`Control::role`]), else from the kind (`button`,
    /// `a` → `link`, `select` → `combobox`, `input` → `textbox`), else `generic`.
    pub fn role(&self) -> &'static str {
        if let Some(c) = &self.control {
            return c.role();
        }
        match self.kind.as_str() {
            "button" => "button",
            "a" => "link",
            "select" => "combobox",
            "input" | "textarea" => "textbox",
            "img" => "img",
            "li" => "listitem",
            "ul" | "ol" => "list",
            "h1" | "h2" | "h3" | "h4" => "heading",
            _ if self.tab => "button",
            _ => "generic",
        }
    }
    /// The accessible name: [`El::name`] when set, else the element's text (or inline
    /// runs' text, or a text input's value / placeholder).
    pub fn accessible_name(&self) -> Option<&str> {
        self.name.as_deref().or(self.text.as_deref()).or(self.inline.as_ref().map(|i| i.text.as_str())).or(match &self.control {
            Some(Control::Input(e)) if !e.masked && !e.value.is_empty() => Some(e.value.as_str()),
            Some(Control::Input(e)) => Some(e.placeholder.as_str()).filter(|p| !p.is_empty()),
            _ => None,
        })
    }
    /// A text input's horizontal scroll: enough to keep the caret inside the content box.
    pub fn input_scroll(&self) -> f64 {
        let Some(Control::Input(e)) = &self.control else { return 0.0 };
        let c = content_box(self);
        let at = e.display_caret().and_then(|i| self.bounds.get(i)).copied().unwrap_or(0.0);
        (at - c.width + 1.0).max(0.0)
    }
    /// A text input's char index (in [`Edit::display`]) nearest to window `x`: where a
    /// click puts the caret.
    pub fn input_index_at(&self, x: f64) -> Option<usize> {
        let Some(Control::Input(_)) = &self.control else { return None };
        let c = content_box(self);
        let lx = x - c.x + self.input_scroll();
        self.bounds.iter().enumerate().min_by(|a, b| (a.1 - lx).abs().total_cmp(&(b.1 - lx).abs())).map(|(i, _)| i)
    }
    /// A text input's caret rect (1px wide, the line height, window coordinates): for
    /// painting and for the OS IME candidate window.
    pub fn caret_rect(&self) -> Option<Rect> {
        let Some(Control::Input(e)) = &self.control else { return None };
        let i = e.display_caret()?;
        let c = content_box(self);
        let lh = font_of(&self.style).2;
        let x = c.x + self.bounds.get(i).copied().unwrap_or(0.0) - self.input_scroll();
        Some(Rect::new(x.round(), c.y + ((c.height - lh) / 2.0).max(0.0), 1.0, lh.min(c.height.max(lh))))
    }
    /// Every inline run piece as `(run index, rect)` in window coordinates (the piece's
    /// box over its full line height), line by line.
    pub fn run_boxes(&self) -> Vec<(usize, Rect)> {
        let Some(inl) = &self.inline else { return Vec::new() };
        let c = content_box(self);
        inl.lines
            .iter()
            .flat_map(|l| l.pieces.iter().map(move |p| (p.run, Rect::new(c.x + p.x, c.y + l.y, p.w, l.height))))
            .collect()
    }
    /// The char offset (in [`Inline::text`]) of the inline text boundary nearest to
    /// `(x, y)`: the line under `y` (clamped to the first / last), the nearest boundary
    /// on it. For carets and drag selections.
    pub fn char_at(&self, x: f64, y: f64) -> Option<usize> {
        let inl = self.inline.as_ref()?;
        let c = content_box(self);
        let line = inl.lines.iter().find(|l| y < c.y + l.y + l.height).or(inl.lines.last())?;
        let mut best: Option<(f64, usize)> = None;
        for p in &line.pieces {
            for (i, bx) in p.bounds.iter().enumerate() {
                let d = (c.x + p.x + p.pad + bx - x).abs();
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, p.start + i));
                }
            }
        }
        best.map(|b| b.1).or_else(|| line.pieces.first().map(|p| p.start))
    }
}

/// A rendered tree, in paint (document) order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dom {
    pub boxes: Vec<Placed>,
    /// The root's border box.
    pub root: Rect,
}

impl Dom {
    /// The border box of element `id`.
    pub fn rect(&self, id: &str) -> Option<Rect> {
        self.boxes.iter().find(|b| b.id.as_deref() == Some(id)).map(|b| b.rect)
    }
    /// `(id, rect)` of every element with an id, in paint order (later = on top): document
    /// order, positioned elements after the flow by `z-index`. An element's inline runs
    /// with ids follow it, one rect per piece.
    pub fn hits(&self) -> impl Iterator<Item = (&str, Rect)> {
        self.boxes.iter().flat_map(|b| {
            let cut = |r: Rect| match b.clip {
                Some(c) => c.intersect(r),
                None => r,
            };
            let own = b.id.as_deref().map(|i| (i, cut(b.rect)));
            let runs: Vec<(&str, Rect)> = match &b.inline {
                Some(inl) if inl.runs.iter().any(|r| r.id.is_some()) => {
                    b.run_boxes().into_iter().filter_map(|(ri, r)| Some((inl.runs[ri].id.as_deref()?, cut(r)))).collect()
                }
                _ => Vec::new(),
            };
            own.into_iter().chain(runs).filter(|(_, r)| r.width > 0.0 && r.height > 0.0)
        })
    }
    /// The topmost element with inline runs under `(x, y)` and the run there (a piece of
    /// it contains the point): `(element, run index)`.
    pub fn run_at(&self, x: f64, y: f64) -> Option<(&Placed, usize)> {
        self.boxes.iter().rev().filter(|b| b.inline.is_some()).find_map(|b| {
            b.run_boxes()
                .into_iter()
                .find(|(_, r)| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height)
                .map(|(ri, _)| (b, ri))
        })
    }
    /// The Tab stops in document order.
    pub fn tabbable(&self) -> impl Iterator<Item = &str> {
        self.boxes.iter().filter(|b| b.tab).filter_map(|b| b.id.as_deref())
    }
    /// The tree shifted by `(dx, dy)`: lay out once to learn a size, then place it.
    /// `position: fixed` boxes stay where they are (they are placed against the
    /// viewport, not the page).
    pub fn moved(mut self, dx: f64, dy: f64) -> Self {
        for b in self.boxes.iter_mut().filter(|b| !b.fixed) {
            b.rect = Rect::new(b.rect.x + dx, b.rect.y + dy, b.rect.width, b.rect.height);
            if let Some(c) = &mut b.clip {
                *c = Rect::new(c.x + dx, c.y + dy, c.width, c.height);
            }
        }
        self.root = Rect::new(self.root.x + dx, self.root.y + dy, self.root.width, self.root.height);
        self
    }
    /// `(id, accessible name)` for elements with an id and a name or text
    /// ([`Placed::accessible_name`]: [`El::name`] first).
    pub fn names(&self) -> impl Iterator<Item = (&str, &str)> {
        self.boxes.iter().filter_map(|b| Some((b.id.as_deref()?, b.accessible_name()?)))
    }
    /// The accessibility nodes of the tree: every element with an id, with its role,
    /// accessible name ([`El::name`] wins over text), checked state and rect, in paint
    /// order. What a host hands its platform accessibility tree.
    pub fn a11y(&self) -> Vec<A11yNode> {
        self.boxes
            .iter()
            .filter_map(|b| {
                Some(A11yNode {
                    id: b.id.clone()?,
                    role: b.role(),
                    name: b.accessible_name().unwrap_or_default().to_string(),
                    checked: b.control.as_ref().and_then(Control::checked),
                    disabled: b.kind == "button" && b.style.opacity.is_some_and(|o| o < 0.6),
                    rect: b.rect,
                    focusable: b.tab || matches!(b.control, Some(Control::Input(_))),
                })
            })
            .collect()
    }
    /// `(slot key, content box)` of every embed slot ([`El::slot`]), in paint order.
    pub fn slots(&self) -> impl Iterator<Item = (&str, Rect)> {
        self.boxes.iter().filter_map(|b| Some((b.slot.as_deref()?, content_box(b))))
    }
    /// The placed element `id`.
    pub fn get(&self, id: &str) -> Option<&Placed> {
        self.boxes.iter().find(|b| b.id.as_deref() == Some(id))
    }
    /// The topmost element with an id containing `(x, y)`.
    pub fn hit(&self, x: f64, y: f64) -> Option<&str> {
        let mut found = None;
        for (id, r) in self.hits() {
            if x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height {
                found = Some(id);
            }
        }
        found
    }
}

/// One accessibility node of a DOM tree ([`Dom::a11y`]).
#[derive(Clone, Debug, PartialEq)]
pub struct A11yNode {
    pub id: String,
    pub role: &'static str,
    pub name: String,
    pub checked: Option<bool>,
    pub disabled: bool,
    pub rect: Rect,
    pub focusable: bool,
}

struct Resolved {
    kind: String,
    id: Option<String>,
    style: Style,
    text: Option<String>,
    tab: bool,
    name: Option<String>,
    svg: Option<String>,
    anim: Option<AnimNode>,
    /// Measured text: break units and line height.
    measured: Option<(Units, f64)>,
    /// Width of a space in the element's font (wrapping text).
    space: f64,
    /// Measured inline runs.
    runs: Option<std::rc::Rc<RunsMeasured>>,
    control: Option<Control>,
    slot: Option<String>,
    /// A text input's display char boundaries.
    bounds: Vec<f64>,
    children: Vec<Resolved>,
}

/// Token kinds of inline runs.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Tok {
    Word,
    Space,
    Newline,
}

/// A measured token of inline runs: a word, a run of spaces, or a newline.
#[derive(Clone, Debug)]
struct RunTok {
    run: usize,
    kind: Tok,
    text: String,
    /// Char offset in the whole text.
    start: usize,
    /// Chip padding before / after (the run's first / last token).
    lp: f64,
    rp: f64,
    /// Char boundary x from the token start; the last is its text width.
    bounds: Vec<f64>,
}

/// Inline runs measured at resolve time (layout and placing need no render context).
#[derive(Debug)]
struct RunsMeasured {
    toks: Vec<RunTok>,
    runs: Vec<PlacedRun>,
    lh: f64,
    wrap: bool,
    anywhere: bool,
    clamp: usize,
    text: String,
}

fn measure_runs(ctx: &mut dyn RenderContext, sheet: &Sheet, el: &El, s: &Style, up: &[Node<'_>]) -> RunsMeasured {
    let mut toks = Vec::new();
    let mut runs = Vec::new();
    let mut text = String::new();
    let mut off = 0usize;
    for (ri, run) in el.runs.iter().enumerate() {
        let mut rs = if up.is_empty() { sheet.resolve(&Node::parse(&run.spec), Some(s)) } else { sheet.resolve_in(&Node::parse(&run.spec), up, Some(s)) };
        if let Some(css) = &run.css {
            for d in css.split(';') {
                if let Some((k, v)) = d.split_once(':') {
                    apply_decl(&mut rs, k.trim(), v.trim());
                }
            }
        }
        ctx.set_font(&font_str(&rs));
        let pad = rs.pad();
        let first = toks.len();
        // Some backends trim spaces when measuring: measure spaces between two glyphs.
        let xx = ctx.measure_text("xx");
        let mut push = |ctx: &mut dyn RenderContext, kind: Tok, t: &str, start: usize| {
            let n = t.chars().count();
            let bounds: Vec<f64> = match kind {
                Tok::Newline => vec![0.0],
                Tok::Space => {
                    let w = ctx.measure_text(&format!("x{t}x")) - xx;
                    (0..=n).map(|i| w * i as f64 / n.max(1) as f64).collect()
                }
                Tok::Word => {
                    let mut b = vec![0.0];
                    let mut acc = String::new();
                    for ch in t.chars() {
                        acc.push(ch);
                        b.push(ctx.measure_text(&acc));
                    }
                    b
                }
            };
            toks.push(RunTok { run: ri, kind, text: t.to_string(), start, lp: 0.0, rp: 0.0, bounds });
        };
        let mut cur = String::new();
        let mut cur_start = off;
        let mut cur_sp = false;
        for (ci, ch) in run.text.chars().enumerate() {
            let at = off + ci;
            if ch == '\n' {
                if !cur.is_empty() {
                    push(ctx, if cur_sp { Tok::Space } else { Tok::Word }, &std::mem::take(&mut cur), cur_start);
                }
                push(ctx, Tok::Newline, "", at);
                cur_start = at + 1;
                continue;
            }
            let sp = ch == ' ';
            if !cur.is_empty() && sp != cur_sp {
                push(ctx, if cur_sp { Tok::Space } else { Tok::Word }, &std::mem::take(&mut cur), cur_start);
            }
            if cur.is_empty() {
                cur_start = at;
            }
            cur_sp = sp;
            cur.push(ch);
        }
        if !cur.is_empty() {
            push(ctx, if cur_sp { Tok::Space } else { Tok::Word }, &cur, cur_start);
        }
        if toks.len() > first {
            toks[first].lp = pad[3];
            toks.last_mut().unwrap().rp = pad[1];
        }
        off += run.text.chars().count();
        text.push_str(&run.text);
        runs.push(PlacedRun { id: run.id.clone(), style: rs, text: run.text.clone() });
    }
    RunsMeasured {
        toks,
        runs,
        lh: font_of(s).2,
        wrap: wraps(s),
        anywhere: matches!(s.overflow_wrap.as_deref(), Some("anywhere" | "break-word")),
        clamp: clamp_of(s),
        text,
    }
}

/// Greedy wrap of measured runs at `limit` (`None` or not wrapping: only newlines break).
/// Spaces at a line start are dropped, spaces before a wrapped word too; with `anywhere`
/// a word too long for a line breaks between chars.
fn wrap_runs(m: &RunsMeasured, limit: Option<f64>) -> Vec<RunLine> {
    let max_w = if m.wrap { limit.unwrap_or(f64::INFINITY) } else { f64::INFINITY };
    let mut lines: Vec<Vec<RunPiece>> = vec![Vec::new()];
    let mut x = 0.0;
    let piece = |t: &RunTok, a: usize, b: usize, x: f64, lp: f64, rp: f64| {
        let base = t.bounds[a];
        let bounds: Vec<f64> = t.bounds[a..=b].iter().map(|v| v - base).collect();
        let w = bounds[b - a] + lp + rp;
        RunPiece { run: t.run, x, w, pad: lp, text: t.text.chars().skip(a).take(b - a).collect(), start: t.start + a, bounds }
    };
    for t in &m.toks {
        let n = t.bounds.len() - 1;
        let w = t.bounds[n] + t.lp + t.rp;
        match t.kind {
            Tok::Newline => {
                lines.push(Vec::new());
                x = 0.0;
                continue;
            }
            Tok::Space => {
                if x > 0.0 && x + w <= max_w {
                    lines.last_mut().unwrap().push(piece(t, 0, n, x, t.lp, t.rp));
                    x += w;
                } else if x > 0.0 {
                    lines.push(Vec::new());
                    x = 0.0;
                }
                continue;
            }
            Tok::Word => {}
        }
        if x + w <= max_w {
            lines.last_mut().unwrap().push(piece(t, 0, n, x, t.lp, t.rp));
            x += w;
            continue;
        }
        if x > 0.0 && (w <= max_w || !m.anywhere) {
            if let Some(l) = lines.last_mut() {
                while l.last().is_some_and(|p| p.text.trim().is_empty()) {
                    l.pop();
                }
            }
            lines.push(vec![piece(t, 0, n, 0.0, t.lp, t.rp)]);
            x = w;
            continue;
        }
        if !m.anywhere {
            lines.last_mut().unwrap().push(piece(t, 0, n, x, t.lp, t.rp));
            x += w;
            continue;
        }
        // Break anywhere: fill the line char by char, then carry on below.
        let (mut a, mut lp) = (0, t.lp);
        loop {
            let mut b = a;
            while b < n && x + lp + (t.bounds[b + 1] - t.bounds[a]) <= max_w {
                b += 1;
            }
            if b == a {
                if x > 0.0 {
                    lines.push(Vec::new());
                    x = 0.0;
                    continue;
                }
                b = a + 1;
            }
            let last = b == n;
            let p = piece(t, a, b, x, lp, if last { t.rp } else { 0.0 });
            x += p.w;
            lines.last_mut().unwrap().push(p);
            if last {
                break;
            }
            lines.push(Vec::new());
            x = 0.0;
            a = b;
            lp = 0.0;
        }
    }
    lines.truncate(m.clamp.max(1));
    lines
        .into_iter()
        .enumerate()
        .map(|(i, pieces)| {
            let width = pieces.iter().rev().find(|p| !p.text.trim().is_empty()).map_or(0.0, |p| p.x + p.w);
            RunLine { y: m.lh * i as f64, height: m.lh, width, pieces }
        })
        .collect()
}

fn font_of(s: &Style) -> (u32, f64, f64, String) {
    let px = s.font_size.unwrap_or(13.0);
    let lh = s.line_height.unwrap_or((px * 1.5 * 100.0).round() / 100.0);
    (s.font_weight.unwrap_or(400), px, lh, s.font_family.clone().unwrap_or_else(|| "Inter, sans-serif".into()))
}

fn font_str(s: &Style) -> String {
    let (w, px, _, fam) = font_of(s);
    match s.font_style.as_deref() {
        Some(st @ ("italic" | "oblique")) => format!("{st} {w} {px}px {fam}"),
        _ => format!("{w} {px}px {fam}"),
    }
}

/// `up`: the ancestors' selector nodes (root first) when the sheet has descendant /
/// child selectors (`None` otherwise, to skip the bookkeeping).
fn resolve<'e>(
    ctx: &mut dyn RenderContext,
    sheet: &Sheet,
    el: &'e El,
    parent: Option<&Style>,
    state: &dyn Fn(&str) -> State,
    up: &mut Option<Vec<Node<'e>>>,
) -> Option<Resolved> {
    let mut st = el.id.as_deref().map_or(State::NONE, state);
    match &el.control {
        Some(c) if c.checked() == Some(true) => st = st | State::CHECKED,
        Some(Control::Input(e)) if e.caret.is_some() => st = st | State::FOCUS,
        _ => {}
    }
    let node = Node::parse(&el.spec).state(st);
    let mut s = match up.as_deref() {
        Some(u) if !u.is_empty() => sheet.resolve_in(&node, u, parent),
        _ => sheet.resolve(&node, parent),
    };
    if let Some(css) = &el.css {
        for d in css.split(';') {
            if let Some((k, v)) = d.split_once(':') {
                apply_decl(&mut s, k.trim(), v.trim());
            }
        }
    }
    if s.display.as_deref() == Some("none") {
        return None;
    }
    let runs = (!el.runs.is_empty()).then(|| std::rc::Rc::new(measure_runs(ctx, sheet, el, &s, up.as_deref().unwrap_or(&[]))));
    let text = if runs.is_some() || matches!(el.control, Some(Control::Input(_))) { None } else { el.text.clone() };
    let mut measured = text.as_ref().map(|t| {
        ctx.set_font(&font_str(&s));
        (units(ctx, t, wraps(&s)), font_of(&s).2)
    });
    let mut bounds = Vec::new();
    if let Some(Control::Input(e)) = &el.control {
        // One unwrapped line: the shown text (or the placeholder) sizes an auto width.
        ctx.set_font(&font_str(&s));
        let shown = e.display();
        bounds.push(0.0);
        let mut acc = String::new();
        for ch in shown.chars() {
            acc.push(ch);
            bounds.push(ctx.measure_text(&acc));
        }
        let t = if e.shows_placeholder() { e.placeholder.clone() } else { shown };
        let w = ctx.measure_text(&t).ceil();
        measured = Some((vec![(t, w, false, false)], font_of(&s).2));
    }
    let space = if text.is_some() && wraps(&s) { ctx.measure_text(" ") } else { 0.0 };
    let children = if let Some(u) = up.as_mut() {
        u.push(node);
        let kids: Vec<Resolved> = el.children.iter().filter_map(|c| resolve(ctx, sheet, c, Some(&s), state, up)).collect();
        if let Some(u) = up.as_mut() {
            u.pop();
        }
        kids
    } else {
        el.children.iter().filter_map(|c| resolve(ctx, sheet, c, Some(&s), state, up)).collect()
    };
    let kind = el.spec.split(['.', '#', ':']).next().unwrap_or("div").to_string();
    Some(Resolved {
        kind,
        id: el.id.clone(),
        style: s,
        text,
        tab: el.tab,
        name: el.name.clone(),
        svg: el.svg.clone(),
        anim: el.anim.clone(),
        measured,
        space,
        runs,
        control: el.control.clone(),
        slot: el.slot.clone(),
        bounds,
        children,
    })
}

/// The most lines wrapping text may take (`line-clamp`).
fn clamp_of(s: &Style) -> usize {
    s.line_clamp.map_or(usize::MAX, usize::from)
}

fn wraps(s: &Style) -> bool {
    matches!(s.white_space.as_deref(), Some("normal") | Some("pre-wrap"))
}

/// Text cut at its break opportunities: `(segment, its width, a space before it, it starts a
/// new line)` per unit; unwrapped text is one unit.
type Units = Vec<(String, f64, bool, bool)>;

fn units(ctx: &mut dyn RenderContext, t: &str, wrap: bool) -> Units {
    if !wrap {
        return vec![(t.to_string(), ctx.measure_text(t).ceil(), false, false)];
    }
    let mut out = Vec::new();
    for (pi, para) in t.split('\n').enumerate() {
        for (wi, word) in para.split(' ').enumerate() {
            for (si, seg) in crate::ui::widgets::atomic::bubble::hyphen_segments(word).into_iter().enumerate() {
                out.push((seg.to_string(), ctx.measure_text(seg), si == 0 && wi > 0, si == 0 && wi == 0 && pi > 0));
            }
        }
    }
    out
}

/// Greedy line breaking of `u` at `limit` (`None` = no limit): the lines' unit ranges and
/// widths. `space` is the width of one space.
fn break_lines(u: &Units, space: f64, limit: Option<f64>) -> Vec<(usize, usize, f64)> {
    let mut lines = Vec::new();
    let (mut start, mut w) = (0, 0.0);
    for (i, (_, sw, sp, newline)) in u.iter().enumerate() {
        let add = if i == start { *sw } else { w + if *sp { space } else { 0.0 } + sw };
        if i > start && (*newline || limit.is_some_and(|l| add > l + 0.5)) {
            lines.push((start, i, w));
            start = i;
            w = *sw;
        } else {
            w = add;
        }
    }
    if start < u.len() {
        lines.push((start, u.len(), w));
    }
    lines
}

fn line_text(u: &Units, a: usize, b: usize) -> String {
    let mut s = String::new();
    for (i, (seg, _, sp, _)) in u[a..b].iter().enumerate() {
        if i > 0 && *sp {
            s.push(' ');
        }
        s.push_str(seg);
    }
    s
}

fn lbox<'a>(r: &Resolved, measures: &'a [Box<dyn Fn(Option<f64>) -> (f64, f64)>], next: &mut usize) -> LBox<'a> {
    let mut b = LBox::new(r.style.clone());
    if r.measured.is_some() || r.runs.is_some() {
        b = b.text(measures[*next].as_ref());
        *next += 1;
    }
    for c in &r.children {
        b = b.child(lbox(c, measures, next));
    }
    b
}

fn collect_measures(r: &Resolved, out: &mut Vec<Box<dyn Fn(Option<f64>) -> (f64, f64)>>) {
    if let Some(m) = &r.runs {
        let m = m.clone();
        out.push(Box::new(move |lim: Option<f64>| {
            let ls = wrap_runs(&m, lim);
            (ls.iter().map(|l| l.width).fold(0.0, f64::max).ceil(), m.lh * ls.len().max(1) as f64)
        }));
    } else if let Some((u, lh)) = &r.measured {
        if wraps(&r.style) {
            // Wrapping: min-content is the widest unit, else the lines at the limit.
            let (u, lh, space, clamp) = (u.clone(), *lh, r.space, clamp_of(&r.style));
            out.push(Box::new(move |lim: Option<f64>| {
                let mut ls = break_lines(&u, space, lim);
                ls.truncate(clamp);
                (ls.iter().map(|l| l.2).fold(0.0, f64::max).ceil(), lh * ls.len().max(1) as f64)
            }));
        } else {
            // One line: min-content is the whole line (nowrap); a narrower box ellipsizes.
            let (w, lh) = (u[0].1, *lh);
            out.push(Box::new(move |lim: Option<f64>| (lim.filter(|l| *l > 0.0).map_or(w, |l| w.min(l)), lh)));
        }
    }
    for c in &r.children {
        collect_measures(c, out);
    }
}

/// `alpha`: the ancestors' opacity; a placed box carries its effective opacity (CSS
/// `opacity` applies to the whole subtree).
/// Positioned subtrees (absolute / relative) leave the flow's paint order: they are
/// collected in `layers` with their `z-index` and painted after it.
/// `fixed`: inside a `position: fixed` subtree. Fixed subtrees form the top layers
/// (above every non-fixed one, by `z-index` among themselves).
#[allow(clippy::too_many_arguments)]
fn place(r: Resolved, l: &Laid, out: &mut Vec<Placed>, layers: &mut Vec<(bool, i32, Vec<Placed>)>, alpha: f64, fixed: bool, clip: Option<Rect>) {
    let mut style = r.style;
    let inner = if style.clips() {
        let e = style.border_edges();
        let pb = Rect::new(l.rect.x + e[3], l.rect.y + e[0], (l.rect.width - e[1] - e[3]).max(0.0), (l.rect.height - e[0] - e[2]).max(0.0));
        Some(clip.map_or(pb, |c| c.intersect(pb)))
    } else {
        clip
    };
    let a = alpha * style.opacity.unwrap_or(1.0);
    style.opacity = (a < 1.0).then_some(a);
    let inline = r.runs.map(|m| {
        let c = content_rect(&style, l.rect);
        let mut lines = wrap_runs(&m, Some(c.width));
        // A box taller than its lines centres them (as plain text); text-align moves each line.
        let top = ((c.height - m.lh * lines.len() as f64) / 2.0).max(0.0);
        for ln in &mut lines {
            ln.y += top;
            let dx = match style.text_align.as_deref() {
                Some("center") => (c.width - ln.width) / 2.0,
                Some("right") | Some("end") => c.width - ln.width,
                _ => 0.0,
            };
            if dx != 0.0 {
                for pc in &mut ln.pieces {
                    pc.x += dx;
                }
            }
        }
        let (runs, text) = std::rc::Rc::try_unwrap(m).map(|m| (m.runs, m.text)).unwrap_or_else(|rc| (rc.runs.clone(), rc.text.clone()));
        Box::new(Inline { runs, lines, text })
    });
    out.push(Placed {
        id: r.id,
        kind: r.kind,
        rect: l.rect,
        style,
        text: r.text,
        tab: r.tab,
        name: r.name,
        svg: r.svg,
        anim: r.anim,
        inline,
        control: r.control,
        slot: r.slot,
        fixed,
        clip,
        bounds: r.bounds,
    });
    for (c, cl) in r.children.into_iter().zip(&l.children) {
        if c.style.positioned() {
            let z = c.style.z_index.unwrap_or(0);
            let f = fixed || c.style.fixed();
            // Reserve the slot first: positioned descendants inside it paint above it.
            let at = layers.len();
            layers.push((f, z, Vec::new()));
            let mut sub = Vec::new();
            // A fixed box escapes its ancestors' overflow clips.
            let cc = if f && !fixed { None } else { inner };
            place(c, cl, &mut sub, layers, a, f, cc);
            layers[at].2 = sub;
        } else {
            place(c, cl, out, layers, a, fixed, inner);
        }
    }
}

/// Lay out `root` at `(x, y)` in a containing block `w` wide, without painting.
/// `state(id)` gives an element's interaction state (hover, focus-visible, ...).
pub fn lay_out(ctx: &mut dyn RenderContext, sheet: &Sheet, root: &El, x: f64, y: f64, w: f64, state: &dyn Fn(&str) -> State) -> Dom {
    lay_out_in(ctx, sheet, root, x, y, w, None, state)
}

/// [`lay_out`] in a `viewport` (window coordinates; the root's box when `None`):
/// `position: fixed` elements are placed against it and stay put when the tree
/// scrolls ([`Dom::moved`]); they paint above everything else (a popover, a toast, a
/// sticky composer over a scrolled page). `%` heights of the root resolve against it.
#[allow(clippy::too_many_arguments)]
pub fn lay_out_in(ctx: &mut dyn RenderContext, sheet: &Sheet, root: &El, x: f64, y: f64, w: f64, viewport: Option<Rect>, state: &dyn Fn(&str) -> State) -> Dom {
    let mut up = sheet.has_complex().then(Vec::new);
    let Some(r) = resolve(ctx, sheet, root, None, state, &mut up) else { return Dom::default() };
    let mut measures = Vec::new();
    collect_measures(&r, &mut measures);
    let laid = {
        let mut next = 0;
        let b = lbox(&r, &measures, &mut next);
        layout_in(&b, x, y, w, viewport)
    };
    drop(measures);
    let mut flow = Vec::new();
    let mut layers = Vec::new();
    let root_fixed = r.style.fixed();
    place(r, &laid, &mut flow, &mut layers, 1.0, root_fixed, None);
    if layers.is_empty() {
        return Dom { boxes: flow, root: laid.rect };
    }
    // Paint order: the root, negative z-index layers, the flow, then the other layers by
    // z-index (ties in document order).
    crate::sort::sort_by_key(&mut layers, |l| (l.0, l.1));
    let mut boxes = Vec::with_capacity(flow.len() + layers.iter().map(|l| l.2.len()).sum::<usize>());
    let mut flow = flow.into_iter();
    boxes.extend(flow.next());
    let split = layers.iter().position(|l| l.0 || l.1 >= 0).unwrap_or(layers.len());
    let mut layers = layers.into_iter();
    for (_, _, l) in layers.by_ref().take(split) {
        boxes.extend(l);
    }
    boxes.extend(flow);
    for (_, _, l) in layers {
        boxes.extend(l);
    }
    Dom { boxes, root: laid.rect }
}

/// Paint one placed box: shadow, background, border, text, kind content.
pub fn paint_box(ctx: &mut dyn RenderContext, b: &Placed) {
    let s = &b.style;
    if let Some(a) = s.opacity {
        ctx.set_global_alpha(a);
    }
    let r = b.rect;
    let shadow: Option<Shadow> = s.box_shadow.as_deref().and_then(<Option<Shadow>>::from_css).flatten();
    let bg = s.background.as_deref().filter(|b| *b != "transparent" && *b != "none");
    if s.border_radii.is_some() || s.border_widths.is_some() || s.border_colors.is_some() {
        paint_frame(ctx, r, s, bg, shadow);
    } else {
        let rad = s.border_radius.unwrap_or(0.0);
        let bw = s.border_width.unwrap_or(0.0);
        let border = s.border_color.as_deref().filter(|_| bw > 0.0 && s.border_drawn());
        if let Some(sh) = shadow {
            ctx.set_shadow(0.0, sh.dy, sh.blur, sh.color);
        }
        if let Some(bg) = bg {
            fill_round_rect(ctx, r, rad, bg);
        } else if let Some(sh) = shadow {
            // A shadow needs something to cast it.
            fill_round_rect(ctx, r, rad, sh.color);
        }
        ctx.clear_shadow();
        if let Some(bc) = border {
            ctx.set_stroke_color(bc);
            ctx.set_stroke_width(bw);
            let dash = s.border_dash(bw);
            if let Some(d) = dash {
                ctx.set_line_dash(&d);
            }
            path_round_rect(ctx, Rect::new(r.x + bw / 2.0, r.y + bw / 2.0, r.width - bw, r.height - bw), (rad - bw / 2.0).max(0.0));
            ctx.stroke();
            if dash.is_some() {
                ctx.set_line_dash(&[]);
            }
        }
    }
    let p = s.pad();
    let be = s.border_edges();
    let content = content_rect(s, r);
    let color = s.color.as_deref().unwrap_or("#000000");
    if let Some(inl) = &b.inline {
        paint_runs(ctx, inl, content, color);
    }
    if let Some(t) = &b.text {
        let (_, px, lh, _) = font_of(s);
        ctx.set_font(&font_str(s));
        let lines: Vec<String> = if wraps(s) {
            let u = units(ctx, t, true);
            let space = ctx.measure_text(" ");
            let ls = break_lines(&u, space, Some(content.width));
            let clamp = clamp_of(s);
            let mut v: Vec<String> = ls.iter().take(clamp).map(|&(a, z, _)| line_text(&u, a, z)).collect();
            if ls.len() > clamp {
                // The clamp cuts after the last kept line and puts the ellipsis at its end.
                if let Some(l) = v.last_mut() {
                    let e = ellipsize(ctx, &format!("{}…", l.trim_end()), content.width + 0.5);
                    *l = if e.ends_with('…') { e } else { format!("{e}…") };
                }
            }
            v
        } else {
            vec![ellipsize(ctx, t, content.width + 0.5)]
        };
        let (align, x) = match s.text_align.as_deref() {
            Some("center") => (TextAlign::Center, content.x + content.width / 2.0),
            Some("right") | Some("end") => (TextAlign::Right, content.x + content.width),
            _ => (TextAlign::Left, content.x),
        };
        // A box taller than its text centres it (flex `align-items:center`, the usual control).
        let top = content.y + ((content.height - lh * lines.len() as f64) / 2.0).max(0.0);
        ctx.set_fill_color(color);
        for (i, l) in lines.iter().enumerate() {
            fill_text_css(ctx, l, x, top + lh * i as f64, px, lh, INTER, align);
        }
    }
    if let Some(svg) = &b.svg {
        if s.shape_rendering.as_deref() == Some("geometricPrecision") {
            crate::render::draw_svg_icon_exact(ctx, svg, content.x, content.y, content.width, content.height, color);
        } else {
            crate::render::draw_svg_icon(ctx, svg, content.x, content.y, content.width, content.height, color);
        }
    }
    if let Some(ctl) = &b.control {
        paint_control(ctx, b, ctl, content, color);
    }
    if b.kind == "select" {
        let c = s.custom("--chevron").unwrap_or(color);
        let (ax, ay) = (r.x + r.width - be[1] - p[1].min(20.0).max(12.0) / 2.0 - 8.0, r.y + r.height / 2.0 - 2.0);
        chevron(ctx, ax.max(content.x + content.width + 2.0), ay, c);
    }
    if s.opacity.is_some() {
        ctx.set_global_alpha(1.0);
    }
}

/// Paint inline runs in content box `c`: each run's chip (a run with a background) once per
/// line it is on, spanning its pieces there, then the text in the run's font and colour
/// (`fg` when the run sets none).
fn paint_runs(ctx: &mut dyn RenderContext, inl: &Inline, c: Rect, fg: &str) {
    for line in &inl.lines {
        let mut i = 0;
        while i < line.pieces.len() {
            let p = &line.pieces[i];
            let mut j = i + 1;
            while j < line.pieces.len() && line.pieces[j].run == p.run {
                j += 1;
            }
            let last = &line.pieces[j - 1];
            if let Some(rs) = inl.runs.get(p.run).map(|r| &r.style) {
                if let Some(bg) = rs.background.as_deref().filter(|b| *b != "transparent" && *b != "none") {
                    // The chip: a 1.2em content box plus the run's vertical padding, centred in the line.
                    let (_, px, _, _) = font_of(rs);
                    let rp = rs.pad();
                    let h = (px * 1.2 + rp[0] + rp[2]).round();
                    let chip = Rect::new(c.x + p.x, c.y + line.y + ((line.height - h) / 2.0).round(), last.x + last.w - p.x, h);
                    let rad = fit_radii(chip, rs.radii());
                    ctx.begin_path();
                    add_radii_path(ctx, chip, rad, false);
                    ctx.set_fill_color(bg);
                    ctx.fill();
                }
            }
            i = j;
        }
        for p in &line.pieces {
            let Some(run) = inl.runs.get(p.run) else { continue };
            let rs = &run.style;
            let (_, px, _, _) = font_of(rs);
            ctx.set_font(&font_str(rs));
            ctx.set_fill_color(rs.color.as_deref().unwrap_or(fg));
            fill_text_css(ctx, &p.text, c.x + p.x + p.pad, c.y + line.y, px, line.height, INTER, TextAlign::Left);
        }
    }
}

/// Paint a built-in control into content box `c` (`fg`: the element's text colour).
fn paint_control(ctx: &mut dyn RenderContext, b: &Placed, ctl: &Control, c: Rect, fg: &str) {
    let s = &b.style;
    let num = |k: &str, d: f64| s.custom(k).and_then(|v| v.trim().trim_end_matches("px").parse::<f64>().ok()).unwrap_or(d);
    match ctl {
        Control::Switch { pos, .. } => {
            let d = c.height.min(c.width);
            let x = c.x + (c.width - d) * pos.clamp(0.0, 1.0);
            ctx.set_fill_color(s.custom("--knob").unwrap_or("#ffffff"));
            ctx.begin_path();
            ctx.arc(x + d / 2.0, c.y + c.height / 2.0, d / 2.0, 0.0, std::f64::consts::TAU);
            ctx.fill();
        }
        Control::Checkbox { on, mixed } => {
            if !on && !mixed {
                return;
            }
            let k = s.custom("--check").unwrap_or(fg);
            let w = num("--check-width", 2.0);
            let (x, y, sz) = (c.x, c.y, c.width.min(c.height));
            ctx.set_stroke_color(k);
            ctx.set_stroke_width(w);
            ctx.set_line_cap("round");
            ctx.begin_path();
            if *mixed {
                ctx.move_to(x + sz * 0.2, y + sz * 0.5);
                ctx.line_to(x + sz * 0.8, y + sz * 0.5);
            } else {
                ctx.move_to(x + sz * 0.18, y + sz * 0.52);
                ctx.line_to(x + sz * 0.42, y + sz * 0.76);
                ctx.line_to(x + sz * 0.84, y + sz * 0.26);
            }
            ctx.stroke();
            ctx.set_line_cap("butt");
        }
        Control::Spinner { t } => {
            let r = c.width.min(c.height) / 2.0;
            let w = num("--spin-width", (r / 4.0).clamp(1.5, 4.0));
            let (cx, cy, rr) = (c.x + c.width / 2.0, c.y + c.height / 2.0, (r - w / 2.0).max(0.5));
            ctx.set_stroke_width(w);
            if let Some(tr) = s.custom("--spin-track") {
                ctx.set_stroke_color(tr);
                ctx.begin_path();
                ctx.arc(cx, cy, rr, 0.0, std::f64::consts::TAU);
                ctx.stroke();
            }
            let a = (t * num("--spin-speed", 1.0)).fract() * std::f64::consts::TAU;
            ctx.set_stroke_color(fg);
            ctx.set_line_cap("round");
            ctx.begin_path();
            ctx.arc(cx, cy, rr, a, a + std::f64::consts::PI * 1.5);
            ctx.stroke();
            ctx.set_line_cap("butt");
        }
        Control::Input(e) => {
            let (_, px, lh, _) = font_of(s);
            let top = c.y + ((c.height - lh) / 2.0).max(0.0);
            let scroll = b.input_scroll();
            ctx.save();
            ctx.begin_path();
            ctx.rect(c.x, c.y, c.width, c.height);
            ctx.clip();
            ctx.set_font(&font_str(s));
            let x0 = c.x - scroll;
            if e.shows_placeholder() {
                ctx.set_fill_color(s.custom("--placeholder").unwrap_or("#8a8a8a"));
                fill_text_css(ctx, &e.placeholder, x0, top, px, lh, INTER, TextAlign::Left);
            } else {
                let at = |i: usize| b.bounds.get(i).copied().unwrap_or(0.0);
                if let (Some(a), Some(k)) = (e.anchor, e.caret) {
                    let (lo, hi) = (a.min(k), a.max(k));
                    if lo != hi && e.preedit.is_none() {
                        ctx.set_fill_color(s.custom("--selection").unwrap_or("rgba(59,130,246,0.35)"));
                        ctx.fill_rect(x0 + at(lo), top, at(hi) - at(lo), lh);
                    }
                }
                ctx.set_fill_color(fg);
                fill_text_css(ctx, &e.display(), x0, top, px, lh, INTER, TextAlign::Left);
                if let (Some(p), Some(k)) = (&e.preedit, e.caret) {
                    let (a, z) = (at(k), at(k + p.chars().count()));
                    ctx.set_stroke_color(fg);
                    ctx.set_stroke_width(1.0);
                    ctx.begin_path();
                    ctx.move_to(x0 + a, top + lh - 2.5);
                    ctx.line_to(x0 + z, top + lh - 2.5);
                    ctx.stroke();
                }
            }
            if let (true, Some(cr)) = (e.caret_on, b.caret_rect()) {
                ctx.set_fill_color(s.custom("--caret").unwrap_or(fg));
                ctx.fill_rect(cr.x, cr.y + 2.0, 1.0, (cr.height - 4.0).max(1.0));
            }
            ctx.restore();
        }
    }
}

/// The native `<select>` chevron (8×4, 2px round stroke) with its left end at `(x, y)`.
pub fn chevron(ctx: &mut dyn RenderContext, x: f64, y: f64, color: &str) {
    ctx.set_stroke_color(color);
    ctx.set_stroke_width(2.0);
    ctx.set_line_cap("round");
    ctx.begin_path();
    ctx.move_to(x, y);
    ctx.line_to(x + 4.0, y + 4.0);
    ctx.line_to(x + 8.0, y);
    ctx.stroke();
    ctx.set_line_cap("butt");
}

/// Paint a laid-out tree (document order: parents under children).
pub fn paint(ctx: &mut dyn RenderContext, dom: &Dom) {
    paint_slots(ctx, dom, &mut |_, _, _| {});
}

/// [`paint`], calling `slot(ctx, key, content box)` where each embed slot
/// ([`El::slot`]) is in paint order, so a kit widget drawn there sits under later
/// layers (a popover, a fixed panel) and inside `overflow: hidden` ancestors.
pub fn paint_slots(ctx: &mut dyn RenderContext, dom: &Dom, slot: &mut dyn FnMut(&mut dyn RenderContext, &str, Rect)) {
    for b in &dom.boxes {
        with_clip(ctx, b, |ctx| {
            paint_box(ctx, b);
            if let Some(k) = &b.slot {
                slot(ctx, k, content_box(b));
            }
        });
    }
}

fn with_clip(ctx: &mut dyn RenderContext, b: &Placed, f: impl FnOnce(&mut dyn RenderContext)) {
    match b.clip {
        Some(c) => {
            if c.width <= 0.0 || c.height <= 0.0 {
                return;
            }
            ctx.save();
            ctx.begin_path();
            ctx.rect(c.x, c.y, c.width, c.height);
            ctx.clip();
            f(ctx);
            ctx.restore();
        }
        None => f(ctx),
    }
}

/// One frame of a tree's animation nodes ([`paint_with`], [`paint_anim`]).
pub struct AnimFrame<'a> {
    /// Seconds on the host's clock.
    pub t: f64,
    /// What can be seen, in the tree's coordinates (a scroll viewport, the window):
    /// nodes outside it do not run.
    pub visible: Rect,
    /// The window pointer, for nodes that track it.
    pub pointer: Option<(f64, f64)>,
    /// The pointer if `id` is the topmost hit under it (z-order safe, as the framework's
    /// `pointer_over`): what other nodes see, so a drawer above one hides the pointer
    /// from it.
    pub pointer_over: &'a dyn Fn(&str) -> Option<(f64, f64)>,
    /// Hidden page or reduced motion: nodes paint frame `t` but ask for no next frame.
    pub paused: bool,
}

impl<'a> AnimFrame<'a> {
    /// A frame at `t` showing `visible`, without a pointer.
    pub fn new(t: f64, visible: Rect) -> Self {
        Self { t, visible, pointer: None, pointer_over: &|_| None, paused: false }
    }
}

/// What the animation nodes of a frame need next.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AnimReport {
    /// Nodes that ran.
    pub painted: usize,
    /// Seconds until the next frame some running node needs (`None`: all still).
    pub next: Option<f64>,
    /// A painted node tracks the pointer: repaint when it moves.
    pub tracks_pointer: bool,
}

/// The content box of a placed element.
pub fn content_box(b: &Placed) -> Rect {
    content_rect(&b.style, b.rect)
}

fn content_rect(s: &Style, r: Rect) -> Rect {
    let (e, p) = (s.border_edges(), s.pad());
    Rect::new(r.x + p[3] + e[3], r.y + p[0] + e[0], (r.width - p[1] - p[3] - e[1] - e[3]).max(0.0), (r.height - p[0] - p[2] - e[0] - e[2]).max(0.0))
}

/// Corner radii scaled down as CSS does when adjacent ones overflow a side.
fn fit_radii(r: Rect, rad: [[f64; 2]; 4]) -> [[f64; 2]; 4] {
    let mut f: f64 = 1.0;
    for (sum, len) in [
        (rad[0][0] + rad[1][0], r.width),
        (rad[3][0] + rad[2][0], r.width),
        (rad[0][1] + rad[3][1], r.height),
        (rad[1][1] + rad[2][1], r.height),
    ] {
        if sum > len && sum > 0.0 {
            f = f.min(len.max(0.0) / sum);
        }
    }
    rad.map(|c| [c[0].max(0.0) * f, c[1].max(0.0) * f])
}

/// A rounded rect outline with per-corner elliptical radii (top-left, top-right,
/// bottom-right, bottom-left; `[h, v]`, already fitted) as cubic segments from
/// `start`, clockwise.
fn radii_segments(r: Rect, rad: [[f64; 2]; 4]) -> ((f64, f64), Vec<[(f64, f64); 3]>) {
    const K: f64 = 0.5523;
    let (x0, y0, x1, y1) = (r.x, r.y, r.x + r.width, r.y + r.height);
    let [tl, tr, br, bl] = rad;
    let line = |a: (f64, f64), b: (f64, f64)| [a, b, b];
    let mut v = Vec::with_capacity(8);
    v.push(line((x0 + tl[0], y0), (x1 - tr[0], y0)));
    v.push([(x1 - tr[0] + tr[0] * K, y0), (x1, y0 + tr[1] - tr[1] * K), (x1, y0 + tr[1])]);
    v.push(line((x1, y0 + tr[1]), (x1, y1 - br[1])));
    v.push([(x1, y1 - br[1] + br[1] * K), (x1 - br[0] + br[0] * K, y1), (x1 - br[0], y1)]);
    v.push(line((x1 - br[0], y1), (x0 + bl[0], y1)));
    v.push([(x0 + bl[0] - bl[0] * K, y1), (x0, y1 - bl[1] + bl[1] * K), (x0, y1 - bl[1])]);
    v.push(line((x0, y1 - bl[1]), (x0, y0 + tl[1])));
    v.push([(x0, y0 + tl[1] - tl[1] * K), (x0 + tl[0] - tl[0] * K, y0), (x0 + tl[0], y0)]);
    ((x0 + tl[0], y0), v)
}

/// Add a per-corner rounded rect to the current path; `reverse` winds it
/// counter-clockwise (a hole under the non-zero rule).
fn add_radii_path(ctx: &mut dyn RenderContext, r: Rect, rad: [[f64; 2]; 4], reverse: bool) {
    let (start, segs) = radii_segments(r, rad);
    if !reverse {
        ctx.move_to(start.0, start.1);
        for [c1, c2, e] in segs {
            ctx.bezier_curve_to(c1.0, c1.1, c2.0, c2.1, e.0, e.1);
        }
    } else {
        ctx.move_to(start.0, start.1);
        let mut prev = start;
        // Walk backwards: each segment's end becomes the next start.
        let pts: Vec<((f64, f64), [(f64, f64); 3])> = segs
            .iter()
            .map(|s| {
                let from = prev;
                prev = s[2];
                (from, *s)
            })
            .collect();
        for (from, [c1, c2, _]) in pts.into_iter().rev() {
            ctx.bezier_curve_to(c2.0, c2.1, c1.0, c1.1, from.0, from.1);
        }
    }
    ctx.close_path();
}

/// Start a path along a placed box's border-box outline, its corner radii included
/// (per-corner, elliptical). For a shadow or fill the box style cannot express.
pub fn path_border_box(ctx: &mut dyn RenderContext, b: &Placed) {
    ctx.begin_path();
    add_radii_path(ctx, b.rect, fit_radii(b.rect, b.style.radii()), false);
}

/// Clip to a placed box's padding box with the inner corner radii CSS gives it (outer
/// radius less the border width on each axis): what `overflow: hidden` clips its
/// children to. Pair with `save` / `restore`.
///
/// ```
/// # use uzor::ui::dom::{clip_padding_box, paint_box, Dom};
/// # fn f(ctx: &mut dyn uzor::render::RenderContext, dom: &Dom) {
/// let (frame, kids) = dom.boxes.split_first().unwrap();
/// paint_box(ctx, frame);
/// ctx.save();
/// clip_padding_box(ctx, frame);
/// kids.iter().for_each(|b| paint_box(ctx, b));
/// ctx.restore();
/// # }
/// ```
pub fn clip_padding_box(ctx: &mut dyn RenderContext, b: &Placed) {
    let (r, e) = (b.rect, b.style.border_edges());
    let inner = Rect::new(r.x + e[3], r.y + e[0], (r.width - e[1] - e[3]).max(0.0), (r.height - e[0] - e[2]).max(0.0));
    ctx.begin_path();
    add_radii_path(ctx, inner, inner_radii(fit_radii(r, b.style.radii()), e), false);
    ctx.clip();
}

/// CSS inner (padding-box) radii: each outer radius less the adjacent border widths.
fn inner_radii(rad: [[f64; 2]; 4], e: [f64; 4]) -> [[f64; 2]; 4] {
    [
        [(rad[0][0] - e[3]).max(0.0), (rad[0][1] - e[0]).max(0.0)],
        [(rad[1][0] - e[1]).max(0.0), (rad[1][1] - e[0]).max(0.0)],
        [(rad[2][0] - e[1]).max(0.0), (rad[2][1] - e[2]).max(0.0)],
        [(rad[3][0] - e[3]).max(0.0), (rad[3][1] - e[2]).max(0.0)],
    ]
}

/// Paint a CSS box's background and border with per-corner radii and per-side border
/// widths / colours (the uniform case keeps the classic single-radius path).
fn paint_frame(ctx: &mut dyn RenderContext, r: Rect, s: &Style, bg: Option<&str>, shadow: Option<Shadow>) {
    let rad = fit_radii(r, s.radii());
    let e = s.border_edges();
    let colors: [Option<&str>; 4] = [0, 1, 2, 3].map(|i| s.border_side_color(i));
    if let Some(sh) = shadow {
        ctx.set_shadow(0.0, sh.dy, sh.blur, sh.color);
    }
    if let Some(fill) = bg.or(shadow.map(|sh| sh.color)) {
        ctx.begin_path();
        add_radii_path(ctx, r, rad, false);
        ctx.set_fill_color(fill);
        ctx.fill();
    }
    ctx.clear_shadow();
    if e.iter().all(|w| *w <= 0.0) || colors.iter().all(Option::is_none) || !s.border_drawn() {
        return;
    }
    let uniform_w = e.iter().all(|w| *w == e[0]);
    let uniform_c = colors.iter().all(|c| *c == colors[0]);
    let dash = s.border_dash(e[0]);
    if uniform_w && uniform_c {
        if let Some(d) = dash {
            ctx.set_line_dash(&d);
        }
        // One stroke along the inset outline (radii shrunk by half the width).
        let w = e[0];
        let inner = Rect::new(r.x + w / 2.0, r.y + w / 2.0, r.width - w, r.height - w);
        let ir = rad.map(|c| [(c[0] - w / 2.0).max(0.0), (c[1] - w / 2.0).max(0.0)]);
        ctx.set_stroke_color(colors[0].unwrap_or("#000000"));
        ctx.set_stroke_width(w);
        ctx.begin_path();
        add_radii_path(ctx, inner, ir, false);
        ctx.stroke();
        if dash.is_some() {
            ctx.set_line_dash(&[]);
        }
        return;
    }
    // The border ring: the outer outline minus the padding-box outline (CSS inner radii).
    let inner = Rect::new(r.x + e[3], r.y + e[0], (r.width - e[1] - e[3]).max(0.0), (r.height - e[0] - e[2]).max(0.0));
    let ir = inner_radii(rad, e);
    let ring = |ctx: &mut dyn RenderContext, color: &str| {
        ctx.begin_path();
        add_radii_path(ctx, r, rad, false);
        add_radii_path(ctx, inner, ir, true);
        ctx.set_fill_color(color);
        ctx.fill();
    };
    if uniform_c {
        ring(ctx, colors[0].unwrap_or("#000000"));
        return;
    }
    // Each side clipped to its trapezoid (outer corners to inner corners).
    let (x0, y0, x1, y1) = (r.x, r.y, r.x + r.width, r.y + r.height);
    let (ix0, iy0, ix1, iy1) = (inner.x, inner.y, inner.x + inner.width, inner.y + inner.height);
    let quads = [
        [(x0, y0), (x1, y0), (ix1, iy0), (ix0, iy0)],
        [(x1, y0), (x1, y1), (ix1, iy1), (ix1, iy0)],
        [(x1, y1), (x0, y1), (ix0, iy1), (ix1, iy1)],
        [(x0, y1), (x0, y0), (ix0, iy0), (ix0, iy1)],
    ];
    for i in 0..4 {
        let Some(c) = colors[i].filter(|_| e[i] > 0.0) else { continue };
        ctx.save();
        ctx.begin_path();
        ctx.move_to(quads[i][0].0, quads[i][0].1);
        for q in &quads[i][1..] {
            ctx.line_to(q.0, q.1);
        }
        ctx.close_path();
        ctx.clip();
        ring(ctx, c);
        ctx.restore();
    }
}

/// Run `b`'s animation (if it has one) for `frame`: the script paints in content-box
/// coordinates (origin at its top-left, `p.w` × `p.h`), clipped to it unless
/// [`AnimNode::overflow`], under the element's opacity; the pointer is given in the same
/// local coordinates. Skipped when the box is outside `frame.visible` or transparent.
/// Returns whether it ran; its needs are added to `report`.
pub fn paint_anim(ctx: &mut dyn RenderContext, b: &Placed, reg: &mut AnimRegistry, frame: &AnimFrame<'_>, report: &mut AnimReport) -> bool {
    let Some(a) = &b.anim else { return false };
    let c = content_box(b);
    let v = frame.visible;
    let outside = c.x >= v.x + v.width || c.y >= v.y + v.height || c.x + c.width <= v.x || c.y + c.height <= v.y;
    let alpha = b.style.opacity.unwrap_or(1.0);
    if outside || c.width <= 0.0 || c.height <= 0.0 || alpha <= 0.0 {
        return false;
    }
    let ptr = if a.track_pointer { frame.pointer } else { b.id.as_deref().and_then(frame.pointer_over) };
    let params = AnimParams { w: c.width, h: c.height, seed: a.seed, mouse: ptr.map(|(x, y)| (x - c.x, y - c.y)), args: a.args };
    ctx.save();
    if !a.overflow {
        ctx.begin_path();
        ctx.rect(c.x, c.y, c.width, c.height);
        ctx.clip();
    }
    ctx.translate(c.x, c.y);
    if alpha < 1.0 {
        ctx.set_global_alpha(alpha);
    }
    let ok = reg.paint(ctx, &a.name, frame.t, &params).is_ok();
    ctx.restore();
    if alpha < 1.0 {
        ctx.set_global_alpha(1.0);
    }
    report.painted += 1;
    report.tracks_pointer |= a.track_pointer && ptr.is_some();
    if let (Some(fps), false, true) = (a.fps.filter(|f| *f > 0.0), frame.paused, ok) {
        let d = 1.0 / fps;
        report.next = Some(report.next.map_or(d, |n: f64| n.min(d)));
    }
    true
}

/// Paint a laid-out tree with its animation nodes (each box, then its animation, in
/// document order, so later boxes cover earlier animations).
pub fn paint_with(ctx: &mut dyn RenderContext, dom: &Dom, reg: &mut AnimRegistry, frame: &AnimFrame<'_>) -> AnimReport {
    let mut report = AnimReport::default();
    for b in &dom.boxes {
        with_clip(ctx, b, |ctx| {
            paint_box(ctx, b);
            paint_anim(ctx, b, reg, frame, &mut report);
        });
    }
    report
}

/// Lay out and paint `root`.
pub fn render(ctx: &mut dyn RenderContext, sheet: &Sheet, root: &El, x: f64, y: f64, w: f64, state: &dyn Fn(&str) -> State) -> Dom {
    let dom = lay_out(ctx, sheet, root, x, y, w, state);
    paint(ctx, &dom);
    dom
}

#[cfg(all(test, feature = "flow-flex"))]
mod tests {
    use super::*;
    use crate::testing::NullRenderContext;

    const CSS: &str = r#"
        :root { --txt: #f5f5f5 }
        div.page { display: flex; gap: 10px; padding: 8px; color: var(--txt); font-size: 13px }
        div.nav { width: 100px }
        div.body { flex-grow: 1; display: flex; flex-direction: column; gap: 6px }
        div.row { display: flex; justify-content: space-between; align-items: center; padding: 4px 8px; background: #222; border-radius: 8px }
        select { padding: 5px 30px 5px 12px; background: #2a2a2a; border-radius: 8px }
        select:hover { background: #333 }
        div.gone { display: none }
    "#;

    fn page() -> El {
        El::new("div.page")
            .child(El::new("div.nav").id("nav"))
            .child(
                El::new("div.body")
                    .child(El::new("div.row").id("r1").child(El::new("span").text("Timezone")).child(El::new("select").id("sel:tz").text("Auto").tab()))
                    .child(El::new("div.row.gone").id("hidden"))
                    .child(El::new("div.row").id("r2").css("padding: 10px 8px").child(El::new("span").text("Accent"))),
            )
    }

    #[test]
    fn lays_out_by_css_and_reports_ids() {
        let mut ctx = NullRenderContext::default();
        let sh = Sheet::parse(CSS).unwrap();
        let d = lay_out(&mut ctx, &sh, &page(), 0.0, 0.0, 500.0, &|_| State::NONE);
        let nav = d.rect("nav").unwrap();
        assert_eq!((nav.x, nav.width), (8.0, 100.0));
        let r1 = d.rect("r1").unwrap();
        assert_eq!(r1.x, 118.0);
        assert_eq!(r1.width, 500.0 - 8.0 - 118.0);
        // The select sits at the row's right edge (space-between), inside its padding.
        let sel = d.rect("sel:tz").unwrap();
        assert!((sel.x + sel.width - (r1.x + r1.width - 8.0)).abs() < 0.01);
        assert!(d.rect("hidden").is_none());
        // Inline css wins: 10px vertical padding.
        let r2 = d.rect("r2").unwrap();
        assert_eq!(r2.y, r1.y + r1.height + 6.0);
        assert!(r2.height > r1.height - 0.01 || r2.height >= 19.5 + 20.0);
        assert_eq!(d.tabbable().collect::<Vec<_>>(), vec!["sel:tz"]);
        assert!(d.names().any(|(i, n)| i == "sel:tz" && n == "Auto"));
    }

    #[test]
    fn hover_state_restyles() {
        let mut ctx = NullRenderContext::default();
        let sh = Sheet::parse(CSS).unwrap();
        let d = render(&mut ctx, &sh, &page(), 0.0, 0.0, 500.0, &|id| if id == "sel:tz" { State::HOVER } else { State::NONE });
        let b = d.boxes.iter().find(|b| b.id.as_deref() == Some("sel:tz")).unwrap();
        assert_eq!(b.style.background.as_deref(), Some("#333"));
        assert_eq!(b.kind, "select");
    }

    #[test]
    fn white_space_normal_wraps_at_spaces_and_hyphens() {
        let u: Units = [("one", 30.0, false, false), ("two-", 40.0, true, false), ("three", 50.0, false, false), ("four", 40.0, true, false)]
            .iter()
            .map(|(t, w, sp, nl)| (t.to_string(), *w, *sp, *nl))
            .collect();
        // Unlimited: one line, spaces counted, no space at the hyphen break.
        assert_eq!(break_lines(&u, 5.0, None), vec![(0, 4, 30.0 + 5.0 + 40.0 + 50.0 + 5.0 + 40.0)]);
        // 100 px: "one two-" | "three four"; 80 px: "four" goes to a third line.
        assert_eq!(break_lines(&u, 5.0, Some(100.0)), vec![(0, 2, 75.0), (2, 4, 95.0)]);
        assert_eq!(break_lines(&u, 5.0, Some(80.0)).len(), 3);
        assert_eq!(line_text(&u, 0, 2), "one two-");
        assert_eq!(line_text(&u, 2, 4), "three four");
        // Laid out in a narrow box the text grows by whole lines; nowrap stays one line.
        let mut ctx = crate::testing::RecordingRenderContext::with_char_width(7.0);
        let sh = Sheet::parse("div.p { width: 60px; font-size: 10px; line-height: 15px } div.w { white-space: normal }").unwrap();
        let text = "aaaa bbbb cccc dddd eeee";
        let one = lay_out(&mut ctx, &sh, &El::new("div.p").child(El::new("div").id("t").text(text)), 0.0, 0.0, 500.0, &|_| State::NONE);
        let many = lay_out(&mut ctx, &sh, &El::new("div.p.w").child(El::new("div").id("t").text(text)), 0.0, 0.0, 500.0, &|_| State::NONE);
        assert_eq!(one.rect("t").unwrap().height, 15.0);
        let h = many.rect("t").unwrap().height;
        assert!(h > 15.0 && (h / 15.0).fract() == 0.0, "{h}");
        // line-clamp: at most two lines, the last ending in an ellipsis.
        let sh2 = Sheet::parse("div.p { width: 60px; font-size: 10px; line-height: 15px; white-space: normal } div.c { -webkit-line-clamp: 2 }").unwrap();
        let clamped = render(&mut ctx, &sh2, &El::new("div.p").child(El::new("div.c").id("t").text(text)), 0.0, 0.0, 500.0, &|_| State::NONE);
        assert_eq!(clamped.rect("t").unwrap().height, 30.0);
        assert_eq!(clamped.boxes.last().unwrap().style.line_clamp, Some(2));
    }

    #[test]
    fn anim_nodes_lay_out_paint_and_pause() {
        use crate::render::{AnimRegistry, FnAnim};
        use std::cell::RefCell;
        use std::rc::Rc;
        let mut ctx = NullRenderContext::default();
        let sh = Sheet::parse("div.row { display: flex; gap: 4px; padding: 2px } div.av { width: 40px; height: 40px; padding: 2px } div.grow { flex-grow: 1 }").unwrap();
        let root = El::new("div.row")
            .child(El::new("div.av").id("a").anim(AnimNode::new("spy").seed(7).args([1.0, 2.0, 3.0, 4.0])))
            .child(El::new("div.grow").id("drawer"))
            .child(El::new("div.av").id("b").anim(AnimNode::new("spy").fps(None).track_pointer(true)));
        let d = lay_out(&mut ctx, &sh, &root, 10.0, 20.0, 300.0, &|_| State::NONE);
        let (a, b) = (d.rect("a").unwrap(), d.rect("b").unwrap());
        assert_eq!((a.x, a.y, a.width), (12.0, 22.0, 40.0));
        assert_eq!(b.x + b.width, 10.0 + 300.0 - 2.0, "flex places the anim box like any other");
        let seen: Rc<RefCell<Vec<AnimParams>>> = Rc::default();
        let s2 = seen.clone();
        let mut reg = AnimRegistry::new();
        reg.fallback("spy", Box::new(FnAnim(move |_: &mut dyn RenderContext, _t: f64, p: &AnimParams| s2.borrow_mut().push(*p))));
        // The pointer is over `a` (topmost there), 5px into its content box.
        let over = |id: &str| (id == "a").then_some((a.x + 2.0 + 5.0, a.y + 2.0 + 6.0));
        let f = AnimFrame { t: 1.0, visible: Rect::new(0.0, 0.0, 1000.0, 1000.0), pointer: Some((0.0, 0.0)), pointer_over: &over, paused: false };
        let r = paint_with(&mut ctx, &d, &mut reg, &f);
        assert_eq!(r.painted, 2);
        assert_eq!(r.next, Some(1.0 / 30.0), "only `a` moves");
        assert!(r.tracks_pointer);
        let v = seen.borrow().clone();
        assert_eq!((v[0].w, v[0].h, v[0].seed, v[0].args), (36.0, 36.0, 7, [1.0, 2.0, 3.0, 4.0, 0.0, 0.0, 0.0, 0.0]));
        assert_eq!(v[0].mouse, Some((5.0, 6.0)), "local coordinates");
        assert_eq!(v[1].mouse, Some((-(b.x + 2.0), -(b.y + 2.0))), "a tracking node sees the window pointer anywhere");
        // Not topmost: no pointer for the hover-only node.
        seen.borrow_mut().clear();
        let f2 = AnimFrame { pointer_over: &|_| None, ..AnimFrame::new(2.0, Rect::new(0.0, 0.0, 1000.0, 1000.0)) };
        paint_with(&mut ctx, &d, &mut reg, &f2);
        assert_eq!(seen.borrow()[0].mouse, None);
        // Scrolled out of view: nothing runs, no frame asked; paused: runs, asks nothing.
        seen.borrow_mut().clear();
        let r = paint_with(&mut ctx, &d, &mut reg, &AnimFrame::new(3.0, Rect::new(0.0, 100.0, 1000.0, 100.0)));
        assert_eq!((r.painted, r.next, seen.borrow().len()), (0, None, 0));
        let r = paint_with(&mut ctx, &d, &mut reg, &AnimFrame { paused: true, ..AnimFrame::new(3.0, Rect::new(0.0, 0.0, 1000.0, 1000.0)) });
        assert_eq!((r.painted, r.next), (2, None));
    }

    #[test]
    fn anim_frame_is_clipped_to_its_content_box() {
        use crate::render::{AnimRegistry, FnAnim};
        let mut ctx = crate::testing::RecordingRenderContext::new();
        let sh = Sheet::parse("div.av { width: 20px; height: 10px }").unwrap();
        let mut reg = AnimRegistry::new();
        reg.fallback("dot", Box::new(FnAnim(|c: &mut dyn RenderContext, _t: f64, _p: &AnimParams| c.fill_rect(0.0, 0.0, 1.0, 1.0))));
        for overflow in [false, true] {
            let d = lay_out(&mut ctx, &sh, &El::new("div.av").anim(AnimNode::new("dot").overflow_visible(overflow)), 5.0, 6.0, 100.0, &|_| State::NONE);
            ctx.ops.clear();
            paint_with(&mut ctx, &d, &mut reg, &AnimFrame::new(0.0, Rect::new(0.0, 0.0, 100.0, 100.0)));
            let ops = format!("{:?}", ctx.ops);
            assert_eq!(ops.contains("Clip"), !overflow, "{ops}");
            assert!(ops.contains("Translate"), "{ops}");
        }
    }

    #[test]
    fn runs_wrap_across_styles_and_hit_per_piece() {
        // 7px per char in every font: easy arithmetic.
        let mut ctx = crate::testing::RecordingRenderContext::with_char_width(7.0);
        let sh = Sheet::parse(
            "div.m { width: 100px; font-size: 10px; line-height: 20px; white-space: pre-wrap; overflow-wrap: anywhere; color: #eee }
             code { font-family: monospace; background: #222; padding: 1px 6px; border-radius: 6px }
             a.link { color: #60a5fa } b { font-weight: 700 } i { font-style: italic }",
        )
        .unwrap();
        let el = El::new("div.m").id("m").runs([
            Run::text("see "),
            Run::new("code", "ab"),
            Run::text(" "),
            Run::new("a.link", "https://x.y/zzzz").id("url:1"),
            Run::new("b", " ok"),
            Run::new("i", "\nit"),
        ]);
        let d = render(&mut ctx, &sh, &el, 0.0, 0.0, 500.0, &|_| State::NONE);
        let b = d.boxes.iter().find(|b| b.id.as_deref() == Some("m")).unwrap();
        let inl = b.inline.as_ref().unwrap();
        assert_eq!(inl.text, "see ab https://x.y/zzzz ok\nit");
        // "see " 28 + chip 6+14+6 = 54, " " 61; the 16-char link (112px) breaks anywhere:
        // 5 chars fill line one (61 + 35 = 96), the rest wraps.
        let l0 = &inl.lines[0];
        let ab = l0.pieces.iter().find(|p| p.text == "ab").unwrap();
        assert_eq!((ab.x, ab.w, ab.pad, ab.start), (28.0, 26.0, 6.0, 4));
        assert_eq!(l0.pieces.last().unwrap().text, "https");
        assert_eq!(inl.lines[1].pieces[0].text, "://x.y/zzzz");
        // The forced newline starts the italic run on its own line.
        let last = inl.lines.last().unwrap();
        assert_eq!(last.pieces[0].text, "it");
        assert_eq!(b.rect.height, 20.0 * inl.lines.len() as f64);
        // The link is a hit on each of its pieces; the plain runs are not.
        let hits: Vec<_> = d.hits().filter(|(id, _)| *id == "url:1").collect();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].1, Rect::new(61.0, 0.0, 35.0, 20.0));
        assert_eq!(d.run_at(65.0, 5.0).map(|(_, r)| r), Some(3));
        assert_eq!(d.run_at(5.0, 5.0).map(|(_, r)| r), Some(0));
        // Char boundaries for carets: "see " is 4 chars, the chip text starts after its padding.
        assert_eq!(b.char_at(28.0 + 6.0 + 7.5, 5.0), Some(5));
        assert_eq!(b.char_at(0.0, 25.0), Some(inl.lines[1].pieces[0].start));
        // Painted: the chip, the italic and bold fonts, the link colour.
        let ops = format!("{:?}", ctx.ops);
        assert!(ops.contains("#222") && ops.contains("italic 400 10px") && ops.contains("700 10px") && ops.contains("#60a5fa"), "{ops}");
        assert!(d.names().any(|(id, n)| id == "m" && n.starts_with("see ab")));
    }

    #[test]
    fn a_multi_word_chip_is_one_box_per_line() {
        let mut ctx = crate::testing::RecordingRenderContext::with_char_width(7.0);
        let sh = Sheet::parse(
            "div.m { width: 100px; font-size: 10px; line-height: 20px; white-space: pre-wrap }
             code { background: #222; padding: 1px 6px; border-radius: 6px }",
        )
        .unwrap();
        // "x " then the chip "aa bb cc dd" (77px + 12px padding) does not fit after it: the
        // chip's words wrap, and each line gets one chip spanning its words.
        let el = El::new("div.m").id("m").runs([Run::text("x "), Run::new("code", "aa bb cc dd ee")]);
        let d = lay_out(&mut ctx, &sh, &el, 0.0, 0.0, 500.0, &|_| State::NONE);
        ctx.ops.clear();
        paint(&mut ctx, &d);
        let fills = ctx.ops.iter().filter(|o| format!("{o:?}").contains("#222")).count();
        let b = d.boxes.iter().find(|b| b.id.as_deref() == Some("m")).unwrap();
        let lines = &b.inline.as_ref().unwrap().lines;
        assert_eq!(lines.len(), 2);
        assert_eq!(fills, 2, "one chip per line, not one per word");
        // Line one's chip spans "aa bb cc " from after "x " (14px).
        let chip_line = &lines[0].pieces;
        let first = chip_line.iter().find(|p| p.run == 1).unwrap();
        assert_eq!((first.x, first.pad), (14.0, 6.0));
    }

    #[test]
    fn runs_size_their_box_like_text() {
        let mut ctx = crate::testing::RecordingRenderContext::with_char_width(7.0);
        let sh = Sheet::parse("div.row { display: flex } div.m { font-size: 10px; line-height: 20px; white-space: normal; max-width: 70px }").unwrap();
        let el = El::new("div.row").child(El::new("div.m").id("m").runs([Run::text("aaaa "), Run::new("b", "bbbb cc")]));
        let d = lay_out(&mut ctx, &sh, &el, 0.0, 0.0, 500.0, &|_| State::NONE);
        let r = d.rect("m").unwrap();
        // max-content 84 > 70: "aaaa bbbb" (63) | "cc".
        assert_eq!((r.width, r.height), (70.0, 40.0));
        // Wrapped spaces are dropped from the line's width.
        let b = d.boxes.iter().find(|b| b.id.as_deref() == Some("m")).unwrap();
        assert_eq!(b.inline.as_ref().unwrap().lines[0].width, 63.0);
        // Not wrapping (white-space unset): one line at max-content.
        let sh2 = Sheet::parse("div.row { display: flex } div.m { font-size: 10px; line-height: 20px }").unwrap();
        let d2 = lay_out(&mut ctx, &sh2, &el, 0.0, 0.0, 500.0, &|_| State::NONE);
        assert_eq!(d2.rect("m").unwrap(), Rect::new(0.0, 0.0, 84.0, 20.0));
    }

    #[test]
    fn absolute_boxes_leave_the_flow_and_paint_on_top() {
        let mut ctx = NullRenderContext::default();
        let sh = Sheet::parse(
            "div.card { position: relative; width: 200px; padding: 10px; border: 2px solid #333 }
             div.line { height: 30px }
             div.badge { position: absolute; top: -4px; right: -4px; width: 16px; height: 16px }
             div.under { position: absolute; inset: 0; z-index: -1 }
             div.cover { position: absolute; left: 10px; bottom: 0; width: 50px; height: 5px; z-index: 2 }
             div.wrap { padding: 4px }",
        )
        .unwrap();
        let el = El::new("div.card")
            .id("card")
            .child(El::new("div.cover").id("cover"))
            .child(El::new("div.line").id("l1"))
            .child(El::new("div.badge").id("badge"))
            .child(El::new("div.wrap").id("wrap").child(El::new("div.badge").id("deep")))
            .child(El::new("div.under").id("under"));
        let d = lay_out(&mut ctx, &sh, &el, 0.0, 0.0, 500.0, &|_| State::NONE);
        // The flow ignores the absolute boxes: l1 at the content top, the card as tall as l1 + wrap.
        assert_eq!(d.rect("l1").unwrap().y, 12.0);
        let card = d.rect("card").unwrap();
        assert_eq!(card.height, 2.0 + 10.0 + 30.0 + 8.0 + 10.0 + 2.0);
        // Against the padding box (inside the 2px border).
        assert_eq!(d.rect("badge").unwrap(), Rect::new(200.0 - 2.0 + 4.0 - 16.0, 2.0 - 4.0, 16.0, 16.0));
        // A static wrapper is no containing block: `deep` uses the card too.
        assert_eq!(d.rect("deep").unwrap(), d.rect("badge").unwrap());
        assert_eq!(d.rect("under").unwrap(), Rect::new(2.0, 2.0, 196.0, card.height - 4.0));
        assert_eq!(d.rect("cover").unwrap(), Rect::new(12.0, card.height - 2.0 - 5.0, 50.0, 5.0));
        // Paint (and hit) order: root, z<0, flow, then z>=0 layers by z-index.
        let order: Vec<&str> = d.boxes.iter().filter_map(|b| b.id.as_deref()).collect();
        assert_eq!(order, ["card", "under", "l1", "wrap", "badge", "deep", "cover"]);
        // A relative box shifts with its subtree, its flow slot unchanged.
        let sh2 = Sheet::parse("div.r { position: relative; top: 5px; left: -3px; height: 10px } div.n { height: 10px }").unwrap();
        let d2 = lay_out(&mut ctx, &sh2, &El::new("div").child(El::new("div.r").id("r")).child(El::new("div.n").id("n")), 0.0, 0.0, 100.0, &|_| State::NONE);
        assert_eq!(d2.rect("r").unwrap(), Rect::new(-3.0, 5.0, 100.0, 10.0));
        assert_eq!(d2.rect("n").unwrap().y, 10.0);
        // Shrink-to-fit width of an absolute box with text.
        let mut rc = crate::testing::RecordingRenderContext::with_char_width(7.0);
        let sh3 = Sheet::parse("div.p { position: relative; width: 300px; height: 50px } span.t { position: absolute; right: 0; bottom: 0; font-size: 10px; line-height: 20px }").unwrap();
        let d3 = lay_out(&mut rc, &sh3, &El::new("div.p").child(El::new("span.t").id("t").text("abc")), 0.0, 0.0, 500.0, &|_| State::NONE);
        assert_eq!(d3.rect("t").unwrap(), Rect::new(279.0, 30.0, 21.0, 20.0));
    }

    #[test]
    fn per_corner_radii_and_per_side_borders() {
        let mut ctx = crate::testing::RecordingRenderContext::new();
        let sh = Sheet::parse(
            "div.top { width: 100px; height: 40px; background: #111; border-radius: 12px 12px 0 0 }
             div.ell { width: 100px; height: 40px; background: #111; border-radius: 20px / 10px }
             div.side { width: 100px; padding: 4px; border-width: 0 0 2px 3px; border-color: red; height: 20px }
             div.mix { width: 100px; height: 20px; border: 1px solid #333; border-left: 4px solid #3b82f6 }",
        )
        .unwrap();
        let s = sh.el("div.top");
        assert_eq!(s.radii(), [[12.0, 12.0], [12.0, 12.0], [0.0, 0.0], [0.0, 0.0]]);
        assert_eq!(sh.el("div.ell").radii()[2], [20.0, 10.0]);
        // Per-side widths move the content box.
        let d = render(&mut ctx, &sh, &El::new("div.side").id("s").child(El::new("div").id("in").css("height: 5px")), 0.0, 0.0, 500.0, &|_| State::NONE);
        assert_eq!(d.rect("in").unwrap(), Rect::new(3.0 + 4.0, 4.0, 100.0 - 3.0 - 8.0, 5.0));
        assert_eq!(d.rect("s").unwrap().height, 20.0);
        let mix = sh.el("div.mix");
        assert_eq!(mix.border_edges(), [1.0, 1.0, 1.0, 4.0]);
        assert_eq!(mix.border_side_color(3), Some("#3b82f6"));
        assert_eq!(mix.border_side_color(0), Some("#333"));
        // Painting a square-bottomed box: the bottom corners are plain line joins (zero
        // radius), the top ones curves; mixed side colours clip each side.
        ctx.ops.clear();
        render(&mut ctx, &sh, &El::new("div.top"), 0.0, 0.0, 500.0, &|_| State::NONE);
        let ops = format!("{:?}", ctx.ops);
        assert!(ops.contains("BezierCurveTo"), "{ops}");
        ctx.ops.clear();
        render(&mut ctx, &sh, &El::new("div.mix"), 0.0, 0.0, 500.0, &|_| State::NONE);
        let ops = format!("{:?}", ctx.ops);
        assert_eq!(ops.matches("Clip").count(), 4, "{ops}");
        assert!(ops.contains("#3b82f6") && ops.contains("#333"));
        // The classic single radius keeps its exact old path.
        let classic = Sheet::parse("div { width: 10px; height: 10px; background: #000; border-radius: 3px }").unwrap();
        assert_eq!(classic.el("div").border_radii, None);
    }

    #[test]
    fn geometric_precision_icons_are_not_snapped() {
        let icon = r#"<svg viewBox="0 0 24 24" stroke-width="1.5"><path d="M4 12H20"/></svg>"#;
        let mut ctx = crate::testing::RecordingRenderContext::new();
        let sh = Sheet::parse("i.ic { width: 24px; height: 24px } i.x { shape-rendering: geometricPrecision }").unwrap();
        for (spec, want) in [("i.ic", 2.0), ("i.ic.x", 1.5)] {
            ctx.ops.clear();
            render(&mut ctx, &sh, &El::new(spec).svg(icon), 0.3, 0.0, 100.0, &|_| State::NONE);
            let w = ctx.ops.iter().find_map(|o| if let crate::testing::DrawOp::StrokeWidth(w) = o { Some(*w) } else { None });
            assert_eq!(w, Some(want), "{spec}");
        }
    }

    #[test]
    fn padding_box_clip_uses_the_inner_radii() {
        let mut ctx = crate::testing::RecordingRenderContext::new();
        let sh = Sheet::parse("div { width: 100px; height: 50px; border-width: 0 0 1px 1px; border-color: #222; border-radius: 12px 0 0 12px }").unwrap();
        let d = lay_out(&mut ctx, &sh, &El::new("div"), 0.0, 0.0, 500.0, &|_| State::NONE);
        clip_padding_box(&mut ctx, &d.boxes[0]);
        let ops = format!("{:?}", ctx.ops);
        // Starts after the top-left corner: 1px border, 11px inner horizontal radius.
        assert!(ops.contains("MoveTo { x: 12.0, y: 0.0 }") && ops.contains("Clip"), "{ops}");
        ctx.ops.clear();
        path_border_box(&mut ctx, &d.boxes[0]);
        assert!(format!("{:?}", ctx.ops).contains("MoveTo { x: 12.0, y: 0.0 }"));
    }

    #[test]
    fn overflow_hidden_clips_paint_and_hits_and_fixed_escapes() {
        let sh = Sheet::parse("div.v { width: 100px; height: 50px; overflow: hidden; border: 2px solid #333 } div.r { height: 40px } div.f { position: fixed; top: 0; left: 0; width: 10px; height: 10px } div.s { height: 20px }").unwrap();
        let mut ctx = NullRenderContext::default();
        let root = El::new("div.v")
            .child(El::new("div.r").id("a"))
            .child(El::new("div.r").id("b"))
            .child(El::new("div.f").id("fx"))
            .child(El::new("div.s").slot("@k"));
        let d = lay_out_in(&mut ctx, &sh, &root, 0.0, 0.0, 300.0, Some(Rect::new(0.0, 0.0, 300.0, 300.0)), &|_| State::NONE);
        let b = d.get("b").unwrap();
        assert_eq!(b.clip, Some(Rect::new(2.0, 2.0, 96.0, 46.0)));
        assert_eq!(d.hit(10.0, 45.0), Some("b"));
        assert_eq!(d.hit(10.0, 60.0), None, "clipped away");
        assert_eq!(d.get("fx").unwrap().clip, None, "fixed escapes the clip");
        // The slot callback runs in paint order, inside the clip.
        let mut seen = Vec::new();
        paint_slots(&mut ctx, &d, &mut |_, k, r| seen.push((k.to_string(), r)));
        assert_eq!(seen.len(), 1);
        let d = d.moved(0.0, -30.0);
        assert_eq!(d.get("b").unwrap().clip, Some(Rect::new(2.0, -28.0, 96.0, 46.0)));
    }
}
