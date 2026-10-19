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
use crate::types::scroll::{ScrollStyle, ScrollView};
use crate::types::spec::Spec;

fn inside(r: Rect, p: (f64, f64)) -> bool {
    p.0 >= r.x && p.0 < r.x + r.width && p.1 >= r.y && p.1 < r.y + r.height
}

fn ids(id: &str) -> (WidgetId, WidgetId) {
    (WidgetId::from(format!("{id}:thumb").as_str()), WidgetId::from(format!("{id}:track").as_str()))
}

macro_rules! scroll_fns {
    ($cx:ident, $begin:ident, $end:ident) => {
/// Input half: sets the extent, applies this frame's wheel when the pointer is
/// over `viewport` (and `wheel` is allowed), advances inertia, follows a thumb
/// drag, pages on a track press. Returns the offset to paint the content at.
pub fn $begin<S: Spec>(cx: &mut $cx<'_, S>, id: &str, viewport: Rect, sv: &mut ScrollView, content_h: f64, wheel: bool, st: &ScrollStyle) -> f64 {
    let now = cx.time.now.0 * 1000.0;
    sv.set_extent(content_h, viewport.height);
    let (thumb_id, track_id) = ids(id);
    let track = sv.track(viewport, st);
    let p = cx.view.pointer();
    if wheel && p.is_some_and(|p| inside(viewport, p)) {
        let (_, dy) = cx.view.scroll();
        if sv.wheel(dy, cx.view.scroll_precise(), now, st) {
            cx.out.request_frame();
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
    let track = sv.track(viewport, st);
    let Some(th) = sv.thumb(track, st) else { return };
    let (thumb_id, track_id) = ids(id);
    let layer = cx.widgets.layer.clone();
    // A slightly wider hit than the painted bar.
    let grow = |r: Rect| Rect::new(r.x - 4.0, r.y, r.width + 7.0, r.height);
    cx.widgets.coord.register_atomic(track_id, WidgetKind::Button, grow(track), Sense::CLICK, &layer);
    cx.widgets.coord.register_atomic(thumb_id.clone(), WidgetKind::Button, grow(th), Sense::CLICK_AND_DRAG, &layer);
    let hot = sv.dragging() || cx.view.is_hovered(&thumb_id) || cx.view.pointer().is_some_and(|p| inside(grow(track), p));
    let ctx = &mut *cx.render;
    ctx.set_fill_color(if hot { st.thumb_hot } else { st.thumb });
    ctx.begin_path();
    ctx.rounded_rect(th.x, th.y, th.width, th.height, th.width / 2.0);
    ctx.fill();
}

    };
}

scroll_fns!(PanelCx, scroll_begin, scroll_end);
scroll_fns!(OverlayCx, scroll_begin_overlay, scroll_end_overlay);
