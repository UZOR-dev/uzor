//! Kit pieces driven by a hand-built [`VisualView`] over several frames.

use uzor::app_context::StateRegistry;
use uzor::input::{InputCoordinator, LayerId};
use uzor::layout::docking::{DockPanel, LeafId};
use uzor::testing::NullRenderContext;
use uzor::tokens::{BuiltinSet, Tokens};
use uzor::{Rect, WidgetId};

use super::*;
use crate::handle::{FrameTime, HookOps, PanelCx, VisualView, Widgets};
use crate::types::ids::WindowId;
use crate::types::spec::{PanelHome, Spec};

#[derive(Clone, Debug)]
struct P;
impl DockPanel for P {
    fn title(&self) -> &str {
        "kit"
    }
    fn type_id(&self) -> &'static str {
        "kit"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Never {}

struct K;
impl Spec for K {
    type Panel = P;
    type Overlay = Never;
    type Action = Never;
    fn decode_panel(_home: PanelHome, _type_id: &str) -> Option<P> {
        Some(P)
    }
}

/// Widget state that outlives a frame, as the kernel keeps it.
#[derive(Default)]
struct Harness {
    states: StateRegistry,
    coord: InputCoordinator,
}

impl Harness {
    fn frame<R>(&mut self, visual: &VisualView<K>, body: impl FnOnce(&mut PanelCx<'_, K>) -> R) -> R {
        let tokens = Tokens::builtin(BuiltinSet::Dark);
        self.coord = InputCoordinator::new();
        let mut render = NullRenderContext::default();
        let mut hooks = HookOps::<K>::default();
        let mut cx = PanelCx {
            window: WindowId(1),
            leaf: LeafId(0),
            panel: &P,
            index: 0,
            rect: Rect::new(0.0, 0.0, 800.0, 600.0),
            dpr: 1.0,
            tokens: tokens.as_ref(),
            render: &mut render,
            widgets: Widgets {
                coord: &mut self.coord,
                states: &mut self.states,
                selectables: &mut Default::default(),
                layer: LayerId::main(),
            },
            view: visual,
            time: FrameTime::default(),
            out: &mut hooks,
        };
        body(&mut cx)
    }
}

fn click(id: &str) -> VisualView<K> {
    let mut v = VisualView::<K>::default();
    v.clicked = vec![(WidgetId::from(id), 1)];
    v
}

fn idle() -> VisualView<K> {
    VisualView::<K>::default()
}

#[test]
fn tabs_lay_out_parts_and_bind_a_click() {
    let mut h = Harness::default();
    let mut tab = 0usize;
    let r = h.frame(&click("t:tab:2"), |cx| {
        tabs("t", Rect::new(0.0, 0.0, 300.0, 30.0), &["A", "B", "C"]).bind(&mut tab).build(cx)
    });
    assert_eq!((r.index, r.changed, tab), (Some(2), true, 2));
    assert_eq!(h.coord.widget_rect(&WidgetId::from("t:tab:1")), Some(Rect::new(100.0, 0.0, 100.0, 30.0)));
    let tabbable = h.coord.focusable_ids(None);
    assert!(["t:tab:0", "t:tab:1", "t:tab:2"].iter().all(|t| tabbable.contains(&WidgetId::from(*t))));
    let r = h.frame(&click("t:tab:2"), |cx| tabs("t", Rect::new(0.0, 0.0, 300.0, 30.0), &["A", "B", "C"]).bind(&mut tab).build(cx));
    assert!(!r.changed, "clicking the active tab is not a change");
}

#[test]
fn list_hits_only_visible_rows() {
    let mut h = Harness::default();
    let mut sel = None;
    let rows = ["a", "b", "c", "d", "e"];
    let r = h.frame(&click("l:row:3"), |cx| {
        list("l", Rect::new(0.0, 0.0, 200.0, 60.0), &rows).row_height(20.0).scroll(40.0).bind(&mut sel).build(cx)
    });
    assert_eq!((r.index, sel), (Some(3), Some(3)));
    assert!(h.coord.widget_rect(&WidgetId::from("l:row:1")).is_none(), "scrolled out");
    assert_eq!(h.coord.widget_rect(&WidgetId::from("l:row:2")), Some(Rect::new(0.0, 0.0, 200.0, 20.0)));
    assert!(h.coord.widget_rect(&WidgetId::from("l:row:4")).is_some());
}

#[test]
fn select_opens_picks_and_closes() {
    let mut h = Harness::default();
    let mut choice = 0usize;
    let opts = ["Low", "Mid", "High"];
    let rect = Rect::new(10.0, 10.0, 120.0, 28.0);
    h.frame(&click("s"), |cx| select("s", rect, &opts).bind(&mut choice).build(cx));
    h.frame(&idle(), |cx| select("s", rect, &opts).bind(&mut choice).build(cx));
    assert!(h.coord.widget_rect(&WidgetId::from("s:list:item:2")).is_some(), "open list registers its options");
    let r = h.frame(&click("s:list:item:2"), |cx| select("s", rect, &opts).bind(&mut choice).build(cx));
    assert_eq!((r.index, r.changed, choice), (Some(2), true, 2));
    h.frame(&idle(), |cx| select("s", rect, &opts).bind(&mut choice).build(cx));
    assert!(h.coord.widget_rect(&WidgetId::from("s:list:item:2")).is_none(), "closed after the pick");
    // A click elsewhere closes it too.
    h.frame(&click("s"), |cx| select("s", rect, &opts).build(cx));
    h.frame(&click("elsewhere"), |cx| select("s", rect, &opts).build(cx));
    h.frame(&idle(), |cx| select("s", rect, &opts).build(cx));
    assert!(h.coord.widget_rect(&WidgetId::from("s:list:item:0")).is_none());
}

#[test]
fn menu_reports_pick_and_closes() {
    let mut h = Harness::default();
    let mut open = true;
    let items = ["Copy", "Delete"];
    h.frame(&idle(), |cx| menu("m", (50.0, 50.0), &items, &mut open).danger(1).build(cx));
    assert!(open && h.coord.widget_rect(&WidgetId::from("m:item:1")).is_some());
    let r = h.frame(&click("m:item:1"), |cx| menu("m", (50.0, 50.0), &items, &mut open).build(cx));
    assert_eq!((r.index, open), (Some(1), false));
    let mut open = true;
    h.frame(&click("other"), |cx| menu("m", (50.0, 50.0), &items, &mut open).build(cx));
    assert!(!open, "a click outside dismisses");
}

#[test]
fn dialog_reports_footer_and_close() {
    let mut h = Harness::default();
    let rect = Rect::new(100.0, 100.0, 400.0, 240.0);
    let r = h.frame(&click("d:footer:1"), |cx| dialog("d", rect).title("Rename").footer(&["Save", "Cancel"]).build(cx));
    assert_eq!((r.index, r.clicked), (Some(1), false));
    let r = h.frame(&click("d:close"), |cx| dialog("d", rect).title("Rename").build(cx));
    assert!(r.clicked && r.changed);
}

#[test]
fn splitter_drags_within_range() {
    let mut h = Harness::default();
    let span = Rect::new(0.0, 0.0, 800.0, 600.0);
    let mut pos = 200.0;
    let mut press = VisualView::<K>::default();
    press.pressed = Some(WidgetId::from("sp"));
    press.button_down = true;
    press.pointer = Some((900.0, 10.0));
    let r = h.frame(&press, |cx| splitter("sp", Axis::X, span, &mut pos).range(100.0, 600.0).build(cx));
    assert!(r.pressed && r.changed);
    assert_eq!(pos, 600.0);
    let r = h.frame(&idle(), |cx| splitter("sp", Axis::X, span, &mut pos).build(cx));
    assert!(!r.pressed, "release ends the drag");
}

#[test]
fn toasts_keep_frames_coming_until_they_expire() {
    let mut h = Harness::default();
    let mut stack = ToastStackState::new();
    stack.push(ToastType { title: None, message: "Saved".into(), timestamp_ms: 0, duration_ms: 1000, severity: ToastSeverity::Info }, 0);
    let area = Rect::new(0.0, 0.0, 800.0, 600.0);
    assert!(h.frame(&idle(), |cx| toasts("toasts", area, &mut stack, 500).build(cx)).changed);
    assert!(!h.frame(&idle(), |cx| toasts("toasts", area, &mut stack, 60_000).build(cx)).changed);
}

#[test]
fn input_is_focusable_and_reports_focus() {
    let mut h = Harness::default();
    let mut v = VisualView::<K>::default();
    v.focused = Some(WidgetId::from("name"));
    let r = h.frame(&v, |cx| input("name", Rect::new(0.0, 0.0, 200.0, 28.0), "Ann").placeholder("Name").build(cx));
    assert!(r.focused);
    assert!(h.coord.focusable_ids(None).contains(&WidgetId::from("name")));
}

#[test]
fn button_reports_like_the_rest() {
    let mut h = Harness::default();
    let r = h.frame(&click("ok"), |cx| button("ok", Rect::new(0.0, 0.0, 80.0, 28.0)).text("OK").build_response(cx));
    assert!(r.clicked);
}
