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
    pub gap: Option<f64>,
    pub box_shadow: Option<String>,
    pub cursor: Option<String>,
    pub text_align: Option<String>,
    /// Custom properties (`--name: value`) set on the element or inherited, as in CSS.
    pub custom: std::collections::BTreeMap<String, String>,
}

impl Style {
    /// A custom property of the element (`--name`), e.g. a widget-specific knob.
    pub fn custom(&self, name: &str) -> Option<&str> {
        self.custom.get(name).map(String::as_str)
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

#[derive(Clone, Debug, PartialEq)]
struct Selector {
    kind: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    state: State,
}

impl Selector {
    fn parse(s: &str) -> Result<Selector, StyleError> {
        let s = s.trim();
        if s.is_empty() || s.contains(char::is_whitespace) || s.contains('>') {
            return Err(StyleError(format!("unsupported selector `{s}` (compound selectors only)")));
        }
        let mut sel = Selector { kind: None, id: None, classes: vec![], state: State::NONE };
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

/// A parsed style sheet.
#[derive(Debug, Default)]
pub struct Sheet {
    rules: Vec<Rule>,
    vars: HashMap<String, String>,
    cache: std::sync::Mutex<HashMap<String, Style>>,
}

impl Clone for Sheet {
    fn clone(&self) -> Self {
        Sheet { rules: self.rules.clone(), vars: self.vars.clone(), cache: Default::default() }
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
        let mut trace = Vec::new();
        let mut s = Style::default();
        if let Some(p) = parent {
            s.color = p.color.clone();
            s.font_size = p.font_size;
            s.font_weight = p.font_weight;
            s.font_family = p.font_family.clone();
            s.line_height = p.line_height;
            s.text_align = p.text_align.clone();
            s.custom = p.custom.clone();
        }
        let mut hits: Vec<&Rule> = self.rules.iter().filter(|r| r.sel.matches(n)).collect();
        hits.sort_by_key(|r| (r.sel.specificity(), r.order));
        // Custom properties first (they cascade, then every var() sees the element's values).
        for r in &hits {
            for (k, v) in &r.decls {
                if k.starts_with("--") {
                    s.custom.insert(k.clone(), v.clone());
                }
            }
        }
        let local = std::mem::take(&mut s.custom);
        s.custom = local.iter().map(|(k, v)| (k.clone(), self.subst_in(v, 0, Some(&local)))).collect();
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

fn apply(s: &mut Style, k: &str, v: &str) {
    match k {
        "color" => s.color = Some(v.into()),
        "background" | "background-color" => s.background = Some(v.into()),
        "border-color" => s.border_color = Some(v.into()),
        "border-width" => s.border_width = px(v),
        "border" => {
            // `1px solid #333`
            for part in v.split_whitespace() {
                if let Some(w) = px(part).filter(|_| part.ends_with("px") || part == "0") {
                    s.border_width = Some(w);
                } else if !matches!(part, "solid" | "dashed" | "none") {
                    s.border_color = Some(part.into());
                }
            }
        }
        "padding" => s.padding = edges(v),
        "margin" => s.margin = edges(v),
        "border-radius" => s.border_radius = px(v),
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
        "width" => s.width = px(v),
        "height" => s.height = px(v),
        "gap" => s.gap = px(v),
        "box-shadow" => s.box_shadow = Some(v.into()),
        "cursor" => s.cursor = Some(v.into()),
        "text-align" => s.text_align = Some(v.into()),
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
                use $crate::ui::style::StyleField;
                $(
                    $($(
                        if let Some(v) = $crate::ui::style::standard(s, $alias).and_then(|v| StyleField::from_css(&v)) {
                            self.$f = v;
                        }
                    )+)?
                    let name = concat!("--", stringify!($f)).replace('_', "-");
                    if let Some(v) = s.custom(&name).and_then(StyleField::from_css) {
                        self.$f = v;
                    }
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
    fn rejects_descendant_selectors() {
        assert!(Sheet::parse(CSS).is_err());
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
}
