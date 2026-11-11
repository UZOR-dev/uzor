//! Tray widgets on the desktop: a live card instead of the plain app icon when the
//! user hovers the tray icon or the taskbar button (`uzor::ui::tray`).
//!
//! - **Flyout**: a borderless, always-on-top glass window anchored at the tray icon,
//!   shown on hover or click (Windows, macOS menu bar), or from a tray menu entry
//!   ([`TrayWidgetConfig::menu_label`]; Linux StatusNotifier items report no hover).
//!   It hides when the pointer leaves it (or the icon), on a click (which brings the
//!   app's first window up) or on Escape.
//! - **Taskbar thumbnail** (Windows): the main window's hover thumbnail and peek
//!   preview are the widget, drawn by DWM from bitmaps the host renders on the CPU
//!   (`DwmSetIconicThumbnail`). **Dock tile** (macOS): the app icon with the status dot.
//! - **Badge**: the tray icon with the status dot ([`uzor::ui::tray::badge`]).
//!
//! The app owns a [`TrayWidgetHandle`] (pass a clone in
//! [`NativeOptions::tray_widget`](super::NativeOptions::tray_widget)) and updates it
//! from any thread; an update that changes something wakes the host loop.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use uzor::render::RenderContext;
use uzor::ui::style::Sheet;
use uzor::ui::tray::{self, TrayLabels, TrayLook, TrayModel, TrayWidget};
use uzor::{Rect, RgbaIcon};
use uzor_render_tiny_skia::TinySkiaCpuRenderContext;

use super::tray::IconRect;
use crate::types::ids::TrayItemId;

/// What opens the flyout over the tray icon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlyoutTrigger {
    /// Hovering the icon (and a left click).
    #[default]
    Hover,
    /// A left click on the icon only.
    Click,
}

/// The tray widget setup. Every field can change at run time through the handle.
#[derive(Clone, Debug, PartialEq)]
pub struct TrayWidgetConfig {
    /// The widget shown; `None` turns the flyout and the thumbnail off (the plain icon).
    pub widget: Option<TrayWidget>,
    /// What the widget shows.
    pub model: TrayModel,
    /// Its words.
    pub labels: TrayLabels,
    /// Dark or light.
    pub dark: bool,
    /// Glass flyout (an overlay); `false`: the opaque card.
    pub glass: bool,
    /// Glass fill override (a CSS colour).
    pub glass_bg: Option<String>,
    /// Glass hairline override (a CSS colour).
    pub glass_line: Option<String>,
    /// What opens the flyout.
    pub trigger: FlyoutTrigger,
    /// Windows: the widget is the taskbar hover thumbnail. macOS: the dock icon wears
    /// the status dot.
    pub thumbnail: bool,
    /// The tray icon wears the status dot.
    pub badge: bool,
    /// A tray menu entry that opens the flyout (needed on Linux).
    pub menu_label: Option<String>,
    /// Critters keep still.
    pub reduced_motion: bool,
}

impl Default for TrayWidgetConfig {
    fn default() -> Self {
        TrayWidgetConfig {
            widget: Some(TrayWidget::Combined),
            model: TrayModel::default(),
            labels: TrayLabels::default(),
            dark: true,
            glass: true,
            glass_bg: None,
            glass_line: None,
            trigger: FlyoutTrigger::Hover,
            thumbnail: true,
            badge: true,
            menu_label: None,
            reduced_motion: false,
        }
    }
}

impl TrayWidgetConfig {
    /// The flyout look.
    pub fn flyout_look(&self) -> TrayLook {
        TrayLook { glass_bg: self.glass_bg.clone(), glass_line: self.glass_line.clone(), ..if self.glass { TrayLook::flyout(self.dark) } else { TrayLook::solid(self.dark) } }
    }
}

/// The tray menu entry the host adds for [`TrayWidgetConfig::menu_label`]. Its picks
/// open the flyout and do not reach the app.
pub const FLYOUT_MENU_ID: TrayItemId = TrayItemId(u32::MAX - 7);

#[derive(Default)]
struct Shared {
    cfg: TrayWidgetConfig,
    rev: u64,
    /// `Some(true)`: show the flyout; `Some(false)`: hide it.
    show: Option<bool>,
}

/// The app's side of the tray widgets: cheap to clone, usable from any thread.
#[derive(Clone, Default)]
pub struct TrayWidgetHandle {
    inner: Arc<Mutex<Shared>>,
}

impl std::fmt::Debug for TrayWidgetHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrayWidgetHandle").field("rev", &self.lock().rev).finish()
    }
}

impl TrayWidgetHandle {
    /// A handle starting from `cfg`.
    pub fn new(cfg: TrayWidgetConfig) -> Self {
        TrayWidgetHandle { inner: Arc::new(Mutex::new(Shared { cfg, rev: 1, show: None })) }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Shared> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Change the setup; the host is woken only when something changed.
    pub fn update(&self, f: impl FnOnce(&mut TrayWidgetConfig)) {
        let changed = {
            let mut g = self.lock();
            let before = g.cfg.clone();
            f(&mut g.cfg);
            let ch = g.cfg != before;
            if ch {
                g.rev += 1;
            }
            ch
        };
        if changed {
            crate::platform::wake();
        }
    }

    /// The widget shown (`None`: off).
    pub fn set_widget(&self, w: Option<TrayWidget>) {
        self.update(|c| c.widget = w);
    }
    /// New data.
    pub fn set_model(&self, m: TrayModel) {
        self.update(|c| c.model = m);
    }
    /// New words (a language switch).
    pub fn set_labels(&self, l: TrayLabels) {
        self.update(|c| c.labels = l);
    }
    /// Dark or light.
    pub fn set_dark(&self, dark: bool) {
        self.update(|c| c.dark = dark);
    }

    /// The current setup.
    pub fn config(&self) -> TrayWidgetConfig {
        self.lock().cfg.clone()
    }

    /// Open the flyout now (a menu entry, a shortcut).
    pub fn show(&self) {
        self.lock().show = Some(true);
        crate::platform::wake();
    }
    /// Close the flyout.
    pub fn hide(&self) {
        self.lock().show = Some(false);
        crate::platform::wake();
    }

    /// The widget rendered on the CPU at `scale` (the thumbnail look unless `look`),
    /// or `None` when it is off.
    pub fn snapshot(&self, scale: f64, look: Option<TrayLook>, t: f64) -> Option<RgbaIcon> {
        let cfg = self.config();
        let w = cfg.widget?;
        let look = look.unwrap_or_else(|| TrayLook::solid(cfg.dark));
        Some(render_rgba(w, &cfg, &look, scale, t))
    }

    fn take(&self, seen: u64) -> Option<(u64, TrayWidgetConfig)> {
        let g = self.lock();
        (g.rev != seen).then(|| (g.rev, g.cfg.clone()))
    }

    fn take_show(&self) -> Option<bool> {
        self.lock().show.take()
    }
}

thread_local! {
    static SHEET: Sheet = tray::sheet();
}

/// Widget `w` of `cfg` rendered on the CPU at `scale` into an RGBA image of its size
/// (transparent outside the card's rounded corners).
pub fn render_rgba(w: TrayWidget, cfg: &TrayWidgetConfig, look: &TrayLook, scale: f64, t: f64) -> RgbaIcon {
    let (wd, ht) = w.size();
    let scale = if scale.is_finite() && scale > 0.05 { scale } else { 1.0 };
    let (pw, ph) = ((wd * scale).round().max(1.0) as u32, (ht * scale).round().max(1.0) as u32);
    let mut ctx = TinySkiaCpuRenderContext::new(pw, ph, scale);
    uzor::render::Painter::scale(&mut ctx, scale, scale);
    SHEET.with(|sheet| {
        tray::render(&mut ctx, sheet, w, &cfg.model, &cfg.labels, look, 0.0, 0.0, t);
    });
    let pm = ctx.pixmap();
    // tiny-skia keeps premultiplied RGBA: unpremultiply for the icon contract.
    let mut px = pm.data().to_vec();
    for p in px.chunks_exact_mut(4) {
        let a = p[3] as u32;
        if a > 0 && a < 255 {
            for c in &mut p[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    RgbaIcon::from_rgba(pw, ph, px)
}

/// Draw anything into a `px`-square RGBA icon on the CPU (a tray or window icon from
/// a vector mark): `draw` gets the context and the icon rect in logical px (`px`).
pub fn rasterize_icon(px: u32, draw: impl FnOnce(&mut dyn RenderContext, Rect)) -> RgbaIcon {
    let px = px.max(1);
    let mut ctx = TinySkiaCpuRenderContext::new(px, px, 1.0);
    draw(&mut ctx, Rect::new(0.0, 0.0, px as f64, px as f64));
    let mut data = ctx.pixmap().data().to_vec();
    for p in data.chunks_exact_mut(4) {
        let a = p[3] as u32;
        if a > 0 && a < 255 {
            for c in &mut p[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    RgbaIcon::from_rgba(px, px, data)
}

/// Where the flyout goes, physical px: centred on the icon, above it when the icon
/// is in the lower half of its monitor (a bottom taskbar), below it otherwise (a top
/// bar), beside it on a left / right taskbar; without an icon, in the corner the
/// platform keeps its tray. Always inside the monitor `mon` (x, y, w, h).
pub fn flyout_position(icon: Option<IconRect>, mon: (f64, f64, f64, f64), size: (f64, f64), scale: f64) -> (i32, i32) {
    let (mx, my, mw, mh) = mon;
    let (w, h) = size;
    let gap = 8.0 * scale;
    let (x, y) = match icon.filter(|r| r.width > 0.0 || r.height > 0.0) {
        Some(r) => {
            let (cx, cy) = (r.x + r.width / 2.0, r.y + r.height / 2.0);
            if cx < mx + mw * 0.08 {
                (r.x + r.width + gap, cy - h / 2.0)
            } else if cx > mx + mw * 0.92 && cy > my + mh * 0.1 && cy < my + mh * 0.9 {
                (r.x - w - gap, cy - h / 2.0)
            } else if cy > my + mh / 2.0 {
                (cx - w / 2.0, r.y - h - gap)
            } else {
                (cx - w / 2.0, r.y + r.height + gap)
            }
        }
        None => {
            let edge = 12.0 * scale;
            if cfg!(target_os = "windows") {
                (mx + mw - w - edge, my + mh - h - 56.0 * scale)
            } else {
                (mx + mw - w - edge, my + 36.0 * scale)
            }
        }
    };
    let x = x.clamp(mx, (mx + mw - w).max(mx));
    let y = y.clamp(my, (my + mh - h).max(my));
    (x.round() as i32, y.round() as i32)
}

/// After the pointer leaves the icon or the flyout, it hides this much later.
const HIDE_DELAY: Duration = Duration::from_millis(350);
/// A flyout opened without the pointer on it (a menu entry) hides after this long
/// unless the pointer comes over it.
const IDLE_HIDE: Duration = Duration::from_secs(8);
/// Frame interval while critters move.
const FRAME: Duration = Duration::from_millis(33);

/// What a flyout event asks of the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FlyoutAction {
    None,
    /// Bring the app's first window up (a click on the card).
    OpenMain,
}

struct Flyout {
    window: Arc<winit::window::Window>,
    rs: uzor_render_hub::WindowRenderState,
    widget: TrayWidget,
    visible: bool,
    /// The pointer is over the flyout.
    inside: bool,
    /// The pointer came over it since it was shown.
    entered: bool,
    hide_at: Option<Instant>,
    dirty: bool,
    last_paint: Option<Instant>,
}

/// The host's tray widget state.
pub(crate) struct TrayWidgets {
    handle: TrayWidgetHandle,
    rev: u64,
    cfg: TrayWidgetConfig,
    flyout: Option<Flyout>,
    /// The tray icon as the app gave it (the badge goes on a copy).
    base_icon: Option<RgbaIcon>,
    /// The badge colour now on the tray icon.
    badge_on: Option<Option<[u8; 4]>>,
    start: Instant,
    #[cfg(target_os = "windows")]
    thumb: Option<win::Thumb>,
    #[cfg(target_os = "macos")]
    dock: Option<Option<[u8; 4]>>,
}

impl TrayWidgets {
    pub(crate) fn new(handle: TrayWidgetHandle) -> Self {
        let cfg = handle.config();
        TrayWidgets {
            handle,
            rev: 0,
            cfg,
            flyout: None,
            base_icon: None,
            badge_on: None,
            start: Instant::now(),
            #[cfg(target_os = "windows")]
            thumb: None,
            #[cfg(target_os = "macos")]
            dock: None,
        }
    }

    /// The menu entry to add to the tray, if any.
    pub(crate) fn menu_item(&self) -> Option<super::tray::TrayMenuItem> {
        self.cfg.menu_label.clone().map(|label| super::tray::TrayMenuItem { id: FLYOUT_MENU_ID, label, enabled: true })
    }

    /// The tray icon the app set (for the badge).
    pub(crate) fn set_base_icon(&mut self, icon: Option<RgbaIcon>) {
        self.base_icon = icon;
        self.badge_on = None;
    }

    /// The app's first window: its taskbar thumbnail becomes the widget (Windows).
    pub(crate) fn attach_main(&mut self, window: &winit::window::Window) {
        #[cfg(target_os = "windows")]
        if self.thumb.is_none() {
            if let Some(hwnd) = super::win32::extract_hwnd(window) {
                self.thumb = Some(win::Thumb::install(hwnd, self.handle.clone()));
                self.thumb.as_ref().map(|t| t.set_enabled(self.cfg.thumbnail && self.cfg.widget.is_some()));
            }
        }
        let _ = window;
    }

    pub(crate) fn owns(&self, id: winit::window::WindowId) -> bool {
        self.flyout.as_ref().is_some_and(|f| f.window.id() == id)
    }

    fn now(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    /// Take the app's changes: the badge, the thumbnail, the flyout's content.
    pub(crate) fn poll(&mut self, el: &winit::event_loop::ActiveEventLoop, hub: &uzor_render_hub::RenderHub, tray: Option<&mut super::tray::TrayHandle>, icon: Option<IconRect>) {
        if let Some((rev, cfg)) = self.handle.take(self.rev) {
            self.rev = rev;
            self.cfg = cfg;
            if let Some(f) = self.flyout.as_mut() {
                f.dirty = true;
            }
            if self.cfg.widget.is_none() {
                self.hide();
            }
            #[cfg(target_os = "windows")]
            if let Some(t) = self.thumb.as_ref() {
                t.set_enabled(self.cfg.thumbnail && self.cfg.widget.is_some());
                t.invalidate();
            }
        }
        // Badge on the tray icon.
        let want = if self.cfg.badge { self.cfg.model.status.rgba() } else { None };
        if let (Some(t), Some(base)) = (tray, self.base_icon.as_ref()) {
            if self.badge_on != Some(want) {
                let _ = t.set_icon(tray::badge(base, want));
                self.badge_on = Some(want);
            }
        }
        // Dock tile.
        #[cfg(target_os = "macos")]
        {
            let dot = if self.cfg.thumbnail { self.cfg.model.status.rgba() } else { None };
            if self.dock != Some(dot) {
                if let Some(base) = self.base_icon.as_ref() {
                    mac::set_dock_icon(&tray::badge(base, dot));
                    self.dock = Some(dot);
                }
            }
        }
        match self.handle.take_show() {
            Some(true) => self.show(el, hub, icon, false),
            Some(false) => self.hide(),
            None => {}
        }
    }

    /// A tray icon event.
    pub(crate) fn on_tray(&mut self, ev: &super::tray::TrayEvent, icon: Option<IconRect>, el: &winit::event_loop::ActiveEventLoop, hub: &uzor_render_hub::RenderHub) {
        use super::tray::TrayEvent;
        match ev {
            TrayEvent::Enter if self.cfg.trigger == FlyoutTrigger::Hover => self.show(el, hub, icon, true),
            TrayEvent::Leave => {
                if let Some(f) = self.flyout.as_mut().filter(|f| f.visible && !f.inside) {
                    f.hide_at = Some(Instant::now() + HIDE_DELAY);
                }
            }
            TrayEvent::LeftClick => {
                if self.flyout.as_ref().is_some_and(|f| f.visible) && self.cfg.trigger == FlyoutTrigger::Click {
                    self.hide();
                } else {
                    self.show(el, hub, icon, true);
                }
            }
            TrayEvent::MenuClick(id) if *id == FLYOUT_MENU_ID => self.show(el, hub, icon, false),
            _ => {}
        }
    }

    fn show(&mut self, el: &winit::event_loop::ActiveEventLoop, hub: &uzor_render_hub::RenderHub, icon: Option<IconRect>, hovering: bool) {
        let Some(w) = self.cfg.widget else { return };
        if self.flyout.as_ref().is_some_and(|f| f.widget != w) {
            // Another size: a new window.
            self.flyout = None;
        }
        if self.flyout.is_none() {
            match make_flyout(el, hub, w, self.cfg.glass) {
                Ok(f) => self.flyout = Some(f),
                Err(e) => {
                    log::warn!("tray flyout: {e}");
                    return;
                }
            }
        }
        let Some(f) = self.flyout.as_mut() else { return };
        let scale = f.window.scale_factor();
        let (wd, ht) = w.size();
        let size = (wd * scale, ht * scale);
        let mon = monitor_for(el, icon);
        let (x, y) = flyout_position(icon, mon, size, scale);
        f.window.set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
        f.hide_at = if hovering { None } else { Some(Instant::now() + IDLE_HIDE) };
        f.entered = false;
        f.dirty = true;
        if !f.visible {
            f.visible = true;
            f.window.set_visible(true);
        }
        f.window.request_redraw();
    }

    fn hide(&mut self) {
        if let Some(f) = self.flyout.as_mut().filter(|f| f.visible) {
            f.visible = false;
            f.inside = false;
            f.hide_at = None;
            f.window.set_visible(false);
        }
    }

    /// A window event of the flyout.
    pub(crate) fn window_event(&mut self, ev: &winit::event::WindowEvent) -> FlyoutAction {
        use winit::event::{ElementState, WindowEvent};
        let Some(f) = self.flyout.as_mut() else { return FlyoutAction::None };
        match ev {
            WindowEvent::CursorEntered { .. } => {
                f.inside = true;
                f.entered = true;
                f.hide_at = None;
            }
            WindowEvent::CursorLeft { .. } => {
                f.inside = false;
                f.hide_at = Some(Instant::now() + HIDE_DELAY);
            }
            WindowEvent::MouseInput { state: ElementState::Released, .. } => {
                self.hide();
                return FlyoutAction::OpenMain;
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) {
                    self.hide();
                }
            }
            WindowEvent::Focused(false) if f.visible && !f.inside && f.entered => self.hide(),
            WindowEvent::Resized(sz) => {
                f.rs.resize_surface(sz.width.max(1), sz.height.max(1));
                f.dirty = true;
            }
            WindowEvent::ScaleFactorChanged { .. } | WindowEvent::RedrawRequested => f.dirty = true,
            WindowEvent::CloseRequested => self.hide(),
            _ => {}
        }
        FlyoutAction::None
    }

    /// Hide when due, paint when dirty or moving. The next time the host must wake
    /// for the flyout, if any.
    pub(crate) fn frame(&mut self, hub: &uzor_render_hub::RenderHub) -> Option<Instant> {
        let now = Instant::now();
        if self.flyout.as_ref().is_some_and(|f| f.visible && f.hide_at.is_some_and(|t| t <= now)) {
            self.hide();
        }
        let t = self.now();
        let cfg = &self.cfg;
        let f = self.flyout.as_mut().filter(|f| f.visible)?;
        let moving = cfg.widget.is_some_and(|w| cfg.model.animating(w)) && !cfg.reduced_motion;
        let due = f.dirty || (moving && f.last_paint.map_or(true, |p| now.duration_since(p) >= FRAME));
        if due {
            paint_flyout(f, cfg, hub, t);
            f.dirty = false;
            f.last_paint = Some(now);
        }
        let next_frame = moving.then(|| f.last_paint.unwrap_or(now) + FRAME);
        match (next_frame, f.hide_at) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }
}

fn monitor_for(el: &winit::event_loop::ActiveEventLoop, icon: Option<IconRect>) -> (f64, f64, f64, f64) {
    let rect = |m: &winit::monitor::MonitorHandle| {
        let p = m.position();
        let s = m.size();
        (f64::from(p.x), f64::from(p.y), f64::from(s.width), f64::from(s.height))
    };
    if let Some(r) = icon {
        let (cx, cy) = (r.x + r.width / 2.0, r.y + r.height / 2.0);
        if let Some(m) = el.available_monitors().find(|m| {
            let (x, y, w, h) = rect(m);
            cx >= x && cx < x + w && cy >= y && cy < y + h
        }) {
            return rect(&m);
        }
    }
    el.primary_monitor().or_else(|| el.available_monitors().next()).map(|m| rect(&m)).unwrap_or((0.0, 0.0, 1920.0, 1080.0))
}

fn make_flyout(el: &winit::event_loop::ActiveEventLoop, hub: &uzor_render_hub::RenderHub, w: TrayWidget, glass: bool) -> Result<Flyout, String> {
    use winit::dpi::LogicalSize;
    use winit::window::{Window, WindowLevel};
    let (wd, ht) = w.size();
    #[allow(unused_mut)]
    let mut attrs = Window::default_attributes()
        .with_title("")
        .with_inner_size(LogicalSize::new(wd, ht))
        .with_decorations(false)
        .with_resizable(false)
        .with_transparent(true)
        .with_blur(glass)
        .with_visible(false)
        .with_active(false)
        .with_window_level(WindowLevel::AlwaysOnTop);
    #[cfg(target_os = "windows")]
    {
        use winit::platform::windows::WindowAttributesExtWindows;
        attrs = attrs.with_skip_taskbar(true);
    }
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android"))))]
    {
        // X11: placed exactly where asked, no taskbar entry, no WM frame.
        use winit::platform::x11::WindowAttributesExtX11;
        attrs = attrs.with_override_redirect(true);
    }
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::WindowAttributesExtMacOS;
        attrs = attrs.with_has_shadow(true);
    }
    let window = Arc::new(el.create_window(attrs).map_err(|e| e.to_string())?);
    #[cfg(target_os = "windows")]
    {
        super::win32::set_corner_style(&window, uzor::CornerStyle::Rounded);
        if glass {
            if let Some(h) = super::win32::extract_hwnd(&window) {
                win::acrylic(h);
            }
        }
    }
    let px = super::window_create::surface_px(&window);
    let size = uzor_render_hub::SurfaceSize { width: px.width.max(1), height: px.height.max(1) };
    let rs = super::window_create::create_surface(hub, &window, size).map_err(|e| e.to_string())?;
    Ok(Flyout { window, rs, widget: w, visible: false, inside: false, entered: false, hide_at: None, dirty: true, last_paint: None })
}

fn paint_flyout(f: &mut Flyout, cfg: &TrayWidgetConfig, hub: &uzor_render_hub::RenderHub, t: f64) {
    let look = cfg.flyout_look();
    let w = f.widget;
    f.rs.begin_frame();
    let _ = f.rs.with_render_context(|ctx| {
        SHEET.with(|sheet| {
            let (wd, ht) = w.size();
            let d = uzor::ui::dom::lay_out_in(ctx, sheet, &tray::tray_widget(w, &cfg.model, &cfg.labels, &look), 0.0, 0.0, wd, Some(Rect::new(0.0, 0.0, wd, ht)), &|_| uzor::ui::style::State::NONE);
            tray::paint(ctx, &d, &cfg.model, t, cfg.reduced_motion);
        });
    });
    let out = uzor_render_hub::submit_frame(
        &mut f.rs,
        uzor_render_hub::SubmitParams { base_color: color::AlphaColor::new([0.0, 0.0, 0.0, 0.0]), msaa_samples: hub.settings().msaa_samples },
    );
    if out.surface_lost {
        log::warn!("tray flyout: surface lost");
    }
    super::app_handler::flush_x11(&f.window);
}

/// Windows: the taskbar thumbnail and peek preview drawn from the widget, and the
/// flyout's acrylic backdrop.
#[cfg(target_os = "windows")]
mod win {
    use super::TrayWidgetHandle;

    type Hwnd = isize;
    type Hbitmap = isize;

    #[repr(C)]
    struct BitmapInfoHeader {
        size: u32,
        width: i32,
        height: i32,
        planes: u16,
        bit_count: u16,
        compression: u32,
        size_image: u32,
        x_ppm: i32,
        y_ppm: i32,
        clr_used: u32,
        clr_important: u32,
    }

    #[repr(C)]
    struct BitmapInfo {
        header: BitmapInfoHeader,
        colors: [u32; 1],
    }

    #[repr(C)]
    #[derive(Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    struct Margins {
        left: i32,
        right: i32,
        top: i32,
        bottom: i32,
    }

    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmSetWindowAttribute(hwnd: Hwnd, attr: u32, pv: *const core::ffi::c_void, cb: u32) -> i32;
        fn DwmSetIconicThumbnail(hwnd: Hwnd, hbmp: Hbitmap, flags: u32) -> i32;
        fn DwmSetIconicLivePreviewBitmap(hwnd: Hwnd, hbmp: Hbitmap, client: *const core::ffi::c_void, flags: u32) -> i32;
        fn DwmInvalidateIconicBitmaps(hwnd: Hwnd) -> i32;
        fn DwmExtendFrameIntoClientArea(hwnd: Hwnd, m: *const Margins) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateDIBSection(hdc: isize, bmi: *const BitmapInfo, usage: u32, bits: *mut *mut core::ffi::c_void, section: isize, offset: u32) -> Hbitmap;
        fn DeleteObject(h: isize) -> i32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetClientRect(hwnd: Hwnd, r: *mut Rect) -> i32;
    }

    #[link(name = "comctl32")]
    extern "system" {
        fn SetWindowSubclass(hwnd: Hwnd, cb: SubclassProc, id: usize, data: usize) -> i32;
        fn DefSubclassProc(hwnd: Hwnd, msg: u32, wp: usize, lp: isize) -> isize;
    }

    type SubclassProc = unsafe extern "system" fn(Hwnd, u32, usize, isize, usize, usize) -> isize;

    const DWMWA_FORCE_ICONIC_REPRESENTATION: u32 = 7;
    const DWMWA_HAS_ICONIC_BITMAP: u32 = 10;
    const DWMWA_SYSTEMBACKDROP_TYPE: u32 = 38;
    const DWMSBT_TRANSIENTWINDOW: u32 = 3;
    const WM_DWMSENDICONICTHUMBNAIL: u32 = 0x0323;
    const WM_DWMSENDICONICLIVEPREVIEWBITMAP: u32 = 0x0326;
    const SUBCLASS_ID: usize = 0x755A_5457; // "uZTW"

    /// The flyout's system backdrop: acrylic (Windows 11; ignored before).
    pub(super) fn acrylic(hwnd: Hwnd) {
        let v: u32 = DWMSBT_TRANSIENTWINDOW;
        let m = Margins { left: -1, right: -1, top: -1, bottom: -1 };
        unsafe {
            DwmExtendFrameIntoClientArea(hwnd, &m);
            DwmSetWindowAttribute(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, &v as *const u32 as *const _, 4);
        }
    }

    pub(super) struct Thumb {
        hwnd: Hwnd,
    }

    impl Thumb {
        /// Subclass the main window: DWM asks it for the thumbnail and peek bitmaps.
        /// The handle lives as long as the window (it is leaked into the subclass).
        pub(super) fn install(hwnd: Hwnd, handle: TrayWidgetHandle) -> Thumb {
            let data = Box::into_raw(Box::new(handle)) as usize;
            unsafe {
                SetWindowSubclass(hwnd, proc, SUBCLASS_ID, data);
            }
            Thumb { hwnd }
        }

        pub(super) fn set_enabled(&self, on: bool) {
            let v: i32 = on as i32;
            unsafe {
                DwmSetWindowAttribute(self.hwnd, DWMWA_FORCE_ICONIC_REPRESENTATION, &v as *const i32 as *const _, 4);
                DwmSetWindowAttribute(self.hwnd, DWMWA_HAS_ICONIC_BITMAP, &v as *const i32 as *const _, 4);
            }
        }

        pub(super) fn invalidate(&self) {
            unsafe {
                DwmInvalidateIconicBitmaps(self.hwnd);
            }
        }
    }

    /// A top-down 32-bit premultiplied BGRA DIB of `img` placed at (`ox`, `oy`) in a
    /// `w`×`h` transparent canvas.
    fn dib(img: &uzor::RgbaIcon, w: u32, h: u32, ox: u32, oy: u32) -> Hbitmap {
        let bmi = BitmapInfo {
            header: BitmapInfoHeader {
                size: std::mem::size_of::<BitmapInfoHeader>() as u32,
                width: w as i32,
                height: -(h as i32),
                planes: 1,
                bit_count: 32,
                compression: 0,
                size_image: 0,
                x_ppm: 0,
                y_ppm: 0,
                clr_used: 0,
                clr_important: 0,
            },
            colors: [0],
        };
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbmp = unsafe { CreateDIBSection(0, &bmi, 0, &mut bits, 0, 0) };
        if hbmp == 0 || bits.is_null() {
            return 0;
        }
        let out = unsafe { std::slice::from_raw_parts_mut(bits as *mut u8, (w * h * 4) as usize) };
        out.fill(0);
        for y in 0..img.height.min(h.saturating_sub(oy)) {
            for x in 0..img.width.min(w.saturating_sub(ox)) {
                let s = ((y * img.width + x) * 4) as usize;
                let d = (((y + oy) * w + x + ox) * 4) as usize;
                let a = img.pixels[s + 3] as u32;
                let pm = |c: u8| ((c as u32 * a + 127) / 255) as u8;
                out[d] = pm(img.pixels[s + 2]);
                out[d + 1] = pm(img.pixels[s + 1]);
                out[d + 2] = pm(img.pixels[s]);
                out[d + 3] = a as u8;
            }
        }
        hbmp
    }

    unsafe extern "system" fn proc(hwnd: Hwnd, msg: u32, wp: usize, lp: isize, _id: usize, data: usize) -> isize {
        if msg == WM_DWMSENDICONICTHUMBNAIL || msg == WM_DWMSENDICONICLIVEPREVIEWBITMAP {
            let handle = &*(data as *const TrayWidgetHandle);
            let cfg = match handle.inner.try_lock() {
                Ok(g) => g.cfg.clone(),
                Err(_) => return DefSubclassProc(hwnd, msg, wp, lp),
            };
            let Some(w) = cfg.widget.filter(|_| cfg.thumbnail) else { return DefSubclassProc(hwnd, msg, wp, lp) };
            let (wd, ht) = w.size();
            let look = uzor::ui::tray::TrayLook::solid(cfg.dark);
            if msg == WM_DWMSENDICONICTHUMBNAIL {
                let (mw, mh) = (((lp >> 16) & 0xffff) as f64, (lp & 0xffff) as f64);
                let scale = (mw / wd).min(mh / ht).max(0.1);
                let img = super::render_rgba(w, &cfg, &look, scale, 0.0);
                let h = dib(&img, img.width, img.height, 0, 0);
                if h != 0 {
                    DwmSetIconicThumbnail(hwnd, h, 0);
                    DeleteObject(h);
                }
            } else {
                let mut r = Rect::default();
                GetClientRect(hwnd, &mut r);
                let (cw, ch) = ((r.right - r.left).max(1) as u32, (r.bottom - r.top).max(1) as u32);
                let scale = (cw as f64 * 0.5 / wd).min(ch as f64 * 0.5 / ht).max(1.0);
                let img = super::render_rgba(w, &cfg, &look, scale, 0.0);
                let (ox, oy) = (cw.saturating_sub(img.width) / 2, ch.saturating_sub(img.height) / 2);
                let h = dib(&img, cw, ch, ox, oy);
                if h != 0 {
                    DwmSetIconicLivePreviewBitmap(hwnd, h, std::ptr::null(), 0);
                    DeleteObject(h);
                }
            }
            return 0;
        }
        DefSubclassProc(hwnd, msg, wp, lp)
    }
}

/// macOS: the dock icon with the status dot.
#[cfg(target_os = "macos")]
mod mac {
    use objc2::rc::Retained;
    use objc2::ClassType;
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::{MainThreadMarker, NSData};

    pub(super) fn set_dock_icon(icon: &uzor::RgbaIcon) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let Some(png) = super::super::screenshot::encode_png(&icon.pixels, icon.width, icon.height) else { return };
        let data = NSData::with_bytes(&png);
        let img: Option<Retained<NSImage>> = NSImage::initWithData(NSImage::alloc(), &data);
        let app = NSApplication::sharedApplication(mtm);
        unsafe { app.setApplicationIconImage(img.as_deref()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flyout_sits_by_the_icon_and_inside_the_monitor() {
        let mon = (0.0, 0.0, 1920.0, 1080.0);
        // Bottom taskbar: above the icon, centred.
        let r = IconRect { x: 1700.0, y: 1050.0, width: 24.0, height: 24.0 };
        let (x, y) = flyout_position(Some(r), mon, (320.0, 200.0), 1.0);
        assert_eq!((x, y), (1552, 842));
        // Near the right edge: clamped in.
        let r = IconRect { x: 1900.0, y: 1050.0, width: 20.0, height: 24.0 };
        assert_eq!(flyout_position(Some(r), mon, (320.0, 200.0), 1.0).0, 1600);
        // Top bar: below.
        let r = IconRect { x: 1000.0, y: 0.0, width: 24.0, height: 24.0 };
        assert_eq!(flyout_position(Some(r), mon, (300.0, 164.0), 1.0), (862, 32));
        // Left taskbar: beside.
        let r = IconRect { x: 4.0, y: 600.0, width: 24.0, height: 24.0 };
        assert_eq!(flyout_position(Some(r), mon, (300.0, 164.0), 1.0), (36, 530));
        // No icon: a corner, inside.
        let (x, y) = flyout_position(None, mon, (300.0, 164.0), 2.0);
        assert!(x >= 0 && y >= 0 && x + 300 <= 1920 && y + 164 <= 1080);
    }

    #[test]
    fn handle_bumps_only_on_change_and_snapshots() {
        let h = TrayWidgetHandle::new(TrayWidgetConfig::default());
        let (r0, _) = h.take(0).unwrap();
        assert!(h.take(r0).is_none());
        h.set_dark(true);
        assert!(h.take(r0).is_none(), "same value, no new revision");
        h.set_widget(Some(TrayWidget::Tasks));
        let (r1, c) = h.take(r0).unwrap();
        assert!(r1 > r0 && c.widget == Some(TrayWidget::Tasks));
        h.show();
        assert_eq!(h.take_show(), Some(true));
        assert_eq!(h.take_show(), None);
        let img = h.snapshot(2.0, None, 0.0).unwrap();
        assert_eq!((img.width, img.height), (600, 328));
        // Opaque card: the middle is filled, the very corner is outside the rounding.
        let mid = ((164 * 600 + 300) * 4) as usize;
        assert_eq!(img.pixels[mid + 3], 255);
        assert!(img.pixels[3] < 255);
        h.set_widget(None);
        assert!(h.snapshot(1.0, None, 0.0).is_none());
    }

    #[test]
    fn rasterized_icons_are_straight_alpha() {
        let i = rasterize_icon(16, |ctx, r| {
            ctx.set_fill_color("rgba(255,0,0,0.5)");
            ctx.fill_rect(r.x, r.y, r.width, r.height);
        });
        assert_eq!(i.pixels.len(), 16 * 16 * 4);
        assert!(i.pixels[0] > 250 && (120..=135).contains(&i.pixels[3]));
    }
}
