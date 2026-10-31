//! Two native windows sharing one app (F9 done-condition), each a DOM page: which
//! window it is, a shared ping counter both windows show live, an event log, and
//! buttons to open more windows or close this one.
#![allow(dead_code)]

use std::collections::BTreeMap;

use uzor::layout::docking::DockPanel;
use uzor::render::InvalidateBits;
use uzor::tokens::{BuiltinSet, Tokens};
use uzor::ui::dom::El;
use uzor::ui::domkit as k;
use uzor::ui::style::Sheet;
use uzor_framework::widgets::kit;
use uzor_framework::{
    App, AppCommand, CadenceCmd, ChromeModel, ChromeTab, DockTarget, Intent, IntentCx, LayoutCmd, OverlayCx,
    OverlayModel, PanelCx, PanelHome, RuntimeConfig, SizePx, Spec, WindowCmd, WindowId,
    WindowIntent, WindowSpec,
};

#[path = "../common/look.rs"]
mod look;

#[derive(Clone, Debug)]
pub struct WinPanel;

impl DockPanel for WinPanel {
    fn title(&self) -> &str {
        "Window"
    }
    fn type_id(&self) -> &'static str {
        "win"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NoOverlay {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NoAction {}

pub struct TwoSpec;

impl Spec for TwoSpec {
    type Panel = WinPanel;
    type Overlay = NoOverlay;
    type Action = NoAction;

    fn decode_panel(_home: PanelHome, _type_id: &str) -> Option<Self::Panel> {
        Some(WinPanel)
    }
}

const CSS: &str = r#"
div.w-head { display: flex; align-items: center; gap: 12px; flex-shrink: 0 }
div.w-badge { width: 44px; height: 44px; border-radius: 12px; background: var(--k-accent); color: var(--k-on-accent);
              font-size: 20px; font-weight: 700; text-align: center; line-height: 44px; flex-shrink: 0 }
div.w-title { display: flex; flex-direction: column; gap: 2px; flex-grow: 1 }
"#;

fn btn(id: &str, label: &str, kind: &str) -> El {
    El::new(format!("button.k-btn.{kind}")).id(id).tab().text(label)
}

/// The app: windows by id (key, letter), a shared ping counter and an event log.
pub struct TwoWindows {
    wins: BTreeMap<WindowId, (String, char)>,
    pings: u32,
    log: Vec<String>,
    opened: u32,
    sheet: Sheet,
}

impl Default for TwoWindows {
    fn default() -> Self {
        Self::new()
    }
}

impl TwoWindows {
    pub fn new() -> Self {
        Self { wins: BTreeMap::new(), pings: 0, log: Vec::new(), opened: 0, sheet: look::sheet(CSS) }
    }

    pub fn config() -> RuntimeConfig {
        RuntimeConfig {
            windows: vec![
                WindowSpec::new("main", "Uzor — window A", SizePx::new(640, 480)),
                WindowSpec::new("aux", "Uzor — window B", SizePx::new(480, 400)),
            ],
            tokens: Tokens::builtin(BuiltinSet::Dark),
            dark: true,
            ..RuntimeConfig::default()
        }
    }

    fn note(&mut self, line: String) {
        self.log.insert(0, line);
        self.log.truncate(4);
    }

    fn tree(&self, win: WindowId, h: f64) -> El {
        let (key, letter) = self.wins.get(&win).cloned().unwrap_or(("?".into(), '?'));
        let rows: Vec<(&str, Option<&str>)> = self.log.iter().map(|l| (l.as_str(), None)).collect();
        let main = key == "main";
        look::root(true, h).child(
            El::new("div.p-page")
                .css(format!("height: {h}px"))
                .child(
                    El::new("div.w-head")
                        .child(El::new("div.w-badge").text(letter.to_string()))
                        .child(
                            El::new("div.w-title")
                                .child(El::new("div.p-h1").text(format!("Window {letter}")))
                                .child(El::new("div.p-muted").text(format!("key \"{key}\" · one App, one state, {} windows", self.wins.len()))),
                        )
                        .child(k::badge("native", "ok")),
                )
                .child(
                    El::new("div.p-grid2")
                        .child(k::stat("tw-wins", "WINDOWS OPEN", &self.wins.len().to_string(), "multi-window host"))
                        .child(k::stat("tw-pings", "PINGS", &self.pings.to_string(), "shared by every window")),
                )
                .child(
                    El::new("div.p-row")
                        .child(btn("tw-ping", "Ping every window", "primary"))
                        .child(k::button("tw-open", "Open another"))
                        .child_if(!main, || btn("tw-close", "Close this one", "danger")),
                )
                .child(El::new("div.p-sec").text("EVENTS"))
                .child_if(!rows.is_empty(), || k::list("tw-log", &rows, None))
                .child_if(rows.is_empty(), || El::new("div.p-muted").text("Nothing yet: ping, open or close a window.")),
        )
    }
}

impl App for TwoWindows {
    type Spec = TwoSpec;

    fn chrome_model(&self, win: WindowId) -> ChromeModel {
        let letter = self.wins.get(&win).map_or('?', |w| w.1);
        ChromeModel { tabs: vec![ChromeTab { label: format!("Window {letter}"), closable: false }], ..ChromeModel::default() }
    }

    fn overlay_model(&self, _win: WindowId, _id: NoOverlay) -> OverlayModel {
        OverlayModel::default()
    }

    fn panel(&mut self, cx: &mut PanelCx<'_, Self::Spec>) {
        let win = cx.window;
        let body = look::leaf_body(cx.rect);
        let resp = kit::dom(body, &self.sheet, self.tree(win, body.height)).build(cx);
        let letter = self.wins.get(&win).map_or('?', |w| w.1);
        match resp.clicked.as_deref() {
            Some("tw-ping") => {
                self.pings += 1;
                self.note(format!("Window {letter} pinged everyone (#{})", self.pings));
                // Every window shows the counter: repaint them all.
                for w in self.wins.keys() {
                    cx.out.command(AppCommand::Cadence(CadenceCmd::Invalidate { win: *w, region: None, bits: InvalidateBits::ALL }));
                }
            }
            Some("tw-open") => {
                self.opened += 1;
                let key = format!("extra-{}", self.opened);
                cx.out.command(AppCommand::Window(WindowCmd::Open(WindowSpec::new(key, "Uzor — another window", SizePx::new(420, 380)))));
            }
            Some("tw-close") => cx.out.command(AppCommand::Window(WindowCmd::Close(win))),
            _ => {}
        }
    }

    fn overlay_body(&mut self, _cx: &mut OverlayCx<'_, Self::Spec>) {}

    fn intent(&mut self, intent: Intent<Self::Spec>, cx: &mut IntentCx<'_, Self::Spec>) {
        match intent {
            Intent::Window(WindowIntent::Opened { win, key }) => {
                let letter = (b'A' + (self.wins.len() as u8 % 26)) as char;
                self.wins.insert(win, (key.as_str().to_string(), letter));
                self.note(format!("Window {letter} opened"));
                cx.command(AppCommand::Layout(LayoutCmd::SetChrome { win, visible: true, height: 32.0 }));
                cx.command(AppCommand::Layout(LayoutCmd::OpenPanel { win, panel: WinPanel, at: DockTarget::Root }));
            }
            Intent::Window(WindowIntent::Closed { win }) => {
                if let Some((_, l)) = self.wins.remove(&win) {
                    self.note(format!("Window {l} closed"));
                }
            }
            _ => {}
        }
    }
}
