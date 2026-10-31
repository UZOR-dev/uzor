//! A one-window, one-panel [`App`] around a [`Page`]: the host every single-page
//! example (the kit gallery, the DOM gallery, the launcher) runs in, natively or
//! headless. The window is undecorated with the uzor chrome, dark tokens, and paints
//! on demand (`TickRate::Dirty`): a page asks for the next frame with
//! `cx.out.request_frame()` while it animates.

use uzor::layout::docking::DockPanel;
use uzor::render::TickRate;
use uzor::tokens::{BuiltinSet, Tokens};
use uzor::CornerStyle;
use uzor_framework::{
    App, AppCommand, ChromeModel, ChromeTab, DockTarget, Intent, IntentCx, LayoutCmd, OverlayCx, OverlayModel, PanelCx,
    PanelHome, RuntimeConfig, SizePx, Spec, WindowId, WindowIntent, WindowSpec,
};

/// One page of content.
pub trait Page: 'static {
    /// Window title and dock header.
    fn title(&self) -> &str;
    /// Initial window size.
    fn size(&self) -> (u32, u32) {
        (1280, 800)
    }
    /// Paint into `cx.rect` (the whole leaf: the panel has no dock header). `now_ms` is the frame clock.
    fn paint(&mut self, cx: &mut PanelCx<'_, PageSpec>, now_ms: u64);
}

/// The single panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PagePanel;
impl DockPanel for PagePanel {
    fn title(&self) -> &str {
        "Page"
    }
    fn type_id(&self) -> &'static str {
        "page"
    }
    fn closable(&self) -> bool {
        false
    }
    fn shows_header(&self) -> bool {
        false
    }
}

/// No overlays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PageOverlay {}
/// No actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PageAction {}

/// The page app's vocabulary.
#[derive(Clone, Copy, Debug, Default)]
pub struct PageSpec;
impl Spec for PageSpec {
    type Panel = PagePanel;
    type Overlay = PageOverlay;
    type Action = PageAction;
    fn decode_panel(_home: PanelHome, _type_id: &str) -> Option<PagePanel> {
        Some(PagePanel)
    }
}

/// The app around a page.
pub struct PageApp<P: Page> {
    /// The hosted page.
    pub page: P,
}

impl<P: Page> PageApp<P> {
    /// Host `page`.
    pub fn new(page: P) -> Self {
        PageApp { page }
    }

    /// One undecorated, rounded window sized by the page, dark, painted on demand.
    pub fn runtime_config(&self) -> RuntimeConfig {
        let (w, h) = self.page.size();
        let mut spec = WindowSpec::new("main", format!("uzor — {}", self.page.title()), SizePx::new(w, h));
        spec.decorations = false;
        spec.corner_style = CornerStyle::Rounded;
        spec.border_color = Some(0x35_35_35);
        RuntimeConfig {
            windows: vec![spec],
            tokens: Tokens::builtin(BuiltinSet::Dark),
            dark: true,
            tick: TickRate::Dirty,
            ..RuntimeConfig::default()
        }
    }
}

impl<P: Page> App for PageApp<P> {
    type Spec = PageSpec;

    fn chrome_model(&self, _win: WindowId) -> ChromeModel {
        ChromeModel { tabs: vec![ChromeTab { label: self.page.title().into(), closable: false }], ..ChromeModel::default() }
    }

    fn overlay_model(&self, _win: WindowId, id: PageOverlay) -> OverlayModel {
        match id {}
    }

    fn panel(&mut self, cx: &mut PanelCx<'_, PageSpec>) {
        let now_ms = (cx.time.now.0 * 1000.0).max(0.0) as u64;
        self.page.paint(cx, now_ms);
    }

    fn overlay_body(&mut self, cx: &mut OverlayCx<'_, PageSpec>) {
        match cx.id {}
    }

    fn intent(&mut self, intent: Intent<PageSpec>, cx: &mut IntentCx<'_, PageSpec>) {
        if let Intent::Window(WindowIntent::Opened { win, .. }) = intent {
            cx.command(AppCommand::Layout(LayoutCmd::SetChrome { win, visible: true, height: 32.0 }));
            cx.command(AppCommand::Layout(LayoutCmd::OpenPanel { win, panel: PagePanel, at: DockTarget::Root }));
        }
    }
}

/// Headless helpers for page tests.
#[cfg(test)]
pub mod testing {
    use super::{Page, PageApp, PageSpec};
    use uzor::input::{ModifierKeys, MouseButton};
    use uzor_framework::{HeadlessHost, InputEvent, Point, PointerInput};

    /// A headless host around a page.
    pub type Host<P> = HeadlessHost<PageSpec, PageApp<P>>;

    /// Open `page` headless and let it lay out.
    pub fn open<P: Page>(page: P) -> Host<P> {
        let app = PageApp::new(page);
        let cfg = app.runtime_config();
        let (mut host, _) = HeadlessHost::new(app, cfg);
        for _ in 0..6 {
            host.advance(0.02);
        }
        host
    }

    /// Move, press and release the left button at `(x, y)`, then let the frame settle.
    pub fn click_at<P: Page>(host: &mut Host<P>, x: f64, y: f64) {
        let p = Point::new(x, y);
        let win = host.snapshot().windows[0].id;
        let mods = ModifierKeys::default();
        host.input(win, InputEvent::Pointer(PointerInput::Moved { pos: p, mods }));
        host.advance(0.02);
        host.input(win, InputEvent::Pointer(PointerInput::Down { pos: p, button: MouseButton::Left, mods }));
        host.advance(0.02);
        host.input(win, InputEvent::Pointer(PointerInput::Up { pos: p, button: MouseButton::Left, mods }));
        for _ in 0..3 {
            host.advance(0.02);
        }
    }
}
