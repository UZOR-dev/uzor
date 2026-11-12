//! DOM number fields: an order form with a whole-number quantity, a price on a 0.01
//! grid and a 1–125 leverage. Type (only numbers go in), use the steppers, the arrow
//! keys or the wheel over a focused field; leaving a field clamps and formats it.
//!
//! ```text
//! cargo run -p uzor-framework --example dom_number --features native
//! ```

use uzor::layout::docking::DockPanel;
use uzor::render::TickRate;
use uzor::tokens::{BuiltinSet, Tokens};
use uzor::ui::dom::{Edit, El};
use uzor::ui::domkit::{self as k, NumberSpec};
use uzor::ui::style::Sheet;
use uzor_framework::widgets::kit;
use uzor_framework::{
    App, AppCommand, ChromeModel, DockTarget, Intent, IntentCx, KeyInput, KeyState, LayoutCmd, OverlayCx, OverlayModel, PanelCx,
    PanelHome, RuntimeConfig, SizePx, Spec, UnhandledInput, WindowId, WindowSpec,
};

#[path = "common/look.rs"]
mod look;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Root;
impl DockPanel for Root {
    fn title(&self) -> &str {
        "Number fields"
    }
    fn type_id(&self) -> &'static str {
        "dom-number"
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
div.form { display: flex; flex-direction: column; gap: 14px; padding: 32px; width: 360px }
div.sum { color: var(--k-muted) }
"#;

/// `(id, label, spec, text)`.
struct Field(&'static str, &'static str, NumberSpec, String);

struct Demo {
    sheet: Sheet,
    fields: Vec<Field>,
    key: Option<KeyInput>,
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
        let mut form = El::new("div.form");
        for Field(id, label, _, text) in &self.fields {
            form = form.child(k::field(label, k::number_input(id, Edit::new(text.as_str()), label)));
        }
        let val = |i: usize| self.fields[i].2.parse(&self.fields[i].3).unwrap_or(0.0);
        form = form.child(El::new("div.sum").text(format!("Notional: {:.2}", val(0) * val(1) * val(2))));
        let root = look::root(true, r.height).child(form);
        let key = self.key.take();
        let mut b = kit::dom(r, &self.sheet, root).key(key.as_ref());
        for Field(id, _, spec, _) in &self.fields {
            b = b.number(*id, *spec);
        }
        let resp = b.build(cx);
        if !resp.edited.is_empty() {
            for (id, text) in resp.edited {
                if let Some(f) = self.fields.iter_mut().find(|f| f.0 == id) {
                    f.3 = text;
                }
            }
            cx.out.request_frame();
        }
    }

    fn intent(&mut self, intent: Intent<S>, cx: &mut IntentCx<'_, S>) {
        match intent {
            Intent::Window(uzor_framework::WindowIntent::Opened { win, .. }) => {
                cx.command(AppCommand::Layout(LayoutCmd::OpenPanel { win, panel: Root, at: DockTarget::Root }));
            }
            Intent::Unhandled(UnhandledInput::Key { key, .. }) if key.state != KeyState::Up => {
                self.key = Some(key);
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), uzor_framework::FrameworkError> {
    let config = RuntimeConfig {
        windows: vec![WindowSpec::new("main", "Number fields", SizePx::new(640, 480))],
        tokens: Tokens::builtin(BuiltinSet::Dark),
        dark: true,
        tick: TickRate::Dirty,
        ..RuntimeConfig::default()
    };
    let fields = vec![
        Field("qty", "Quantity", NumberSpec::integer().range(1.0, 1000.0), "10".into()),
        Field("price", "Price", NumberSpec::new().min(0.0).step(0.01).decimals(2), "64250.50".into()),
        Field("lev", "Leverage", NumberSpec::integer().range(1.0, 125.0), "5".into()),
    ];
    uzor_framework::host::native::run_native_default(Demo { sheet: look::sheet(CSS), fields, key: None }, config)
}
