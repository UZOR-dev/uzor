//! The component kit in one panel: every piece of
//! [`uzor_framework::widgets::kit`] with the same builder shape.
//!
//! Call [`Gallery::paint`] from an [`App::panel`](uzor_framework::App::panel)
//! hook. The gallery owns the values (text, choices, sizes); each piece
//! reads them and writes changes back through `.bind` or its
//! [`Response`](kit::Response).

use uzor::Rect;
use uzor_framework::widgets::kit::{self, Axis, ToastSeverity, ToastType};
use uzor_framework::{PanelCx, Spec};
use uzor::ui::widgets::atomic::toast::ToastStackState;

const PAD: f64 = 12.0;
const ROW: f64 = 28.0;
const GAP: f64 = 8.0;

/// The values the gallery's pieces edit.
#[derive(Default)]
pub struct Gallery {
    /// Text of the name field.
    pub name: String,
    /// Active tab.
    pub tab: usize,
    /// Chosen option of the select.
    pub level: usize,
    /// Selected list row.
    pub row: Option<usize>,
    /// Left pane width, moved by the splitter.
    pub split: f64,
    /// The context menu is open.
    pub menu_open: bool,
    /// The dialog is open.
    pub dialog_open: bool,
    /// Toasts on screen.
    pub toasts: ToastStackState,
    /// How many times "Save" was pressed.
    pub saves: u32,
    /// The last menu item picked.
    pub last_action: Option<usize>,
}

const TABS: [&str; 3] = ["Profile", "Limits", "Nodes"];
const LEVELS: [&str; 3] = ["Low", "Normal", "High"];
const ROWS: [&str; 6] = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta"];
const MENU: [&str; 3] = ["Rename", "Duplicate", "Delete"];

impl Gallery {
    /// Paint every piece into `cx`. `now_ms` drives the toasts.
    pub fn paint<S: Spec>(&mut self, cx: &mut PanelCx<'_, S>, now_ms: u64) {
        let area = cx.rect;
        if self.split <= 0.0 {
            self.split = area.x + area.width * 0.6;
        }
        let x = area.x + PAD;
        let mut y = area.y + PAD;
        let w = (self.split - x - PAD).max(120.0);

        kit::tabs("gallery:tabs", Rect::new(x, y, w, ROW), &TABS).bind(&mut self.tab).build(cx);
        y += ROW + GAP;

        kit::input("gallery:name", Rect::new(x, y, w, ROW), &self.name).placeholder("Name").build(cx);
        y += ROW + GAP;

        let half = (w - GAP) / 2.0;
        if kit::button("gallery:save", Rect::new(x, y, half, ROW)).text("Save").build_response(cx).clicked {
            self.saves += 1;
            self.toasts.push(
                ToastType { title: None, message: "Saved".into(), timestamp_ms: 0, duration_ms: 3000, severity: ToastSeverity::Info },
                now_ms,
            );
        }
        if kit::button("gallery:open", Rect::new(x + half + GAP, y, half, ROW)).text("Details…").build_response(cx).clicked {
            self.dialog_open = true;
        }
        let save_rect = Rect::new(x, y, half, ROW);
        y += ROW + GAP;

        let list_h = ROW * 4.0;
        let picked = kit::list("gallery:list", Rect::new(x, y, w, list_h), &ROWS).row_height(ROW).bind(&mut self.row).build(cx);
        if picked.index.is_some() {
            self.menu_open = true;
        }
        let menu_at = (x + w * 0.5, y + list_h * 0.5);
        y += list_h + GAP;

        // Right pane, past the splitter.
        let rx = self.split + PAD;
        let rw = (area.x + area.width - rx - PAD).max(80.0);
        kit::splitter("gallery:split", Axis::X, area, &mut self.split)
            .range(area.x + 200.0, area.x + area.width - 160.0)
            .build(cx);

        // Floating pieces last, so they sit on top.
        kit::tooltip("gallery:tip", save_rect).text("Saves the name").anchor(save_rect).build(cx);
        kit::select("gallery:level", Rect::new(rx, area.y + PAD, rw, ROW), &LEVELS).bind(&mut self.level).build(cx);
        let m = kit::menu("gallery:menu", menu_at, &MENU, &mut self.menu_open).title("Row").danger(2).build(cx);
        if m.index.is_some() {
            self.last_action = m.index;
        }
        if self.dialog_open {
            let d = Rect::new(area.x + area.width * 0.5 - 180.0, area.y + area.height * 0.5 - 100.0, 360.0, 200.0);
            let r = kit::dialog("gallery:dialog", d).title("Details").footer(&["OK", "Cancel"]).build(cx);
            if r.clicked || r.index.is_some() {
                self.dialog_open = false;
            }
        }
        let _ = y;
        kit::toasts("gallery:toasts", area, &mut self.toasts, now_ms).build(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use uzor::app_context::StateRegistry;
    use uzor::input::{InputCoordinator, LayerId};
    use uzor::layout::docking::{DockPanel, LeafId};
    use uzor::testing::NullRenderContext;
    use uzor::tokens::{BuiltinSet, Tokens};
    use uzor::WidgetId;
    use uzor_framework::{FrameTime, HookOps, PanelHome, VisualView, WindowId, Widgets};

    #[derive(Clone, Debug)]
    struct P;
    impl DockPanel for P {
        fn title(&self) -> &str {
            "gallery"
        }
        fn type_id(&self) -> &'static str {
            "gallery"
        }
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    enum Never {}
    struct G;
    impl Spec for G {
        type Panel = P;
        type Overlay = Never;
        type Action = Never;
        fn decode_panel(_home: PanelHome, _type_id: &str) -> Option<P> {
            Some(P)
        }
    }

    #[test]
    fn every_piece_registers() {
        let tokens = Tokens::builtin(BuiltinSet::Dark);
        let mut coord = InputCoordinator::new();
        let mut states = StateRegistry::new();
        let mut render = NullRenderContext::default();
        let mut hooks = HookOps::<G>::default();
        let view = VisualView::<G>::default();
        let mut gallery = Gallery { menu_open: true, dialog_open: true, ..Default::default() };
        let mut cx = PanelCx {
            window: WindowId(1),
            leaf: LeafId(0),
            panel: &P,
            index: 0,
            rect: Rect::new(0.0, 0.0, 900.0, 600.0),
            dpr: 1.0,
            tokens: tokens.as_ref(),
            render: &mut render,
            widgets: Widgets { coord: &mut coord, states: &mut states, selectables: &mut Default::default(), layer: LayerId::main() },
            view: &view,
            time: FrameTime::default(),
            out: &mut hooks,
        };
        gallery.paint(&mut cx, 0);
        for id in [
            "gallery:tabs:tab:0",
            "gallery:name",
            "gallery:save",
            "gallery:list:row:0",
            "gallery:split",
            "gallery:level",
            "gallery:menu:item:2",
            "gallery:dialog:footer:0",
        ] {
            assert!(coord.widget_rect(&WidgetId::from(id)).is_some(), "{id} not registered");
        }
        assert!((gallery.split - 540.0).abs() < 1e-9, "splitter starts at 60%");
    }
}
