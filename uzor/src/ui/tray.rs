//! Tray widgets: small live cards for a tray flyout, a taskbar hover thumbnail or a dock
//! tile, built from the [`dash`](super::dash) blocks. Every builder takes plain data
//! (labels already translated, numbers already formatted), so nothing here knows a
//! particular app.
//!
//! | widget | [`TrayWidget`] | size | shows |
//! |---|---|---|---|
//! | task statuses | `Tasks` | 300×164 | head, up to three tasks (state ring, title, note), counts |
//! | limits | `Limits` | 300×132 | up to four usage rings with label and note |
//! | live agents | `Agents` | 300×168 | up to four agents: critter, name, what it does, status dot |
//! | combined | `Combined` | 320×200 | two rings, two tasks and a crew strip of critters |
//!
//! The look is the dashboard's: monochrome, grey captions, colour only in the small
//! status dots (and the critters). [`TrayLook`] picks dark or light and glass (the
//! flyout, an overlay) or opaque (a thumbnail bitmap). [`render`] lays out and paints a
//! widget; [`paint`] paints a laid-out one with its critters. [`ring_icon`] and
//! [`badge`] make icon-size variants for a tray or dock icon (pure pixels, no renderer).
//!
//! | element / classes | what |
//! |---|---|
//! | `div.tw` (`.glass`) | the card; `--d-*` theme variables inline ([`TrayLook::vars`]) |
//! | `div.tw-head` > `span.d-dot`, `div.tw-title`, `div.tw-cap` | status dot, title, caption |
//! | `div.tw-list` > `div.tw-row` | task / agent rows |
//! | `div.tw-rings` > `div.tw-meter` | rings ([`dash::ring`](super::dash::ring)) |
//! | `div.tw-crew` > `div.tw-mate` | critters with a dot (combined) |
//! | `div.tw-foot` | a muted last line |

use super::dash::{self, Meter, TaskState};
use super::dom::{self, Dom, El};
use super::style::{Sheet, State};
use super::widgets::atomic::critter::{critter_view, draw_critter_motion, CritterFamily, CritterMotion, CritterState};
use crate::platform::types::RgbaIcon;
use crate::render::RenderContext;
use crate::types::Rect;

/// Which tray widget to show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TrayWidget {
    /// Task statuses.
    #[default]
    Tasks,
    /// Usage / limit rings.
    Limits,
    /// Live agents (critters with a status dot).
    Agents,
    /// Rings, tasks and the crew in one card.
    Combined,
}

impl TrayWidget {
    /// Every widget, in menu order.
    pub const ALL: [TrayWidget; 4] = [TrayWidget::Tasks, TrayWidget::Limits, TrayWidget::Agents, TrayWidget::Combined];

    /// A stable key (settings, storage).
    pub fn key(self) -> &'static str {
        match self {
            TrayWidget::Tasks => "tasks",
            TrayWidget::Limits => "limits",
            TrayWidget::Agents => "agents",
            TrayWidget::Combined => "combined",
        }
    }

    /// The widget of a [`key`](Self::key).
    pub fn from_key(k: &str) -> Option<TrayWidget> {
        TrayWidget::ALL.into_iter().find(|w| w.key() == k)
    }

    /// The card size in logical px.
    pub fn size(self) -> (f64, f64) {
        match self {
            TrayWidget::Tasks => (300.0, 164.0),
            TrayWidget::Limits => (300.0, 132.0),
            TrayWidget::Agents => (300.0, 168.0),
            TrayWidget::Combined => (320.0, 200.0),
        }
    }
}

/// The one coloured thing on a tray widget: its status dot (and the icon badge).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TrayStatus {
    /// Nothing runs: a grey dot.
    #[default]
    Idle,
    /// Something works: green.
    Live,
    /// Something waits for the user: amber.
    Attention,
    /// Failing or disconnected: red.
    Error,
}

impl TrayStatus {
    /// Dot class suffix.
    fn class(self) -> &'static str {
        match self {
            TrayStatus::Idle => "span.d-dot",
            TrayStatus::Live => "span.d-dot.live",
            TrayStatus::Attention => "span.d-dot.warn",
            TrayStatus::Error => "span.d-dot.err",
        }
    }

    /// The dot colour as RGBA (icon badges); `None` for idle (no badge).
    pub fn rgba(self) -> Option<[u8; 4]> {
        match self {
            TrayStatus::Idle => None,
            TrayStatus::Live => Some([0x5f, 0xd0, 0x8a, 0xff]),
            TrayStatus::Attention => Some([0xe3, 0xb3, 0x41, 0xff]),
            TrayStatus::Error => Some([0xe0, 0x6a, 0x5f, 0xff]),
        }
    }
}

/// One task line.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrayTask {
    pub key: String,
    pub title: String,
    pub state: TaskState,
    /// A muted right-hand note (assignee, age).
    pub side: String,
}

impl TrayTask {
    pub fn new(key: impl Into<String>, title: impl Into<String>, state: TaskState) -> Self {
        TrayTask { key: key.into(), title: title.into(), state, side: String::new() }
    }
    pub fn side(mut self, s: impl Into<String>) -> Self {
        self.side = s.into();
        self
    }
}

/// One agent: a critter (by name and family) in a state, with a status dot.
#[derive(Clone, Debug, PartialEq)]
pub struct TrayAgent {
    pub key: String,
    pub name: String,
    /// What it does now.
    pub sub: String,
    /// The critter's motion state.
    pub state: CritterState,
    pub family: CritterFamily,
    pub status: TrayStatus,
}

impl TrayAgent {
    pub fn new(key: impl Into<String>, name: impl Into<String>) -> Self {
        TrayAgent { key: key.into(), name: name.into(), sub: String::new(), state: CritterState::Idle, family: CritterFamily::Agent, status: TrayStatus::Idle }
    }
    pub fn sub(mut self, s: impl Into<String>) -> Self {
        self.sub = s.into();
        self
    }
    pub fn state(mut self, s: CritterState) -> Self {
        self.state = s;
        self
    }
    pub fn family(mut self, f: CritterFamily) -> Self {
        self.family = f;
        self
    }
    pub fn status(mut self, s: TrayStatus) -> Self {
        self.status = s;
        self
    }
}

/// What the tray widgets show. The app fills it from its own data.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrayModel {
    /// The app or space name in the head.
    pub title: String,
    /// The head's dot.
    pub status: TrayStatus,
    /// Tasks in display order (running first, say); a widget shows the first few.
    pub tasks: Vec<TrayTask>,
    /// Task counts: running, open, done.
    pub counts: (usize, usize, usize),
    /// Usage meters (5h, week…).
    pub meters: Vec<Meter>,
    /// Agents in display order (working first).
    pub agents: Vec<TrayAgent>,
    /// A muted last line (updated at, connection).
    pub foot: String,
}

impl TrayModel {
    /// Whether a widget of this model moves (a working critter): the host keeps
    /// painting frames while it shows.
    pub fn animating(&self, w: TrayWidget) -> bool {
        matches!(w, TrayWidget::Agents | TrayWidget::Combined) && self.agents.iter().any(|a| matches!(a.state, CritterState::Working | CritterState::Thinking | CritterState::Attention))
    }
}

/// Translated words of the tray widgets.
#[derive(Clone, Debug, PartialEq)]
pub struct TrayLabels {
    pub tasks: String,
    pub limits: String,
    pub agents: String,
    pub no_tasks: String,
    pub no_limits: String,
    pub no_agents: String,
    /// `{n}` running.
    pub running: String,
    /// `{n}` open.
    pub open: String,
    /// `{n}` done.
    pub done: String,
    /// `{n}` working (agents).
    pub working: String,
    /// `+{n}` more.
    pub more: String,
}

impl Default for TrayLabels {
    fn default() -> Self {
        TrayLabels::ru()
    }
}

impl TrayLabels {
    /// Russian (the default).
    pub fn ru() -> Self {
        TrayLabels {
            tasks: "Задачи".into(),
            limits: "Лимиты".into(),
            agents: "Агенты".into(),
            no_tasks: "Задач нет".into(),
            no_limits: "Лимиты ещё не пришли".into(),
            no_agents: "Агентов нет".into(),
            running: "{n} в работе".into(),
            open: "{n} открыто".into(),
            done: "{n} готово".into(),
            working: "{n} работают".into(),
            more: "+{n}".into(),
        }
    }

    /// English.
    pub fn en() -> Self {
        TrayLabels {
            tasks: "Tasks".into(),
            limits: "Limits".into(),
            agents: "Agents".into(),
            no_tasks: "No tasks".into(),
            no_limits: "No limits reported yet".into(),
            no_agents: "No agents".into(),
            running: "{n} running".into(),
            open: "{n} open".into(),
            done: "{n} done".into(),
            working: "{n} working".into(),
            more: "+{n}".into(),
        }
    }
}

fn n(tpl: &str, v: usize) -> String {
    tpl.replace("{n}", &v.to_string())
}

/// How a tray widget looks: dark or light, glass (an overlay: the flyout) or opaque
/// (a thumbnail, a dock tile), and the glass itself.
#[derive(Clone, Debug, PartialEq)]
pub struct TrayLook {
    pub dark: bool,
    pub glass: bool,
    /// Glass fill; `None`: the theme's.
    pub glass_bg: Option<String>,
    /// Glass hairline; `None`: the theme's.
    pub glass_line: Option<String>,
    /// Corner radius, px.
    pub radius: f64,
}

impl Default for TrayLook {
    fn default() -> Self {
        TrayLook { dark: true, glass: true, glass_bg: None, glass_line: None, radius: 14.0 }
    }
}

impl TrayLook {
    /// The flyout look (glass) in a theme.
    pub fn flyout(dark: bool) -> Self {
        TrayLook { dark, ..Default::default() }
    }
    /// The opaque look (thumbnails) in a theme.
    pub fn solid(dark: bool) -> Self {
        TrayLook { dark, glass: false, ..Default::default() }
    }

    /// The card's inline variables: the `--d-*` palette of the theme and the glass.
    pub fn vars(&self) -> String {
        let (pal, bg, line) = if self.dark {
            (
                "--d-card: #1c1c1e; --d-line: #2c2c2e; --d-fg: #f2f2f2; --d-sub: #9a9a9a; --d-faint: #6b6b6b; --d-hover: #262626; --d-track: #343436; --d-ring: #e6e6e6; --tw-crit-bg: #2a2a2c",
                "rgba(30,30,32,0.78)",
                "rgba(255,255,255,0.12)",
            )
        } else {
            (
                "--d-card: #ffffff; --d-line: #e4e4e7; --d-fg: #18181b; --d-sub: #6b6b70; --d-faint: #a1a1a6; --d-hover: #f2f2f4; --d-track: #e6e6ea; --d-ring: #26262a; --tw-crit-bg: #f0f0f2",
                "rgba(248,248,250,0.80)",
                "rgba(0,0,0,0.10)",
            )
        };
        format!(
            "{pal}; --tw-bg: {}; --tw-line: {}; border-radius: {}px",
            if self.glass { self.glass_bg.as_deref().unwrap_or(bg) } else { "var(--d-card)" },
            if self.glass { self.glass_line.as_deref().unwrap_or(line) } else { "var(--d-line)" },
            self.radius
        )
    }
}

/// The tray widgets' rules (on top of [`KIT_CSS`](super::domkit::KIT_CSS) and
/// [`DASH_CSS`](super::dash::DASH_CSS)).
pub const TRAY_CSS: &str = r#"
div.tw { background: var(--tw-bg); border: 1px solid var(--tw-line); color: var(--d-fg); font-size: 12px; font-family: Inter, sans-serif;
         padding: 12px 14px; display: flex; flex-direction: column; gap: 8px; overflow: hidden }
div.tw.glass { backdrop-filter: blur(24px); box-shadow: 0 12px 32px rgba(0,0,0,0.28) }
div.tw-head { display: flex; align-items: center; gap: 7px; height: 16px; flex-shrink: 0 }
div.tw-title { color: var(--d-sub); font-size: 11px; font-weight: 600; flex-shrink: 0 }
div.tw-cap { color: var(--d-faint); font-size: 11px; min-width: 0 }
div.tw-grow { flex-grow: 1 }
span.d-dot.warn { background: #e3b341 }
span.d-dot.err { background: #e06a5f }
div.tw-list { display: flex; flex-direction: column; gap: 2px; min-height: 0 }
div.tw-row { display: flex; align-items: center; gap: 8px; height: 24px; flex-shrink: 0; min-width: 0 }
div.tw-row > span.tw-st { width: 11px; height: 11px; border-radius: 6px; border: 1.5px solid var(--d-faint); flex-shrink: 0 }
div.tw-row > span.tw-st.run { border-color: var(--d-fg) }
div.tw-row > span.tw-st.done { background: var(--d-sub); border-color: var(--d-sub) }
div.tw-row > div.tw-t { flex-grow: 1; min-width: 0; color: var(--d-fg) }
div.tw-row.done > div.tw-t { color: var(--d-faint) }
div.tw-row > div.tw-side { color: var(--d-sub); font-size: 11px; flex-shrink: 0 }
div.tw-row > span.tw-av { width: 20px; height: 20px; flex-shrink: 0 }
div.tw-row > div.tw-name { color: var(--d-fg); flex-shrink: 0 }
div.tw-row > div.tw-sub { color: var(--d-sub); font-size: 11px; flex-grow: 1; min-width: 0 }
div.tw-empty { color: var(--d-faint); font-size: 12px; padding: 4px 0 }
div.tw-foot { color: var(--d-faint); font-size: 11px; flex-shrink: 0 }
div.tw-rings { display: flex; gap: 10px; justify-content: space-around; flex-grow: 1; align-items: center }
div.tw-meter { display: flex; flex-direction: column; align-items: center; gap: 3px; min-width: 52px }
div.tw-meter > span.d-ring { width: 46px; height: 46px; font-size: 11px }
div.tw-meter > div.tw-ml { color: var(--d-sub); font-size: 11px; text-align: center }
div.tw-meter > div.tw-mn { color: var(--d-faint); font-size: 10px; text-align: center }
div.tw-split { display: flex; gap: 12px; min-height: 0; flex-grow: 1 }
div.tw-split > div.tw-rings { flex-grow: 0; flex-direction: column; gap: 6px; justify-content: flex-start; align-items: stretch }
div.tw-mini { display: flex; align-items: center; gap: 8px }
div.tw-mini > span.d-ring { width: 30px; height: 30px; font-size: 0px; --ring-width: 3px }
div.tw-mini > div.tw-mv { display: flex; flex-direction: column }
div.tw-mini div.tw-mval { color: var(--d-fg); font-size: 12px; font-weight: 600 }
div.tw-mini div.tw-ml { color: var(--d-sub); font-size: 10px }
div.tw-split > div.tw-list { flex-grow: 1; min-width: 0 }
div.tw-vr { width: 1px; background: var(--d-line); flex-shrink: 0 }
div.tw-crew { display: flex; align-items: center; gap: 8px; height: 26px; flex-shrink: 0 }
div.tw-mate { width: 22px; height: 22px; position: relative; flex-shrink: 0 }
div.tw-mate > span.tw-av { width: 22px; height: 22px }
div.tw-mate > span.d-dot { position: absolute; right: -2px; bottom: -1px; width: 7px; height: 7px; border: 1.5px solid var(--d-card) }
div.tw-crew > div.tw-cap { margin-left: 2px }
"#;

/// [`KIT_CSS`](super::domkit::KIT_CSS) + [`DASH_CSS`](super::dash::DASH_CSS) + [`TRAY_CSS`].
pub fn sheet() -> Sheet {
    Sheet::parse(&[super::domkit::KIT_CSS, dash::DASH_CSS, TRAY_CSS].concat()).expect("TRAY_CSS parses")
}

/// The slot key of agent `i`'s critter.
fn critter_slot(i: usize) -> String {
    format!("@tw-critter:{i}")
}

fn head(title: &str, status: TrayStatus, cap: &str) -> El {
    El::new("div.tw-head")
        .child(El::new(status.class()))
        .child(El::new("div.tw-title").text(title))
        .child(El::new("div.tw-grow"))
        .child_if(!cap.is_empty(), || El::new("div.tw-cap").text(cap))
}

fn task_row(t: &TrayTask) -> El {
    let (row, st) = match t.state {
        TaskState::Running => ("div.tw-row", "span.tw-st.run"),
        TaskState::Done => ("div.tw-row.done", "span.tw-st.done"),
        TaskState::Open => ("div.tw-row", "span.tw-st"),
    };
    El::new(row)
        .name(&t.title)
        .child(El::new(st))
        .child(El::new("div.tw-t").text(&t.title))
        .child_if(!t.side.is_empty(), || El::new("div.tw-side").text(&t.side))
}

fn agent_row(i: usize, a: &TrayAgent) -> El {
    El::new("div.tw-row")
        .name(&a.name)
        .child(El::new("span.tw-av").slot(critter_slot(i)))
        .child(El::new("div.tw-name").text(&a.name))
        .child(El::new("div.tw-sub").text(&a.sub))
        .child(El::new(a.status.class()))
}

fn counts(m: &TrayModel, l: &TrayLabels) -> String {
    let (r, o, d) = m.counts;
    let mut parts = Vec::new();
    if r > 0 {
        parts.push(n(&l.running, r));
    }
    if o > 0 {
        parts.push(n(&l.open, o));
    }
    if d > 0 {
        parts.push(n(&l.done, d));
    }
    parts.join(" · ")
}

fn working(m: &TrayModel) -> usize {
    m.agents.iter().filter(|a| a.status == TrayStatus::Live).count()
}

/// A tray widget card (`div.tw`) at its [`size`](TrayWidget::size). Critters are embed
/// slots: paint the laid-out tree with [`paint`].
pub fn tray_widget(w: TrayWidget, m: &TrayModel, l: &TrayLabels, look: &TrayLook) -> El {
    let (wd, ht) = w.size();
    let root = El::new(if look.glass { "div.tw.glass" } else { "div.tw" }).id("tw").name(&m.title).css(format!("{}; width: {wd}px; height: {ht}px", look.vars()));
    let title = if m.title.is_empty() { match w {
        TrayWidget::Tasks => &l.tasks,
        TrayWidget::Limits => &l.limits,
        TrayWidget::Agents | TrayWidget::Combined => &l.agents,
    } } else { &m.title };
    match w {
        TrayWidget::Tasks => {
            let shown = 3;
            let rest = m.tasks.len().saturating_sub(shown);
            root.child(head(title, m.status, &l.tasks))
                .child(if m.tasks.is_empty() {
                    El::new("div.tw-empty").text(&l.no_tasks)
                } else {
                    El::new("div.tw-list").children(m.tasks.iter().take(shown).map(task_row))
                })
                .child(El::new("div.tw-grow"))
                .child(El::new("div.tw-foot").text({
                    let c = counts(m, l);
                    if rest > 0 { format!("{c}{}{}", if c.is_empty() { "" } else { " · " }, n(&l.more, rest)) } else { c }
                }))
        }
        TrayWidget::Limits => root.child(head(title, m.status, &l.limits)).child(if m.meters.is_empty() {
            El::new("div.tw-empty").text(&l.no_limits)
        } else {
            El::new("div.tw-rings").children(m.meters.iter().take(4).map(|me| {
                El::new("div.tw-meter")
                    .name(&me.label)
                    .child(dash::ring(me.frac, &me.value, false))
                    .child(El::new("div.tw-ml").text(&me.label))
                    .child_if(!me.note.is_empty(), || El::new("div.tw-mn").text(&me.note))
            }))
        }),
        TrayWidget::Agents => {
            let shown = 4;
            let rest = m.agents.len().saturating_sub(shown);
            let wk = working(m);
            root.child(head(title, m.status, &if wk > 0 { n(&l.working, wk) } else { l.agents.clone() }))
                .child(if m.agents.is_empty() {
                    El::new("div.tw-empty").text(&l.no_agents)
                } else {
                    El::new("div.tw-list").children(m.agents.iter().take(shown).enumerate().map(|(i, a)| agent_row(i, a)))
                })
                .child_if(rest > 0, || El::new("div.tw-foot").text(n(&l.more, rest)))
        }
        TrayWidget::Combined => {
            let rings = El::new("div.tw-rings").children(m.meters.iter().take(2).map(|me| {
                El::new("div.tw-mini")
                    .name(&me.label)
                    .child(dash::ring(me.frac, "", true))
                    .child(El::new("div.tw-mv").child(El::new("div.tw-mval").text(&me.value)).child(El::new("div.tw-ml").text(&me.label)))
            }));
            let tasks = if m.tasks.is_empty() {
                El::new("div.tw-list").child(El::new("div.tw-empty").text(&l.no_tasks))
            } else {
                El::new("div.tw-list").children(m.tasks.iter().take(3).map(task_row))
            };
            let shown = 7;
            let rest = m.agents.len().saturating_sub(shown);
            let crew = El::new("div.tw-crew")
                .children(m.agents.iter().take(shown).enumerate().map(|(i, a)| {
                    El::new("div.tw-mate").name(&a.name).child(El::new("span.tw-av").slot(critter_slot(i))).child(El::new(a.status.class()))
                }))
                .child_if(rest > 0, || El::new("div.tw-cap").text(n(&l.more, rest)))
                .child_if(m.agents.is_empty(), || El::new("div.tw-cap").text(&l.no_agents));
            let wk = working(m);
            root.child(head(title, m.status, &counts(m, l)))
                .child(
                    El::new("div.tw-split")
                        .child_if(!m.meters.is_empty(), || rings)
                        .child_if(!m.meters.is_empty(), || El::new("div.tw-vr"))
                        .child(tasks),
                )
                .child(crew)
                .child_if(!m.foot.is_empty() || wk > 0, || El::new("div.tw-foot").text(if m.foot.is_empty() { n(&l.working, wk) } else { m.foot.clone() }))
        }
    }
}

/// Paints one agent's creature into its slot: `(ctx, rect, agent, t seconds, reduced motion)`.
pub type CritterPainter = std::rc::Rc<dyn Fn(&mut dyn RenderContext, Rect, &TrayAgent, f64, bool)>;

thread_local! {
    static CRITTER_PAINTER: std::cell::RefCell<Option<CritterPainter>> = const { std::cell::RefCell::new(None) };
}

/// Draw the agents' creatures with the app's own painter instead of uzor's critter (an app
/// whose sidebar creatures have their own shapes and expressions shows the same ones in the
/// tray). Per thread: set it on the thread that paints the widgets (a native host's event
/// loop thread, where the flyout and the taskbar thumbnail are drawn). `None` restores
/// uzor's critter. [`TrayAgent::key`] is passed through as the creature's identity.
pub fn set_critter_painter(p: Option<CritterPainter>) {
    CRITTER_PAINTER.with(|c| *c.borrow_mut() = p);
}

/// Paint a laid-out tray widget with its critters at time `t` (seconds).
pub fn paint(ctx: &mut dyn RenderContext, d: &Dom, m: &TrayModel, t: f64, reduced: bool) {
    dom::paint_slots(ctx, d, &mut |ctx, key, r| paint_slot(ctx, key, r, m, t, reduced));
}

/// Paint one embed slot of a tray widget (a critter) into `r`; other keys are left alone.
/// For a host that lays the widget out itself (a kit `dom` panel: call it for each of
/// `resp.dom.slots()`).
pub fn paint_slot(ctx: &mut dyn RenderContext, key: &str, r: Rect, m: &TrayModel, t: f64, reduced: bool) {
    let Some(i) = key.strip_prefix("@tw-critter:").and_then(|s| s.parse::<usize>().ok()) else { return };
    let Some(a) = m.agents.get(i) else { return };
    if let Some(f) = CRITTER_PAINTER.with(|p| p.borrow().clone()) {
        f(ctx, r, a, t, reduced);
        return;
    }
    let mut v = critter_view(&a.name, a.family);
    v.off = a.state == CritterState::Off;
    draw_critter_motion(ctx, r, &v, &CritterMotion { state: a.state, t, look_at: None, reduced });
}

/// Lay out and paint widget `w` with its top-left at `(x, y)`; the laid-out tree back.
#[allow(clippy::too_many_arguments)]
pub fn render(ctx: &mut dyn RenderContext, sheet: &Sheet, w: TrayWidget, m: &TrayModel, l: &TrayLabels, look: &TrayLook, x: f64, y: f64, t: f64) -> Dom {
    let (wd, ht) = w.size();
    let tree = tray_widget(w, m, l, look);
    let d = dom::lay_out_in(ctx, sheet, &tree, x, y, wd, Some(Rect::new(x, y, wd, ht)), &|_| State::NONE);
    paint(ctx, &d, m, t, false);
    d
}

// ---------------------------------------------------------------------------
// Icon-size variants (pure pixels)
// ---------------------------------------------------------------------------

fn blend(px: &mut [u8], c: [u8; 4], cov: f64) {
    let a = (c[3] as f64 / 255.0) * cov.clamp(0.0, 1.0);
    if a <= 0.0 {
        return;
    }
    let da = px[3] as f64 / 255.0;
    let oa = a + da * (1.0 - a);
    for k in 0..3 {
        let s = c[k] as f64;
        let d = px[k] as f64;
        px[k] = if oa > 0.0 { ((s * a + d * da * (1.0 - a)) / oa).round().clamp(0.0, 255.0) as u8 } else { 0 };
    }
    px[3] = (oa * 255.0).round() as u8;
}

/// A disc of `c` at (`cx`, `cy`) radius `r` (anti-aliased), with an optional `ring`
/// px cut-out border of transparency around it (so a badge reads on any icon).
fn disc(img: &mut RgbaIcon, cx: f64, cy: f64, r: f64, c: [u8; 4], cut: f64) {
    let (w, h) = (img.width as i64, img.height as i64);
    let reach = r + cut + 1.0;
    let (x0, x1) = (((cx - reach).floor() as i64).max(0), ((cx + reach).ceil() as i64).min(w));
    let (y0, y1) = (((cy - reach).floor() as i64).max(0), ((cy + reach).ceil() as i64).min(h));
    for y in y0..y1 {
        for x in x0..x1 {
            let d = ((x as f64 + 0.5 - cx).powi(2) + (y as f64 + 0.5 - cy).powi(2)).sqrt();
            let i = ((y * w + x) * 4) as usize;
            let px = &mut img.pixels[i..i + 4];
            if cut > 0.0 {
                // Knock the icon out around the dot.
                let k = (r + cut + 0.5 - d).clamp(0.0, 1.0);
                if k > 0.0 {
                    let keep = 1.0 - k;
                    px[3] = (px[3] as f64 * keep).round() as u8;
                }
            }
            blend(px, c, r + 0.5 - d);
        }
    }
}

/// `icon` with a status dot in its bottom-right corner (a quarter of the icon wide,
/// with a thin transparent cut-out). `None`: the icon unchanged.
pub fn badge(icon: &RgbaIcon, dot: Option<[u8; 4]>) -> RgbaIcon {
    let mut out = icon.clone();
    let Some(c) = dot else { return out };
    let s = icon.width.min(icon.height) as f64;
    let r = (s * 0.17).max(2.0);
    let cut = (s * 0.06).max(1.0);
    disc(&mut out, icon.width as f64 - r - cut * 0.5, icon.height as f64 - r - cut * 0.5, r, c, cut);
    out
}

/// An icon-size widget, `px` square: a usage ring filled to `frac` (clockwise from the
/// top) around a centre dot (the status, grey when idle). Monochrome but for the dot.
pub fn ring_icon(px: u32, frac: f64, dark: bool, status: TrayStatus) -> RgbaIcon {
    let s = px.max(8) as f64;
    let mut img = RgbaIcon::from_rgba(px.max(8), px.max(8), vec![0; (px.max(8) * px.max(8) * 4) as usize]);
    let (fg, track) = if dark { ([0xee, 0xee, 0xee, 0xff], [0xee, 0xee, 0xee, 0x55]) } else { ([0x22, 0x22, 0x26, 0xff], [0x22, 0x22, 0x26, 0x40]) };
    let frac = if frac.is_finite() { frac.clamp(0.0, 1.0) } else { 0.0 };
    let (c, rr, w) = (s / 2.0, s / 2.0 - s * 0.09, (s * 0.13).max(1.5));
    let w_ = img.width as usize;
    for y in 0..img.height as usize {
        for x in 0..w_ {
            let (dx, dy) = (x as f64 + 0.5 - c, y as f64 + 0.5 - c);
            let d = (dx * dx + dy * dy).sqrt();
            let cov = (w / 2.0 + 0.5 - (d - rr).abs()).clamp(0.0, 1.0);
            if cov <= 0.0 {
                continue;
            }
            // Angle from 12 o'clock, clockwise, 0..1.
            let a = (dx.atan2(-dy) / std::f64::consts::TAU).rem_euclid(1.0);
            let col = if a <= frac && frac > 0.0 { fg } else { track };
            let i = (y * w_ + x) * 4;
            blend(&mut img.pixels[i..i + 4], col, cov);
        }
    }
    let dot = status.rgba().unwrap_or(if dark { [0x9a, 0x9a, 0x9a, 0xff] } else { [0x6b, 0x6b, 0x70, 0xff] });
    disc(&mut img, c, c, s * 0.16, dot, 0.0);
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_app_painter_draws_the_creatures() {
        use std::cell::Cell;
        use std::rc::Rc;
        let seen = Rc::new(Cell::new(0usize));
        let s2 = seen.clone();
        set_critter_painter(Some(Rc::new(move |_ctx: &mut dyn RenderContext, _r: Rect, a: &TrayAgent, _t: f64, _red: bool| {
            assert_eq!(a.key, "k1");
            s2.set(s2.get() + 1);
        })));
        let m = TrayModel { agents: vec![TrayAgent::new("k1", "one")], ..Default::default() };
        let mut ctx = crate::testing::RecordingRenderContext::new();
        paint_slot(&mut ctx, "@tw-critter:0", Rect::new(0.0, 0.0, 20.0, 20.0), &m, 0.0, false);
        paint_slot(&mut ctx, "@tw-critter:5", Rect::new(0.0, 0.0, 20.0, 20.0), &m, 0.0, false);
        set_critter_painter(None);
        paint_slot(&mut ctx, "@tw-critter:0", Rect::new(0.0, 0.0, 20.0, 20.0), &m, 0.0, false);
        assert_eq!(seen.get(), 1);
    }
    use crate::ui::dom::lay_out_in;

    fn model() -> TrayModel {
        TrayModel {
            title: "Hatchery".into(),
            status: TrayStatus::Live,
            tasks: vec![
                TrayTask::new("1", "Release 1.5.33", TaskState::Running).side("scout"),
                TrayTask::new("2", "Tray widgets", TaskState::Open),
                TrayTask::new("3", "Docs", TaskState::Done),
                TrayTask::new("4", "Later", TaskState::Open),
            ],
            counts: (1, 2, 1),
            meters: vec![Meter::new("5h", 0.34).note("2 h"), Meter::new("7d", 0.61)],
            agents: vec![TrayAgent::new("a", "scout").sub("tests").state(CritterState::Working).status(TrayStatus::Live), TrayAgent::new("b", "rev")],
            foot: String::new(),
        }
    }

    fn lay(w: TrayWidget, m: &TrayModel) -> Dom {
        let mut ctx = crate::testing::NullRenderContext::default();
        let (wd, ht) = w.size();
        lay_out_in(&mut ctx, &sheet(), &tray_widget(w, m, &TrayLabels::ru(), &TrayLook::default()), 0.0, 0.0, wd, Some(Rect::new(0.0, 0.0, wd, ht)), &|_| State::NONE)
    }

    #[test]
    fn keys_round_trip_and_sizes_are_compact() {
        for w in TrayWidget::ALL {
            assert_eq!(TrayWidget::from_key(w.key()), Some(w));
            let (wd, ht) = w.size();
            assert!((280.0..=320.0).contains(&wd) && (120.0..=200.0).contains(&ht));
        }
        assert_eq!(TrayWidget::from_key("nope"), None);
    }

    #[test]
    fn every_widget_lays_out_at_its_size_with_critter_slots() {
        let m = model();
        for w in TrayWidget::ALL {
            let d = lay(w, &m);
            let r = d.rect("tw").unwrap();
            assert_eq!((r.width, r.height), w.size());
        }
        let d = lay(TrayWidget::Agents, &m);
        let slots: Vec<_> = d.boxes.iter().filter_map(|b| b.slot.clone()).collect();
        assert_eq!(slots, vec!["@tw-critter:0".to_string(), "@tw-critter:1".to_string()]);
        assert!(m.animating(TrayWidget::Agents) && !m.animating(TrayWidget::Tasks));
    }

    #[test]
    fn glass_only_on_the_flyout_look() {
        let m = model();
        let mut ctx = crate::testing::NullRenderContext::default();
        let d = lay_out_in(&mut ctx, &sheet(), &tray_widget(TrayWidget::Tasks, &m, &TrayLabels::en(), &TrayLook::solid(false)), 0.0, 0.0, 300.0, None, &|_| State::NONE);
        assert_eq!(d.get("tw").unwrap().style.backdrop_blur, None);
        let d = lay(TrayWidget::Tasks, &m);
        assert_eq!(d.get("tw").unwrap().style.backdrop_blur, Some(24.0));
    }

    #[test]
    fn empty_model_says_so() {
        let m = TrayModel::default();
        for w in TrayWidget::ALL {
            let d = lay(w, &m);
            assert!(d.rect("tw").is_some());
        }
    }

    #[test]
    fn icon_ring_and_badge() {
        let i = ring_icon(32, 0.5, true, TrayStatus::Live);
        assert_eq!(i.pixels.len(), 32 * 32 * 4);
        // Centre is the green dot.
        let c = ((16 * 32 + 16) * 4) as usize;
        assert_eq!(&i.pixels[c..c + 3], &[0x5f, 0xd0, 0x8a]);
        // Right edge middle (a quarter turn, inside the filled half) is bright; left is track.
        let right = ((16 * 32 + 29) * 4) as usize;
        let left = ((16 * 32 + 2) * 4) as usize;
        assert!(i.pixels[right + 3] > i.pixels[left + 3]);
        let plain = RgbaIcon::from_rgba(16, 16, vec![255; 16 * 16 * 4]);
        assert_eq!(badge(&plain, None).pixels, plain.pixels);
        let b = badge(&plain, TrayStatus::Error.rgba());
        let k = ((13 * 16 + 13) * 4) as usize;
        assert_eq!(&b.pixels[k..k + 3], &[0xe0, 0x6a, 0x5f]);
    }
}
