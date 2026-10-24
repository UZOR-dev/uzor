//! Scroll container over a [`PanelCx`]: wheel routing, trackpad 1:1 + fling,
//! a draggable / pageable scrollbar, paint.
//!
//! ```ignore
//! let off = scroll_begin(cx, "thread", vp, &mut self.sv, content_h, &st);
//! // … paint content at y - off, clipped to vp, register its widgets …
//! scroll_end(cx, "thread", vp, &mut self.sv, &st);
//! ```

use uzor::input::{Sense, WidgetKind};
use uzor::{Rect, WidgetId};

use crate::handle::{OverlayCx, PanelCx};
use crate::types::scroll::{ScrollKey, ScrollStyle, ScrollView};
use crate::types::spec::Spec;

fn inside(r: Rect, p: (f64, f64)) -> bool {
    p.0 >= r.x && p.0 < r.x + r.width && p.1 >= r.y && p.1 < r.y + r.height
}

/// Scroll containers of the window as of the previous frame, innermost (last painted) last:
/// `(id, viewport, can scroll up, can scroll down)`. A wheel goes to the innermost container
/// under the pointer that can still move that way and chains outwards at an edge — the DOM's
/// nested scroll chaining (`overscroll-behavior: auto`).
///
/// Keyboard scrolling ([`queue_scroll_key`]) picks its container from the same chain: the
/// one last clicked in (the browser's "focused" scroller) when it can move that way, else
/// the innermost hovered one, chaining outwards at an edge. While an overlay has scroll
/// containers, only those take keys (a modal makes the page under it inert).
#[derive(Default)]
struct Chain {
    t: f64,
    /// `(id, viewport, can scroll up, can scroll down, painted by an overlay, chains on)`.
    prev: Vec<Entry>,
    cur: Vec<Entry>,
    /// The container last clicked in.
    active: Option<String>,
    /// A keyboard step waiting for its container this frame.
    key: Option<ScrollKey>,
}

type Entry = (String, Rect, bool, bool, bool, bool);

fn chain_key() -> WidgetId {
    WidgetId::from("uzor.scroll.chain")
}

fn chain(states: &mut uzor::app_context::state::StateRegistry, now: f64) -> &mut Chain {
    let c = states.get_or_insert_with(chain_key(), Chain::default);
    if c.t != now {
        c.t = now;
        c.prev = std::mem::take(&mut c.cur);
        c.key = None;
    }
    c
}

/// Queue a keyboard scroll step for this frame (call before the frame's `scroll_begin`s,
/// e.g. at the top of the panel hook, with a key the app did not use otherwise). The
/// container that takes it glides there; see [`Chain`] for which one.
pub fn queue_scroll_key<S: Spec>(cx: &mut PanelCx<'_, S>, key: ScrollKey) {
    let now = cx.time.now.0 * 1000.0;
    chain(cx.widgets.states, now).key = Some(key);
    cx.out.request_frame();
}

/// The container a keyboard step goes to, from the previous frame's chain.
fn key_owner(c: &Chain, p: Option<(f64, f64)>, key: ScrollKey) -> Option<String> {
    let up = matches!(key, ScrollKey::LineUp | ScrollKey::PageUp | ScrollKey::Home);
    let overlay = c.prev.iter().any(|e| e.4);
    let live: Vec<&Entry> = c.prev.iter().filter(|e| !overlay || e.4).collect();
    let contains = |o: &Rect, i: &Rect| i.x >= o.x && i.y >= o.y && i.x + i.width <= o.x + o.width && i.y + i.height <= o.y + o.height;
    // Innermost first: the active container and the ones enclosing it, else those under the pointer.
    let active = c.active.as_ref().and_then(|a| live.iter().rev().find(|e| &e.0 == a));
    let mut chain: Vec<&&Entry> = match (active, p) {
        (Some(a), _) => live.iter().rev().filter(|e| contains(&e.1, &a.1)).collect(),
        (None, Some(p)) => live.iter().rev().filter(|e| inside(e.1, p)).collect(),
        (None, None) => Vec::new(),
    };
    // Last in line (and alone when nothing is clicked or hovered): the top overlay's
    // container, else the largest one (the browser's document scroller).
    let area = |e: &&&Entry| e.1.width * e.1.height;
    let doc = if overlay { live.iter().last() } else { live.iter().max_by(|a, b| area(a).total_cmp(&area(b))) };
    if let Some(d) = doc.filter(|d| !chain.iter().any(|e| e.0 == d.0)) {
        chain.push(d);
    }
    for e in chain {
        if if up { e.2 } else { e.3 } {
            return Some(e.0.clone());
        }
        if !e.5 {
            return None; // overscroll-behavior: contain
        }
    }
    None
}

/// Whether container `id` takes a wheel of `dy` at `p` (innermost able one wins; the
/// innermost one under the pointer when none can move, so edges still settle there).
fn wheel_owner(c: &Chain, id: &str, p: (f64, f64), dy: f64) -> bool {
    let under: Vec<&Entry> = c.prev.iter().filter(|e| inside(e.1, p)).collect();
    if under.is_empty() || !under.iter().any(|e| e.0 == id) {
        return true; // first frame of this container: no chain yet
    }
    // Innermost first: the first that can move takes it; one that contains overscroll keeps it.
    let able = under.iter().rev().find(|e| (if dy > 0.0 { e.2 } else { e.3 }) || !e.5);
    match able {
        Some(e) => e.0 == id,
        None => under.last().is_some_and(|e| e.0 == id),
    }
}

fn ids(id: &str) -> (WidgetId, WidgetId) {
    (WidgetId::from(format!("{id}:thumb").as_str()), WidgetId::from(format!("{id}:track").as_str()))
}

macro_rules! scroll_fns {
    ($cx:ident, $begin:ident, $end:ident, $overlay:expr) => {
/// Input half: sets the extent, applies this frame's wheel when the pointer is
/// over `viewport` (and `wheel` is allowed), advances inertia, follows a thumb
/// drag, pages on a track press. Returns the offset to paint the content at.
pub fn $begin<S: Spec>(cx: &mut $cx<'_, S>, id: &str, viewport: Rect, sv: &mut ScrollView, content_h: f64, wheel: bool, st: &ScrollStyle) -> f64 {
    let now = cx.time.now.0 * 1000.0;
    sv.set_extent(content_h, viewport.height);
    let (thumb_id, track_id) = ids(id);
    let track = sv.track(viewport, st);
    let p = cx.view.raw_pointer();
    if wheel && p.is_some_and(|p| inside(viewport, p)) {
        let (_, dy) = cx.view.scroll();
        let owner = dy == 0.0 || wheel_owner(chain(cx.widgets.states, now), id, p.unwrap_or_default(), dy);
        if owner && sv.wheel(dy, cx.view.scroll_precise(), now, st) {
            cx.out.request_frame();
        }
    }
    if let Some(p) = p.filter(|p| inside(viewport, *p) && !cx.view.clicks().is_empty()) {
        // A click makes the innermost scrollable container under it the keyboard one.
        let c = chain(cx.widgets.states, now);
        c.active = c.prev.iter().rev().find(|e| inside(e.1, p) && (e.2 || e.3)).map(|e| e.0.clone()).or(c.active.take());
    }
    let c = chain(cx.widgets.states, now);
    if let Some(k) = c.key {
        if key_owner(c, p, k).as_deref() == Some(id) {
            c.key = None;
            if sv.key(k, true) {
                cx.out.request_frame();
            }
        }
    }
    if cx.view.is_pressed(&thumb_id) && !sv.dragging() {
        if let Some(p) = p {
            sv.begin_drag(p.1);
        }
    }
    if sv.dragging() {
        if cx.view.button_down() {
            if let Some(p) = p {
                sv.drag_to(p.1, track, st);
            }
        } else {
            sv.end_drag();
        }
        cx.out.request_frame();
    }
    if cx.view.clicks().iter().any(|(w, _)| *w == track_id) {
        if let Some(p) = p {
            sv.page_towards(p.1, track, st);
        }
    }
    if sv.tick(now, st) {
        cx.out.request_frame();
    }
    sv.offset
}

/// Output half: registers the track and thumb above the content and paints
/// the thumb (nothing when the content fits).
pub fn $end<S: Spec>(cx: &mut $cx<'_, S>, id: &str, viewport: Rect, sv: &ScrollView, st: &ScrollStyle) {
    let now = cx.time.now.0 * 1000.0;
    chain(cx.widgets.states, now).cur.push((id.to_string(), viewport, sv.can_scroll(1.0), sv.can_scroll(-1.0), $overlay, sv.overscroll.chains()));
    let track = sv.track(viewport, st);
    let Some(th) = sv.thumb(track, st) else { return };
    let (thumb_id, track_id) = ids(id);
    let layer = cx.widgets.layer.clone();
    // A slightly wider hit than the painted bar.
    let grow = |r: Rect| Rect::new(r.x - 4.0, r.y, r.width + 7.0, r.height);
    // Scrollbars keep the arrow (as the browser's); the thumb captures the pointer on drag.
    cx.widgets.coord.register_atomic(track_id, WidgetKind::ScrollbarTrack, grow(track), Sense::CLICK, &layer);
    cx.widgets.coord.register_atomic(thumb_id.clone(), WidgetKind::ScrollbarHandle, grow(th), Sense::CLICK_AND_DRAG, &layer);
    let hot = sv.dragging() || cx.view.is_hovered(&thumb_id) || cx.view.raw_pointer().is_some_and(|p| inside(grow(track), p));
    let ctx = &mut *cx.render;
    ctx.set_fill_color(if hot { st.thumb_hot } else { st.thumb });
    ctx.begin_path();
    ctx.rounded_rect(th.x, th.y, th.width, th.height, th.width / 2.0);
    ctx.fill();
}

    };
}

scroll_fns!(PanelCx, scroll_begin, scroll_end, false);
scroll_fns!(OverlayCx, scroll_begin_overlay, scroll_end_overlay, true);

#[cfg(test)]
mod tests {
    use super::*;

    fn e(id: &str, r: Rect, up: bool, down: bool, ov: bool) -> Entry {
        (id.to_string(), r, up, down, ov, true)
    }

    #[test]
    fn key_goes_to_active_then_hovered_and_chains_out() {
        let page = Rect::new(0.0, 0.0, 1000.0, 800.0);
        let side = Rect::new(0.0, 0.0, 200.0, 800.0);
        let inner = Rect::new(300.0, 100.0, 400.0, 300.0);
        let mut c = Chain { prev: vec![e("page", page, true, true, false), e("side", side, false, true, false), e("inner", inner, true, false, false)], ..Chain::default() };
        // Hovered, innermost first.
        assert_eq!(key_owner(&c, Some((50.0, 50.0)), ScrollKey::PageDown).as_deref(), Some("side"));
        assert_eq!(key_owner(&c, Some((400.0, 200.0)), ScrollKey::LineUp).as_deref(), Some("inner"));
        // `inner` is at its end: Down chains to the page around it.
        assert_eq!(key_owner(&c, Some((400.0, 200.0)), ScrollKey::LineDown).as_deref(), Some("page"));
        // `side` is at its top: Home chains out to the page around it.
        assert_eq!(key_owner(&c, Some((50.0, 50.0)), ScrollKey::Home).as_deref(), Some("page"));
        // A container that cannot move hands the key to the document.
        let mut d = Chain { prev: vec![e("side", side, false, false, false), e("thread", Rect::new(200.0, 0.0, 800.0, 800.0), true, false, false)], ..Chain::default() };
        d.active = Some("side".into());
        assert_eq!(key_owner(&d, Some((600.0, 400.0)), ScrollKey::PageUp).as_deref(), Some("thread"));
        // The clicked container wins over the hovered one.
        c.active = Some("inner".into());
        assert_eq!(key_owner(&c, Some((50.0, 50.0)), ScrollKey::End).as_deref(), Some("page"));
        assert_eq!(key_owner(&c, None, ScrollKey::PageUp).as_deref(), Some("inner"));
        // Nothing hovered: the largest container (the document).
        c.active = None;
        assert_eq!(key_owner(&c, Some((900.0, 900.0)), ScrollKey::PageDown).as_deref(), Some("page"));
        // An overlay's containers take keys alone.
        c.prev.push(e("dlg", Rect::new(300.0, 300.0, 300.0, 200.0), true, true, true));
        assert_eq!(key_owner(&c, Some((50.0, 50.0)), ScrollKey::PageDown).as_deref(), Some("dlg"), "over the scrim: the dialog");
        assert_eq!(key_owner(&c, Some((400.0, 400.0)), ScrollKey::PageDown).as_deref(), Some("dlg"));
        c.active = Some("dlg".into());
        assert_eq!(key_owner(&c, None, ScrollKey::PageDown).as_deref(), Some("dlg"));
    }

    #[test]
    fn overscroll_contain_stops_the_chain() {
        let page = Rect::new(0.0, 0.0, 1000.0, 800.0);
        let inner = Rect::new(300.0, 100.0, 400.0, 300.0);
        let mut c = Chain { prev: vec![e("page", page, true, true, false), e("inner", inner, true, false, false)], ..Chain::default() };
        assert_eq!(key_owner(&c, Some((400.0, 200.0)), ScrollKey::LineDown).as_deref(), Some("page"));
        assert!(wheel_owner(&c, "page", (400.0, 200.0), -1.0));
        c.prev[1].5 = false;
        assert_eq!(key_owner(&c, Some((400.0, 200.0)), ScrollKey::LineDown), None);
        assert!(!wheel_owner(&c, "page", (400.0, 200.0), -1.0) && wheel_owner(&c, "inner", (400.0, 200.0), -1.0));
        // What it can take, it still takes.
        assert_eq!(key_owner(&c, Some((400.0, 200.0)), ScrollKey::LineUp).as_deref(), Some("inner"));
    }
}
