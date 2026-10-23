//! DOM-style event propagation over the widget tree.
//!
//! Widgets registered with `register_composite` / `register_child` form a
//! parent chain. This module turns that chain into the browser event model:
//! a capture phase root → target, the target phase, a bubble phase target →
//! root, with `stop_propagation` and `prevent_default`; and hover-path
//! enter/leave diffs (`mouseenter`/`mouseleave`, which do not bubble).
//! It is pure: it takes the parent relation as a closure, so it serves the
//! coordinator, the framework view and tests alike.

use crate::types::WidgetId;

/// Which leg of the dispatch a handler runs in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Capture,
    Target,
    Bubble,
}

/// The per-handler event context (the DOM `Event` a listener sees).
#[derive(Debug)]
pub struct EventCx<'a> {
    pub target: &'a WidgetId,
    pub current: &'a WidgetId,
    pub phase: Phase,
    stopped: bool,
    prevented: bool,
}

impl EventCx<'_> {
    /// Stop after the current widget (`event.stopPropagation()`).
    pub fn stop_propagation(&mut self) {
        self.stopped = true;
    }
    /// Mark the default action cancelled (`event.preventDefault()`).
    pub fn prevent_default(&mut self) {
        self.prevented = true;
    }
    pub fn default_prevented(&self) -> bool {
        self.prevented
    }
    /// The handler runs on the event's own target.
    pub fn at_target(&self) -> bool {
        self.phase == Phase::Target
    }
}

/// What a dispatch ended with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dispatched {
    pub stopped: bool,
    pub default_prevented: bool,
}

/// The propagation path of `target`, root first, target last. A parent
/// cycle is cut at the first repeat.
pub fn propagation_path(target: &WidgetId, parent: impl Fn(&WidgetId) -> Option<WidgetId>) -> Vec<WidgetId> {
    let mut path = vec![target.clone()];
    while let Some(p) = parent(path.last().unwrap()) {
        if path.contains(&p) {
            break;
        }
        path.push(p);
    }
    path.reverse();
    path
}

/// Dispatch over a root-first `path` (its last entry is the target):
/// capture root → parent, target, bubble parent → root. `bubbles = false`
/// skips the bubble leg (focus, enter/leave).
pub fn dispatch(path: &[WidgetId], bubbles: bool, mut handler: impl FnMut(&mut EventCx<'_>)) -> Dispatched {
    let Some((target, ancestors)) = path.split_last() else {
        return Dispatched::default();
    };
    let mut prevented = false;
    let legs = ancestors
        .iter()
        .map(|w| (w, Phase::Capture))
        .chain(std::iter::once((target, Phase::Target)))
        .chain(ancestors.iter().rev().filter(|_| bubbles).map(|w| (w, Phase::Bubble)));
    for (current, phase) in legs {
        let mut cx = EventCx { target, current, phase, stopped: false, prevented };
        handler(&mut cx);
        prevented = cx.prevented;
        if cx.stopped {
            return Dispatched { stopped: true, default_prevented: prevented };
        }
    }
    Dispatched { stopped: false, default_prevented: prevented }
}

/// The enter/leave diff between two root-first hover paths: the widgets
/// left (deepest first) and entered (outermost first), as the DOM orders
/// `mouseleave` / `mouseenter`. The common ancestors get neither.
pub fn enter_leave(old: &[WidgetId], new: &[WidgetId]) -> (Vec<WidgetId>, Vec<WidgetId>) {
    let common = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let left = old[common..].iter().rev().cloned().collect();
    let entered = new[common..].to_vec();
    (left, entered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn w(s: &str) -> WidgetId {
        WidgetId::new(s)
    }

    fn tree() -> HashMap<WidgetId, WidgetId> {
        [("chip", "list"), ("chip-x", "chip"), ("list", "root")].into_iter().map(|(c, p)| (w(c), w(p))).collect()
    }

    #[test]
    fn path_runs_root_to_target() {
        let t = tree();
        let p = propagation_path(&w("chip-x"), |id| t.get(id).cloned());
        assert_eq!(p, vec![w("root"), w("list"), w("chip"), w("chip-x")]);
        let mut cyc = HashMap::new();
        cyc.insert(w("a"), w("b"));
        cyc.insert(w("b"), w("a"));
        assert_eq!(propagation_path(&w("a"), |id| cyc.get(id).cloned()), vec![w("b"), w("a")]);
    }

    #[test]
    fn capture_target_bubble_order() {
        let p = vec![w("root"), w("chip"), w("chip-x")];
        let mut seen = Vec::new();
        let r = dispatch(&p, true, |cx| seen.push((cx.current.clone(), cx.phase)));
        assert_eq!(
            seen,
            vec![
                (w("root"), Phase::Capture),
                (w("chip"), Phase::Capture),
                (w("chip-x"), Phase::Target),
                (w("chip"), Phase::Bubble),
                (w("root"), Phase::Bubble),
            ]
        );
        assert_eq!(r, Dispatched::default());
        let mut n = 0;
        dispatch(&p, false, |_| n += 1);
        assert_eq!(n, 3, "non-bubbling events skip the bubble leg");
    }

    #[test]
    fn stop_propagation_and_prevent_default() {
        let p = vec![w("root"), w("chip"), w("chip-x")];
        let mut seen = Vec::new();
        let r = dispatch(&p, true, |cx| {
            seen.push(cx.current.clone());
            if cx.at_target() {
                cx.prevent_default();
                cx.stop_propagation();
            }
        });
        assert_eq!(seen, vec![w("root"), w("chip"), w("chip-x")]);
        assert_eq!(r, Dispatched { stopped: true, default_prevented: true });
        // A capturing ancestor can stop the event before the target sees it.
        let mut hit_target = false;
        dispatch(&p, true, |cx| {
            hit_target |= cx.at_target();
            if cx.current == &w("root") {
                cx.stop_propagation();
            }
        });
        assert!(!hit_target);
        // preventDefault persists across later handlers.
        let mut later = false;
        dispatch(&p, true, |cx| {
            if cx.phase == Phase::Capture {
                cx.prevent_default();
            }
            if cx.phase == Phase::Bubble {
                later = cx.default_prevented();
            }
        });
        assert!(later);
    }

    #[test]
    fn enter_leave_skips_common_ancestors() {
        let old = vec![w("root"), w("list"), w("chip"), w("chip-x")];
        let new = vec![w("root"), w("list"), w("row")];
        let (left, entered) = enter_leave(&old, &new);
        assert_eq!(left, vec![w("chip-x"), w("chip")]);
        assert_eq!(entered, vec![w("row")]);
        let (l, e) = enter_leave(&old, &old);
        assert!(l.is_empty() && e.is_empty());
        let (l, e) = enter_leave(&[], &new);
        assert!(l.is_empty() && e == new);
    }
}
