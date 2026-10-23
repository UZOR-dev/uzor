//! Hidden ARIA mirror of the accessibility tree (feature `a11y-dom`).
//!
//! The canvas is opaque to assistive technology, so after each painted frame
//! the coordinator's [`A11yTree`] is written into one visually hidden `<div>`
//! as role/name/state elements. The DOM is only touched when the markup
//! changes (structure, names, states, focus), not on every frame. The canvas
//! carries `role="application"` and `aria-activedescendant` points at the
//! focused node; a focused text field names the IME textarea instead, since
//! that element holds DOM focus while typing.

use uzor::a11y::{A11yRole, A11yTree};
use web_sys::{Document, Element, HtmlCanvasElement};

const ID: &str = "uzor-a11y";

pub(super) struct Aria {
    root: Element,
    canvas: HtmlCanvasElement,
    ime: Option<Element>,
    last: String,
    last_active: Option<usize>,
}

fn aria_role(r: A11yRole) -> &'static str {
    match r {
        A11yRole::Button => "button",
        A11yRole::CheckBox => "checkbox",
        A11yRole::RadioButton => "radio",
        A11yRole::Switch => "switch",
        A11yRole::Slider => "slider",
        A11yRole::TextInput => "textbox",
        A11yRole::Tab => "tab",
        A11yRole::TabList => "tablist",
        A11yRole::MenuItem => "menuitem",
        A11yRole::ListItem => "listitem",
        A11yRole::Group => "group",
        A11yRole::Dialog => "dialog",
        A11yRole::Tooltip => "tooltip",
        A11yRole::Link => "link",
        A11yRole::Unknown => "generic",
    }
}

fn escape_into(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

/// The mirror's markup for `tree` and the DOM id of the focused node.
pub(crate) fn markup(tree: &A11yTree) -> (String, Option<usize>) {
    let mut h = String::new();
    let mut active = None;
    for (i, n) in tree.nodes().iter().enumerate() {
        h.push_str("<div id=\"a11y-");
        h.push_str(&i.to_string());
        h.push_str("\" role=\"");
        h.push_str(aria_role(n.role));
        h.push_str("\" aria-label=\"");
        escape_into(&mut h, &n.name);
        h.push('"');
        if let Some(c) = n.checked.or(n.toggled) {
            h.push_str(if c { " aria-checked=\"true\"" } else { " aria-checked=\"false\"" });
        }
        if n.disabled {
            h.push_str(" aria-disabled=\"true\"");
        }
        if n.role == A11yRole::Dialog {
            h.push_str(" aria-modal=\"true\"");
        }
        h.push_str("></div>");
        if n.focused {
            active = Some(i);
        }
    }
    (h, active)
}

impl Aria {
    /// Insert the hidden container after the canvas. `None` when the DOM refuses.
    pub(super) fn install(document: &Document, canvas: &HtmlCanvasElement) -> Option<Self> {
        canvas
            .insert_adjacent_html(
                "afterend",
                "<div id=\"uzor-a11y\" style=\"position:fixed;left:0;top:0;width:1px;height:1px;overflow:hidden;clip-path:inset(50%);pointer-events:none\"></div>",
            )
            .ok()?;
        let _ = canvas.set_attribute("role", "application");
        Some(Self { root: document.get_element_by_id(ID)?, canvas: canvas.clone(), ime: document.get_element_by_id("uzor-ime"), last: String::new(), last_active: None })
    }

    /// Bring the mirror up to date with this frame's tree.
    pub(super) fn sync(&mut self, tree: &A11yTree) {
        let (h, active) = markup(tree);
        if h == self.last && active == self.last_active {
            return;
        }
        if h != self.last {
            self.root.set_inner_html(&h);
            self.last = h;
        }
        self.last_active = active;
        let node = active.and_then(|i| tree.nodes().get(i).map(|n| (i, n)));
        match node {
            Some((i, n)) => {
                let _ = self.canvas.set_attribute("aria-activedescendant", &format!("a11y-{i}"));
                if let (Some(ime), A11yRole::TextInput) = (&self.ime, n.role) {
                    let _ = ime.remove_attribute("aria-hidden");
                    let _ = ime.set_attribute("aria-label", &n.name);
                }
            }
            None => {
                let _ = self.canvas.remove_attribute("aria-activedescendant");
                if let Some(ime) = &self.ime {
                    let _ = ime.set_attribute("aria-hidden", "true");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uzor::a11y::A11yNode;
    use uzor::Rect;

    #[test]
    fn markup_carries_role_name_state_and_focus() {
        let r = Rect::new(0.0, 0.0, 1.0, 1.0);
        let mut t = A11yTree::new();
        t.push(A11yNode::new("a", A11yRole::Button, "Save \"<x>\" & go", r));
        t.push(A11yNode::new("b", A11yRole::Switch, "Dark", r).with_checked(true).with_focused(true));
        let (h, active) = markup(&t);
        assert_eq!(
            h,
            "<div id=\"a11y-0\" role=\"button\" aria-label=\"Save &quot;&lt;x>&quot; &amp; go\"></div>\
             <div id=\"a11y-1\" role=\"switch\" aria-label=\"Dark\" aria-checked=\"true\"></div>"
        );
        assert_eq!(active, Some(1));
    }
}
