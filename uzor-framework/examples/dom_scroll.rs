//! DOM scroll containers the framework scrolls (opt in with [`kit::DomBuilder::dom_scroll`]):
//! a page and a long list inside it, both plain `overflow: auto` elements with ids. Wheel, touchpad, a finger, the thumb and
//! keyboard steps move them; the list chains to the page at its ends. The app keeps no
//! scroll state: it reads the offsets back from `DomResponse::scrolls`. The second list
//! has `scrollbar-width: none`: it scrolls the same, and its bar is the app's own (a
//! plain div placed from the offset).
//!
//! ```text
//! cargo run -p uzor-framework --example dom_scroll --features native
//! ```

use uzor::layout::docking::DockPanel;
use uzor::render::TickRate;
use uzor::tokens::{BuiltinSet, Tokens};
use uzor::ui::dom::El;
use uzor::ui::style::Sheet;
use uzor_framework::widgets::kit;
use uzor_framework::widgets::queue_scroll_key;
use uzor_framework::ScrollKey;
use uzor_framework::{
    App, AppCommand, ChromeModel, DockTarget, Intent, IntentCx, KeyState, LayoutCmd, OverlayCx, OverlayModel, PanelCx, PanelHome,
    RuntimeConfig, SizePx, Spec, UnhandledInput, WindowId, WindowSpec,
};

#[path = "common/look.rs"]
mod look;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Root;
impl DockPanel for Root {
    fn title(&self) -> &str {
        "DOM scroll"
    }
    fn type_id(&self) -> &'static str {
        "dom-scroll"
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
div.page { height: 100%; overflow: auto; padding: 24px; display: flex; flex-direction: column; gap: 16px }
div.note { color: var(--k-muted) }
div.list { height: 320px; overflow: auto; border: 1px solid var(--k-line); border-radius: 8px; flex-shrink: 0 }
div.row { height: 32px; padding: 0px 12px; display: flex; align-items: center; border-bottom: 1px solid var(--k-line) }
div.bare { scrollbar-width: none }
div.own-wrap { position: relative; height: 160px; flex-shrink: 0 }
div.own-bar { position: absolute; right: 2px; width: 4px; border-radius: 2px; background: var(--k-accent) }
div.block { height: 120px; border-radius: 8px; background: var(--k-panel); flex-shrink: 0 }
"#;

struct Demo {
    sheet: Sheet,
    offsets: Vec<(String, f64)>,
    key: Option<ScrollKey>,
    /// The list opens where a saved offset says (row 41), before it was ever laid out.
    restored: bool,
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
        if !std::mem::replace(&mut self.restored, true) {
            kit::dom_scroll_to(cx, "list", 40.0 * 32.0, false);
        }
        if let Some(k) = self.key.take() {
            queue_scroll_key(cx, k);
        }
        let r = cx.rect;
        let at = |id: &str| self.offsets.iter().find(|(i, _)| i == id).map_or(0.0, |(_, y)| *y);
        let mut page = El::new("div.page").id("page").css(format!("height: {}px", r.height));
        page = page.child(El::new("h2").text("Scroll containers"));
        page = page.child(El::new("div.note").text(format!("page {:.0}px · list {:.0}px", at("page"), at("list"))));
        page = page.child(El::new("div.list").id("list").children((0..200).map(|i| El::new("div.row").text(format!("Row {}", i + 1)))));
        // 50 rows of 32 px in a 160 px box (158 inside its border): the bar from the offset.
        let (content, view) = (50.0 * 32.0, 158.0);
        let thumb = view * view / content;
        let top = at("bare") / (content - view) * (view - thumb);
        page = page.child(
            El::new("div.own-wrap")
                .child(El::new("div.list.bare").id("bare").css("height: 160px").children((0..50).map(|i| El::new("div.row").text(format!("Own bar {}", i + 1)))))
                .child(El::new("div.own-bar").css(format!("top: {top}px; height: {thumb}px"))),
        );
        for _ in 0..8 {
            page = page.child(El::new("div.block"));
        }
        let resp = kit::dom(r, &self.sheet, page).dom_scroll().build(cx);
        if resp.scrolls != self.offsets {
            self.offsets = resp.scrolls;
            cx.out.request_frame();
        }
    }

    fn intent(&mut self, intent: Intent<S>, cx: &mut IntentCx<'_, S>) {
        match intent {
            Intent::Window(uzor_framework::WindowIntent::Opened { win, .. }) => {
                cx.command(AppCommand::Layout(LayoutCmd::OpenPanel { win, panel: Root, at: DockTarget::Root }));
            }
            Intent::Unhandled(UnhandledInput::Key { key, .. }) if key.state != KeyState::Up => {
                self.key = ScrollKey::from_key(key.code, key.mods);
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), uzor_framework::FrameworkError> {
    let config = RuntimeConfig {
        windows: vec![WindowSpec::new("main", "DOM scroll", SizePx::new(900, 700))],
        tokens: Tokens::builtin(BuiltinSet::Dark),
        dark: true,
        tick: TickRate::Dirty,
        ..RuntimeConfig::default()
    };
    let demo = Demo { sheet: look::sheet(CSS), offsets: Vec::new(), key: None, restored: false };
    uzor_framework::host::native::run_native_default(demo, config)
}
