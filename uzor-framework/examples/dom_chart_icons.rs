//! The chart icon set ([`uzor::ui::assets::icons::chart`]) as a DOM gallery: every
//! icon at toolbar size with its name; hover one to see it lit.
//!
//! ```text
//! cargo run -p uzor-framework --example dom_chart_icons --features native
//! ```

use uzor::layout::docking::DockPanel;
use uzor::render::TickRate;
use uzor::tokens::{BuiltinSet, Tokens};
use uzor::ui::assets::icons::chart;
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
        "Chart icons"
    }
    fn type_id(&self) -> &'static str {
        "dom-chart-icons"
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
div.grid { display: flex; flex-wrap: wrap; gap: 6px; padding: 16px; overflow: auto; height: 100% }
div.cell { display: flex; flex-direction: column; align-items: center; gap: 4px; width: 92px; padding: 8px 4px; border-radius: 8px; color: var(--k-muted) }
div.cell:hover { background: var(--k-hover); color: var(--k-fg) }
div.cell > span.ic { width: 28px; height: 28px }
div.cell > span.nm { font-size: 10px; white-space: nowrap; overflow: hidden; width: 88px; text-align: center }
"#;

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
        let grid = El::new("div.grid").children(chart::ALL.iter().map(|(name, svg)| {
            El::new("div.cell").id(format!("icon:{name}")).name(*name).child(El::new("span.ic").svg(*svg)).child(El::new("span.nm").text(*name))
        }));
        let root = look::root(true, r.height).child(grid);
        kit::dom(r, &self.sheet, root).dom_scroll().build(cx);
    }

    fn intent(&mut self, intent: Intent<S>, cx: &mut IntentCx<'_, S>) {
        if let Intent::Window(uzor_framework::WindowIntent::Opened { win, .. }) = intent {
            cx.command(AppCommand::Layout(LayoutCmd::OpenPanel { win, panel: Root, at: DockTarget::Root }));
        }
    }
}

fn main() -> Result<(), uzor_framework::FrameworkError> {
    let config = RuntimeConfig {
        windows: vec![WindowSpec::new("main", "Chart icons", SizePx::new(900, 640))],
        tokens: Tokens::builtin(BuiltinSet::Dark),
        dark: true,
        tick: TickRate::Dirty,
        ..RuntimeConfig::default()
    };
    uzor_framework::host::native::run_native_default(Demo { sheet: look::sheet(CSS) }, config)
}
