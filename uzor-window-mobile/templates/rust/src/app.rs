//! __APP_TITLE__: the app. The same `App` runs on the desktop (`cargo run`), on
//! iOS (the Swift host calls `__APP___run`) and on Android (`android_main`).
//! Edge-to-edge: the top bar is padded by the safe area's top, the bottom bar by
//! its bottom or the keyboard; the sheet re-parses when the media environment
//! (size, insets, pointer kind) changes, so `@media` and `env()` follow it.

use uzor::layout::docking::DockPanel;
use uzor::render::TickRate;
use uzor::tokens::{BuiltinSet, Tokens};
use uzor::ui::dom::El;
use uzor::ui::domkit as k;
use uzor::ui::style::Sheet;
use uzor_framework::widgets::kit;
use uzor_framework::{
    App, AppCommand, DockTarget, Intent, IntentCx, LayoutCmd, OverlayCx, OverlayModel, PanelCx, PanelHome,
    RuntimeConfig, SizePx, Spec, WindowId, WindowIntent, WindowSpec,
};

#[derive(Clone, Debug)]
pub struct Screen;

impl DockPanel for Screen {
    fn title(&self) -> &str {
        "__APP_TITLE__"
    }
    fn type_id(&self) -> &'static str {
        "screen"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NoOverlay {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NoAction {}

pub struct AppSpec;

impl Spec for AppSpec {
    type Panel = Screen;
    type Overlay = NoOverlay;
    type Action = NoAction;
    fn decode_panel(_home: PanelHome, _type_id: &str) -> Option<Screen> {
        Some(Screen)
    }
}

const CSS: &str = r#"
div.m-page { display: flex; flex-direction: column; background: var(--k-bg) }
div.m-top { display: flex; align-items: center; height: 44px; padding: 0 16px; font-size: 17px; font-weight: 600;
            border-bottom: 1px solid var(--k-line); flex-shrink: 0 }
div.m-body { display: flex; flex-direction: column; gap: 12px; padding: 16px; flex-grow: 1; overflow: hidden }
div.m-tabs { display: flex; border-top: 1px solid var(--k-line); background: var(--k-panel); flex-shrink: 0 }
button.m-tab { flex-grow: 1; height: 49px; color: var(--k-muted); text-align: center; font-size: 11px }
button.m-tab.on { color: var(--k-fg) }
"#;

pub struct MobileApp {
    sheet: Sheet,
    media_gen: u64,
    tab: usize,
    taps: u32,
}

impl Default for MobileApp {
    fn default() -> Self {
        Self::new()
    }
}

impl MobileApp {
    pub fn new() -> Self {
        Self { sheet: k::sheet_with(CSS).expect("app css"), media_gen: uzor::ui::media::generation(), tab: 0, taps: 0 }
    }

    pub fn config() -> RuntimeConfig {
        RuntimeConfig {
            // The phone ignores the size; the desktop opens a phone-shaped window.
            windows: vec![WindowSpec::new("main", "__APP_TITLE__", SizePx::new(390, 844))],
            tokens: Tokens::builtin(BuiltinSet::Dark),
            dark: true,
            // Paint on demand: a phone must not spin frames while idle.
            tick: TickRate::Dirty,
            ..RuntimeConfig::default()
        }
    }

    fn tree(&self, h: f64, top: f64, bottom: f64) -> El {
        let tabs = ["Главная", "Чаты", "Настройки"];
        let rows: Vec<(&str, Option<&str>)> = vec![("Первый", Some("сейчас")), ("Второй", Some("5 мин")), ("Третий", None)];
        El::new("div.k-root").css(format!("height: {h}px")).child(
            El::new("div.m-page")
                .css(format!("height: {h}px"))
                .child(El::new("div.m-top").css(format!("padding-top: {top}px; height: {}px", 44.0 + top)).text("__APP_TITLE__"))
                .child(
                    El::new("div.m-body")
                        .child(k::list("rows", &rows, None))
                        .child(El::new("button.k-btn.primary").id("tap").tab().text(format!("Нажато: {}", self.taps))),
                )
                .child(tabs.iter().enumerate().fold(
                    El::new("div.m-tabs").css(format!("padding-bottom: {bottom}px")),
                    |bar, (i, t)| bar.child(El::new(if i == self.tab { "button.m-tab.on" } else { "button.m-tab" }).id(format!("tab-{i}")).tab().text(*t)),
                )),
        )
    }
}

impl App for MobileApp {
    type Spec = AppSpec;

    fn overlay_model(&self, _win: WindowId, _id: NoOverlay) -> OverlayModel {
        OverlayModel::default()
    }

    fn panel(&mut self, cx: &mut PanelCx<'_, Self::Spec>) {
        // `@media` / `env()` were evaluated at parse time: re-parse when they moved.
        let gen = uzor::ui::media::generation();
        if gen != self.media_gen {
            self.media_gen = gen;
            self.sheet = k::sheet_with(CSS).expect("app css");
        }
        let safe = cx.view.safe_area();
        let bottom = safe.bottom.max(cx.view.keyboard());
        let body = cx.rect;
        let resp = kit::dom(body, &self.sheet, self.tree(body.height, safe.top, bottom)).build(cx);
        match resp.clicked.as_deref() {
            Some("tap") => {
                self.taps += 1;
                uzor_window_mobile::haptics::haptic(uzor_window_mobile::haptics::Haptic::Light);
            }
            Some(id) if id.starts_with("tab-") => self.tab = id[4..].parse().unwrap_or(0),
            _ => {}
        }
    }

    fn overlay_body(&mut self, _cx: &mut OverlayCx<'_, Self::Spec>) {}

    fn intent(&mut self, intent: Intent<Self::Spec>, cx: &mut IntentCx<'_, Self::Spec>) {
        if let Intent::Window(WindowIntent::Opened { win, .. }) = intent {
            // No desktop chrome strip on a phone: the screen is the app.
            cx.command(AppCommand::Layout(LayoutCmd::SetChrome { win, visible: false, height: 0.0 }));
            cx.command(AppCommand::Layout(LayoutCmd::OpenPanel { win, panel: Screen, at: DockTarget::Root }));
        }
    }
}
