//! A small DOM: a tree of elements styled by a [`Sheet`](crate::ui::style::Sheet), laid out
//! with [`flow`](crate::ui::flow) (`display:flex` needs the `flow-flex` feature, on by default)
//! and painted as CSS boxes (background, border, radius,
//! `box-shadow`, one line of text with `text-overflow:ellipsis`). Screens describe *what*
//! is on them (`El::new("div.card").child(...)`) and the stylesheet decides *how it looks
//! and where it goes*: no per-screen pixel math. [`render`] returns every element's border
//! box by `id`, so the app hit-tests, registers and focuses by id as in a browser.
//!
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
    /// Single-line text width / line height.
    measured: Option<(f64, f64)>,
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
        (ctx.measure_text(t).ceil(), font_of(&s).2)
    });
    let children = el.children.iter().filter_map(|c| resolve(ctx, sheet, c, Some(&s), state)).collect();
    let kind = el.spec.split(['.', '#', ':']).next().unwrap_or("div").to_string();
    Some(Resolved { kind, id: el.id.clone(), style: s, text: el.text.clone(), tab: el.tab, name: el.name.clone(), measured, children })
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
    if let Some((w, lh)) = r.measured {
        // One line: min-content is the whole line (nowrap); a narrower box ellipsizes.
        out.push(Box::new(move |lim: Option<f64>| (lim.filter(|l| *l > 0.0).map_or(w, |l| w.min(l)), lh)));
    }
    for c in &r.children {
        collect_measures(c, out);
    }
}

fn place(r: Resolved, l: &Laid, out: &mut Vec<Placed>) {
    out.push(Placed { id: r.id, kind: r.kind, rect: l.rect, style: r.style, text: r.text, tab: r.tab, name: r.name });
    for (c, cl) in r.children.into_iter().zip(&l.children) {
        place(c, cl, out);
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
    place(r, &laid, &mut boxes);
    Dom { boxes, root: laid.rect }
}

/// Paint one placed box: shadow, background, border, text, kind content.
pub fn paint_box(ctx: &mut dyn RenderContext, b: &Placed) {
    let s = &b.style;
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
        let shown = ellipsize(ctx, t, content.width + 0.5);
        let (align, x) = match s.text_align.as_deref() {
            Some("center") => (TextAlign::Center, content.x + content.width / 2.0),
            Some("right") | Some("end") => (TextAlign::Right, content.x + content.width),
            _ => (TextAlign::Left, content.x),
        };
        // A box taller than its line centres it (flex `align-items:center`, the usual control).
        let top = content.y + ((content.height - lh) / 2.0).max(0.0);
        ctx.set_fill_color(color);
        fill_text_css(ctx, &shown, x, top, px, lh, INTER, align);
    }
    if b.kind == "select" {
        let c = s.custom("--chevron").unwrap_or(color);
        let (ax, ay) = (r.x + r.width - bw - p[1].min(20.0).max(12.0) / 2.0 - 8.0, r.y + r.height / 2.0 - 2.0);
        chevron(ctx, ax.max(content.x + content.width + 2.0), ay, c);
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

/// Lay out and paint `root` (document order: parents under children).
pub fn render(ctx: &mut dyn RenderContext, sheet: &Sheet, root: &El, x: f64, y: f64, w: f64, state: &dyn Fn(&str) -> State) -> Dom {
    let dom = lay_out(ctx, sheet, root, x, y, w, state);
    for b in &dom.boxes {
        paint_box(ctx, b);
    }
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
}
