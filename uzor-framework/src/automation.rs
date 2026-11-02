//! Automation port (feature `automation`): lets a tool outside the event
//! loop drive the app the way a user does — read the accessibility tree,
//! inject input, take screenshots — without the host knowing any transport.
//!
//! A tool holds an [`AutomationClient`] and calls [`AutomationClient::call`]
//! from its own thread; the host owns the matching [`AutomationPort`], is
//! woken through the waker it installs, and answers every [`AutoCmd`] on the
//! loop thread. Input replies come after the frame that follows the
//! injection was painted, so a caller reading the tree next sees its effect.
//!
//! The feature is off by default: release builds that do not name it carry
//! none of this code.

use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use uzor::a11y::A11yTree;
use uzor::input::core::WidgetInfo;
use uzor::types::{Rect, WidgetId};

use crate::handle::App;
use crate::runtime::Runtime;
use crate::types::bus::InputEvent;
use crate::types::frame::FrameRequest;
use crate::types::ids::WindowId;
use crate::types::spec::Spec;
use crate::types::window::SizePx;

/// Where a screenshot's pixels come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ShotSource {
    /// The window's own pixels when the host can read them back, else offscreen.
    #[default]
    Auto,
    /// Read back what the window last presented (GPU or software surface).
    Window,
    /// Paint the window once more into a CPU (tiny-skia) pixmap.
    Offscreen,
}

/// One request to the host.
#[derive(Clone, Debug)]
pub enum AutoCmd {
    /// List the open windows.
    Windows,
    /// The accessibility tree and every registered widget of a window
    /// (`None` = the focused window, else the first).
    Tree {
        /// Target window.
        win: Option<WindowId>,
    },
    /// Feed input events as if the OS sent them, in one tick; the reply
    /// comes after the following paint.
    Input {
        /// Target window.
        win: Option<WindowId>,
        /// Events, in order.
        events: Vec<InputEvent>,
    },
    /// A PNG of the window.
    Screenshot {
        /// Target window.
        win: Option<WindowId>,
        /// Pixel source.
        source: ShotSource,
    },
    /// The app's own state report ([`App::automation_state`]).
    AppState,
}

/// One open window.
#[derive(Clone, Debug, PartialEq)]
pub struct AutoWindow {
    /// Window id.
    pub id: WindowId,
    /// The app's window key.
    pub key: String,
    /// Viewport in logical pixels.
    pub viewport: Rect,
    /// Device pixel ratio.
    pub dpr: f64,
    /// Has OS keyboard focus.
    pub focused: bool,
}

/// A window's widgets at its last composed frame.
#[derive(Clone, Debug)]
pub struct AutoTree {
    /// The window.
    pub window: WindowId,
    /// Accessibility nodes (values of secret fields are bullets).
    pub a11y: A11yTree,
    /// Every registered widget, with or without an accessibility role.
    pub widgets: Vec<WidgetInfo>,
    /// Hovered widget.
    pub hovered: Option<WidgetId>,
    /// Focused widget.
    pub focused: Option<WidgetId>,
}

/// A screenshot.
#[derive(Clone, Debug)]
pub struct AutoShot {
    /// The window.
    pub window: WindowId,
    /// PNG bytes.
    pub png: Vec<u8>,
    /// Width, physical pixels.
    pub width: u32,
    /// Height, physical pixels.
    pub height: u32,
    /// `"window"` or `"offscreen"`.
    pub source: &'static str,
}

/// The host's answer.
#[derive(Clone, Debug)]
pub enum AutoReply {
    /// Open windows.
    Windows(Vec<AutoWindow>),
    /// A window's tree.
    Tree(AutoTree),
    /// Input delivered and the next frame painted.
    InputDone {
        /// The window it went to.
        window: WindowId,
        /// Host frame counter after the paint.
        frame: u64,
    },
    /// A screenshot.
    Shot(AutoShot),
    /// The app's state report (`None`: the app reports nothing).
    State(Option<String>),
    /// The command failed.
    Error(AutoError),
}

/// Why a command failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AutoError {
    /// No such window (or no window open).
    NoWindow,
    /// The host cannot do this here.
    Unsupported(&'static str),
    /// The host is gone.
    Closed,
    /// No answer in time.
    Timeout,
    /// The host tried and failed.
    Failed(String),
}

impl AutoError {
    /// Stable English error code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoWindow => "no_window",
            Self::Unsupported(_) => "unsupported",
            Self::Closed => "host_closed",
            Self::Timeout => "timeout",
            Self::Failed(_) => "failed",
        }
    }
}

impl core::fmt::Display for AutoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoWindow => write!(f, "no such window"),
            Self::Unsupported(what) => write!(f, "not supported by this host: {what}"),
            Self::Closed => write!(f, "the app's event loop is gone"),
            Self::Timeout => write!(f, "the app did not answer in time"),
            Self::Failed(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for AutoError {}

/// One queued request and where its answer goes.
pub type AutoJob = (AutoCmd, SyncSender<AutoReply>);

type WakeFn = Box<dyn Fn() + Send + Sync>;
type WakeSlot = Arc<Mutex<Option<WakeFn>>>;

/// The tool's end: send commands from any thread.
#[derive(Clone)]
pub struct AutomationClient {
    tx: Sender<AutoJob>,
    wake: WakeSlot,
}

impl core::fmt::Debug for AutomationClient {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("AutomationClient")
    }
}

impl AutomationClient {
    /// Queue a command, wake the host and wait up to `timeout` for its answer.
    pub fn call(&self, cmd: AutoCmd, timeout: Duration) -> Result<AutoReply, AutoError> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.tx.send((cmd, tx)).map_err(|_| AutoError::Closed)?;
        if let Some(wake) = self.wake.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
            wake();
        }
        match rx.recv_timeout(timeout) {
            Ok(AutoReply::Error(e)) => Err(e),
            Ok(r) => Ok(r),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(AutoError::Timeout),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(AutoError::Closed),
        }
    }
}

/// The host's end. Cloning shares the same queue.
#[derive(Clone)]
pub struct AutomationPort {
    rx: Arc<Mutex<Receiver<AutoJob>>>,
    wake: WakeSlot,
}

impl core::fmt::Debug for AutomationPort {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("AutomationPort")
    }
}

impl AutomationPort {
    /// Install how a client wakes the host's loop (the host calls this once).
    pub fn set_waker(&self, wake: impl Fn() + Send + Sync + 'static) {
        *self.wake.lock().unwrap_or_else(|p| p.into_inner()) = Some(Box::new(wake));
    }

    /// Every queued job, oldest first.
    pub fn drain(&self) -> Vec<AutoJob> {
        let rx = self.rx.lock().unwrap_or_else(|p| p.into_inner());
        let mut out = Vec::new();
        while let Ok(job) = rx.try_recv() {
            out.push(job);
        }
        out
    }
}

/// A connected client/port pair.
pub fn channel() -> (AutomationClient, AutomationPort) {
    let (tx, rx) = mpsc::channel();
    let wake: WakeSlot = Arc::new(Mutex::new(None));
    (
        AutomationClient { tx, wake: Arc::clone(&wake) },
        AutomationPort { rx: Arc::new(Mutex::new(rx)), wake },
    )
}

// ── Host-side helpers shared by the native and headless hosts ────────────────

/// The window a command means: the given one if open, else the focused
/// window, else the first.
pub(crate) fn resolve_window<S: Spec, A: App<Spec = S>>(rt: &Runtime<S, A>, win: Option<WindowId>) -> Option<WindowId> {
    let snap = rt.snapshot();
    match win {
        Some(w) => snap.window(w).map(|v| v.id),
        None => snap
            .windows
            .iter()
            .find(|w| w.focused)
            .or_else(|| snap.windows.first())
            .map(|w| w.id),
    }
}

/// Every open window; `dpr_of` gives the host's scale for a window.
pub(crate) fn windows<S: Spec, A: App<Spec = S>>(rt: &Runtime<S, A>, dpr_of: impl Fn(WindowId) -> f64) -> Vec<AutoWindow> {
    rt.snapshot()
        .windows
        .iter()
        .map(|w| AutoWindow {
            id: w.id,
            key: w.key.as_str().to_string(),
            viewport: w.geometry.viewport,
            dpr: dpr_of(w.id),
            focused: w.focused,
        })
        .collect()
}

/// The tree of `win` at its last composed frame.
pub(crate) fn tree<S: Spec, A: App<Spec = S>>(rt: &Runtime<S, A>, win: WindowId) -> AutoTree {
    let snap = rt.snapshot();
    let (hovered, focused) = snap
        .window(win)
        .map(|w| (w.input.hovered.clone(), w.input.focused.clone()))
        .unwrap_or_default();
    AutoTree {
        window: win,
        a11y: rt.a11y_tree(win),
        widgets: rt.widget_snapshot(win),
        hovered,
        focused,
    }
}

/// Paint `win` once more into a tiny-skia pixmap and encode it as PNG.
/// `background` is the window clear colour (`0xRRGGBBAA`).
pub(crate) fn offscreen_shot<S: Spec, A: App<Spec = S>>(
    rt: &mut Runtime<S, A>,
    win: WindowId,
    dpr: f64,
    background: u32,
) -> Result<AutoShot, AutoError> {
    use uzor::render::{Painter, ShapeHelpers};
    let vp = rt.snapshot().window(win).map(|w| w.geometry.viewport).ok_or(AutoError::NoWindow)?;
    let dpr = if dpr > 0.0 { dpr } else { 1.0 };
    let w = (vp.width * dpr).round().max(1.0) as u32;
    let h = (vp.height * dpr).round().max(1.0) as u32;
    let mut ctx = uzor_render_tiny_skia::TinySkiaCpuRenderContext::new(w, h, dpr);
    let [r, g, b] = [(background >> 24) & 0xff, (background >> 16) & 0xff, (background >> 8) & 0xff];
    ctx.set_fill_color(&format!("#{r:02x}{g:02x}{b:02x}"));
    ctx.fill_rect(0.0, 0.0, vp.width, vp.height);
    let req = FrameRequest {
        window: win,
        size: SizePx::new(w, h),
        dpr,
        background,
        regions: Default::default(),
        invalidations: Default::default(),
    };
    rt.paint(&req, &mut ctx);
    let png = encode_rgba_png(ctx.pixels(), ctx.width(), ctx.height())
        .ok_or_else(|| AutoError::Failed("png encode failed".into()))?;
    Ok(AutoShot { window: win, png, width: ctx.width(), height: ctx.height(), source: "offscreen" })
}

/// Encode straight or premultiplied RGBA8 as an opaque-safe PNG.
pub fn encode_rgba_png(pixels: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    if pixels.len() < (width as usize) * (height as usize) * 4 {
        return None;
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, width, height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut wr = enc.write_header().ok()?;
        wr.write_image_data(&pixels[..(width as usize) * (height as usize) * 4]).ok()?;
    }
    Some(out)
}

/// Answer every non-input command against the runtime (the parts no OS is
/// needed for). Input and window-readback screenshots are the host's.
pub(crate) fn answer_plain<S: Spec, A: App<Spec = S>>(
    rt: &mut Runtime<S, A>,
    cmd: &AutoCmd,
    dpr_of: &dyn Fn(WindowId) -> f64,
    background_of: &dyn Fn(WindowId) -> u32,
) -> Option<AutoReply> {
    Some(match cmd {
        AutoCmd::Windows => AutoReply::Windows(windows(rt, dpr_of)),
        AutoCmd::Tree { win } => match resolve_window(rt, *win) {
            Some(w) => AutoReply::Tree(tree(rt, w)),
            None => AutoReply::Error(AutoError::NoWindow),
        },
        AutoCmd::AppState => AutoReply::State(rt.app().automation_state()),
        AutoCmd::Screenshot { win, source: ShotSource::Offscreen } => match resolve_window(rt, *win) {
            Some(w) => match offscreen_shot(rt, w, dpr_of(w), background_of(w)) {
                Ok(s) => AutoReply::Shot(s),
                Err(e) => AutoReply::Error(e),
            },
            None => AutoReply::Error(AutoError::NoWindow),
        },
        AutoCmd::Screenshot { .. } | AutoCmd::Input { .. } => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handle::{IntentCx, OverlayCx, PanelCx};
    use crate::host::HeadlessHost;
    use crate::runtime::RuntimeConfig;
    use crate::types::bus::{KeyInput, KeyState, PointerInput};
    use crate::types::command::{AppCommand, DockTarget, LayoutCmd};
    use crate::types::intent::Intent;
    use crate::types::overlay_model::OverlayModel;
    use crate::types::spec::PanelHome;
    use crate::types::window::Point;
    use uzor::input::{KeyCode, ModifierKeys, MouseButton};
    use uzor::layout::docking::DockPanel;

    #[derive(Clone, Debug)]
    struct P;
    impl DockPanel for P {
        fn title(&self) -> &str {
            "auto"
        }
        fn type_id(&self) -> &'static str {
            "auto"
        }
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    enum Ov {}
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    enum Act {}
    struct TS;
    impl Spec for TS {
        type Panel = P;
        type Overlay = Ov;
        type Action = Act;
        fn decode_panel(_h: PanelHome, _t: &str) -> Option<P> {
            Some(P)
        }
    }

    #[derive(Default)]
    struct Form {
        clicks: u32,
    }

    impl App for Form {
        type Spec = TS;
        fn overlay_model(&self, _w: WindowId, id: Ov) -> OverlayModel {
            match id {}
        }
        fn panel(&mut self, cx: &mut PanelCx<'_, TS>) {
            use crate::widgets::{kit, lm};
            lm::button("go", Rect::new(20.0, 20.0, 120.0, 32.0)).text("Go").bind_count(&mut self.clicks).build(cx);
            kit::field("nick", Rect::new(20.0, 70.0, 200.0, 28.0)).name("Nick").build(cx);
            kit::field("token", Rect::new(20.0, 110.0, 200.0, 28.0)).name("Token").masked(true).build(cx);
        }
        fn overlay_body(&mut self, _cx: &mut OverlayCx<'_, TS>) {}
        fn intent(&mut self, _i: Intent<TS>, _cx: &mut IntentCx<'_, TS>) {}
        fn automation_state(&self) -> Option<String> {
            Some(format!("{{\"clicks\":{}}}", self.clicks))
        }
    }

    fn setup() -> (HeadlessHost<TS, Form>, AutomationClient, AutomationPort) {
        let (mut host, handle) = HeadlessHost::new(Form::default(), RuntimeConfig::default());
        let win = host.open_window(SizePx::new(400, 300), 1.0);
        handle
            .dispatch(AppCommand::Layout(LayoutCmd::OpenPanel { win, panel: P, at: DockTarget::Root }))
            .expect("dispatch");
        host.advance(0.017);
        host.advance(0.017);
        let (client, port) = channel();
        (host, client, port)
    }

    /// Run `cmd` from another thread while the host serves, like a real tool.
    fn call(host: &mut HeadlessHost<TS, Form>, client: &AutomationClient, port: &AutomationPort, cmd: AutoCmd) -> Result<AutoReply, AutoError> {
        let c = client.clone();
        let t = std::thread::spawn(move || c.call(cmd, Duration::from_secs(5)));
        while !t.is_finished() {
            host.serve_automation(port, 0.017);
            std::thread::sleep(Duration::from_millis(1));
        }
        t.join().expect("client thread")
    }

    fn tree_of(host: &mut HeadlessHost<TS, Form>, c: &AutomationClient, p: &AutomationPort) -> AutoTree {
        match call(host, c, p, AutoCmd::Tree { win: None }) {
            Ok(AutoReply::Tree(t)) => t,
            other => panic!("tree: {other:?}"),
        }
    }

    fn click(host: &mut HeadlessHost<TS, Form>, c: &AutomationClient, p: &AutomationPort, sel: &str) {
        let t = tree_of(host, c, p);
        let b = uzor::a11y::query(&t.a11y, sel).map(|n| n.bounds).or_else(|_| {
            t.widgets.iter().find(|w| sel.strip_prefix('#') == Some(w.id.as_str())).map(|w| w.rect).ok_or(())
        });
        let b = b.unwrap_or_else(|_| panic!("selector {sel}"));
        let pos = Point::new(b.x + b.width / 2.0, b.y + b.height / 2.0);
        let mods = ModifierKeys::default();
        for ev in [
            InputEvent::Pointer(PointerInput::Moved { pos, mods }),
            InputEvent::Pointer(PointerInput::Down { pos, button: MouseButton::Left, mods }),
            InputEvent::Pointer(PointerInput::Up { pos, button: MouseButton::Left, mods }),
        ] {
            assert!(matches!(call(host, c, p, AutoCmd::Input { win: None, events: vec![ev] }), Ok(AutoReply::InputDone { .. })));
        }
        call(host, c, p, AutoCmd::Input { win: None, events: vec![] }).expect("settle");
    }

    fn type_text(host: &mut HeadlessHost<TS, Form>, c: &AutomationClient, p: &AutomationPort, text: &str) {
        let mut events = Vec::new();
        for ch in text.chars() {
            for state in [KeyState::Down, KeyState::Up] {
                events.push(InputEvent::Key(KeyInput {
                    code: KeyCode::Unknown,
                    text: (state == KeyState::Down).then(|| ch.to_string()),
                    state,
                    mods: ModifierKeys::default(),
                }));
            }
        }
        call(host, c, p, AutoCmd::Input { win: None, events }).expect("type");
    }

    #[test]
    fn injected_click_reaches_the_app() {
        let (mut host, c, p) = setup();
        match call(&mut host, &c, &p, AutoCmd::Windows) {
            Ok(AutoReply::Windows(w)) => assert_eq!(w.len(), 1),
            other => panic!("windows: {other:?}"),
        }
        click(&mut host, &c, &p, "#go");
        assert_eq!(host.runtime().app().clicks, 1);
        match call(&mut host, &c, &p, AutoCmd::AppState) {
            Ok(AutoReply::State(Some(s))) => assert_eq!(s, "{\"clicks\":1}"),
            other => panic!("state: {other:?}"),
        }
    }

    #[test]
    fn typed_text_shows_in_tree_and_secret_stays_masked() {
        let (mut host, c, p) = setup();
        click(&mut host, &c, &p, "textinput[name=Nick]");
        type_text(&mut host, &c, &p, "lavr");
        click(&mut host, &c, &p, "#token");
        type_text(&mut host, &c, &p, "s3cret");
        let t = tree_of(&mut host, &c, &p);
        let nick = uzor::a11y::query(&t.a11y, "#nick").expect("nick");
        assert_eq!(nick.value.as_deref(), Some("lavr"));
        let tok = uzor::a11y::query(&t.a11y, "#token").expect("token");
        assert!(tok.masked);
        assert_eq!(tok.value.as_deref(), Some("\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}"));
        assert!(tok.focused);
        assert!(!format!("{t:?}").contains("s3cret"));
        assert!(t.widgets.iter().any(|w| w.id.as_str() == "go"));
    }

    #[test]
    fn offscreen_screenshot_is_a_png_of_the_window() {
        let (mut host, c, p) = setup();
        for source in [ShotSource::Auto, ShotSource::Offscreen] {
            match call(&mut host, &c, &p, AutoCmd::Screenshot { win: None, source }) {
                Ok(AutoReply::Shot(s)) => {
                    assert_eq!(&s.png[..8], b"\x89PNG\r\n\x1a\n");
                    assert_eq!((s.width, s.height), (400, 300));
                    assert_eq!(s.source, "offscreen");
                    let dec = png::Decoder::new(std::io::Cursor::new(&s.png));
                    let mut r = dec.read_info().expect("png header");
                    let mut buf = vec![0; r.output_buffer_size()];
                    let info = r.next_frame(&mut buf).expect("png frame");
                    let px = &buf[..info.buffer_size()];
                    // Something was painted: not every pixel is the clear colour.
                    assert!(px.chunks(4).any(|q| q != &px[..4]));
                }
                other => panic!("screenshot: {other:?}"),
            }
        }
    }

    #[test]
    fn unknown_window_and_dead_host_are_errors() {
        let (mut host, c, p) = setup();
        assert_eq!(
            call(&mut host, &c, &p, AutoCmd::Tree { win: Some(WindowId(99)) }).unwrap_err(),
            AutoError::NoWindow
        );
        drop(p);
        drop(host);
        assert_eq!(c.call(AutoCmd::Windows, Duration::from_millis(50)).unwrap_err(), AutoError::Closed);
    }
}
