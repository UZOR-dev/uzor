//! Automation port (feature `automation`): lets a tool outside the event
//! loop drive the app the way a user does — read the accessibility tree,
//! inject input, take screenshots — without the host knowing any transport.
//!
//! A tool holds an [`AutomationClient`] and calls [`AutomationClient::call`]
//! from its own thread (or awaits [`AutomationClient::call_async`], which is
//! what the in-page web bridge does: no threads there); the host owns the
//! matching [`AutomationPort`], is woken through the waker it installs, and
//! answers every [`AutoCmd`] on the loop thread. Input replies come after the
//! frame that follows the injection was painted, so a caller reading the tree
//! next sees its effect. [`AutoCmd::Frame`] waits for the next painted frame,
//! so callers check conditions once per drawn frame instead of sleeping.
//!
//! Hosts also report the input they push (from the user or injected) to an
//! [input tap](AutomationPort::tap_on) when one is installed — the recorder
//! of dev control uses it.
//!
//! The feature is off by default: release builds that do not name it carry
//! none of this code.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll};
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
    /// The host's painted-frame counter, now (forces nothing).
    Frames,
    /// Answer once the host has painted a frame past `after` (its frame
    /// counter is above it). Idle hosts paint nothing: callers bound the
    /// wait with their own timeout.
    Frame {
        /// The last frame the caller saw.
        after: u64,
    },
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
    /// The painted-frame counter (for [`AutoCmd::Frames`] / [`AutoCmd::Frame`]).
    Frame {
        /// Frames painted so far.
        frame: u64,
    },
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
pub type AutoJob = (AutoCmd, AutoReplyTx);

// ── Reply slot ───────────────────────────────────────────────────────────────

#[derive(Default)]
struct SlotState {
    reply: Option<AutoReply>,
    done: bool,
    waker: Option<std::task::Waker>,
}

#[derive(Default)]
struct Slot {
    state: Mutex<SlotState>,
    cv: Condvar,
}

impl Slot {
    fn lock(&self) -> std::sync::MutexGuard<'_, SlotState> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn finish(&self, reply: Option<AutoReply>) {
        let waker = {
            let mut st = self.lock();
            if st.done {
                return;
            }
            st.done = true;
            st.reply = reply;
            st.waker.take()
        };
        self.cv.notify_all();
        if let Some(w) = waker {
            w.wake();
        }
    }
}

/// Where the host sends one answer (a one-shot slot a caller can block on
/// or await). Dropping it unanswered tells the caller the host is gone.
pub struct AutoReplyTx(Arc<Slot>);

impl core::fmt::Debug for AutoReplyTx {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("AutoReplyTx")
    }
}

impl AutoReplyTx {
    /// Deliver the answer. `Err` gives it back when the caller stopped
    /// waiting (timed out or dropped its future).
    pub fn send(self, reply: AutoReply) -> Result<(), AutoReply> {
        if self.is_abandoned() {
            return Err(reply);
        }
        self.0.finish(Some(reply));
        Ok(())
    }

    /// The caller no longer waits for this answer.
    pub fn is_abandoned(&self) -> bool {
        Arc::strong_count(&self.0) == 1
    }
}

impl Drop for AutoReplyTx {
    fn drop(&mut self) {
        self.0.finish(None);
    }
}

/// Milliseconds on a monotonic clock (`performance`-free on the web: the
/// wall clock there, which is enough for timeouts).
pub fn now_ms() -> f64 {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    {
        js_sys::Date::now()
    }
    #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
    {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        START.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() * 1000.0
    }
}

/// The answer to one [`AutomationClient::call_async`]: resolves with the
/// host's reply, or `Timeout` once the deadline passes when polled (the
/// web bridge polls every animation frame), or `Closed`.
pub struct ReplyFuture {
    slot: Option<Arc<Slot>>,
    early: Option<AutoError>,
    deadline_ms: f64,
}

impl ReplyFuture {
    /// The deadline on the [`now_ms`] clock.
    pub fn deadline_ms(&self) -> f64 {
        self.deadline_ms
    }
}

impl Future for ReplyFuture {
    type Output = Result<AutoReply, AutoError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(e) = self.early.take() {
            return Poll::Ready(Err(e));
        }
        let Some(slot) = self.slot.clone() else { return Poll::Ready(Err(AutoError::Closed)) };
        let mut st = slot.lock();
        if st.done {
            let out = match st.reply.take() {
                Some(AutoReply::Error(e)) => Err(e),
                Some(r) => Ok(r),
                None => Err(AutoError::Closed),
            };
            drop(st);
            self.slot = None;
            return Poll::Ready(out);
        }
        if now_ms() >= self.deadline_ms {
            drop(st);
            self.slot = None;
            return Poll::Ready(Err(AutoError::Timeout));
        }
        st.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

/// Resolves once [`now_ms`] reaches a deadline (checked when polled; the
/// native [`block_on`] polls every few milliseconds, the web bridge every
/// animation frame).
pub struct Delay(f64);

impl Delay {
    /// Ready after `ms` milliseconds.
    pub fn ms(ms: f64) -> Self {
        Self(now_ms() + ms.max(0.0))
    }
}

impl Future for Delay {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if now_ms() >= self.0 {
            Poll::Ready(())
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

/// Run a future to completion on this thread (native): parks between polls,
/// woken by replies, and re-polls every 2 ms for deadlines.
#[cfg(not(target_arch = "wasm32"))]
pub fn block_on<F: Future>(fut: F) -> F::Output {
    struct Unpark(std::thread::Thread);
    impl std::task::Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = std::task::Waker::from(Arc::new(Unpark(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut fut = std::pin::pin!(fut);
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::park_timeout(Duration::from_millis(2));
    }
}

// ── Input tap ────────────────────────────────────────────────────────────────

/// One input event a host pushed into the runtime, with the window's
/// widgets at the last composed frame (for turning points into selectors).
#[derive(Clone, Debug)]
pub struct TapEvent {
    /// The window.
    pub window: WindowId,
    /// The event as pushed.
    pub event: InputEvent,
    /// The window's tree (pointer presses, wheels, keys and IME commits only).
    pub tree: Option<AutoTree>,
}

type TapFn = Box<dyn Fn(TapEvent) + Send + Sync>;

#[derive(Default)]
struct Tap {
    on: AtomicBool,
    sink: Mutex<Option<TapFn>>,
}

type WakeFn = Box<dyn Fn() + Send + Sync>;
type WakeSlot = Arc<Mutex<Option<WakeFn>>>;

/// The tool's end: send commands from any thread.
#[derive(Clone)]
pub struct AutomationClient {
    tx: Sender<AutoJob>,
    wake: WakeSlot,
    tap: Arc<Tap>,
}

impl core::fmt::Debug for AutomationClient {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("AutomationClient")
    }
}

impl AutomationClient {
    /// Queue a command, wake the host and wait up to `timeout` for its answer.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn call(&self, cmd: AutoCmd, timeout: Duration) -> Result<AutoReply, AutoError> {
        block_on(self.call_async(cmd, timeout))
    }

    /// Queue a command and wake the host; the future resolves with its
    /// answer, or `Timeout` after `timeout`.
    pub fn call_async(&self, cmd: AutoCmd, timeout: Duration) -> ReplyFuture {
        let deadline_ms = now_ms() + timeout.as_secs_f64() * 1000.0;
        let slot = Arc::new(Slot::default());
        if self.tx.send((cmd, AutoReplyTx(Arc::clone(&slot)))).is_err() {
            return ReplyFuture { slot: None, early: Some(AutoError::Closed), deadline_ms };
        }
        if let Some(wake) = self.wake.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
            wake();
        }
        ReplyFuture { slot: Some(slot), early: None, deadline_ms }
    }

    /// Install (or with `None` remove) the input tap; while on, the host
    /// reports every input it pushes (see [`TapEvent`]).
    pub fn set_tap(&self, sink: Option<Box<dyn Fn(TapEvent) + Send + Sync>>) {
        let on = sink.is_some();
        *self.tap.sink.lock().unwrap_or_else(|p| p.into_inner()) = sink;
        self.tap.on.store(on, Ordering::SeqCst);
    }
}

/// The host's end. Cloning shares the same queue.
#[derive(Clone)]
pub struct AutomationPort {
    rx: Arc<Mutex<Receiver<AutoJob>>>,
    wake: WakeSlot,
    tap: Arc<Tap>,
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

    /// An input tap is installed: hosts report pushed input through
    /// [`tap_input`].
    pub fn tap_on(&self) -> bool {
        self.tap.on.load(Ordering::Relaxed)
    }

    fn tap_send(&self, ev: TapEvent) {
        if let Some(sink) = self.tap.sink.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
            sink(ev);
        }
    }
}

/// A connected client/port pair.
pub fn channel() -> (AutomationClient, AutomationPort) {
    let (tx, rx) = mpsc::channel();
    let wake: WakeSlot = Arc::new(Mutex::new(None));
    let tap = Arc::new(Tap::default());
    (
        AutomationClient { tx, wake: Arc::clone(&wake), tap: Arc::clone(&tap) },
        AutomationPort { rx: Arc::new(Mutex::new(rx)), wake, tap },
    )
}

/// Report one pushed input event to the port's tap (a no-op unless a tap is
/// on). Hosts call it for every event they push, user's or injected.
pub(crate) fn tap_input<S: Spec, A: App<Spec = S>>(port: &AutomationPort, rt: &Runtime<S, A>, win: WindowId, event: &InputEvent) {
    use crate::types::bus::{ImeInput, KeyState, PointerInput};
    if !port.tap_on() {
        return;
    }
    let wants_tree = match event {
        InputEvent::Pointer(PointerInput::Down { .. } | PointerInput::Up { .. }) | InputEvent::Wheel(_) => true,
        InputEvent::Key(k) => k.state == KeyState::Down,
        InputEvent::Ime(ImeInput::Commit(_)) => true,
        _ => false,
    };
    if !wants_tree {
        return;
    }
    port.tap_send(TapEvent { window: win, event: event.clone(), tree: Some(tree(rt, win)) });
}

/// Painted-frame bookkeeping a host keeps for [`AutoCmd::Frames`] /
/// [`AutoCmd::Frame`].
#[derive(Default)]
pub(crate) struct FrameWaits {
    /// Frames painted so far.
    pub frames: u64,
    waiting: Vec<(u64, AutoReplyTx)>,
}

impl FrameWaits {
    /// Answer `Frames` now, or `Frame` now or once a later frame is painted.
    /// Returns the job back when it is neither.
    pub fn take(&mut self, cmd: AutoCmd, reply: AutoReplyTx) -> Option<(AutoCmd, AutoReplyTx)> {
        match cmd {
            AutoCmd::Frames => {
                let _ = reply.send(AutoReply::Frame { frame: self.frames });
                None
            }
            AutoCmd::Frame { after } => {
                if self.frames > after {
                    let _ = reply.send(AutoReply::Frame { frame: self.frames });
                } else {
                    self.waiting.push((after, reply));
                }
                None
            }
            other => Some((other, reply)),
        }
    }

    /// One more frame was painted.
    pub fn painted(&mut self) {
        self.frames += 1;
        let frames = self.frames;
        let mut keep = Vec::new();
        for (after, tx) in std::mem::take(&mut self.waiting) {
            if tx.is_abandoned() {
                continue;
            }
            if frames > after {
                let _ = tx.send(AutoReply::Frame { frame: frames });
            } else {
                keep.push((after, tx));
            }
        }
        self.waiting = keep;
    }

    /// Someone waits for a frame (hosts that only paint on demand keep
    /// time moving while this holds).
    pub fn anyone_waiting(&mut self) -> bool {
        self.waiting.retain(|(_, tx)| !tx.is_abandoned());
        !self.waiting.is_empty()
    }
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
        AutoCmd::Screenshot { .. } | AutoCmd::Input { .. } | AutoCmd::Frames | AutoCmd::Frame { .. } => return None,
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
