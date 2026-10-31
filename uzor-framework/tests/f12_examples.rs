//! The DOM examples run headless: the image viewer and the two-window app.

#[path = "../examples/viewer/mod.rs"]
mod viewer;
#[path = "../examples/two_windows/mod.rs"]
mod two_windows;

use uzor::input::{KeyCode, ModifierKeys, MouseButton};
use uzor_framework::{HeadlessHost, InputEvent, KeyInput, KeyState, PointerInput, WindowId};

fn main_win<S: uzor_framework::Spec, A: uzor_framework::App<Spec = S>>(h: &HeadlessHost<S, A>) -> WindowId {
    h.snapshot().windows[0].id
}

fn click<S: uzor_framework::Spec, A: uzor_framework::App<Spec = S>>(h: &mut HeadlessHost<S, A>, win: WindowId, x: f64, y: f64) {
    let pos = uzor_framework::Point::new(x, y);
    h.input(win, InputEvent::Pointer(PointerInput::Moved { pos, mods: ModifierKeys::default() }));
    h.advance(0.016);
    h.input(win, InputEvent::Pointer(PointerInput::Down { pos, button: MouseButton::Left, mods: ModifierKeys::default() }));
    h.advance(0.016);
    h.input(win, InputEvent::Pointer(PointerInput::Up { pos, button: MouseButton::Left, mods: ModifierKeys::default() }));
    for _ in 0..4 {
        h.advance(0.016);
    }
}

fn key<S: uzor_framework::Spec, A: uzor_framework::App<Spec = S>>(h: &mut HeadlessHost<S, A>, win: WindowId, code: KeyCode) {
    h.input(win, InputEvent::Key(KeyInput { code, text: None, state: KeyState::Down, mods: ModifierKeys::default() }));
}

fn settle<S: uzor_framework::Spec, A: uzor_framework::App<Spec = S>>(h: &mut HeadlessHost<S, A>) {
    for _ in 0..30 {
        h.advance(0.1);
    }
}

#[test]
fn viewer_opens_steps_and_repaints_on_keys() {
    let app = viewer::Viewer::new(viewer::samples(), None, true);
    let (mut h, _handle) = HeadlessHost::new(app, viewer::Viewer::config());
    settle(&mut h);
    let win = main_win(&h);
    // The third card of the first row (M tiles in 1280 px: four columns).
    click(&mut h, win, 760.0, 150.0);
    settle(&mut h);
    assert_eq!(*h.runtime().app().open.lock().unwrap(), Some(2));
    assert!(h.advance(0.1).frames.is_empty(), "idle in the lightbox");
    key(&mut h, win, KeyCode::ArrowRight);
    let out = h.advance(0.1);
    assert!(out.frames.iter().any(|f| f.window == win), "Right repaints at once");
    assert_eq!(*h.runtime().app().open.lock().unwrap(), Some(3));
    key(&mut h, win, KeyCode::Escape);
    h.advance(0.1);
    assert_eq!(*h.runtime().app().open.lock().unwrap(), None);
}

#[test]
fn two_windows_share_one_state() {
    let (mut h, _handle) = HeadlessHost::new(two_windows::TwoWindows::new(), two_windows::TwoWindows::config());
    settle(&mut h);
    assert_eq!(h.runtime().app().window_count(), 2);
    let a = h.snapshot().windows.iter().find(|w| w.key.as_str() == "main").unwrap().id;
    let r = h.runtime().app().hit(a, "tw-ping").expect("ping button laid out");
    click(&mut h, a, r.x + 4.0, r.y + 4.0);
    settle(&mut h);
    assert_eq!(h.runtime().app().pings(), 1);
    let r = h.runtime().app().hit(a, "tw-open").expect("open button");
    click(&mut h, a, r.x + 4.0, r.y + 4.0);
    settle(&mut h);
    assert_eq!(h.runtime().app().window_count(), 3);
}
