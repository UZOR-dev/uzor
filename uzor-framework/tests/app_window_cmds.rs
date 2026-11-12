//! App-side window commands: an app that draws its own chrome holds the cursor over
//! content, and starts OS window moves / resizes from its own strip and corner.

use uzor::layout::docking::DockPanel;
use uzor::render::TickRate;
use uzor::{CursorIcon, ResizeDirection};
use uzor_framework::{
    App, AppCommand, ChromeModel, DockTarget, HeadlessHost, InputEvent, Intent, IntentCx, LayoutCmd, OverlayCx, OverlayModel, PanelCx,
    PanelHome, Point, PointerInput, RuntimeConfig, SizePx, Spec, UnhandledInput, WindowCmd, WindowCommand, WindowId, WindowSpec,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Root;
impl DockPanel for Root {
    fn title(&self) -> &str {
        "own chrome"
    }
    fn type_id(&self) -> &'static str {
        "own-chrome"
    }
    fn shows_header(&self) -> bool {
        false
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Ov {}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Act {}
struct S;
impl Spec for S {
    type Panel = Root;
    type Overlay = Ov;
    type Action = Act;
    fn decode_panel(_h: PanelHome, _t: &str) -> Option<Root> {
        Some(Root)
    }
}

/// A 32 px strip on top drags the window; the bottom-right 12 px corner resizes it;
/// over the strip the cursor is a grab hand, elsewhere the framework's.
struct OwnChrome {
    cursor_held: bool,
    /// Ask for the whole pointer stream (`WindowCmd::UnhandledPointer`).
    all: bool,
    moves_seen: usize,
}

impl App for OwnChrome {
    type Spec = S;
    fn chrome_model(&self, _w: WindowId) -> ChromeModel {
        ChromeModel::default()
    }
    fn overlay_model(&self, _w: WindowId, _id: Ov) -> OverlayModel {
        OverlayModel::default()
    }
    fn overlay_body(&mut self, _cx: &mut OverlayCx<'_, S>) {}
    fn panel(&mut self, _cx: &mut PanelCx<'_, S>) {}

    fn intent(&mut self, intent: Intent<S>, cx: &mut IntentCx<'_, S>) {
        match intent {
            Intent::Window(uzor_framework::WindowIntent::Opened { win, .. }) => {
                cx.command(AppCommand::Layout(LayoutCmd::SetChrome { win, visible: false, height: 0.0 }));
                cx.command(AppCommand::Layout(LayoutCmd::OpenPanel { win, panel: Root, at: DockTarget::Root }));
                if self.all {
                    cx.command(AppCommand::Window(WindowCmd::UnhandledPointer { win, all: true }));
                }
            }
            Intent::Unhandled(UnhandledInput::Pointer { win, pointer, .. }) => match pointer {
                PointerInput::Moved { pos, .. } => {
                    self.moves_seen += 1;
                    let over = pos.y < 32.0;
                    if over != self.cursor_held {
                        self.cursor_held = over;
                        let icon = over.then_some(CursorIcon::Grab);
                        cx.command(AppCommand::Window(WindowCmd::SetCursor { win, icon }));
                    }
                }
                PointerInput::Down { pos, .. } if pos.y < 32.0 => cx.command(AppCommand::Window(WindowCmd::DragMove(win))),
                PointerInput::Down { pos, .. } if pos.x > 388.0 && pos.y > 288.0 => {
                    cx.command(AppCommand::Window(WindowCmd::DragResize { win, dir: ResizeDirection::SouthEast }))
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn host() -> (HeadlessHost<S, OwnChrome>, WindowId) {
    host_with(true)
}

fn host_with(all: bool) -> (HeadlessHost<S, OwnChrome>, WindowId) {
    let mut spec = WindowSpec::new("main", "own chrome", SizePx::new(400, 300));
    spec.decorations = false;
    let cfg = RuntimeConfig { windows: vec![spec], tick: TickRate::Dirty, ..RuntimeConfig::default() };
    let (mut h, _) = HeadlessHost::new(OwnChrome { cursor_held: false, all, moves_seen: 0 }, cfg);
    for _ in 0..3 {
        h.advance(0.016);
    }
    let win = h.snapshot().windows[0].id;
    (h, win)
}

/// Feed one pointer event, run two ticks, return the host commands they produced.
fn send(h: &mut HeadlessHost<S, OwnChrome>, win: WindowId, input: PointerInput) -> Vec<WindowCommand> {
    h.input(win, InputEvent::Pointer(input));
    let mut out: Vec<WindowCommand> = h.advance(0.016).window_commands.iter().map(|(_, c)| c.clone()).collect();
    out.extend(h.advance(0.016).window_commands.iter().map(|(_, c)| c.clone()));
    out
}

fn moved(x: f64, y: f64) -> PointerInput {
    PointerInput::Moved { pos: Point { x, y }, mods: Default::default() }
}
fn down(x: f64, y: f64) -> PointerInput {
    PointerInput::Down { pos: Point { x, y }, button: uzor::input::MouseButton::Left, mods: Default::default() }
}
fn up(x: f64, y: f64) -> PointerInput {
    PointerInput::Up { pos: Point { x, y }, button: uzor::input::MouseButton::Left, mods: Default::default() }
}

fn cursors(cmds: &[WindowCommand]) -> Vec<CursorIcon> {
    cmds.iter().filter_map(|c| if let WindowCommand::SetCursor(i) = c { Some(*i) } else { None }).collect()
}

#[test]
fn the_apps_cursor_holds_over_content_until_it_lets_go() {
    let (mut h, win) = host();
    let c = send(&mut h, win, moved(200.0, 10.0));
    assert_eq!(cursors(&c).last(), Some(&CursorIcon::Grab), "{c:?}");
    // Further moves over the strip: the framework does not put its arrow back.
    let c = send(&mut h, win, moved(220.0, 12.0));
    assert!(!cursors(&c).contains(&CursorIcon::Default), "{c:?}");
    // Off the strip the app lets go and the framework's cursor returns.
    let c = send(&mut h, win, moved(200.0, 150.0));
    let c2 = send(&mut h, win, moved(201.0, 151.0));
    let all: Vec<_> = cursors(&c).into_iter().chain(cursors(&c2)).collect();
    assert_eq!(all.last(), Some(&CursorIcon::Default), "{c:?} {c2:?}");
}

#[test]
fn a_press_on_the_strip_drags_and_on_the_corner_resizes() {
    let (mut h, win) = host();
    send(&mut h, win, moved(200.0, 10.0));
    let c = send(&mut h, win, down(200.0, 10.0));
    assert!(c.iter().any(|c| matches!(c, WindowCommand::DragWindow)), "{c:?}");
    send(&mut h, win, up(200.0, 10.0));
    send(&mut h, win, moved(395.0, 295.0));
    let c = send(&mut h, win, down(395.0, 295.0));
    assert!(c.iter().any(|c| matches!(c, WindowCommand::DragResizeWindow(ResizeDirection::SouthEast))), "{c:?}");
    send(&mut h, win, up(395.0, 295.0));
    // A press elsewhere asks the OS for nothing.
    let c = send(&mut h, win, down(200.0, 150.0));
    assert!(!c.iter().any(|c| matches!(c, WindowCommand::DragWindow | WindowCommand::DragResizeWindow(_))), "{c:?}");
}

#[test]
fn moves_reach_the_app_only_when_it_asks_for_the_whole_stream() {
    let (mut h, win) = host_with(false);
    send(&mut h, win, moved(200.0, 150.0));
    send(&mut h, win, moved(210.0, 10.0));
    assert_eq!(h.runtime().app().moves_seen, 0, "default: presses only");
    let c = send(&mut h, win, down(210.0, 10.0));
    assert!(c.iter().any(|c| matches!(c, WindowCommand::DragWindow)), "presses still arrive");
    let (mut h, win) = host_with(true);
    send(&mut h, win, moved(200.0, 150.0));
    send(&mut h, win, moved(210.0, 10.0));
    assert_eq!(h.runtime().app().moves_seen, 2);
}
