//! Per-frame accessibility tree.

use crate::types::WidgetId;
use super::A11yNode;

/// Per-frame accessibility tree, rebuilt fresh every frame (immediate-mode,
/// mirrors `InputCoordinator::begin_frame`/`end_frame`'s own per-frame
/// widget list) rather than retained/diffed. Retained diffing against
/// AccessKit's `TreeUpdate` is an H7 concern once the adapter exists.
#[derive(Default, Debug, Clone)]
pub struct A11yTree {
    nodes: Vec<A11yNode>,
}

impl A11yTree {
    /// Create an empty tree.
    pub fn new() -> Self {
        Self::default()
    }

    /// Remove every node — call at the start of a frame before pushing this
    /// frame's nodes.
    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    /// Add a node for this frame.
    pub fn push(&mut self, node: A11yNode) {
        self.nodes.push(node);
    }

    /// All nodes currently in the tree.
    pub fn nodes(&self) -> &[A11yNode] {
        &self.nodes
    }

    /// Mark `id` as the one focused node (hosts that track focus outside the
    /// coordinator correct the tree with this).
    pub fn set_focus(&mut self, id: Option<&WidgetId>) {
        for n in &mut self.nodes {
            n.focused = Some(&n.id) == id;
        }
    }

    /// Find a node by its widget id.
    pub fn get(&self, id: &WidgetId) -> Option<&A11yNode> {
        self.nodes.iter().find(|n| &n.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::a11y::A11yRole;
    use crate::types::Rect;

    fn node(id: &str) -> A11yNode {
        A11yNode::new(id, A11yRole::Button, id, Rect::new(0.0, 0.0, 10.0, 10.0))
    }

    #[test]
    fn new_tree_is_empty() {
        let tree = A11yTree::new();
        assert!(tree.nodes().is_empty());
    }

    #[test]
    fn push_adds_nodes_in_order() {
        let mut tree = A11yTree::new();
        tree.push(node("a"));
        tree.push(node("b"));
        assert_eq!(tree.nodes().len(), 2);
        assert_eq!(tree.nodes()[0].id, WidgetId::from("a"));
        assert_eq!(tree.nodes()[1].id, WidgetId::from("b"));
    }

    #[test]
    fn get_finds_node_by_id() {
        let mut tree = A11yTree::new();
        tree.push(node("a"));
        tree.push(node("b"));

        let found = tree.get(&WidgetId::from("b"));
        assert!(found.is_some());
        assert_eq!(found.map(|n| n.name.as_str()), Some("b"));

        assert!(tree.get(&WidgetId::from("missing")).is_none());
    }

    #[test]
    fn coordinator_builds_roles_names_and_states_from_registrations() {
        use crate::input::core::sense::Sense;
        use crate::input::{InputCoordinator, LayerId, WidgetKind};
        use crate::input::text::TextFieldConfig;
        let mut c = InputCoordinator::new();
        let main = LayerId::main();
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        c.register_atomic("card", WidgetKind::Surface, r, Sense::CLICK, &main);
        c.register_atomic("save", WidgetKind::Button, r, Sense::CLICK, &main);
        c.register_atomic("sw:dark", WidgetKind::Toggle, r, Sense::CLICK, &main);
        c.register_text_field("name", r, TextFieldConfig::text());
        c.set_a11y(&WidgetId::from("save"), "Save", None);
        c.set_a11y(&WidgetId::from("sw:dark"), "Dark mode", Some(true));
        let t = c.a11y_tree();
        let got: Vec<_> = t.nodes().iter().map(|n| (n.id.as_str().to_string(), n.role, n.name.clone(), n.checked)).collect();
        assert_eq!(
            got,
            vec![
                ("save".into(), A11yRole::Button, "Save".into(), None),
                ("sw:dark".into(), A11yRole::Switch, "Dark mode".into(), Some(true)),
                ("name".into(), A11yRole::TextInput, String::new(), None),
            ]
        );
    }

    #[test]
    fn clear_empties_the_tree() {
        let mut tree = A11yTree::new();
        tree.push(node("a"));
        assert!(!tree.nodes().is_empty());

        tree.clear();
        assert!(tree.nodes().is_empty());
        assert!(tree.get(&WidgetId::from("a")).is_none());
    }
}
