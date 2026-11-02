//! Selectors over an [`A11yTree`] — the lookup automation and tests use to
//! name a widget without coordinates.
//!
//! Grammar (whitespace around parts is ignored):
//!
//! ```text
//! selector := [ '#' id ] [ role ] ( '[' attr op value ']' )* [ ':nth(' n ')' ]
//! attr     := id | name | value | kind | role | focused | disabled | masked
//! op       := '='  (exact) | '~=' (contains, case-insensitive) | '^=' (prefix)
//! value    := "quoted string" | bare-word
//! ```
//!
//! Examples: `#lh-nick`, `button[name="Sign in"]`, `textinput[name~=nick]`,
//! `[id^=room-]:nth(0)`. A role is the lower-case [`A11yRole`] name
//! (`button`, `checkbox`, `textinput`, …). Without `:nth` a selector must
//! match exactly one node; more is [`QueryError::Ambiguous`].

use super::{A11yNode, A11yRole, A11yTree};

/// How an attribute value is compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchOp {
    /// Equal.
    Exact,
    /// Contains, case-insensitive.
    Contains,
    /// Starts with.
    Prefix,
}

/// One `[attr op value]` predicate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Predicate {
    /// Attribute name.
    pub attr: String,
    /// Comparison.
    pub op: MatchOp,
    /// Expected value.
    pub value: String,
}

/// A parsed selector.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selector {
    /// Role filter.
    pub role: Option<A11yRole>,
    /// Attribute predicates (`#id` is an exact `id` predicate).
    pub preds: Vec<Predicate>,
    /// Pick the n-th match (0-based) instead of requiring a unique one.
    pub nth: Option<usize>,
}

/// Why a selector did not resolve.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryError {
    /// The selector text does not parse.
    Parse(String),
    /// No node matches.
    NotFound,
    /// More than one node matches and no `:nth` was given.
    Ambiguous(usize),
}

impl core::fmt::Display for QueryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(m) => write!(f, "bad selector: {m}"),
            Self::NotFound => write!(f, "no widget matches the selector"),
            Self::Ambiguous(n) => write!(f, "{n} widgets match the selector; add :nth(i) or narrow it"),
        }
    }
}

impl std::error::Error for QueryError {}

/// The lower-case name of a role, as selectors spell it.
pub fn role_name(role: A11yRole) -> &'static str {
    match role {
        A11yRole::Button => "button",
        A11yRole::CheckBox => "checkbox",
        A11yRole::RadioButton => "radio",
        A11yRole::Switch => "switch",
        A11yRole::Slider => "slider",
        A11yRole::TextInput => "textinput",
        A11yRole::Tab => "tab",
        A11yRole::TabList => "tablist",
        A11yRole::MenuItem => "menuitem",
        A11yRole::ListItem => "listitem",
        A11yRole::Group => "group",
        A11yRole::Dialog => "dialog",
        A11yRole::Tooltip => "tooltip",
        A11yRole::Link => "link",
        A11yRole::Unknown => "unknown",
    }
}

/// Parse a role name (see [`role_name`]).
pub fn parse_role(s: &str) -> Option<A11yRole> {
    const ALL: [A11yRole; 15] = [
        A11yRole::Button,
        A11yRole::CheckBox,
        A11yRole::RadioButton,
        A11yRole::Switch,
        A11yRole::Slider,
        A11yRole::TextInput,
        A11yRole::Tab,
        A11yRole::TabList,
        A11yRole::MenuItem,
        A11yRole::ListItem,
        A11yRole::Group,
        A11yRole::Dialog,
        A11yRole::Tooltip,
        A11yRole::Link,
        A11yRole::Unknown,
    ];
    let s = s.to_ascii_lowercase();
    ALL.into_iter().find(|r| role_name(*r) == s)
}

impl Selector {
    /// Parse selector text (see the module docs).
    pub fn parse(src: &str) -> Result<Self, QueryError> {
        let err = |m: &str| QueryError::Parse(m.to_string());
        let s = src.trim();
        if s.is_empty() {
            return Err(err("empty selector"));
        }
        let mut sel = Selector::default();
        let b = s.as_bytes();
        let mut i = 0;
        let word_end = |from: usize| {
            let mut j = from;
            while j < b.len() && !matches!(b[j], b'[' | b':' | b' ') {
                j += 1;
            }
            j
        };
        if b[0] == b'#' {
            let j = word_end(1);
            if j == 1 {
                return Err(err("empty #id"));
            }
            sel.preds.push(Predicate { attr: "id".into(), op: MatchOp::Exact, value: s[1..j].to_string() });
            i = j;
        } else if b[0] != b'[' && b[0] != b':' {
            let j = word_end(0);
            let role = &s[..j];
            sel.role = Some(parse_role(role).ok_or_else(|| QueryError::Parse(format!("unknown role '{role}'")))?);
            i = j;
        }
        while i < b.len() {
            match b[i] {
                b' ' => i += 1,
                b'[' => {
                    let close = find_close(s, i + 1).ok_or_else(|| err("unclosed '['"))?;
                    sel.preds.push(parse_pred(&s[i + 1..close])?);
                    i = close + 1;
                }
                b':' => {
                    let rest = &s[i..];
                    let Some(inner) = rest.strip_prefix(":nth(") else {
                        return Err(err("only :nth(n) is supported"));
                    };
                    let close = inner.find(')').ok_or_else(|| err("unclosed ':nth('"))?;
                    let n: usize = inner[..close].trim().parse().map_err(|_| err("bad :nth index"))?;
                    sel.nth = Some(n);
                    i += 5 + close + 1;
                }
                _ => return Err(QueryError::Parse(format!("unexpected '{}'", &s[i..]))),
            }
        }
        Ok(sel)
    }

    /// Does `node` satisfy every part of the selector (`:nth` aside)?
    pub fn matches(&self, node: &A11yNode) -> bool {
        if self.role.is_some_and(|r| r != node.role) {
            return false;
        }
        self.preds.iter().all(|p| {
            let actual: String = match p.attr.as_str() {
                "id" => node.id.as_str().to_string(),
                "name" => node.name.clone(),
                "value" => node.value.clone().unwrap_or_default(),
                "role" => role_name(node.role).to_string(),
                "kind" => node.kind.map(|k| format!("{k:?}")).unwrap_or_default(),
                "focused" => node.focused.to_string(),
                "disabled" => node.disabled.to_string(),
                "masked" => node.masked.to_string(),
                _ => return false,
            };
            match p.op {
                MatchOp::Exact => actual == p.value,
                MatchOp::Prefix => actual.starts_with(&p.value),
                MatchOp::Contains => actual.to_lowercase().contains(&p.value.to_lowercase()),
            }
        })
    }

    /// Every matching node, in tree order.
    pub fn find_all<'t>(&self, tree: &'t A11yTree) -> Vec<&'t A11yNode> {
        tree.nodes().iter().filter(|n| self.matches(n)).collect()
    }

    /// The one node the selector names: the `:nth` match, or the unique one.
    pub fn find_one<'t>(&self, tree: &'t A11yTree) -> Result<&'t A11yNode, QueryError> {
        let all = self.find_all(tree);
        match self.nth {
            Some(n) => all.get(n).copied().ok_or(QueryError::NotFound),
            None => match all.len() {
                0 => Err(QueryError::NotFound),
                1 => Ok(all[0]),
                n => Err(QueryError::Ambiguous(n)),
            },
        }
    }
}

/// Parse and resolve in one step.
pub fn query<'t>(tree: &'t A11yTree, selector: &str) -> Result<&'t A11yNode, QueryError> {
    Selector::parse(selector)?.find_one(tree)
}

/// Parse and list every match.
pub fn query_all<'t>(tree: &'t A11yTree, selector: &str) -> Result<Vec<&'t A11yNode>, QueryError> {
    Ok(Selector::parse(selector)?.find_all(tree))
}

fn find_close(s: &str, from: usize) -> Option<usize> {
    let mut quoted = false;
    for (k, c) in s[from..].char_indices() {
        match c {
            '"' => quoted = !quoted,
            ']' if !quoted => return Some(from + k),
            _ => {}
        }
    }
    None
}

fn parse_pred(body: &str) -> Result<Predicate, QueryError> {
    let (at, op, oplen) = if let Some(k) = body.find("~=") {
        (k, MatchOp::Contains, 2)
    } else if let Some(k) = body.find("^=") {
        (k, MatchOp::Prefix, 2)
    } else if let Some(k) = body.find('=') {
        (k, MatchOp::Exact, 1)
    } else {
        return Err(QueryError::Parse(format!("predicate '[{body}]' has no operator")));
    };
    let attr = body[..at].trim().to_ascii_lowercase();
    const ATTRS: [&str; 8] = ["id", "name", "value", "kind", "role", "focused", "disabled", "masked"];
    if !ATTRS.contains(&attr.as_str()) {
        return Err(QueryError::Parse(format!("unknown attribute '{attr}'")));
    }
    let raw = body[at + oplen..].trim();
    let value = if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
        raw[1..raw.len() - 1].replace("\\\"", "\"")
    } else {
        raw.to_string()
    };
    Ok(Predicate { attr, op, value })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Rect;

    fn tree() -> A11yTree {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        let mut t = A11yTree::new();
        t.push(A11yNode::new("lh-nick", A11yRole::TextInput, "Nick", r).with_value("lavr"));
        t.push(A11yNode::new("lh-code", A11yRole::TextInput, "Token", r).with_value("•••").with_masked(true));
        t.push(A11yNode::new("lh-pass", A11yRole::Button, "Sign in with passkey", r));
        t.push(A11yNode::new("lh-bind", A11yRole::Button, "Sign in", r));
        t
    }

    #[test]
    fn id_selector_finds_the_node() {
        assert_eq!(query(&tree(), "#lh-code").unwrap().name, "Token");
    }

    #[test]
    fn role_and_exact_name() {
        assert_eq!(query(&tree(), r#"button[name="Sign in"]"#).unwrap().id.as_str(), "lh-bind");
    }

    #[test]
    fn contains_is_ambiguous_without_nth() {
        let t = tree();
        assert_eq!(query(&t, "button[name~=sign]").unwrap_err(), QueryError::Ambiguous(2));
        assert_eq!(query(&t, "button[name~=SIGN]:nth(1)").unwrap().id.as_str(), "lh-bind");
        assert_eq!(query(&t, "button[name~=sign]:nth(5)").unwrap_err(), QueryError::NotFound);
    }

    #[test]
    fn prefix_and_flags() {
        let t = tree();
        assert_eq!(query_all(&t, "[id^=lh-]").unwrap().len(), 4);
        assert_eq!(query(&t, "textinput[masked=true]").unwrap().id.as_str(), "lh-code");
    }

    #[test]
    fn parse_errors() {
        assert!(matches!(Selector::parse(""), Err(QueryError::Parse(_))));
        assert!(matches!(Selector::parse("blob"), Err(QueryError::Parse(_))));
        assert!(matches!(Selector::parse("button[name"), Err(QueryError::Parse(_))));
        assert!(matches!(Selector::parse("[colour=red]"), Err(QueryError::Parse(_))));
        assert!(matches!(Selector::parse("button:first"), Err(QueryError::Parse(_))));
    }

    #[test]
    fn missing_is_not_found() {
        assert_eq!(query(&tree(), "#nope").unwrap_err(), QueryError::NotFound);
    }
}
