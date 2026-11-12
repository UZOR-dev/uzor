//! DOM `box-shadow` with an x offset: cards whose shadow falls right, left, straight
//! down and diagonally (a hard 3 px shadow, as hand-drawn modals use, and soft ones).
//!
//! ```text
//! cargo run -p uzor-framework --example dom_shadow_offset --features native
//! ```

use uzor::layout::docking::DockPanel;
use uzor::render::TickRate;
use uzor::tokens::{BuiltinSet, Tokens};
use uzor::ui::dom::El;
use uzor::ui::style::Sheet;
use uzor_framework::widgets::kit;
use uzor_framework::{
    App, AppCommand, ChromeModel, DockTarget, Intent, IntentCx, LayoutCmd, OverlayCx, OverlayModel, PanelCx, PanelHome, RuntimeConfig,
    SizePx, Spec, WindowId, WindowSpec,
};

#[path = "common/look.rs"]
mod look;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Root;
impl DockPanel for Root {
    fn title(&self) -> &str {
        "Shadow offsets"
    }
    fn type_id(&self) -> &'static str {
        "dom-shadow-offset"
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

const CSS: &str = r#"
div.grid { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 40px; padding: 40px }
div.card { height: 90px; padding: 12px; border-radius: 6px; background: var(--k-panel); border: 1px solid var(--k-line) }
div.hard { border-radius: 0; box-shadow: 3px 3px 0 rgba(0,0,0,0.6) }
"#;

/// `(label, box-shadow)`.
const CARDS: &[(&str, &str)] = &[
    ("3px 3px 0 (hard)", "3px 3px 0 rgba(0,0,0,0.6)"),
    ("8px 0 0 (right)", "8px 0 0 rgba(0,0,0,0.6)"),
    ("-8px 0 0 (left)", "-8px 0 0 rgba(0,0,0,0.6)"),
    ("0 8px 16px (down)", "0 8px 16px rgba(0,0,0,0.7)"),
    ("12px 12px 20px", "12px 12px 20px rgba(0,0,0,0.7)"),
    ("-12px -6px 20px", "-12px -6px 20px rgba(0,0,0,0.7)"),
];

struct Demo {
    sheet: Sheet,
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
        let grid = El::new("div.grid").children(CARDS.iter().map(|(label, shadow)| El::new("div.card").css(format!("box-shadow: {shadow}")).text(*label)));
        kit::dom(r, &self.sheet, look::root(true, r.height).child(grid)).build(cx);
    }

    fn intent(&mut self, intent: Intent<S>, cx: &mut IntentCx<'_, S>) {
        if let Intent::Window(uzor_framework::WindowIntent::Opened { win, .. }) = intent {
            cx.command(AppCommand::Layout(LayoutCmd::OpenPanel { win, panel: Root, at: DockTarget::Root }));
        }
    }
}

fn main() -> Result<(), uzor_framework::FrameworkError> {
    let config = RuntimeConfig {
        windows: vec![WindowSpec::new("main", "Shadow offsets", SizePx::new(720, 420))],
        tokens: Tokens::builtin(BuiltinSet::Dark),
        dark: true,
        tick: TickRate::Dirty,
        ..RuntimeConfig::default()
    };
    uzor_framework::host::native::run_native_default(Demo { sheet: look::sheet(CSS) }, config)
}
