//! The L4 dashboard on the `uzor-framework` [`App`], on the DOM engine.
//!
//! One window, five dock leaves. The Settings leaf is a DOM page (theme segmented
//! control, checkbox, switch, a "new window" button, live stats). The four cells are
//! DOM cards that each embed a canvas slot: a sparkline painted with the render
//! context into the box the DOM laid out (`Dom::slots`). Their titles keep the old
//! cadence names; `panel` runs for every visible leaf of a frame, so they share the
//! window clock. With "Live repaint" off the window paints on demand
//! (`TickRate::Dirty`), so the rebuild counters only move on input.
//!
//! Left on the old door, so not in this crate: `uzor-desktop` render control
//! (backend, vsync, MSAA, fps limit), the tree-debug blackbox, tray and the png icon,
//! and per-cell `RenderRegion` cadence.

use std::collections::{BTreeMap, VecDeque};

use uzor::layout::docking::DockPanel;
use uzor::render::{RenderContext, TickRate};
use uzor::tokens::{BuiltinSet, Tokens};
use uzor::ui::dom::El;
use uzor::ui::domkit::{self as k, icons};
use uzor::ui::style::Sheet;
use uzor::{CornerStyle, Rect};

use uzor_framework::widgets::kit;
use uzor_framework::{
    App, AppCommand, ChromeModel, ChromeTab, DockIntent, DockTarget, Intent, IntentCx, LayoutCmd,
    LeafView, OverlayCx, OverlayModel, PanelCx, PanelHome, RuntimeConfig, SizePx, Spec, SplitDir,
    ThemeCmd, WindowCmd, WindowId, WindowIntent, WindowSpec,
};

use crate::look;

const ID_SETTINGS: &str = "settings";
const ID_DIRTY: &str = "paint:r0_dirty";
const ID_30: &str = "paint:r1_30fps";
const ID_120: &str = "paint:r2_120fps";
const ID_UNCAP: &str = "paint:r3_uncap";

/// Samples a sparkline keeps.
const SPARK: usize = 90;

/// Dashboard classes on top of the shared look.
const CSS: &str = r#"
div.d-note { color: var(--k-muted); font-size: 12px; white-space: normal; flex-shrink: 0 }
div.d-spark { flex-grow: 1; border-radius: 10px; background: var(--k-panel); border: 1px solid var(--k-line); padding: 8px 10px;
              display: flex; flex-direction: column; gap: 6px; min-height: 40px }
div.k-slot.d-chart { border: none; border-radius: 0px; flex-grow: 1 }
div.d-legend { display: flex; gap: 10px; color: var(--k-muted); font-size: 11px; height: 14px; flex-shrink: 0 }
"#;

/// The five dock leaves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DashPanel {
    /// Theme, checkbox, toggle.
    Settings,
    /// Old dirty-driven cell.
    PaintDirty,
    /// Old 30 fps cell.
    Paint30,
    /// Old 120 fps cell.
    Paint120,
    /// Old uncapped cell.
    PaintUncap,
}

impl DashPanel {
    fn from_type_id(type_id: &str) -> Option<Self> {
        Some(match type_id {
            ID_SETTINGS => Self::Settings,
            ID_DIRTY => Self::PaintDirty,
            ID_30 => Self::Paint30,
            ID_120 => Self::Paint120,
            ID_UNCAP => Self::PaintUncap,
            _ => return None,
        })
    }

    fn paint_index(self) -> Option<usize> {
        Some(match self {
            Self::PaintDirty => 0,
            Self::Paint30 => 1,
            Self::Paint120 => 2,
            Self::PaintUncap => 3,
            Self::Settings => return None,
        })
    }

    fn kind(self) -> &'static str {
        match self {
            Self::PaintDirty => "ok",
            Self::Paint30 => "info",
            Self::Paint120 => "warn",
            _ => "error",
        }
    }

    fn color(self) -> &'static str {
        match self {
            Self::PaintDirty => "#22c55e",
            Self::Paint30 => "#3b82f6",
            Self::Paint120 => "#f59e0b",
            _ => "#ef4444",
        }
    }

    fn blurb(self) -> &'static str {
        match self {
            Self::PaintDirty => "dirty-driven",
            Self::Paint30 => "30 fps",
            Self::Paint120 => "120 fps",
            Self::PaintUncap => "uncapped",
            Self::Settings => "",
        }
    }
}

impl DockPanel for DashPanel {
    fn title(&self) -> &str {
        match self {
            Self::Settings => "Settings",
            Self::PaintDirty => "fps = 0 (dirty)",
            Self::Paint30 => "fps = 30",
            Self::Paint120 => "fps = 120",
            Self::PaintUncap => "uncapped",
        }
    }

    fn type_id(&self) -> &'static str {
        match self {
            Self::Settings => ID_SETTINGS,
            Self::PaintDirty => ID_DIRTY,
            Self::Paint30 => ID_30,
            Self::Paint120 => ID_120,
            Self::PaintUncap => ID_UNCAP,
        }
    }

    fn min_size(&self) -> (f32, f32) {
        match self {
            Self::Settings => (200.0, 160.0),
            _ => (120.0, 80.0),
        }
    }

    fn closable(&self) -> bool {
        false
    }
}

/// No overlays. Modal / popup / dropdown / context-menu tags are a
/// compile error after M1; this dashboard does not open any.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DashOverlay {}

/// No keymap. Theme is the two settings buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DashAction {}

/// App vocabulary.
#[derive(Clone, Copy, Debug, Default)]
pub struct DashSpec;

impl Spec for DashSpec {
    type Panel = DashPanel;
    type Overlay = DashOverlay;
    type Action = DashAction;

    fn decode_panel(_home: PanelHome, type_id: &str) -> Option<DashPanel> {
        DashPanel::from_type_id(type_id)
    }
}

/// The dashboard.
pub struct DashboardApp {
    dark: bool,
    show_counts: bool,
    live: bool,
    counts: [u64; 4],
    /// The last [`SPARK`] signal samples of each cell, one per paint.
    spark: [VecDeque<f64>; 4],
    /// 0 = settings only, then one split per step, 4 = grid complete.
    phase: BTreeMap<WindowId, u8>,
    extra: u32,
    sheet: Sheet,
    /// Hit rects of the last settings page (test door, see [`DashboardApp::hit`]).
    hits: BTreeMap<String, Rect>,
    /// Cells whose last paint laid out a chart slot.
    charts: [bool; 4],
}

impl Default for DashboardApp {
    fn default() -> Self {
        Self::new()
    }
}

impl DashboardApp {
    /// Dark theme, rebuild counters visible, live repaint off.
    pub fn new() -> Self {
        Self {
            dark: true,
            show_counts: true,
            live: false,
            counts: [0; 4],
            spark: Default::default(),
            phase: BTreeMap::new(),
            extra: 0,
            sheet: look::sheet(CSS),
            hits: BTreeMap::new(),
            charts: [false; 4],
        }
    }

    /// One undecorated window, dark tokens, painted on demand. Same size as the old
    /// L4 bin.
    pub fn runtime_config() -> RuntimeConfig {
        RuntimeConfig {
            windows: vec![main_window()],
            tokens: Tokens::builtin(BuiltinSet::Dark),
            dark: true,
            tick: TickRate::Dirty,
            ..RuntimeConfig::default()
        }
    }

    /// `true` while the dark token set is the one the buttons last asked for.
    pub fn is_dark(&self) -> bool {
        self.dark
    }

    /// Where the settings page last laid out element `id` (window coordinates).
    pub fn hit(&self, id: &str) -> Option<Rect> {
        self.hits.get(id).copied()
    }

    /// How many cells laid out their sparkline slot on their last paint.
    pub fn chart_slots(&self) -> usize {
        self.charts.iter().filter(|c| **c).count()
    }

    fn settings_tree(&self, h: f64, dt_ms: f64) -> El {
        let total: u64 = self.counts.iter().sum();
        let theme = if self.dark { "Dark" } else { "Light" };
        look::root(self.dark, h).child(
            El::new("div.p-page")
                .css(format!("height: {h}px"))
                .child(
                    El::new("div.p-row")
                        .child(look::ico(icons::GRID, true))
                        .child(El::new("div.p-h1").text("Dashboard"))
                        .child(El::new("div.p-grow"))
                        .child(k::badge("DOM", "info")),
                )
                .child(El::new("div.d-note").text(
                    "Five dock leaves. This page is DOM; every cell is a DOM card with a canvas slot painted by the app.",
                ))
                .child(El::new("div.p-sec").text("THEME"))
                .child(k::segmented("dash-theme", &["Dark", "Light"], usize::from(!self.dark)))
                .child(El::new("div.p-sec").text("OPTIONS"))
                .child(
                    El::new("div.p-card")
                        .child(k::checkbox("settings:counts", self.show_counts, "Rebuild counters"))
                        .child(
                            El::new("div.p-row")
                                .child(k::switch("settings:live", self.live, "Live repaint"))
                                .child(El::new("div").text("Live repaint"))
                                .child(El::new("div.p-grow"))
                                .child(k::badge(if self.live { "60 fps" } else { "on demand" }, if self.live { "warn" } else { "ok" })),
                        ),
                )
                .child(El::new("div.p-sec").text("WINDOWS"))
                .child(
                    El::new("div.p-row")
                        .child(k::button("settings:new", "New window"))
                        .child(El::new("div.p-muted").text(format!("{} extra", self.extra))),
                )
                .child(
                    El::new("div.p-grid2")
                        .child(k::stat("settings:frame", "FRAME", &format!("{dt_ms:.1} ms"), &format!("theme {theme}")))
                        .child(k::stat("settings:total", "REBUILDS", &total.to_string(), "all cells")),
                ),
        )
    }

    fn paint_settings(&mut self, cx: &mut PanelCx<'_, DashSpec>) {
        let body = look::leaf_body(cx.rect);
        let tree = self.settings_tree(body.height, cx.time.dt * 1000.0);
        let resp = kit::dom(body, &self.sheet, tree).build(cx);
        self.hits = resp.dom.hits().map(|(id, r)| (id.to_string(), r)).collect();
        if let Some(c) = resp.clicked.as_deref() {
            match c {
                "settings:counts" => self.show_counts = !self.show_counts,
                "settings:live" => self.live = !self.live,
                "settings:new" => {
                    self.extra = self.extra.wrapping_add(1);
                    cx.out.command(AppCommand::Window(WindowCmd::Open(extra_window(self.extra))));
                }
                _ => {
                    if let Some(i) = k::picked("dash-theme", "", c) {
                        self.set_theme(cx, i == 0);
                    }
                }
            }
            cx.out.request_frame();
        }
        if self.live {
            cx.out.request_frame();
        }
    }

    fn set_theme(&mut self, cx: &mut PanelCx<'_, DashSpec>, dark: bool) {
        if dark == self.dark {
            return;
        }
        self.dark = dark;
        let set = if dark { BuiltinSet::Dark } else { BuiltinSet::Light };
        cx.out.command(AppCommand::Theme(ThemeCmd::SetTokens(Tokens::builtin(set))));
        cx.out.command(AppCommand::Theme(ThemeCmd::SetDark(dark)));
    }

    fn paint_cell(&mut self, cx: &mut PanelCx<'_, DashSpec>, panel: DashPanel) {
        let Some(idx) = panel.paint_index() else {
            return;
        };
        self.counts[idx] = self.counts[idx].wrapping_add(1);
        let t = cx.time.now.0;
        let f = 0.6 + idx as f64 * 0.45;
        let v = (0.5 + 0.32 * (t * f + idx as f64).sin() + 0.12 * (t * f * 3.7).sin()).clamp(0.0, 1.0);
        let spark = &mut self.spark[idx];
        spark.push_back(v);
        while spark.len() > SPARK {
            spark.pop_front();
        }
        let count = self.counts[idx];
        let body = look::leaf_body(cx.rect);
        let h = body.height;
        let key = format!("spark:{idx}");
        let tree = look::root(self.dark, h).child(
            El::new("div.p-page")
                .css(format!("height: {h}px"))
                .child(
                    El::new("div.p-row")
                        .child(look::ico(icons::CHART, false))
                        .child(El::new("div.p-h1").text(panel.title()))
                        .child(El::new("div.p-grow"))
                        .child(k::badge(panel.blurb(), panel.kind())),
                )
                .child_if(self.show_counts && h > 220.0, || {
                    El::new("div.p-grid2")
                        .child(k::stat(&format!("{}:count", panel.type_id()), "REBUILDS", &count.to_string(), "paints of this leaf"))
                        .child(k::stat(&format!("{}:last", panel.type_id()), "SIGNAL", &format!("{:.0}%", v * 100.0), "last sample"))
                })
                .child(
                    El::new("div.d-spark")
                        .child(
                            El::new("div.d-legend")
                                .child(El::new("div").text(format!("last {} paints", self.spark[idx].len())))
                                .child(El::new("div.p-grow"))
                                .child(El::new("div").text(if self.show_counts { format!("#{count}") } else { String::new() })),
                        )
                        .child(El::new("div.k-slot.d-chart").slot(&key)),
                ),
        );
        let resp = kit::dom(body, &self.sheet, tree).build(cx);
        self.charts[idx] = false;
        for (slot, r) in resp.dom.slots() {
            if slot == key {
                self.charts[idx] = true;
                paint_spark(cx.render, r, &self.spark[idx], panel.color(), self.dark);
            }
        }
    }

    fn on_layout(&mut self, win: WindowId, cx: &mut IntentCx<'_, DashSpec>) {
        let Some(phase) = self.phase.get(&win).copied() else {
            return;
        };
        let leaves = cx
            .view
            .window(win)
            .map(|w| w.dock.leaves.clone())
            .unwrap_or_default();
        let has = |id: &str| leaf_with(&leaves, id).is_some();
        let mut split = |type_id: &str, dir: SplitDir, panel: DashPanel| -> bool {
            let Some(leaf) = leaf_with(&leaves, type_id) else {
                return false;
            };
            cx.command(AppCommand::Layout(LayoutCmd::Split {
                win,
                leaf,
                dir,
                panel,
            }));
            true
        };
        let next = match phase {
            0 if has(ID_SETTINGS) && !has(ID_DIRTY) => {
                split(ID_SETTINGS, SplitDir::Right, DashPanel::PaintDirty).then_some(1)
            }
            1 if has(ID_DIRTY) && !has(ID_120) => {
                split(ID_DIRTY, SplitDir::Down, DashPanel::Paint120).then_some(2)
            }
            2 if has(ID_DIRTY) && !has(ID_30) => {
                split(ID_DIRTY, SplitDir::Right, DashPanel::Paint30).then_some(3)
            }
            3 if has(ID_120) && !has(ID_UNCAP) => {
                split(ID_120, SplitDir::Right, DashPanel::PaintUncap).then_some(4)
            }
            _ => None,
        };
        if let Some(next) = next {
            self.phase.insert(win, next);
        }
    }
}

impl App for DashboardApp {
    type Spec = DashSpec;

    fn chrome_model(&self, _win: WindowId) -> ChromeModel {
        ChromeModel {
            tabs: vec![ChromeTab {
                label: "Dashboard".into(),
                closable: false,
            }],
            ..ChromeModel::default()
        }
    }

    fn overlay_model(&self, _win: WindowId, id: DashOverlay) -> OverlayModel {
        match id {}
    }

    fn panel(&mut self, cx: &mut PanelCx<'_, DashSpec>) {
        match *cx.panel {
            DashPanel::Settings => self.paint_settings(cx),
            other => self.paint_cell(cx, other),
        }
    }

    fn overlay_body(&mut self, cx: &mut OverlayCx<'_, DashSpec>) {
        match cx.id {}
    }

    fn intent(&mut self, intent: Intent<DashSpec>, cx: &mut IntentCx<'_, DashSpec>) {
        match intent {
            Intent::Window(WindowIntent::Opened { win, .. }) => {
                if self.phase.insert(win, 0).is_none() {
                    cx.command(AppCommand::Layout(LayoutCmd::SetChrome {
                        win,
                        visible: true,
                        height: 32.0,
                    }));
                    cx.command(AppCommand::Layout(LayoutCmd::OpenPanel {
                        win,
                        panel: DashPanel::Settings,
                        at: DockTarget::Root,
                    }));
                }
            }
            Intent::Window(WindowIntent::NewWindowRequested { .. }) => {
                self.extra = self.extra.wrapping_add(1);
                let spec = extra_window(self.extra);
                cx.command(AppCommand::Window(WindowCmd::Open(spec)));
            }
            Intent::Window(WindowIntent::Closed { win }) => {
                self.phase.remove(&win);
            }
            Intent::Dock(DockIntent::LayoutChanged { win, .. }) => self.on_layout(win, cx),
            Intent::Action(action) => match action {},
            Intent::Overlay(_)
            | Intent::Window(_)
            | Intent::Dock(_)
            | Intent::Text(_)
            | Intent::Drop(_)
            | Intent::PasteImage(_)
            | Intent::Timer(_)
            | Intent::Screenshot { .. }
            | Intent::Tray(_)
            | Intent::Unhandled(_) => {}
        }
    }
}

fn main_window() -> WindowSpec {
    let mut spec = WindowSpec::new("main", "uzor — L4 Dashboard", SizePx::new(1400, 900));
    spec.min_inner_size = Some(SizePx::new(900, 600));
    spec.decorations = false;
    spec.corner_style = CornerStyle::Rounded;
    spec.border_color = Some(0xFB_B2_6A);
    spec
}

fn extra_window(n: u32) -> WindowSpec {
    let mut spec = WindowSpec::new(format!("extra-{n}"), "uzor — extra", SizePx::new(560, 420));
    spec.min_inner_size = Some(SizePx::new(420, 320));
    spec.decorations = false;
    spec.corner_style = CornerStyle::Rounded;
    spec
}

fn leaf_with(leaves: &[LeafView], type_id: &str) -> Option<uzor::layout::docking::LeafId> {
    leaves.iter().find_map(|leaf| {
        leaf.panels
            .iter()
            .any(|p| p.type_id == type_id)
            .then_some(leaf.leaf)
    })
}

/// A filled sparkline of `samples` (0 … 1) in `r`, newest on the right.
fn paint_spark(ctx: &mut dyn RenderContext, r: Rect, samples: &VecDeque<f64>, color: &str, dark: bool) {
    if r.width < 4.0 || r.height < 4.0 {
        return;
    }
    ctx.save();
    ctx.clip_rect(r.x, r.y, r.width, r.height);
    // Grid: quarter lines.
    ctx.set_stroke_color(if dark { "#2a2d33" } else { "#e3e5e8" });
    ctx.set_stroke_width(1.0);
    for q in 1..4 {
        let y = (r.y + r.height * q as f64 / 4.0).round() + 0.5;
        ctx.begin_path();
        ctx.move_to(r.x, y);
        ctx.line_to(r.x + r.width, y);
        ctx.stroke();
    }
    if samples.len() >= 2 {
        let step = r.width / (SPARK - 1) as f64;
        let x0 = r.x + r.width - step * (samples.len() - 1) as f64;
        let y_of = |v: f64| r.y + 4.0 + (1.0 - v) * (r.height - 8.0);
        ctx.begin_path();
        ctx.move_to(x0, r.y + r.height);
        for (i, v) in samples.iter().enumerate() {
            ctx.line_to(x0 + step * i as f64, y_of(*v));
        }
        ctx.line_to(r.x + r.width, r.y + r.height);
        ctx.close_path();
        ctx.set_global_alpha(0.18);
        ctx.set_fill_color(color);
        ctx.fill();
        ctx.set_global_alpha(1.0);
        ctx.begin_path();
        for (i, v) in samples.iter().enumerate() {
            let (x, y) = (x0 + step * i as f64, y_of(*v));
            if i == 0 {
                ctx.move_to(x, y);
            } else {
                ctx.line_to(x, y);
            }
        }
        ctx.set_stroke_color(color);
        ctx.set_stroke_width(2.0);
        ctx.stroke();
        if let Some(v) = samples.back() {
            ctx.set_fill_color(color);
            ctx.begin_path();
            ctx.arc(r.x + r.width - 3.0, y_of(*v), 3.0, 0.0, std::f64::consts::TAU);
            ctx.fill();
        }
    }
    ctx.restore();
}

#[cfg(test)]
mod tests {
    use super::*;

    use uzor::input::{ModifierKeys, MouseButton};
    use uzor_framework::{HeadlessHost, InputEvent, Point, PointerInput};

    fn panel_ids(host: &HeadlessHost<DashSpec, DashboardApp>) -> Vec<&'static str> {
        let snap = host.snapshot();
        let mut ids = Vec::new();
        for win in &snap.windows {
            for leaf in &win.dock.leaves {
                for panel in &leaf.panels {
                    ids.push(panel.type_id);
                }
            }
        }
        ids
    }

    fn settings_rect(host: &HeadlessHost<DashSpec, DashboardApp>) -> Rect {
        let snap = host.snapshot();
        let win = snap.windows.first().expect("window");
        win.dock
            .leaves
            .iter()
            .find(|leaf| leaf.panels.iter().any(|p| p.type_id == ID_SETTINGS))
            .expect("settings leaf")
            .rect
    }

    fn open_settled() -> HeadlessHost<DashSpec, DashboardApp> {
        let (mut host, _) = HeadlessHost::new(DashboardApp::new(), DashboardApp::runtime_config());
        for _ in 0..8 {
            host.advance(0.02);
            if panel_ids(&host).len() >= 5 {
                break;
            }
        }
        host
    }

    fn click(host: &mut HeadlessHost<DashSpec, DashboardApp>, x: f64, y: f64) {
        let win = host.snapshot().windows[0].id;
        host.input(
            win,
            InputEvent::Pointer(PointerInput::Down {
                pos: Point::new(x, y),
                button: MouseButton::Left,
                mods: ModifierKeys::default(),
            }),
        );
        host.advance(0.02);
        host.input(
            win,
            InputEvent::Pointer(PointerInput::Up {
                pos: Point::new(x, y),
                button: MouseButton::Left,
                mods: ModifierKeys::default(),
            }),
        );
        host.advance(0.02);
        // The frame that reports the click to the hook, then the tick that
        // drains the ThemeCmd the hook queued.
        host.advance(0.02);
        host.advance(0.02);
    }

    #[test]
    fn dashboard_opens_settings_and_paint_grid() {
        let host = open_settled();
        let mut ids = panel_ids(&host);
        ids.sort_unstable();
        let mut expect = vec![ID_120, ID_30, ID_DIRTY, ID_SETTINGS, ID_UNCAP];
        expect.sort_unstable();
        assert_eq!(ids, expect, "dock leaves after settle");
        let rect = settings_rect(&host);
        assert!(rect.width > 100.0 && rect.height > 100.0, "{rect:?}");
        assert!(host.runtime().app().is_dark());
    }

    #[test]
    fn dashboard_theme_buttons_switch_token_set() {
        let mut host = open_settled();
        let before = host.snapshot().theme_rev;
        let rect = settings_rect(&host);
        let (x, y) = center(&host, "dash-theme:1");
        assert!(
            y > rect.y + 24.0,
            "light button {y} overlaps the header of {rect:?}"
        );
        click(&mut host, x, y);
        assert!(
            !host.runtime().app().is_dark(),
            "light button at ({x}, {y}) rect {rect:?}"
        );
        assert!(host.snapshot().theme_rev > before);

        let (x, y) = center(&host, "dash-theme:0");
        click(&mut host, x, y);
        assert!(host.runtime().app().is_dark());
    }

    fn center(host: &HeadlessHost<DashSpec, DashboardApp>, id: &str) -> (f64, f64) {
        let r = host.runtime().app().hit(id).unwrap_or_else(|| panic!("{id} not on the settings page"));
        (r.x + r.width * 0.5, r.y + r.height * 0.5)
    }

    #[test]
    fn dashboard_switches_and_new_window() {
        let mut host = open_settled();
        let (x, y) = center(&host, "settings:counts");
        click(&mut host, x, y);
        assert!(!host.runtime().app().show_counts, "checkbox toggles the counters");
        let (x, y) = center(&host, "settings:live");
        click(&mut host, x, y);
        assert!(host.runtime().app().live, "switch turns live repaint on");
        let (x, y) = center(&host, "settings:new");
        click(&mut host, x, y);
        for _ in 0..6 {
            host.advance(0.02);
        }
        assert_eq!(host.snapshot().windows.len(), 2, "new window button opens a window");
    }

    #[test]
    fn dashboard_cells_embed_a_chart_slot() {
        let host = open_settled();
        assert_eq!(host.runtime().app().chart_slots(), 4, "every cell lays out its sparkline slot");
    }

}
