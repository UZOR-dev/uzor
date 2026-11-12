//! A DOM toolbar from the catalog: a vertical tool rail with a flyout per split tool
//! (the chevron opens it beside the tool), and a horizontal bar with an end group.
//! Click a tool to make it the active one; pick a flyout row to swap the group's tool.
//!
//! ```text
//! cargo run -p uzor-framework --example dom_toolbar --features native
//! ```

use uzor::layout::docking::DockPanel;
use uzor::render::TickRate;
use uzor::tokens::{BuiltinSet, Tokens};
use uzor::types::Rect;
use uzor::ui::dom::El;
use uzor::ui::domkit::{self as k, icon, icons, MenuItem, ToolItem};
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
        "Toolbar"
    }
    fn type_id(&self) -> &'static str {
        "dom-toolbar"
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
div.page { position: relative; display: flex; flex-direction: column; height: 100% }
div.body { display: flex; flex-grow: 1 }
div.note { padding: 24px; color: var(--k-muted) }
"#;

/// A tool group: its key, the tools in its flyout, the one shown.
struct Group(&'static str, &'static [(&'static str, &'static str)], usize);

const LINES: &[(&str, &str)] = &[("Trend line", icons::EDIT), ("Ray", icons::RIGHT), ("Pin", icons::PIN)];
const SHAPES: &[(&str, &str)] = &[("Box", icons::BOX), ("Grid", icons::GRID)];

struct Demo {
    sheet: Sheet,
    groups: Vec<Group>,
    active: String,
    flyout: Option<usize>,
    /// The rail's tool boxes, as last laid out (flyout anchors).
    tools: Vec<(String, Rect)>,
}

impl Demo {
    fn rail(&self) -> Vec<ToolItem> {
        let mut items = vec![ToolItem::button("cursor", icon(icons::PLUS)).name("Cursor").on(self.active == "cursor")];
        for g in &self.groups {
            let (name, body) = g.1[g.2];
            items.push(ToolItem::split(g.0, icon(body)).name(name).on(self.active == g.0));
        }
        items.extend([ToolItem::separator(), ToolItem::button("trash", icon(icons::TRASH)).name("Delete all"), ToolItem::fill(), ToolItem::button("settings", icon(icons::SETTINGS)).name("Settings")]);
        items
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
        let top = [
            ToolItem::button("symbol", icon(icons::SEARCH)).label("BTCUSDT").name("Symbol"),
            ToolItem::separator(),
            ToolItem::button("chart", icon(icons::CHART)).name("Chart type"),
            ToolItem::button("undo", icon(icons::UNDO)).name("Undo"),
            ToolItem::fill(),
            ToolItem::button("shot", icon(icons::IMAGE)).name("Screenshot"),
            ToolItem::button("full", icon(icons::EXPAND)).name("Full screen"),
        ];
        let body = El::new("div.body")
            .child(k::toolbar("rail", &self.rail(), true))
            .child(El::new("div.note").text(format!("Active tool: {}", self.active)));
        let mut page = El::new("div.page").child(k::toolbar("top", &top, false)).child(body);
        // The open flyout, beside its tool (laid out last frame).
        if let Some(gi) = self.flyout {
            let g = &self.groups[gi];
            let anchor = self.tools.iter().find(|(k, _)| *k == format!("rail:{}", g.0)).map(|(_, b)| *b).unwrap_or(Rect::new(4.0, 44.0, 32.0, 32.0));
            let items: Vec<MenuItem> = g.1.iter().map(|(n, b)| MenuItem::new(*n).icon(b)).collect();
            let at = k::flyout_at(anchor, r, 180.0, 12.0 + 34.0 * items.len() as f64, true);
            page = page.child(k::menu("fly", None, &items, at));
        }
        let root = look::root(true, r.height).child(page);
        let resp = kit::dom(r, &self.sheet, root).build(cx);
        self.tools = resp.dom.hits().filter(|(k, _)| k.starts_with("rail:")).map(|(k, b)| (k.to_string(), b)).collect();
        let Some(hit) = resp.clicked else { return };
        if let Some(i) = k::picked("fly", "item", &hit) {
            if let Some(gi) = self.flyout.take() {
                self.groups[gi].2 = i;
                self.active = self.groups[gi].0.to_string();
            }
        } else if let Some((key, menu)) = k::picked_tool("rail", &hit) {
            let gi = self.groups.iter().position(|g| g.0 == key);
            if menu {
                self.flyout = if self.flyout == gi { None } else { gi };
            } else {
                self.flyout = None;
                if key != "trash" && key != "settings" {
                    self.active = key.to_string();
                }
            }
        } else {
            self.flyout = None;
        }
        cx.out.request_frame();
    }

    fn intent(&mut self, intent: Intent<S>, cx: &mut IntentCx<'_, S>) {
        if let Intent::Window(uzor_framework::WindowIntent::Opened { win, .. }) = intent {
            cx.command(AppCommand::Layout(LayoutCmd::OpenPanel { win, panel: Root, at: DockTarget::Root }));
        }
    }
}

fn main() -> Result<(), uzor_framework::FrameworkError> {
    let config = RuntimeConfig {
        windows: vec![WindowSpec::new("main", "Toolbar", SizePx::new(800, 520))],
        tokens: Tokens::builtin(BuiltinSet::Dark),
        dark: true,
        tick: TickRate::Dirty,
        ..RuntimeConfig::default()
    };
    let groups = vec![Group("lines", LINES, 0), Group("shapes", SHAPES, 0)];
    let css = format!("{}{CSS}", k::TOOLBAR_CSS);
    uzor_framework::host::native::run_native_default(Demo { sheet: look::sheet(&css), groups, active: "cursor".into(), flyout: None, tools: Vec::new() }, config)
}
