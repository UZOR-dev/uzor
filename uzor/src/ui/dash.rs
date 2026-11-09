//! Dashboard widgets for the DOM engine: compact, monochrome blocks for a control
//! surface of spaces, agents and tasks. Every builder takes plain data from the caller
//! (labels already translated, numbers already formatted), so nothing here knows
//! about a particular app; [`DASH_CSS`] styles them on top of
//! [`KIT_CSS`](super::domkit::KIT_CSS).
//!
//! | widget | builder | element / classes | hit ids |
//! |---|---|---|---|
//! | space switcher | [`space_switcher`], [`space_menu`] | `button.d-switch`; glass `div.d-pop.d-menu` > `div.d-mi` (`.on`) | `id`; `{id}:item:{i}`, `{id}:new` |
//! | summary tile | [`tile`], [`tiles`] | `div.d-tile` > label, `div.d-big`, note, [`hist`] | `{id}:{i}` (in [`tiles`]) |
//! | mini histogram | [`hist`] | `div.d-hist` > `div.d-hb` (`.hi`) | — |
//! | live agents | [`live_agents`] | `div.d-agents` > `div.d-group` > `div.d-node` > `div.d-agent` | `{id}:agent:{key}` |
//! | task list / tree | [`task_list`], [`flatten_tree`] | `div.d-tasks` > `div.d-task` (`.done`, `.d-depth-N`) | `{id}:row:{key}`, `{id}:fold:{key}` |
//! | limit rings | [`rings`], [`ring`] | `div.d-rings` > `div.d-meter` > `span.d-ring` + [`Control::Ring`] | — |
//! | task card | [`task_card`] | `div.d-card` > head, facts, progress, actions | `id`, `{id}:act:{key}` |
//! | mini board | [`mini_board`] | `div.d-board` > `div.d-col` > `div.d-bcard` | `{id}:card:{key}` |
//! | 24h activity | [`activity`] | `div.d-act` > `div.d-bars` > `div.d-bar` | — |
//! | mini panel | [`mini_panel`] | `div.d-mini` (`.widget`, `.collapsed`, `.rail`) | `{id}:expand`, `{id}:collapse`, `{id}:open` |
//! | glass overlay | [`glass`], [`popover`] | `div.d-glass` wrapper, `div.d-pop` | `{id}:close` |
//! | provider glyph | [`glyph`], [`provider_glyph`] | `span.d-glyph` (monochrome svg or a letter) | — |
//!
//! Embedded widgets are opaque (`--d-card` on `--d-line`); only overlays are glass:
//! `--k-glass-bg`, `--k-glass-line` and `--k-glass-blur` theme them (a
//! `backdrop-filter: blur()` box, see [`Style::backdrop_blur`](super::style::Style::backdrop_blur)).

use std::collections::HashSet;

use super::dom::{Control, El};
use super::domkit::{icon, icons, KIT_CSS};
use super::style::{Sheet, StyleError};
use crate::types::Rect;

/// The dashboard look: neutral greys, one green live dot. Theme through the variables.
pub const DASH_CSS: &str = r#"
:root { --d-bg: #141414; --d-card: #1f1f1f; --d-line: #2a2a2a; --d-fg: #f5f5f5; --d-sub: #9a9a9a; --d-faint: #6b6b6b;
        --d-hover: #262626; --d-on: #2b2b2b; --d-bar: #3a3a3a; --d-bar-hi: #d6d6d6; --d-live: #5fd08a; --d-ring: #d6d6d6;
        --d-track: #2e2e2e; --d-radius: 12px;
        --k-glass-bg: rgba(32,32,34,0.72); --k-glass-line: rgba(255,255,255,0.12); --k-glass-blur: 28px;
        --k-glass-scrim: rgba(0,0,0,0.30); --k-glass-shadow: rgba(0,0,0,0.45) }
div.d-root { color: var(--d-fg); font-size: 13px; font-family: Inter, sans-serif }
div.d-box { background: var(--d-card); border: 1px solid var(--d-line); border-radius: var(--d-radius); padding: 12px 14px;
            display: flex; flex-direction: column; gap: 8px; min-width: 0 }
div.d-title { display: flex; align-items: center; gap: 8px; color: var(--d-sub); font-size: 12px; font-weight: 500 }
div.d-title > div.d-grow { flex-grow: 1 }
div.d-cap { color: var(--d-sub); font-size: 11px }
div.d-faint { color: var(--d-faint); font-size: 11px }
span.d-dot { width: 7px; height: 7px; border-radius: 4px; background: var(--d-faint); flex-shrink: 0 }
span.d-dot.live { background: var(--d-live) }
span.d-glyph { width: 14px; height: 14px; color: var(--d-sub); flex-shrink: 0; font-size: 9px; font-weight: 700; text-align: center;
               shape-rendering: geometricPrecision }
span.d-glyph.letter { border: 1px solid var(--d-faint); border-radius: 4px; line-height: 12px; color: var(--d-sub) }
span.d-chev { width: 12px; height: 12px; color: var(--d-faint); flex-shrink: 0 }
button.d-switch { display: flex; align-items: center; gap: 8px; height: 32px; padding: 0 10px; border-radius: 9px;
                  background: var(--d-card); border: 1px solid var(--d-line); color: var(--d-fg); font-weight: 500 }
button.d-switch:hover { background: var(--d-hover) }
button.d-switch > span.d-av { width: 20px; height: 20px; border-radius: 6px; background: var(--d-on); color: var(--d-fg);
                              font-size: 11px; font-weight: 600; text-align: center; line-height: 20px }
button.d-switch > div.d-sw-name { flex-grow: 1 }
button.d-switch > div.d-sw-sub { color: var(--d-sub); font-size: 11px }
div.d-pop { position: fixed; z-index: 130; padding: 6px; border-radius: 12px; background: var(--d-card); border: 1px solid var(--d-line);
            box-shadow: 0 12px 32px var(--k-shadow); display: flex; flex-direction: column; gap: 2px }
div.d-pop > div.d-pop-head { display: flex; align-items: center; padding: 4px 6px 6px 8px; color: var(--d-sub); font-size: 11px; font-weight: 600 }
div.d-pop > div.d-pop-head > div.d-grow { flex-grow: 1 }
div.d-pop button.k-act { width: 22px; height: 22px; padding: 4px }
div.d-mi { display: flex; align-items: center; gap: 8px; padding: 7px 8px; border-radius: 8px; flex-shrink: 0 }
div.d-mi:hover { background: rgba(255,255,255,0.06) }
div.d-mi.on { background: rgba(255,255,255,0.09) }
div.d-mi > span.d-av { width: 20px; height: 20px; border-radius: 6px; background: rgba(255,255,255,0.08); font-size: 11px; font-weight: 600;
                       text-align: center; line-height: 20px }
div.d-mi > div.d-mi-main { flex-grow: 1; min-width: 0; display: flex; flex-direction: column }
div.d-mi div.d-mi-sub { color: var(--d-sub); font-size: 11px }
div.d-mi > span.d-tick { width: 14px; height: 14px; color: var(--d-fg) }
div.d-mi.new { color: var(--d-sub) }
div.d-msep { height: 1px; margin: 4px 6px; background: var(--k-glass-line); flex-shrink: 0 }
div.d-tiles { display: grid; gap: 10px }
div.d-tile { background: var(--d-card); border: 1px solid var(--d-line); border-radius: var(--d-radius); padding: 12px 14px 10px;
             display: flex; flex-direction: column; gap: 2px; min-width: 0 }
div.d-tile:hover { border-color: #353535 }
div.d-big { font-size: 26px; font-weight: 600; line-height: 32px; color: var(--d-fg) }
div.d-tile > div.d-hist { margin-top: 6px }
div.d-hist { display: flex; align-items: flex-end; gap: 2px; height: 26px }
div.d-hb { flex-grow: 1; flex-basis: 0; border-radius: 1px; background: var(--d-bar) }
div.d-hb.hi { background: var(--d-bar-hi) }
div.d-agents { display: flex; flex-direction: column; gap: 10px }
div.d-group { display: flex; flex-direction: column; gap: 2px }
div.d-group-head { display: flex; align-items: center; gap: 6px; color: var(--d-sub); font-size: 11px; font-weight: 600; padding: 0 2px 2px }
div.d-group-head > div.d-grow { flex-grow: 1 }
div.d-node { display: flex; align-items: center; gap: 6px; color: var(--d-faint); font-size: 11px; padding: 4px 2px 0 }
div.d-agent { display: flex; align-items: center; gap: 8px; height: 30px; padding: 0 8px; border-radius: 8px; flex-shrink: 0 }
div.d-agent:hover { background: var(--d-hover) }
div.d-agent > div.d-a-main { flex-grow: 1; min-width: 0; display: flex; align-items: center; gap: 8px }
div.d-agent div.d-a-name { color: var(--d-fg); flex-shrink: 0 }
div.d-agent div.d-a-sub { color: var(--d-sub); font-size: 12px; flex-grow: 1; min-width: 0 }
div.d-agent > div.d-a-side { color: var(--d-faint); font-size: 11px }
div.d-empty { color: var(--d-faint); font-size: 12px; padding: 6px 2px }
div.d-tasks { display: flex; flex-direction: column; gap: 1px }
div.d-task { display: flex; align-items: center; gap: 8px; height: 30px; padding: 0 8px; border-radius: 8px; flex-shrink: 0 }
div.d-task:hover { background: var(--d-hover) }
div.d-task.sel { background: var(--d-on) }
div.d-task > span.d-fold { width: 14px; height: 14px; color: var(--d-faint); flex-shrink: 0; border-radius: 3px }
div.d-task > span.d-fold:hover { color: var(--d-fg) }
div.d-task > span.d-nofold { width: 14px; height: 14px; flex-shrink: 0 }
div.d-task > span.d-st { width: 12px; height: 12px; border-radius: 6px; border: 1.5px solid var(--d-faint); flex-shrink: 0 }
div.d-task > span.d-st.run { border-color: var(--d-fg) }
div.d-task > span.d-st.done { background: var(--d-sub); border-color: var(--d-sub) }
div.d-task > div.d-t-title { flex-grow: 1; min-width: 0; color: var(--d-fg) }
div.d-task.done > div.d-t-title { color: var(--d-faint) }
div.d-task > div.d-t-side { color: var(--d-sub); font-size: 11px; flex-shrink: 0 }
div.d-task > div.d-t-kids { display: flex; align-items: center; gap: 6px; color: var(--d-sub); font-size: 11px; flex-shrink: 0 }
div.d-pbar { width: 36px; height: 3px; border-radius: 2px; background: var(--d-track); display: flex }
div.d-pbar > div.d-pfill { height: 3px; border-radius: 2px; background: var(--d-bar-hi) }
div.d-rings { display: flex; gap: 14px; flex-wrap: wrap }
div.d-meter { display: flex; flex-direction: column; align-items: center; gap: 6px; min-width: 64px }
span.d-ring { width: 54px; height: 54px; color: var(--d-fg); font-size: 12px; font-weight: 600; text-align: center;
              --ring-track: var(--d-track); --ring-color: var(--d-ring); --ring-width: 4px }
span.d-ring.small { width: 30px; height: 30px; font-size: 0px; --ring-width: 3px }
div.d-meter > div.d-m-label { color: var(--d-sub); font-size: 11px; text-align: center }
div.d-meter > div.d-m-note { color: var(--d-faint); font-size: 10px; text-align: center }
div.d-card { background: var(--d-card); border: 1px solid var(--d-line); border-radius: var(--d-radius); padding: 12px 14px;
             display: flex; flex-direction: column; gap: 8px; min-width: 0 }
div.d-card:hover { border-color: #353535 }
div.d-card-head { display: flex; align-items: center; gap: 8px }
div.d-card-head > div.d-card-title { flex-grow: 1; min-width: 0; font-weight: 600; font-size: 14px }
div.d-card-head > div.d-card-status { color: var(--d-sub); font-size: 11px }
div.d-card-body { color: #c8c8c8; white-space: normal; line-clamp: 3 }
div.d-facts { display: grid; grid-template-columns: 1fr 1fr; column-gap: 12px; row-gap: 4px }
div.d-fact { display: flex; flex-direction: column; min-width: 0 }
div.d-fact > div.d-f-label { color: var(--d-faint); font-size: 11px }
div.d-fact > div.d-f-value { color: var(--d-fg); font-size: 12px }
div.d-prog { display: flex; align-items: center; gap: 8px; color: var(--d-sub); font-size: 11px }
div.d-prog > div.d-pbar { flex-grow: 1; width: auto }
div.d-card-acts { display: flex; gap: 6px }
button.d-btn { height: 26px; padding: 0 10px; border-radius: 7px; background: var(--d-on); color: var(--d-fg); font-size: 12px; border: 1px solid var(--d-line) }
button.d-btn:hover { background: #333333 }
button.d-btn.ghost { background: transparent }
div.d-board { display: grid; gap: 8px }
div.d-col { display: flex; flex-direction: column; gap: 6px; min-width: 0 }
div.d-col-head { display: flex; align-items: center; gap: 6px; color: var(--d-sub); font-size: 11px; font-weight: 600; padding: 0 2px }
div.d-col-head > div.d-grow { flex-grow: 1 }
div.d-bcard { background: var(--d-card); border: 1px solid var(--d-line); border-radius: 9px; padding: 8px 10px; display: flex; flex-direction: column; gap: 4px }
div.d-bcard:hover { border-color: #3a3a3a }
div.d-bcard > div.d-bc-title { color: var(--d-fg); font-size: 12px; white-space: normal; line-clamp: 2 }
div.d-bcard > div.d-bc-meta { display: flex; align-items: center; gap: 6px; color: var(--d-sub); font-size: 11px }
div.d-bcard > div.d-bc-meta > div.d-grow { flex-grow: 1; min-width: 0 }
div.d-act { display: flex; flex-direction: column; gap: 6px }
div.d-bars { display: flex; align-items: flex-end; gap: 3px; height: 56px }
div.d-bar { flex-grow: 1; flex-basis: 0; border-radius: 2px; background: var(--d-bar) }
div.d-bar.hi { background: var(--d-bar-hi) }
div.d-axis { display: flex; justify-content: space-between; color: var(--d-faint); font-size: 10px }
div.d-mini { background: var(--d-card); border: 1px solid var(--d-line); border-radius: var(--d-radius); display: flex; flex-direction: column;
             gap: 10px; padding: 12px 14px; min-width: 0 }
div.d-mini-head { display: flex; align-items: center; gap: 8px }
div.d-mini-head > div.d-mini-name { flex-grow: 1; min-width: 0; display: flex; flex-direction: column }
div.d-mini-head div.d-mini-title { font-weight: 600; color: var(--d-fg) }
div.d-mini-head div.d-mini-kind { color: var(--d-sub); font-size: 11px }
div.d-mini-head button.k-act { width: 24px; height: 24px; padding: 5px; color: var(--d-sub) }
div.d-kpis { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 8px }
div.d-kpi { display: flex; flex-direction: column; min-width: 0 }
div.d-kpi > div.d-kpi-v { font-size: 18px; font-weight: 600; color: var(--d-fg); line-height: 22px }
div.d-kpi > div.d-kpi-l { color: var(--d-sub); font-size: 11px }
div.d-mini.collapsed { border-radius: 0; border-top: 0; border-bottom: 0; border-right: 0; background: var(--d-bg); gap: 14px }
div.d-mini.collapsed div.d-kpis { grid-template-columns: 1fr 1fr }
div.d-mini.rail { width: 52px; padding: 10px 6px; align-items: center; gap: 12px; border-radius: 0; border-top: 0; border-bottom: 0; border-right: 0; background: var(--d-bg) }
div.d-mini.rail > div.d-rail-v { font-size: 11px; font-weight: 600; text-align: center; color: var(--d-fg) }
div.d-mini.rail > div.d-rail-l { font-size: 9px; text-align: center; color: var(--d-faint); margin-top: -10px }
div.d-glass div.d-pop, div.d-glass div.k-pop, div.d-glass div.k-menu, div.d-glass div.k-dialog {
    background: var(--k-glass-bg); border: 1px solid var(--k-glass-line); backdrop-filter: blur(var(--k-glass-blur));
    box-shadow: 0 16px 40px var(--k-glass-shadow) }
div.d-glass div.k-scrim { background: var(--k-glass-scrim) }
div.d-glass div.k-dialog-foot { border-top: 1px solid var(--k-glass-line) }
div.d-glass button.k-btn.primary { background: #f0f0f0; color: #111111 }
div.d-glass button.k-btn { background: rgba(255,255,255,0.08) }
"#;

/// [`KIT_CSS`] + [`DASH_CSS`] parsed.
pub fn sheet() -> Sheet {
    Sheet::parse(&[KIT_CSS, DASH_CSS].concat()).expect("DASH_CSS parses")
}

/// [`KIT_CSS`] + [`DASH_CSS`] followed by the app's own rules.
pub fn sheet_with(app_css: &str) -> Result<Sheet, StyleError> {
    Sheet::parse(&[KIT_CSS, DASH_CSS, app_css].concat())
}

/// Inline variables that retune the overlay glass on a subtree (a theme knob):
/// background, hairline and blur radius.
pub fn glass_vars(bg: &str, line: &str, blur_px: f64) -> String {
    format!("--k-glass-bg: {bg}; --k-glass-line: {line}; --k-glass-blur: {blur_px}px")
}

// ---------------------------------------------------------------------------
// Provider glyphs
// ---------------------------------------------------------------------------

/// Monochrome 24×24 stroke glyphs of agent providers (`currentColor`, no brand colour).
pub mod glyphs {
    /// Claude: a radiating burst.
    pub const CLAUDE: &str = r#"<path d="M12 3v6M12 15v6M3 12h6M15 12h6M5.6 5.6l4.2 4.2M14.2 14.2l4.2 4.2M18.4 5.6l-4.2 4.2M9.8 14.2l-4.2 4.2"/>"#;
    /// Codex: a hexagonal knot.
    pub const CODEX: &str = r#"<path d="M12 3l7.8 4.5v9L12 21l-7.8-4.5v-9z"/><path d="M12 8.5l3 1.75v3.5L12 15.5l-3-1.75v-3.5z"/>"#;
    /// Grok: a slashed ring.
    pub const GROK: &str = r#"<circle cx="12" cy="12" r="8"/><path d="M6 18L18 6"/>"#;
    /// Kimi: a K in a rounded square.
    pub const KIMI: &str = r#"<rect x="4" y="4" width="16" height="16" rx="4"/><path d="M10 8v8M15 8l-5 4 5 4"/>"#;
    /// OpenCode: code brackets.
    pub const OPENCODE: &str = r#"<path d="M9 7l-5 5 5 5M15 7l5 5-5 5"/>"#;
    /// Gemini: a four-point star.
    pub const GEMINI: &str = r#"<path d="M12 3c.8 4.6 3.4 8.2 9 9-5.6.8-8.2 4.4-9 9-.8-4.6-3.4-8.2-9-9 5.6-.8 8.2-4.4 9-9z"/>"#;
    /// Cursor: a pointer.
    pub const CURSOR: &str = r#"<path d="M6 4l13 7-6 1.6L10.6 19z"/>"#;
    /// Qwen: a hexagon with a tail.
    pub const QWEN: &str = r#"<path d="M8 5h8l4 7-4 7H8l-4-7z"/><path d="M14 14l3 3"/>"#;
    /// DeepSeek: a wave.
    pub const DEEPSEEK: &str = r#"<path d="M3 13c3-5 6-5 9 0s6 5 9 0"/><circle cx="17" cy="8" r="1"/>"#;
    /// A human (the owner or a person in a room).
    pub const HUMAN: &str = r#"<circle cx="12" cy="8" r="3.5"/><path d="M5 20c1-4 4-6 7-6s6 2 7 6"/>"#;
}

/// The glyph of a provider by name (case-insensitive, prefixes such as `claude-code`
/// match `claude`), or `None` for an unknown one ([`glyph`] then draws its letter).
pub fn provider_glyph(name: &str) -> Option<&'static str> {
    let n = name.trim().to_ascii_lowercase();
    let table: [(&str, &'static str); 11] = [
        ("claude", glyphs::CLAUDE),
        ("anthropic", glyphs::CLAUDE),
        ("codex", glyphs::CODEX),
        ("openai", glyphs::CODEX),
        ("grok", glyphs::GROK),
        ("kimi", glyphs::KIMI),
        ("opencode", glyphs::OPENCODE),
        ("gemini", glyphs::GEMINI),
        ("cursor", glyphs::CURSOR),
        ("qwen", glyphs::QWEN),
        ("deepseek", glyphs::DEEPSEEK),
    ];
    if n == "human" || n == "owner" || n == "user" {
        return Some(glyphs::HUMAN);
    }
    table.iter().find(|(k, _)| n.starts_with(k)).map(|(_, g)| *g)
}

/// A small grey provider glyph (`span.d-glyph`), named for screen readers. An unknown
/// provider shows its first letter in a hairline square; an empty name, nothing.
pub fn glyph(provider: &str) -> El {
    match provider_glyph(provider) {
        Some(g) => El::new("span.d-glyph").name(provider).svg(icon_w(g, 1.8)),
        None => {
            let l: String = provider.trim().chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
            El::new("span.d-glyph.letter").name(provider).text(l)
        }
    }
}

fn icon_w(body: &str, w: f64) -> String {
    format!(r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="{w}" stroke-linecap="round" stroke-linejoin="round">{body}</svg>"#)
}

fn dot(live: bool) -> El {
    El::new(if live { "span.d-dot.live" } else { "span.d-dot" })
}

fn initial(name: &str) -> String {
    name.trim().chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_default()
}

fn grow() -> El {
    El::new("div.d-grow")
}

// ---------------------------------------------------------------------------
// Space switcher
// ---------------------------------------------------------------------------

/// One space (workspace, project) in a switcher.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpaceItem {
    pub key: String,
    pub name: String,
    /// A muted second line (e.g. "3 nodes · 5 agents").
    pub sub: String,
    /// Something runs in it now (green dot).
    pub live: bool,
}

impl SpaceItem {
    pub fn new(key: impl Into<String>, name: impl Into<String>) -> Self {
        SpaceItem { key: key.into(), name: name.into(), ..Default::default() }
    }
    pub fn sub(mut self, s: impl Into<String>) -> Self {
        self.sub = s.into();
        self
    }
    pub fn live(mut self, on: bool) -> Self {
        self.live = on;
        self
    }
}

/// The closed switcher: the current space's initial, name, its sub line and a chevron.
/// The button is the hit `id` (open [`space_menu`] on it).
pub fn space_switcher(id: &str, current: &SpaceItem, name: &str) -> El {
    El::new("button.d-switch")
        .id(id)
        .tab()
        .name(name)
        .child(El::new("span.d-av").text(initial(&current.name)))
        .child(El::new("div.d-sw-name").text(&current.name))
        .child_if(current.live, || dot(true))
        .child_if(!current.sub.is_empty(), || El::new("div.d-sw-sub").text(&current.sub))
        .child(El::new("span.d-chev").svg(icon(icons::DOWN)))
}

/// The open switcher: a glass overlay (wrap it with nothing else, it carries
/// [`glass`]) at `at` listing `items` (`{id}:item:{i}`, the current one ticked) and,
/// when `new_label` is given, a last row `{id}:new`.
pub fn space_menu(id: &str, title: &str, items: &[SpaceItem], current: usize, new_label: Option<&str>, at: Rect) -> El {
    let rows = items.iter().enumerate().map(|(i, s)| {
        El::new(if i == current { "div.d-mi.on" } else { "div.d-mi" })
            .id(format!("{id}:item:{i}"))
            .tab()
            .name(&s.name)
            .child(El::new("span.d-av").text(initial(&s.name)))
            .child(
                El::new("div.d-mi-main")
                    .child(El::new("div").text(&s.name))
                    .child_if(!s.sub.is_empty(), || El::new("div.d-mi-sub").text(&s.sub)),
            )
            .child_if(s.live, || dot(true))
            .child_if(i == current, || El::new("span.d-tick").svg(icon(icons::CHECK)))
    });
    let mut pop = El::new("div.d-pop.d-menu")
        .id(format!("{id}:menu"))
        .name(title)
        .css(at_css(at))
        .child(El::new("div.d-pop-head").text(title))
        .children(rows);
    if let Some(l) = new_label {
        pop = pop
            .child(El::new("div.d-msep"))
            .child(El::new("div.d-mi.new").id(format!("{id}:new")).tab().child(El::new("span.d-av").svg(icon(icons::PLUS))).child(El::new("div").text(l)));
    }
    glass(pop)
}

/// The index picked from a [`space_menu`] hit, if `hit` is one.
pub fn picked_space(id: &str, hit: &str) -> Option<usize> {
    hit.strip_prefix(id)?.strip_prefix(":item:")?.parse().ok()
}

fn at_css(at: Rect) -> String {
    format!("top: {}px; left: {}px; width: {}px", at.y, at.x, at.width)
}

// ---------------------------------------------------------------------------
// Tiles and histograms
// ---------------------------------------------------------------------------

/// A summary tile: caption, big value, a muted note and an optional mini histogram.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tile {
    pub label: String,
    pub value: String,
    pub note: String,
    pub hist: Vec<f64>,
}

impl Tile {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Tile { label: label.into(), value: value.into(), ..Default::default() }
    }
    pub fn note(mut self, n: impl Into<String>) -> Self {
        self.note = n.into();
        self
    }
    pub fn hist(mut self, v: impl Into<Vec<f64>>) -> Self {
        self.hist = v.into();
        self
    }
}

/// A mini histogram `h` px tall: one bar per value, scaled to the largest; the last
/// bar is highlighted (now). Negative and non-finite values count as zero.
pub fn hist(values: &[f64], h: f64) -> El {
    bars("div.d-hist", "div.d-hb", values, h, values.len().checked_sub(1)).css(format!("height: {h}px"))
}

fn bars(spec: &str, bar: &str, values: &[f64], h: f64, hi: Option<usize>) -> El {
    let max = values.iter().copied().filter(|v| v.is_finite()).fold(0.0f64, f64::max);
    El::new(spec).children(values.iter().enumerate().map(|(i, v)| {
        let v = if v.is_finite() { v.max(0.0) } else { 0.0 };
        let px = if max > 0.0 { 2.0 + (h - 2.0) * v / max } else { 2.0 };
        let px = (px * 10.0).round() / 10.0;
        El::new(if Some(i) == hi { format!("{bar}.hi") } else { bar.to_string() }).css(format!("height: {px}px"))
    }))
}

/// One [`Tile`].
pub fn tile(t: &Tile) -> El {
    El::new("div.d-tile")
        .name(&t.label)
        .child(El::new("div.d-cap").text(&t.label))
        .child(El::new("div.d-big").text(&t.value))
        .child_if(!t.note.is_empty(), || El::new("div.d-faint").text(&t.note))
        .child_if(!t.hist.is_empty(), || hist(&t.hist, 26.0))
}

/// A row of tiles in `cols` equal columns; each tile is the hit `{id}:{i}`.
pub fn tiles(id: &str, ts: &[Tile], cols: usize) -> El {
    El::new("div.d-tiles")
        .id(id)
        .css(format!("grid-template-columns: repeat({}, 1fr)", cols.max(1)))
        .children(ts.iter().enumerate().map(|(i, t)| tile(t).id(format!("{id}:{i}"))))
}

// ---------------------------------------------------------------------------
// Live agents
// ---------------------------------------------------------------------------

/// One agent row.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Agent {
    pub key: String,
    pub name: String,
    /// What it does now (a task title, a status).
    pub sub: String,
    /// Provider name for the glyph (`claude`, `codex`, …); empty: none.
    pub provider: String,
    pub live: bool,
    /// A muted right-hand note (uptime, tokens).
    pub side: String,
}

impl Agent {
    pub fn new(key: impl Into<String>, name: impl Into<String>) -> Self {
        Agent { key: key.into(), name: name.into(), ..Default::default() }
    }
    pub fn sub(mut self, s: impl Into<String>) -> Self {
        self.sub = s.into();
        self
    }
    pub fn provider(mut self, p: impl Into<String>) -> Self {
        self.provider = p.into();
        self
    }
    pub fn live(mut self, on: bool) -> Self {
        self.live = on;
        self
    }
    pub fn side(mut self, s: impl Into<String>) -> Self {
        self.side = s.into();
        self
    }
}

/// Agents of one node (machine, host).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NodeAgents {
    pub node: String,
    pub agents: Vec<Agent>,
}

/// Nodes of one space (or any outer group).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AgentGroup {
    pub title: String,
    /// A muted count on the right of the head.
    pub sub: String,
    pub nodes: Vec<NodeAgents>,
}

/// One agent row: glyph, name, what it does, live dot; the hit `{id}:agent:{key}`.
pub fn agent_row(id: &str, a: &Agent) -> El {
    El::new("div.d-agent")
        .id(format!("{id}:agent:{}", a.key))
        .tab()
        .name(&a.name)
        .child_if(!a.provider.is_empty(), || glyph(&a.provider))
        .child(
            El::new("div.d-a-main")
                .child(El::new("div.d-a-name").text(&a.name))
                .child_if(!a.sub.is_empty(), || El::new("div.d-a-sub").text(&a.sub)),
        )
        .child_if(!a.side.is_empty(), || El::new("div.d-a-side").text(&a.side))
        .child(dot(a.live))
}

/// Agents grouped by space, then node. A node title is shown only when a group has
/// more than one node or the node has a name. `empty` shows when there are no agents.
pub fn live_agents(id: &str, groups: &[AgentGroup], empty: &str) -> El {
    let any = groups.iter().any(|g| g.nodes.iter().any(|n| !n.agents.is_empty()));
    if !any {
        return El::new("div.d-agents").id(id).child(El::new("div.d-empty").text(empty));
    }
    El::new("div.d-agents").id(id).children(groups.iter().filter(|g| g.nodes.iter().any(|n| !n.agents.is_empty())).map(|g| {
        let live = g.nodes.iter().flat_map(|n| &n.agents).any(|a| a.live);
        let mut el = El::new("div.d-group").child_if(!g.title.is_empty(), || {
            El::new("div.d-group-head").child(dot(live)).child(El::new("div").text(&g.title)).child(grow()).child(El::new("div").text(&g.sub))
        });
        for n in g.nodes.iter().filter(|n| !n.agents.is_empty()) {
            if !n.node.is_empty() {
                el = el.child(El::new("div.d-node").text(&n.node));
            }
            el = el.children(n.agents.iter().map(|a| agent_row(id, a)));
        }
        el
    }))
}

// ---------------------------------------------------------------------------
// Tasks: tree, list, card, mini board
// ---------------------------------------------------------------------------

/// The state of a task as the widgets draw it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TaskState {
    #[default]
    Open,
    Running,
    Done,
}

/// A task with its subtasks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TaskNode {
    pub key: String,
    pub title: String,
    pub state: TaskState,
    /// Provider of the agent on it (glyph), empty: none.
    pub provider: String,
    /// A muted right-hand note (assignee, status word, age).
    pub side: String,
    pub children: Vec<TaskNode>,
}

impl TaskNode {
    pub fn new(key: impl Into<String>, title: impl Into<String>, state: TaskState) -> Self {
        TaskNode { key: key.into(), title: title.into(), state, ..Default::default() }
    }
    pub fn provider(mut self, p: impl Into<String>) -> Self {
        self.provider = p.into();
        self
    }
    pub fn side(mut self, s: impl Into<String>) -> Self {
        self.side = s.into();
        self
    }
    pub fn kids(mut self, k: impl IntoIterator<Item = TaskNode>) -> Self {
        self.children = k.into_iter().collect();
        self
    }
    /// `(done, total)` over all descendants (not the task itself).
    pub fn progress(&self) -> (u32, u32) {
        self.children.iter().fold((0, 0), |(d, t), c| {
            let (cd, ct) = c.progress();
            (d + cd + u32::from(c.state == TaskState::Done), t + ct + 1)
        })
    }
}

/// One visible row of a task tree.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TaskRow {
    pub key: String,
    pub title: String,
    pub state: TaskState,
    pub provider: String,
    pub side: String,
    pub depth: usize,
    /// `Some(folded)` when the task has subtasks.
    pub fold: Option<bool>,
    /// `(done, total)` over the descendants; `(0, 0)` without subtasks.
    pub kids: (u32, u32),
}

/// Build tasks into a tree from flat `(key, parent)` links: `items` in display order,
/// `parent_of(item)` the parent key (a missing parent makes the item a root). Cycles
/// are cut (an item already placed is not placed again).
pub fn build_tree<T>(items: &[T], key_of: impl Fn(&T) -> &str, parent_of: impl Fn(&T) -> Option<&str>, node_of: impl Fn(&T) -> TaskNode) -> Vec<TaskNode> {
    let keys: HashSet<&str> = items.iter().map(&key_of).collect();
    fn place<T>(
        k: &str,
        items: &[T],
        key_of: &dyn Fn(&T) -> &str,
        parent_of: &dyn Fn(&T) -> Option<&str>,
        node_of: &dyn Fn(&T) -> TaskNode,
        seen: &mut HashSet<String>,
    ) -> Option<TaskNode> {
        if !seen.insert(k.to_string()) {
            return None;
        }
        let it = items.iter().find(|i| key_of(i) == k)?;
        let mut n = node_of(it);
        n.children = items
            .iter()
            .filter(|i| parent_of(i) == Some(k) && key_of(i) != k)
            .filter_map(|i| place(key_of(i), items, key_of, parent_of, node_of, seen))
            .collect();
        Some(n)
    }
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for it in items {
        let root = parent_of(it).is_none_or(|p| !keys.contains(p) || p == key_of(it));
        if root {
            if let Some(n) = place(key_of(it), items, &key_of, &parent_of, &node_of, &mut seen) {
                out.push(n);
            }
        }
    }
    // Items only reachable through a cycle become roots.
    for it in items {
        if !seen.contains(key_of(it)) {
            if let Some(n) = place(key_of(it), items, &key_of, &parent_of, &node_of, &mut seen) {
                out.push(n);
            }
        }
    }
    out
}

/// The visible rows of `roots`: depth-first, the subtasks of a key in `folded` hidden.
pub fn flatten_tree(roots: &[TaskNode], folded: &HashSet<String>) -> Vec<TaskRow> {
    fn walk(n: &TaskNode, depth: usize, folded: &HashSet<String>, out: &mut Vec<TaskRow>) {
        let f = folded.contains(&n.key);
        out.push(TaskRow {
            key: n.key.clone(),
            title: n.title.clone(),
            state: n.state,
            provider: n.provider.clone(),
            side: n.side.clone(),
            depth,
            fold: (!n.children.is_empty()).then_some(f),
            kids: n.progress(),
        });
        if !f {
            for c in &n.children {
                walk(c, depth + 1, folded, out);
            }
        }
    }
    let mut out = Vec::new();
    for r in roots {
        walk(r, 0, folded, &mut out);
    }
    out
}

/// Toggle folding when `hit` is a `{id}:fold:{key}` of a [`task_list`]; `true` if it was.
pub fn toggle_fold(id: &str, hit: &str, folded: &mut HashSet<String>) -> bool {
    let Some(k) = hit.strip_prefix(id).and_then(|r| r.strip_prefix(":fold:")) else { return false };
    if !folded.remove(k) {
        folded.insert(k.to_string());
    }
    true
}

fn pbar(done: u32, total: u32, w: Option<f64>) -> El {
    let pct = if total == 0 { 0.0 } else { (done as f64 / total as f64 * 1000.0).round() / 10.0 };
    El::new("div.d-pbar").css(w.map(|w| format!("width: {w}px")).unwrap_or_default()).child(El::new("div.d-pfill").css(format!("width: {pct}%")))
}

fn state_dot(s: TaskState) -> El {
    El::new(match s {
        TaskState::Open => "span.d-st",
        TaskState::Running => "span.d-st.run",
        TaskState::Done => "span.d-st.done",
    })
}

/// Task rows (a flat list or a [`flatten_tree`] result): fold chevron, state, provider
/// glyph, title, progress over the subtasks and a side note. Hits: the row
/// `{id}:row:{key}`, the chevron `{id}:fold:{key}`; `sel` marks one row.
pub fn task_list(id: &str, rows: &[TaskRow], sel: Option<&str>) -> El {
    El::new("div.d-tasks").id(id).children(rows.iter().map(|r| {
        let mut spec = String::from("div.d-task");
        if r.state == TaskState::Done {
            spec.push_str(".done");
        }
        if sel == Some(r.key.as_str()) {
            spec.push_str(".sel");
        }
        let fold = match r.fold {
            Some(f) => El::new("span.d-fold").id(format!("{id}:fold:{}", r.key)).name(if f { "Expand" } else { "Collapse" }).svg(icon(if f { icons::RIGHT } else { icons::DOWN })),
            None => El::new("span.d-nofold"),
        };
        El::new(spec)
            .id(format!("{id}:row:{}", r.key))
            .tab()
            .name(&r.title)
            .css(format!("padding-left: {}px", 6.0 + 18.0 * r.depth.min(8) as f64))
            .child(fold)
            .child(state_dot(r.state))
            .child_if(!r.provider.is_empty(), || glyph(&r.provider))
            .child(El::new("div.d-t-title").text(&r.title))
            .child_if(r.kids.1 > 0, || El::new("div.d-t-kids").child(pbar(r.kids.0, r.kids.1, Some(36.0))).child(El::new("div").text(format!("{}/{}", r.kids.0, r.kids.1))))
            .child_if(!r.side.is_empty(), || El::new("div.d-t-side").text(&r.side))
    }))
}

/// A task card's content.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TaskCard {
    pub title: String,
    pub status: String,
    pub state: TaskState,
    pub provider: String,
    /// Up to a few short facts (`label`, `value`) in two columns.
    pub facts: Vec<(String, String)>,
    pub body: String,
    /// `(done, total)` subtasks, with a caption such as "subtasks".
    pub progress: Option<(u32, u32, String)>,
    /// `(key, label)` buttons, hits `{id}:act:{key}`; the first is filled.
    pub actions: Vec<(String, String)>,
}

/// A task card (opaque; hit `id`).
pub fn task_card(id: &str, c: &TaskCard) -> El {
    El::new("div.d-card")
        .id(id)
        .name(&c.title)
        .child(
            El::new("div.d-card-head")
                .child(state_dot(c.state).css("width: 10px; height: 10px; border-radius: 5px"))
                .child_if(!c.provider.is_empty(), || glyph(&c.provider))
                .child(El::new("div.d-card-title").text(&c.title))
                .child_if(!c.status.is_empty(), || El::new("div.d-card-status").text(&c.status)),
        )
        .child_if(!c.facts.is_empty(), || {
            El::new("div.d-facts").children(c.facts.iter().map(|(l, v)| El::new("div.d-fact").child(El::new("div.d-f-label").text(l)).child(El::new("div.d-f-value").text(v))))
        })
        .child_if(!c.body.is_empty(), || El::new("div.d-card-body").text(&c.body))
        .child_if(c.progress.as_ref().is_some_and(|p| p.1 > 0), || {
            let (d, t, cap) = c.progress.clone().unwrap_or_default();
            El::new("div.d-prog").child(El::new("div").text(cap)).child(pbar(d, t, None)).child(El::new("div").text(format!("{d}/{t}")))
        })
        .child_if(!c.actions.is_empty(), || {
            El::new("div.d-card-acts").children(c.actions.iter().enumerate().map(|(i, (k, l))| {
                El::new(if i == 0 { "button.d-btn" } else { "button.d-btn.ghost" }).id(format!("{id}:act:{k}")).tab().text(l)
            }))
        })
}

/// A card on a [`mini_board`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoardCard {
    pub key: String,
    pub title: String,
    pub provider: String,
    /// A muted meta line (assignee, age).
    pub meta: String,
    /// `(done, total)` subtasks.
    pub kids: (u32, u32),
    pub live: bool,
}

/// A column of a [`mini_board`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoardCol {
    pub title: String,
    pub cards: Vec<BoardCard>,
}

/// A compact board: columns with counts and small cards (`{id}:card:{key}`); at most
/// `max_cards` per column, then a muted "+N".
pub fn mini_board(id: &str, cols: &[BoardCol], max_cards: usize) -> El {
    El::new("div.d-board").id(id).css(format!("grid-template-columns: repeat({}, 1fr)", cols.len().max(1))).children(cols.iter().map(|c| {
        let extra = c.cards.len().saturating_sub(max_cards);
        El::new("div.d-col")
            .child(El::new("div.d-col-head").child(El::new("div").text(&c.title)).child(grow()).child(El::new("div").text(c.cards.len().to_string())))
            .children(c.cards.iter().take(max_cards).map(|k| {
                El::new("div.d-bcard")
                    .id(format!("{id}:card:{}", k.key))
                    .tab()
                    .name(&k.title)
                    .child(El::new("div.d-bc-title").text(&k.title))
                    .child(
                        El::new("div.d-bc-meta")
                            .child_if(!k.provider.is_empty(), || glyph(&k.provider))
                            .child(El::new("div.d-grow").text(&k.meta))
                            .child_if(k.kids.1 > 0, || El::new("div").text(format!("{}/{}", k.kids.0, k.kids.1)))
                            .child_if(k.live, || dot(true)),
                    )
            }))
            .child_if(extra > 0, || El::new("div.d-faint").css("padding: 0 4px").text(format!("+{extra}")))
    }))
}

// ---------------------------------------------------------------------------
// Rings, activity
// ---------------------------------------------------------------------------

/// A limit / quota meter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Meter {
    pub label: String,
    /// Shown inside the ring (e.g. "62%").
    pub value: String,
    /// A muted line under the label (e.g. "resets in 3h").
    pub note: String,
    /// 0 … 1.
    pub frac: f64,
}

impl Meter {
    pub fn new(label: impl Into<String>, frac: f64) -> Self {
        let frac = if frac.is_finite() { frac.clamp(0.0, 1.0) } else { 0.0 };
        Meter { label: label.into(), value: format!("{}%", (frac * 100.0).round()), frac, ..Default::default() }
    }
    pub fn value(mut self, v: impl Into<String>) -> Self {
        self.value = v.into();
        self
    }
    pub fn note(mut self, n: impl Into<String>) -> Self {
        self.note = n.into();
        self
    }
}

/// A ring alone (`span.d-ring`, `small` without text).
pub fn ring(frac: f64, text: &str, small: bool) -> El {
    El::new(if small { "span.d-ring.small" } else { "span.d-ring" }).text(text).control(Control::ring(frac))
}

/// Limit rings with labels and notes.
pub fn rings(id: &str, ms: &[Meter]) -> El {
    El::new("div.d-rings").id(id).children(ms.iter().map(|m| {
        El::new("div.d-meter")
            .name(&m.label)
            .child(ring(m.frac, &m.value, false))
            .child(El::new("div.d-m-label").text(&m.label))
            .child_if(!m.note.is_empty(), || El::new("div.d-m-note").text(&m.note))
    }))
}

/// Activity over a period (24 hourly bars, say): title, total, bars (the `hi` one
/// highlighted, e.g. the current hour) and up to three axis labels.
pub fn activity(id: &str, title: &str, total: &str, values: &[f64], hi: Option<usize>, axis: &[&str]) -> El {
    El::new("div.d-box.d-act")
        .id(id)
        .name(title)
        .child(El::new("div.d-title").child(El::new("div").text(title)).child(grow()).child(El::new("div").css("color: var(--d-fg)").text(total)))
        .child(bars("div.d-bars", "div.d-bar", values, 56.0, hi))
        .child_if(!axis.is_empty(), || El::new("div.d-axis").children(axis.iter().map(|a| El::new("div").text(*a))))
}

/// A titled opaque box around any widget (`div.d-box` with a `div.d-title`).
pub fn section(title: &str, side: &str, body: El) -> El {
    El::new("div.d-box")
        .name(title)
        .child(El::new("div.d-title").child(El::new("div").text(title)).child(grow()).child_if(!side.is_empty(), || El::new("div").text(side)))
        .child(body)
}

// ---------------------------------------------------------------------------
// Mini panel
// ---------------------------------------------------------------------------

/// The metrics of a space, session or agent panel without its chat.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MiniPanel {
    /// What it is ("Space", "Session", "Agent"), shown small above the title.
    pub kind: String,
    pub title: String,
    pub live: bool,
    pub provider: String,
    /// `(label, value)` key numbers.
    pub kpis: Vec<(String, String)>,
    pub meters: Vec<Meter>,
    /// A short activity trend.
    pub trend: Vec<f64>,
    /// A muted last line (last event, updated at).
    pub foot: String,
}

/// How a [`MiniPanel`] shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PanelMode {
    /// A card in a chat or a sidebar: head with an "open" button (`{id}:open`).
    #[default]
    Widget,
    /// The collapsed state of the full panel: the same metrics in the panel's column,
    /// with an expand button (`{id}:expand`).
    Collapsed,
    /// A narrow rail (icons, rings and numbers only) with an expand button.
    Rail,
    /// The full panel (the host draws it); [`mini_panel`] then draws nothing.
    Full,
}

impl PanelMode {
    /// The next mode after `hit` on a panel `id`: `{id}:collapse` → `Collapsed`,
    /// `{id}:rail` → `Rail`, `{id}:expand` → `Full`. `None` when it is not such a hit.
    pub fn after(self, id: &str, hit: &str) -> Option<PanelMode> {
        match hit.strip_prefix(id)?.strip_prefix(':')? {
            "collapse" => Some(PanelMode::Collapsed),
            "rail" => Some(PanelMode::Rail),
            "expand" => Some(PanelMode::Full),
            _ => None,
        }
    }
}

/// A compact metrics-only panel. `labels`: accessible names of the head buttons
/// (`open`, `expand`, `rail`).
pub fn mini_panel(id: &str, p: &MiniPanel, mode: PanelMode, labels: [&str; 3]) -> El {
    let [open_l, expand_l, rail_l] = labels;
    match mode {
        PanelMode::Full => El::new("div.d-mini-none"),
        PanelMode::Rail => El::new("div.d-mini.rail")
            .id(id)
            .name(&p.title)
            .child(El::new("button.k-act").id(format!("{id}:expand")).tab().name(expand_l).svg(icon(icons::EXPAND)))
            .child_if(!p.provider.is_empty(), || glyph(&p.provider))
            .child(dot(p.live))
            .children(p.kpis.iter().take(3).flat_map(|(l, v)| [El::new("div.d-rail-v").text(v), El::new("div.d-rail-l").text(l)]))
            .children(p.meters.iter().map(|m| ring(m.frac, "", true).name(&m.label))),
        PanelMode::Widget | PanelMode::Collapsed => {
            let collapsed = mode == PanelMode::Collapsed;
            let (btn_id, btn_l, btn_icon) =
                if collapsed { (format!("{id}:expand"), expand_l, icons::EXPAND) } else { (format!("{id}:open"), open_l, icons::RIGHT) };
            El::new(if collapsed { "div.d-mini.collapsed" } else { "div.d-mini.widget" })
                .id(id)
                .name(&p.title)
                .child(
                    El::new("div.d-mini-head")
                        .child(dot(p.live))
                        .child_if(!p.provider.is_empty(), || glyph(&p.provider))
                        .child(El::new("div.d-mini-name").child_if(!p.kind.is_empty(), || El::new("div.d-mini-kind").text(&p.kind)).child(El::new("div.d-mini-title").text(&p.title)))
                        .child_if(collapsed, || El::new("button.k-act").id(format!("{id}:rail")).tab().name(rail_l).svg(icon(icons::RIGHT)))
                        .child(El::new("button.k-act").id(btn_id).tab().name(btn_l).svg(icon(btn_icon))),
                )
                .child_if(!p.kpis.is_empty(), || {
                    El::new("div.d-kpis").children(p.kpis.iter().map(|(l, v)| El::new("div.d-kpi").child(El::new("div.d-kpi-v").text(v)).child(El::new("div.d-kpi-l").text(l))))
                })
                .child_if(!p.meters.is_empty(), || rings(&format!("{id}:rings"), &p.meters))
                .child_if(!p.trend.is_empty(), || hist(&p.trend, 22.0))
                .child_if(!p.foot.is_empty(), || El::new("div.d-faint").text(&p.foot))
        }
    }
}

// ---------------------------------------------------------------------------
// Glass overlays
// ---------------------------------------------------------------------------

/// Put an overlay (a [`popover`], a kit [`dialog`](super::domkit::dialog),
/// [`menu`](super::domkit::menu) or select popover) under the glass look: translucent
/// `--k-glass-bg`, a `--k-glass-line` hairline and a `--k-glass-blur` backdrop blur.
/// Embedded widgets stay opaque; only overlays should be wrapped.
pub fn glass(overlay: El) -> El {
    El::new("div.d-glass").child(overlay)
}

/// A popover at `at` (fixed, viewport coordinates) with a title, a close button
/// (`{id}:close`) and `body`. Opaque by itself; wrap in [`glass`] for the overlay look.
pub fn popover(id: &str, title: &str, body: El, at: Rect, close: &str) -> El {
    El::new("div.d-pop")
        .id(id)
        .name(title)
        .css(at_css(at))
        .child(El::new("div.d-pop-head").child(El::new("div.d-grow").text(title)).child(El::new("button.k-act").id(format!("{id}:close")).tab().name(close).svg(icon(icons::CLOSE))))
        .child(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::dom::{self, Dom};
    use crate::ui::style::State;

    fn lay(tree: &El, w: f64, h: f64) -> Dom {
        let sheet = sheet();
        let mut m = crate::testing::NullRenderContext::default();
        dom::lay_out_in(&mut m, &sheet, tree, 0.0, 0.0, w, Some(Rect::new(0.0, 0.0, w, h)), &|_| State::NONE)
    }

    fn tree() -> Vec<TaskNode> {
        vec![
            TaskNode::new("a", "Release", TaskState::Running).provider("claude").kids([
                TaskNode::new("a1", "Changelog", TaskState::Done),
                TaskNode::new("a2", "Package", TaskState::Open).kids([TaskNode::new("a21", "Docs", TaskState::Done)]),
            ]),
            TaskNode::new("b", "Lone", TaskState::Open).provider("mystery"),
        ]
    }

    #[test]
    fn progress_counts_all_descendants() {
        let t = tree();
        assert_eq!(t[0].progress(), (2, 3));
        assert_eq!(t[1].progress(), (0, 0));
    }

    #[test]
    fn flatten_folds_and_toggles() {
        let t = tree();
        let mut folded = HashSet::new();
        let rows = flatten_tree(&t, &folded);
        assert_eq!(rows.iter().map(|r| (r.key.as_str(), r.depth)).collect::<Vec<_>>(), [("a", 0), ("a1", 1), ("a2", 1), ("a21", 2), ("b", 0)]);
        assert_eq!(rows[0].fold, Some(false));
        assert_eq!(rows[4].fold, None);
        assert!(toggle_fold("t", "t:fold:a2", &mut folded));
        assert!(!toggle_fold("t", "t:row:a2", &mut folded));
        let rows = flatten_tree(&t, &folded);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[2].fold, Some(true));
        toggle_fold("t", "t:fold:a2", &mut folded);
        assert!(folded.is_empty());
    }

    #[test]
    fn build_tree_from_parent_links_survives_cycles() {
        let items = [("a", None), ("b", Some("a")), ("c", Some("b")), ("x", Some("y")), ("y", Some("x")), ("o", Some("gone"))];
        let roots = build_tree(&items, |i| i.0, |i| i.1, |i| TaskNode::new(i.0, i.0, TaskState::Open));
        let keys: Vec<&str> = roots.iter().map(|n| n.key.as_str()).collect();
        assert_eq!(keys, ["a", "o", "x"]);
        assert_eq!(roots[0].children[0].children[0].key, "c");
        assert_eq!(roots[2].children[0].key, "y");
        assert!(roots[2].children[0].children.is_empty());
    }

    #[test]
    fn glyphs_are_monochrome_and_fall_back_to_a_letter() {
        for p in ["claude", "Codex", "grok-4", "kimi", "opencode", "gemini", "cursor", "qwen", "deepseek", "human"] {
            let g = provider_glyph(p).unwrap_or_else(|| panic!("{p}"));
            assert!(!g.contains("fill=\"#") && !g.contains("stroke=\"#"), "{p} carries a colour");
        }
        let e = glyph("mystery");
        assert_eq!(e.text.as_deref(), Some("M"));
        assert!(glyph("claude").svg.is_some());
    }

    #[test]
    fn widgets_lay_out_with_their_hits() {
        let spaces = [SpaceItem::new("s1", "Hatchery").sub("2 nodes").live(true), SpaceItem::new("s2", "Uzor")];
        let rows = flatten_tree(&tree(), &HashSet::new());
        let page = El::new("div.d-root")
            .css("display: flex; flex-direction: column; gap: 8px")
            .child(space_switcher("sp", &spaces[0], "Space"))
            .child(tiles("tl", &[Tile::new("Agents", "5").hist(vec![1.0, 3.0, 2.0]), Tile::new("Tasks", "12")], 2))
            .child(live_agents(
                "ag",
                &[AgentGroup { title: "Hatchery".into(), sub: "2".into(), nodes: vec![NodeAgents { node: "box-1".into(), agents: vec![Agent::new("k1", "knyaz").provider("claude").live(true)] }] }],
                "nobody",
            ))
            .child(task_list("tk", &rows, Some("a2")))
            .child(rings("lm", &[Meter::new("5h", 0.62).note("3h")]))
            .child(task_card("card", &TaskCard { title: "Release".into(), actions: vec![("open".into(), "Open".into())], progress: Some((2, 3, "sub".into())), ..Default::default() }))
            .child(mini_board("mb", &[BoardCol { title: "Open".into(), cards: vec![BoardCard { key: "a".into(), title: "Release".into(), ..Default::default() }] }], 3))
            .child(activity("act", "24h", "88", &[1.0; 24], Some(23), &["0", "12", "24"]))
            .child(mini_panel("mp", &MiniPanel { title: "Hatchery".into(), kpis: vec![("agents".into(), "5".into())], ..Default::default() }, PanelMode::Widget, ["Open", "Expand", "Rail"]))
            .child(space_menu("sp", "Spaces", &spaces, 0, Some("New space"), Rect::new(10.0, 40.0, 240.0, 0.0)));
        let d = lay(&page, 600.0, 2000.0);
        let hits: HashSet<String> = d.hits().map(|(i, _)| i.to_string()).collect();
        for h in ["sp", "tl:0", "tl:1", "ag:agent:k1", "tk:row:a", "tk:fold:a", "tk:fold:a2", "card", "card:act:open", "mb:card:a", "mp:open", "sp:item:0", "sp:item:1", "sp:new"] {
            assert!(hits.contains(h), "missing hit {h}: {hits:?}");
        }
        assert!(!hits.contains("tk:fold:a1"));
        assert_eq!(picked_space("sp", "sp:item:1"), Some(1));
        assert_eq!(picked_space("sp", "sp:new"), None);
        // The rings carry a meter control.
        assert!(d.boxes.iter().any(|b| matches!(b.control, Some(Control::Ring { frac }) if (frac - 0.62).abs() < 1e-9)));
    }

    #[test]
    fn overlays_are_glass_and_embedded_widgets_opaque() {
        let page = El::new("div.d-root")
            .child(tile(&Tile::new("Agents", "5")).id("t"))
            .child(glass(popover("p", "Spaces", El::new("div").text("x"), Rect::new(0.0, 100.0, 200.0, 0.0), "Close")));
        let d = lay(&page, 400.0, 400.0);
        let t = d.get("t").unwrap();
        assert_eq!(t.style.background.as_deref(), Some("#1f1f1f"));
        assert_eq!(t.style.backdrop_blur, None);
        let p = d.get("p").unwrap();
        assert_eq!(p.style.backdrop_blur, Some(28.0));
        assert_eq!(p.style.background.as_deref(), Some("rgba(32,32,34,0.72)"));
        // The theme retunes the glass.
        let page = El::new("div.d-root").css(glass_vars("rgba(0,0,0,0.5)", "#ffffff22", 12.0)).child(glass(popover("p", "S", El::new("div"), Rect::new(0.0, 0.0, 100.0, 0.0), "Close")));
        let d = lay(&page, 400.0, 400.0);
        assert_eq!(d.get("p").unwrap().style.backdrop_blur, Some(12.0));
    }

    #[test]
    fn panel_modes_follow_their_hits() {
        assert_eq!(PanelMode::Full.after("mp", "mp:collapse"), Some(PanelMode::Collapsed));
        assert_eq!(PanelMode::Collapsed.after("mp", "mp:rail"), Some(PanelMode::Rail));
        assert_eq!(PanelMode::Rail.after("mp", "mp:expand"), Some(PanelMode::Full));
        assert_eq!(PanelMode::Widget.after("mp", "mp2:expand"), None);
        assert_eq!(PanelMode::Widget.after("mp", "mp:open"), None);
        let p = MiniPanel { title: "S".into(), meters: vec![Meter::new("5h", 0.3)], kpis: vec![("a".into(), "1".into())], ..Default::default() };
        let d = lay(&El::new("div.d-root").child(mini_panel("mp", &p, PanelMode::Rail, ["o", "e", "r"])), 300.0, 600.0);
        assert!(d.rect("mp:expand").is_some());
        let d = lay(&El::new("div.d-root").child(mini_panel("mp", &p, PanelMode::Collapsed, ["o", "e", "r"])), 300.0, 600.0);
        assert!(d.rect("mp:expand").is_some() && d.rect("mp:rail").is_some());
    }
}
