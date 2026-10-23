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
    order: usize,
    decls: Vec<(String, String)>,
}

/// A parsed style sheet.
#[derive(Clone, Debug, Default)]
pub struct Sheet {
    rules: Vec<Rule>,
    vars: HashMap<String, String>,
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
                sheet.rules.push(Rule { sel: Selector::parse(s)?, order, decls: decls.clone() });
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

    fn subst(&self, v: &str, depth: u32) -> String {
        let Some(i) = v.find("var(") else { return v.to_string() };
        let Some(j) = v[i..].find(')').map(|j| i + j) else { return v.to_string() };
        let inner = &v[i + 4..j];
        let (name, fb) = inner.split_once(',').map(|(a, b)| (a.trim(), Some(b.trim()))).unwrap_or((inner.trim(), None));
        let val = if depth > 8 { None } else { self.vars.get(name).map(|x| self.subst(x, depth + 1)) };
        let val = val.or(fb.map(str::to_string)).unwrap_or_default();
        format!("{}{}{}", &v[..i], val, self.subst(&v[j + 1..], depth))
    }

    /// Cascade every matching rule (specificity, then source order) over the inherited
    /// properties of `parent`.
    pub fn resolve(&self, n: &Node<'_>, parent: Option<&Style>) -> Style {
        let mut s = Style::default();
        if let Some(p) = parent {
            s.color = p.color.clone();
            s.font_size = p.font_size;
            s.font_weight = p.font_weight;
            s.font_family = p.font_family.clone();
            s.line_height = p.line_height;
        }
        let mut hits: Vec<&Rule> = self.rules.iter().filter(|r| r.sel.matches(n)).collect();
        hits.sort_by_key(|r| (r.sel.specificity(), r.order));
        let mut lh_raw: Option<String> = None;
        for r in hits {
            for (k, v) in &r.decls {
                let v = self.subst(v, 0);
                if k == "line-height" {
                    lh_raw = Some(v.clone());
                }
                apply(&mut s, k, &v);
            }
        }
        // A unitless line-height multiplies the final font size.
        if let Some(raw) = lh_raw {
            if let Ok(m) = raw.trim().parse::<f64>() {
                s.line_height = Some(m * s.font_size.unwrap_or(16.0));
            }
        }
        s
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
        _ => {}
    }
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
    fn border_shorthand_and_comments() {
        let sh = Sheet::parse("/* x */ .c{border:1px solid #1b1b1b;border-radius:12px;margin:0 0 8px}").unwrap();
        let s = sh.resolve(&Node::new("div").class("c"), None);
        assert_eq!((s.border_width, s.border_color.as_deref(), s.border_radius, s.margin), (Some(1.0), Some("#1b1b1b"), Some(12.0), Some([0.0, 0.0, 8.0, 0.0])));
        assert!(Sheet::parse(".a{color:#fff").is_err());
        assert!(Sheet::parse(".a{color}").is_err());
    }
}
