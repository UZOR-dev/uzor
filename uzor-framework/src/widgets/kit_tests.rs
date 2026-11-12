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
    /// Paint into this recorder instead of a null context.
    record: Option<uzor::testing::RecordingRenderContext>,
}

impl Harness {
    fn frame<R>(&mut self, visual: &VisualView<K>, body: impl FnOnce(&mut PanelCx<'_, K>) -> R) -> R {
        let tokens = Tokens::builtin(BuiltinSet::Dark);
        if self.keep_coord {
            self.coord.begin_frame_widgets_only();
        } else {
            self.coord = InputCoordinator::new();
        }
        let mut null = NullRenderContext::default();
        let render: &mut dyn uzor::render::RenderContext = match self.record.as_mut() {
            Some(r) => r,
            None => &mut null,
        };
        let mut hooks = HookOps::<K>::default();
        let mut cx = PanelCx {
            window: WindowId(1),
            leaf: LeafId(0),
            panel: &P,
            index: 0,
            rect: Rect::new(0.0, 0.0, 800.0, 600.0),
            dpr: 1.0,
            tokens: tokens.as_ref(),
            render,
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

/// Each `fill_text` / `fill_rect` with the clip in force when it ran (the intersection of
/// every `rect` + `clip` under the open `save`s; `None` = unclipped).
fn painted_with_clips(ops: &[uzor::testing::DrawOp]) -> Vec<(String, f64, Option<Rect>)> {
    use uzor::testing::DrawOp as D;
    let (mut stack, mut cur, mut path, mut out) = (Vec::new(), None::<Rect>, None::<Rect>, Vec::new());
    for op in ops {
        match op {
            D::Save => stack.push(cur),
            D::Restore => cur = stack.pop().flatten(),
            D::BeginPath => path = None,
            D::Rect { x, y, w, h } => path = Some(Rect::new(*x, *y, *w, *h)),
            D::Clip => cur = path.map(|p| cur.map_or(p, |c: Rect| c.intersect(p))),
            D::FillText { text, x, .. } => out.push((text.clone(), *x, cur)),
            D::FillRect { x, .. } => out.push(("<rect>".into(), *x, cur)),
            _ => {}
        }
    }
    out
}

#[test]
fn field_clips_scrolls_with_the_caret_and_stays_inside_its_viewport() {
    // 10 px a char: 60 chars of a pasted token in a 200 px field.
    let rect = Rect::new(10.0, 100.0, 200.0, 28.0);
    let id = WidgetId::from("tok");
    let long = format!("hsreq1.{}", "x".repeat(53));
    let mut h = Harness { keep_coord: true, record: Some(uzor::testing::RecordingRenderContext::with_char_width(10.0)), ..Default::default() };
    let mut v = idle();
    v.focused = Some(id.clone());
    h.frame(&idle(), |cx| field("tok", rect).placeholder("hsreq1.…").set(Some(&long)).build(cx));
    h.coord.text_fields_mut().focus(id.clone());
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let frame = |h: &mut Harness, v: &VisualView<K>, r: Rect, vp: Rect| {
        h.record.as_mut().unwrap().ops.clear();
        h.frame(v, |cx| field("tok", r).placeholder("hsreq1.…").clip_to(vp).build(cx));
        painted_with_clips(&h.record.as_ref().unwrap().ops)
    };
    // Focused, caret at the end: the text scrolls left so the caret shows, all inside the field.
    let p = frame(&mut h, &v, rect, viewport);
    let (t, x, clip) = p.iter().find(|(t, ..)| t == &long).cloned().expect("text painted");
    assert_eq!(clip, Some(rect), "text clipped to the field: {t}");
    assert_eq!(x, rect.x - (600.0 - 199.0), "scrolled so the end shows");
    let caret = p.iter().find(|(t, ..)| t == "<rect>").map(|c| (c.1, c.2));
    if let Some((cx, Some(c))) = caret {
        assert!(cx >= c.x && cx < c.x + c.width, "caret {cx} inside {c:?}");
    }
    // Caret home: back to the start.
    h.coord.text_fields_mut().field_state_mut(&id).unwrap().cursor = 0;
    let p = frame(&mut h, &v, rect, viewport);
    assert_eq!(p.iter().find(|(t, ..)| t == &long).unwrap().1, rect.x);
    // Not focused: from the start, still clipped.
    let p = frame(&mut h, &idle(), rect, viewport);
    assert_eq!(p.iter().find(|(t, ..)| t == &long).map(|e| (e.1, e.2)), Some((rect.x, Some(rect))));
    // Scrolled half under a footer: clipped to the viewport, and hit only there.
    let half = Rect::new(0.0, 0.0, 400.0, 114.0);
    let p = frame(&mut h, &idle(), rect, half);
    assert_eq!(p.iter().find(|(t, ..)| t == &long).unwrap().2, Some(Rect::new(10.0, 100.0, 200.0, 14.0)));
    assert_eq!(h.coord.widget_rect(&id), Some(Rect::new(10.0, 100.0, 200.0, 14.0)));
    // Scrolled out of the viewport: nothing painted (no placeholder or text under the footer).
    h.coord.text_fields_mut().clear(&id);
    let p = frame(&mut h, &idle(), rect, Rect::new(0.0, 0.0, 400.0, 90.0));
    assert!(p.is_empty(), "{p:?}");
    let p = frame(&mut h, &idle(), rect, viewport);
    assert_eq!(p.iter().find(|(t, ..)| t == "hsreq1.…").unwrap().2, Some(rect), "placeholder clipped too");
}

#[test]
fn field_scroll_keeps_the_caret_in_view() {
    assert_eq!(field_scroll(0.0, 50.0, 100.0, 200.0), 0.0);
    assert_eq!(field_scroll(0.0, 600.0, 600.0, 200.0), 401.0);
    assert_eq!(field_scroll(401.0, 500.0, 600.0, 200.0), 401.0, "caret still in view: no jump");
    assert_eq!(field_scroll(401.0, 300.0, 600.0, 200.0), 300.0);
    assert_eq!(field_scroll(401.0, 100.0, 100.0, 200.0), 0.0, "text shrank: back to the start");
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

#[test]
fn dom_tree_registers_controls_inputs_and_reports_clicks() {
    use domkit::{self, SelectOpt};
    let sheet = domkit::sheet();
    let tree = |open: bool| {
        let opts = [SelectOpt::new("Low"), SelectOpt::new("Mid").sub("default"), SelectOpt::new("High").disabled(true)];
        let mut root = uzor::ui::dom::El::new("div.k-col")
            .child(domkit::switch("notify", true, "Notifications"))
            .child(domkit::input("nick", uzor::ui::dom::Edit::new("ann").placeholder("Nick")))
            .child(domkit::select_trigger("lvl", "Mid", open, "Level"));
        if open {
            root = root.child(domkit::select_popover("lvl", &opts, Some(1), domkit::popover_at(Rect::new(0.0, 80.0, 160.0, 30.0), Rect::new(0.0, 0.0, 800.0, 600.0), 160.0, 120.0)));
        }
        root
    };
    let mut h = Harness { keep_coord: true, ..Default::default() };
    let area = Rect::new(0.0, 0.0, 400.0, 600.0);
    let r = h.frame(&click("lvl"), |cx| dom(area, &sheet, tree(false)).build(cx));
    assert_eq!(r.clicked.as_deref(), Some("lvl"));
    assert!(h.coord.focusable_ids(None).contains(&WidgetId::from("notify")));
    let r = h.frame(&click("lvl:opt:0"), |cx| dom(area, &sheet, tree(true)).build(cx));
    assert_eq!(domkit::picked_opt("lvl", r.clicked.as_deref().unwrap_or("")), Some(0));
    assert!(h.coord.widget_rect(&WidgetId::from("lvl:opt:2")).is_none(), "disabled rows are not targets");
    // The input is a text-store field seeded with its value.
    let st = h.coord.text_fields().field_state(&WidgetId::from("nick")).expect("field");
    assert_eq!(st.text, "ann");
    // Accessible names come from El::name, then text.
    let a11y = r.dom.a11y();
    assert!(a11y.iter().any(|n| n.id == "notify" && n.name == "Notifications" && n.checked == Some(true)));
}

#[test]
fn area_lines_wrap_at_spaces_and_hard_breaks() {
    let mut r = uzor::testing::RecordingRenderContext::with_char_width(10.0);
    // 10 px a char in a 60 px box: "hello world" wraps after the space.
    let l = area_lines(&mut r, "hello world\nab", 60.0);
    assert_eq!(l, vec![AreaLine { start: 0, end: 6, hard: false }, AreaLine { start: 6, end: 11, hard: true }, AreaLine { start: 12, end: 14, hard: false }]);
    // A word longer than the box breaks between chars; empty text is one line.
    assert_eq!(area_lines(&mut r, "abcdefghij", 40.0).len(), 3);
    assert_eq!(area_lines(&mut r, "", 40.0), vec![AreaLine::default()]);
    assert_eq!(area_lines(&mut r, "a\n", 40.0).len(), 2);
}

#[test]
fn area_paints_lines_and_moves_by_them() {
    use uzor::input::keyboard::keyboard::KeyPress;
    let rect = Rect::new(10.0, 100.0, 60.0, 80.0);
    let id = WidgetId::from("msg");
    let mut h = Harness { keep_coord: true, record: Some(uzor::testing::RecordingRenderContext::with_char_width(10.0)), ..Default::default() };
    let a = || area("msg", rect).placeholder("Message").line_height(20.0).font(13.0, "Inter");
    let r = h.frame(&idle(), |cx| a().set(Some("hello world\nab")).build(cx));
    assert_eq!((r.lines, r.content_h), (3, 60.0));
    let texts: Vec<String> = painted_with_clips(&h.record.as_ref().unwrap().ops).into_iter().map(|t| t.0).filter(|t| t != "<rect>").collect();
    assert_eq!(texts, ["hello ", "world", "ab"]);
    let f = h.coord.text_fields_mut();
    assert!(f.has_lines(&id));
    // A click on the third line at x = 10 + 15 puts the caret after "a"/"b" (char 13 or 14).
    f.begin_drag_at_xy(id.clone(), 10.0 + 11.0, 100.0 + 45.0);
    f.on_drag_end();
    assert_eq!(f.cursor(&id), 13);
    // Up: onto "world" at the same x; Up again: "hello "; Up past the first line: start.
    f.on_key(KeyPress::ArrowUp);
    assert_eq!(f.cursor(&id), 7);
    f.on_key(KeyPress::ArrowUp);
    assert_eq!(f.cursor(&id), 1);
    f.on_key(KeyPress::End);
    assert_eq!(f.cursor(&id), 5, "End stops at the soft wrap");
    f.on_key(KeyPress::ArrowDown);
    f.on_key(KeyPress::Home);
    assert_eq!(f.cursor(&id), 6);
    f.on_key(KeyPress::ArrowDown);
    f.on_key(KeyPress::ArrowDown);
    assert_eq!(f.cursor(&id), 14, "past the last line: the end");
    let mut v = idle();
    v.focused = Some(id.clone());
    let r = h.frame(&v, |cx| a().build(cx));
    assert!(r.focused && r.caret_top == 40.0);
}

/// A DOM element with `touch-action: none` is registered as a touch capture at a phone
/// size; its siblings (and a button drawn over it) are not.
#[test]
fn dom_touch_action_none_registers_a_touch_capture() {
    let sheet = uzor::ui::style::Sheet::parse("div.col { display: flex; flex-direction: column; height: 844px } div.chart { flex-grow: 1; touch-action: none } button.b { height: 44px }").unwrap();
    let tree = uzor::ui::dom::El::new("div.col")
        .child(uzor::ui::dom::El::new("button.b").id("top").tab().text("Top"))
        .child(uzor::ui::dom::El::new("div.chart").id("chart"))
        .child(uzor::ui::dom::El::new("button.b").id("bottom").tab().text("Bottom"));
    let mut h = Harness { keep_coord: true, ..Default::default() };
    let r = h.frame(&VisualView::default(), |cx| dom(Rect::new(0.0, 0.0, 390.0, 844.0), &sheet, tree).build(cx));
    let chart = r.dom.rect("chart").unwrap();
    assert!((chart.y - 44.0).abs() < 0.5 && (chart.height - (844.0 - 88.0)).abs() < 0.5, "{chart:?}");
    assert_eq!(h.coord.touch_capture_at(195.0, 400.0), Some(WidgetId::from("chart")));
    assert_eq!(h.coord.touch_capture_at(195.0, 20.0), None);
    assert_eq!(h.coord.touch_capture_at(195.0, 830.0), None);
}

/// A phone screen (390×844, notch 47, home indicator 34) with a paint slot between the
/// top bar and the tab bar: the slot is painted once, into its whole content box (not
/// clipped by the bars or the safe area), clipped to it, under a popover declared later;
/// it captures touch and gets its gestures.
#[test]
fn paint_slot_in_a_phone_dom_paints_in_order_and_gets_its_gestures() {
    use crate::types::gesture::Gesture as G;
    use crate::types::window::Point;
    use uzor::testing::DrawOp;
    use uzor::ui::dom::El;
    use uzor::ui::media::{Edges, MediaEnv};
    use uzor::ui::mobile::{self as m, icons, Tab};
    let env = MediaEnv::touch(390.0, 844.0, Edges::new(47.0, 0.0, 34.0, 0.0));
    let sheet = uzor::ui::style::Sheet::parse_with(&[domkit::KIT_CSS, domkit::KIT_TOUCH_CSS, m::MOBILE_CSS].concat(), &env).unwrap();
    let tabs = [Tab::new(icons::CHAT, "График"), Tab::new(icons::TASKS, "Вотчлист"), Tab::new(icons::GEAR, "Ещё")];
    let tree = m::screen(true, [
        m::top_bar("nav", "Назад", "BTCUSDT", None, &[]),
        El::new("div.m-content").child(El::new("div").paint_slot("chart").css("flex-grow: 1; min-height: 0")),
        m::tab_bar("tabs", &tabs, 0),
        El::new("div").id("pop").css("position: fixed; left: 20px; top: 300px; width: 200px; height: 100px; background: #fefefe"),
    ])
    .css("height: 844px");
    let mut v = VisualView::<K>::default();
    let pinch = G::Pinch { center: Point::new(195.0, 400.0), scale: 1.25 };
    v.captured_gestures = vec![(WidgetId::from("chart"), pinch), (WidgetId::from("other"), G::Tap { pos: Point::new(1.0, 1.0) })];
    let mut h = Harness { keep_coord: true, record: Some(uzor::testing::RecordingRenderContext::with_char_width(8.0)), ..Default::default() };
    let mut seen: Vec<(String, Rect, Vec<G>)> = Vec::new();
    let mut painter = |ctx: &mut dyn uzor::render::RenderContext, f: &SlotFrame<'_>| {
        seen.push((f.key.to_string(), f.rect, f.gestures.clone()));
        ctx.set_fill_color("#abcdef");
        ctx.fill_rect(f.rect.x, f.rect.y, f.rect.width, f.rect.height);
    };
    let r = h.frame(&v, |cx| dom(Rect::new(0.0, 0.0, 390.0, 844.0), &sheet, tree).paint_slots(&mut painter).build(cx));
    assert_eq!(seen.len(), 1, "painted once");
    let (key, rect, gestures) = &seen[0];
    assert_eq!(key, "chart");
    assert_eq!(gestures, &vec![pinch], "only its own gestures");
    assert_eq!(r.captured, vec![("chart".to_string(), pinch)]);
    // Whole content box: between the bars, inside the safe area, full width.
    let content = r.dom.slots().find(|(k, _)| *k == "chart").unwrap().1;
    assert_eq!(*rect, content);
    let back = r.dom.rect("nav:back").unwrap();
    let tab = r.dom.rect("tabs:0").unwrap();
    assert!(back.y >= 47.0, "{back:?}");
    assert!(rect.y >= back.y + back.height - 0.5, "{rect:?} under {back:?}");
    assert!(rect.y + rect.height <= tab.y + 0.5, "{rect:?} above {tab:?}");
    assert!(tab.y + tab.height <= 844.0 - 34.0 + 0.5);
    assert!(rect.x >= 0.0 && rect.x + rect.width <= 390.5 && rect.width > 380.0 && rect.height > 500.0, "{rect:?}");
    // Clipped to its box, and painted before the popover declared after it.
    let ops = &h.record.as_ref().unwrap().ops;
    let fill = ops.iter().position(|o| matches!(o, DrawOp::FillColor(c) if c == "#abcdef")).expect("slot painted");
    let clip = ops[..fill].iter().rposition(|o| matches!(o, DrawOp::Clip)).expect("clipped");
    assert!(ops[..clip].iter().rev().take(3).any(|o| matches!(o, DrawOp::Rect { x, y, w, h } if (*x, *y, *w, *h) == (rect.x, rect.y, rect.width, rect.height))), "clip is the slot box");
    let pop = ops.iter().rposition(|o| format!("{o:?}").to_ascii_lowercase().contains("fefefe")).expect("popover painted");
    assert!(fill < pop, "the popover paints over the slot");
    // It captures touch; the bars do not.
    assert_eq!(h.coord.touch_capture_at(195.0, rect.y + rect.height / 2.0), Some(WidgetId::from("chart")));
    assert_eq!(h.coord.touch_capture_at(tab.x + 5.0, tab.y + 5.0), None);
}

/// The standalone [`paint_slot`]: registers a touch capture, clips, reports its gestures.
#[test]
fn standalone_paint_slot_captures_and_reports() {
    use crate::types::gesture::Gesture as G;
    use crate::types::window::Point;
    let mut v = VisualView::<K>::default();
    let g = G::LongPressDrag { pos: Point::new(60.0, 70.0), delta: (5.0, 0.0) };
    v.captured_gestures = vec![(WidgetId::from("book"), g)];
    v.clicked = vec![(WidgetId::from("book"), 1)];
    let mut h = Harness { keep_coord: true, ..Default::default() };
    let area = Rect::new(10.0, 20.0, 370.0, 300.0);
    let mut local = None;
    let r = h.frame(&v, |cx| paint_slot("book", area).paint(cx, |_, f| local = Some(f.local(60.0, 70.0))));
    assert_eq!(r.gestures, vec![g]);
    assert!(r.clicked);
    assert_eq!(local, Some((50.0, 50.0)));
    assert_eq!(h.coord.touch_capture_at(100.0, 100.0), Some(WidgetId::from("book")));
}

/// A chart in a DOM tree on a desktop: the paint slot takes the mouse drag and the
/// wheel as well as its fingers, and its frame reports them.
#[test]
fn paint_slot_takes_the_mouse_drag_and_the_wheel() {
    use uzor::ui::dom::El;
    let sheet = uzor::ui::style::Sheet::parse(domkit::KIT_CSS).unwrap();
    let tree = || El::new("div").css("height: 400px").child(El::new("div").paint_slot("chart").css("height: 300px"));
    let mut v = VisualView::<K>::default();
    v.pressed = Some(WidgetId::from("chart"));
    v.hovered = Some(WidgetId::from("chart"));
    v.pointer = Some((120.0, 150.0));
    v.scroll = (0.0, 2.0);
    let mut h = Harness { keep_coord: true, ..Default::default() };
    let seen = std::cell::Cell::new(None);
    let mut painter = |_: &mut dyn uzor::render::RenderContext, f: &SlotFrame<'_>| seen.set(Some((f.pressed, f.wheel, f.pointer)));
    h.frame(&v, |cx| dom(Rect::new(0.0, 0.0, 800.0, 400.0), &sheet, tree()).paint_slots(&mut painter).build(cx));
    assert_eq!(seen.get(), Some((Some((120.0, 150.0)), (0.0, 2.0), Some((120.0, 150.0)))));
    let sense = h.coord.widget_sense(&WidgetId::from("chart")).expect("registered");
    assert!(sense.drag && sense.scroll && sense.click && sense.touch && sense.gesture, "{sense:?}");
    // A finger's scroll is not a wheel; a finger never presses like a mouse.
    v.scroll_touch = true;
    v.coarse_pointer = true;
    h.frame(&v, |cx| dom(Rect::new(0.0, 0.0, 800.0, 400.0), &sheet, tree()).paint_slots(&mut painter).build(cx));
    assert_eq!(seen.get().map(|s| (s.0, s.1)), Some((None, (0.0, 0.0))));
    // The standalone slot too.
    let r = h.frame(&VisualView::<K> { pressed: Some(WidgetId::from("book")), pointer: Some((30.0, 40.0)), ..Default::default() }, |cx| paint_slot("book", Rect::new(10.0, 20.0, 300.0, 200.0)).paint(cx, |_, _| {}));
    assert_eq!(r.pressed, Some((30.0, 40.0)));
    let sense = h.coord.widget_sense(&WidgetId::from("book")).expect("registered");
    assert!(sense.drag && sense.scroll && sense.touch);
}

/// A registered painter paints its slot in place of the closure, gets the slot's
/// input as local events (drag, wheel, pinch, keys while focused), turns them into
/// actions, makes the slot a Tab stop and names it; an unregistered slot still goes to
/// the closure.
#[test]
fn slot_registry_paints_routes_input_and_names_the_slot() {
    use crate::types::gesture::Gesture as G;
    use crate::types::window::Point;
    use uzor::ui::dom::{El, SlotAction, SlotEvent, SlotPaint, SlotPainter, SlotRegistry, SlotValue};
    #[derive(Default)]
    struct Chart {
        events: Vec<SlotEvent>,
        painted: Vec<Rect>,
    }
    impl SlotPainter for Chart {
        fn paint(&mut self, _: &mut dyn uzor::render::RenderContext, f: &SlotPaint<'_>) {
            self.painted.push(f.rect);
        }
        fn event(&mut self, ev: &SlotEvent, _: &SlotPaint<'_>) -> Option<SlotAction> {
            self.events.push(ev.clone());
            match ev {
                SlotEvent::Wheel { dy, .. } => Some(SlotAction::new("zoom").num(*dy)),
                _ => None,
            }
        }
        fn role(&self) -> &'static str {
            "slider"
        }
        fn name(&self) -> Option<String> {
            Some("Цена".into())
        }
        fn focusable(&self) -> bool {
            true
        }
    }
    let sheet = uzor::ui::style::Sheet::parse(domkit::KIT_CSS).unwrap();
    let tree = || {
        El::new("div")
            .css("height: 400px")
            .child(El::new("div").paint_slot("chart").css("height: 300px"))
            .child(El::new("div").paint_slot("free").css("height: 50px"))
    };
    let mut reg = SlotRegistry::new();
    reg.register("chart", Chart::default());
    let mut v = VisualView::<K>::default();
    v.pressed = Some(WidgetId::from("chart"));
    v.hovered = Some(WidgetId::from("chart"));
    v.pointer = Some((120.0, 150.0));
    v.scroll = (0.0, 2.0);
    v.captured_gestures = vec![(WidgetId::from("chart"), G::Pinch { center: Point::new(100.0, 100.0), scale: 1.5 })];
    let mut h = Harness { keep_coord: true, ..Default::default() };
    let mut closure_keys = Vec::new();
    let mut fallback = |_: &mut dyn uzor::render::RenderContext, f: &SlotFrame<'_>| closure_keys.push(f.key.to_string());
    let area = Rect::new(0.0, 0.0, 800.0, 400.0);
    let r = h.frame(&v, |cx| dom(area, &sheet, tree()).slot_registry(&mut reg).paint_slots(&mut fallback).build(cx));
    assert_eq!(closure_keys, vec!["free".to_string()], "only the unregistered slot goes to the closure");
    assert_eq!(r.slot_actions, vec![SlotAction { slot: "chart".into(), name: "zoom".into(), value: SlotValue::Num(2.0) }]);
    let c = reg.get::<Chart>("chart").unwrap();
    assert_eq!(c.painted, vec![Rect::new(0.0, 0.0, 800.0, 300.0)]);
    assert_eq!(
        c.events,
        vec![
            SlotEvent::Enter { x: 120.0, y: 150.0 },
            SlotEvent::Down { x: 120.0, y: 150.0 },
            SlotEvent::Wheel { x: 120.0, y: 150.0, dx: 0.0, dy: 2.0 },
            SlotEvent::Pinch { x: 100.0, y: 100.0, scale: 1.5 },
        ]
    );
    let wid = WidgetId::from("chart");
    let sense = h.coord.widget_sense(&wid).expect("registered");
    assert!(sense.focus && sense.keyboard && sense.drag && sense.scroll && sense.touch, "{sense:?}");
    assert_eq!(h.coord.widget_kind(&wid), Some(uzor::input::WidgetKind::Slider));
    let tree_a11y = h.coord.a11y_tree();
    let node = tree_a11y.nodes().iter().find(|n| n.id == wid).expect("a11y node");
    assert_eq!((node.role, node.name.as_str()), (uzor::a11y::A11yRole::Slider, "Цена"));
    // Next frame: focused, an arrow key, the drag moved.
    v.focused = Some(wid.clone());
    v.pointer = Some((130.0, 150.0));
    v.scroll = (0.0, 0.0);
    v.captured_gestures.clear();
    let key = KeyInput { code: uzor::input::KeyCode::ArrowRight, text: None, state: crate::KeyState::Down, mods: Default::default() };
    let r = h.frame(&v, |cx| dom(area, &sheet, tree()).slot_registry(&mut reg).key(Some(&key)).build(cx));
    assert!(r.key_used);
    let c = reg.get::<Chart>("chart").unwrap();
    assert_eq!(
        &c.events[4..],
        &[
            SlotEvent::Focus(true),
            SlotEvent::Move { x: 130.0, y: 150.0 },
            SlotEvent::Drag { x: 130.0, y: 150.0, dx: 10.0, dy: 0.0 },
            SlotEvent::Key { code: uzor::input::KeyCode::ArrowRight, text: None },
        ]
    );
}

#[test]
fn dom_search_bar_and_virtual_list_are_live_controls() {
    use uzor::ui::dom::{Edit, El};
    use uzor::ui::mobile::{row, search_bar, search_filter, Row, VirtualList};
    let sheet = uzor::ui::mobile::sheet_with("").expect("mobile sheet");
    let labels: Vec<String> = (0..500).map(|i| format!("Ticker {i:03}")).collect();
    let v = VirtualList::new(64.0);
    let tree = |q: &str, scroll: f64| {
        let hits = search_filter(&labels, q);
        let list = v.el("list", hits.len(), scroll, 700.0, None, |i| row(&format!("t:{}", hits[i]), &Row::new(labels[hits[i]].clone())));
        El::new("div.m-screen").css("height: 844px").child(search_bar("q", Edit::new(q).placeholder("Search"), "Search", Some("Cancel"))).child(list)
    };
    let mut h = Harness { keep_coord: true, ..Default::default() };
    let area = Rect::new(0.0, 0.0, 390.0, 844.0);
    let r = h.frame(&click("t:2"), |cx| dom(area, &sheet, tree("", 0.0)).build(cx));
    assert_eq!(r.clicked.as_deref(), Some("t:2"), "rows are targets");
    assert!(h.coord.widget_rect(&WidgetId::from("t:30")).is_none(), "rows out of sight are not built");
    // Scrolled down, the rows in sight are the targets.
    let r = h.frame(&click("t:300"), |cx| dom(area, &sheet, tree("", 300.0 * 64.0)).build(cx));
    assert_eq!(r.clicked.as_deref(), Some("t:300"));
    assert!(h.coord.widget_rect(&WidgetId::from("t:2")).is_none());
    // Typed: the list narrows to the matches, the clear button is live.
    let r = h.frame(&click("q:clear"), |cx| dom(area, &sheet, tree("ticker 49", 0.0)).build(cx));
    assert_eq!(r.clicked.as_deref(), Some("q:clear"));
    assert!(h.coord.text_fields().field_state(&WidgetId::from("q")).is_some(), "the input is a text-store field (it owns the typing)");
    assert!(h.coord.widget_rect(&WidgetId::from("t:490")).is_some() && h.coord.widget_rect(&WidgetId::from("t:2")).is_none());
}

/// `overflow: auto` with an id scrolls itself: the wheel moves it inside its range, the
/// tree is laid out at the new `scrollTop` the same frame, an app-scrolled element and
/// `overflow: hidden` stay the app's, and `dom_scroll_to` jumps.
#[test]
fn dom_scroll_containers_are_framework_owned() {
    use uzor::ui::dom::El;
    let sheet = uzor::ui::style::Sheet::parse("").unwrap();
    let rows = |pre: &str| (0..10).map(|i| El::new("div").id(format!("{pre}{i}")).css("height: 30px")).collect::<Vec<_>>();
    let tree = |app_top: f64| {
        El::new("div")
            .css("height: 400px")
            .child(El::new("div").id("list").css("height: 100px; overflow: auto").children(rows("r")))
            .child(El::new("div").id("own").css("height: 100px; overflow: auto").scroll_top(app_top).children(rows("o")))
            .child(El::new("div").id("hid").css("height: 100px; overflow: hidden").children(rows("h")))
    };
    let mut h = Harness { keep_coord: true, ..Default::default() };
    let panel = Rect::new(0.0, 0.0, 800.0, 400.0);
    let mut v = VisualView::<K>::default();
    v.pointer = Some((50.0, 50.0));
    v.scroll = (0.0, -2.0);
    let r = h.frame(&v, |cx| dom(panel, &sheet, tree(5.0)).dom_scroll().build(cx));
    assert_eq!(r.scrolls, vec![("list".to_string(), 80.0)], "two wheel lines of 40px; `own` and `hid` are not owned");
    assert_eq!(r.dom.get("r0").unwrap().rect.y, -80.0, "laid out at the new scrollTop this frame");
    assert!(h.coord.widget_sense(&WidgetId::from("list")).is_some_and(|s| s.scroll), "registered as a scroll target");
    // Past the end: clamped to content (300) - view (100).
    v.scroll = (0.0, -10.0);
    let r = h.frame(&v, |cx| dom(panel, &sheet, tree(5.0)).dom_scroll().build(cx));
    assert_eq!(r.scrolls[0].1, 200.0);
    // The pointer over another element: the list keeps its place, and the stored offset
    // is applied to the next tree without any app state.
    v.pointer = Some((50.0, 350.0));
    let r = h.frame(&v, |cx| dom(panel, &sheet, tree(5.0)).dom_scroll().build(cx));
    assert_eq!(r.scrolls[0].1, 200.0);
    assert_eq!(r.dom.get("r9").unwrap().rect.y, 70.0, "row 9 ends at the list's bottom");
    assert_eq!(r.dom.get("h0").unwrap().rect.y, 200.0, "overflow: hidden does not scroll");
    // A jump from the app.
    v.scroll = (0.0, 0.0);
    h.frame(&v, |cx| dom_scroll_to(cx, "list", 30.0, false));
    let r = h.frame(&v, |cx| dom(panel, &sheet, tree(5.0)).dom_scroll().build(cx));
    assert_eq!(r.scrolls[0].1, 30.0);
    assert_eq!(h.frame(&v, |cx| dom_scroll_top(cx, "list")), 30.0);
}


/// `dom_scroll_to` before the container's first frame (an app restoring its offset
/// when a view opens) lands there once the extent is known, not at the end.
#[test]
fn dom_scroll_to_before_the_first_frame() {
    use uzor::ui::dom::El;
    let sheet = uzor::ui::style::Sheet::parse("").unwrap();
    let tree = || El::new("div").css("height: 400px").child(El::new("div").id("list").css("height: 100px; overflow: auto").children((0..10).map(|i| El::new("div").id(format!("r{i}")).css("height: 30px"))));
    let mut h = Harness { keep_coord: true, ..Default::default() };
    let panel = Rect::new(0.0, 0.0, 800.0, 400.0);
    let v = VisualView::<K>::default();
    h.frame(&v, |cx| dom_scroll_to(cx, "list", 60.0, false));
    let r = h.frame(&v, |cx| dom(panel, &sheet, tree()).dom_scroll().build(cx));
    assert_eq!(r.scrolls, vec![("list".to_string(), 60.0)]);
    assert_eq!(r.dom.get("r2").unwrap().rect.y, 0.0);
}

/// Without [`DomBuilder::dom_scroll`], an `overflow: auto` box with an id is not
/// framework-owned: HQ already scrolls those itself (`@body` + `scroll_body`), and
/// a zero `scroll_top` this frame would otherwise have been taken over.
#[test]
fn overflow_auto_stays_the_apps_unless_dom_scroll_is_on() {
    use uzor::ui::dom::El;
    let sheet = uzor::ui::style::Sheet::parse("").unwrap();
    let tree = El::new("div").css("height: 400px").child(
        El::new("div").id("list").css("height: 100px; overflow: auto").children((0..10).map(|i| El::new("div").id(format!("r{i}")).css("height: 30px"))),
    );
    let mut h = Harness { keep_coord: true, ..Default::default() };
    let panel = Rect::new(0.0, 0.0, 800.0, 400.0);
    let mut v = VisualView::<K>::default();
    v.pointer = Some((50.0, 50.0));
    v.scroll = (0.0, -2.0);
    let r = h.frame(&v, |cx| dom(panel, &sheet, tree.clone()).build(cx));
    assert!(r.scrolls.is_empty(), "opt-in is off");
    assert_eq!(r.dom.get("r0").unwrap().rect.y, 0.0, "the list did not scroll itself");
    let r = h.frame(&v, |cx| dom(panel, &sheet, tree).dom_scroll().build(cx));
    assert_eq!(r.scrolls, vec![("list".to_string(), 80.0)]);
}

/// `scrollbar-width: none` on a framework-owned scroll container: no thumb or track is
/// painted or registered (the app draws its own bar), and the wheel still scrolls it.
#[test]
fn dom_scroll_scrollbar_width_none_hides_the_bar_only() {
    use uzor::ui::dom::El;
    let sheet = uzor::ui::style::Sheet::parse("div.bare { scrollbar-width: none }").unwrap();
    let rows = |pre: &str| (0..10).map(|i| El::new("div").id(format!("{pre}{i}")).css("height: 30px")).collect::<Vec<_>>();
    let tree = || {
        El::new("div")
            .css("height: 400px")
            .child(El::new("div").id("plain").css("height: 100px; overflow: auto").children(rows("p")))
            .child(El::new("div.bare").id("bare").css("height: 100px; overflow: auto").children(rows("b")))
    };
    let mut h = Harness { keep_coord: true, ..Default::default() };
    let panel = Rect::new(0.0, 0.0, 800.0, 400.0);
    let mut v = VisualView::<K>::default();
    v.pointer = Some((50.0, 150.0));
    v.scroll = (0.0, -2.0);
    let r = h.frame(&v, |cx| dom(panel, &sheet, tree()).dom_scroll().build(cx));
    assert_eq!(r.scrolls, vec![("plain".to_string(), 0.0), ("bare".to_string(), 80.0)], "the bar-less box scrolls");
    assert!(h.coord.widget_rect(&WidgetId::from("plain:thumb")).is_some(), "the host's bar by default");
    assert!(h.coord.widget_rect(&WidgetId::from("bare:thumb")).is_none() && h.coord.widget_rect(&WidgetId::from("bare:track")).is_none());
    assert_eq!(r.dom.get("bare").unwrap().style.scrollbar_width.as_deref(), Some("none"));
}

/// A DOM number field: the steppers, the arrow keys and the wheel over the focused field
/// step it on the spec's grid, a paste keeps only what may be typed, and blur tidies it.
#[test]
fn dom_number_field_steps_filters_and_tidies() {
    use uzor::input::keyboard::KeyCode;
    use uzor::ui::domkit::{number_input, NumberSpec};
    let sheet = domkit::sheet();
    let spec = NumberSpec::new().range(0.0, 10.0).step(0.5);
    let tree = || uzor::ui::dom::El::new("div.k-col").child(number_input("qty", uzor::ui::dom::Edit::new("3"), "Quantity"));
    let area = Rect::new(0.0, 0.0, 400.0, 300.0);
    let mut h = Harness { keep_coord: true, ..Default::default() };
    let qty = WidgetId::from("qty");
    // First frame seeds the store.
    h.frame(&VisualView::<K>::default(), |cx| dom(area, &sheet, tree()).number("qty", spec).build(cx));
    let r = h.frame(&click("qty:inc"), |cx| dom(area, &sheet, tree()).number("qty", spec).build(cx));
    assert_eq!(r.edited, vec![("qty".to_string(), "3.5".to_string())]);
    let mut v = VisualView::<K>::default();
    v.focused = Some(qty.clone());
    let down = key(KeyCode::ArrowDown, None);
    let r = h.frame(&v, |cx| dom(area, &sheet, tree()).number("qty", spec).key(Some(&down)).build(cx));
    assert!(r.key_used);
    assert_eq!(r.edited.last().map(|e| e.1.as_str()), Some("3"));
    // The wheel over the focused field (up = more).
    v.hovered = Some(qty.clone());
    v.pointer = Some((20.0, 10.0));
    v.scroll = (0.0, 1.0);
    let r = h.frame(&v, |cx| dom(area, &sheet, tree()).number("qty", spec).build(cx));
    assert_eq!(r.edited.last().map(|e| e.1.as_str()), Some("3.5"));
    // A paste of junk: only the number stays; leaving the field clamps and formats it.
    v.scroll = (0.0, 0.0);
    h.coord.text_fields_mut().set_text(&qty, "12,50 kg");
    let r = h.frame(&v, |cx| dom(area, &sheet, tree()).number("qty", spec).build(cx));
    assert_eq!(r.edited.last().map(|e| e.1.as_str()), Some("12,50"));
    let r = h.frame(&VisualView::<K>::default(), |cx| dom(area, &sheet, tree()).number("qty", spec).build(cx));
    assert_eq!(r.edited.last().map(|e| e.1.as_str()), Some("10"));
    assert!(h.coord.widget_sense(&WidgetId::from("qty:inc")).is_some(), "the steppers are click targets");
}
