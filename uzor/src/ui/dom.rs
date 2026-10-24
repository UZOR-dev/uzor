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

use crate::render::{RenderContext, TextAlign};
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
    pub children: Vec<El>,
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
    Some(Resolved { kind, id: el.id.clone(), style: s, text: el.text.clone(), tab: el.tab, name: el.name.clone(), svg: el.svg.clone(), measured, space, children })
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
            let (u, lh, space) = (u.clone(), *lh, r.space);
            out.push(Box::new(move |lim: Option<f64>| {
                let ls = break_lines(&u, space, lim);
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
    out.push(Placed { id: r.id, kind: r.kind, rect: l.rect, style, text: r.text, tab: r.tab, name: r.name, svg: r.svg });
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
            break_lines(&u, space, Some(content.width)).into_iter().map(|(a, z, _)| line_text(&u, a, z)).collect()
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
    }
}
