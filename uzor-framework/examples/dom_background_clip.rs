//! DOM `background-clip`: boxes with a translucent border over a light page. With
//! the default `border-box` the border tints the box's own fill; with `padding-box`
//! it tints the page behind, as a 1 px line drawn over a fill does; `content-box`
//! keeps the padding clear too.
//!
//! ```text
//! cargo run -p uzor-framework --example dom_background_clip --features native
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
        "Background clip"
    }
    fn type_id(&self) -> &'static str {
        "dom-background-clip"
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
div.page { padding: 40px; height: 100%; background: #d0d4dc }
div.grid { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 40px }
div.card { height: 110px; padding: 14px; border-radius: 8px; background: #2962ff; color: #ffffff; border: 8px solid rgba(255,255,255,0.35) }
"#;

/// `(label, background-clip)`.
const CARDS: &[(&str, &str)] = &[("border-box (default)", "border-box"), ("padding-box", "padding-box"), ("content-box", "content-box")];

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
        let grid = El::new("div.grid").children(CARDS.iter().map(|(label, clip)| El::new("div.card").css(format!("background-clip: {clip}")).text(*label)));
        kit::dom(r, &self.sheet, look::root(true, r.height).child(El::new("div.page").child(grid))).build(cx);
    }

    fn intent(&mut self, intent: Intent<S>, cx: &mut IntentCx<'_, S>) {
        if let Intent::Window(uzor_framework::WindowIntent::Opened { win, .. }) = intent {
            cx.command(AppCommand::Layout(LayoutCmd::OpenPanel { win, panel: Root, at: DockTarget::Root }));
        }
    }
}

fn main() -> Result<(), uzor_framework::FrameworkError> {
    let config = RuntimeConfig {
        windows: vec![WindowSpec::new("main", "Background clip", SizePx::new(720, 280))],
        tokens: Tokens::builtin(BuiltinSet::Dark),
        dark: true,
        tick: TickRate::Dirty,
        ..RuntimeConfig::default()
    };
    uzor_framework::host::native::run_native_default(Demo { sheet: look::sheet(CSS) }, config)
}
