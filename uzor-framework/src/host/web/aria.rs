//! Hidden ARIA mirror of the accessibility tree (feature `a11y-dom`).
//!
//! The canvas is opaque to assistive technology, so after each painted frame
//! the coordinator's [`A11yTree`] is written into one visually hidden `<div>`
//! as role/name/state elements. The DOM is only touched when the markup
//! changes (structure, names, states, focus), not on every frame. The canvas
//! carries `role="application"` and `aria-activedescendant` points at the
//! focused node; a focused text field names the IME textarea instead, since
//! that element holds DOM focus while typing.
//!
//! Each node is placed over its widget (whole CSS px, `position:absolute`
//! inside a zero-size transparent container at the canvas origin), so screen
//! reader hit-testing and highlight boxes line up with the canvas. A move
//! alone (scrolling, resizing) restyles just the nodes that moved; the
//! markup is rebuilt only when structure, names or states change.

use uzor::a11y::{A11yRole, A11yTree};
use web_sys::{Document, Element, HtmlCanvasElement};

const ID: &str = "uzor-a11y";

pub(super) struct Aria {
    root: Element,
    canvas: HtmlCanvasElement,
    ime: Option<Element>,
    document: Document,
    last: String,
    last_active: Option<usize>,
    last_boxes: Vec<[i32; 4]>,
}

/// A node's box in whole CSS px, relative to the canvas.
fn box_of(r: &uzor::Rect) -> [i32; 4] {
    [r.x.round() as i32, r.y.round() as i32, r.width.round().max(1.0) as i32, r.height.round().max(1.0) as i32]
}

fn style_into(out: &mut String, b: [i32; 4]) {
    use std::fmt::Write;
    let _ = write!(out, "position:absolute;left:{}px;top:{}px;width:{}px;height:{}px", b[0], b[1], b[2], b[3]);
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

/// Every node's box, in tree order.
pub(crate) fn boxes(tree: &A11yTree) -> Vec<[i32; 4]> {
    tree.nodes().iter().map(|n| box_of(&n.bounds)).collect()
}

/// The mirror's markup for `tree` (without positions) and the DOM id of the focused node.
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
        // A tab's state is its selection (`aria-selected`); checkboxes, radios and switches are checked.
        if let Some(c) = n.checked.or(n.toggled) {
            let attr = if n.role == A11yRole::Tab { "aria-selected" } else { "aria-checked" };
            h.push_str(&format!(" {attr}=\"{c}\""));
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

/// `markup` with each node's box inlined as its `style` (nodes are `<div id=…`, in order).
pub(crate) fn with_boxes(h: &str, bx: &[[i32; 4]]) -> String {
    let mut out = String::with_capacity(h.len() + bx.len() * 64);
    let mut rest = h;
    for b in bx {
        let Some(at) = rest.find("<div ") else { break };
        out.push_str(&rest[..at + 5]);
        out.push_str("style=\"");
        style_into(&mut out, *b);
        out.push_str("\" ");
        rest = &rest[at + 5..];
    }
    out.push_str(rest);
    out
}

impl Aria {
    /// Insert the hidden container after the canvas. `None` when the DOM refuses.
    pub(super) fn install(document: &Document, canvas: &HtmlCanvasElement) -> Option<Self> {
        canvas
            .insert_adjacent_html(
                "afterend",
                "<div id=\"uzor-a11y\" style=\"position:fixed;left:0;top:0;width:0;height:0;overflow:visible;opacity:0;pointer-events:none\"></div>",
            )
            .ok()?;
        let _ = canvas.set_attribute("role", "application");
        Some(Self {
            root: document.get_element_by_id(ID)?,
            canvas: canvas.clone(),
            ime: document.get_element_by_id("uzor-ime"),
            document: document.clone(),
            last: String::new(),
            last_active: None,
            last_boxes: Vec::new(),
        })
    }

    /// Bring the mirror up to date with this frame's tree.
    pub(super) fn sync(&mut self, tree: &A11yTree) {
        let (h, active) = markup(tree);
        let bx = boxes(tree);
        if h == self.last && active == self.last_active && bx == self.last_boxes {
            return;
        }
        let rebuilt = h != self.last || bx.len() != self.last_boxes.len();
        if rebuilt {
            // Container at the canvas origin (read before the writes below).
            let o = self.canvas.get_bounding_client_rect();
            let _ = self.root.set_attribute(
                "style",
                &format!(
                    "position:fixed;left:{}px;top:{}px;width:0;height:0;overflow:visible;opacity:0;pointer-events:none",
                    o.left().round(),
                    o.top().round()
                ),
            );
            self.root.set_inner_html(&with_boxes(&h, &bx));
            self.last = h;
        } else {
            for (i, (b, old)) in bx.iter().zip(&self.last_boxes).enumerate() {
                if b != old {
                    if let Some(el) = self.document.get_element_by_id(&format!("a11y-{i}")) {
                        let mut st = String::new();
                        style_into(&mut st, *b);
                        let _ = el.set_attribute("style", &st);
                    }
                }
            }
        }
        self.last_boxes = bx;
        if !rebuilt && active == self.last_active {
            return;
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

    #[test]
    fn tab_state_is_selection_not_checked() {
        let r = Rect::new(0.0, 0.0, 1.0, 1.0);
        let mut t = A11yTree::new();
        t.push(A11yNode::new("t0", A11yRole::Tab, "Board", r).with_checked(true));
        t.push(A11yNode::new("t1", A11yRole::Tab, "Limits", r).with_checked(false));
        let (h, _) = markup(&t);
        assert_eq!(
            h,
            "<div id=\"a11y-0\" role=\"tab\" aria-label=\"Board\" aria-selected=\"true\"></div>\
             <div id=\"a11y-1\" role=\"tab\" aria-label=\"Limits\" aria-selected=\"false\"></div>"
        );
    }

    #[test]
    fn boxes_are_whole_px_and_inlined_per_node() {
        let mut t = A11yTree::new();
        t.push(A11yNode::new("a", A11yRole::Button, "A", Rect::new(10.4, 20.6, 36.0, 0.2)));
        t.push(A11yNode::new("b", A11yRole::Link, "B", Rect::new(0.0, 0.0, 5.0, 5.0)));
        let bx = boxes(&t);
        assert_eq!(bx, vec![[10, 21, 36, 1], [0, 0, 5, 5]]);
        let (h, _) = markup(&t);
        assert_eq!(
            with_boxes(&h, &bx),
            "<div style=\"position:absolute;left:10px;top:21px;width:36px;height:1px\" id=\"a11y-0\" role=\"button\" aria-label=\"A\"></div>\
             <div style=\"position:absolute;left:0px;top:0px;width:5px;height:5px\" id=\"a11y-1\" role=\"link\" aria-label=\"B\"></div>"
        );
    }
}
