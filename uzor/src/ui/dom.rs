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
use crate::ui::flow::{layout, LBox, Laid};
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
    pub fn children(mut self, cs: impl IntoIterator<Item = El>) -> Self {
        self.children.extend(cs);
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
    /// `(id, rect)` of every element with an id, in document order (later = on top).
    pub fn hits(&self) -> impl Iterator<Item = (&str, Rect)> {
        self.boxes.iter().filter_map(|b| b.id.as_deref().map(|i| (i, b.rect)))
    }
    /// The Tab stops in document order.
    pub fn tabbable(&self) -> impl Iterator<Item = &str> {
        self.boxes.iter().filter(|b| b.tab).filter_map(|b| b.id.as_deref())
    }
    /// The tree shifted by `(dx, dy)`: lay out once to learn a size, then place it.
    pub fn moved(mut self, dx: f64, dy: f64) -> Self {
        for b in &mut self.boxes {
            b.rect = Rect::new(b.rect.x + dx, b.rect.y + dy, b.rect.width, b.rect.height);
        }
        self.root = Rect::new(self.root.x + dx, self.root.y + dy, self.root.width, self.root.height);
        self
    }
    /// `(id, accessible name)` for elements with an id and a name or text.
    pub fn names(&self) -> impl Iterator<Item = (&str, &str)> {
        self.boxes.iter().filter_map(|b| Some((b.id.as_deref()?, b.name.as_deref().or(b.text.as_deref())?)))
    }
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
    children: Vec<Resolved>,
}

fn font_of(s: &Style) -> (u32, f64, f64, String) {
    let px = s.font_size.unwrap_or(13.0);
    let lh = s.line_height.unwrap_or((px * 1.5 * 100.0).round() / 100.0);
    (s.font_weight.unwrap_or(400), px, lh, s.font_family.clone().unwrap_or_else(|| "Inter, sans-serif".into()))
}

fn font_str(s: &Style) -> String {
    let (w, px, _, fam) = font_of(s);
    format!("{w} {px}px {fam}")
}

fn resolve(ctx: &mut dyn RenderContext, sheet: &Sheet, el: &El, parent: Option<&Style>, state: &dyn Fn(&str) -> State) -> Option<Resolved> {
    let st = el.id.as_deref().map_or(State::NONE, state);
    let mut s = sheet.resolve(&Node::parse(&el.spec).state(st), parent);
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
    let measured = el.text.as_ref().map(|t| {
        ctx.set_font(&font_str(&s));
        (units(ctx, t, wraps(&s)), font_of(&s).2)
    });
    let space = if el.text.is_some() && wraps(&s) { ctx.measure_text(" ") } else { 0.0 };
    let children = el.children.iter().filter_map(|c| resolve(ctx, sheet, c, Some(&s), state)).collect();
    let kind = el.spec.split(['.', '#', ':']).next().unwrap_or("div").to_string();
    Some(Resolved { kind, id: el.id.clone(), style: s, text: el.text.clone(), tab: el.tab, name: el.name.clone(), svg: el.svg.clone(), anim: el.anim.clone(), measured, space, children })
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
    if r.measured.is_some() {
        b = b.text(measures[*next].as_ref());
        *next += 1;
    }
    for c in &r.children {
        b = b.child(lbox(c, measures, next));
    }
    b
}

fn collect_measures(r: &Resolved, out: &mut Vec<Box<dyn Fn(Option<f64>) -> (f64, f64)>>) {
    if let Some((u, lh)) = &r.measured {
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
fn place(r: Resolved, l: &Laid, out: &mut Vec<Placed>, alpha: f64) {
    let mut style = r.style;
    let a = alpha * style.opacity.unwrap_or(1.0);
    style.opacity = (a < 1.0).then_some(a);
    out.push(Placed { id: r.id, kind: r.kind, rect: l.rect, style, text: r.text, tab: r.tab, name: r.name, svg: r.svg, anim: r.anim });
    for (c, cl) in r.children.into_iter().zip(&l.children) {
        place(c, cl, out, a);
    }
}

/// Lay out `root` at `(x, y)` in a containing block `w` wide, without painting.
/// `state(id)` gives an element's interaction state (hover, focus-visible, ...).
pub fn lay_out(ctx: &mut dyn RenderContext, sheet: &Sheet, root: &El, x: f64, y: f64, w: f64, state: &dyn Fn(&str) -> State) -> Dom {
    let Some(r) = resolve(ctx, sheet, root, None, state) else { return Dom::default() };
    let mut measures = Vec::new();
    collect_measures(&r, &mut measures);
    let laid = {
        let mut next = 0;
        let b = lbox(&r, &measures, &mut next);
        layout(&b, x, y, w)
    };
    let mut boxes = Vec::new();
    place(r, &laid, &mut boxes, 1.0);
    Dom { boxes, root: laid.rect }
}

/// Paint one placed box: shadow, background, border, text, kind content.
pub fn paint_box(ctx: &mut dyn RenderContext, b: &Placed) {
    let s = &b.style;
    if let Some(a) = s.opacity {
        ctx.set_global_alpha(a);
    }
    let r = b.rect;
    let rad = s.border_radius.unwrap_or(0.0);
    let shadow: Option<Shadow> = s.box_shadow.as_deref().and_then(<Option<Shadow>>::from_css).flatten();
    let bw = s.border_width.unwrap_or(0.0);
    let border = s.border_color.as_deref().filter(|_| bw > 0.0);
    if let Some(sh) = shadow {
        ctx.set_shadow(0.0, sh.dy, sh.blur, sh.color);
    }
    if let Some(bg) = s.background.as_deref().filter(|b| *b != "transparent" && *b != "none") {
        fill_round_rect(ctx, r, rad, bg);
    } else if let Some(sh) = shadow {
        // A shadow needs something to cast it.
        fill_round_rect(ctx, r, rad, sh.color);
    }
    ctx.clear_shadow();
    if let Some(bc) = border {
        ctx.set_stroke_color(bc);
        ctx.set_stroke_width(bw);
        path_round_rect(ctx, Rect::new(r.x + bw / 2.0, r.y + bw / 2.0, r.width - bw, r.height - bw), (rad - bw / 2.0).max(0.0));
        ctx.stroke();
    }
    let p = s.padding.unwrap_or([0.0; 4]);
    let content = Rect::new(r.x + p[3] + bw, r.y + p[0] + bw, (r.width - p[1] - p[3] - 2.0 * bw).max(0.0), (r.height - p[0] - p[2] - 2.0 * bw).max(0.0));
    let color = s.color.as_deref().unwrap_or("#000000");
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
        crate::render::draw_svg_icon(ctx, svg, content.x, content.y, content.width, content.height, color);
    }
    if b.kind == "select" {
        let c = s.custom("--chevron").unwrap_or(color);
        let (ax, ay) = (r.x + r.width - bw - p[1].min(20.0).max(12.0) / 2.0 - 8.0, r.y + r.height / 2.0 - 2.0);
        chevron(ctx, ax.max(content.x + content.width + 2.0), ay, c);
    }
    if s.opacity.is_some() {
        ctx.set_global_alpha(1.0);
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
    for b in &dom.boxes {
        paint_box(ctx, b);
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
    let s = &b.style;
    let (r, bw, p) = (b.rect, s.border_width.unwrap_or(0.0), s.padding.unwrap_or([0.0; 4]));
    Rect::new(r.x + p[3] + bw, r.y + p[0] + bw, (r.width - p[1] - p[3] - 2.0 * bw).max(0.0), (r.height - p[0] - p[2] - 2.0 * bw).max(0.0))
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
        paint_box(ctx, b);
        paint_anim(ctx, b, reg, frame, &mut report);
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
}
