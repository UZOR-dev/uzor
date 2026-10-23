//! Declarative motion for the style layer (DOM-like roadmap item 8): CSS
//! `transition` and `@keyframes` / `animation`, sampled per frame from resolved
//! [`Style`]s, with `prefers-reduced-motion` respected.
//!
//! ```
//! use uzor::ui::style::{Node, Sheet, State};
//! use uzor::ui::motion::Motion;
//! let sheet = Sheet::parse(".btn{background:#000000;transition:background 100ms linear}
//!     .btn:hover{background:#ffffff}").unwrap();
//! let mut m = Motion::default();
//! let rest = sheet.resolve(&Node::new("button").class("btn"), None);
//! let hover = sheet.resolve(&Node::new("button").class("btn").state(State::HOVER), None);
//! m.style("ok", &rest, &sheet, 0.0);
//! let mid = m.style("ok", &hover, &sheet, 0.05); // hover starts the transition
//! let mid = m.style("ok", &hover, &sheet, 0.10);
//! assert_eq!(mid.background.as_deref(), Some("rgba(128,128,128,1)"));
//! m.end_frame(0.10); // once per frame, after every element was styled
//! assert!(m.running()); // ask the host for another frame
//! ```
//!
//! A [`Motion`] keeps one track per (element key, property). Each frame the
//! app hands it the element's target style and gets back the style to paint.
//! It interpolates colours (`color`, `background`, `border-color`) and lengths
//! (`opacity`, `border-width`, `border-radius`, `width`, `height`,
//! `font-size`). While [`Motion::running`] is true, the app asks for another
//! frame. With `reduced` set, transitions jump to their end and keyframe
//! animations hold their first frame, as `prefers-reduced-motion: reduce` does
//! for the CSS that honours it.

use std::collections::HashMap;

use super::style::{Sheet, Style};

/// A CSS timing function.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Timing {
    /// `linear`.
    Linear,
    /// `cubic-bezier(x1, y1, x2, y2)` (`ease`, `ease-in`, `ease-out`, `ease-in-out`).
    Bezier(f64, f64, f64, f64),
    /// `steps(n)` (jump-end).
    Steps(u32),
}

impl Default for Timing {
    fn default() -> Self {
        Timing::EASE
    }
}

impl Timing {
    /// CSS `ease`, the initial value.
    pub const EASE: Timing = Timing::Bezier(0.25, 0.1, 0.25, 1.0);

    /// Parse a timing keyword or function; `None` when `v` is not one.
    pub fn parse(v: &str) -> Option<Timing> {
        Some(match v.trim() {
            "linear" => Timing::Linear,
            "ease" => Timing::EASE,
            "ease-in" => Timing::Bezier(0.42, 0.0, 1.0, 1.0),
            "ease-out" => Timing::Bezier(0.0, 0.0, 0.58, 1.0),
            "ease-in-out" => Timing::Bezier(0.42, 0.0, 0.58, 1.0),
            "step-end" => Timing::Steps(1),
            v => {
                if let Some(a) = v.strip_prefix("cubic-bezier(").and_then(|r| r.strip_suffix(')')) {
                    let n: Vec<f64> = a.split(',').map(|x| x.trim().parse().ok()).collect::<Option<_>>()?;
                    let [x1, y1, x2, y2] = n[..] else { return None };
                    Timing::Bezier(x1.clamp(0.0, 1.0), y1, x2.clamp(0.0, 1.0), y2)
                } else if let Some(a) = v.strip_prefix("steps(").and_then(|r| r.strip_suffix(')')) {
                    Timing::Steps(a.split(',').next()?.trim().parse().ok().filter(|n| *n > 0)?)
                } else {
                    return None;
                }
            }
        })
    }

    /// Eased progress for linear progress `t` in 0..=1.
    pub fn at(self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Timing::Linear => t,
            Timing::Steps(n) => (t * n as f64).floor() / n as f64,
            Timing::Bezier(x1, y1, x2, y2) => {
                let b = |a: f64, c: f64, s: f64| 3.0 * a * s * (1.0 - s) * (1.0 - s) + 3.0 * c * s * s * (1.0 - s) + s * s * s;
                // Solve x(s) = t by bisection (monotone for x1, x2 in 0..=1).
                let (mut lo, mut hi) = (0.0, 1.0);
                for _ in 0..24 {
                    let mid = (lo + hi) / 2.0;
                    if b(x1, x2, mid) < t {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                b(y1, y2, (lo + hi) / 2.0)
            }
        }
    }
}

/// Seconds from a CSS time (`150ms`, `.2s`).
fn secs(v: &str) -> Option<f64> {
    let v = v.trim();
    if let Some(n) = v.strip_suffix("ms") {
        n.parse::<f64>().ok().map(|x| x / 1000.0)
    } else {
        v.strip_suffix('s')?.parse().ok()
    }
}

/// One `transition` entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    /// CSS property name, or `all`.
    pub property: String,
    /// Seconds.
    pub duration: f64,
    /// Seconds.
    pub delay: f64,
    /// Timing.
    pub timing: Timing,
}

/// Parse a `transition` value (`background 150ms ease-out, opacity .2s`).
pub fn parse_transitions(v: &str) -> Vec<Transition> {
    if v.trim() == "none" {
        return Vec::new();
    }
    split_top(v)
        .into_iter()
        .filter_map(|part| {
            let mut t = Transition { property: "all".into(), duration: 0.0, delay: 0.0, timing: Timing::EASE };
            let mut times = 0;
            for w in words(part) {
                if let Some(s) = secs(w) {
                    if times == 0 { t.duration = s } else { t.delay = s }
                    times += 1;
                } else if let Some(f) = Timing::parse(w) {
                    t.timing = f;
                } else {
                    t.property = w.to_ascii_lowercase();
                }
            }
            (t.duration > 0.0).then_some(t)
        })
        .collect()
}

/// Iteration direction of an `animation`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Direction {
    /// `normal`.
    #[default]
    Normal,
    /// `reverse`.
    Reverse,
    /// `alternate`.
    Alternate,
    /// `alternate-reverse`.
    AlternateReverse,
}

/// One `animation` entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    /// `@keyframes` name.
    pub name: String,
    /// Seconds per iteration.
    pub duration: f64,
    /// Seconds.
    pub delay: f64,
    /// Timing (per keyframe interval, as CSS).
    pub timing: Timing,
    /// Iterations (`infinite` = `f64::INFINITY`).
    pub iterations: f64,
    /// Direction.
    pub direction: Direction,
    /// `forwards` / `both`: hold the last frame after the end.
    pub fill_forwards: bool,
}

/// Parse an `animation` shorthand (`pulse 2s ease-in-out infinite alternate`).
pub fn parse_animations(v: &str) -> Vec<Animation> {
    if v.trim() == "none" {
        return Vec::new();
    }
    split_top(v)
        .into_iter()
        .filter_map(|part| {
            let mut a = Animation { name: String::new(), duration: 0.0, delay: 0.0, timing: Timing::EASE, iterations: 1.0, direction: Direction::Normal, fill_forwards: false };
            let mut times = 0;
            for w in words(part) {
                if let Some(s) = secs(w) {
                    if times == 0 { a.duration = s } else { a.delay = s }
                    times += 1;
                } else if let Some(f) = Timing::parse(w) {
                    a.timing = f;
                } else {
                    match w {
                        "infinite" => a.iterations = f64::INFINITY,
                        "normal" => a.direction = Direction::Normal,
                        "reverse" => a.direction = Direction::Reverse,
                        "alternate" => a.direction = Direction::Alternate,
                        "alternate-reverse" => a.direction = Direction::AlternateReverse,
                        "forwards" | "both" => a.fill_forwards = true,
                        "none" | "backwards" | "running" => {}
                        w => match w.parse::<f64>() {
                            Ok(n) => a.iterations = n.max(0.0),
                            Err(_) => a.name = w.to_string(),
                        },
                    }
                }
            }
            (!a.name.is_empty() && a.duration > 0.0).then_some(a)
        })
        .collect()
}

/// `a, b(c, d), e` → `["a", "b(c, d)", "e"]` (commas inside parentheses stay).
fn split_top(v: &str) -> Vec<&str> {
    let (mut out, mut depth, mut start) = (Vec::new(), 0i32, 0);
    for (i, c) in v.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push(v[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(v[start..].trim());
    out.retain(|s| !s.is_empty());
    out
}

/// Whitespace-separated words, keeping `fn(a, b)` whole.
fn words(v: &str) -> Vec<&str> {
    let (mut out, mut depth, mut start) = (Vec::new(), 0i32, None);
    for (i, c) in v.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            c if c.is_whitespace() && depth == 0 => {
                if let Some(s) = start.take() {
                    out.push(&v[s..i]);
                }
                continue;
            }
            _ => {}
        }
        start.get_or_insert(i);
    }
    if let Some(s) = start {
        out.push(&v[s..]);
    }
    out
}

/// One `@keyframes` rule: stops (offset 0..=1, declarations) sorted by offset.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Keyframes {
    /// Stops.
    pub stops: Vec<(f64, Vec<(String, String)>)>,
}

impl Keyframes {
    /// Parse the body of `@keyframes name { ... }`.
    pub(crate) fn parse(body: &str) -> Option<Keyframes> {
        let mut kf = Keyframes::default();
        let mut rest = body;
        while let Some(open) = rest.find('{') {
            let close = rest[open..].find('}')? + open;
            let decls: Vec<(String, String)> = rest[open + 1..close]
                .split(';')
                .filter_map(|d| d.split_once(':'))
                .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
                .collect();
            for sel in rest[..open].split(',') {
                let off = match sel.trim() {
                    "from" => 0.0,
                    "to" => 1.0,
                    p => p.strip_suffix('%')?.trim().parse::<f64>().ok()? / 100.0,
                };
                kf.stops.push((off.clamp(0.0, 1.0), decls.clone()));
            }
            rest = &rest[close + 1..];
        }
        crate::sort::sort_by_key(&mut kf.stops, |s| (s.0 * 1e6) as i64);
        (!kf.stops.is_empty()).then_some(kf)
    }
}

/// An animatable value.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Val {
    Num(f64),
    Rgba([f64; 4]),
}

const COLORS: [&str; 3] = ["color", "background", "border-color"];
const NUMS: [&str; 6] = ["opacity", "border-width", "border-radius", "width", "height", "font-size"];

fn get(s: &Style, p: &str) -> Option<Val> {
    let c = |v: &Option<String>| {
        v.as_deref().map(|x| {
            let (r, g, b, a) = crate::core::render::parse_color(x);
            Val::Rgba([r as f64, g as f64, b as f64, a as f64 / 255.0])
        })
    };
    match p {
        "color" => c(&s.color),
        "background" => c(&s.background),
        "border-color" => c(&s.border_color),
        "opacity" => s.opacity.map(Val::Num),
        "border-width" => s.border_width.map(Val::Num),
        "border-radius" => s.border_radius.map(Val::Num),
        "width" => s.width.map(Val::Num),
        "height" => s.height.map(Val::Num),
        "font-size" => s.font_size.map(Val::Num),
        _ => None,
    }
}

fn put(s: &mut Style, p: &str, v: Val) {
    match v {
        Val::Rgba([r, g, b, a]) => {
            let a = (a * 1000.0).round() / 1000.0;
            let txt = Some(format!("rgba({},{},{},{})", r.round() as u8, g.round() as u8, b.round() as u8, a));
            match p {
                "color" => s.color = txt,
                "background" => s.background = txt,
                "border-color" => s.border_color = txt,
                _ => {}
            }
        }
        Val::Num(n) => {
            let n = Some(n);
            match p {
                "opacity" => s.opacity = n,
                "border-width" => s.border_width = n,
                "border-radius" => s.border_radius = n,
                "width" => s.width = n,
                "height" => s.height = n,
                "font-size" => s.font_size = n,
                _ => {}
            }
        }
    }
}

fn lerp(a: Val, b: Val, t: f64) -> Val {
    match (a, b) {
        (Val::Num(x), Val::Num(y)) => Val::Num(x + (y - x) * t),
        (Val::Rgba(x), Val::Rgba(y)) => Val::Rgba([0, 1, 2, 3].map(|i| x[i] + (y[i] - x[i]) * t)),
        _ => b,
    }
}

/// A running property transition.
#[derive(Clone, Debug)]
struct Track {
    from: Val,
    to: Val,
    start: f64,
    tr: Transition,
}

impl Track {
    fn at(&self, now: f64) -> Val {
        let t = ((now - self.start - self.tr.delay) / self.tr.duration).clamp(0.0, 1.0);
        lerp(self.from, self.to, self.tr.timing.at(t))
    }
    fn done(&self, now: f64) -> bool {
        now >= self.start + self.tr.delay + self.tr.duration
    }
}

#[derive(Clone, Debug, Default)]
struct Element {
    /// Last target per property and the value painted last frame.
    target: HashMap<&'static str, (Val, Val)>,
    tracks: HashMap<&'static str, Track>,
    /// Animation name → start time.
    anims: HashMap<String, f64>,
    seen: bool,
}

/// Per-element transition and animation state (see the module docs).
#[derive(Clone, Debug, Default)]
pub struct Motion {
    els: HashMap<String, Element>,
    /// `prefers-reduced-motion: reduce`.
    pub reduced: bool,
    running: bool,
}

impl Motion {
    /// A motion store honouring `reduced`.
    pub fn new(reduced: bool) -> Self {
        Motion { reduced, ..Default::default() }
    }

    /// The style to paint for element `key` whose cascaded style is `target`,
    /// at `now` seconds. `sheet` supplies `@keyframes`.
    pub fn style(&mut self, key: &str, target: &Style, sheet: &Sheet, now: f64) -> Style {
        let reduced = self.reduced;
        let el = self.els.entry(key.to_string()).or_default();
        el.seen = true;
        let mut out = target.clone();
        // Transitions: a changed target starts a track from the painted value.
        for p in COLORS.iter().chain(NUMS.iter()).copied() {
            let Some(to) = get(target, p) else {
                el.target.remove(p);
                el.tracks.remove(p);
                continue;
            };
            let tr = target.transitions.iter().rev().find(|t| t.property == p || t.property == "all");
            let shown = match el.target.get(p) {
                Some(&(old, painted)) if old != to => match tr.filter(|_| !reduced) {
                    Some(tr) => {
                        el.tracks.insert(p, Track { from: painted, to, start: now, tr: tr.clone() });
                        painted
                    }
                    None => {
                        el.tracks.remove(p);
                        to
                    }
                },
                _ => match el.tracks.get(p) {
                    Some(t) if !t.done(now) => t.at(now),
                    _ => {
                        el.tracks.remove(p);
                        to
                    }
                },
            };
            el.target.insert(p, (to, shown));
            // At rest the cascaded text stays as written.
            if shown != to {
                put(&mut out, p, shown);
            }
        }
        // Keyframe animations, over the transitioned values (as CSS).
        let names: Vec<&str> = target.animations.iter().map(|a| a.name.as_str()).collect();
        el.anims.retain(|n, _| names.contains(&n.as_str()));
        for a in &target.animations {
            let Some(kf) = sheet.keyframes(&a.name) else { continue };
            let start = *el.anims.entry(a.name.clone()).or_insert(now);
            let Some(p) = sample(a, kf, now - start, reduced) else { continue };
            for (k, v) in p.props {
                let base = get(&out, k);
                let from = p.from.iter().find(|(f, _)| f == k).map(|(_, v)| v.as_str());
                let val = |raw: Option<&str>| raw.and_then(|r| {
                    let mut s = Style::default();
                    super::style::apply_decl(&mut s, k, r);
                    get(&s, k)
                }).or(base);
                if let (Some(a0), Some(b0)) = (val(from), val(Some(v))) {
                    put(&mut out, k, lerp(a0, b0, p.t));
                }
            }
        }
        out
    }

    /// Whether any element is still moving (ask for another frame).
    pub fn running(&self) -> bool {
        self.running
    }

    /// Close the frame: forget elements not styled this frame and work out
    /// [`Motion::running`] for `now`.
    pub fn end_frame(&mut self, now: f64) {
        self.els.retain(|_, e| std::mem::take(&mut e.seen));
        let reduced = self.reduced;
        self.running = self.els.values().any(|e| e.tracks.values().any(|t| !t.done(now)) || (!reduced && !e.anims.is_empty()));
    }
}

/// A keyframe sample: the properties of the next stop, the previous stop's
/// raw values, and the eased fraction between them.
struct Sampled<'a> {
    props: &'a [(String, String)],
    from: &'a [(String, String)],
    t: f64,
}

fn sample<'a>(a: &Animation, kf: &'a Keyframes, elapsed: f64, reduced: bool) -> Option<Sampled<'a>> {
    let local = if reduced { 0.0 } else { elapsed - a.delay };
    if local < 0.0 {
        return None;
    }
    let total = a.duration * a.iterations;
    let (iter, frac) = if local >= total {
        if !a.fill_forwards {
            return None;
        }
        let n = a.iterations.max(1.0);
        (n - 1.0, if a.iterations.fract() == 0.0 { 1.0 } else { a.iterations.fract() })
    } else {
        let x = local / a.duration;
        (x.floor(), x.fract())
    };
    let odd = iter as u64 % 2 == 1;
    let p = match a.direction {
        Direction::Normal => frac,
        Direction::Reverse => 1.0 - frac,
        Direction::Alternate => if odd { 1.0 - frac } else { frac },
        Direction::AlternateReverse => if odd { frac } else { 1.0 - frac },
    };
    let s = &kf.stops;
    let i = s.iter().position(|(o, _)| *o >= p).unwrap_or(s.len() - 1);
    if i == 0 || s[i].0 <= p {
        return Some(Sampled { props: &s[i].1, from: &s[i].1, t: 1.0 });
    }
    let (o0, o1) = (s[i - 1].0, s[i].0);
    let t = a.timing.at((p - o0) / (o1 - o0));
    Some(Sampled { props: &s[i].1, from: &s[i - 1].1, t })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::style::{Node, State};

    fn sheet() -> Sheet {
        Sheet::parse(
            ".b{background:#000000;opacity:1;transition:background 100ms linear, opacity 200ms linear 100ms}
             .b:hover{background:#ffffff;opacity:0}
             @keyframes pulse { from { opacity: 0 } 50% { opacity: 1 } to { opacity: 0.5 } }
             .p{opacity:1;animation:pulse 1s linear 2 forwards}
             .s{animation:spin 1s linear infinite alternate}",
        )
        .unwrap()
    }

    #[test]
    fn parses_transition_and_animation_shorthands() {
        let t = parse_transitions("background 150ms ease-out, opacity .2s cubic-bezier(0, 0, 1, 1) 50ms");
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].property.as_str(), t[0].duration, t[0].timing), ("background", 0.15, Timing::Bezier(0.0, 0.0, 0.58, 1.0)));
        assert_eq!((t[1].duration, t[1].delay), (0.2, 0.05));
        assert!(parse_transitions("none").is_empty());
        let a = parse_animations("spin 2s steps(4) infinite alternate-reverse");
        assert_eq!((a[0].name.as_str(), a[0].iterations, a[0].direction, a[0].timing), ("spin", f64::INFINITY, Direction::AlternateReverse, Timing::Steps(4)));
        let s = sheet();
        let kf = s.keyframes("pulse").unwrap();
        assert_eq!(kf.stops.iter().map(|s| s.0).collect::<Vec<_>>(), [0.0, 0.5, 1.0]);
        assert!(Sheet::parse("@keyframes x { nope { a: b } }").is_err());
    }

    #[test]
    fn timing_functions_hit_their_ends_and_ease() {
        for f in [Timing::Linear, Timing::EASE, Timing::parse("ease-in-out").unwrap()] {
            assert!(f.at(0.0).abs() < 1e-6 && (f.at(1.0) - 1.0).abs() < 1e-6);
        }
        assert!(Timing::EASE.at(0.5) > 0.75, "ease front-loads");
        assert_eq!(Timing::Steps(4).at(0.3), 0.25);
    }

    #[test]
    fn transition_interpolates_from_the_painted_value_with_delay() {
        let s = sheet();
        let rest = s.resolve(&Node::new("x").class("b"), None);
        let hov = s.resolve(&Node::new("x").class("b").state(State::HOVER), None);
        let mut m = Motion::default();
        assert_eq!(m.style("k", &rest, &s, 0.0).background.as_deref(), Some("#000000"), "first sight: no transition");
        m.end_frame(0.0);
        assert!(!m.running());
        let a = m.style("k", &hov, &s, 1.0);
        assert_eq!(a.background.as_deref(), Some("rgba(0,0,0,1)"));
        m.end_frame(1.0);
        assert!(m.running());
        let b = m.style("k", &hov, &s, 1.05);
        assert_eq!(b.background.as_deref(), Some("rgba(128,128,128,1)"));
        assert_eq!(b.opacity, Some(1.0), "opacity waits out its 100ms delay");
        let c = m.style("k", &hov, &s, 1.2);
        assert!((c.opacity.unwrap() - 0.5).abs() < 1e-9);
        // Back to rest mid-way: starts from what was painted, not from the end.
        let d = m.style("k", &rest, &s, 1.2);
        assert!((d.opacity.unwrap() - 0.5).abs() < 1e-9);
        let e = m.style("k", &rest, &s, 2.0);
        assert_eq!((e.background.as_deref(), e.opacity), (Some("#000000"), Some(1.0)));
        m.end_frame(2.0);
        assert!(!m.running());
    }

    #[test]
    fn keyframes_sample_iterate_and_fill() {
        let s = sheet();
        let p = s.resolve(&Node::new("x").class("p"), None);
        let mut m = Motion::default();
        let at = |m: &mut Motion, t: f64| m.style("p", &p, &s, t).opacity.unwrap();
        assert_eq!(at(&mut m, 10.0), 0.0);
        assert!((at(&mut m, 10.25) - 0.5).abs() < 1e-9);
        assert!((at(&mut m, 10.75) - 0.75).abs() < 1e-9);
        assert!((at(&mut m, 11.25) - 0.5).abs() < 1e-9, "second iteration");
        assert_eq!(at(&mut m, 13.0), 0.5, "forwards holds the last frame");
        m.end_frame(13.0);
        // An element not styled for a frame is forgotten (animation restarts).
        m.end_frame(13.1);
        assert_eq!(at(&mut m, 20.0), 0.0);
    }

    #[test]
    fn reduced_motion_jumps_and_holds_the_first_frame() {
        let s = sheet();
        let rest = s.resolve(&Node::new("x").class("b"), None);
        let hov = s.resolve(&Node::new("x").class("b").state(State::HOVER), None);
        let p = s.resolve(&Node::new("x").class("p"), None);
        let mut m = Motion::new(true);
        m.style("k", &rest, &s, 0.0);
        assert_eq!(m.style("k", &hov, &s, 1.0).background.as_deref(), Some("#ffffff"));
        assert_eq!(m.style("p", &p, &s, 1.0).opacity, Some(0.0));
        assert_eq!(m.style("p", &p, &s, 1.4).opacity, Some(0.0));
        m.end_frame(1.4);
        assert!(!m.running(), "reduced motion never keeps frames coming");
    }
}
