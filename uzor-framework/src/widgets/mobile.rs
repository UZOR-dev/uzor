//! Drives the mobile shell widgets of [`uzor::ui::mobile`] from this frame's input: a
//! finger (the touch recogniser's pans, [`VisualView::gestures`]) and the mouse (a press
//! held on a hit target and moved) both become one [`Drag`] stream, which moves a
//! [`SheetState`], [`DrawerState`] or [`SwipeState`]. Releases carry a velocity, so a
//! flick snaps to the next stop with either input.
//!
//! The keyboard and the safe area come from the view too: [`composer_rect`] puts the
//! composer over the soft keyboard (or the home indicator), [`reveal_field`] says how far
//! to scroll a focused field out from under it.

use uzor::types::Rect;
use uzor::ui::mobile::{DrawerState, SheetState, SwipeState, DRAWER_W};

use crate::handle::VisualView;
use crate::types::gesture::{Axis, Edge, Gesture, GestureConfig, VelocityTracker};
use uzor::WidgetId;
use crate::types::spec::Spec;
use crate::types::window::Point;

/// One step of a drag.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Drag {
    /// The drag began (past the slop).
    Start {
        /// Where the press went down.
        origin: (f64, f64),
        /// Where it is now.
        pos: (f64, f64),
    },
    /// It moved.
    Move {
        /// Where it is.
        pos: (f64, f64),
        /// Motion since the last step.
        delta: (f64, f64),
    },
    /// It ended.
    End {
        /// Where it lifted.
        pos: (f64, f64),
        /// Release velocity, px/s.
        velocity: (f64, f64),
    },
}

/// What may start a drag.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Grip {
    /// A press on this hit target (mouse) or a pan that started inside `rect` (touch).
    Target(Rect),
    /// A pan from this window edge (touch), or a mouse press within `width` px of it.
    Edge(Edge, f64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Src {
    Touch,
    Mouse,
}

/// Turns touch pans and mouse drags on one target into [`Drag`] steps. Keep one per
/// draggable thing (sheet handle, drawer edge, a row) across frames.
#[derive(Clone, Debug, Default)]
pub struct DragTracker {
    active: Option<Src>,
    /// Mouse: where the press began, whether it left the slop, last position.
    press: Option<(f64, f64)>,
    last: (f64, f64),
    vel: VelocityTracker,
    moved: bool,
    swallow: bool,
}

impl DragTracker {
    /// Whether a drag is in progress.
    pub fn active(&self) -> bool {
        self.active.is_some() && self.moved
    }

    /// True once after a mouse drag ended on the target: the release click that
    /// follows is the drag's, not a tap (ignore that click).
    pub fn take_swallow(&mut self) -> bool {
        std::mem::take(&mut self.swallow)
    }

    /// The drag steps of this frame. `id` is the hit target's id (for the mouse press),
    /// `axis` limits touch pans to that direction (`None`: any), `now` is the frame
    /// time in seconds.
    pub fn update<S: Spec>(&mut self, view: &VisualView<S>, id: &WidgetId, grip: Grip, axis: Option<Axis>, now: f64) -> Vec<Drag> {
        let cfg = GestureConfig::default();
        let mut out = Vec::new();
        // Touch: the recogniser's pans.
        for g in view.gestures() {
            match *g {
                Gesture::PanStart { origin, pos, axis: a, edge } if self.active.is_none() => {
                    let ok_axis = axis.is_none_or(|x| x == a);
                    let ok = match grip {
                        Grip::Target(r) => contains(r, origin),
                        Grip::Edge(e, _) => edge == Some(e),
                    };
                    if ok && ok_axis {
                        self.active = Some(Src::Touch);
                        self.moved = true;
                        self.last = (pos.x, pos.y);
                        out.push(Drag::Start { origin: (origin.x, origin.y), pos: (pos.x, pos.y) });
                        // The slop already travelled counts.
                        out.push(Drag::Move { pos: (pos.x, pos.y), delta: (pos.x - origin.x, pos.y - origin.y) });
                    }
                }
                Gesture::Pan { pos, delta, .. } if self.active == Some(Src::Touch) => {
                    self.last = (pos.x, pos.y);
                    out.push(Drag::Move { pos: (pos.x, pos.y), delta });
                }
                Gesture::PanEnd { pos, velocity, .. } if self.active == Some(Src::Touch) => {
                    self.active = None;
                    self.moved = false;
                    out.push(Drag::End { pos: (pos.x, pos.y), velocity });
                }
                _ => {}
            }
        }
        if self.active == Some(Src::Touch) || view.coarse_pointer() {
            return out;
        }
        // Mouse: a held press that moves.
        let p = view.raw_pointer();
        let held = view.is_pressed(id) || (self.active == Some(Src::Mouse) && view.button_down());
        let edge_press = |pt: (f64, f64)| match grip {
            Grip::Edge(Edge::Left, w) => pt.0 <= w,
            _ => false,
        };
        match (held, p) {
            (true, Some(pt)) => {
                if self.press.is_none() {
                    self.press = Some(view.press_origin().unwrap_or(pt));
                    self.last = self.press.unwrap_or(pt);
                    self.vel.reset();
                    self.moved = false;
                    self.active = Some(Src::Mouse);
                }
                let o = self.press.unwrap_or(pt);
                self.vel.add(now, Point::new(pt.0, pt.1), &cfg);
                if !self.moved {
                    let (dx, dy) = (pt.0 - o.0, pt.1 - o.1);
                    let along = match axis {
                        Some(Axis::Horizontal) => dx.abs() > cfg.slop && dx.abs() > dy.abs(),
                        Some(Axis::Vertical) => dy.abs() > cfg.slop && dy.abs() > dx.abs(),
                        None => dx.hypot(dy) > cfg.slop,
                    };
                    if along {
                        self.moved = true;
                        out.push(Drag::Start { origin: o, pos: pt });
                        out.push(Drag::Move { pos: pt, delta: (dx, dy) });
                        self.last = pt;
                    }
                } else if pt != self.last {
                    out.push(Drag::Move { pos: pt, delta: (pt.0 - self.last.0, pt.1 - self.last.1) });
                    self.last = pt;
                }
            }
            (true, None) => {}
            (false, _) => {
                if self.active == Some(Src::Mouse) {
                    if self.moved {
                        let v = self.vel.velocity(now, &cfg);
                        out.push(Drag::End { pos: self.last, velocity: v });
                        self.swallow = true;
                    }
                    self.active = None;
                    self.moved = false;
                    self.press = None;
                }
                let _ = edge_press;
            }
        }
        out
    }
}

fn contains(r: Rect, p: Point) -> bool {
    p.x >= r.x && p.x < r.x + r.width && p.y >= r.y && p.y < r.y + r.height
}

/// Feed `steps` to a bottom sheet in a viewport `vh` tall; true while it needs frames.
pub fn drive_sheet(st: &mut SheetState, steps: &[Drag], vh: f64, dt: f64) -> bool {
    for s in steps {
        match *s {
            Drag::Start { .. } => {}
            Drag::Move { delta, .. } => st.drag_by(delta.1, vh),
            Drag::End { velocity, .. } => {
                st.release(velocity.1, vh);
            }
        }
    }
    st.dragging() | st.step(dt, vh)
}

/// Feed `steps` to an edge drawer; true while it needs frames.
pub fn drive_drawer(st: &mut DrawerState, steps: &[Drag], dt: f64) -> bool {
    for s in steps {
        match *s {
            Drag::Start { .. } => {}
            Drag::Move { delta, .. } => st.drag_by(delta.0, DRAWER_W),
            Drag::End { velocity, .. } => {
                st.release(velocity.0);
            }
        }
    }
    st.dragging() | st.step(dt, DRAWER_W)
}

/// Feed `steps` to a swipe row with `left_w` / `right_w` px of actions; true while it
/// needs frames.
pub fn drive_swipe(st: &mut SwipeState, steps: &[Drag], left_w: f64, right_w: f64, dt: f64) -> bool {
    for s in steps {
        match *s {
            Drag::Start { .. } => {}
            Drag::Move { delta, .. } => st.drag_by(delta.0, left_w, right_w),
            Drag::End { velocity, .. } => {
                st.release(velocity.0, left_w, right_w);
            }
        }
    }
    st.dragging() | st.step(dt)
}

/// The composer's rect in `viewport`: `height` tall, its bottom on the soft keyboard
/// when it is up, else on the bottom safe area (home indicator).
pub fn composer_rect<S: Spec>(view: &VisualView<S>, viewport: Rect, height: f64) -> Rect {
    let bottom = uzor::ui::mobile::composer_bottom(viewport.y, viewport.height, view.keyboard(), view.safe_area().bottom);
    Rect::new(viewport.x, bottom - height, viewport.width, height)
}

/// How far to scroll (px, positive = content up) so the focused field at `field`
/// clears the soft keyboard by 8 px in `viewport` (0 when visible or no keyboard).
pub fn reveal_field<S: Spec>(view: &VisualView<S>, viewport: Rect, field: Rect) -> f64 {
    if view.keyboard() <= 0.0 {
        return 0.0;
    }
    uzor::ui::mobile::reveal_above(field.y, field.y + field.height, viewport.y, viewport.height, view.keyboard(), 8.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::spec::{PanelHome, Spec};
    use uzor::layout::docking::DockPanel;

    #[derive(Clone, Debug)]
    struct P;
    impl DockPanel for P {
        fn title(&self) -> &str {
            "m"
        }
        fn type_id(&self) -> &'static str {
            "m"
        }
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    enum Never {}
    struct T;
    impl Spec for T {
        type Panel = P;
        type Overlay = Never;
        type Action = Never;
        fn decode_panel(_home: PanelHome, _type_id: &str) -> Option<P> {
            Some(P)
        }
    }

    fn pan(origin: (f64, f64), path: &[(f64, f64)], v: (f64, f64)) -> Vec<Gesture> {
        let mut g = vec![Gesture::PanStart { origin: Point::new(origin.0, origin.1), pos: Point::new(path[0].0, path[0].1), axis: Axis::Vertical, edge: None }];
        let mut last = path[0];
        for p in &path[1..] {
            g.push(Gesture::Pan { pos: Point::new(p.0, p.1), delta: (p.0 - last.0, p.1 - last.1), total: (p.0 - origin.0, p.1 - origin.1), edge: None });
            last = *p;
        }
        g.push(Gesture::PanEnd { pos: Point::new(last.0, last.1), velocity: v, edge: None });
        g
    }

    #[test]
    fn a_touch_pan_on_the_handle_moves_and_flicks_the_sheet() {
        let mut view = VisualView::<T>::default();
        view.coarse_pointer = true;
        view.gestures = pan((195.0, 430.0), &[(195.0, 420.0), (195.0, 300.0)], (0.0, -1500.0));
        let mut tr = DragTracker::default();
        let steps = tr.update(&view, &WidgetId::from("card:grab"), Grip::Target(Rect::new(0.0, 420.0, 390.0, 24.0)), Some(Axis::Vertical), 1.0);
        assert!(matches!(steps.first(), Some(Drag::Start { .. })) && matches!(steps.last(), Some(Drag::End { .. })));
        let mut st = SheetState::default();
        st.open(uzor::ui::mobile::Detent::Half);
        drive_sheet(&mut st, &steps, 844.0, 1.0 / 60.0);
        assert_eq!(st.detent, uzor::ui::mobile::Detent::Full, "flicked up to full");
        // A pan that starts elsewhere is not the sheet's.
        let mut tr2 = DragTracker::default();
        view.gestures = pan((195.0, 700.0), &[(195.0, 690.0)], (0.0, 0.0));
        assert!(tr2.update(&view, &WidgetId::from("card:grab"), Grip::Target(Rect::new(0.0, 420.0, 390.0, 24.0)), None, 1.0).is_empty());
    }

    #[test]
    fn an_edge_pan_opens_the_drawer() {
        let mut view = VisualView::<T>::default();
        view.coarse_pointer = true;
        view.gestures = vec![
            Gesture::PanStart { origin: Point::new(4.0, 300.0), pos: Point::new(14.0, 300.0), axis: Axis::Horizontal, edge: Some(Edge::Left) },
            Gesture::Pan { pos: Point::new(200.0, 300.0), delta: (186.0, 0.0), total: (196.0, 0.0), edge: Some(Edge::Left) },
            Gesture::PanEnd { pos: Point::new(200.0, 300.0), velocity: (100.0, 0.0), edge: Some(Edge::Left) },
        ];
        let mut tr = DragTracker::default();
        let steps = tr.update(&view, &WidgetId::from("edge"), Grip::Edge(Edge::Left, 20.0), Some(Axis::Horizontal), 1.0);
        let mut d = DrawerState::default();
        drive_drawer(&mut d, &steps, 1.0 / 60.0);
        assert!(d.open, "past half: open");
    }

    #[test]
    fn a_mouse_drag_swipes_a_row_and_swallows_its_click() {
        let id = WidgetId::from("chat:1");
        let mut view = VisualView::<T>::default();
        view.pressed = Some(id.clone());
        view.button_down = true;
        view.press_origin = Some((300.0, 100.0));
        let mut tr = DragTracker::default();
        let mut st = SwipeState::default();
        let mut t = 0.0;
        for x in [300.0, 296.0, 260.0, 200.0, 150.0] {
            view.pointer = Some((x, 101.0));
            t += 1.0 / 60.0;
            let s = tr.update(&view, &id, Grip::Target(Rect::new(0.0, 66.0, 390.0, 68.0)), Some(Axis::Horizontal), t);
            drive_swipe(&mut st, &s, 0.0, 234.0, 1.0 / 60.0);
        }
        assert!(st.offset < -100.0);
        view.pressed = None;
        view.button_down = false;
        let s = tr.update(&view, &id, Grip::Target(Rect::new(0.0, 66.0, 390.0, 68.0)), Some(Axis::Horizontal), t + 0.016);
        assert!(matches!(s.as_slice(), [Drag::End { .. }]));
        drive_swipe(&mut st, &s, 0.0, 234.0, 1.0 / 60.0);
        assert!(st.is_open());
        assert!(tr.take_swallow() && !tr.take_swallow());
    }

    #[test]
    fn a_still_mouse_press_is_a_click_not_a_drag() {
        let id = WidgetId::from("row");
        let mut view = VisualView::<T>::default();
        view.pressed = Some(id.clone());
        view.button_down = true;
        view.press_origin = Some((100.0, 100.0));
        view.pointer = Some((103.0, 101.0));
        let mut tr = DragTracker::default();
        assert!(tr.update(&view, &id, Grip::Target(Rect::new(0.0, 0.0, 390.0, 200.0)), None, 0.0).is_empty());
        view.pressed = None;
        view.button_down = false;
        assert!(tr.update(&view, &id, Grip::Target(Rect::new(0.0, 0.0, 390.0, 200.0)), None, 0.1).is_empty());
        assert!(!tr.take_swallow());
    }

    #[test]
    fn the_composer_rides_the_keyboard() {
        let mut view = VisualView::<T>::default();
        view.safe_area = crate::types::window::Insets::new(47.0, 0.0, 34.0, 0.0);
        let vp = Rect::new(0.0, 0.0, 390.0, 844.0);
        assert_eq!(composer_rect(&view, vp, 56.0).y, 844.0 - 34.0 - 56.0);
        view.keyboard = 336.0;
        assert_eq!(composer_rect(&view, vp, 56.0).y, 844.0 - 336.0 - 56.0);
        assert_eq!(reveal_field(&view, vp, Rect::new(0.0, 600.0, 390.0, 44.0)), 144.0);
    }
}
