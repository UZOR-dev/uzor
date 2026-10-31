//! The DOM widget catalog: ready-made [`El`] trees for the common controls and chat
//! building blocks, styled by [`KIT_CSS`] (every look is a CSS rule an app may override
//! by appending its own sheet, see [`sheet_with`]).
//!
//! | widget | builder | element / classes | hit ids |
//! |---|---|---|---|
//! | button | [`button`] | `button.k-btn` (`.primary`, `.ghost`) | `id` |
//! | switch | [`switch`] | `button.k-switch` + [`Control::Switch`] (`:checked`) | `id` |
//! | checkbox | [`checkbox`] | `div.k-check` > `span.k-box` + [`Control::Checkbox`] | `id` |
//! | text input | [`input`] | `div.k-input` + [`Control::Input`] (`:focus`) | `id` |
//! | spinner | [`spinner`] | `span.k-spinner` + [`Control::Spinner`] | — |
//! | select trigger | [`select_trigger`] | `select.k-select` (`.open`) | `id` |
//! | select popover | [`select_popover`] | fixed `div.k-pop` > `div.k-opt` (`.sel`, `.dis`) > label + `div.k-sub` | `{id}:opt:{i}` (enabled rows only) |
//! | message bubble | [`bubble`] | `div.k-msg.me` / `.them` > `div.k-who`, `div.k-body` (runs), `div.k-time` | `id` |
//! | hover action row | [`action_row`] | `div.k-actions` > `button.k-act` | `{id}:{key}` |
//! | collapsible card | [`card`] | `div.k-card` (`.open`) > `div.k-card-head` + `div.k-card-body` | `id` (the head) |
//! | chip / filter | [`chip`] | `button.k-chip` (`.on`) | `id` |
//! | banner | [`banner`] | `div.k-banner.info` / `.warn` / `.error` | `{id}:act`, `{id}:close` |
//! | side panel | [`side_panel`] | fixed `div.k-side` > `div.k-side-head` + `div.k-side-body` | `{id}:close` |
//! | tabs | [`tabs`] | `div.k-tabs` > `button.k-tab` (`.on`) | `{id}:tab:{i}` |
//! | segmented | [`segmented`] | `div.k-seg` > `button.k-seg-btn` (`.on`) | `{id}:{i}` |
//! | list | [`list`] | `div.k-list` > `div.k-li` (`.sel`) | `{id}:row:{i}` |
//! | table | [`table`] | `div.k-table` > grid `div.k-tr` (`.k-th`, `.sel`) > `div.k-td` (`.num`) | `{id}:row:{i}` |
//! | menu | [`menu`] | fixed `div.k-menu` > `div.k-mi` (`.danger`, `.dis`), `div.k-sep` | `{id}:item:{i}` |
//! | dialog | [`dialog`] | fixed `div.k-scrim` + `div.k-dialog` > head, body, foot | `{id}:scrim`, `{id}:close`, `{id}:foot:{i}` |
//! | tooltip | [`tooltip`] | fixed `div.k-tip` | — |
//! | toasts | [`toasts`] | fixed `div.k-toasts` > `div.k-toast` (`.ok`, `.warn`, `.error`) | `{id}:{i}:close` |
//! | stat, progress, badge, kbd, field | [`stat`], [`progress`], [`badge`], [`kbd`], [`field`] | `div.k-stat`, `div.k-progress`, `span.k-badge`, `span.k-kbd`, `div.k-field` | `id` (stat) |
//!
//! The light look is [`LIGHT_VARS`] on the page root ([`theme_vars`]); every colour in
//! [`KIT_CSS`] is a `--k-*` variable.
//!
//! Inline markdown for bubbles is [`inline_md`]; long lists render only their visible
//! rows with [`VirtualList`]. Kit widgets from `uzor-framework` go *inside* a DOM tree
//! through an embed slot ([`El::slot`] + [`Dom::slots`](super::dom::Dom::slots)); a
//! DOM tree goes inside a kit panel by laying it out in the panel's rect.

use super::dom::{Control, Dom, Edit, El, Run};
use super::style::{Sheet, StyleError};
use crate::types::Rect;

/// The catalog's default look (dark, 13px Inter). Variables on `:root` theme it:
/// `--k-accent`, `--k-bg`, `--k-fg`, `--k-muted`, `--k-line`, `--k-panel`.
pub const KIT_CSS: &str = r#"
:root { --k-accent: #3b82f6; --k-bg: #17181b; --k-panel: #1f2125; --k-fg: #e8e8ea; --k-muted: #9a9ca3; --k-line: #33363c;
        --k-me: #2563eb; --k-them: #26282d; --k-warn: #f59e0b; --k-error: #ef4444; --k-info: #3b82f6; --k-ok: #22c55e;
        --k-btn: #2b2e34; --k-btn-hover: #353840; --k-track: #4a4d55; --k-track-hover: #575b64; --k-accent-hover: #2f6fd8;
        --k-field: #111214; --k-sunken: #141518; --k-hover: #2c2f36; --k-hi: #33363d; --k-sel: rgba(59,130,246,0.18);
        --k-dis: #5d6068; --k-body: #c9cbd1; --k-on-accent: #ffffff; --k-scrim: rgba(0,0,0,0.55); --k-shadow: rgba(0,0,0,0.45) }
div.k-root { color: var(--k-fg); font-size: 13px; font-family: Inter, sans-serif }
button.k-btn { padding: 6px 14px; border-radius: 8px; background: var(--k-btn); color: var(--k-fg); font-weight: 500 }
button.k-btn:hover { background: var(--k-btn-hover) }
button.k-btn.primary { background: var(--k-accent); color: var(--k-on-accent) }
button.k-btn.primary:hover { background: var(--k-accent-hover) }
button.k-btn.ghost { background: transparent; border: 1px solid var(--k-line) }
button.k-btn.ghost:hover { background: var(--k-hover) }
button.k-btn.danger { background: transparent; border: 1px solid var(--k-error); color: var(--k-error) }
button.k-btn:focus-visible { border: 2px solid var(--k-accent) }
button.k-switch { width: 36px; height: 20px; padding: 2px; border-radius: 10px; background: var(--k-track); --knob: #ffffff }
button.k-switch:checked { background: var(--k-accent) }
button.k-switch:hover { background: var(--k-track-hover) }
button.k-switch:checked:hover { background: var(--k-accent-hover) }
div.k-check { display: flex; align-items: center; gap: 8px }
span.k-box { width: 16px; height: 16px; padding: 1px; border-radius: 4px; border: 1.5px solid var(--k-track-hover); --check: #ffffff; --check-width: 2px }
span.k-box:checked { background: var(--k-accent); border-color: var(--k-accent) }
div.k-input { height: 32px; padding: 0 10px; border-radius: 8px; background: var(--k-field); border: 1px solid var(--k-line);
              --placeholder: var(--k-muted); --selection: rgba(59,130,246,0.4); --caret: var(--k-fg) }
div.k-input:hover { border-color: var(--k-track) }
div.k-input:focus { border-color: var(--k-accent) }
span.k-spinner { width: 18px; height: 18px; color: var(--k-accent); --spin-track: var(--k-line); --spin-speed: 1.2 }
select.k-select { height: 32px; padding: 0 30px 0 12px; border-radius: 8px; background: var(--k-btn); color: var(--k-fg); --chevron: var(--k-muted) }
select.k-select:hover { background: var(--k-btn-hover) }
select.k-select.open { border: 1px solid var(--k-accent) }
div.k-pop { position: fixed; z-index: 100; padding: 4px; border-radius: 10px; background: var(--k-panel); border: 1px solid var(--k-line);
            box-shadow: 0 8px 24px var(--k-shadow); display: flex; flex-direction: column }
div.k-opt { display: flex; align-items: center; gap: 8px; padding: 7px 10px; border-radius: 6px; color: var(--k-fg); flex-shrink: 0 }
div.k-opt:hover { background: var(--k-hover) }
div.k-opt.sel { background: var(--k-sel) }
div.k-opt.hi { background: var(--k-hi) }
span.k-tick { width: 14px; height: 14px; flex-shrink: 0; color: var(--k-accent) }
div.k-optl { display: flex; flex-direction: column; flex-grow: 1; min-width: 0 }
div.k-search { display: flex; align-items: center; gap: 8px; height: 28px; margin: 2px 2px 4px; padding: 0 8px; border-radius: 6px; background: var(--k-sunken); flex-shrink: 0 }
span.k-mag { width: 14px; height: 14px; color: var(--k-muted) }
div.k-ph { color: var(--k-muted) }
div.k-empty { padding: 7px 14px; color: var(--k-muted) }
div.k-thumb { position: absolute; right: 3px; width: 3px; border-radius: 2px; background: var(--k-track) }
div.k-opt.dis { color: var(--k-dis) }
div.k-opt .k-sub { color: var(--k-muted); font-size: 11px }
div.k-opt.dis .k-sub { color: var(--k-dis) }
div.k-msg { display: flex; flex-direction: column; max-width: 75%; padding: 8px 12px; border-radius: 14px; white-space: normal; gap: 2px }
div.k-msg.me { align-self: flex-end; background: var(--k-me); color: #ffffff; border-radius: 14px 14px 4px 14px }
div.k-msg.them { align-self: flex-start; background: var(--k-them); border-radius: 14px 14px 14px 4px }
div.k-msg > div.k-who { font-weight: 600; font-size: 12px; color: var(--k-accent) }
div.k-msg.me > div.k-who { color: #dbe7ff }
div.k-msg div.k-time { font-size: 10px; color: var(--k-muted); text-align: right }
div.k-msg.me div.k-time { color: #c7d7f7 }
div.k-body code { font-family: monospace; background: rgba(0,0,0,0.3); padding: 0 5px; border-radius: 5px }
div.k-body b { font-weight: 700 }
div.k-body i { font-style: italic }
div.k-body a.link { color: #93c5fd }
div.k-body a.mention { color: #93c5fd; font-weight: 600 }
div.k-msg.me div.k-body a.link { color: #ffffff }
div.k-msg > div.k-actions { align-self: flex-end }
div.k-actions { display: flex; gap: 2px; padding: 2px; border-radius: 8px; background: var(--k-panel); border: 1px solid var(--k-line) }
button.k-act { width: 26px; height: 26px; padding: 5px; border-radius: 6px; color: var(--k-muted) }
button.k-act:hover { background: var(--k-hover); color: var(--k-fg) }
div.k-card { border-radius: 10px; border: 1px solid var(--k-line); background: var(--k-panel); display: flex; flex-direction: column }
div.k-card-head { display: flex; align-items: center; gap: 6px; padding: 8px 10px; font-weight: 600 }
div.k-card-head:hover { background: var(--k-hover); border-radius: 10px }
div.k-card > div.k-card-head > span.k-chev { width: 14px; height: 14px; color: var(--k-muted) }
div.k-card-body { padding: 4px 12px 10px; white-space: normal; color: var(--k-body) }
button.k-chip { padding: 4px 10px; border-radius: 999px; border: 1px solid var(--k-line); color: var(--k-muted); font-size: 12px }
button.k-chip:hover { color: var(--k-fg) }
button.k-chip.on { background: var(--k-sel); border-color: var(--k-accent); color: var(--k-fg) }
div.k-banner { display: flex; align-items: center; gap: 10px; padding: 8px 12px; border-radius: 8px; border-left: 3px solid var(--k-info); background: rgba(59,130,246,0.12) }
div.k-banner.warn { border-left: 3px solid var(--k-warn); background: rgba(245,158,11,0.12) }
div.k-banner.error { border-left: 3px solid var(--k-error); background: rgba(239,68,68,0.12) }
div.k-banner.ok { border-left: 3px solid var(--k-ok); background: rgba(34,197,94,0.12) }
div.k-banner > div.k-banner-text { flex-grow: 1; white-space: normal }
div.k-banner button.k-act { width: 22px; height: 22px; padding: 4px }
div.k-side { position: fixed; top: 0; right: 0; bottom: 0; width: 320px; z-index: 50; background: var(--k-panel); border-left: 1px solid var(--k-line);
             display: flex; flex-direction: column }
div.k-side-head { display: flex; align-items: center; padding: 12px 14px; border-bottom: 1px solid var(--k-line); font-weight: 600 }
div.k-side-head > div.k-side-title { flex-grow: 1 }
div.k-side-body { padding: 12px 14px; display: flex; flex-direction: column; gap: 10px; flex-grow: 1 }
div.k-slot { border: 1px dashed var(--k-line); border-radius: 8px }
div.k-tabs { display: flex; gap: 2px; padding: 3px; border-radius: 9px; background: var(--k-sunken); border: 1px solid var(--k-line) }
button.k-tab { flex-grow: 1; padding: 5px 12px; border-radius: 6px; color: var(--k-muted); font-weight: 500; text-align: center }
button.k-tab:hover { color: var(--k-fg); background: var(--k-hover) }
button.k-tab.on { background: var(--k-btn-hover); color: var(--k-fg) }
div.k-list { display: flex; flex-direction: column; padding: 4px; border-radius: 10px; border: 1px solid var(--k-line); background: var(--k-panel); overflow: hidden }
div.k-li { display: flex; align-items: center; gap: 8px; height: 30px; padding: 0 10px; border-radius: 6px; flex-shrink: 0 }
div.k-li:hover { background: var(--k-hover) }
div.k-li.sel { background: var(--k-sel); color: var(--k-fg) }
div.k-li > div.k-li-main { flex-grow: 1; min-width: 0 }
div.k-li > div.k-li-side { color: var(--k-muted); font-size: 12px }
div.k-table { display: flex; flex-direction: column; border-radius: 10px; border: 1px solid var(--k-line); background: var(--k-panel); overflow: hidden }
div.k-tr { display: grid; align-items: center; height: 30px; padding: 0 12px; column-gap: 12px; border-bottom: 1px solid var(--k-line); flex-shrink: 0 }
div.k-tr:hover { background: var(--k-hover) }
div.k-tr.sel { background: var(--k-sel) }
div.k-tr.k-th { color: var(--k-muted); font-size: 11px; font-weight: 600; background: var(--k-sunken) }
div.k-td.num { text-align: right }
div.k-menu { position: fixed; z-index: 120; padding: 4px; border-radius: 10px; background: var(--k-panel); border: 1px solid var(--k-line);
             box-shadow: 0 8px 24px var(--k-shadow); display: flex; flex-direction: column }
div.k-menu-title { padding: 6px 10px 4px; color: var(--k-muted); font-size: 11px; font-weight: 600 }
div.k-mi { display: flex; align-items: center; gap: 8px; padding: 7px 10px; border-radius: 6px; flex-shrink: 0 }
div.k-mi:hover { background: var(--k-hover) }
div.k-mi.danger { color: var(--k-error) }
div.k-mi.dis { color: var(--k-dis) }
div.k-mi > span.k-mi-icon { width: 14px; height: 14px; color: var(--k-muted) }
div.k-mi > div.k-mi-label { flex-grow: 1 }
div.k-sep { height: 1px; margin: 4px 6px; background: var(--k-line); flex-shrink: 0 }
span.k-kbd { padding: 1px 6px; border-radius: 5px; border: 1px solid var(--k-line); background: var(--k-sunken); color: var(--k-muted); font-size: 11px; font-family: monospace }
span.k-badge { padding: 1px 8px; border-radius: 999px; font-size: 11px; font-weight: 600; background: var(--k-sel); color: var(--k-accent) }
span.k-badge.ok { background: rgba(34,197,94,0.15); color: var(--k-ok) }
span.k-badge.warn { background: rgba(245,158,11,0.15); color: var(--k-warn) }
span.k-badge.error { background: rgba(239,68,68,0.15); color: var(--k-error) }
span.k-badge.muted { background: var(--k-hover); color: var(--k-muted) }
div.k-scrim { position: fixed; top: 0; left: 0; right: 0; bottom: 0; z-index: 140; background: var(--k-scrim) }
div.k-dialog { position: fixed; z-index: 150; border-radius: 14px; background: var(--k-panel); border: 1px solid var(--k-line);
               box-shadow: 0 16px 48px var(--k-shadow); display: flex; flex-direction: column }
div.k-dialog-head { display: flex; align-items: center; padding: 14px 16px 6px; font-size: 15px; font-weight: 600 }
div.k-dialog-head > div.k-dialog-title { flex-grow: 1 }
div.k-dialog-body { padding: 6px 16px 12px; flex-grow: 1; white-space: normal; color: var(--k-body); display: flex; flex-direction: column; gap: 10px }
div.k-dialog-foot { display: flex; justify-content: flex-end; gap: 8px; padding: 12px 16px; border-top: 1px solid var(--k-line) }
div.k-tip { position: fixed; z-index: 160; padding: 5px 9px; border-radius: 6px; background: var(--k-fg); color: var(--k-bg); font-size: 12px }
div.k-toasts { position: fixed; right: 16px; bottom: 16px; z-index: 170; width: 300px; display: flex; flex-direction: column; gap: 8px }
div.k-toast { display: flex; align-items: center; gap: 10px; padding: 10px 12px; border-radius: 10px; background: var(--k-panel); border: 1px solid var(--k-line);
              border-left: 3px solid var(--k-info); box-shadow: 0 8px 24px var(--k-shadow) }
div.k-toast.ok { border-left: 3px solid var(--k-ok) }
div.k-toast.warn { border-left: 3px solid var(--k-warn) }
div.k-toast.error { border-left: 3px solid var(--k-error) }
div.k-toast > div.k-toast-text { flex-grow: 1; white-space: normal }
div.k-seg { display: flex; padding: 3px; gap: 2px; border-radius: 9px; background: var(--k-sunken); border: 1px solid var(--k-line) }
button.k-seg-btn { flex-grow: 1; padding: 5px 10px; border-radius: 6px; color: var(--k-muted); text-align: center; font-weight: 500 }
button.k-seg-btn:hover { color: var(--k-fg) }
button.k-seg-btn.on { background: var(--k-accent); color: var(--k-on-accent) }
div.k-stat { display: flex; flex-direction: column; gap: 4px; padding: 12px 14px; border-radius: 12px; background: var(--k-panel); border: 1px solid var(--k-line) }
div.k-stat > div.k-stat-label { color: var(--k-muted); font-size: 11px; font-weight: 600 }
div.k-stat > div.k-stat-value { font-size: 24px; font-weight: 700 }
div.k-stat > div.k-stat-note { color: var(--k-muted); font-size: 11px }
div.k-progress { height: 6px; border-radius: 3px; background: var(--k-line); overflow: hidden }
div.k-progress > div.k-bar { height: 6px; border-radius: 3px; background: var(--k-accent) }
div.k-field { display: flex; flex-direction: column; gap: 6px }
div.k-field > div.k-field-label { color: var(--k-muted); font-size: 11px; font-weight: 600 }
"#;

/// Inline `:root` overrides for the light look: put them on the page root
/// (`El::new("div.k-root").css(LIGHT_VARS)`); custom properties cascade to the subtree.
pub const LIGHT_VARS: &str = "--k-bg: #f6f7f9; --k-panel: #ffffff; --k-fg: #1d1f24; --k-muted: #6b6f78; --k-line: #dfe1e6; \
--k-them: #eceef2; --k-btn: #eceef2; --k-btn-hover: #e2e5ea; --k-track: #c9ccd3; --k-track-hover: #b7bbc4; --k-accent-hover: #2563eb; \
--k-field: #ffffff; --k-sunken: #f0f1f4; --k-hover: #eef0f4; --k-hi: #e6e9ef; --k-sel: rgba(59,130,246,0.14); --k-dis: #b0b4bc; \
--k-body: #3a3d45; --k-scrim: rgba(15,17,21,0.35); --k-shadow: rgba(15,17,21,0.18)";

/// The page-root inline variables for the dark (`""`, the sheet's own `:root`) or light look.
pub fn theme_vars(dark: bool) -> &'static str {
    if dark {
        ""
    } else {
        LIGHT_VARS
    }
}

/// [`KIT_CSS`] parsed.
pub fn sheet() -> Sheet {
    Sheet::parse(KIT_CSS).expect("KIT_CSS parses")
}

/// [`KIT_CSS`] followed by the app's own rules (later rules of equal specificity win).
pub fn sheet_with(app_css: &str) -> Result<Sheet, StyleError> {
    Sheet::parse(&[KIT_CSS, app_css].concat())
}

/// A 24×24 stroke icon (`currentColor`) from its path markup.
pub fn icon(body: &str) -> String {
    format!(r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{body}</svg>"#)
}

/// Icon bodies used by the catalog.
pub mod icons {
    pub const CLOSE: &str = r#"<path d="M6 6l12 12M18 6L6 18"/>"#;
    pub const RIGHT: &str = r#"<path d="M9 6l6 6-6 6"/>"#;
    pub const DOWN: &str = r#"<path d="M6 9l6 6 6-6"/>"#;
    pub const REPLY: &str = r#"<path d="M9 14L4 9l5-5"/><path d="M4 9h10a6 6 0 0 1 6 6v3"/>"#;
    pub const COPY: &str = r#"<rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h8"/>"#;
    pub const SMILE: &str = r#"<circle cx="12" cy="12" r="9"/><path d="M8.5 14.5a4.5 4.5 0 0 0 7 0M9 9.5h.01M15 9.5h.01"/>"#;
    pub const MORE: &str = r#"<path d="M5 12h.01M12 12h.01M19 12h.01"/>"#;
    pub const CHECK: &str = r#"<path d="M5 12.5l4.5 4.5L19 7.5"/>"#;
    pub const SEARCH: &str = r#"<circle cx="11" cy="11" r="6.5"/><path d="M16 16l4 4"/>"#;
    pub const LEFT: &str = r#"<path d="M15 6l-6 6 6 6"/>"#;
    pub const PLUS: &str = r#"<path d="M12 5v14M5 12h14"/>"#;
    pub const SETTINGS: &str = r#"<circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1L7 17M17 7l2.1-2.1"/>"#;
    pub const MENU: &str = r#"<path d="M4 6h16M4 12h16M4 18h16"/>"#;
    pub const SUN: &str = r#"<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M2 12h2M20 12h2M5 5l1.4 1.4M17.6 17.6L19 19M5 19l1.4-1.4M17.6 6.4L19 5"/>"#;
    pub const MOON: &str = r#"<path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z"/>"#;
    pub const SAVE: &str = r#"<path d="M5 3h11l3 3v15H5z"/><path d="M8 3v5h7V3M8 21v-7h8v7"/>"#;
    pub const UNDO: &str = r#"<path d="M4 9h11a5 5 0 0 1 0 10H8"/><path d="M8 5L4 9l4 4"/>"#;
    pub const EXPAND: &str = r#"<path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/>"#;
    pub const PLAY: &str = r#"<path d="M7 4l13 8-13 8z"/>"#;
    pub const GRID: &str = r#"<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>"#;
    pub const IMAGE: &str = r#"<rect x="3" y="4" width="18" height="16" rx="2"/><circle cx="9" cy="10" r="2"/><path d="M21 16l-5-5-9 9"/>"#;
    pub const WINDOW: &str = r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 9h18"/>"#;
    pub const DOWNLOAD: &str = r#"<path d="M12 3v12M7 10l5 5 5-5M4 20h16"/>"#;
    pub const CHART: &str = r#"<path d="M4 20V4M4 20h16M8 16v-5M12 16V8M16 16v-8"/>"#;
    pub const LIST: &str = r#"<path d="M9 6h11M9 12h11M9 18h11M4 6h.01M4 12h.01M4 18h.01"/>"#;
    pub const TERMINAL: &str = r#"<path d="M4 7l5 5-5 5M12 19h8"/>"#;
    pub const TRASH: &str = r#"<path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/>"#;
    pub const EDIT: &str = r#"<path d="M4 20h4L19 9l-4-4L4 16z"/>"#;
    pub const PIN: &str = r#"<path d="M9 4h6l-1 6 4 4H6l4-4zM12 14v6"/>"#;
    pub const BOX: &str = r#"<path d="M21 8l-9-5-9 5 9 5 9-5zM3 8v8l9 5 9-5V8M12 13v8"/>"#;
    pub const BOLT: &str = r#"<path d="M13 2L4 14h7l-1 8 9-12h-7z"/>"#;
}

/// A push button.
pub fn button(id: &str, label: &str) -> El {
    El::new("button.k-btn").id(id).tab().text(label)
}

/// A switch (`role="switch"`): the track is the button's box, the knob its control.
pub fn switch(id: &str, on: bool, name: &str) -> El {
    El::new("button.k-switch").id(id).tab().name(name).control(Control::switch(on))
}

/// A switch whose knob is at `pos` (0 … 1) while it slides.
pub fn switch_at(id: &str, on: bool, pos: f64, name: &str) -> El {
    El::new("button.k-switch").id(id).tab().name(name).control(Control::Switch { on, pos })
}

/// A checkbox with its label; the row is the hit.
pub fn checkbox(id: &str, on: bool, label: &str) -> El {
    El::new("div.k-check").id(id).tab().name(label).child(El::new("span.k-box").control(Control::checkbox(on))).child(El::new("span").text(label))
}

/// A busy spinner at time `t` seconds.
pub fn spinner(t: f64) -> El {
    El::new("span.k-spinner").name("Loading").control(Control::spinner(t))
}

/// A single-line text input showing `edit`.
pub fn input(id: &str, edit: Edit) -> El {
    El::new("div.k-input").id(id).tab().control(Control::input(edit))
}

/// One option of a select.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SelectOpt {
    pub label: String,
    /// A muted second line.
    pub sub: Option<String>,
    /// Shown greyed, not pickable (no hit id).
    pub disabled: bool,
}

impl SelectOpt {
    pub fn new(label: impl Into<String>) -> Self {
        SelectOpt { label: label.into(), ..Default::default() }
    }
    pub fn sub(mut self, s: impl Into<String>) -> Self {
        self.sub = Some(s.into());
        self
    }
    pub fn disabled(mut self, on: bool) -> Self {
        self.disabled = on;
        self
    }
}

/// The closed select: the chosen label and a chevron.
pub fn select_trigger(id: &str, label: &str, open: bool, name: &str) -> El {
    El::new(if open { "select.k-select.open" } else { "select.k-select" }).id(id).tab().name(name).text(label)
}

/// Where a popover `h` tall goes for a trigger at `anchor` in `viewport`: below it, or
/// above when it does not fit below and fits better above; at least the trigger's width.
pub fn popover_at(anchor: Rect, viewport: Rect, w: f64, h: f64) -> Rect {
    let w = w.max(anchor.width);
    let below = viewport.y + viewport.height - (anchor.y + anchor.height + 4.0);
    let above = anchor.y - 4.0 - viewport.y;
    let y = if h > below && above > below { (anchor.y - 4.0 - h).max(viewport.y) } else { anchor.y + anchor.height + 4.0 };
    let x = anchor.x.min(viewport.x + viewport.width - w).max(viewport.x);
    Rect::new(x, y, w, h.min(below.max(above)))
}

/// The open select's list as a `position: fixed` popover at `at` (see [`popover_at`]):
/// it paints above the page and does not scroll with it. Enabled rows are hits
/// `{id}:opt:{i}`; `chosen` is marked `.sel` (with a tick).
pub fn select_popover(id: &str, opts: &[SelectOpt], chosen: Option<usize>, at: Rect) -> El {
    select_list(id, opts, chosen, &ListView::all(), at)
}

/// What part of a long option list a popover shows. The select's own model (keyboard,
/// wheel, type-to-filter) keeps it; the popover only draws it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ListView<'a> {
    /// The keyboard-highlighted row (`.hi`).
    pub hi: Option<usize>,
    /// First shown row.
    pub top: usize,
    /// How many rows show (a scroll thumb appears when there are more).
    pub rows: usize,
    /// A search line with this query (`Some("")`: its placeholder).
    pub query: Option<&'a str>,
    /// Height of a row (sets the thumb's travel; rows are styled by CSS).
    pub row_h: f64,
}

impl ListView<'_> {
    /// Every row, no search line.
    pub fn all() -> Self {
        ListView { hi: None, top: 0, rows: usize::MAX, query: None, row_h: 0.0 }
    }
}

/// [`select_popover`] for a windowed, searchable list: rows `view.top ..` of `opts` (the
/// already filtered options; hits keep their index in `opts`), the highlighted row, a
/// search line and a scroll thumb. "No matches" when `opts` is empty. The popover is
/// `position: fixed` at `at`, so `at` is in the coordinates of the viewport given to
/// [`lay_out_in`](crate::ui::dom::lay_out_in) (the window, origin 0,0). Its rows' `--k-accent`
/// (the chosen row's tick) can be set inline on the returned element.
pub fn select_list(id: &str, opts: &[SelectOpt], chosen: Option<usize>, view: &ListView<'_>, at: Rect) -> El {
    let mut pop = El::new("div.k-pop").id(format!("{id}:pop")).css(format!("top: {}px; left: {}px; width: {}px", at.y, at.x, at.width));
    if let Some(q) = view.query {
        let line = El::new("div.k-search").child(El::new("span.k-mag").svg(icon(icons::SEARCH)));
        pop = pop.child(if q.is_empty() { line.child(El::new("div.k-ph").text("Type to search")) } else { line.child(El::new("div.k-q").text(q)) });
    }
    if opts.is_empty() {
        pop = pop.child(El::new("div.k-empty").text("No matches"));
    }
    let end = view.top.saturating_add(view.rows).min(opts.len());
    for (i, o) in opts.iter().enumerate().take(end).skip(view.top) {
        let mut spec = String::from("div.k-opt");
        if chosen == Some(i) {
            spec.push_str(".sel");
        }
        if view.hi == Some(i) {
            spec.push_str(".hi");
        }
        if o.disabled {
            spec.push_str(".dis");
        }
        let mut label = El::new("div.k-optl").child(El::new("div.k-label").text(&o.label));
        if let Some(s) = &o.sub {
            label = label.child(El::new("div.k-sub").text(s));
        }
        let tick = El::new("span.k-tick");
        let tick = if chosen == Some(i) { tick.svg(icon(icons::CHECK)) } else { tick };
        let mut row = El::new(spec).name(&o.label).child(tick).child(label);
        if !o.disabled {
            row = row.id(format!("{id}:opt:{i}")).tab();
        }
        pop = pop.child(row);
    }
    let n = opts.len();
    if n > view.rows && view.row_h > 0.0 {
        let track = view.row_h * view.rows as f64;
        let th = (track * view.rows as f64 / n as f64).max(18.0);
        let ty = (track - th) * view.top as f64 / (n - view.rows) as f64 + if view.query.is_some() { 34.0 } else { 0.0 } + 4.0;
        pop = pop.child(El::new("div.k-thumb").css(format!("top: {ty}px; height: {th}px")));
    }
    pop
}

/// The option index a select popover hit id names (`{id}:opt:{i}`).
pub fn picked_opt(id: &str, hit: &str) -> Option<usize> {
    hit.strip_prefix(id)?.strip_prefix(":opt:")?.parse().ok()
}

/// A chat message bubble: optional sender line, markdown body ([`inline_md`]), time.
pub fn bubble(id: &str, mine: bool, who: Option<&str>, body: &str, time: Option<&str>) -> El {
    let mut b = El::new(if mine { "div.k-msg.me" } else { "div.k-msg.them" }).id(id);
    if let Some(w) = who {
        b = b.child(El::new("div.k-who").text(w));
    }
    b = b.child(El::new("div.k-body").runs(inline_md(body)));
    if let Some(t) = time {
        b = b.child(El::new("div.k-time").text(t));
    }
    b
}

/// A hover action row (reply, react, copy, more…): `(key, icon body, name)` per button,
/// hits `{id}:{key}`.
pub fn action_row(id: &str, actions: &[(&str, &str, &str)]) -> El {
    El::new("div.k-actions").id(id).children(actions.iter().map(|(k, ic, n)| El::new("button.k-act").id(format!("{id}:{k}")).tab().name(*n).svg(icon(ic))))
}

/// A collapsible card: the head (hit `id`) toggles `open`; the body shows when open.
pub fn card(id: &str, title: &str, open: bool, body: El) -> El {
    let head = El::new("div.k-card-head")
        .id(id)
        .tab()
        .name(title)
        .child(El::new("span.k-chev").svg(icon(if open { icons::DOWN } else { icons::RIGHT })))
        .child(El::new("span").text(title));
    El::new(if open { "div.k-card.open" } else { "div.k-card" }).child(head).child_if(open, || El::new("div.k-card-body").child(body))
}

/// A filter chip.
pub fn chip(id: &str, label: &str, on: bool) -> El {
    El::new(if on { "button.k-chip.on" } else { "button.k-chip" }).id(id).tab().text(label)
}

/// A banner (`kind`: `info`, `warn`, `error`) with an optional action button
/// (`{id}:act`) and a close button (`{id}:close`).
pub fn banner(id: &str, kind: &str, text: &str, action: Option<&str>, closable: bool) -> El {
    let mut b = El::new(format!("div.k-banner.{kind}")).id(id).name(text).child(El::new("div.k-banner-text").text(text));
    if let Some(a) = action {
        b = b.child(El::new("button.k-btn.ghost").id(format!("{id}:act")).tab().text(a));
    }
    if closable {
        b = b.child(El::new("button.k-act").id(format!("{id}:close")).tab().name("Close").svg(icon(icons::CLOSE)));
    }
    b
}

/// A side panel docked right (`position: fixed`: it stays while the page scrolls), with a
/// title row and a close button (`{id}:close`).
pub fn side_panel(id: &str, title: &str, body: impl IntoIterator<Item = El>) -> El {
    El::new("div.k-side")
        .id(id)
        .child(
            El::new("div.k-side-head")
                .child(El::new("div.k-side-title").text(title))
                .child(El::new("button.k-act").id(format!("{id}:close")).tab().name("Close").svg(icon(icons::CLOSE))),
        )
        .child(El::new("div.k-side-body").children(body))
}

/// An embed slot box (`div.k-slot`) named `key`, `w` × `h`: a kit widget or canvas goes
/// in its content box (see [`Dom::slots`]).
pub fn slot(key: &str, w: f64, h: f64) -> El {
    El::new("div.k-slot").slot(key).css(format!("width: {w}px; height: {h}px"))
}

/// A tab strip: one `button.k-tab` per label (`.on` for `active`), hits `{id}:tab:{i}`.
pub fn tabs(id: &str, labels: &[&str], active: usize) -> El {
    El::new("div.k-tabs").id(id).children(labels.iter().enumerate().map(|(i, l)| {
        El::new(if i == active { "button.k-tab.on" } else { "button.k-tab" }).id(format!("{id}:tab:{i}")).tab().text(*l)
    }))
}

/// A segmented control (one choice of a few, e.g. Dark / Light): hits `{id}:{i}`.
pub fn segmented(id: &str, labels: &[&str], active: usize) -> El {
    El::new("div.k-seg").id(id).children(labels.iter().enumerate().map(|(i, l)| {
        El::new(if i == active { "button.k-seg-btn.on" } else { "button.k-seg-btn" }).id(format!("{id}:{i}")).tab().text(*l)
    }))
}

/// The tab, segment, row or item index a hit id names (`{id}:{kind}:{i}`, or `{id}:{i}` for an empty `kind`).
pub fn picked(id: &str, kind: &str, hit: &str) -> Option<usize> {
    let rest = hit.strip_prefix(id)?.strip_prefix(':')?;
    let rest = if kind.is_empty() { rest } else { rest.strip_prefix(kind)?.strip_prefix(':')? };
    rest.parse().ok()
}

/// A list of rows (`div.k-li`, `.sel` for `selected`), each a main label and an optional
/// muted side text; hits `{id}:row:{i}`.
pub fn list(id: &str, rows: &[(&str, Option<&str>)], selected: Option<usize>) -> El {
    El::new("div.k-list").id(id).children(rows.iter().enumerate().map(|(i, (main, side))| {
        El::new(if selected == Some(i) { "div.k-li.sel" } else { "div.k-li" })
            .id(format!("{id}:row:{i}"))
            .tab()
            .name(*main)
            .child(El::new("div.k-li-main").text(*main))
            .child_if(side.is_some(), || El::new("div.k-li-side").text(side.unwrap_or_default()))
    }))
}

/// A table: a header row and data rows on one CSS grid track list (`columns`, e.g.
/// `"1fr 80px"`); `num` columns align right. Rows are hits `{id}:row:{i}`, `.sel` for `selected`.
pub fn table(id: &str, head: &[&str], columns: &str, num: &[usize], rows: &[Vec<String>], selected: Option<usize>) -> El {
    let grid = format!("grid-template-columns: {columns}");
    let cell = |c: usize, t: &str| El::new(if num.contains(&c) { "div.k-td.num" } else { "div.k-td" }).text(t);
    let mut t = El::new("div.k-table").id(id).child(
        El::new("div.k-tr.k-th").css(grid.clone()).children(head.iter().enumerate().map(|(c, h)| cell(c, h))),
    );
    for (i, r) in rows.iter().enumerate() {
        t = t.child(
            El::new(if selected == Some(i) { "div.k-tr.sel" } else { "div.k-tr" })
                .id(format!("{id}:row:{i}"))
                .tab()
                .name(r.first().map(String::as_str).unwrap_or_default())
                .css(grid.clone())
                .children(r.iter().enumerate().map(|(c, v)| cell(c, v))),
        );
    }
    t
}

/// One entry of a [`menu`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MenuItem {
    pub label: String,
    /// An icon body ([`icons`]).
    pub icon: Option<&'static str>,
    /// A shortcut hint (`Ctrl+K`).
    pub kbd: Option<String>,
    /// Red (a destructive action).
    pub danger: bool,
    /// Greyed, not pickable (no hit id).
    pub disabled: bool,
    /// A divider line instead of an item.
    pub separator: bool,
}

impl MenuItem {
    pub fn new(label: impl Into<String>) -> Self {
        MenuItem { label: label.into(), ..Default::default() }
    }
    pub fn separator() -> Self {
        MenuItem { separator: true, ..Default::default() }
    }
    pub fn icon(mut self, body: &'static str) -> Self {
        self.icon = Some(body);
        self
    }
    pub fn kbd(mut self, k: impl Into<String>) -> Self {
        self.kbd = Some(k.into());
        self
    }
    pub fn danger(mut self, on: bool) -> Self {
        self.danger = on;
        self
    }
    pub fn disabled(mut self, on: bool) -> Self {
        self.disabled = on;
        self
    }
}

/// A context menu / command list as a `position: fixed` box at `at` (viewport
/// coordinates; see [`popover_at`]). Enabled items are hits `{id}:item:{i}` (`i` counts
/// separators too, so it is the index into `items`).
pub fn menu(id: &str, title: Option<&str>, items: &[MenuItem], at: Rect) -> El {
    let mut m = El::new("div.k-menu").id(format!("{id}:menu")).css(format!("top: {}px; left: {}px; width: {}px", at.y, at.x, at.width));
    if let Some(t) = title {
        m = m.child(El::new("div.k-menu-title").text(t));
    }
    m.children(items.iter().enumerate().map(|(i, it)| {
        if it.separator {
            return El::new("div.k-sep");
        }
        let mut spec = String::from("div.k-mi");
        if it.danger {
            spec.push_str(".danger");
        }
        if it.disabled {
            spec.push_str(".dis");
        }
        let mut row = El::new(spec).name(&it.label);
        if let Some(ic) = it.icon {
            row = row.child(El::new("span.k-mi-icon").svg(icon(ic)));
        }
        row = row.child(El::new("div.k-mi-label").text(&it.label));
        if let Some(k) = &it.kbd {
            row = row.child(kbd(k));
        }
        if !it.disabled {
            row = row.id(format!("{id}:item:{i}")).tab();
        }
        row
    }))
}

/// A keyboard shortcut hint (`span.k-kbd`).
pub fn kbd(text: &str) -> El {
    El::new("span.k-kbd").text(text)
}

/// A small status pill (`kind`: `""`, `ok`, `warn`, `error`, `muted`).
pub fn badge(text: &str, kind: &str) -> El {
    El::new(if kind.is_empty() { "span.k-badge".to_string() } else { format!("span.k-badge.{kind}") }).text(text)
}

/// A `w × h` box centred in `viewport` (where a [`dialog`] goes).
pub fn centered(viewport: Rect, w: f64, h: f64) -> Rect {
    let (w, h) = (w.min(viewport.width), h.min(viewport.height));
    Rect::new(viewport.x + (viewport.width - w) / 2.0, viewport.y + (viewport.height - h) / 2.0, w, h)
}

/// A modal dialog at `at` over a full-viewport scrim (`{id}:scrim`, a hit so a click
/// outside can close it), with a title, a close button (`{id}:close`), `body` and footer
/// buttons `{id}:foot:{i}` (the first is `.primary`).
pub fn dialog(id: &str, title: &str, body: El, footer: &[&str], at: Rect) -> El {
    let foot = El::new("div.k-dialog-foot").children(footer.iter().enumerate().map(|(i, l)| {
        El::new(if i == 0 { "button.k-btn.primary" } else { "button.k-btn" }).id(format!("{id}:foot:{i}")).tab().text(*l)
    }));
    El::new("div.k-modal")
        .child(El::new("div.k-scrim").id(format!("{id}:scrim")))
        .child(
            El::new("div.k-dialog")
                .id(id)
                .name(title)
                .css(format!("top: {}px; left: {}px; width: {}px; height: {}px", at.y, at.x, at.width, at.height))
                .child(
                    El::new("div.k-dialog-head")
                        .child(El::new("div.k-dialog-title").text(title))
                        .child(El::new("button.k-act").id(format!("{id}:close")).tab().name("Close").svg(icon(icons::CLOSE))),
                )
                .child(El::new("div.k-dialog-body").child(body))
                .child_if(!footer.is_empty(), || foot),
        )
}

/// A tooltip below `anchor` (fixed, viewport coordinates).
pub fn tooltip(text: &str, anchor: Rect) -> El {
    El::new("div.k-tip").text(text).css(format!("top: {}px; left: {}px", anchor.y + anchor.height + 6.0, anchor.x))
}

/// A toast stack in the bottom-right corner of the viewport: `(kind, text)` per toast
/// (`kind` as for [`banner`]); each has a close hit `{id}:{i}:close`.
pub fn toasts(id: &str, items: &[(&str, &str)]) -> El {
    El::new("div.k-toasts").id(id).children(items.iter().enumerate().map(|(i, (kind, text))| {
        El::new(if kind.is_empty() { "div.k-toast".to_string() } else { format!("div.k-toast.{kind}") })
            .name(*text)
            .child(El::new("div.k-toast-text").text(*text))
            .child(El::new("button.k-act").id(format!("{id}:{i}:close")).tab().name("Close").svg(icon(icons::CLOSE)))
    }))
}

/// A metric tile: a caption, a big value and a muted note.
pub fn stat(id: &str, label: &str, value: &str, note: &str) -> El {
    El::new("div.k-stat")
        .id(id)
        .name(label)
        .child(El::new("div.k-stat-label").text(label))
        .child(El::new("div.k-stat-value").text(value))
        .child_if(!note.is_empty(), || El::new("div.k-stat-note").text(note))
}

/// A progress bar, `v` in 0 … 1.
pub fn progress(v: f64) -> El {
    let pct = (v.clamp(0.0, 1.0) * 100.0 * 10.0).round() / 10.0;
    El::new("div.k-progress").child(El::new("div.k-bar").css(format!("width: {pct}%")))
}

/// A labelled form row: a small caption above `control`.
pub fn field(label: &str, control: El) -> El {
    El::new("div.k-field").child(El::new("div.k-field-label").text(label)).child(control)
}

/// Inline markdown to DOM runs: `**bold**`, `*italic*` / `_italic_`, `***both***`,
/// `` `code` `` (verbatim), http(s) links (`a.link`, hit `url:…`) and `@nick` mentions
/// (`a.mention`, hit `nick:…`). Unclosed markers stay literal; `_` only opens at a word
/// start and closes at a word end (`snake_case` stays as is).
pub fn inline_md(text: &str) -> Vec<Run> {
    let cs: Vec<char> = text.chars().collect();
    let mut out: Vec<Run> = Vec::new();
    let (mut bold, mut ital) = (false, false);
    let mut cur = String::new();
    let flush = |out: &mut Vec<Run>, cur: &mut String, bold: bool, ital: bool| {
        if cur.is_empty() {
            return;
        }
        let spec = match (bold, ital) {
            (true, true) => "b.i",
            (true, false) => "b",
            (false, true) => "i",
            _ => "span",
        };
        for sp in crate::ui::widgets::composite::chat_thread::linkify(cur, "link", 400) {
            let r = match crate::ui::widgets::composite::chat_thread::span_link(&sp) {
                Some(crate::ui::widgets::composite::chat_thread::Link::Mention(n)) => Run::new("a.mention", sp.text.as_str()).id(format!("nick:{n}")),
                Some(crate::ui::widgets::composite::chat_thread::Link::Url(u)) => Run::new("a.link", sp.text.as_str()).id(format!("url:{u}")),
                None => Run::new(spec, sp.text.as_str()),
            };
            let r = if (bold || ital) && r.spec.starts_with('a') {
                r.css(format!("{}{}", if bold { "font-weight: 700;" } else { "" }, if ital { "font-style: italic" } else { "" }))
            } else if spec == "b.i" {
                r.css("font-weight: 700; font-style: italic")
            } else if spec == "i" {
                r.css("font-style: italic")
            } else if spec == "b" {
                r.css("font-weight: 700")
            } else {
                r
            };
            out.push(r);
        }
        cur.clear();
    };
    let closes = |from: usize, pat: &[char]| -> bool {
        let n = pat.len();
        (from..cs.len().saturating_sub(n - 1)).any(|j| cs[j..j + n] == *pat && j > from)
    };
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c == '`' {
            if let Some(j) = (i + 1..cs.len()).find(|&j| cs[j] == '`') {
                flush(&mut out, &mut cur, bold, ital);
                let code: String = cs[i + 1..j].iter().collect();
                if !code.is_empty() {
                    out.push(Run::new("code", code));
                }
                i = j + 1;
                continue;
            }
        }
        if c == '*' && i + 1 < cs.len() && cs[i + 1] == '*' && (bold || closes(i + 2, &['*', '*'])) {
            flush(&mut out, &mut cur, bold, ital);
            bold = !bold;
            i += 2;
            continue;
        }
        if c == '*' && (ital || (i + 1 < cs.len() && !cs[i + 1].is_whitespace() && closes(i + 1, &['*']))) {
            flush(&mut out, &mut cur, bold, ital);
            ital = !ital;
            i += 1;
            continue;
        }
        if c == '_' {
            let word_before = i > 0 && cs[i - 1].is_alphanumeric();
            let word_after = i + 1 < cs.len() && cs[i + 1].is_alphanumeric();
            let opens = !ital
                && !word_before
                && i + 1 < cs.len()
                && !cs[i + 1].is_whitespace()
                && (i + 2..cs.len()).any(|j| cs[j] == '_' && !cs[j - 1].is_whitespace() && (j + 1 == cs.len() || !cs[j + 1].is_alphanumeric()));
            if opens || (ital && !word_after) {
                flush(&mut out, &mut cur, bold, ital);
                ital = !ital;
                i += 1;
                continue;
            }
        }
        cur.push(c);
        i += 1;
    }
    flush(&mut out, &mut cur, bold, ital);
    out
}

/// A virtualized list: only the rows in (or near) the viewport are built and laid out;
/// spacers stand for the rest. Row heights start at an estimate and are learned from
/// laid-out rows ([`VirtualList::measure`]), so long chats stay cheap.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VirtualList {
    heights: Vec<Option<f64>>,
    /// Height of a row not measured yet.
    pub estimate: f64,
    /// Gap between rows.
    pub gap: f64,
}

/// The rows a [`VirtualList`] shows and the space around them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Window {
    pub start: usize,
    /// One past the last shown row.
    pub end: usize,
    /// Height of the spacer above the first shown row.
    pub top: f64,
    /// Height of the spacer below the last shown row.
    pub bottom: f64,
}

impl VirtualList {
    pub fn new(len: usize, estimate: f64) -> Self {
        VirtualList { heights: vec![None; len], estimate, gap: 0.0 }
    }
    pub fn len(&self) -> usize {
        self.heights.len()
    }
    pub fn is_empty(&self) -> bool {
        self.heights.is_empty()
    }
    /// Grow or shrink to `n` rows (new rows unmeasured; kept rows keep their height).
    pub fn set_len(&mut self, n: usize) {
        self.heights.resize(n, None);
    }
    /// Forget a row's height (its content changed).
    pub fn invalidate(&mut self, i: usize) {
        if let Some(h) = self.heights.get_mut(i) {
            *h = None;
        }
    }
    fn h(&self, i: usize) -> f64 {
        self.heights[i].unwrap_or(self.estimate)
    }
    /// The whole list's height (measured rows plus estimates).
    pub fn total(&self) -> f64 {
        (0..self.len()).map(|i| self.h(i)).sum::<f64>() + self.gap * self.len().saturating_sub(1) as f64
    }
    /// Top of row `i` in list coordinates.
    pub fn offset_of(&self, i: usize) -> f64 {
        (0..i.min(self.len())).map(|k| self.h(k) + self.gap).sum()
    }
    /// The row at list `y`.
    pub fn index_at(&self, y: f64) -> usize {
        let mut acc = 0.0;
        for i in 0..self.len() {
            acc += self.h(i) + self.gap;
            if y < acc {
                return i;
            }
        }
        self.len().saturating_sub(1)
    }
    /// The rows covering `scroll .. scroll + view`, plus `overscan` pixels each way.
    pub fn window(&self, scroll: f64, view: f64, overscan: f64) -> Window {
        if self.is_empty() {
            return Window::default();
        }
        let a = (scroll - overscan).max(0.0);
        let b = scroll + view + overscan;
        let start = self.index_at(a);
        let mut end = start;
        let mut y = self.offset_of(start);
        while end < self.len() && y < b {
            y += self.h(end) + self.gap;
            end += 1;
        }
        let top = self.offset_of(start);
        let bottom = (self.total() - self.offset_of(end)).max(0.0);
        Window { start, end, top, bottom }
    }
    /// The list element: `container` holding spacers and the rows `row(i)` for `w`.
    /// A container column `gap` must equal [`VirtualList::gap`] (spacers are emitted
    /// only when they stand for rows, so the gap is not doubled).
    pub fn el(&self, container: El, w: Window, mut row: impl FnMut(usize) -> El) -> El {
        let mut c = container;
        if w.start > 0 {
            c = c.child(El::new("div.k-vspace").css(format!("height: {}px", (w.top - self.gap).max(0.0))));
        }
        for i in w.start..w.end {
            c = c.child(row(i));
        }
        if w.end < self.len() {
            c = c.child(El::new("div.k-vspace").css(format!("height: {}px", w.bottom)));
        }
        c
    }
    /// Learn the shown rows' heights from a laid-out tree: row `i`'s element has id
    /// `id_of(i)`. Returns whether any height changed (re-lay out to settle).
    pub fn measure(&mut self, dom: &Dom, w: Window, id_of: impl Fn(usize) -> String) -> bool {
        let mut changed = false;
        for i in w.start..w.end.min(self.len()) {
            if let Some(r) = dom.rect(&id_of(i)) {
                if self.heights[i].is_none_or(|h| (h - r.height).abs() > 0.25) {
                    self.heights[i] = Some(r.height);
                    changed = true;
                }
            }
        }
        changed
    }
}

#[cfg(all(test, feature = "flow-flex"))]
mod tests {
    use super::*;
    use crate::testing::NullRenderContext;
    use crate::ui::dom::lay_out_in;
    use crate::ui::style::State;

    #[test]
    fn markdown_bold_italic_code_links() {
        let r = inline_md("a **b** *i* _u_ snake_case ***bi*** `x*y` see https://e.x @bob");
        let specs: Vec<(&str, &str)> = r.iter().map(|r| (r.spec.as_str(), r.text.as_str())).collect();
        assert!(specs.contains(&("b", "b")));
        assert!(specs.contains(&("i", "i")));
        assert!(specs.contains(&("i", "u")));
        assert!(specs.contains(&("b.i", "bi")));
        assert!(specs.contains(&("code", "x*y")));
        assert!(specs.iter().any(|(s, t)| *s == "span" && t.contains("snake_case")));
        assert!(r.iter().any(|r| r.spec == "a.link" && r.id.as_deref() == Some("url:https://e.x")));
        assert!(r.iter().any(|r| r.spec == "a.mention" && r.id.as_deref() == Some("nick:bob")));
        // unclosed markers stay literal
        let r = inline_md("2 * 3 and *open");
        assert_eq!(r.iter().map(|r| r.text.as_str()).collect::<String>(), "2 * 3 and *open");
    }

    #[test]
    fn virtual_list_windows_and_learns() {
        let mut v = VirtualList::new(1000, 40.0);
        let w = v.window(4000.0, 400.0, 0.0);
        assert_eq!((w.start, w.end), (100, 110));
        assert_eq!(w.top, 4000.0);
        assert_eq!(w.top + (w.end - w.start) as f64 * 40.0 + w.bottom, v.total());
        let mut ctx = NullRenderContext::default();
        let el = v.el(El::new("div").css("display:flex;flex-direction:column"), w, |i| El::new("div").id(format!("r{i}")).css(if i % 2 == 0 { "height:60px" } else { "height:40px" }));
        let dom = crate::ui::dom::lay_out(&mut ctx, &sheet(), &el, 0.0, 0.0, 300.0, &|_| State::NONE);
        assert!(v.measure(&dom, w, |i| format!("r{i}")));
        assert_eq!(v.total(), 990.0 * 40.0 + 5.0 * 60.0 + 5.0 * 40.0);
    }

    #[test]
    fn select_popover_is_fixed_and_skips_disabled() {
        let opts = [SelectOpt::new("Auto").sub("follow the system"), SelectOpt::new("Dark"), SelectOpt::new("Light").disabled(true)];
        let page = El::new("div.k-root")
            .css("height: 2000px")
            .child(select_trigger("theme", "Auto", true, "Theme"))
            .child(select_popover("theme", &opts, Some(0), Rect::new(10.0, 50.0, 200.0, 120.0)));
        let mut ctx = NullRenderContext::default();
        let vp = Rect::new(0.0, 0.0, 800.0, 600.0);
        let dom = lay_out_in(&mut ctx, &sheet(), &page, 0.0, 0.0, 800.0, Some(vp), &|_| State::NONE);
        assert!(dom.rect("theme:opt:0").is_some() && dom.rect("theme:opt:1").is_some());
        assert!(dom.rect("theme:opt:2").is_none());
        let pop = dom.rect("theme:pop").unwrap();
        assert_eq!((pop.x, pop.y, pop.width), (10.0, 50.0, 200.0));
        // scrolling the page leaves the popover in place and on top
        let moved = dom.clone().moved(0.0, -300.0);
        assert_eq!(moved.rect("theme:pop"), Some(pop));
        assert_eq!(moved.rect("theme").map(|r| r.y), dom.rect("theme").map(|r| r.y - 300.0));
        assert_eq!(moved.hit(20.0, 60.0), Some("theme:opt:0"));
        assert_eq!(picked_opt("theme", "theme:opt:1"), Some(1));
    }

    #[test]
    fn select_list_windows_highlights_and_searches() {
        let opts: Vec<SelectOpt> = (0..20).map(|i| SelectOpt::new(format!("Zone {i}"))).collect();
        let view = ListView { hi: Some(6), top: 5, rows: 4, query: Some("zo"), row_h: 30.0 };
        let pop = select_list("tz", &opts, Some(7), &view, Rect::new(0.0, 0.0, 240.0, 200.0));
        let mut ctx = NullRenderContext::default();
        let vp = Rect::new(0.0, 0.0, 800.0, 600.0);
        let dom = lay_out_in(&mut ctx, &sheet(), &pop, 0.0, 0.0, 800.0, Some(vp), &|_| State::NONE);
        let shown: Vec<&str> = dom.hits().map(|(id, _)| id).filter(|id| id.starts_with("tz:opt:")).collect();
        assert_eq!(shown, ["tz:opt:5", "tz:opt:6", "tz:opt:7", "tz:opt:8"]);
        assert_eq!(dom.get("tz:opt:6").and_then(|p| p.style.background.as_deref()), Some("#33363d"));
        assert!(dom.get("tz:opt:5").is_some_and(|p| p.style.background.is_none()));
        let row = dom.rect("tz:opt:5").unwrap();
        assert!(row.y > 30.0, "below the search line");
        let none = select_list("tz", &[], None, &ListView { query: Some(""), ..ListView::all() }, Rect::new(0.0, 0.0, 240.0, 80.0));
        let dom = lay_out_in(&mut ctx, &sheet(), &none, 0.0, 0.0, 800.0, Some(vp), &|_| State::NONE);
        assert!(dom.hits().all(|(id, _)| !id.contains(":opt:")));
    }

    #[test]
    fn controls_are_css_styled_and_named() {
        let page = El::new("div.k-root")
            .child(switch("wifi", true, "Wi-Fi"))
            .child(checkbox("agree", false, "I agree"))
            .child(input("q", Edit::new("hello").placeholder("Search").caret(5)))
            .child(spinner(0.3));
        let mut ctx = NullRenderContext::default();
        let dom = crate::ui::dom::lay_out(&mut ctx, &sheet(), &page, 0.0, 0.0, 400.0, &|_| State::NONE);
        let sw = dom.get("wifi").unwrap();
        assert_eq!((sw.rect.width, sw.rect.height), (36.0, 20.0));
        // :checked picked the accent background
        assert_eq!(sw.style.background.as_deref(), Some("#3b82f6"));
        // :focus while the input has a caret
        assert_eq!(dom.get("q").unwrap().style.border_color.as_deref(), Some("#3b82f6"));
        let a = dom.a11y();
        assert!(a.iter().any(|n| n.id == "wifi" && n.role == "switch" && n.name == "Wi-Fi" && n.checked == Some(true)));
        assert!(a.iter().any(|n| n.id == "q" && n.role == "textbox" && n.name == "hello"));
        assert!(a.iter().any(|n| n.id == "agree" && n.name == "I agree"));
        assert!(dom.get("q").unwrap().caret_rect().is_some());
    }

    #[test]
    fn light_vars_retheme_the_whole_kit() {
        let page = |vars: &str| {
            El::new("div.k-root")
                .css(vars)
                .child(button("b", "Save"))
                .child(input("q", Edit::new("x")))
                .child(tabs("t", &["A", "B"], 1))
        };
        let mut ctx = NullRenderContext::default();
        let dark = crate::ui::dom::lay_out(&mut ctx, &sheet(), &page(theme_vars(true)), 0.0, 0.0, 400.0, &|_| State::NONE);
        let light = crate::ui::dom::lay_out(&mut ctx, &sheet(), &page(theme_vars(false)), 0.0, 0.0, 400.0, &|_| State::NONE);
        assert_eq!(dark.get("b").and_then(|p| p.style.background.as_deref()), Some("#2b2e34"));
        assert_eq!(light.get("b").and_then(|p| p.style.background.as_deref()), Some("#eceef2"));
        assert_eq!(light.get("q").and_then(|p| p.style.background.as_deref()), Some("#ffffff"));
        assert_eq!(light.get("t:tab:1").and_then(|p| p.style.color.as_deref()), Some("#1d1f24"));
        // No colour literal is left outside :root except the fixed bubble/link tints and knobs.
        for line in KIT_CSS.lines().filter(|l| !l.starts_with(":root") && !l.starts_with("        --")) {
            if line.contains("k-msg") || line.contains("k-body") || line.contains("k-banner") || line.contains("k-badge") || line.contains("k-toast") {
                continue;
            }
            let rest = line.replace("--knob: #ffffff", "").replace("--check: #ffffff", "");
            assert!(!rest.contains(": #") && !rest.contains(" #"), "hard-coded colour in {line}");
        }
    }

    #[test]
    fn tabs_segmented_list_table_hits() {
        let rows = vec![vec!["alpha".to_string(), "7".to_string()], vec!["beta".to_string(), "14".to_string()]];
        let page = El::new("div.k-root")
            .child(tabs("tb", &["One", "Two", "Three"], 0))
            .child(segmented("theme", &["Dark", "Light"], 1))
            .child(list("ls", &[("alpha", Some("3")), ("beta", None)], Some(1)))
            .child(table("tt", &["Name", "Count"], "1fr 80px", &[1], &rows, Some(0)));
        let mut ctx = NullRenderContext::default();
        let dom = crate::ui::dom::lay_out(&mut ctx, &sheet(), &page, 0.0, 0.0, 400.0, &|_| State::NONE);
        let (a, b) = (dom.rect("tb:tab:0").unwrap(), dom.rect("tb:tab:2").unwrap());
        assert!(b.x > a.x && (a.y - b.y).abs() < 0.01, "tabs share a row");
        assert!(dom.get("theme:1").is_some_and(|p| p.style.background.as_deref() == Some("#3b82f6")));
        assert_eq!(picked("theme", "", "theme:1"), Some(1));
        assert_eq!(picked("tb", "tab", "tb:tab:2"), Some(2));
        assert_eq!(picked("tt", "row", "tb:tab:2"), None);
        assert!(dom.get("ls:row:1").is_some_and(|p| p.style.background.is_some()));
        // Table cells sit on the grid: the count column is 80px at the right.
        let row = dom.rect("tt:row:0").unwrap();
        assert!(row.height >= 30.0);
        assert!(dom.get("tt:row:0").is_some_and(|p| p.style.background.is_some()), ".sel row tinted");
    }

    #[test]
    fn menu_dialog_toasts_are_fixed_layers() {
        let vp = Rect::new(0.0, 0.0, 800.0, 600.0);
        let items = [MenuItem::new("Rename").kbd("F2"), MenuItem::separator(), MenuItem::new("Locked").disabled(true), MenuItem::new("Delete").danger(true)];
        let page = El::new("div.k-root")
            .css("height: 2000px")
            .child(menu("m", Some("Row"), &items, Rect::new(100.0, 100.0, 200.0, 0.0)))
            .child(dialog("d", "Details", El::new("div").text("Body"), &["OK", "Cancel"], centered(vp, 360.0, 200.0)))
            .child(toasts("ts", &[("ok", "Saved"), ("", "Synced")]))
            .child(tooltip("Saves the name", Rect::new(10.0, 10.0, 80.0, 30.0)));
        let mut ctx = NullRenderContext::default();
        let dom = lay_out_in(&mut ctx, &sheet(), &page, 0.0, 0.0, 800.0, Some(vp), &|_| State::NONE);
        assert!(dom.rect("m:item:0").is_some() && dom.rect("m:item:3").is_some());
        assert!(dom.rect("m:item:2").is_none(), "disabled item is not a hit");
        assert_eq!(dom.rect("m:menu").map(|r| (r.x, r.y)), Some((100.0, 100.0)));
        let d = dom.rect("d").unwrap();
        assert_eq!((d.x, d.y, d.width, d.height), (220.0, 200.0, 360.0, 200.0));
        assert_eq!(dom.rect("d:scrim"), Some(vp), "scrim covers the viewport");
        // The dialog is above the scrim, the scrim above the page.
        assert_eq!(dom.hit(400.0, 300.0).map(|s| s.starts_with("d")), Some(true));
        assert_eq!(dom.hit(10.0, 590.0), Some("d:scrim"));
        let t = dom.rect("ts").unwrap();
        assert!((t.x + t.width - 784.0).abs() < 0.5 && (t.y + t.height - 584.0).abs() < 0.5, "toasts sit 16px from the bottom-right: {t:?}");
        assert!(dom.rect("ts:1:close").is_some());
        let moved = dom.clone().moved(0.0, -500.0);
        assert_eq!(moved.rect("d"), Some(d), "fixed layers do not scroll");
    }

    #[test]
    fn stat_progress_badge_field() {
        let page = El::new("div.k-root").css("width: 300px")
            .child(stat("s", "FRAMES", "1 204", "+12 / s"))
            .child(progress(0.25))
            .child(badge("live", "ok"))
            .child(field("Name", input("n", Edit::new(""))));
        let mut ctx = NullRenderContext::default();
        let dom = crate::ui::dom::lay_out(&mut ctx, &sheet(), &page, 0.0, 0.0, 300.0, &|_| State::NONE);
        assert!(dom.rect("s").is_some_and(|r| r.height > 40.0));
        let bar = dom.boxes.iter().find(|b| b.style.background.as_deref() == Some("#3b82f6") && b.rect.height == 6.0).expect("bar");
        assert!((bar.rect.width - 75.0).abs() < 0.5, "25% of 300: {:?}", bar.rect);
        assert!(dom.rect("n").is_some());
    }
}
