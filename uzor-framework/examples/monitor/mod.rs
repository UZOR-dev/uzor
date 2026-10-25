//! Observe monitor: shells, screen-marked windows, one-shots, burns.
//!
//! The dock is assembled the same way as the framework demo: one panel at
//! the root, then `Split` on `LayoutChanged`. The page pushes JSON; this
//! module only paints it.

use std::sync::Mutex;

use uzor::input::{Sense, WidgetKind};
use uzor::layout::docking::DockPanel;
use uzor::render::{TextAlign, TextBaseline};
use uzor::tokens::{BuiltinSet, TokenTheme, Tokens};
use uzor::ui::widgets::atomic::text::{draw_text, TextOverflow, TextSettings, TextView};
use uzor::{Rect, WidgetId};
use uzor_framework::widgets::{scroll_begin, scroll_end};
use uzor_framework::{
    App, AppCommand, DockIntent, DockTarget, DragOutChrome, DragOutPolicy, Intent, LayoutCmd,
    LayoutPolicy, PanelCx, PanelHome, RuntimeConfig, ScrollStyle, ScrollView, SizePx, Spec,
    SplitDir, WindowId, WindowIntent, WindowSpec,
};

/// Canvas id the page provides.
pub const CANVAS_ID: &str = "hq-monitor";

const ROW_TWO: f64 = 34.0;
const ROW_ONE: f64 = 22.0;
const MARK: &str = "#c4a35a";
const MARK_TEXT: &str = "#e4d2a8";
const DIM: &str = "#8e97a8";
const SELECTED: &str = "#243044";
const HOVER: &str = "#1a2230";

static SNAP: Mutex<Option<serde_json::Value>> = Mutex::new(None);
static WIN: Mutex<Option<WindowId>> = Mutex::new(None);
static HANDLE: Mutex<Option<uzor_framework::Handle<MonitorSpec>>> = Mutex::new(None);

/// Replace the frame the panels paint and wake the window if it exists.
pub fn push_snap(json: &str) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return;
    };
    if let Ok(mut slot) = SNAP.lock() {
        *slot = Some(value);
    }
    let win = WIN.lock().ok().and_then(|g| *g);
    let handle = HANDLE.lock().ok().and_then(|g| g.clone());
    if let (Some(win), Some(handle)) = (win, handle) {
        let _ = handle.request_repaint(win);
    }
}

fn take_snap() -> Option<serde_json::Value> {
    SNAP.lock().ok().and_then(|g| g.clone())
}

/// Four dock panels. Shells is the root; the others are split in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonitorPanel {
    /// Shell cycles, hidden or not. A bar means screen marked a window.
    Shells,
    /// Rows screen marked as a console window.
    Windows,
    /// Shell starts that did not repeat.
    Once,
    /// SLO burns.
    Burns,
}

impl DockPanel for MonitorPanel {
    fn title(&self) -> &str {
        match self {
            MonitorPanel::Shells => "Shells",
            MonitorPanel::Windows => "Windows",
            MonitorPanel::Once => "Once",
            MonitorPanel::Burns => "Burns",
        }
    }

    fn type_id(&self) -> &'static str {
        match self {
            MonitorPanel::Shells => "shells",
            MonitorPanel::Windows => "windows",
            MonitorPanel::Once => "once",
            MonitorPanel::Burns => "burns",
        }
    }

    fn min_size(&self) -> (f32, f32) {
        match self {
            MonitorPanel::Burns => (160.0, 48.0),
            _ => (180.0, 80.0),
        }
    }

    fn closable(&self) -> bool {
        false
    }

    fn preferred_strip_height(&self) -> Option<f32> {
        match self {
            MonitorPanel::Burns => Some(96.0),
            _ => None,
        }
    }
}

/// No overlays. The dock is the whole surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MonitorOverlay {}

/// No keymap. Selection is a click on a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MonitorAction {}

/// Panel / overlay / action vocabulary.
pub struct MonitorSpec;

impl Spec for MonitorSpec {
    type Panel = MonitorPanel;
    type Overlay = MonitorOverlay;
    type Action = MonitorAction;

    fn decode_panel(_home: PanelHome, type_id: &str) -> Option<MonitorPanel> {
        match type_id {
            "shells" => Some(MonitorPanel::Shells),
            "windows" => Some(MonitorPanel::Windows),
            "once" => Some(MonitorPanel::Once),
            "burns" => Some(MonitorPanel::Burns),
            _ => None,
        }
    }
}

struct Line {
    key: String,
    primary: String,
    secondary: String,
    /// Screen marked a window on this row.
    mark: bool,
}

/// The app. Scroll and selection live here; the frame lives in [`SNAP`].
pub struct MonitorApp {
    phase: u8,
    win: Option<WindowId>,
    scrolls: [ScrollView; 4],
    selected: Option<String>,
}

impl MonitorApp {
    /// Empty dock, no frame yet.
    pub fn new() -> Self {
        Self {
            phase: 0,
            win: None,
            scrolls: Default::default(),
            selected: None,
        }
    }

    /// One window. Drag-out stays inside the page (`multi_window: false`).
    pub fn runtime_config() -> RuntimeConfig {
        RuntimeConfig {
            windows: vec![WindowSpec::new("main", "observe", SizePx::new(1280, 800))],
            tokens: Tokens::builtin(BuiltinSet::Dark),
            dark: true,
            layout: LayoutPolicy {
                drag_out: DragOutPolicy::Enabled {
                    chrome: DragOutChrome::UzorChrome,
                },
                edge_expand_px: 16.0,
                expand_thickness_px: 140.0,
                tab_new_button: false,
                ..LayoutPolicy::default()
            },
            ..RuntimeConfig::default()
        }
    }

    fn on_layout(&mut self, win: WindowId, cx: &mut uzor_framework::IntentCx<'_, MonitorSpec>) {
        if self.win != Some(win) {
            return;
        }
        let leaves = cx
            .view
            .window(win)
            .map(|w| w.dock.leaves.clone())
            .unwrap_or_default();
        let find = |id: &str| {
            leaves
                .iter()
                .find(|leaf| leaf.panels.iter().any(|p| p.type_id == id))
                .map(|leaf| leaf.leaf)
        };
        match self.phase {
            0 if leaves.len() == 1 => {
                cx.command(AppCommand::Layout(LayoutCmd::Split {
                    win,
                    leaf: leaves[0].leaf,
                    dir: SplitDir::Right,
                    panel: MonitorPanel::Windows,
                }));
                self.phase = 1;
            }
            1 if leaves.len() >= 2 => {
                if let Some(leaf) = find("windows") {
                    cx.command(AppCommand::Layout(LayoutCmd::Split {
                        win,
                        leaf,
                        dir: SplitDir::Down,
                        panel: MonitorPanel::Once,
                    }));
                    self.phase = 2;
                }
            }
            2 if leaves.len() >= 3 => {
                if let Some(leaf) = find("shells") {
                    cx.command(AppCommand::Layout(LayoutCmd::Split {
                        win,
                        leaf,
                        dir: SplitDir::Down,
                        panel: MonitorPanel::Burns,
                    }));
                    self.phase = 3;
                }
            }
            _ => {}
        }
    }
}

impl App for MonitorApp {
    type Spec = MonitorSpec;

    fn init(&mut self, cx: &mut uzor_framework::InitCx<'_, Self::Spec>) {
        if let Ok(mut slot) = HANDLE.lock() {
            *slot = Some(cx.handle());
        }
    }

    fn chrome_model(&self, _win: WindowId) -> uzor_framework::ChromeModel {
        uzor_framework::ChromeModel {
            tabs: vec![uzor_framework::ChromeTab {
                label: status_line(take_snap().as_ref()),
                closable: false,
            }],
            ..uzor_framework::ChromeModel::default()
        }
    }

    fn overlay_model(&self, _win: WindowId, id: MonitorOverlay) -> uzor_framework::OverlayModel {
        match id {}
    }

    fn panel(&mut self, cx: &mut PanelCx<'_, Self::Spec>) {
        let snap = take_snap();
        let (slot, scroll_id, lines, empty, row_h, two) = match *cx.panel {
            MonitorPanel::Shells => (0, "scroll-shells", shell_lines(snap.as_ref()), "пусто", ROW_TWO, true),
            MonitorPanel::Windows => (1, "scroll-windows", window_lines(snap.as_ref()), "пусто", ROW_TWO, true),
            MonitorPanel::Once => (2, "scroll-once", once_lines(snap.as_ref()), "пусто", ROW_TWO, true),
            MonitorPanel::Burns => (3, "scroll-burns", burn_lines(snap.as_ref()), "пусто", ROW_ONE, false),
        };
        let empty = if snap.is_none() { "нет кадра" } else { empty };
        self.paint_list(cx, slot, scroll_id, &lines, empty, row_h, two);
    }

    fn overlay_body(&mut self, cx: &mut uzor_framework::OverlayCx<'_, Self::Spec>) {
        match cx.id {}
    }

    fn intent(&mut self, intent: Intent<Self::Spec>, cx: &mut uzor_framework::IntentCx<'_, Self::Spec>) {
        match intent {
            Intent::Window(WindowIntent::Opened { win, key }) => {
                if key.as_str() == "main" {
                    self.win = Some(win);
                    self.phase = 0;
                    if let Ok(mut slot) = WIN.lock() {
                        *slot = Some(win);
                    }
                }
                cx.command(AppCommand::Layout(LayoutCmd::SetChrome {
                    win,
                    visible: true,
                    height: 32.0,
                }));
                cx.command(AppCommand::Layout(LayoutCmd::OpenPanel {
                    win,
                    panel: MonitorPanel::Shells,
                    at: DockTarget::Root,
                }));
            }
            Intent::Dock(DockIntent::LayoutChanged { win, .. }) => self.on_layout(win, cx),
            _ => {}
        }
    }
}

impl MonitorApp {
    fn paint_list(
        &mut self,
        cx: &mut PanelCx<'_, MonitorSpec>,
        slot: usize,
        scroll_id: &str,
        lines: &[Line],
        empty: &str,
        row_h: f64,
        two: bool,
    ) {
        // The leaf rect includes the tab the kernel paints. Same inset as the demo.
        let viewport = Rect::new(
            cx.rect.x + 6.0,
            cx.rect.y + 28.0,
            (cx.rect.width - 12.0).max(1.0),
            (cx.rect.height - 34.0).max(1.0),
        );
        let count = lines.len().max(1);
        let content_h = count as f64 * row_h;
        let style = ScrollStyle::default();
        let off = scroll_begin(
            cx,
            scroll_id,
            viewport,
            &mut self.scrolls[slot],
            content_h,
            true,
            &style,
        );
        cx.render.save();
        cx.render.clip_rect(viewport.x, viewport.y, viewport.width, viewport.height);
        let settings = text_settings(cx.tokens);
        if lines.is_empty() {
            let rect = Rect::new(viewport.x + 12.0, viewport.y + 8.0, (viewport.width - 20.0).max(1.0), 20.0);
            label(cx, rect, empty, Some(DIM), &settings);
        } else {
            for (i, line) in lines.iter().enumerate() {
                let y = viewport.y - off + i as f64 * row_h;
                if y + row_h < viewport.y || y > viewport.y + viewport.height {
                    continue;
                }
                let rect = Rect::new(viewport.x + 4.0, y, (viewport.width - 14.0).max(1.0), row_h);
                let hit = WidgetId::from(format!("hit:{scroll_id}:{i}:{}", line.key));
                let clicked = cx.view.clicked(&hit);
                let hovered = cx.view.is_hovered(&hit);
                if clicked {
                    self.selected = Some(line.key.clone());
                }
                let on = self.selected.as_deref() == Some(line.key.as_str());
                if on || hovered {
                    cx.render.set_fill_color(if on { SELECTED } else { HOVER });
                    cx.render.fill_rect(rect.x, rect.y, rect.width, rect.height);
                }
                if line.mark {
                    cx.render.set_fill_color(MARK);
                    cx.render.fill_rect(rect.x + 2.0, rect.y + 4.0, 3.0, (rect.height - 8.0).max(1.0));
                }
                let text_x = rect.x + 10.0;
                let text_w = (rect.width - 16.0).max(1.0);
                if two {
                    label(
                        cx,
                        Rect::new(text_x, y + 1.0, text_w, 16.0),
                        &line.primary,
                        if line.mark { Some(MARK_TEXT) } else { None },
                        &settings,
                    );
                    if !line.secondary.is_empty() {
                        label(cx, Rect::new(text_x, y + 16.0, text_w, 16.0), &line.secondary, Some(DIM), &settings);
                    }
                } else {
                    label(
                        cx,
                        Rect::new(text_x, y, text_w, row_h),
                        &line.primary,
                        if line.mark { Some(MARK_TEXT) } else { None },
                        &settings,
                    );
                }
                let layer = cx.widgets.layer.clone();
                cx.widgets
                    .coord
                    .register_atomic(hit, WidgetKind::Custom, rect, Sense::CLICK, &layer);
            }
        }
        cx.render.restore();
        scroll_end(cx, scroll_id, viewport, &self.scrolls[slot], &style);
    }
}

fn text_settings(tokens: &Tokens) -> TextSettings {
    TextSettings::default().with_theme(Box::new(TokenTheme::from_tokens(tokens)))
}

fn label(cx: &mut PanelCx<'_, MonitorSpec>, rect: Rect, text: &str, color: Option<&str>, settings: &TextSettings) {
    let view = TextView {
        text,
        align: TextAlign::Left,
        baseline: TextBaseline::Middle,
        color,
        font: Some("12px Segoe UI, sans-serif"),
        overflow: TextOverflow::Ellipsis,
        hovered: false,
    };
    draw_text(cx.render, rect, &view, settings);
}

fn status_line(snap: Option<&serde_json::Value>) -> String {
    let Some(snap) = snap else {
        return "observe  нет кадра".to_string();
    };
    let collector = field(snap, "collector");
    let collector = if collector.is_empty() { "?" } else { collector };
    let proven = snap.get("frame_proven").and_then(|v| v.as_bool()).unwrap_or(false);
    let gen = snap.get("gen").and_then(|v| v.as_u64()).unwrap_or(0);
    let shells = snap.get("cycles").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let windows = snap.get("rows").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let frame = if proven { "frame" } else { "no frame" };
    format!("observe  {collector}  {frame}  gen {gen}  {shells} shells  {windows} windows")
}

fn field<'a>(v: &'a serde_json::Value, key: &str) -> &'a str {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("")
}

fn gap(ms: i64) -> String {
    if ms <= 0 {
        "·".to_string()
    } else if ms >= 1000 {
        format!("{}s", ms / 1000)
    } else {
        format!("{ms}ms")
    }
}

fn cmd_tag(template: &str) -> &'static str {
    let t = template.to_ascii_lowercase();
    if t.contains("cmd.exe") || t.starts_with("cmd ") || t.starts_with("\"cmd\"") {
        "cmd  "
    } else {
        ""
    }
}

fn shell_lines(snap: Option<&serde_json::Value>) -> Vec<Line> {
    let Some(rows) = snap.and_then(|s| s.get("cycles")).and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    rows.iter().map(shell_line).collect()
}

fn once_lines(snap: Option<&serde_json::Value>) -> Vec<Line> {
    let Some(rows) = snap.and_then(|s| s.get("once")).and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    rows.iter().map(shell_line).collect()
}

fn shell_line(v: &serde_json::Value) -> Line {
    let who = field(v, "who");
    let template = field(v, "template");
    let who_cmd = field(v, "who_cmd");
    let n = v.get("n").and_then(|x| x.as_u64()).unwrap_or(0);
    let exits = v.get("exits").and_then(|x| x.as_u64()).unwrap_or(0);
    let every = v.get("every_ms").and_then(|x| x.as_i64()).unwrap_or(0);
    let visible = v.get("visible").and_then(|x| x.as_bool()).unwrap_or(false);
    let exit = if exits > 0 { format!("  exit {exits}") } else { String::new() };
    let win = if visible { "win  " } else { "" };
    Line {
        key: format!("{who}\u{1}{template}"),
        primary: format!("{win}{tag}{who}  ×{n}{exit}  {}  {template}", gap(every), tag = cmd_tag(template)),
        secondary: who_cmd.to_string(),
        mark: visible,
    }
}

fn window_lines(snap: Option<&serde_json::Value>) -> Vec<Line> {
    let Some(snap) = snap else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Some(rows) = snap.get("rows").and_then(|v| v.as_array()) {
        for row in rows {
            let exe = field(row, "exe");
            let parent = field(row, "parent");
            let cmd = field(row, "cmdline");
            let ancestor = field(row, "ancestor");
            let parent_cmd = field(row, "parent_cmdline");
            let pid = row.get("pid").and_then(|x| x.as_i64()).unwrap_or(0);
            let via = if !ancestor.is_empty() && ancestor != parent && ancestor != exe {
                format!("{parent} · {ancestor}")
            } else {
                parent.to_string()
            };
            out.push(Line {
                key: format!("w:{pid}:{exe}:{cmd}"),
                primary: format!("{exe}  {pid}  ← {via}  {cmd}"),
                secondary: parent_cmd.to_string(),
                mark: true,
            });
        }
    }
    if out.is_empty() {
        if let Some(rows) = snap.get("exemplars").and_then(|v| v.as_array()) {
            for row in rows {
                if field(row, "kind") != "console" {
                    continue;
                }
                let exe = field(row, "exe");
                let cmd = field(row, "cmdline");
                let ancestor = field(row, "ancestor");
                let pid = row.get("pid").and_then(|x| x.as_i64()).unwrap_or(0);
                out.push(Line {
                    key: format!("c:{pid}:{exe}:{cmd}"),
                    primary: format!("{exe}  {pid}  ← {ancestor}  {cmd}"),
                    secondary: String::new(),
                    mark: true,
                });
            }
        }
    }
    out
}

fn burn_lines(snap: Option<&serde_json::Value>) -> Vec<Line> {
    let Some(rows) = snap.and_then(|s| s.get("burns")).and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    rows.iter()
        .map(|v| {
            let slo = field(v, "slo");
            let detail = field(v, "detail");
            let cnt = v.get("cnt").and_then(|x| x.as_u64()).unwrap_or(0);
            Line {
                key: format!("{slo}\u{1}{detail}"),
                primary: format!("{slo}  ×{cnt}  {detail}"),
                secondary: String::new(),
                mark: false,
            }
        })
        .collect()
}
