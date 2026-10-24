//! A draggable pane separator (the DOM "dragger"): a thin line with a wider
//! grab strip. A press on the strip starts the drag; while the button is
//! held the pane edge follows the pointer (the engine captures the pointer
//! for the drag-sensitive strip); the release ends it. The app owns the
//! sizes, the splitter only reports where the edge should go.

use uzor::input::{InputCoordinator, LayerId, Sense, WidgetKind};
use uzor::{Rect, WidgetId};

use crate::handle::VisualView;
use crate::types::spec::Spec;

/// The direction the separator moves in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// A vertical line, dragged left / right.
    X,
    /// A horizontal line, dragged up / down.
    Y,
}

/// What a frame of the splitter yields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SplitterEvent {
    /// Not dragging.
    Idle,
    /// Dragging: the pointer coordinate along the axis.
    Drag(f64),
    /// The drag ended this frame (persist the size now).
    Released,
}

/// Per-separator drag state.
#[derive(Clone, Debug, Default)]
pub struct Splitter {
    dragging: bool,
}

impl Splitter {
    /// Advance one frame from the view: a press on `id` with the button
    /// down starts a drag; the drag follows the pointer until release.
    pub fn update<S: Spec>(&mut self, view: &VisualView<S>, id: &WidgetId, axis: Axis) -> SplitterEvent {
        if view.is_pressed(id) && view.button_down() {
            self.dragging = true;
        }
        if !self.dragging {
            return SplitterEvent::Idle;
        }
        if !view.button_down() {
            self.dragging = false;
            return SplitterEvent::Released;
        }
        match (view.raw_pointer(), axis) {
            (Some((x, _)), Axis::X) => SplitterEvent::Drag(x),
            (Some((_, y)), Axis::Y) => SplitterEvent::Drag(y),
            (None, _) => SplitterEvent::Idle,
        }
    }

    /// A drag is in progress.
    pub fn dragging(&self) -> bool {
        self.dragging
    }

    /// Paint the line highlighted: while dragged, or hovered while no
    /// other button is held.
    pub fn hot<S: Spec>(&self, view: &VisualView<S>, id: &WidgetId) -> bool {
        self.dragging || (!view.button_down() && view.is_hovered(id))
    }
}

/// The grab strip of a line at `pos` across `span` (`grab` px wide,
/// centred on the 1 px line, like the 9 px `.rz` hit of a 1 px border).
pub fn grab_rect(axis: Axis, pos: f64, span: Rect, grab: f64) -> Rect {
    let h = (grab - 1.0) / 2.0;
    match axis {
        Axis::X => Rect::new(pos - h, span.y, grab, span.height),
        Axis::Y => Rect::new(span.x, pos - h, span.width, grab),
    }
}

/// Register the grab strip (a drag-sensitive `Splitter`, resize cursor).
pub fn register(coord: &mut InputCoordinator, id: impl Into<WidgetId>, rect: Rect, layer: &LayerId) {
    coord.register_atomic(id.into(), WidgetKind::Splitter, rect, Sense::CLICK_AND_DRAG, layer);
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
            "p"
        }
        fn type_id(&self) -> &'static str {
            "p"
        }
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    enum Tag {}
    struct TestSpec;
    impl Spec for TestSpec {
        type Panel = P;
        type Overlay = Tag;
        type Action = Tag;
        fn decode_panel(_home: PanelHome, _type_id: &str) -> Option<P> {
            None
        }
    }

    #[test]
    fn press_drag_release() {
        let id = WidgetId::from("split");
        let mut s = Splitter::default();
        let mut v = VisualView::<TestSpec>::default();
        v.pointer = Some((100.0, 5.0));
        v.hovered = Some(id.clone());
        assert_eq!(s.update(&v, &id, Axis::X), SplitterEvent::Idle);
        assert!(s.hot(&v, &id));
        v.pressed = Some(id.clone());
        v.button_down = true;
        assert_eq!(s.update(&v, &id, Axis::X), SplitterEvent::Drag(100.0));
        // The pointer leaves the strip and the press: the drag keeps going.
        v.pressed = None;
        v.hovered = None;
        v.pointer = Some((140.0, 30.0));
        assert_eq!(s.update(&v, &id, Axis::X), SplitterEvent::Drag(140.0));
        assert_eq!(s.update(&v, &id, Axis::Y), SplitterEvent::Drag(30.0));
        assert!(s.hot(&v, &id));
        v.button_down = false;
        assert_eq!(s.update(&v, &id, Axis::X), SplitterEvent::Released);
        assert_eq!(s.update(&v, &id, Axis::X), SplitterEvent::Idle);
        assert!(!s.hot(&v, &id));
    }

    #[test]
    fn grab_strip_centres_on_the_line() {
        let span = Rect::new(0.0, 10.0, 300.0, 200.0);
        assert_eq!(grab_rect(Axis::X, 50.0, span, 9.0), Rect::new(46.0, 10.0, 9.0, 200.0));
        assert_eq!(grab_rect(Axis::Y, 50.0, span, 9.0), Rect::new(0.0, 46.0, 300.0, 9.0));
    }
}
