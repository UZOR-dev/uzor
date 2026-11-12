//! Paint slots driven by registered painters: [`SlotPainter`] and [`SlotRegistry`].
//!
//! An [`El::paint_slot`](super::El::paint_slot) reserves a box in the tree; a
//! [`SlotPainter`] registered under its key paints into the box's content rect, gets
//! the slot's input as [`SlotEvent`]s in slot-local coordinates and gives the slot its
//! accessibility role and name. The registry is what the host hands the tree each
//! frame instead of one closure for every slot ([`SlotRegistry::paint`]); it lives in
//! the app's state across frames, so a painter keeps its own state (a pan, a zoom, a
//! hovered bar). Logic crates (charts, graphs, documents, scenes) ship painters; the
//! tree stays plain markup.
//!
//! ```
//! use uzor::render::RenderContext;
//! use uzor::ui::dom::{SlotPaint, SlotPainter, SlotRegistry};
//!
//! struct Dot;
//! impl SlotPainter for Dot {
//!     fn paint(&mut self, ctx: &mut dyn RenderContext, f: &SlotPaint<'_>) {
//!         ctx.set_fill_color("#888");
//!         ctx.fill_rect(f.rect.x, f.rect.y, 4.0, 4.0);
//!     }
//!     fn role(&self) -> &'static str {
//!         "img"
//!     }
//! }
//! let mut reg = SlotRegistry::new();
//! reg.register("dot", Dot);
//! assert!(reg.contains("dot"));
//! ```

use std::any::Any;
use std::collections::HashMap;

use super::{A11yNode, Dom};
use crate::input::KeyCode;
use crate::render::RenderContext;
use crate::types::Rect;
use crate::ui::style::Style;

/// What a painter is given to paint one frame of its slot.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotPaint<'a> {
    /// The slot key ([`El::paint_slot`](super::El::paint_slot)); for a prefix
    /// registration ([`SlotRegistry::register_prefix`]) the full key.
    pub key: &'a str,
    /// The content box to paint into, in the tree's coordinates. Painting is clipped
    /// to it and to the element's scroll containers.
    pub rect: Rect,
    /// The pointer over the slot, slot-local (`None` when it is elsewhere or under a
    /// popover).
    pub hover: Option<(f64, f64)>,
    /// A mouse button holds the slot (a drag in progress).
    pub held: bool,
    /// The slot has keyboard focus.
    pub focused: bool,
    /// The slot element's resolved style, for theme colours: `color`, and custom
    /// properties through [`Style::custom`] (`--chart-line`, `--k-muted`), inherited
    /// from the sheet like any CSS variable. `None` outside a laid-out tree.
    pub style: Option<&'a Style>,
}

impl<'a> SlotPaint<'a> {
    /// A frame of `key` in `rect` with no pointer, no focus and no style.
    pub fn new(key: &'a str, rect: Rect) -> Self {
        Self { key, rect, hover: None, held: false, focused: false, style: None }
    }
    /// A theme colour: custom property `name` of the slot's style, else `fallback`.
    pub fn var<'s>(&'s self, name: &str, fallback: &'s str) -> &'s str {
        self.style.and_then(|s| s.custom(name)).unwrap_or(fallback)
    }
    /// The slot's CSS `color`, else `fallback`.
    pub fn color<'s>(&'s self, fallback: &'s str) -> &'s str {
        self.style.and_then(|s| s.color.as_deref()).unwrap_or(fallback)
    }
    /// `(x, y)` in the tree's coordinates mapped into the slot.
    pub fn local(&self, x: f64, y: f64) -> (f64, f64) {
        (x - self.rect.x, y - self.rect.y)
    }
}

/// One input event for a slot, in slot-local coordinates (`(0, 0)` is the content
/// box's top-left corner).
#[derive(Clone, Debug, PartialEq)]
pub enum SlotEvent {
    /// The pointer came over the slot.
    Enter { x: f64, y: f64 },
    /// The pointer moved over the slot.
    Move { x: f64, y: f64 },
    /// The pointer left the slot.
    Leave,
    /// A mouse button went down on the slot.
    Down { x: f64, y: f64 },
    /// The held pointer moved by `(dx, dy)` since the last frame.
    Drag { x: f64, y: f64, dx: f64, dy: f64 },
    /// The held mouse button was released (at the last place it was seen).
    Up { x: f64, y: f64 },
    /// The slot was clicked or tapped.
    Click { x: f64, y: f64 },
    /// Wheel or trackpad scroll over the slot.
    Wheel { x: f64, y: f64, dx: f64, dy: f64 },
    /// A one-finger pan step (touch).
    Pan { x: f64, y: f64, dx: f64, dy: f64 },
    /// A pan ended.
    PanEnd { x: f64, y: f64 },
    /// Two fingers: `scale` is distance now / distance at the start; `(x, y)` the
    /// midpoint.
    Pinch { x: f64, y: f64, scale: f64 },
    /// Two fingers: the midpoint moved.
    PinchPan { x: f64, y: f64, dx: f64, dy: f64 },
    /// The pinch ended.
    PinchEnd { scale: f64 },
    /// A long press (touch).
    LongPress { x: f64, y: f64 },
    /// A double tap or double click.
    DoubleTap { x: f64, y: f64 },
    /// A key pressed while the slot has focus.
    Key { code: KeyCode, text: Option<String> },
    /// The slot gained (`true`) or lost keyboard focus.
    Focus(bool),
}

/// A value carried by a [`SlotAction`].
#[derive(Clone, Debug, Default, PartialEq)]
pub enum SlotValue {
    #[default]
    None,
    Num(f64),
    Text(String),
    Point(f64, f64),
}

/// What a painter reports from an event: the app reads it like a click on an id.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotAction {
    /// The slot key it came from (filled by the registry).
    pub slot: String,
    /// The action name (`"select"`, `"zoom"`, `"open"`).
    pub name: String,
    pub value: SlotValue,
}

impl SlotAction {
    /// An action `name` without a value.
    pub fn new(name: impl Into<String>) -> Self {
        Self { slot: String::new(), name: name.into(), value: SlotValue::None }
    }
    /// With a number.
    pub fn num(mut self, v: f64) -> Self {
        self.value = SlotValue::Num(v);
        self
    }
    /// With a text (an item id).
    pub fn text(mut self, v: impl Into<String>) -> Self {
        self.value = SlotValue::Text(v.into());
        self
    }
    /// With a slot-local point.
    pub fn point(mut self, x: f64, y: f64) -> Self {
        self.value = SlotValue::Point(x, y);
        self
    }
}

/// What paints a slot and handles its input: a chart, a graph, a document page, a
/// scene. Colours should come from the theme (pass resolved CSS variables in);
/// respect reduced motion by not asking for frames ([`SlotPainter::wants_frame`]).
pub trait SlotPainter: Any {
    /// Paint the slot into `f.rect` (already clipped).
    fn paint(&mut self, ctx: &mut dyn RenderContext, f: &SlotPaint<'_>);
    /// Handle one input event; return an action for the app, if any.
    fn event(&mut self, ev: &SlotEvent, f: &SlotPaint<'_>) -> Option<SlotAction> {
        let _ = (ev, f);
        None
    }
    /// The ARIA role the slot reports (`img`, `figure`, `application`, `slider`,
    /// `button`, `document`).
    fn role(&self) -> &'static str {
        "img"
    }
    /// The accessible name (a chart's title and summary), over the element's own.
    fn name(&self) -> Option<String> {
        None
    }
    /// Whether the slot is a keyboard stop (it handles [`SlotEvent::Key`]).
    fn focusable(&self) -> bool {
        false
    }
    /// Whether the painter needs another frame (an animation in flight).
    fn wants_frame(&self) -> bool {
        false
    }
}

/// Per-frame input of one slot as a host sees it, in the tree's coordinates.
/// [`SlotRegistry::input`] turns the change since the last frame into [`SlotEvent`]s.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SlotInput {
    /// The pointer over the slot (z-order safe).
    pub pointer: Option<(f64, f64)>,
    /// The pointer while a mouse button holds the slot.
    pub pressed: Option<(f64, f64)>,
    /// Clicked or tapped this frame.
    pub clicked: bool,
    /// Wheel scroll over the slot this frame.
    pub wheel: (f64, f64),
    /// The slot has keyboard focus.
    pub focused: bool,
    /// A key pressed this frame (delivered only while focused).
    pub key: Option<(KeyCode, Option<String>)>,
    /// Touch and other events already in event form (slot-local), appended as is.
    pub events: Vec<SlotEvent>,
}

#[derive(Clone, Debug, PartialEq)]
enum Pat {
    Exact(String),
    Prefix(String),
}

impl Pat {
    fn matches(&self, key: &str) -> bool {
        match self {
            Pat::Exact(k) => k == key,
            Pat::Prefix(p) => key.starts_with(p.as_str()),
        }
    }
}

/// Per concrete key: what the last frame saw.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Seen {
    hover: Option<(f64, f64)>,
    held: Option<(f64, f64)>,
    focused: bool,
}

/// Painters by slot key. Keep it in the app's state; hand it to the tree each frame
/// (`uzor_framework::widgets::kit::dom(..).slot_registry(&mut reg)`, or
/// [`SlotRegistry::paint`] for a bare tree). Exact keys win over prefixes; among
/// prefixes the longest wins. A slot nobody registered paints nothing extra (its CSS
/// box still paints).
#[derive(Default)]
pub struct SlotRegistry {
    entries: Vec<(Pat, Box<dyn SlotPainter>)>,
    seen: HashMap<String, Seen>,
}

impl std::fmt::Debug for SlotRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlotRegistry").field("keys", &self.entries.iter().map(|(p, _)| p).collect::<Vec<_>>()).finish()
    }
}

impl SlotRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    /// Register `painter` for the slot `key`, replacing one already there.
    pub fn register(&mut self, key: impl Into<String>, painter: impl SlotPainter) -> &mut Self {
        self.put(Pat::Exact(key.into()), Box::new(painter));
        self
    }
    /// Register `painter` for every slot whose key starts with `prefix` (a list of
    /// critters `@crit:0`, `@crit:1`, ...); [`SlotPaint::key`] tells them apart.
    pub fn register_prefix(&mut self, prefix: impl Into<String>, painter: impl SlotPainter) -> &mut Self {
        self.put(Pat::Prefix(prefix.into()), Box::new(painter));
        self
    }
    fn put(&mut self, pat: Pat, p: Box<dyn SlotPainter>) {
        match self.entries.iter_mut().find(|(q, _)| *q == pat) {
            Some(e) => e.1 = p,
            None => self.entries.push((pat, p)),
        }
    }
    /// Remove the exact registration `key` (or the prefix registration equal to it).
    pub fn remove(&mut self, key: &str) -> bool {
        let n = self.entries.len();
        self.entries.retain(|(p, _)| !matches!(p, Pat::Exact(k) | Pat::Prefix(k) if k == key));
        self.seen.retain(|k, _| self.entries.iter().any(|(p, _)| p.matches(k)));
        n != self.entries.len()
    }
    /// Number of registrations.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    fn index(&self, key: &str) -> Option<usize> {
        let mut best: Option<(usize, usize)> = None;
        for (i, (p, _)) in self.entries.iter().enumerate() {
            if !p.matches(key) {
                continue;
            }
            let rank = match p {
                Pat::Exact(_) => usize::MAX,
                Pat::Prefix(s) => s.len(),
            };
            if best.is_none_or(|(_, r)| rank > r) {
                best = Some((i, rank));
            }
        }
        best.map(|b| b.0)
    }
    /// A painter is registered for `key`.
    pub fn contains(&self, key: &str) -> bool {
        self.index(key).is_some()
    }
    /// The painter for `key`.
    pub fn painter(&self, key: &str) -> Option<&dyn SlotPainter> {
        self.index(key).map(|i| &*self.entries[i].1)
    }
    /// The painter for `key`, mutable (to update its model).
    pub fn painter_mut(&mut self, key: &str) -> Option<&mut dyn SlotPainter> {
        let i = self.index(key)?;
        Some(&mut *self.entries[i].1)
    }
    /// The painter for `key` as its concrete type.
    pub fn get<T: SlotPainter>(&self, key: &str) -> Option<&T> {
        let p: &dyn Any = self.painter(key)?;
        p.downcast_ref()
    }
    /// The painter for `key` as its concrete type, mutable.
    pub fn get_mut<T: SlotPainter>(&mut self, key: &str) -> Option<&mut T> {
        let p: &mut dyn Any = self.painter_mut(key)?;
        p.downcast_mut()
    }
    /// Some painter needs another frame.
    pub fn wants_frame(&self) -> bool {
        self.entries.iter().any(|(_, p)| p.wants_frame())
    }

    fn frame<'k>(&self, key: &'k str, rect: Rect, style: Option<&'k Style>) -> SlotPaint<'k> {
        let s = self.seen.get(key).copied().unwrap_or_default();
        SlotPaint { key, rect, hover: s.hover, held: s.held.is_some(), focused: s.focused, style }
    }

    /// Paint slot `key` into `rect`, clipped to it, with the slot element's `style`
    /// for its colours. `false` when nobody is registered.
    pub fn paint_slot(&mut self, ctx: &mut dyn RenderContext, key: &str, rect: Rect, style: Option<&Style>) -> bool {
        let Some(i) = self.index(key) else { return false };
        let f = self.frame(key, rect, style);
        ctx.save();
        ctx.begin_path();
        ctx.rect(rect.x, rect.y, rect.width, rect.height);
        ctx.clip();
        self.entries[i].1.paint(ctx, &f);
        ctx.restore();
        true
    }

    /// Paint a laid-out tree ([`super::paint_slots`]) with every registered slot drawn
    /// by its painter in paint order (so later layers stay on top).
    pub fn paint(&mut self, ctx: &mut dyn RenderContext, dom: &Dom) {
        super::paint_slots(ctx, dom, &mut |ctx, key, r| {
            let style = dom.boxes.iter().find(|b| b.slot.as_deref() == Some(key)).map(|b| &b.style);
            self.paint_slot(ctx, key, r, style);
        });
    }

    /// Send one event to `key`'s painter. The returned action carries the slot key.
    pub fn send(&mut self, key: &str, rect: Rect, ev: &SlotEvent) -> Option<SlotAction> {
        let i = self.index(key)?;
        let f = self.frame(key, rect, None);
        let mut a = self.entries[i].1.event(ev, &f)?;
        a.slot = key.to_string();
        Some(a)
    }

    /// Feed this frame's input of slot `key` (content box `rect`): the change since the
    /// last frame becomes events (enter, move, leave, down, drag, up, click, wheel,
    /// key, focus), then `input.events`, in that order. Returns the painter's actions.
    pub fn input(&mut self, key: &str, rect: Rect, input: &SlotInput) -> Vec<SlotAction> {
        if !self.contains(key) {
            return Vec::new();
        }
        let loc = |(x, y): (f64, f64)| (x - rect.x, y - rect.y);
        let was = self.seen.get(key).copied().unwrap_or_default();
        let hover = input.pointer.map(loc);
        let held = input.pressed.map(loc);
        let mut evs = Vec::new();
        if input.focused != was.focused {
            evs.push(SlotEvent::Focus(input.focused));
        }
        match (was.hover, hover) {
            (None, Some((x, y))) => evs.push(SlotEvent::Enter { x, y }),
            (Some(a), Some((x, y))) if a != (x, y) => evs.push(SlotEvent::Move { x, y }),
            (Some(_), None) if held.is_none() => evs.push(SlotEvent::Leave),
            _ => {}
        }
        match (was.held, held) {
            (None, Some((x, y))) => evs.push(SlotEvent::Down { x, y }),
            (Some((ax, ay)), Some((x, y))) if (ax, ay) != (x, y) => evs.push(SlotEvent::Drag { x, y, dx: x - ax, dy: y - ay }),
            (Some((x, y)), None) => evs.push(SlotEvent::Up { x, y }),
            _ => {}
        }
        if input.clicked {
            let (x, y) = hover.or(was.held).unwrap_or((0.0, 0.0));
            evs.push(SlotEvent::Click { x, y });
        }
        if input.wheel != (0.0, 0.0) {
            let (x, y) = hover.unwrap_or((0.0, 0.0));
            evs.push(SlotEvent::Wheel { x, y, dx: input.wheel.0, dy: input.wheel.1 });
        }
        if input.focused {
            if let Some((code, text)) = &input.key {
                evs.push(SlotEvent::Key { code: *code, text: text.clone() });
            }
        }
        evs.extend(input.events.iter().cloned());
        // The frame the painter sees reflects this frame's state.
        self.seen.insert(key.to_string(), Seen { hover, held, focused: input.focused });
        evs.iter().filter_map(|e| self.send(key, rect, e)).collect()
    }

    /// [`Dom::a11y`] with every registered slot's role and name from its painter.
    pub fn a11y(&self, dom: &Dom) -> Vec<A11yNode> {
        let mut nodes = dom.a11y();
        for n in &mut nodes {
            let Some(b) = dom.get(&n.id) else { continue };
            let Some(p) = b.slot.as_deref().and_then(|k| self.painter(k)) else { continue };
            n.role = p.role();
            if let Some(name) = p.name() {
                n.name = name;
            }
            n.focusable |= p.focusable();
        }
        nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::NullRenderContext;
    use crate::ui::dom::{lay_out, El};
    use crate::ui::style::{Sheet, State};

    #[derive(Default)]
    struct Probe {
        events: Vec<SlotEvent>,
        painted: Vec<(String, Rect)>,
    }
    impl SlotPainter for Probe {
        fn paint(&mut self, _: &mut dyn RenderContext, f: &SlotPaint<'_>) {
            self.painted.push((f.key.to_string(), f.rect));
        }
        fn event(&mut self, ev: &SlotEvent, _: &SlotPaint<'_>) -> Option<SlotAction> {
            self.events.push(ev.clone());
            match ev {
                SlotEvent::Click { x, y } => Some(SlotAction::new("pick").point(*x, *y)),
                _ => None,
            }
        }
        fn role(&self) -> &'static str {
            "figure"
        }
        fn name(&self) -> Option<String> {
            Some("Выручка".into())
        }
        fn focusable(&self) -> bool {
            true
        }
    }

    #[test]
    fn exact_beats_prefix_and_longest_prefix_wins() {
        let mut r = SlotRegistry::new();
        r.register_prefix("@c", Probe::default()).register_prefix("@crit:", Probe::default()).register("@crit:7", Probe::default());
        assert_eq!(r.index("@crit:7"), Some(2));
        assert_eq!(r.index("@crit:1"), Some(1));
        assert_eq!(r.index("@cx"), Some(0));
        assert_eq!(r.index("other"), None);
        assert!(r.remove("@crit:7"));
        assert_eq!(r.index("@crit:7"), Some(1));
        assert!(r.get_mut::<Probe>("@crit:7").is_some());
    }

    #[test]
    fn input_becomes_local_events_and_actions() {
        let mut r = SlotRegistry::new();
        r.register("chart", Probe::default());
        let rect = Rect::new(100.0, 50.0, 200.0, 100.0);
        let a = r.input("chart", rect, &SlotInput { pointer: Some((110.0, 60.0)), ..Default::default() });
        assert!(a.is_empty());
        r.input("chart", rect, &SlotInput { pointer: Some((120.0, 60.0)), pressed: Some((120.0, 60.0)), ..Default::default() });
        r.input("chart", rect, &SlotInput { pointer: Some((130.0, 65.0)), pressed: Some((130.0, 65.0)), ..Default::default() });
        let a = r.input("chart", rect, &SlotInput { pointer: Some((130.0, 65.0)), clicked: true, wheel: (0.0, -3.0), ..Default::default() });
        assert_eq!(a, vec![SlotAction { slot: "chart".into(), name: "pick".into(), value: SlotValue::Point(30.0, 15.0) }]);
        r.input("chart", rect, &SlotInput { focused: true, key: Some((KeyCode::ArrowLeft, None)), ..Default::default() });
        let p = r.get::<Probe>("chart").unwrap();
        assert_eq!(
            p.events,
            vec![
                SlotEvent::Enter { x: 10.0, y: 10.0 },
                SlotEvent::Move { x: 20.0, y: 10.0 },
                SlotEvent::Down { x: 20.0, y: 10.0 },
                SlotEvent::Move { x: 30.0, y: 15.0 },
                SlotEvent::Drag { x: 30.0, y: 15.0, dx: 10.0, dy: 5.0 },
                SlotEvent::Up { x: 30.0, y: 15.0 },
                SlotEvent::Click { x: 30.0, y: 15.0 },
                SlotEvent::Wheel { x: 30.0, y: 15.0, dx: 0.0, dy: -3.0 },
                SlotEvent::Focus(true),
                SlotEvent::Leave,
                SlotEvent::Key { code: KeyCode::ArrowLeft, text: None },
            ]
        );
    }

    #[test]
    fn paints_registered_slots_in_order_and_reports_their_role() {
        let tree = El::new("div")
            .css("width: 300px")
            // `height` is the border box here (the DOM sizes border boxes).
            .child(El::new("div").paint_slot("a").css("height: 40px; padding: 5px"))
            .child(El::new("div").paint_slot("free").css("height: 20px"))
            .child(El::new("div").paint_slot("b").css("height: 30px"));
        let mut ctx = NullRenderContext::default();
        let d = lay_out(&mut ctx, &Sheet::default(), &tree, 0.0, 0.0, 300.0, &|_| State::NONE);
        let mut r = SlotRegistry::new();
        r.register("a", Probe::default()).register("b", Probe::default());
        r.paint(&mut ctx, &d);
        assert_eq!(r.get::<Probe>("a").unwrap().painted, vec![("a".to_string(), Rect::new(5.0, 5.0, 290.0, 30.0))]);
        assert_eq!(r.get::<Probe>("b").unwrap().painted, vec![("b".to_string(), Rect::new(0.0, 60.0, 300.0, 30.0))]);
        let a11y = r.a11y(&d);
        let a = a11y.iter().find(|n| n.id == "a").unwrap();
        assert_eq!((a.role, a.name.as_str(), a.focusable), ("figure", "Выручка", true));
        assert_eq!(a11y.iter().find(|n| n.id == "free").unwrap().role, "generic");
    }
}
