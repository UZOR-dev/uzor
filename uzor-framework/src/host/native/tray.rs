//! System tray icon + menu (moved from `uzor-desktop`, design §7.3).
//!
//! Menu selections become [`HostEvent::Tray`](crate::HostEvent::Tray) via the
//! native host's about_to_wait drain — never a second input path. Hover and clicks
//! on the icon (Windows, macOS) drive the tray widget flyout ([`super::tray_widget`]).
//!
//! Linux: the icon is a StatusNotifier item (libappindicator), which needs a GTK loop
//! on the thread that made it. The tray lives on its own thread with its own GTK
//! loop; menu picks come back through a channel and wake the host. A StatusNotifier
//! item reports no hover or clicks on the icon itself, only menu picks.

#[cfg(not(any(target_os = "ios", target_os = "android", target_os = "linux")))]
use std::collections::HashMap;

#[cfg(not(any(target_os = "ios", target_os = "android", target_os = "linux")))]
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
#[cfg(not(any(target_os = "ios", target_os = "android", target_os = "linux")))]
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use uzor::RgbaIcon;

use crate::types::ids::TrayItemId;

/// Errors from tray construction / icon updates.
#[derive(Debug)]
pub enum TrayError {
    /// Tray icon or menu construction failed.
    Build(String),
    /// Icon conversion / update failed.
    Icon(String),
}

impl std::fmt::Display for TrayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrayError::Build(s) => write!(f, "tray build error: {s}"),
            TrayError::Icon(s) => write!(f, "tray icon error: {s}"),
        }
    }
}

impl std::error::Error for TrayError {}

/// High-level tray events drained by [`TrayHandle::next_event`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayEvent {
    /// A context menu item was selected.
    MenuClick(TrayItemId),
    /// Left single-click on the tray icon.
    LeftClick,
    /// Right click on the tray icon.
    RightClick,
    /// Left double-click on the tray icon.
    DoubleClick,
    /// The pointer came over the icon (Windows, macOS).
    Enter,
    /// The pointer left the icon (Windows, macOS).
    Leave,
}

/// Where the tray icon is on screen, physical px (from the last icon event).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct IconRect {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

/// One tray menu entry declared at startup.
#[derive(Clone, Debug)]
pub struct TrayMenuItem {
    /// Stable id delivered as [`HostEvent::Tray`](crate::HostEvent::Tray).
    pub id: TrayItemId,
    /// Visible label.
    pub label: String,
    /// Whether the item is enabled.
    pub enabled: bool,
}

/// Spec for a system-tray icon (native-only `RuntimeConfig` field).
#[derive(Clone, Debug, Default)]
pub struct TraySpec {
    /// Hover tooltip.
    pub tooltip: Option<String>,
    /// Optional RGBA icon; the host may fill from the window icon when absent.
    pub icon: Option<RgbaIcon>,
    /// Context-menu entries.
    pub items: Vec<TrayMenuItem>,
}

/// Fluent builder for a system tray icon + context menu.
#[derive(Default)]
pub struct TrayBuilder {
    icon: Option<RgbaIcon>,
    tooltip: Option<String>,
    items: Vec<TrayMenuItem>,
}

impl TrayBuilder {
    /// Empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the tray icon.
    pub fn icon(mut self, icon: RgbaIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set the hover tooltip.
    pub fn tooltip(mut self, t: impl Into<String>) -> Self {
        self.tooltip = Some(t.into());
        self
    }

    /// Add an enabled menu item.
    pub fn menu_item(mut self, id: TrayItemId, label: impl Into<String>) -> Self {
        self.items.push(TrayMenuItem {
            id,
            label: label.into(),
            enabled: true,
        });
        self
    }

    /// Add a disabled menu item.
    pub fn menu_item_disabled(mut self, id: TrayItemId, label: impl Into<String>) -> Self {
        self.items.push(TrayMenuItem {
            id,
            label: label.into(),
            enabled: false,
        });
        self
    }

    /// Add a menu item (enabled or not).
    pub fn item(mut self, item: TrayMenuItem) -> Self {
        self.items.push(item);
        self
    }

    /// Build from a [`TraySpec`].
    pub fn from_spec(spec: TraySpec) -> Self {
        Self {
            icon: spec.icon,
            tooltip: spec.tooltip,
            items: spec.items,
        }
    }

    /// Create the OS tray icon. Must run on the main thread after the event
    /// loop has started.
    pub fn build(self) -> Result<TrayHandle, TrayError> {
        #[cfg(any(target_os = "ios", target_os = "android"))]
        {
            let _ = self;
            return Err(TrayError::Build("ios has no status tray".into()));
        }
        #[cfg(target_os = "linux")]
        {
            let dims = self.icon.as_ref().map(|i| (i.width, i.height)).unwrap_or((32, 32));
            let tray = linux::LinuxTray::spawn(self.icon, self.tooltip, self.items)?;
            return Ok(TrayHandle { tray, icon_dims: dims, rect: std::cell::Cell::new(None) });
        }
        #[cfg(not(any(target_os = "ios", target_os = "android", target_os = "linux")))]
        {
        let menu = Menu::new();
        let mut id_map: HashMap<tray_icon::menu::MenuId, TrayItemId> = HashMap::new();

        for item in &self.items {
            let mi = MenuItem::new(&item.label, item.enabled, None);
            id_map.insert(mi.id().clone(), item.id);
            menu.append(&mi)
                .map_err(|e| TrayError::Build(e.to_string()))?;
        }

        let mut builder = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false);

        if let Some(tooltip) = &self.tooltip {
            builder = builder.with_tooltip(tooltip);
        }

        let icon_dims = self.icon.as_ref().map(|i| (i.width, i.height));

        if let Some(rgba) = self.icon {
            let icon = Icon::from_rgba(rgba.pixels, rgba.width, rgba.height)
                .map_err(|e| TrayError::Build(e.to_string()))?;
            builder = builder.with_icon(icon);
        }

        let tray = builder
            .build()
            .map_err(|e| TrayError::Build(e.to_string()))?;

        Ok(TrayHandle {
            tray,
            id_map,
            icon_dims: icon_dims.unwrap_or((32, 32)),
            rect: std::cell::Cell::new(None),
        })
        }
    }
}

/// Live system tray handle. Drop removes the icon.
pub struct TrayHandle {
    #[cfg(not(any(target_os = "ios", target_os = "android", target_os = "linux")))]
    tray: TrayIcon,
    #[cfg(not(any(target_os = "ios", target_os = "android", target_os = "linux")))]
    id_map: HashMap<tray_icon::menu::MenuId, TrayItemId>,
    #[cfg(target_os = "linux")]
    tray: linux::LinuxTray,
    #[cfg_attr(any(target_os = "ios", target_os = "android"), allow(dead_code))]
    icon_dims: (u32, u32),
    #[cfg_attr(any(target_os = "ios", target_os = "android"), allow(dead_code))]
    rect: std::cell::Cell<Option<IconRect>>,
}

impl TrayHandle {
    /// Replace the tray icon.
    pub fn set_icon(&mut self, icon: RgbaIcon) -> Result<(), TrayError> {
        #[cfg(any(target_os = "ios", target_os = "android"))]
        {
            let _ = icon;
            return Err(TrayError::Icon("ios has no status tray".into()));
        }
        #[cfg(target_os = "linux")]
        {
            self.icon_dims = (icon.width, icon.height);
            self.tray.send(linux::Cmd::Icon(icon));
            return Ok(());
        }
        #[cfg(not(any(target_os = "ios", target_os = "android", target_os = "linux")))]
        {
        self.icon_dims = (icon.width, icon.height);
        let os_icon = Icon::from_rgba(icon.pixels, icon.width, icon.height)
            .map_err(|e| TrayError::Icon(e.to_string()))?;
        self.tray
            .set_icon(Some(os_icon))
            .map_err(|e| TrayError::Icon(e.to_string()))
        }
    }

    /// Update the tooltip.
    pub fn set_tooltip(&mut self, text: &str) {
        #[cfg(target_os = "linux")]
        {
            self.tray.send(linux::Cmd::Tooltip(text.to_string()));
        }
        #[cfg(not(any(target_os = "ios", target_os = "android", target_os = "linux")))]
        {
            let _ = self.tray.set_tooltip(Some(text));
        }
        #[cfg(any(target_os = "ios", target_os = "android"))]
        {
            let _ = text;
        }
    }

    /// The icon's screen rect (physical px) from the last hover or click on it, when
    /// the platform reports one (Windows, macOS).
    pub fn icon_rect(&self) -> Option<IconRect> {
        self.rect.get()
    }

    /// Drain one pending tray event (non-blocking).
    pub fn next_event(&self) -> Option<TrayEvent> {
        #[cfg(any(target_os = "ios", target_os = "android"))]
        {
            return None;
        }
        #[cfg(target_os = "linux")]
        {
            return self.tray.next();
        }
        #[cfg(not(any(target_os = "ios", target_os = "android", target_os = "linux")))]
        {
        if let Ok(menu_ev) = MenuEvent::receiver().try_recv() {
            if let Some(id) = self.id_map.get(&menu_ev.id) {
                return Some(TrayEvent::MenuClick(*id));
            }
        }
        while let Ok(tray_ev) = TrayIconEvent::receiver().try_recv() {
            let keep = |r: &tray_icon::Rect| {
                self.rect.set(Some(IconRect { x: r.position.x, y: r.position.y, width: f64::from(r.size.width), height: f64::from(r.size.height) }));
            };
            match tray_ev {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    rect,
                    ..
                } => {
                    keep(&rect);
                    return Some(TrayEvent::LeftClick);
                }
                TrayIconEvent::Click {
                    button: MouseButton::Right,
                    button_state: MouseButtonState::Up,
                    rect,
                    ..
                } => {
                    keep(&rect);
                    return Some(TrayEvent::RightClick);
                }
                TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    rect,
                    ..
                } => {
                    keep(&rect);
                    return Some(TrayEvent::DoubleClick);
                }
                TrayIconEvent::Enter { rect, .. } => {
                    keep(&rect);
                    return Some(TrayEvent::Enter);
                }
                TrayIconEvent::Move { rect, .. } => keep(&rect),
                TrayIconEvent::Leave { rect, .. } => {
                    keep(&rect);
                    return Some(TrayEvent::Leave);
                }
                _ => {}
            }
        }
        None
        }
    }

    /// Remembered icon dimensions.
    pub fn icon_dims(&self) -> (u32, u32) {
        self.icon_dims
    }
}

/// Linux: the StatusNotifier item on its own GTK thread.
#[cfg(target_os = "linux")]
mod linux {
    use std::collections::HashMap;
    use std::sync::mpsc::{channel, Receiver, Sender};

    use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem};
    use tray_icon::{Icon, TrayIconBuilder};
    use uzor::RgbaIcon;

    use super::{TrayError, TrayEvent, TrayMenuItem};
    use crate::types::ids::TrayItemId;

    /// A change for the GTK thread.
    pub(super) enum Cmd {
        Icon(RgbaIcon),
        Tooltip(String),
    }

    pub(super) struct LinuxTray {
        cmds: Sender<Cmd>,
        events: Receiver<TrayEvent>,
    }

    impl LinuxTray {
        pub(super) fn spawn(icon: Option<RgbaIcon>, tooltip: Option<String>, items: Vec<TrayMenuItem>) -> Result<Self, TrayError> {
            let (cmd_tx, cmd_rx) = channel::<Cmd>();
            let (ev_tx, ev_rx) = channel::<TrayEvent>();
            let (ready_tx, ready_rx) = channel::<Result<(), String>>();
            std::thread::Builder::new()
                .name("uzor-tray-gtk".into())
                .spawn(move || {
                    if let Err(e) = gtk::init() {
                        let _ = ready_tx.send(Err(format!("gtk init: {e}")));
                        return;
                    }
                    let menu = Menu::new();
                    let mut ids: HashMap<MenuId, TrayItemId> = HashMap::new();
                    for it in &items {
                        let mi = MenuItem::new(&it.label, it.enabled, None);
                        ids.insert(mi.id().clone(), it.id);
                        if let Err(e) = menu.append(&mi) {
                            let _ = ready_tx.send(Err(e.to_string()));
                            return;
                        }
                    }
                    let mut b = TrayIconBuilder::new().with_menu(Box::new(menu));
                    if let Some(t) = &tooltip {
                        b = b.with_tooltip(t).with_title(t);
                    }
                    if let Some(i) = icon {
                        match Icon::from_rgba(i.pixels, i.width, i.height) {
                            Ok(i) => b = b.with_icon(i),
                            Err(e) => {
                                let _ = ready_tx.send(Err(e.to_string()));
                                return;
                            }
                        }
                    }
                    let tray = match b.build() {
                        Ok(t) => t,
                        Err(e) => {
                            let _ = ready_tx.send(Err(e.to_string()));
                            return;
                        }
                    };
                    // GTK calls these from C: a panic must not unwind out of them (that aborts
                    // the process); logged once, the tray keeps running.
                    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
                        let _ = super::super::tray_widget::guarded("tray menu", || {
                            if let Some(id) = ids.get(&e.id) {
                                let _ = ev_tx.send(TrayEvent::MenuClick(*id));
                                crate::platform::wake();
                            }
                        });
                    }));
                    let _ = ready_tx.send(Ok(()));
                    gtk::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                        loop {
                            match cmd_rx.try_recv() {
                                Ok(Cmd::Icon(i)) => {
                                    // The icon stays as it was when setting it panicked.
                                    let _ = super::super::tray_widget::guarded("tray icon", || {
                                        if let Ok(i) = Icon::from_rgba(i.pixels, i.width, i.height) {
                                            let _ = tray.set_icon(Some(i));
                                        }
                                    });
                                }
                                Ok(Cmd::Tooltip(t)) => {
                                    let _ = tray.set_tooltip(Some(&t));
                                }
                                Err(std::sync::mpsc::TryRecvError::Empty) => return gtk::glib::ControlFlow::Continue,
                                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                    gtk::main_quit();
                                    return gtk::glib::ControlFlow::Break;
                                }
                            }
                        }
                    });
                    gtk::main();
                })
                .map_err(|e| TrayError::Build(e.to_string()))?;
            match ready_rx.recv() {
                Ok(Ok(())) => Ok(LinuxTray { cmds: cmd_tx, events: ev_rx }),
                Ok(Err(e)) => Err(TrayError::Build(e)),
                Err(_) => Err(TrayError::Build("tray thread ended".into())),
            }
        }

        pub(super) fn send(&self, c: Cmd) {
            let _ = self.cmds.send(c);
        }

        pub(super) fn next(&self) -> Option<TrayEvent> {
            self.events.try_recv().ok()
        }
    }
}
