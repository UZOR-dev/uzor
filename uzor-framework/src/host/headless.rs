//! The headless host (design §9.1): deterministic, clock-injected, no OS.
//!
//! Time never moves on its own — [`HeadlessHost::advance`] adds `dt` to
//! the host clock and runs exactly one tick (plus the settling tick the
//! host's own echo answers cause), painting every due frame request into
//! a recording context. Host-bound commands that need an OS answer are
//! answered automatically: `Spawn` gets a `Created` echo, `Close` gets
//! `Destroyed`. Every §9.2 script runs against this host.

use std::sync::Arc;

use uzor::testing::RecordingRenderContext;

use crate::runtime::{Runtime, RuntimeConfig, TickOutput};
use crate::types::bus::{HostCaps, InputEnvelope, InputEvent, WindowInput};
use crate::types::command::{AppCommand, WindowCmd};
use crate::types::ids::{Seconds, WindowId};
use crate::types::snapshot::VisualSnapshot;
use crate::types::spec::Spec;
use crate::types::window::{SizePx, WindowCommand, WindowSpec};
use crate::{App, Handle, Waker};

/// A runtime driven by hand: no event loop, no wall clock, no GPU.
pub struct HeadlessHost<S: Spec, A: App<Spec = S>> {
    rt: Runtime<S, A>,
    handle: Handle<S>,
    now: Seconds,
    /// Capabilities reported in every `Created` echo.
    pub caps: HostCaps,
    /// Device pixel ratio reported for auto-answered spawned windows.
    pub dpr: f64,
    /// The paint recorder of the last `advance` (draw ops are inspectable
    /// through it; it is reused, so it accumulates — call
    /// [`RecordingRenderContext`] accessors per test section).
    pub ctx: RecordingRenderContext,
    last: TickOutput,
    command_log: Vec<(WindowId, WindowCommand)>,
    #[cfg(feature = "automation")]
    auto: HeadlessAuto,
    /// Paint target of `advance` instead of `ctx` (the live loop: real text
    /// metrics, see [`run_headless`]).
    #[cfg(feature = "automation")]
    live_ctx: Option<Box<dyn uzor::render::RenderContext>>,
}

/// Automation bookkeeping of the headless host.
#[cfg(feature = "automation")]
#[derive(Default)]
struct HeadlessAuto {
    /// The port last served (its input tap sees pushed input).
    port: Option<crate::automation::AutomationPort>,
    waits: crate::automation::FrameWaits,
    /// Last clear colour per window.
    background: std::collections::HashMap<WindowId, u32>,
}

impl<S: Spec, A: App<Spec = S>> HeadlessHost<S, A> {
    /// A host and the app's handle, with a no-op waker (design §9.1).
    pub fn new(app: A, cfg: RuntimeConfig) -> (Self, Handle<S>) {
        Self::with_waker(app, cfg, Waker::noop())
    }

    /// A host whose app handle wakes `waker` on every dispatch (a live
    /// loop sleeps until then; see [`run_headless`]).
    pub fn with_waker(app: A, cfg: RuntimeConfig, waker: Waker) -> (Self, Handle<S>) {
        let (rt, handle) = Runtime::new(app, cfg, waker);
        let host = Self {
            rt,
            handle: handle.clone(),
            now: Seconds::ZERO,
            caps: HostCaps {
                multi_window: true,
                os_resize_bezel: false,
                clipboard_async: false,
            },
            dpr: 1.0,
            ctx: RecordingRenderContext::new(),
            last: TickOutput::default(),
            command_log: Vec::new(),
            #[cfg(feature = "automation")]
            auto: HeadlessAuto::default(),
            #[cfg(feature = "automation")]
            live_ctx: None,
        };
        (host, handle)
    }

    /// Open a window and run the ticks that settle its creation (the
    /// `Spawn` command is auto-answered with a `Created` echo carrying
    /// `size` and `dpr`). Returns the engine-assigned id.
    pub fn open_window(&mut self, size: SizePx, dpr: f64) -> WindowId {
        let n = self.snapshot().windows.len();
        let key = format!("win-{n}");
        let mut spec = WindowSpec::new(key, "headless", size);
        let _ = &mut spec; // spec travels whole; dpr rides the echo.
        self.handle
            .dispatch(AppCommand::Window(WindowCmd::Open(spec)))
            .unwrap_or_else(|e| panic!("headless inbox: {e}"));
        self.dpr = dpr;
        self.advance(0.0);
        self.snapshot()
            .windows
            .last()
            .unwrap_or_else(|| panic!("window did not open"))
            .id
    }

    /// Queue one input event for the next tick (hop 1 of §4.4).
    pub fn input(&mut self, win: WindowId, event: InputEvent) {
        #[cfg(feature = "automation")]
        if let Some(port) = &self.auto.port {
            crate::automation::tap_input(port, &self.rt, win, &event);
        }
        self.rt.push_input(InputEnvelope {
            window: win,
            t: self.now,
            event,
        });
    }

    /// Move the host clock by `dt` seconds, run one tick (plus one
    /// settling tick when the tick's host-bound commands needed an echo
    /// answer), and paint every due frame request. The output of the whole
    /// advance is kept for [`HeadlessHost::window_commands`].
    pub fn advance(&mut self, dt: f64) -> &TickOutput {
        self.now = self.now.after(Seconds(dt));
        let mut out = self.rt.tick(self.now);

        // Answer the commands only an OS could: spawns get created with
        // the host's caps, closes get destroyed.
        let mut echoes = Vec::new();
        for (win, cmd) in &out.window_commands {
            match cmd {
                WindowCommand::Spawn(spec) => echoes.push((
                    *win,
                    WindowInput::Created {
                        size: spec.inner_size,
                        dpr: self.dpr,
                        // A known position: expand gutters and drag-out
                        // need a known outer rect.
                        position: Some((100, 100)),
                        caps: self.caps,
                    },
                )),
                WindowCommand::Close => echoes.push((*win, WindowInput::Destroyed)),
                _ => {}
            }
        }
        if !echoes.is_empty() {
            for (win, event) in echoes {
                self.rt.push_input(InputEnvelope {
                    window: win,
                    t: self.now,
                    event: InputEvent::Window(event),
                });
            }
            let more = self.rt.tick(self.now);
            out.window_commands.extend(more.window_commands);
            out.frames.extend(more.frames);
            out.wake = earliest(out.wake, more.wake);
        }

        let frames = std::mem::take(&mut out.frames);
        for req in &frames {
            #[cfg(feature = "automation")]
            match self.live_ctx.as_deref_mut() {
                Some(ctx) => self.rt.paint(req, ctx),
                None => self.rt.paint(req, &mut self.ctx),
            }
            #[cfg(not(feature = "automation"))]
            self.rt.paint(req, &mut self.ctx);
            #[cfg(feature = "automation")]
            {
                self.auto.background.insert(req.window, req.background);
                self.auto.waits.painted();
            }
        }
        out.frames = frames;
        self.command_log.extend(out.window_commands.iter().cloned());
        self.last = out;
        &self.last
    }

    /// Paint window `win` once into `ctx` at its viewport size and `dpr`, outside the
    /// tick (frame dumps, screenshots, goldens). The caller sizes `ctx`
    /// (viewport × `dpr`) and clears it. Returns the viewport size in logical pixels,
    /// or `None` for an unknown window.
    pub fn paint_window(&mut self, win: WindowId, dpr: f64, ctx: &mut dyn uzor::render::RenderContext) -> Option<(f64, f64)> {
        let snap = self.snapshot();
        let vp = snap.window(win)?.geometry.viewport;
        drop(snap);
        let size = SizePx::new(vp.width.round().max(1.0) as u32, vp.height.round().max(1.0) as u32);
        let req = crate::FrameRequest {
            window: win,
            size,
            dpr,
            background: 0,
            regions: Default::default(),
            invalidations: Default::default(),
        };
        self.rt.paint(&req, ctx);
        Some((vp.width, vp.height))
    }

    /// Answer every job queued on `port` (feature `automation`): input is
    /// fed and followed by one `advance(dt)` before its reply; screenshots
    /// are offscreen (there is no window to read back). While someone waits
    /// for a painted frame ([`crate::automation::AutoCmd::Frame`]) the
    /// clock moves by `dt` per call.
    #[cfg(feature = "automation")]
    pub fn serve_automation(&mut self, port: &crate::automation::AutomationPort, dt: f64) {
        let jobs = self.serve_jobs(port, &mut |_| dt);
        if jobs == 0 && self.auto.waits.anyone_waiting() {
            self.advance(dt);
        }
    }

    /// Answer queued jobs; `dt_of` gives the clock step for each input's
    /// advance. Returns the number of jobs.
    #[cfg(feature = "automation")]
    fn serve_jobs(&mut self, port: &crate::automation::AutomationPort, dt_of: &mut dyn FnMut(&Self) -> f64) -> usize {
        use crate::automation::{answer_plain, resolve_window, AutoCmd, AutoError, AutoReply, ShotSource};
        if self.auto.port.is_none() {
            self.auto.port = Some(port.clone());
        }
        let jobs = port.drain();
        let n = jobs.len();
        for (cmd, reply) in jobs {
            let Some((cmd, reply)) = self.auto.waits.take(cmd, reply) else { continue };
            let dpr = self.dpr;
            let bgs = self.auto.background.clone();
            let bg_of = move |w: WindowId| bgs.get(&w).copied().unwrap_or(0xffffffff);
            let answer = match &cmd {
                AutoCmd::Input { win, events } => match resolve_window(&self.rt, *win) {
                    Some(w) => {
                        for ev in events.clone() {
                            self.input(w, ev);
                        }
                        let dt = dt_of(self);
                        self.advance(dt);
                        AutoReply::InputDone { window: w, frame: self.auto.waits.frames }
                    }
                    None => AutoReply::Error(AutoError::NoWindow),
                },
                AutoCmd::Screenshot { win, .. } => {
                    let c = AutoCmd::Screenshot { win: *win, source: ShotSource::Offscreen };
                    answer_plain(&mut self.rt, &c, &|_| dpr, &bg_of).unwrap_or(AutoReply::Error(AutoError::Unsupported("screenshot")))
                }
                other => answer_plain(&mut self.rt, other, &|_| dpr, &bg_of).unwrap_or(AutoReply::Error(AutoError::Unsupported("command"))),
            };
            let _ = reply.send(answer);
        }
        n
    }

    /// The wake hint of the last `advance`.
    pub fn last_wake(&self) -> crate::types::frame::Wake {
        self.last.wake
    }

    /// The last published snapshot.
    pub fn snapshot(&self) -> Arc<VisualSnapshot<S>> {
        self.rt.snapshot()
    }

    /// The host-bound window commands of the last `advance`.
    pub fn window_commands(&self) -> &[(WindowId, WindowCommand)] {
        &self.last.window_commands
    }

    /// Every host-bound window command since construction, in order (the
    /// executor's own log — assertions over a whole script read this, not
    /// the last tick's slice).
    pub fn command_log(&self) -> &[(WindowId, WindowCommand)] {
        &self.command_log
    }

    /// The runtime (tests drive app-level tools through it).
    pub fn runtime(&self) -> &Runtime<S, A> {
        &self.rt
    }

    /// The runtime, mutably.
    pub fn runtime_mut(&mut self) -> &mut Runtime<S, A> {
        &mut self.rt
    }
}

/// The sooner of two wake hints (`Immediate` < `At(t)` < `Idle`).
fn earliest(
    a: crate::types::frame::Wake,
    b: crate::types::frame::Wake,
) -> crate::types::frame::Wake {
    use crate::types::frame::Wake;
    match (a, b) {
        (Wake::Immediate, _) | (_, Wake::Immediate) => Wake::Immediate,
        (Wake::At(x), Wake::At(y)) => Wake::At(if x.get() <= y.get() { x } else { y }),
        (Wake::At(x), Wake::Idle) | (Wake::Idle, Wake::At(x)) => Wake::At(x),
        (Wake::Idle, Wake::Idle) => Wake::Idle,
    }
}

/// Run `app` live with no window and no display: a real-time loop on a
/// [`HeadlessHost`] that serves `port` (feature `automation`, native
/// only). Windows the app opens are auto-created at their spec size and
/// `dpr`; frames paint into a recorder that is cleared each turn;
/// screenshots are offscreen (tiny-skia). Returns when the app exits or
/// its last window closes.
#[cfg(all(feature = "automation", not(target_arch = "wasm32")))]
pub fn run_headless<S: Spec, A: App<Spec = S>>(
    app: A,
    cfg: RuntimeConfig,
    dpr: f64,
    port: crate::automation::AutomationPort,
) {
    use crate::types::frame::Wake;
    use std::sync::{Condvar, Mutex};
    use std::time::{Duration, Instant};

    #[derive(Default)]
    struct Signal {
        woken: Mutex<bool>,
        cv: Condvar,
    }
    impl Signal {
        fn wake(&self) {
            *self.woken.lock().unwrap_or_else(|p| p.into_inner()) = true;
            self.cv.notify_all();
        }
        /// Sleep up to `max` unless woken; true when woken.
        fn wait(&self, max: Option<Duration>) -> bool {
            let mut w = self.woken.lock().unwrap_or_else(|p| p.into_inner());
            if !*w {
                w = match max {
                    Some(d) => self.cv.wait_timeout(w, d).unwrap_or_else(|p| p.into_inner()).0,
                    None => self.cv.wait(w).unwrap_or_else(|p| p.into_inner()),
                };
            }
            std::mem::replace(&mut *w, false)
        }
    }

    // A paced frame: animations tick at most this often.
    const FRAME: f64 = 1.0 / 60.0;
    let sig = Arc::new(Signal::default());
    let s1 = Arc::clone(&sig);
    let (mut host, _handle) = HeadlessHost::with_waker(app, cfg, Waker::new(move || s1.wake()));
    host.dpr = if dpr > 0.0 { dpr } else { 1.0 };
    // Frames paint into a 1x1 tiny-skia context: drawing is clipped away,
    // but text measures with the real fonts, exactly as in the offscreen
    // screenshots (the recorder's zero-wide text would leave layouts that
    // apps cache, such as wrapped messages, collapsed).
    host.live_ctx = Some(Box::new(uzor_render_tiny_skia::TinySkiaCpuRenderContext::new(1, 1, host.dpr)));
    let s2 = Arc::clone(&sig);
    port.set_waker(move || s2.wake());
    let start = Instant::now();
    let clock = move || start.elapsed().as_secs_f64();

    host.advance(0.0);
    host.ctx.ops.clear();
    let mut had_window = !host.snapshot().windows.is_empty();
    let mut last_tick = clock();
    let mut due_now = true;
    let mut want_more = false;
    loop {
        let now = clock();
        let jobs = host.serve_jobs(&port, &mut |h: &HeadlessHost<S, A>| (now - h.now.get()).max(0.0));
        if jobs > 0 {
            last_tick = now;
        }
        let wake = host.last_wake();
        let paint_more = std::mem::take(&mut want_more) || host.rt.take_paint_wants_tick();
        let due = due_now
            || paint_more
            || match wake {
                Wake::Immediate => now - last_tick >= FRAME,
                Wake::At(t) => t.get() <= now,
                Wake::Idle => false,
            };
        if due {
            let dt = (now - host.now.get()).max(0.0);
            host.advance(dt);
            host.ctx.ops.clear();
            last_tick = now;
        }
        due_now = false;
        let windows = host.snapshot().windows.len();
        had_window |= windows > 0;
        if host.rt.should_exit() || (had_window && windows == 0) {
            return;
        }
        let now = clock();
        want_more = host.rt.take_paint_wants_tick();
        let sleep = match host.last_wake() {
            _ if want_more => Some((last_tick + FRAME - now).max(0.0)),
            Wake::Immediate => Some((last_tick + FRAME - now).max(0.0)),
            Wake::At(t) => Some((t.get() - now).max(0.0)),
            Wake::Idle => None,
        };
        if sleep.is_some_and(|d| d <= 0.0) {
            continue;
        }
        if sig.wait(sleep.map(Duration::from_secs_f64)) {
            due_now = true;
        }
    }
}
