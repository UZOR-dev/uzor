//! Native winit `ApplicationHandler` (design §2.3).
//!
//! The host translates events and executes commands; it never decides
//! routing, layout, or intent delivery — those stay in the runtime.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use uzor::RenderBackend;
use uzor_render_hub::{RenderHub, WindowRenderState};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId as WinitWindowId;

use super::clipboard::NativeClipboard;
use super::executor::{self, ExecutorFollowUp};
use super::paint::paint_frame;
use super::screenshot::{capture_screenshot, encode_png};
use super::single_instance::{self, SingleInstanceGuard};
use super::tray::{TrayBuilder, TrayEvent, TrayHandle, TraySpec};
use super::tray_widget::{FlyoutAction, TrayWidgetHandle, TrayWidgets};
use super::window::NativeWindow;
use super::prewarm::{self, PrewarmConfig, Startup};
use super::window_create::{build_hub, create_window_warm, geometry_echo};
use crate::runtime::{Runtime, RuntimeConfig};
use crate::types::bus::{
    HostCaps, HostEvent, InputEnvelope, InputEvent, PointerInput, RenderInfo, WindowInput,
};
use crate::types::error::FrameworkError;
use crate::types::frame::Wake;
use crate::types::ids::{Seconds, WindowId};
use crate::types::window::{RenderCmd, WindowCommand, WindowSpec};
use crate::{App, Spec, Waker};

/// Android destroys the native window of a backgrounded app: its surfaces must go
/// in `suspended` and come back in `resumed`. Elsewhere the surface survives.
const DROP_SURFACES_ON_SUSPEND: bool = cfg!(target_os = "android");

/// User-event payload that wakes the loop from another thread.
#[derive(Debug, Clone, Copy)]
pub enum HostWake {
    /// A handle dispatch arrived; run a tick.
    Wake,
    /// An automation command is queued (feature `automation`).
    #[cfg(feature = "automation")]
    Automation,
}

/// Native host: one runtime, a map of OS windows, a render hub.
pub struct NativeHost<S: Spec, A: App<Spec = S>> {
    runtime: Runtime<S, A>,
    windows: HashMap<WinitWindowId, NativeWindow>,
    by_id: HashMap<WindowId, WinitWindowId>,
    hub: RenderHub,
    clipboard: NativeClipboard,
    tray: Option<TrayHandle>,
    tray_spec: Option<TraySpec>,
    /// Tray widgets: the flyout, the taskbar thumbnail, the icon badge.
    tray_widgets: Option<TrayWidgets>,
    /// Pending spawn specs that could not be created yet (before resumed).
    pending: Vec<(WindowId, WindowSpec)>,
    start: Instant,
    /// When true, the next about_to_wait should exit after draining.
    exit: bool,
    /// Cursor locked: device mouse motion becomes RawDelta.
    raw_delta: bool,
    /// Between `suspended` and the next `resumed` (app in the background on
    /// Android / iOS): nothing paints.
    suspended: bool,
    /// Automation port and its in-flight work (feature `automation`).
    #[cfg(feature = "automation")]
    auto: AutoState,
    /// The startup thread building the hub and the prewarm; `hub` is a
    /// placeholder until [`Self::settle`] joins it.
    startup: Option<Startup>,
    /// The startup prewarm, for the first window.
    prewarmed: Option<uzor_render_hub::Prewarmed>,
    /// Prewarm switches (after the `UZOR_PREWARM` override).
    prewarm: PrewarmConfig,
    /// `RenderPreference::backend`, for a hub built after a failed startup thread.
    preferred: Option<RenderBackend>,
}

/// The longest a window waits hidden for its first frame
/// ([`PrewarmConfig::hold_until_first_frame`]) before it is shown anyway.
const HOLD_MAX: std::time::Duration = std::time::Duration::from_secs(2);

/// The native host's automation bookkeeping.
#[cfg(feature = "automation")]
#[derive(Default)]
struct AutoState {
    port: Option<crate::automation::AutomationPort>,
    /// Input replies owed after this loop turn's paint.
    after_paint: Vec<(crate::automation::AutoReplyTx, WindowId)>,
    /// Queries answered after this loop turn's paint (they read the new frame).
    queries: Vec<crate::automation::AutoJob>,
    /// Frames painted since start, and callers waiting for the next one.
    waits: crate::automation::FrameWaits,
    /// Last clear colour per window (offscreen screenshots reuse it).
    background: HashMap<WindowId, u32>,
}

impl<S: Spec, A: App<Spec = S>> NativeHost<S, A> {
    /// Take the hub (and the prewarm) from the startup thread, once.
    fn settle(&mut self) {
        if let Some(s) = self.startup.take() {
            match s.join() {
                Some((hub, warm)) => {
                    self.hub = hub;
                    self.prewarmed = warm;
                }
                None => self.hub = build_hub(self.preferred),
            }
        }
    }

    fn now(&self) -> Seconds {
        Seconds(self.start.elapsed().as_secs_f64())
    }

    fn caps() -> HostCaps {
        HostCaps {
            multi_window: true,
            #[cfg(target_os = "windows")]
            os_resize_bezel: true,
            #[cfg(not(target_os = "windows"))]
            os_resize_bezel: false,
            clipboard_async: false,
        }
    }

    fn push(&mut self, window: WindowId, event: InputEvent) {
        #[cfg(feature = "automation")]
        if let Some(port) = &self.auto.port {
            crate::automation::tap_input(port, &self.runtime, window, &event);
        }
        self.runtime.push_input(InputEnvelope {
            window,
            t: self.now(),
            event,
        });
    }

    /// Back from the background: surfaces dropped in `suspended` are made again
    /// at the window's current size, and every window repaints.
    fn resume_surfaces(&mut self) {
        self.settle();
        let mut back = Vec::new();
        for nw in self.windows.values_mut() {
            if nw.render_state.is_none() {
                let px = super::window_create::surface_px(&nw.window);
                let size = uzor_render_hub::SurfaceSize { width: px.width.max(1), height: px.height.max(1) };
                // Android makes the surface again on every resume: prewarmed on
                // the spot, so the pipelines come from the cache before the frame.
                match super::window_create::create_surface_warm(&self.hub, &nw.window, size, None, &self.prewarm) {
                    Ok((rs, cache)) => {
                        nw.render_state = Some(rs);
                        if let Some(c) = cache {
                            prewarm::save_cache(c);
                        }
                    }
                    Err(e) => log::warn!("resume: surface for {:?} failed: {e}", nw.id),
                }
            }
            nw.window.request_redraw();
            back.push((nw.id, geometry_echo(&nw.window)));
        }
        for (id, (size, dpr, _)) in back {
            self.push(id, InputEvent::Window(WindowInput::Occluded(false)));
            // The surface may come back at another size (rotated while away).
            self.push(id, InputEvent::Window(WindowInput::Resized { size, dpr }));
        }
    }

    /// Echo the window's safe area and keyboard: the wrapper's values
    /// ([`crate::platform`]) win over what winit reports (iOS safe area).
    fn push_insets(&mut self, id: WindowId) {
        let Some(nw) = self.by_id.get(&id).and_then(|w| self.windows.get(w)) else {
            return;
        };
        let (pushed, keyboard) = crate::platform::current();
        let safe = pushed.unwrap_or_else(|| super::window_create::safe_area(&nw.window));
        self.push(id, InputEvent::Window(WindowInput::Insets { safe, keyboard }));
    }

    fn execute_all(&mut self, el: &ActiveEventLoop, cmds: Vec<(WindowId, WindowCommand)>) {
        for (id, cmd) in cmds {
            self.execute_one(el, id, cmd);
        }
    }

    fn execute_one(&mut self, el: &ActiveEventLoop, id: WindowId, cmd: WindowCommand) {
        self.settle();
        // Spawn / Exit / clipboard / screenshot / render do not need the
        // window surface first.
        match &cmd {
            WindowCommand::Spawn(spec) => {
                let spec = spec.clone();
                // No native window to attach to while suspended (Android): wait for `resumed`.
                if self.suspended {
                    self.pending.push((id, spec));
                    return;
                }
                match create_window_warm(el, &self.hub, id, spec, self.prewarmed.take(), &self.prewarm) {
                    Ok(nw) => {
                        prewarm::trace("window-created", &format!("{id:?}"));
                        let (size, dpr, position) = geometry_echo(&nw.window);
                        let wid = nw.winit_id();
                        self.windows.insert(wid, nw);
                        self.by_id.insert(id, wid);
                        self.push(
                            id,
                            InputEvent::Window(WindowInput::Created {
                                size,
                                dpr,
                                position,
                                caps: Self::caps(),
                            }),
                        );
                        self.push_insets(id);
                        // Install tray on first successful window.
                        if self.tray.is_none() {
                            if let Some(spec) = self.tray_spec.take() {
                                let icon = spec.icon.clone();
                                let mut b = TrayBuilder::from_spec(spec);
                                if let Some(item) = self.tray_widgets.as_ref().and_then(|t| t.menu_item()) {
                                    b = b.item(item);
                                }
                                match b.build() {
                                    Ok(t) => {
                                        self.tray = Some(t);
                                        if let Some(tw) = self.tray_widgets.as_mut() {
                                            tw.set_base_icon(icon);
                                        }
                                    }
                                    Err(e) => log::warn!("tray build failed: {e}"),
                                }
                            }
                            // The first window's taskbar thumbnail is the widget.
                            if let (Some(tw), Some(nw)) = (self.tray_widgets.as_mut(), self.windows.get(&wid)) {
                                tw.attach_main(&nw.window);
                            }
                        }
                    }
                    Err(e) => {
                        // Also on stderr: without a logger a failed spawn (no GPU device, a
                        // surface error) would otherwise leave a black window and no trace.
                        log::warn!("spawn window {id:?} failed: {e}");
                        eprintln!("[uzor] spawn window {id:?} failed: {e}");
                    }
                }
                return;
            }
            WindowCommand::ExitApp => {
                self.exit = true;
                el.exit();
                return;
            }
            _ => {}
        }

        let Some(&wid) = self.by_id.get(&id) else {
            log::warn!("execute: unknown window {id:?}");
            return;
        };
        let follow = {
            let Some(nw) = self.windows.get_mut(&wid) else {
                return;
            };
            let mut os = nw.as_os();
            executor::execute(&mut os, cmd)
        };

        let Some(follow) = follow else {
            return;
        };
        match follow {
            ExecutorFollowUp::ClipboardWrite(text) => self.clipboard.write(&text),
            ExecutorFollowUp::ClipboardWriteImage(img) => self.clipboard.write_image(&img),
            ExecutorFollowUp::ClipboardRead(ticket) => {
                let result = self.clipboard.read(ticket);
                // No text: an image paste is the app's (e.g. an attachment chip).
                let image = if result.text.is_none() { self.clipboard.read_image() } else { None };
                self.push(id, InputEvent::Clipboard(result));
                if let Some(img) = image {
                    self.push(id, InputEvent::ClipboardImage(img));
                }
            }
            ExecutorFollowUp::Screenshot(ticket) => self.take_screenshot(id, ticket),
            ExecutorFollowUp::Spawn(spec) => {
                // Unreachable: Spawn handled above. Keep for exhaustiveness.
                let _ = spec;
            }
            ExecutorFollowUp::Close => {
                if let Some(wid) = self.by_id.remove(&id) {
                    self.windows.remove(&wid);
                    self.push(id, InputEvent::Window(WindowInput::Destroyed));
                }
                if self.windows.is_empty() {
                    self.exit = true;
                    el.exit();
                }
            }
            ExecutorFollowUp::ExitApp => {
                self.exit = true;
                el.exit();
            }
            ExecutorFollowUp::Render(cmd) => self.apply_render(cmd),
            ExecutorFollowUp::OuterRectEcho { position, size } => {
                self.push(
                    id,
                    InputEvent::Window(WindowInput::OuterRect { position, size }),
                );
            }
        }
    }

    fn apply_render(&mut self, cmd: RenderCmd) {
        match cmd {
            RenderCmd::SetBackend(b) => {
                if let Err(e) = self.hub.set_active(b) {
                    log::warn!("set_backend failed: {e}");
                }
            }
            RenderCmd::SetFpsLimit(fps) => self.hub.set_fps_limit(fps),
            RenderCmd::SetMsaa(n) => self.hub.set_msaa(n),
            RenderCmd::SetVsync(on) => self.hub.set_vsync(on),
        }
    }

    fn take_screenshot(&mut self, id: WindowId, ticket: crate::types::ids::Ticket) {
        let Some(&wid) = self.by_id.get(&id) else {
            return;
        };
        let Some(nw) = self.windows.get_mut(&wid) else {
            return;
        };
        let Some((device, queue, surface)) = nw.render_state.as_mut().and_then(|rs| rs.gpu_handles()) else {
            log::warn!("screenshot: no GPU surface for {id:?}");
            return;
        };
        let Some((pixels, w, h)) = capture_screenshot(device, queue, surface, None) else {
            return;
        };
        let Some(png) = encode_png(&pixels, w, h) else {
            return;
        };
        self.push(
            id,
            InputEvent::Host(HostEvent::Screenshot {
                ticket,
                png: Arc::<[u8]>::from(png),
            }),
        );
    }

    /// Take queued automation jobs: input goes into this turn's tick, its
    /// reply and every query wait for the paint.
    #[cfg(feature = "automation")]
    fn automation_intake(&mut self) {
        use crate::automation::{resolve_window, AutoCmd, AutoError, AutoReply};
        let Some(port) = self.auto.port.clone() else {
            return;
        };
        for (cmd, reply) in port.drain() {
            match cmd {
                AutoCmd::Input { win, events } => match resolve_window(&self.runtime, win) {
                    Some(w) => {
                        for ev in events {
                            self.push(w, ev);
                        }
                        self.auto.after_paint.push((reply, w));
                    }
                    None => {
                        let _ = reply.send(AutoReply::Error(AutoError::NoWindow));
                    }
                },
                other => {
                    if let Some(job) = self.auto.waits.take(other, reply) {
                        self.auto.queries.push(job);
                    }
                }
            }
        }
    }

    /// Answer what waited for this turn's paint. True when work is left
    /// that needs another turn right away.
    #[cfg(feature = "automation")]
    fn automation_answer(&mut self) -> bool {
        use crate::automation::{answer_plain, resolve_window, AutoCmd, AutoError, AutoReply, ShotSource};
        let frame = self.auto.waits.frames;
        for (reply, window) in std::mem::take(&mut self.auto.after_paint) {
            let _ = reply.send(AutoReply::InputDone { window, frame });
        }
        let dprs: HashMap<WindowId, f64> = self
            .by_id
            .iter()
            .filter_map(|(id, wid)| self.windows.get(wid).map(|nw| (*id, nw.window.scale_factor())))
            .collect();
        let backgrounds = self.auto.background.clone();
        let dpr_of = |w: WindowId| dprs.get(&w).copied().unwrap_or(1.0);
        let bg_of = |w: WindowId| backgrounds.get(&w).copied().unwrap_or(0xffffffff);
        for (cmd, reply) in std::mem::take(&mut self.auto.queries) {
            let answer = match cmd {
                AutoCmd::Screenshot { win, source } if source != ShotSource::Offscreen => {
                    match resolve_window(&self.runtime, win) {
                        None => AutoReply::Error(AutoError::NoWindow),
                        Some(w) => match (self.window_readback(w), source) {
                            (Some(shot), _) => AutoReply::Shot(shot),
                            (None, ShotSource::Window) => {
                                AutoReply::Error(AutoError::Unsupported("window readback on this surface"))
                            }
                            (None, _) => {
                                let c = AutoCmd::Screenshot { win: Some(w), source: ShotSource::Offscreen };
                                answer_plain(&mut self.runtime, &c, &dpr_of, &bg_of)
                                    .unwrap_or(AutoReply::Error(AutoError::Unsupported("screenshot")))
                            }
                        },
                    }
                }
                other => answer_plain(&mut self.runtime, &other, &dpr_of, &bg_of)
                    .unwrap_or(AutoReply::Error(AutoError::Unsupported("command"))),
            };
            let _ = reply.send(answer);
        }
        false
    }

    /// What window `id` last presented, read back from its surface.
    #[cfg(feature = "automation")]
    fn window_readback(&mut self, id: WindowId) -> Option<crate::automation::AutoShot> {
        let wid = *self.by_id.get(&id)?;
        let nw = self.windows.get_mut(&wid)?;
        let rs = nw.render_state.as_mut()?;
        let (pixels, w, h) = if let Some((device, queue, surface)) = rs.gpu_handles() {
            capture_screenshot(device, queue, surface, None)?
        } else {
            rs.software_readback()?
        };
        let png = encode_png(&pixels, w, h)?;
        Some(crate::automation::AutoShot { window: id, png, width: w, height: h, source: "window" })
    }

    fn drain_tray(&mut self, el: &ActiveEventLoop) {
        let mut events = Vec::new();
        let mut rect = None;
        if let Some(tray) = self.tray.as_ref() {
            while let Some(ev) = tray.next_event() {
                events.push(ev);
            }
            rect = tray.icon_rect();
        }
        if !events.is_empty() {
            self.settle();
        }
        for ev in events {
            if let Some(tw) = self.tray_widgets.as_mut() {
                tw.on_tray(&ev, rect, el, &self.hub);
            }
            match ev {
                TrayEvent::MenuClick(id) if id == super::tray_widget::FLYOUT_MENU_ID => {}
                TrayEvent::MenuClick(id) => {
                    // Tray has no window; attach to the first open window.
                    if let Some((&win, _)) = self.by_id.iter().next() {
                        self.push(win, InputEvent::Host(HostEvent::Tray { id }));
                    }
                }
                TrayEvent::DoubleClick if self.tray_widgets.is_some() => self.raise_main(),
                _ => {}
            }
        }
        if let Some(tw) = self.tray_widgets.as_mut() {
            if self.startup.is_none() {
                tw.poll(el, &self.hub, self.tray.as_mut(), rect);
            }
        }
    }

    /// Bring the app's first window up (a click on the tray widget).
    fn raise_main(&mut self) {
        let first = self.windows.values().min_by_key(|nw| nw.created);
        if let Some(nw) = first {
            nw.window.set_visible(true);
            nw.window.set_minimized(false);
            nw.window.focus_window();
        }
    }

    fn paint_due(&mut self, frames: Vec<crate::types::frame::FrameRequest>) {
        if !frames.is_empty() {
            self.settle();
        }
        for req in frames {
            let Some(&wid) = self.by_id.get(&req.window) else {
                continue;
            };
            // Split borrows: take the window entry, paint, put metrics back.
            let Some(mut nw) = self.windows.remove(&wid) else {
                continue;
            };
            // A transparent window clears to alpha 0 so what the app leaves
            // unpainted (rounded corners) shows the desktop.
            let mut req = req;
            if nw.transparent {
                req.background = 0;
            }
            #[cfg(feature = "automation")]
            {
                self.auto.waits.painted();
                self.auto.background.insert(req.window, req.background);
            }
            // Suspended (app in the background): no surface on Android, and iOS
            // kills an app that submits GPU work from the background.
            let Some(rs) = nw.render_state.as_mut().filter(|_| !self.suspended) else {
                self.windows.insert(wid, nw);
                continue;
            };
            let t_paint = Instant::now();
            match paint_frame(&mut self.runtime, &mut self.hub, rs, &req) {
                Ok(()) if !nw.presented => {
                    nw.presented = true;
                    let m = self.hub.metrics();
                    prewarm::trace(
                        "first-frame",
                        &format!(
                            "{:?}: paint {:.1} ms (submit {:.1}, render {:.1}, present {:.1})",
                            req.window,
                            t_paint.elapsed().as_secs_f64() * 1e3,
                            m.submit_us as f64 / 1e3,
                            m.render_to_texture_us as f64 / 1e3,
                            m.present_us as f64 / 1e3
                        ),
                    );
                    if let Some(c) = nw.pipeline_cache.take() {
                        prewarm::save_cache(c);
                    }
                }
                Ok(()) => {}
                Err(e) => log::warn!("paint failed: {e}"),
            }
            flush_x11(&nw.window);
            if std::env::var_os("UZOR_TRACE_PAINT").is_some() {
                eprintln!("[paint] {:?} t={:.3}", req.window, self.now().get());
            }
            // Report render facts.
            let m = *self.hub.metrics();
            let info = RenderInfo {
                backend: Some(self.hub.active()),
                available: self.hub.available_backends(),
                fps_ema: 0.0,
                frame_ms: m.submit_us as f64 / 1000.0,
                frame_count: 0,
            };
            self.windows.insert(wid, nw);
            self.push(req.window, InputEvent::Host(HostEvent::RenderInfo(info)));
        }
    }
}

impl<S: Spec, A: App<Spec = S>> ApplicationHandler<HostWake> for NativeHost<S, A> {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        self.settle();
        if self.suspended {
            self.suspended = false;
            self.resume_surfaces();
        }
        let pending = std::mem::take(&mut self.pending);
        for (id, spec) in pending {
            self.execute_one(el, id, WindowCommand::Spawn(spec));
        }
    }

    /// The app went to the background. App state (the runtime, the windows'
    /// layout and input state) stays; only the surfaces go where the OS takes
    /// them away. Android destroys the native window behind every surface, so
    /// they are dropped here and made again in `resumed`. iOS keeps the layer
    /// but forbids GPU work in the background, so painting just stops.
    /// Every window reads `Occluded(true)` (and `Occluded(false)` on return),
    /// so apps can pause timers and the cadence stops asking for frames.
    fn suspended(&mut self, _el: &ActiveEventLoop) {
        if self.suspended {
            return;
        }
        self.suspended = true;
        let ids: Vec<WindowId> = self.windows.values().map(|nw| nw.id).collect();
        if DROP_SURFACES_ON_SUSPEND {
            for nw in self.windows.values_mut() {
                nw.render_state = None;
            }
        }
        for id in ids {
            self.push(id, InputEvent::Window(WindowInput::Occluded(true)));
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, id: WinitWindowId, ev: WindowEvent) {
        if let Some(tw) = self.tray_widgets.as_mut().filter(|t| t.owns(id)) {
            if tw.window_event(&ev) == FlyoutAction::OpenMain {
                self.raise_main();
            }
            el.set_control_flow(ControlFlow::Poll);
            return;
        }
        let Some(nw) = self.windows.get_mut(&id) else {
            return;
        };
        let framework_id = nw.id;

        // Scale factor: update mapper and emit a real Resized with current size.
        if let WindowEvent::ScaleFactorChanged { scale_factor, .. } = &ev {
            let _ = nw.mapper.map(&ev);
            let (size, _, _) = geometry_echo(&nw.window);
            self.push(
                framework_id,
                InputEvent::Window(WindowInput::Resized {
                    size,
                    dpr: *scale_factor,
                }),
            );
            return;
        }

        // Resize: the surface (GPU swapchain / software presenter pixmap) follows the
        // window, otherwise frames keep presenting at the creation size (only the
        // top-left part drawn after maximize) and a shrink can stall presents.
        if let WindowEvent::Resized(size) = &ev {
            if let Some(rs) = nw.render_state.as_mut() {
                resize_render_surface(rs, size.width, size.height);
            }
            nw.window.request_redraw();
            // The OS (WM, double-click, Win+Up) can maximize too: echo the state
            // so a restore toggle reads the truth.
            let maximized = nw.window.is_maximized();
            if nw.maximized != Some(maximized) {
                nw.maximized = Some(maximized);
                self.push(framework_id, InputEvent::Window(WindowInput::Maximized(maximized)));
            }
            // `self.windows` borrow ended by push; re-fetch below.
            let Some(nw) = self.windows.get_mut(&id) else {
                return;
            };
            let events = nw.mapper.map(&ev);
            for e in events {
                self.push(framework_id, e);
            }
            // Rotation moves the notch: the safe area follows the size.
            self.push_insets(framework_id);
            return;
        }

        if matches!(ev, WindowEvent::RedrawRequested) {
            // Force an immediate tick+paint for this window by waking.
            // about_to_wait does the real work; request a poll.
            el.set_control_flow(ControlFlow::Poll);
            return;
        }

        // OS file drag: the pointer is grabbed by the drag source, so no
        // CursorMoved arrives; query it so drops (and the hover) target the
        // widget under the pointer.
        match &ev {
            WindowEvent::HoveredFile(_) | WindowEvent::DroppedFile(_) => {
                if let Some(p) = query_pointer(&nw.window) {
                    nw.dnd_hover = matches!(ev, WindowEvent::HoveredFile(_)).then_some(p);
                    let pos = crate::Point::new(p.0, p.1);
                    self.push(framework_id, InputEvent::Pointer(PointerInput::Moved { pos, mods: Default::default() }));
                } else {
                    nw.dnd_hover = None;
                }
            }
            WindowEvent::HoveredFileCancelled => nw.dnd_hover = None,
            _ => {}
        }
        let Some(nw) = self.windows.get_mut(&id) else {
            return;
        };
        let events = nw.mapper.map(&ev);
        for e in events {
            self.push(framework_id, e);
        }
    }

    fn device_event(&mut self, _el: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        if !self.raw_delta {
            return;
        }
        if let DeviceEvent::MouseMotion { delta } = event {
            if let Some((&win, _)) = self.by_id.iter().next() {
                self.push(
                    win,
                    InputEvent::Pointer(PointerInput::RawDelta {
                        dx: delta.0,
                        dy: delta.1,
                    }),
                );
            }
        }
    }

    fn user_event(&mut self, _el: &ActiveEventLoop, _event: HostWake) {
        // about_to_wait will tick; nothing else to do.
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        self.drain_tray(el);
        if crate::platform::take_changed().is_some() {
            let ids: Vec<WindowId> = self.by_id.keys().copied().collect();
            for id in ids {
                self.push_insets(id);
            }
        }
        #[cfg(feature = "automation")]
        self.automation_intake();
        let (moved, dnd) = dnd_moves(self.windows.values_mut());
        for (id, p) in moved {
            self.push(id, InputEvent::Pointer(PointerInput::Moved { pos: crate::Point::new(p.0, p.1), mods: Default::default() }));
        }
        let out = self.runtime.tick(self.now());
        // Windows already open had their Created delivered by this tick, so
        // the icon command is in `out`. A spawn inside execute_all is not
        // in this list: its button waits until the next tick.
        // Held windows (`hold_until_first_frame`) are shown after the paint
        // below presents their first frame; the others before it.
        let hold = self.prewarm.hold_until_first_frame;
        let mut reveal = Vec::new();
        for (wid, nw) in &mut self.windows {
            if nw.defer_show && !hold {
                nw.defer_show = false;
                reveal.push(*wid);
            }
        }
        self.execute_all(el, out.window_commands);
        for wid in reveal {
            if let Some(nw) = self.windows.get(&wid) {
                nw.window.set_visible(true);
                prewarm::trace("shown", &format!("{:?}", nw.id));
            }
        }
        if std::env::var_os("UZOR_TRACE_PAINT").is_some() {
            eprintln!("[tick] frames {} wake {:?}", out.frames.len(), out.wake);
        }
        self.paint_due(out.frames);
        let mut held = false;
        if hold {
            let mut shown = Vec::new();
            for nw in self.windows.values_mut() {
                if !nw.defer_show {
                    continue;
                }
                if nw.presented || nw.created.elapsed() >= HOLD_MAX {
                    nw.defer_show = false;
                    nw.window.set_visible(true);
                    prewarm::trace("shown", &format!("{:?} presented={}", nw.id, nw.presented));
                    shown.push((nw.id, geometry_echo(&nw.window)));
                } else {
                    held = true;
                }
            }
            // One more frame once mapped, at the same size: a platform that
            // drops what was presented to a hidden window (X11 without a
            // compositor) gets it again; everywhere else it is a cheap warm frame.
            for (id, (size, dpr, _)) in shown {
                self.push(id, InputEvent::Window(WindowInput::Resized { size, dpr }));
            }
        }
        #[cfg(feature = "automation")]
        let auto_busy = self.automation_answer();
        #[cfg(not(feature = "automation"))]
        let auto_busy = false;

        if self.exit || self.runtime.should_exit() {
            el.exit();
            return;
        }

        // In the background nothing paints: park until the OS brings the app back
        // (frames the cadence still asks for would spin the loop for nothing).
        if self.suspended {
            el.set_control_flow(ControlFlow::Wait);
            return;
        }

        // A paint that asked for another frame (or declared commands) must not
        // park the loop on the pre-paint wake.
        let wake = if self.runtime.take_paint_wants_tick() || auto_busy { Wake::Immediate } else { out.wake };
        el.set_control_flow(match wake {
            Wake::Idle => ControlFlow::Wait,
            Wake::Immediate => ControlFlow::Poll,
            Wake::At(t) => {
                let now = self.now().get();
                let target = t.get();
                if target <= now {
                    ControlFlow::Poll
                } else {
                    let dur = std::time::Duration::from_secs_f64(target - now);
                    ControlFlow::WaitUntil(Instant::now() + dur)
                }
            }
        });
        if dnd && !matches!(wake, Wake::Immediate) {
            el.set_control_flow(ControlFlow::WaitUntil(Instant::now() + std::time::Duration::from_millis(40)));
        }
        // A window still hidden waiting for its first frame: come back to
        // show it by `HOLD_MAX` even when nothing paints.
        if held && !matches!(wake, Wake::Immediate) {
            let soon = Instant::now() + std::time::Duration::from_millis(50);
            let at = match el.control_flow() {
                ControlFlow::WaitUntil(t) if t < soon => t,
                _ => soon,
            };
            el.set_control_flow(ControlFlow::WaitUntil(at));
        }
        // The tray flyout: hide when due, paint when moving.
        let next = match self.tray_widgets.as_mut() {
            Some(tw) if self.startup.is_none() => tw.frame(&self.hub),
            _ => None,
        };
        if let Some(at) = next {
            match el.control_flow() {
                ControlFlow::Poll => {}
                ControlFlow::WaitUntil(t) if t <= at => {}
                _ => el.set_control_flow(ControlFlow::WaitUntil(at)),
            }
        }
    }
}

/// While a file drag hovers a window, follow the pointer (X11 DnD sends no
/// motion to the target window): moved windows, and whether any drag is active.
fn dnd_moves<'a>(windows: impl Iterator<Item = &'a mut super::window::NativeWindow>) -> (Vec<(crate::WindowId, (f64, f64))>, bool) {
    let mut moved = Vec::new();
    let mut any = false;
    for nw in windows {
        let Some(last) = nw.dnd_hover else { continue };
        any = true;
        if let Some(p) = query_pointer(&nw.window) {
            if p != last {
                nw.dnd_hover = Some(p);
                moved.push((nw.id, p));
            }
        }
    }
    (moved, any)
}

/// Pointer position in the window's logical px (X11 XQueryPointer), if available.
#[cfg(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android"))))]
fn query_pointer(window: &winit::window::Window) -> Option<(f64, f64)> {
    use std::sync::OnceLock;
    use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
    static XLIB: OnceLock<Option<x11_dl::xlib::Xlib>> = OnceLock::new();
    let xlib = XLIB.get_or_init(|| x11_dl::xlib::Xlib::open().ok()).as_ref()?;
    let RawDisplayHandle::Xlib(dh) = window.display_handle().ok()?.as_raw() else { return None };
    let d = dh.display?.as_ptr().cast();
    let win = match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Xlib(w) => w.window,
        RawWindowHandle::Xcb(w) => w.window.get() as _,
        _ => return None,
    };
    let (mut root, mut child) = (0, 0);
    let (mut rx, mut ry, mut wx, mut wy, mut mask) = (0, 0, 0, 0, 0);
    // SAFETY: winit's live display + window; XQueryPointer only reads.
    let ok = unsafe { (xlib.XQueryPointer)(d, win, &mut root, &mut child, &mut rx, &mut ry, &mut wx, &mut wy, &mut mask) };
    if ok == 0 {
        return None;
    }
    let k = window.scale_factor();
    Some((wx as f64 / k, wy as f64 / k))
}

#[cfg(not(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android")))))]
fn query_pointer(_window: &winit::window::Window) -> Option<(f64, f64)> {
    None
}

/// Prefer a fixed backend when set; otherwise autodetection.
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderPreference {
    /// Explicit backend, or `None` for autodetection.
    pub backend: Option<RenderBackend>,
}

/// Dev control at run time: which app directory the discovery file goes in,
/// the app name `ping` reports, and whether to run with no window. Served
/// only by a build with the `dev-control` feature; any other build prints
/// one line and ignores it (and refuses `headless`), so an app sets it
/// without a `cfg`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevControlConfig {
    /// Directory name under `$XDG_RUNTIME_DIR` (else `~/.config`), e.g. `"hatchery-hq"`.
    pub app_dir: String,
    /// App name in `ping` and the discovery file.
    pub app: String,
    /// Run with no window and no display ([`crate::host::run_headless`]):
    /// the same RPC, screenshots offscreen.
    pub headless: bool,
}

impl DevControlConfig {
    /// A config for `app_dir` / `app`.
    pub fn new(app_dir: &str, app: &str) -> Self {
        Self { app_dir: app_dir.to_string(), app: app.to_string(), headless: false }
    }

    /// `Some` when the process was started with `--dev` (or `--headless`,
    /// which implies it), or `env_var` is set to anything but empty or `0`
    /// (e.g. `HQ_DEV=1`; `HQ_DEV=headless` also asks for no window).
    pub fn from_args_env(app_dir: &str, app: &str, env_var: &str) -> Option<Self> {
        Self::from_parts(app_dir, app, std::env::args().skip(1), std::env::var(env_var).ok())
    }

    fn from_parts(app_dir: &str, app: &str, args: impl Iterator<Item = String>, env: Option<String>) -> Option<Self> {
        let (mut dev, mut headless) = (false, false);
        for a in args {
            match a.as_str() {
                "--dev" => dev = true,
                "--headless" => headless = true,
                _ => {}
            }
        }
        let env = env.unwrap_or_default();
        headless |= env == "headless";
        dev |= !env.is_empty() && env != "0";
        (dev || headless).then(|| Self { headless, ..Self::new(app_dir, app) })
    }
}

/// Extra native-only startup options (design §6.1).
#[derive(Clone, Debug, Default)]
pub struct NativeOptions {
    /// Optional single-instance mutex name.
    pub single_instance: Option<String>,
    /// Optional system tray.
    pub tray: Option<TraySpec>,
    /// Tray widgets (needs [`Self::tray`]): a live card in a flyout over the tray
    /// icon, as the taskbar thumbnail (Windows) and a status dot on the icon. Keep a
    /// clone of the handle to update it.
    pub tray_widget: Option<TrayWidgetHandle>,
    /// Render backend preference.
    pub render: RenderPreference,
    /// Serve this automation port (feature `automation`); see
    /// [`crate::automation::channel`].
    #[cfg(feature = "automation")]
    pub automation: Option<crate::automation::AutomationPort>,
    /// Serve dev control (feature `dev-control`; ignored without it).
    pub dev_control: Option<DevControlConfig>,
    /// The activity `android_main` received. winit cannot build an event loop on
    /// Android without it; build with `android-native-activity` or
    /// `android-game-activity`.
    #[cfg(target_os = "android")]
    pub android_app: Option<winit::platform::android::activity::AndroidApp>,
    /// Renderer prewarm and the on-disk pipeline cache (on by default;
    /// [`PrewarmConfig::off`] or `UZOR_PREWARM=0` turns them off).
    pub prewarm: PrewarmConfig,
}

/// Run the app on the native host (feature `native`).
pub fn run_native<S, A>(
    app: A,
    cfg: RuntimeConfig,
    native: NativeOptions,
) -> Result<(), FrameworkError>
where
    S: Spec,
    A: App<Spec = S>,
{
    prewarm::mark_origin();
    let _guard: Option<SingleInstanceGuard> = native
        .single_instance
        .as_deref()
        .map(single_instance::single_instance);

    #[cfg(feature = "dev-control")]
    let mut native = native;
    // Kept for the whole run: dropping it stops the server and removes the
    // discovery file.
    #[cfg(feature = "dev-control")]
    let _dev = native.dev_control.clone().and_then(|c| {
        let (client, port) = crate::automation::channel();
        match crate::dev_control::start(&c.app_dir, &c.app, client) {
            Ok(d) => {
                // The token is only in the discovery file, never printed.
                eprintln!("{}: dev control on 127.0.0.1:{} (discovery file {})", c.app, d.port(), d.discovery_path().display());
                native.automation = Some(port);
                Some(d)
            }
            Err(e) => {
                eprintln!("{}: dev control did not start: {e}", c.app);
                None
            }
        }
    });
    #[cfg(not(feature = "dev-control"))]
    if let Some(c) = native.dev_control.as_ref() {
        if c.headless {
            return Err(FrameworkError::Host(format!("{}: --headless needs a build with dev control", c.app)));
        }
        eprintln!("{}: this build cannot be driven remotely; ignored", c.app);
    }
    #[cfg(feature = "dev-control")]
    if let Some(c) = native.dev_control.as_ref().filter(|c| c.headless) {
        let Some(port) = native.automation.clone().filter(|_| _dev.is_some()) else {
            return Err(FrameworkError::Host(format!("{}: headless without dev control is unreachable; not starting", c.app)));
        };
        eprintln!("{}: headless (no window, offscreen screenshots)", c.app);
        crate::host::run_headless(app, cfg, 1.0, port);
        return Ok(());
    }

    // The hub's adapter probe and the prewarm run on their own thread while
    // the event loop, the runtime and the app's first tick are built here.
    #[cfg_attr(not(target_os = "android"), allow(unused_mut))]
    let mut pcfg = native.prewarm.clone().effective();
    #[cfg(target_os = "android")]
    if pcfg.cache_base.is_none() {
        pcfg.cache_base = native.android_app.as_ref().and_then(|a| a.internal_data_path());
    }
    let startup = pcfg.enabled.then(|| Startup::spawn(native.render.backend, pcfg.clone()));
    prewarm::trace("run", if startup.is_some() { "prewarm on" } else { "prewarm off" });

    #[cfg_attr(not(target_os = "android"), allow(unused_mut))]
    let mut builder = EventLoop::<HostWake>::with_user_event();
    #[cfg(target_os = "android")]
    {
        use winit::platform::android::EventLoopBuilderExtAndroid;
        let Some(app) = native.android_app.clone() else {
            return Err(FrameworkError::Host("android: NativeOptions::android_app is not set".into()));
        };
        builder.with_android_app(app);
    }
    let event_loop = builder
        .build()
        .map_err(|e| FrameworkError::Host(format!("event loop: {e}")))?;
    let proxy = event_loop.create_proxy();
    #[cfg(feature = "automation")]
    if let Some(port) = native.automation.as_ref() {
        let proxy = proxy.clone();
        port.set_waker(move || {
            let _ = proxy.send_event(HostWake::Automation);
        });
    }
    {
        let proxy = proxy.clone();
        crate::platform::set_wake(move || {
            let _ = proxy.send_event(HostWake::Wake);
        });
    }
    let waker = Waker::new(move || {
        let _ = proxy.send_event(HostWake::Wake);
    });

    let (runtime, _handle) = Runtime::new(app, cfg, waker);
    // With a startup thread the real hub arrives in `settle`.
    let hub = if startup.is_some() { RenderHub::fixed(RenderBackend::TinySkia) } else { build_hub(native.render.backend) };

    let mut host = NativeHost {
        runtime,
        windows: HashMap::new(),
        by_id: HashMap::new(),
        hub,
        clipboard: NativeClipboard::new(),
        tray: None,
        tray_spec: native.tray,
        tray_widgets: native.tray_widget.clone().map(TrayWidgets::new),
        pending: Vec::new(),
        start: Instant::now(),
        exit: false,
        raw_delta: false,
        suspended: false,
        #[cfg(feature = "automation")]
        auto: AutoState { port: native.automation.clone(), ..Default::default() },
        startup,
        prewarmed: None,
        prewarm: pcfg,
        preferred: native.render.backend,
    };

    // The first tick drains Open commands into Spawn follow-ups. Run it
    // before the loop so resumed() can create the windows.
    let out = host.runtime.tick(host.now());
    for (id, cmd) in out.window_commands {
        if let WindowCommand::Spawn(spec) = cmd {
            host.pending.push((id, spec));
        }
    }

    event_loop
        .run_app(&mut host)
        .map_err(|e| FrameworkError::Host(format!("event loop run: {e}")))?;
    Ok(())
}

/// Convenience: run with default native options.
pub fn run_native_default<S, A>(app: A, cfg: RuntimeConfig) -> Result<(), FrameworkError>
where
    S: Spec,
    A: App<Spec = S>,
{
    run_native(app, cfg, NativeOptions::default())
}

/// Resize a window's render surface to the new physical size (no-op for 0×0,
/// which winit reports for a minimized window).
pub(crate) fn resize_render_surface(rs: &mut WindowRenderState, width: u32, height: u32) {
    if width == 0 || height == 0 {
        return;
    }
    rs.resize_surface(width, height);
}

/// Flushes the window's Xlib connection after a present. softbuffer's X11
/// path writes `shm::PutImage` plus a `GetInputFocus` completion probe into
/// the shared Xlib/XCB output buffer without flushing it, so the frame (and
/// the probe the next present waits on) sat in the buffer until some other
/// traffic flushed it: the last frame after a state change stayed unseen
/// until the mouse moved.
#[cfg(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android"))))]
pub(super) fn flush_x11(window: &winit::window::Window) {
    use std::sync::OnceLock;
    use winit::raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
    static XLIB: OnceLock<Option<x11_dl::xlib::Xlib>> = OnceLock::new();
    let Ok(h) = window.display_handle() else { return };
    if let RawDisplayHandle::Xlib(x) = h.as_raw() {
        if let (Some(d), Some(xlib)) = (x.display, XLIB.get_or_init(|| x11_dl::xlib::Xlib::open().ok())) {
            // SAFETY: winit's live display pointer; XFlush only writes the buffer out.
            unsafe { (xlib.XFlush)(d.as_ptr().cast()) };
        }
    }
}

#[cfg(not(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android")))))]
pub(super) fn flush_x11(_window: &winit::window::Window) {}

#[cfg(test)]
mod dev_config_tests {
    use super::DevControlConfig;

    fn parse(args: &[&str], env: Option<&str>) -> Option<DevControlConfig> {
        DevControlConfig::from_parts("d", "a", args.iter().map(|s| s.to_string()), env.map(str::to_string))
    }

    #[test]
    fn flags_and_env() {
        assert_eq!(parse(&[], None), None);
        assert_eq!(parse(&[], Some("0")), None);
        assert!(!parse(&["--dev"], None).unwrap().headless);
        assert!(parse(&["--dev", "--headless"], None).unwrap().headless);
        assert!(parse(&["--headless"], None).unwrap().headless);
        assert!(parse(&[], Some("headless")).unwrap().headless);
        assert!(!parse(&[], Some("1")).unwrap().headless);
    }
}
