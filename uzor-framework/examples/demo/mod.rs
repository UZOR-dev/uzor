//! One [`DemoApp`] for the native and web entries (design §9.3, brief F11).
//!
//! No `cfg`: both entry points construct [`DemoApp::new`] and
//! [`DemoApp::runtime_config`]. The app drives two windows on a multi-window
//! host, a dock of three panel kinds with splitters, tab tear-off and
//! drag-out (policy), one overlay of each kind, Ctrl+K / Esc, a text field
//! registered for IME, layout save / restore through an in-memory stand-in
//! for the back office, expand gutters (policy) and a theme switch.

use std::collections::{BTreeMap, BTreeSet};

use uzor::input::keyboard::KeyboardShortcut;
use uzor::input::{KeyCode, ModifierKeys};
use uzor::layout::docking::{DockPanel, Pin};
use uzor::layout::OverlayKind;
use uzor::tokens::{BuiltinSet, Tokens};
use uzor::ui::dom::{Dom, Edit, El};
use uzor::ui::domkit::{self as k, icons, MenuItem};
use uzor::ui::style::{Sheet, State};
use uzor::{Rect, WidgetId};
use uzor_framework::widgets::kit;
use uzor_framework::{
    App, AppCommand, Binding, ChromeModel, ChromeTab, DockTarget, DragOutChrome, DragOutPolicy,
    FocusCmd, Intent, IntentCx, KeymapCmd, KeymapScope, LayoutBlob, LayoutCmd, LayoutPolicy,
    ModalModel, OverlayCmd, OverlayCx, OverlayModel, OverlaySize, PanelCx, PanelHome,
    RuntimeConfig, SizePx, Spec, SplitDir, ThemeCmd, Ticket, TooltipModel, WindowId, WindowSpec,
};

#[path = "../common/look.rs"]
mod look;

/// Canvas id the web entry binds.
#[allow(dead_code)]
pub const CANVAS_ID: &str = "uzor-demo";

/// The three dock panel kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DemoPanel {
    /// Tree of commands (settings, theme, layout).
    Navigator,
    /// The text field.
    Editor,
    /// Pinned (`Pin::User { pinned: true }`): it does not tear off.
    Inspector,
}

impl DockPanel for DemoPanel {
    fn title(&self) -> &str {
        match self {
            DemoPanel::Navigator => "Navigator",
            DemoPanel::Editor => "Editor",
            DemoPanel::Inspector => "Inspector",
        }
    }

    fn type_id(&self) -> &'static str {
        match self {
            DemoPanel::Navigator => "nav",
            DemoPanel::Editor => "editor",
            DemoPanel::Inspector => "inspector",
        }
    }

    fn min_size(&self) -> (f32, f32) {
        (96.0, 64.0)
    }

    fn pin(&self) -> Pin {
        match self {
            DemoPanel::Inspector => Pin::User { pinned: true },
            _ => Pin::Free,
        }
    }
}

/// One overlay of each kind the kernel knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DemoOverlay {
    /// Settings modal (text field, focus ring).
    Settings,
    /// Context menu.
    Menu,
    /// Ctrl+K command palette.
    Palette,
    /// Hover tooltip on the navigator.
    Tooltip,
}

/// Keymap targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum DemoAction {
    /// Open the palette.
    Palette,
    /// Toggle dark / light tokens.
    Theme,
    /// Ask the kernel for a layout blob and keep it in memory.
    SaveLayout,
    /// Restore the blob kept by [`DemoApp::office`].
    RestoreLayout,
    /// Close the top overlay (Esc).
    Dismiss,
}

pub struct DemoSpec;

impl Spec for DemoSpec {
    type Panel = DemoPanel;
    type Overlay = DemoOverlay;
    type Action = DemoAction;

    fn decode_panel(_home: PanelHome, type_id: &str) -> Option<DemoPanel> {
        match type_id {
            "nav" => Some(DemoPanel::Navigator),
            "editor" => Some(DemoPanel::Editor),
            "inspector" => Some(DemoPanel::Inspector),
            _ => None,
        }
    }
}

/// Settings button.
pub fn id_settings() -> WidgetId {
    WidgetId::from("demo-settings")
}
/// Menu button.
pub fn id_menu() -> WidgetId {
    WidgetId::from("demo-menu")
}
/// Theme switch.
pub fn id_theme() -> WidgetId {
    WidgetId::from("demo-theme")
}
/// Save layout.
pub fn id_save() -> WidgetId {
    WidgetId::from("demo-save")
}
/// Restore layout.
pub fn id_restore() -> WidgetId {
    WidgetId::from("demo-restore")
}
/// Tooltip anchor.
pub fn id_tip() -> WidgetId {
    WidgetId::from("demo-tip")
}
/// Editor text field (IME).
pub fn id_note() -> WidgetId {
    WidgetId::from("demo-note")
}
/// Settings-modal text field.
pub fn id_settings_note() -> WidgetId {
    WidgetId::from("demo-settings-note")
}

/// Navigator button rects in the same coordinates `panel` draws (taken from the
/// laid-out DOM trees, see [`navigator_hits`]).
#[allow(dead_code)]
pub struct DemoHits {
    /// Settings.
    pub settings: Rect,
    /// Context menu.
    pub menu: Rect,
    /// Theme.
    pub theme: Rect,
    /// Save.
    pub save: Rect,
    /// Restore.
    pub restore: Rect,
    /// Tooltip anchor.
    pub tip: Rect,
    /// Editor text field (when `panel` is the editor's leaf).
    pub note: Rect,
}

/// Hit rects for one leaf's content: the navigator rows and the editor field as the
/// panel lays them out in `panel` (both trees are laid out; their row geometry is fixed
/// CSS, so it does not depend on the font backend).
#[allow(dead_code)] // used by tests/f11_demo.rs
pub fn navigator_hits(panel: Rect) -> DemoHits {
    let mut ctx = look::Measure;
    let sheet = demo_sheet();
    let body = look::leaf_body(panel);
    let nav = uzor::ui::dom::lay_out(&mut ctx, &sheet, &nav_tree(true, true, body.height), body.x, body.y, body.width, &|_| State::NONE);
    let ed = uzor::ui::dom::lay_out(&mut ctx, &sheet, &editor_tree(true, "", body.height), body.x, body.y, body.width, &|_| State::NONE);
    let r = |d: &Dom, id: WidgetId| d.rect(id.as_str()).unwrap_or_default();
    DemoHits {
        settings: r(&nav, id_settings()),
        menu: r(&nav, id_menu()),
        theme: r(&nav, id_theme()),
        save: r(&nav, id_save()),
        restore: r(&nav, id_restore()),
        tip: r(&nav, id_tip()),
        note: r(&ed, id_note()),
    }
}

const DEMO_CSS: &str = r#"
button.d-item { display: flex; align-items: center; gap: 10px; height: 30px; padding: 0 10px; border-radius: 7px;
                width: 100%; max-width: 240px; color: var(--k-fg); flex-shrink: 0 }
button.d-item:hover { background: var(--k-hover) }
button.d-item:focus-visible { border: 2px solid var(--k-accent) }
div.d-nav { display: flex; flex-direction: column; gap: 2px; flex-shrink: 0 }
div.d-preview { padding: 12px; border-radius: 10px; background: var(--k-sunken); border: 1px dashed var(--k-line);
                color: var(--k-body); min-height: 60px; flex-shrink: 0 }
div.d-modal { display: flex; flex-direction: column; gap: 12px; padding: 14px 16px; background: var(--k-panel);
              border: 1px solid var(--k-line); overflow: hidden }
div.d-pop-title { padding: 6px 10px 4px 10px; color: var(--k-muted); font-size: 11px; font-weight: 600 }
"#;

fn demo_sheet() -> Sheet {
    look::sheet(DEMO_CSS)
}

fn nav_item(id: WidgetId, icon: &'static str, label: &str, hint: Option<&str>) -> El {
    let mut b = El::new("button.d-item").id(id.as_str()).tab().name(label).child(look::ico(icon)).child(El::new("div.p-grow").text(label));
    if let Some(h) = hint {
        b = b.child(k::kbd(h));
    }
    b
}

/// The navigator: the command rows and a shortcut card.
fn nav_tree(dark: bool, shortcuts: bool, h: f64) -> El {
    let rows = El::new("div.d-nav")
        .child(nav_item(id_settings(), icons::SETTINGS, "Settings", None))
        .child(nav_item(id_menu(), icons::MENU, "Menu", None))
        .child(nav_item(id_theme(), if dark { icons::SUN } else { icons::MOON }, if dark { "Light" } else { "Dark" }, None))
        .child(nav_item(id_save(), icons::SAVE, "Save", None))
        .child(nav_item(id_restore(), icons::UNDO, "Restore", None))
        .child(nav_item(id_tip(), icons::EXPAND, "Expand", None));
    let keys = El::new("div.p-card")
        .child(El::new("div.p-row").child(k::kbd("Ctrl+K")).child(El::new("div.p-muted").text("command palette")))
        .child(El::new("div.p-row").child(k::kbd("Esc")).child(El::new("div.p-muted").text("close the top overlay")))
        .child(El::new("div.p-row").child(k::kbd("Drag")).child(El::new("div.p-muted").text("a tab to tear it off")));
    look::root(dark, h).child(
        El::new("div.p-page")
            .css(format!("height: {h}px"))
            .child(El::new("div.p-sec").text("COMMANDS"))
            .child(rows)
            .child_if(shortcuts, || El::new("div.p-sec").text("SHORTCUTS"))
            .child_if(shortcuts, || keys),
    )
}

/// The editor: an IME text field and a live markdown preview of it.
fn editor_tree(dark: bool, note: &str, h: f64) -> El {
    let words = note.split_whitespace().count();
    let preview = if note.trim().is_empty() {
        El::new("div.d-preview").runs(k::inline_md("Type in the field: **bold**, _italic_, `code` render here."))
    } else {
        El::new("div.d-preview").runs(k::inline_md(note))
    };
    look::root(dark, h).child(
        El::new("div.p-page")
            .css(format!("height: {h}px"))
            .child(k::field("NOTE", k::input(id_note().as_str(), Edit::new(note).placeholder("Note — IME, selection, clipboard"))))
            .child(El::new("div.p-row").child(El::new("div.p-sec.p-grow").text("PREVIEW")).child(k::badge("markdown", "muted")))
            .child(preview)
            .child(El::new("div.p-muted").text(format!("{} chars · {words} words", note.chars().count()))),
    )
}

/// The pinned inspector: stat tiles over the app state.
fn inspector_tree(dark: bool, saved: usize, windows: usize, panels: usize, h: f64) -> El {
    look::root(dark, h).child(
        El::new("div.p-page")
            .css(format!("height: {h}px"))
            .child(El::new("div.p-row").child(look::ico(icons::PIN)).child(El::new("div.p-grow").text("Pinned panel")).child(k::badge("pinned", "ok")))
            .child(
                El::new("div.p-grid2")
                    .child(k::stat("ins-theme", "THEME", if dark { "Dark" } else { "Light" }, "tokens + --k-* vars"))
                    .child(k::stat("ins-saved", "LAYOUTS SAVED", &saved.to_string(), "in-memory office"))
                    .child(k::stat("ins-windows", "WINDOWS", &windows.to_string(), "multi-window host"))
                    .child(k::stat("ins-panels", "PANELS", &panels.to_string(), "in this window")),
            ),
    )
}

/// The demo. [`DemoApp::office`] is the in-memory back-office stand-in
/// (layout blobs keyed by window label).
pub struct DemoApp {
    keys: BTreeMap<WindowId, String>,
    mains: BTreeSet<WindowId>,
    phase: BTreeMap<WindowId, u8>,
    blob_asked: BTreeSet<WindowId>,
    office: BTreeMap<String, LayoutBlob>,
    dark: bool,
    /// The editor text (mirrors the text store, for the preview).
    note: String,
    /// Settings: show the shortcut card under the navigator.
    shortcuts: bool,
    /// Panels per window (from the last layout change).
    panels: BTreeMap<WindowId, usize>,
    sheet: Sheet,
}

impl Default for DemoApp {
    fn default() -> Self {
        Self::new()
    }
}

impl DemoApp {
    /// Empty office, dark theme.
    pub fn new() -> Self {
        Self {
            keys: BTreeMap::new(),
            mains: BTreeSet::new(),
            phase: BTreeMap::new(),
            blob_asked: BTreeSet::new(),
            office: BTreeMap::new(),
            dark: true,
            note: String::new(),
            shortcuts: true,
            panels: BTreeMap::new(),
            sheet: demo_sheet(),
        }
    }

    /// Two windows, drag-out enabled, expand gutters on, dark tokens.
    ///
    /// A single-canvas host ignores the second spawn (`multi_window: false`);
    /// the app still asks for both.
    pub fn runtime_config() -> RuntimeConfig {
        RuntimeConfig {
            windows: vec![
                WindowSpec::new("main", "Uzor demo", SizePx::new(960, 600)),
                WindowSpec::new("side", "Uzor demo — side", SizePx::new(420, 300)),
            ],
            tokens: Tokens::builtin(BuiltinSet::Dark),
            dark: true,
            layout: LayoutPolicy {
                drag_out: DragOutPolicy::Enabled {
                    chrome: DragOutChrome::UzorChrome,
                },
                edge_expand_px: 16.0,
                expand_thickness_px: 160.0,
                tab_new_button: true,
                ..LayoutPolicy::default()
            },
            ..RuntimeConfig::default()
        }
    }

    /// Blobs stored for the back office, keyed by window label.
    #[allow(dead_code)]
    pub fn office(&self) -> &BTreeMap<String, LayoutBlob> {
        &self.office
    }

    /// `true` while the demo is on the dark token set.
    #[allow(dead_code)]
    pub fn is_dark(&self) -> bool {
        self.dark
    }

    fn theme_set(&self) -> BuiltinSet {
        if self.dark {
            BuiltinSet::Dark
        } else {
            BuiltinSet::Light
        }
    }
}

trait CmdSink {
    fn cmd(&mut self, cmd: AppCommand<DemoSpec>);
}

impl CmdSink for IntentCx<'_, DemoSpec> {
    fn cmd(&mut self, cmd: AppCommand<DemoSpec>) {
        self.command(cmd);
    }
}

struct HookSink<'a>(&'a mut uzor_framework::HookOps<DemoSpec>);

impl CmdSink for HookSink<'_> {
    fn cmd(&mut self, cmd: AppCommand<DemoSpec>) {
        self.0.command(cmd);
    }
}

impl DemoApp {
    fn toggle_theme<C: CmdSink>(&mut self, cx: &mut C) {
        self.dark = !self.dark;
        cx.cmd(AppCommand::Theme(ThemeCmd::SetTokens(Tokens::builtin(
            self.theme_set(),
        ))));
        cx.cmd(AppCommand::Theme(ThemeCmd::SetDark(self.dark)));
    }

    fn save<C: CmdSink>(&mut self, cx: &mut C, win: WindowId) {
        cx.cmd(AppCommand::Layout(LayoutCmd::RequestBlob {
            win,
            ticket: Ticket(7),
        }));
    }

    fn restore<C: CmdSink>(&self, cx: &mut C, win: WindowId) {
        let Some(key) = self.keys.get(&win) else {
            return;
        };
        let Some(blob) = self.office.get(key).cloned() else {
            return;
        };
        cx.cmd(AppCommand::Layout(LayoutCmd::Restore { win, blob }));
    }

    fn on_layout_changed(&mut self, win: WindowId, cx: &mut IntentCx<'_, DemoSpec>) {
        if !self.mains.contains(&win) {
            return;
        }
        let phase = self.phase.get(&win).copied().unwrap_or(0);
        let leaves = cx
            .view
            .window(win)
            .map(|w| w.dock.leaves.clone())
            .unwrap_or_default();
        self.panels.insert(win, leaves.iter().map(|l| l.panels.len()).sum());
        match phase {
            0 if leaves.len() == 1 => {
                cx.command(AppCommand::Layout(LayoutCmd::Split {
                    win,
                    leaf: leaves[0].leaf,
                    dir: SplitDir::Right,
                    panel: DemoPanel::Editor,
                }));
                self.phase.insert(win, 1);
            }
            1 if leaves.len() >= 2 => {
                let leaf = leaves
                    .iter()
                    .find(|l| l.panels.iter().any(|p| p.type_id == "editor"))
                    .map(|l| l.leaf)
                    .unwrap_or(leaves[leaves.len() - 1].leaf);
                cx.command(AppCommand::Layout(LayoutCmd::Split {
                    win,
                    leaf,
                    dir: SplitDir::Down,
                    panel: DemoPanel::Inspector,
                }));
                self.phase.insert(win, 2);
            }
            _ => {}
        }
        let phase = self.phase.get(&win).copied().unwrap_or(0);
        if phase == 2 && leaves.len() >= 3 && self.blob_asked.insert(win) {
            cx.command(AppCommand::Layout(LayoutCmd::RequestBlob {
                win,
                ticket: Ticket(1),
            }));
        }
    }
}

impl App for DemoApp {
    type Spec = DemoSpec;

    fn init(&mut self, cx: &mut uzor_framework::InitCx<'_, Self::Spec>) {
        let ctrl_k = KeyboardShortcut::new(
            ModifierKeys {
                ctrl: true,
                ..ModifierKeys::default()
            },
            KeyCode::K,
        );
        cx.command(AppCommand::Keymap(KeymapCmd::Bind {
            scope: KeymapScope::Global,
            binding: Binding::new(ctrl_k, DemoAction::Palette),
        }));
        cx.command(AppCommand::Keymap(KeymapCmd::Bind {
            scope: KeymapScope::Global,
            binding: Binding::new(KeyboardShortcut::key(KeyCode::Escape), DemoAction::Dismiss),
        }));
    }

    fn chrome_model(&self, _win: WindowId) -> ChromeModel {
        ChromeModel {
            tabs: vec![ChromeTab {
                label: "Demo".into(),
                closable: false,
            }],
            ..ChromeModel::default()
        }
    }

    fn overlay_model(&self, _win: WindowId, id: DemoOverlay) -> OverlayModel {
        match id {
            DemoOverlay::Settings => OverlayModel::Modal(ModalModel::titled("Settings")),
            // The menu and the palette are DOM trees the app paints whole.
            DemoOverlay::Menu | DemoOverlay::Palette => OverlayModel::Custom,
            DemoOverlay::Tooltip => OverlayModel::Tooltip(TooltipModel {
                text: "Drag a header into the gutter to expand".into(),
            }),
        }
    }

    fn panel(&mut self, cx: &mut PanelCx<'_, Self::Spec>) {
        match *cx.panel {
            DemoPanel::Navigator => self.paint_navigator(cx),
            DemoPanel::Editor => {
                let body = look::leaf_body(cx.rect);
                let resp = kit::dom(body, &self.sheet, editor_tree(self.dark, &self.note, body.height)).build(cx);
                for (id, v) in resp.edited {
                    if id == id_note().as_str() {
                        self.note = v;
                    }
                }
            }
            DemoPanel::Inspector => {
                let body = look::leaf_body(cx.rect);
                let panels = self.panels.get(&cx.window).copied().unwrap_or(0);
                let windows = self.keys.len();
                let tree = inspector_tree(self.dark, self.office.len(), windows, panels, body.height);
                kit::dom(body, &self.sheet, tree).build(cx);
            }
        }
    }

    fn overlay_body(&mut self, cx: &mut OverlayCx<'_, Self::Spec>) {
        match cx.id {
            DemoOverlay::Settings => self.paint_settings(cx),
            DemoOverlay::Menu => self.paint_menu(cx),
            DemoOverlay::Palette => self.paint_palette(cx),
            // The kernel draws the tooltip text.
            DemoOverlay::Tooltip => {}
        }
    }

    fn intent(&mut self, intent: Intent<Self::Spec>, cx: &mut IntentCx<'_, Self::Spec>) {
        match intent {
            Intent::Window(uzor_framework::WindowIntent::Opened { win, key }) => {
                self.keys.insert(win, key.as_str().to_string());
                cx.command(AppCommand::Layout(LayoutCmd::SetChrome {
                    win,
                    visible: true,
                    height: 32.0,
                }));
                let panel = match key.as_str() {
                    "main" => {
                        self.mains.insert(win);
                        self.phase.insert(win, 0);
                        DemoPanel::Navigator
                    }
                    "pin" => DemoPanel::Inspector,
                    _ => DemoPanel::Navigator,
                };
                cx.command(AppCommand::Layout(LayoutCmd::OpenPanel {
                    win,
                    panel,
                    at: DockTarget::Root,
                }));
            }
            Intent::Dock(uzor_framework::DockIntent::LayoutChanged { win, .. }) => {
                self.on_layout_changed(win, cx);
            }
            Intent::Dock(uzor_framework::DockIntent::LayoutBlob { win, blob, .. }) => {
                if let Some(key) = self.keys.get(&win).cloned() {
                    self.office.insert(key, blob);
                }
            }
            Intent::Action(DemoAction::Palette) => {
                if let Some(win) = self.mains.iter().next().copied() {
                    cx.command(open_overlay(
                        win,
                        DemoOverlay::Palette,
                        OverlayKind::Dropdown,
                        None,
                        320.0,
                        176.0,
                    ));
                }
            }
            Intent::Action(DemoAction::Theme) => self.toggle_theme(cx),
            Intent::Action(DemoAction::SaveLayout) => {
                if let Some(win) = self.mains.iter().next().copied() {
                    self.save(cx, win);
                }
            }
            Intent::Action(DemoAction::RestoreLayout) => {
                if let Some(win) = self.mains.iter().next().copied() {
                    self.restore(cx, win);
                }
            }
            Intent::Action(DemoAction::Dismiss) => {
                let wins: Vec<WindowId> = self.keys.keys().copied().collect();
                for win in wins {
                    cx.command(AppCommand::Overlay(OverlayCmd::CloseTop { win }));
                }
            }
            _ => {}
        }
    }
}

impl DemoApp {
    fn paint_navigator(&mut self, cx: &mut PanelCx<'_, DemoSpec>) {
        let win = cx.window;
        let body = look::leaf_body(cx.rect);
        let tree = nav_tree(self.dark, self.shortcuts, body.height);
        let resp = kit::dom(body, &self.sheet, tree).build(cx);
        let rect = |id: WidgetId| resp.dom.rect(id.as_str()).unwrap_or_default();
        let (menu_at, tip_at) = (rect(id_menu()), rect(id_tip()));
        if cx.view.clicked(&id_settings()) {
            cx.out.command(open_overlay(win, DemoOverlay::Settings, OverlayKind::Modal, None, 400.0, 250.0));
        }
        if cx.view.clicked(&id_menu()) {
            cx.out.command(open_overlay(win, DemoOverlay::Menu, OverlayKind::ContextMenu, Some(menu_at), 220.0, 150.0));
        }
        if cx.view.clicked(&id_theme()) {
            let mut sink = HookSink(cx.out);
            self.toggle_theme(&mut sink);
        }
        if cx.view.clicked(&id_save()) {
            let mut sink = HookSink(cx.out);
            self.save(&mut sink, win);
        }
        if cx.view.clicked(&id_restore()) {
            let mut sink = HookSink(cx.out);
            self.restore(&mut sink, win);
        }
        if cx.view.is_hovered(&id_tip()) {
            if !cx.view.overlay_open(DemoOverlay::Tooltip) {
                cx.out.command(open_overlay(win, DemoOverlay::Tooltip, OverlayKind::Tooltip, Some(tip_at), 260.0, 36.0));
            }
        } else if cx.view.overlay_open(DemoOverlay::Tooltip) {
            cx.out.command(close_overlay(win, DemoOverlay::Tooltip));
        }
    }

    /// The settings modal body (the kernel draws the frame and title): a focused text
    /// field, the theme as a segmented control and a switch.
    fn paint_settings(&mut self, cx: &mut OverlayCx<'_, DemoSpec>) {
        let win = cx.window;
        if !cx.view.is_focused(&id_settings_note()) && !cx.view.is_focused(&WidgetId::from("demo-set-theme:0")) && !cx.view.is_focused(&WidgetId::from("demo-set-theme:1")) && !cx.view.is_focused(&WidgetId::from("demo-set-keys")) {
            cx.out.command(AppCommand::Focus(FocusCmd::Focus { win, id: id_settings_note() }));
        }
        let r = cx.rect;
        let tree = look::root(self.dark, r.height).child(
            El::new("div.d-modal")
                .css(format!("height: {}px", r.height))
                .child(
                    El::new("div.p-row")
                        .child(look::ico(icons::SETTINGS))
                        .child(El::new("div.p-h1.p-grow").text("Settings"))
                        .child(El::new("button.k-act").id("demo-set-close").tab().name("Close").svg(k::icon(icons::CLOSE))),
                )
                .child(k::field("DISPLAY NAME", k::input(id_settings_note().as_str(), Edit::new("").placeholder("Your name"))))
                .child(El::new("div.p-row").child(El::new("div.p-grow").text("Theme")).child(k::segmented("demo-set-theme", &["Dark", "Light"], usize::from(!self.dark))))
                .child(El::new("div.p-row").child(El::new("div.p-grow").text("Shortcut card in the navigator")).child(k::switch("demo-set-keys", self.shortcuts, "Shortcut card"))),
        );
        let resp = kit::dom(r, &self.sheet, tree).build(cx);
        match resp.clicked.as_deref() {
            Some("demo-set-keys") => self.shortcuts = !self.shortcuts,
            Some("demo-set-close") => cx.out.command(close_overlay(win, DemoOverlay::Settings)),
            Some(hit) => {
                if let Some(i) = k::picked("demo-set-theme", "", hit) {
                    if (i == 0) != self.dark {
                        let mut sink = HookSink(cx.out);
                        self.toggle_theme(&mut sink);
                    }
                }
            }
            None => {}
        }
    }

    /// The panel context menu, a DOM menu painted into the overlay rect.
    fn paint_menu(&mut self, cx: &mut OverlayCx<'_, DemoSpec>) {
        let win = cx.window;
        let items = [
            MenuItem::new("Inspector is pinned").icon(icons::PIN).disabled(true),
            MenuItem::separator(),
            MenuItem::new(if self.dark { "Light theme" } else { "Dark theme" }).icon(if self.dark { icons::SUN } else { icons::MOON }),
            MenuItem::new("Save layout").icon(icons::SAVE),
            MenuItem::new("Close").icon(icons::CLOSE).danger(true),
        ];
        let resp = kit::dom(cx.rect, &self.sheet, pop_tree(self.dark, cx.rect, "demo-menu-list", Some("PANEL"), &items)).build(cx);
        match resp.clicked.as_deref().and_then(|h| k::picked("demo-menu-list", "item", h)) {
            Some(2) => {
                let mut sink = HookSink(cx.out);
                self.toggle_theme(&mut sink);
                sink.cmd(close_overlay(win, DemoOverlay::Menu));
            }
            Some(3) => {
                let mut sink = HookSink(cx.out);
                self.save(&mut sink, win);
                sink.cmd(close_overlay(win, DemoOverlay::Menu));
            }
            Some(4) => cx.out.command(close_overlay(win, DemoOverlay::Menu)),
            _ => {}
        }
    }

    fn paint_palette(&mut self, cx: &mut OverlayCx<'_, DemoSpec>) {
        let win = cx.window;
        let items = [
            MenuItem::new("Toggle theme").icon(if self.dark { icons::SUN } else { icons::MOON }).kbd("T"),
            MenuItem::new("Save layout").icon(icons::SAVE).kbd("Ctrl+S"),
            MenuItem::new("Restore layout").icon(icons::UNDO).kbd("Ctrl+R"),
        ];
        let resp = kit::dom(cx.rect, &self.sheet, pop_tree(self.dark, cx.rect, "demo-palette", Some("COMMAND PALETTE"), &items)).build(cx);
        let hit = resp.clicked.as_deref().and_then(|h| k::picked("demo-palette", "item", h));
        let mut sink = HookSink(cx.out);
        match hit {
            Some(0) => self.toggle_theme(&mut sink),
            Some(1) => self.save(&mut sink, win),
            Some(2) => self.restore(&mut sink, win),
            _ => return,
        }
        sink.cmd(close_overlay(win, DemoOverlay::Palette));
    }
}

/// A popup list (menu, palette) filling `r`: `div.p-pop` with catalog menu rows; the
/// items are hits `{id}:item:{i}`.
fn pop_tree(dark: bool, r: Rect, id: &str, title: Option<&str>, items: &[MenuItem]) -> El {
    // Reuse the catalog rows; drop the fixed placement of `k::menu`.
    let m = k::menu(id, title, items, r);
    let rows = m.children;
    look::root(dark, r.height).child(El::new("div.p-pop").css(format!("height: {}px", r.height)).children(rows))
}

/// Palette row ids (`demo-palette:item:{i}`): theme, save, restore.
#[allow(dead_code)]
pub fn id_palette_item(i: usize) -> WidgetId {
    WidgetId::from(format!("demo-palette:item:{i}").as_str())
}

fn open_overlay(
    win: WindowId,
    id: DemoOverlay,
    kind: OverlayKind,
    anchor: Option<Rect>,
    width: f64,
    height: f64,
) -> AppCommand<DemoSpec> {
    AppCommand::Overlay(OverlayCmd::Open {
        win,
        id,
        kind,
        anchor,
        size: OverlaySize::Fixed { width, height },
        policy: None,
    })
}

fn close_overlay(win: WindowId, id: DemoOverlay) -> AppCommand<DemoSpec> {
    AppCommand::Overlay(OverlayCmd::Close { win, id })
}
