//! An app that draws its own window chrome (a frameless window): its title strip
//! drags the window (`WindowCmd::DragMove`), the bottom-right grip resizes it
//! (`WindowCmd::DragResize`), the cursor over both is the app's (`WindowCmd::SetCursor`)
//! and the whole pointer stream reaches it (`WindowCmd::UnhandledPointer`). Ctrl+V
//! reads the clipboard synchronously (`NativeClipboard::read_text`).
//!
//! ```text
//! cargo run -p uzor-framework --example app_chrome --features native
//! ```

use uzor::input::keyboard::KeyCode;
use uzor::layout::docking::DockPanel;
use uzor::render::TickRate;
use uzor::tokens::{BuiltinSet, Tokens};
use uzor::{CursorIcon, ResizeDirection};
use uzor_framework::host::native::NativeClipboard;
use uzor_framework::{
    App, AppCommand, ChromeModel, DockTarget, Intent, IntentCx, KeyState, LayoutCmd, OverlayCx, OverlayModel, PanelCx, PanelHome,
    PointerInput, RuntimeConfig, SizePx, Spec, UnhandledInput, WindowCmd, WindowId, WindowSpec,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Root;
impl DockPanel for Root {
    fn title(&self) -> &str {
        "Own chrome"
    }
    fn type_id(&self) -> &'static str {
        "app-chrome"
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

const STRIP: f64 = 32.0;
const GRIP: f64 = 16.0;

#[derive(Clone, Copy, PartialEq)]
enum Zone {
    Strip,
    Grip,
    Body,
}

struct Demo {
    size: (f64, f64),
    zone: Zone,
    pointer: (f64, f64),
    pasted: String,
    clipboard: NativeClipboard,
}

impl Demo {
    fn zone(&self, x: f64, y: f64) -> Zone {
        if y < STRIP {
            Zone::Strip
        } else if x > self.size.0 - GRIP && y > self.size.1 - GRIP {
            Zone::Grip
        } else {
            Zone::Body
        }
    }
}

impl App for Demo {
    type Spec = S;
    fn chrome_model(&self, _w: WindowId) -> ChromeModel {
        ChromeModel::default()
    }
    fn overlay_model(&self, _w: WindowId, _id: Ov) -> OverlayModel {
        OverlayModel::default()
    }
    fn overlay_body(&mut self, _cx: &mut OverlayCx<'_, S>) {}

    fn panel(&mut self, cx: &mut PanelCx<'_, S>) {
        let r = cx.rect;
        self.size = (r.width, r.height);
        let g = &mut *cx.render;
        g.set_fill_color("#16181d");
        g.fill_rect(r.x, r.y, r.width, r.height);
        g.set_fill_color(if self.zone == Zone::Strip { "#2a2f3a" } else { "#22262e" });
        g.fill_rect(r.x, r.y, r.width, STRIP);
        g.set_font("13px sans-serif");
        g.set_fill_color("#d1d4dc");
        g.fill_text("Own chrome: drag me", r.x + 12.0, r.y + 21.0);
        g.fill_text(&format!("pointer {:.0}, {:.0}", self.pointer.0, self.pointer.1), r.x + 12.0, r.y + 64.0);
        g.fill_text(&format!("Ctrl+V: {}", if self.pasted.is_empty() { "(nothing yet)" } else { &self.pasted }), r.x + 12.0, r.y + 88.0);
        g.set_fill_color("#5d606b");
        for i in 0..3 {
            let d = 4.0 + i as f64 * 4.0;
            g.fill_rect(r.x + r.width - d, r.y + r.height - 3.0, 2.0, 2.0);
            g.fill_rect(r.x + r.width - 3.0, r.y + r.height - d, 2.0, 2.0);
        }
    }

    fn intent(&mut self, intent: Intent<S>, cx: &mut IntentCx<'_, S>) {
        match intent {
            Intent::Window(uzor_framework::WindowIntent::Opened { win, .. }) => {
                cx.command(AppCommand::Layout(LayoutCmd::SetChrome { win, visible: false, height: 0.0 }));
                cx.command(AppCommand::Layout(LayoutCmd::OpenPanel { win, panel: Root, at: DockTarget::Root }));
                cx.command(AppCommand::Window(WindowCmd::UnhandledPointer { win, all: true }));
            }
            Intent::Unhandled(UnhandledInput::Pointer { win, pointer, .. }) => match pointer {
                PointerInput::Moved { pos, .. } => {
                    self.pointer = (pos.x, pos.y);
                    let zone = self.zone(pos.x, pos.y);
                    if zone != self.zone {
                        self.zone = zone;
                        let icon = match zone {
                            Zone::Strip => Some(CursorIcon::Grab),
                            Zone::Grip => Some(CursorIcon::ResizeSouthEast),
                            Zone::Body => None,
                        };
                        cx.command(AppCommand::Window(WindowCmd::SetCursor { win, icon }));
                    }
                }
                PointerInput::Down { pos, .. } => match self.zone(pos.x, pos.y) {
                    Zone::Strip => cx.command(AppCommand::Window(WindowCmd::DragMove(win))),
                    Zone::Grip => cx.command(AppCommand::Window(WindowCmd::DragResize { win, dir: ResizeDirection::SouthEast })),
                    Zone::Body => {}
                },
                _ => {}
            },
            Intent::Unhandled(UnhandledInput::Key { key, .. }) if key.state == KeyState::Down && key.mods.ctrl && key.code == KeyCode::V => {
                self.pasted = self.clipboard.read_text().unwrap_or_default();
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), uzor_framework::FrameworkError> {
    let mut spec = WindowSpec::new("main", "Own chrome", SizePx::new(560, 320));
    spec.decorations = false;
    let config = RuntimeConfig { windows: vec![spec], tokens: Tokens::builtin(BuiltinSet::Dark), dark: true, tick: TickRate::Dirty, ..RuntimeConfig::default() };
    let demo = Demo { size: (560.0, 320.0), zone: Zone::Body, pointer: (0.0, 0.0), pasted: String::new(), clipboard: NativeClipboard::new() };
    uzor_framework::host::native::run_native_default(demo, config)
}
