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
    /// Keep the coordinator (and its text store) across frames, as the kernel does.
    keep_coord: bool,
}

impl Harness {
    fn frame<R>(&mut self, visual: &VisualView<K>, body: impl FnOnce(&mut PanelCx<'_, K>) -> R) -> R {
        let tokens = Tokens::builtin(BuiltinSet::Dark);
        if self.keep_coord {
            self.coord.begin_frame_widgets_only();
        } else {
            self.coord = InputCoordinator::new();
        }
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
    let mut name = String::from("Ann");
    let r = h.frame(&v, |cx| input("name", Rect::new(0.0, 0.0, 200.0, 28.0), &mut name).placeholder("Name").build(cx));
    assert!(r.focused);
    assert!(h.coord.focusable_ids(None).contains(&WidgetId::from("name")));
}

#[test]
fn input_edits_itself_with_undo_and_syncs_both_ways() {
    let mut h = Harness { keep_coord: true, ..Default::default() };
    let rect = Rect::new(0.0, 0.0, 200.0, 28.0);
    let mut name = String::from("Ann");
    let id = WidgetId::from("name");
    h.frame(&idle(), |cx| input("name", rect, &mut name).build(cx));
    assert_eq!(h.coord.text_fields().text(&id), "Ann", "the app's value seeds the field");
    let f = h.coord.text_fields_mut();
    f.focus(id.clone());
    for c in "a b".chars() {
        f.on_char(c);
    }
    let r = h.frame(&idle(), |cx| input("name", rect, &mut name).build(cx));
    assert!(r.changed && r.focused);
    assert_eq!(name, "Anna b");
    h.coord.text_fields_mut().on_key(uzor::KeyPress::Undo);
    let r = h.frame(&idle(), |cx| input("name", rect, &mut name).build(cx));
    assert!(r.changed);
    assert_eq!(name, "Anna ");
    name = "Bob".into();
    let r = h.frame(&idle(), |cx| input("name", rect, &mut name).build(cx));
    assert!(!r.changed, "an app write is not a user change");
    assert_eq!(h.coord.text_fields().text(&id), "Bob");
    assert_eq!(h.coord.text_fields_mut().on_key(uzor::KeyPress::Undo), uzor::TextAction::None);
}

#[test]
fn disabled_input_is_not_a_text_field() {
    let mut h = Harness::default();
    let mut name = String::new();
    h.frame(&idle(), |cx| input("name", Rect::new(0.0, 0.0, 200.0, 28.0), &mut name).disabled(true).build(cx));
    assert!(!h.coord.text_fields().has_field(&WidgetId::from("name")));
    assert!(!h.coord.focusable_ids(None).contains(&WidgetId::from("name")));
}

#[test]
fn button_reports_like_the_rest() {
    let mut h = Harness::default();
    let r = h.frame(&click("ok"), |cx| button("ok", Rect::new(0.0, 0.0, 80.0, 28.0)).text("OK").build_response(cx));
    assert!(r.clicked);
}

#[test]
fn checkbox_and_switch_report_the_new_state_and_are_named() {
    let mut h = Harness::default();
    let mut on = false;
    let r = h.frame(&click("cb"), |cx| checkbox("cb", Rect::new(0.0, 0.0, 120.0, 20.0)).label("Notify").bind(&mut on).build_response(cx));
    assert!(on && r.checked && r.changed);
    let mut sw = true;
    let r = h.frame(&click("sw"), |cx| switch("sw", Rect::new(0.0, 0.0, 40.0, 20.0)).label("Dark").disabled(true).bind(&mut sw).build_response(cx));
    assert!(sw && r.checked && !r.changed, "a disabled switch does not flip");
}

#[test]
fn table_reports_rows_and_header_columns() {
    let mut h = Harness::default();
    let rows = vec![vec!["a".to_string(), "1".into()], vec!["b".into(), "2".into()]];
    let rect = Rect::new(0.0, 0.0, 300.0, 200.0);
    let mut sel = None;
    let r = h.frame(&click("t:row:1"), |cx| table("t", rect, &["Name", "N"], &rows).columns("2fr 80px").bind(&mut sel).build(cx));
    assert_eq!((r.index, r.changed, sel), (Some(1), true, Some(1)));
    let r = h.frame(&click("t:col:0"), |cx| table("t", rect, &["Name", "N"], &rows).build(cx));
    assert_eq!(r.column, Some(0));
    let ids = h.coord.focusable_ids(None);
    assert!(ids.contains(&WidgetId::from("t:col:1")) && ids.contains(&WidgetId::from("t:row:0")));
}

#[test]
fn drawer_closes_on_scrim_and_slides_with_progress() {
    let mut h = Harness::default();
    let area = Rect::new(0.0, 0.0, 800.0, 600.0);
    let mut open = true;
    let d = drawer("d", area, &mut open).width(300.0).progress(0.5);
    assert_eq!(d.body_rect().x, 650.0);
    assert_eq!(drawer("d", area, &mut true).side(Side::Left).body_rect().x, 0.0);
    let r = h.frame(&click("d"), |cx| drawer("d", area, &mut open).build(cx));
    assert!(open && !r.changed, "a click inside the panel keeps it open");
    let r = h.frame(&click("d:scrim"), |cx| drawer("d", area, &mut open).build(cx));
    assert!(!open && r.changed);
}
