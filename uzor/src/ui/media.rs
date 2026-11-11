//! The environment CSS media queries and `env()` read: viewport size, pointer kind,
//! safe-area and soft-keyboard insets, colour scheme, reduced motion.
//!
//! [`Sheet::parse`](super::style::Sheet::parse) evaluates `@media` blocks and
//! substitutes `env(...)` against [`current`] at parse time; hosts call
//! [`set_current`] when the window, its insets or the keyboard change, and apps
//! re-parse their sheet when [`generation`] moved (or parse explicitly with
//! [`Sheet::parse_with`](super::style::Sheet::parse_with)).
//!
//! Supported queries: media types `all` / `screen` (and `only`, `print` never
//! matches), `and`, comma lists (or), a leading `not`, and the features
//! `width` / `height` with `min-` / `max-`, `orientation: portrait | landscape`,
//! `pointer` / `any-pointer: coarse | fine | none`, `hover` / `any-hover: hover | none`,
//! `prefers-color-scheme: dark | light`, `prefers-reduced-motion: reduce | no-preference`.
//! Lengths are `px` (or unitless), `em` / `rem` at 16 px.
//!
//! `env()` names: `safe-area-inset-top | right | bottom | left` and
//! `keyboard-inset-height` (the soft keyboard), in px.
//!
//! Viewport and root units in values (`100vh`, `50vw`, `10vmin`, `5vmax`, `1.5rem`)
//! become px at parse time too ([`MediaEnv::units`]; `rem` is 16 px).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

/// Insets in logical px (top, right, bottom, left as in CSS).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Edges {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Edges {
    pub const ZERO: Edges = Edges { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 };
    pub fn new(top: f64, right: f64, bottom: f64, left: f64) -> Self {
        Edges { top, right, bottom, left }
    }
}

/// What media queries test.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediaEnv {
    /// Viewport width, logical px.
    pub width: f64,
    /// Viewport height, logical px.
    pub height: f64,
    /// The primary pointer is a finger (`pointer: coarse`).
    pub coarse: bool,
    /// The primary pointer can hover (`hover: hover`).
    pub hover: bool,
    /// Safe area (notch, status bar, home indicator, rounded corners).
    pub safe: Edges,
    /// Soft keyboard height over the bottom of the viewport.
    pub keyboard: f64,
    /// `prefers-color-scheme: dark`.
    pub dark: bool,
    /// `prefers-reduced-motion: reduce`.
    pub reduced_motion: bool,
}

impl Default for MediaEnv {
    /// A desktop window: 1280 x 800, mouse, no insets, dark.
    fn default() -> Self {
        Self::desktop(1280.0, 800.0)
    }
}

impl MediaEnv {
    /// A mouse-driven window of this size.
    pub fn desktop(width: f64, height: f64) -> Self {
        MediaEnv { width, height, coarse: false, hover: true, safe: Edges::ZERO, keyboard: 0.0, dark: true, reduced_motion: false }
    }

    /// A touch screen of this size with these safe-area insets.
    pub fn touch(width: f64, height: f64, safe: Edges) -> Self {
        MediaEnv { coarse: true, hover: false, safe, ..Self::desktop(width, height) }
    }

    /// The value of an `env()` variable, px.
    pub fn env(&self, name: &str) -> Option<f64> {
        Some(match name {
            "safe-area-inset-top" => self.safe.top,
            "safe-area-inset-right" => self.safe.right,
            "safe-area-inset-bottom" => self.safe.bottom,
            "safe-area-inset-left" => self.safe.left,
            "keyboard-inset-height" => self.keyboard,
            _ => return None,
        })
    }

    /// Whether a media query list (`screen and (max-width: 600px), (pointer: coarse)`) matches.
    pub fn matches(&self, query: &str) -> bool {
        let q = query.trim();
        if q.is_empty() {
            return true;
        }
        split_top(q, ',').iter().any(|part| self.matches_one(part.trim()))
    }

    fn matches_one(&self, q: &str) -> bool {
        let lower = q.to_ascii_lowercase();
        let (neg, rest) = match lower.strip_prefix("not ") {
            Some(r) => (true, r.trim().to_string()),
            None => (false, lower.clone()),
        };
        let mut ok = true;
        for term in split_and(&rest) {
            let t = term.trim();
            let t = t.strip_prefix("only ").map(str::trim).unwrap_or(t);
            let r = if let Some(inner) = t.strip_prefix('(').and_then(|x| x.strip_suffix(')')) {
                self.feature(inner.trim())
            } else {
                match t {
                    "all" | "screen" | "" => Some(true),
                    "print" | "speech" => Some(false),
                    _ => None,
                }
            };
            // Unknown terms make the query false (as in the browser).
            ok &= r.unwrap_or(false);
        }
        ok != neg
    }

    fn feature(&self, f: &str) -> Option<bool> {
        let (name, val) = match f.split_once(':') {
            Some((n, v)) => (n.trim(), Some(v.trim())),
            None => (f, None),
        };
        let len = |v: Option<&str>| v.and_then(length);
        Some(match name {
            "min-width" => self.width >= len(val)?,
            "max-width" => self.width <= len(val)?,
            "width" => (self.width - len(val)?).abs() < 0.5,
            "min-height" => self.height >= len(val)?,
            "max-height" => self.height <= len(val)?,
            "height" => (self.height - len(val)?).abs() < 0.5,
            "orientation" => match val? {
                "portrait" => self.height >= self.width,
                "landscape" => self.width > self.height,
                _ => return None,
            },
            "pointer" | "any-pointer" => match val {
                Some("coarse") => self.coarse,
                Some("fine") => !self.coarse,
                Some("none") => false,
                None => true,
                _ => return None,
            },
            "hover" | "any-hover" => match val {
                Some("hover") | None => self.hover,
                Some("none") => !self.hover,
                _ => return None,
            },
            "prefers-color-scheme" => match val? {
                "dark" => self.dark,
                "light" => !self.dark,
                _ => return None,
            },
            "prefers-reduced-motion" => match val {
                Some("reduce") | None => self.reduced_motion,
                Some("no-preference") => !self.reduced_motion,
                _ => return None,
            },
            _ => return None,
        })
    }

    /// `v` with every `env(name[, fallback])` replaced by its px value (unknown names
    /// take the fallback, else `0px`).
    pub fn substitute(&self, v: &str) -> String {
        let Some(i) = v.find("env(") else { return v.to_string() };
        // The matching paren (fallbacks may hold parens: `env(x, calc(1px))`).
        let mut depth = 0;
        let mut j = None;
        for (k, c) in v[i + 3..].char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        j = Some(i + 3 + k);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(j) = j else { return v.to_string() };
        let inner = &v[i + 4..j];
        let (name, fb) = inner.split_once(',').map(|(a, b)| (a.trim(), Some(b.trim()))).unwrap_or((inner.trim(), None));
        let val = match self.env(name) {
            Some(px) => format!("{}px", fmt_px(px)),
            None => fb.map(str::to_string).unwrap_or_else(|| "0px".into()),
        };
        format!("{}{}{}", &v[..i], val, self.substitute(&v[j + 1..]))
    }
}

impl MediaEnv {
    /// `v` with every `<number>vh | vw | vmin | vmax | rem` turned into px against this
    /// viewport (1vh = 1% of the height, `rem` = 16 px). Other text is kept as it is:
    /// a unit only counts right after a number and before a non-letter.
    pub fn units(&self, v: &str) -> String {
        let b = v.as_bytes();
        let mut out = String::with_capacity(v.len());
        let mut i = 0;
        let mut last = 0;
        while i < b.len() {
            let c = b[i];
            // A number starts after a non-identifier char.
            let starts = (c.is_ascii_digit() || (c == b'.' && b.get(i + 1).is_some_and(u8::is_ascii_digit)) || (c == b'-' && b.get(i + 1).is_some_and(|d| d.is_ascii_digit() || *d == b'.')))
                && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'#' || b[i - 1] == b'-' || b[i - 1] == b'_' || b[i - 1] == b'.'));
            if !starts {
                i += 1;
                continue;
            }
            let s = i;
            i += 1;
            while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
                i += 1;
            }
            let num_end = i;
            while i < b.len() && b[i].is_ascii_alphabetic() {
                i += 1;
            }
            let unit = &v[num_end..i];
            let scale = match unit {
                "vh" => self.height / 100.0,
                "vw" => self.width / 100.0,
                "vmin" => self.width.min(self.height) / 100.0,
                "vmax" => self.width.max(self.height) / 100.0,
                "rem" => 16.0,
                _ => continue,
            };
            let Ok(n) = v[s..num_end].parse::<f64>() else { continue };
            out.push_str(&v[last..s]);
            out.push_str(&fmt_px((n * scale * 100.0).round() / 100.0));
            out.push_str("px");
            last = i;
        }
        out.push_str(&v[last..]);
        out
    }
}

fn fmt_px(v: f64) -> String {
    if v.fract() == 0.0 { format!("{}", v as i64) } else { format!("{v}") }
}

fn length(v: &str) -> Option<f64> {
    let v = v.trim();
    if let Some(n) = v.strip_suffix("rem").or_else(|| v.strip_suffix("em")) {
        return n.trim().parse::<f64>().ok().map(|x| x * 16.0);
    }
    v.strip_suffix("px").unwrap_or(v).trim().parse().ok()
}

/// Split at `sep` outside parentheses.
fn split_top(s: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut depth, mut start) = (0i32, 0);
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            c if c == sep && depth == 0 => {
                out.push(&s[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

/// Split a lower-cased query at ` and ` outside parentheses.
fn split_and(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i] as char;
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
        }
        if depth == 0 && s[i..].starts_with(" and ") {
            out.push(std::mem::take(&mut cur));
            i += 5;
            continue;
        }
        if depth == 0 && s[i..].starts_with("and (") && cur.trim().is_empty() {
            i += 4;
            continue;
        }
        cur.push(c);
        i += 1;
    }
    out.push(cur);
    out.into_iter().filter(|t| !t.trim().is_empty()).collect()
}

static CURRENT: RwLock<Option<MediaEnv>> = RwLock::new(None);
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// The environment [`Sheet::parse`](super::style::Sheet::parse) evaluates against.
pub fn current() -> MediaEnv {
    CURRENT.read().ok().and_then(|g| *g).unwrap_or_default()
}

/// Set the environment; returns whether it changed (then [`generation`] moves).
pub fn set_current(env: MediaEnv) -> bool {
    let mut g = CURRENT.write().unwrap_or_else(|e| e.into_inner());
    if g.as_ref() == Some(&env) {
        return false;
    }
    *g = Some(env);
    GENERATION.fetch_add(1, Ordering::Relaxed);
    true
}

/// Bumped on every change of [`current`]: re-parse sheets when it moved.
pub fn generation() -> u64 {
    GENERATION.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_orientation_and_lists() {
        let phone = MediaEnv::touch(390.0, 844.0, Edges::new(47.0, 0.0, 34.0, 0.0));
        let desk = MediaEnv::desktop(1440.0, 900.0);
        assert!(phone.matches("(max-width: 600px)"));
        assert!(!desk.matches("(max-width: 600px)"));
        assert!(desk.matches("screen and (min-width: 1024px) and (orientation: landscape)"));
        assert!(phone.matches("(orientation: portrait)"));
        assert!(phone.matches("print, (max-width: 37.5em)"), "comma = or, em at 16px");
        assert!(!phone.matches("print"));
        assert!(desk.matches("not print"));
        assert!(!phone.matches("not screen and (pointer: coarse)"));
        assert!(phone.matches("only screen and (max-width: 600px)"));
        assert!(!phone.matches("(frobnicate: 1)"), "unknown features never match");
    }

    #[test]
    fn pointer_hover_scheme_motion() {
        let phone = MediaEnv::touch(390.0, 844.0, Edges::ZERO);
        let desk = MediaEnv::default();
        assert!(phone.matches("(pointer: coarse)") && !desk.matches("(pointer: coarse)"));
        assert!(desk.matches("(hover: hover)") && phone.matches("(hover: none)"));
        assert!(desk.matches("(any-pointer: fine)"));
        assert!(desk.matches("(prefers-color-scheme: dark)"));
        assert!(desk.matches("(prefers-reduced-motion: no-preference)"));
    }

    #[test]
    fn env_substitution() {
        let phone = MediaEnv { keyboard: 291.0, ..MediaEnv::touch(390.0, 844.0, Edges::new(47.0, 0.0, 34.0, 0.0)) };
        assert_eq!(phone.substitute("env(safe-area-inset-top)"), "47px");
        assert_eq!(phone.substitute("0 12px env(safe-area-inset-bottom, 8px)"), "0 12px 34px");
        assert_eq!(phone.substitute("env(keyboard-inset-height)"), "291px");
        assert_eq!(phone.substitute("env(nope, 5px)"), "5px");
        assert_eq!(phone.substitute("env(nope)"), "0px");
        assert_eq!(phone.substitute("solid"), "solid");
    }

    #[test]
    fn viewport_and_root_units() {
        let phone = MediaEnv::touch(390.0, 844.0, Edges::ZERO);
        assert_eq!(phone.units("100vh"), "844px");
        assert_eq!(phone.units("50vw"), "195px");
        assert_eq!(phone.units("0 2vmin 10vmax"), "0 7.8px 84.4px");
        assert_eq!(phone.units("1.5rem 12px"), "24px 12px");
        assert_eq!(phone.units("-10vw"), "-39px");
        assert_eq!(phone.units("#10vh1a rgba(0,0,0,.5) 3em"), "#10vh1a rgba(0,0,0,.5) 3em", "colours, em and plain numbers stay");
        assert_eq!(phone.units("a1vh solid"), "a1vh solid");
    }
}
