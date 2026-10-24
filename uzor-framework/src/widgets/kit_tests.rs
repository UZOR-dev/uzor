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

#[test]
fn field_registers_names_masks_and_feeds_the_store() {
    let mut h = Harness { keep_coord: true, ..Default::default() };
    let rect = Rect::new(10.0, 0.0, 200.0, 28.0);
    let id = WidgetId::from("pw");
    let pw = || field("pw", rect).config(uzor::TextFieldConfig::password()).placeholder("Password").masked(true).clip(true).colors("white", "gray");
    let r = h.frame(&idle(), |cx| pw().build(cx));
    assert_eq!(r, FieldResponse::default());
    assert!(h.coord.focusable_ids(None).contains(&id));
    assert!(h.coord.a11y_tree().nodes().iter().any(|n| n.name == "Password"));
    let f = h.coord.text_fields_mut();
    f.focus(id.clone());
    for c in "abc".chars() {
        f.on_char(c);
    }
    let mut v = idle();
    v.focused = Some(id.clone());
    let r = h.frame(&v, |cx| pw().build(cx));
    assert_eq!(r, FieldResponse { text: "abc".into(), focused: true });
    let st = h.coord.text_fields().field_state(&id).unwrap();
    assert_eq!(st.text, "abc");
    let r = h.frame(&v, |cx| pw().set(Some("xy")).build(cx));
    assert_eq!(r.text, "xy");
    let r = h.frame(&v, |cx| pw().set(Some("")).build(cx));
    assert_eq!(r.text, "");
}

fn key(code: uzor::input::keyboard::KeyCode, text: Option<&str>) -> crate::KeyInput {
    crate::KeyInput { code, text: text.map(str::to_string), state: crate::KeyState::Down, mods: Default::default() }
}

#[test]
fn select_keyboard_search_escape_and_stays_inside_its_container() {
    use uzor::input::keyboard::KeyCode as K;
    let mut h = Harness::default();
    let zones: Vec<String> = (0..30).map(|i| format!("Zone/City_{i:02}")).collect();
    let opts: Vec<&str> = zones.iter().map(String::as_str).collect();
    let mut choice = 3usize;
    // The trigger sits near the bottom-right of a 400x300 dialog.
    let dlg = Rect::new(0.0, 0.0, 400.0, 300.0);
    let rect = Rect::new(300.0, 240.0, 90.0, 28.0);
    let none: Option<&crate::KeyInput> = None;
    let frame = |h: &mut Harness, ev: &_, k: Option<&crate::KeyInput>, c: &mut usize| h.frame(ev, |cx| select("tz", rect, &opts).within(dlg).key(k).bind(c).build(cx));
    let r = frame(&mut h, &click("tz"), none, &mut choice);
    assert!(r.checked, "a click opens it");
    frame(&mut h, &idle(), none, &mut choice);
    // Searchable (30 > 10 rows): 280 wide, flipped above and shifted left, inside the dialog.
    let list = h.coord.widget_rect(&WidgetId::from("tz:list")).expect("list registered");
    assert!(list.x >= 0.0 && list.x + list.width <= 400.0 && list.y >= 0.0 && list.y + list.height <= 300.0, "{list:?}");
    assert!(list.y + list.height <= 240.0, "opens above the trigger: {list:?}");
    // The chosen option is highlighted; ArrowDown moves; Enter picks it.
    let down = key(K::ArrowDown, None);
    let r = frame(&mut h, &idle(), Some(&down), &mut choice);
    assert!(r.pressed && r.checked);
    let enter = key(K::Enter, None);
    let r = frame(&mut h, &idle(), Some(&enter), &mut choice);
    assert_eq!((r.index, r.changed, choice, r.checked), (Some(4), true, 4, false));
    // Enter on the focused trigger is not handled here (not focused in the harness): reopen by click,
    // type to filter, then Enter picks the first match.
    frame(&mut h, &click("tz"), none, &mut choice);
    for t in ["2", "7"] {
        let k = key(K::Num2, Some(t));
        frame(&mut h, &idle(), Some(&k), &mut choice);
    }
    frame(&mut h, &idle(), none, &mut choice);
    assert!(h.coord.widget_rect(&WidgetId::from("tz:list:item:27")).is_some());
    assert!(h.coord.widget_rect(&WidgetId::from("tz:list:item:26")).is_none(), "filtered out");
    let r = frame(&mut h, &idle(), Some(&enter), &mut choice);
    assert_eq!((r.index, choice), (Some(27), 27));
    // Escape closes without picking, and says it used the key.
    frame(&mut h, &click("tz"), none, &mut choice);
    let esc = key(K::Escape, None);
    let r = frame(&mut h, &idle(), Some(&esc), &mut choice);
    assert!(r.pressed && !r.checked && r.index.is_none());
    frame(&mut h, &idle(), none, &mut choice);
    assert!(h.coord.widget_rect(&WidgetId::from("tz:list")).is_none());
}

#[test]
fn pointer_over_answers_only_for_the_topmost_target() {
    let term = WidgetId::from("term");
    let mut v = VisualView::<K>::default();
    v.pointer = Some((300.0, 100.0));
    v.scroll = (0.0, -2.0);
    // A drawer covers the terminal: its card is the hover target.
    v.hovered = Some(WidgetId::from("bd-card"));
    v.hover_path = vec![WidgetId::from("bd-card")];
    assert_eq!(v.pointer_over(&term), None);
    assert_eq!(v.scroll_over(&term), (0.0, 0.0));
    assert_eq!(v.raw_pointer(), Some((300.0, 100.0)), "the raw position stays available, opt-in");
    v.hovered = Some(term.clone());
    v.hover_path = vec![term.clone()];
    assert_eq!(v.pointer_over(&term), Some((300.0, 100.0)));
    assert_eq!(v.scroll_over(&term), (0.0, -2.0));
    // Capture: the pressed widget keeps the pointer while a drag crosses a cover.
    v.hovered = Some(WidgetId::from("bd-card"));
    v.hover_path = vec![WidgetId::from("bd-card")];
    v.pressed = Some(term.clone());
    assert_eq!(v.pointer_over(&term), Some((300.0, 100.0)));
}

#[test]
fn press_origin_is_reported_only_for_a_held_press() {
    let mut v = VisualView::<K>::default();
    v.press_origin = Some((12.0, 3.0));
    assert_eq!(v.press_origin(), None, "no button held");
    v.button_down = true;
    assert_eq!(v.press_origin(), None, "held over nothing pressable");
    v.pressed = Some(WidgetId::from("bar"));
    v.pointer = Some((40.0, 9.0));
    assert_eq!(v.press_origin(), Some((12.0, 3.0)));
    assert_eq!(v.pressed_at(&WidgetId::from("bar")), Some((12.0, 3.0)));
    assert_eq!(v.pressed_at(&WidgetId::from("other")), None);
}
