//! Mobile app-shell widgets: ready-made [`El`] trees for phone screens, styled by
//! [`MOBILE_CSS`] (monochrome, the Settings look; only small status dots carry colour;
//! content is opaque, only overlays — sheet, drawer, toast — are glass; the action modal is opaque),
//! plus the pure state machines that move them under a finger or a mouse
//! ([`SheetState`], [`DrawerState`], [`SwipeState`]). The framework's
//! `widgets::mobile` feeds these from touch gestures and mouse drags.
//!
//! | widget | builder | element / classes | hit ids |
//! |---|---|---|---|
//! | screen | [`screen`] | `div.m-screen` | — |
//! | large title | [`large_title`] | `div.m-top` > `div.m-h1`, `button.m-ibtn`, `div.m-search` | `{id}:{key}`, `{id}:search` |
//! | top bar + back | [`top_bar`] | `div.m-nav` > `button.m-back`, `div.m-ttl`, `div.m-nav-end` | `{id}:back`, `{id}:{key}` |
//! | bottom tab bar | [`tab_bar`] | `div.m-tabs` > `button.m-tab` (`.on`) > `span.m-tbadge` / `span.m-dot` | `{id}:{i}` |
//! | list row | [`row`] | `div.m-row` > `div.m-av`, `div.m-main` > `div.m-l1`, `div.m-l2` | `id` |
//! | swipe row | [`swipe_row`] | `div.m-swipe` > `div.m-acts.r` / `.l` > `button.m-sact` + the row | `{id}:act:{key}` |
//! | section label | [`section`] | `div.m-sec` | — |
//! | edge drawer | [`drawer`] | `div.m-scrim` + `div.m-drawer.m-glass` > `div.m-ditem` (`.on`), `div.m-dsec` | `{id}:scrim`, `{id}:{key}` |
//! | side rail | [`side_rail`] | `div.m-rail` > `button.m-ritem` (`.on`) | `{id}:{i}` |
//! | floating button | [`fab`] | `button.m-fab` | `id` |
//! | action modal | [`action_modal`] | `div.m-scrim` + `div.m-sheet.m-solid` > `div.m-srow` | `{id}:scrim`, `{id}:close`, `{id}:{key}` |
//! | bottom sheet | [`bottom_sheet`] | `div.m-sheet.m-glass` (`top` from [`SheetState`]) > `div.m-grab` | `{id}:grab`, `{id}:scrim` |
//! | toast | [`toast`] | `div.m-toast.m-glass` > `div.m-toast-tx`, `button.m-toast-act` | `{id}:act` |
//! | segmented | [`segmented`] | `div.m-seg` > `button.m-seg-b` (`.on`) > `span.m-seg-n` | `{id}:{i}` |
//! | composer | [`composer`] | `div.m-composer` > `button.m-ibtn`, `div.m-inp` (slot or input), `button.m-send` (`.off`) | `{id}:plus`, `{id}:field`, `{id}:send` |
//! | agent card | [`hero`], [`quick_actions`], [`kv_group`] | `div.m-hero`, `div.m-qa` > `button.m-q`, `div.m-kv.m-tile` > `div.m-kvr` | `{id}:{key}` |
//! | button | [`btn`] | `button.m-btn` (`.pri`, `.ghost`, `.block`) | `id` |
//! | search bar | [`search_bar`] | `div.m-sbar` > `div.m-search.m-sfield` > `input.m-sinput`, `button.m-sclear`; `button.m-scancel` | `id`, `{id}:clear`, `{id}:cancel` |
//! | virtual list | [`VirtualList::el`] | `div.m-vlist` > `div.m-vbody` > `div.m-vpad`, `div.m-vrow` > the row; `div.m-empty` | `id`, the rows' |
//!
//! Geometry the host needs (where the composer sits over the keyboard, how far a focused
//! field must scroll to clear it) is in [`composer_bottom`] and [`reveal_above`].

use super::dom::{Control, Edit, El};
use super::style::{Sheet, StyleError};

mod list;
pub use list::{search_bar, search_filter, search_rank, VirtualList};

/// The mobile look. Variables on `:root` theme it (`--m-bg`, `--m-bar`, `--m-card`,
/// `--m-txt`, `--m-sub`, `--m-dim`, `--m-line`, `--m-glass`, `--m-solid`, the dot colours).
/// Safe-area padding comes from `env(safe-area-inset-*)`, so re-parse the sheet when
/// [`super::media::generation`] moves.
pub const MOBILE_CSS: &str = r#"
:root { --m-bg: #121212; --m-bar: #161616; --m-card: #1c1c1c; --m-press: #262626; --m-line: #262626; --m-line2: #2e2e2e;
        --m-txt: #f2f2f2; --m-sub: #9a9a9a; --m-dim: #6c6c6c; --m-mute: #4a4a4a; --m-ok: #5fd08a; --m-warn: #d8b45a; --m-err: #d07070;
        --m-strong: #ececec; --m-on-strong: #111111; --m-glass: rgba(30,30,32,0.72); --m-solid: rgb(30,30,32); --m-glass-line: rgba(255,255,255,0.10);
        --m-scrim: rgba(0,0,0,0.45); --m-hi: rgba(255,255,255,0.10) }
div.m-screen { display: flex; flex-direction: column; background: var(--m-bg); color: var(--m-txt); font-size: 15px; font-family: Inter, sans-serif; overflow: hidden }
div.m-safe-top { height: env(safe-area-inset-top, 0px); flex-shrink: 0 }
div.m-content { flex-grow: 1; min-height: 0; display: flex; flex-direction: column; overflow: hidden }
div.m-top { flex-shrink: 0; padding: 4px 16px 10px; background: var(--m-bg); display: flex; flex-direction: column }
div.m-top-row { display: flex; align-items: center; gap: 10px; height: 44px }
div.m-h1 { flex-grow: 1; font-size: 30px; font-weight: 700 }
div.m-search { display: flex; align-items: center; gap: 8px; height: 38px; margin-top: 6px; padding: 0 12px; border-radius: 10px; background: var(--m-card); border: 1px solid var(--m-line); color: var(--m-dim) }
div.m-search > span.m-ic { width: 18px; height: 18px }
div.m-search > div.m-ph { flex-grow: 1 }
div.m-search > div.m-field { flex-grow: 1; height: 24px }
button.m-ibtn { width: 44px; height: 44px; padding: 11px; border-radius: 22px; color: var(--m-txt); flex-shrink: 0 }
button.m-ibtn.fill { width: 36px; height: 36px; padding: 8px; border-radius: 18px; background: var(--m-card); border: 1px solid var(--m-line2) }
button.m-ibtn.sub { color: var(--m-sub) }
div.m-nav { flex-shrink: 0; display: flex; align-items: center; height: 48px; padding: 0 8px; background: var(--m-bar); border-bottom: 1px solid var(--m-line) }
button.m-back { display: flex; align-items: center; gap: 2px; min-width: 92px; height: 44px; padding: 0 6px; color: var(--m-txt); font-size: 16px }
button.m-back > span.m-ic { width: 22px; height: 22px; flex-shrink: 0 }
div.m-ttl { flex-grow: 1; min-width: 0; display: flex; flex-direction: column; align-items: center }
div.m-ttl > div.m-ttl-main { font-size: 16px; font-weight: 600; max-width: 100% }
div.m-ttl > div.m-ttl-sub { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--m-sub) }
div.m-nav-end { min-width: 92px; display: flex; justify-content: flex-end; gap: 2px }
span.m-dot { width: 8px; height: 8px; border-radius: 4px; background: var(--m-mute); flex-shrink: 0 }
span.m-dot.ok { background: var(--m-ok) }
span.m-dot.warn { background: var(--m-warn) }
span.m-dot.err { background: var(--m-err) }
span.m-dot.sm { width: 7px; height: 7px; border-radius: 4px }
span.m-ic { width: 24px; height: 24px; flex-shrink: 0 }
div.m-tabs { flex-shrink: 0; display: flex; background: var(--m-bar); border-top: 1px solid var(--m-line); padding: 6px 6px env(safe-area-inset-bottom, 0px) }
button.m-tab { flex-grow: 1; flex-basis: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 3px; min-height: 49px; color: var(--m-dim); font-size: 11px; font-weight: 500; position: relative }
button.m-tab.on { color: var(--m-txt) }
button.m-tab > span.m-ic { width: 25px; height: 25px }
span.m-tbadge { position: absolute; top: 2px; left: 52%; min-width: 17px; height: 17px; padding: 0 5px; border-radius: 9px; background: var(--m-strong); color: var(--m-on-strong); font-size: 11px; font-weight: 700; text-align: center; border: 2px solid var(--m-bar) }
button.m-tab > span.m-dot { position: absolute; top: 6px; left: 58% }
div.m-sec { font-size: 13px; color: var(--m-dim); font-weight: 600; padding: 16px 20px 8px; flex-shrink: 0 }
div.m-list { display: flex; flex-direction: column; background: var(--m-bg) }
div.m-row { display: flex; align-items: center; gap: 12px; min-height: 68px; padding: 10px 16px; background: var(--m-bg); border-bottom: 1px solid var(--m-line); flex-shrink: 0; position: relative }
div.m-row.dense { min-height: 52px }
div.m-row:active { background: var(--m-press) }
div.m-row.on { background: var(--m-press) }
div.m-av { width: 44px; height: 44px; border-radius: 22px; background: #2a2a2a; flex-shrink: 0; display: flex; align-items: center; justify-content: center; text-align: center; font-weight: 600; font-size: 16px; color: #d8d8d8; position: relative }
div.m-av.sq { border-radius: 12px }
div.m-av > span.m-dot { position: absolute; right: -1px; bottom: -1px; width: 12px; height: 12px; border-radius: 6px; border: 2px solid var(--m-bg) }
div.m-main { flex-grow: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px }
div.m-l1 { display: flex; align-items: center; gap: 8px }
div.m-l1 > div.m-t { flex-grow: 1; min-width: 0; font-weight: 600; font-size: 16px }
div.m-l1 > div.m-time { font-size: 13px; color: var(--m-dim); flex-shrink: 0 }
div.m-l2 { display: flex; align-items: center; gap: 8px; color: var(--m-sub); font-size: 14px }
div.m-l2 > div.m-s { flex-grow: 1; min-width: 0 }
span.m-cnt { min-width: 20px; height: 20px; padding: 0 6px; border-radius: 10px; background: var(--m-strong); color: var(--m-on-strong); font-size: 12px; font-weight: 700; text-align: center; flex-shrink: 0 }
span.m-cnt.muted { background: #3a3a3a; color: #cfcfcf }
span.m-chev { width: 16px; height: 16px; color: var(--m-mute); flex-shrink: 0 }
div.m-swipe { position: relative; overflow: hidden; flex-shrink: 0 }
div.m-acts { position: absolute; top: 0; bottom: 0; display: flex }
div.m-acts.r { right: 0 }
div.m-acts.l { left: 0 }
button.m-sact { width: 78px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 4px; font-size: 12px; font-weight: 500; color: var(--m-txt); background: #2a2a2a; border-left: 1px solid #333333 }
button.m-sact.alt { background: #333333 }
button.m-sact.strong { background: #e6e6e6; color: var(--m-on-strong) }
button.m-sact > span.m-ic { width: 22px; height: 22px }
button.m-btn { display: flex; align-items: center; justify-content: center; gap: 6px; height: 44px; padding: 0 18px; border-radius: 12px; background: #2a2a2a; border: 1px solid #333333; color: var(--m-txt); font-size: 15px; font-weight: 500 }
button.m-btn.pri { background: var(--m-strong); border-color: var(--m-strong); color: var(--m-on-strong); font-weight: 600 }
button.m-btn.ghost { background: transparent; border-color: transparent; color: var(--m-sub) }
button.m-btn.block { flex-grow: 1 }
div.m-seg { display: flex; height: 38px; padding: 3px; border-radius: 10px; background: var(--m-card); border: 1px solid var(--m-line) }
button.m-seg-b { flex-grow: 1; flex-basis: 0; display: flex; align-items: center; justify-content: center; gap: 6px; border-radius: 8px; font-size: 14px; color: var(--m-sub); font-weight: 500 }
button.m-seg-b.on { background: #2e2e2e; color: var(--m-txt); box-shadow: 0 1px 2px rgba(0,0,0,0.4) }
span.m-seg-n { font-size: 12px; color: var(--m-dim) }
div.m-glass { background: var(--m-glass); border: 1px solid var(--m-glass-line); backdrop-filter: blur(28px); box-shadow: 0 22px 64px rgba(0,0,0,0.5) }
div.m-solid { background: var(--m-solid); border: 1px solid var(--m-glass-line); box-shadow: 0 22px 64px rgba(0,0,0,0.5) }
div.m-scrim { position: fixed; top: 0; left: 0; right: 0; bottom: 0; z-index: 50; background: var(--m-scrim) }
div.m-sheet { position: fixed; left: 0; right: 0; z-index: 60; border-radius: 22px 22px 0 0; padding: 8px 0 env(safe-area-inset-bottom, 0px); display: flex; flex-direction: column; overflow: hidden }
div.m-grab { height: 23px; flex-shrink: 0; display: flex; justify-content: center; padding-top: 0 }
div.m-grab > div.m-grab-bar { width: 38px; height: 5px; border-radius: 3px; background: rgba(255,255,255,0.28) }
div.m-sh { display: flex; align-items: center; gap: 12px; padding: 2px 20px 12px; flex-shrink: 0 }
div.m-sh > div.m-sh-t { flex-grow: 1; font-size: 18px; font-weight: 600 }
div.m-sheet-body { flex-grow: 1; min-height: 0; display: flex; flex-direction: column; overflow: hidden }
div.m-srow { display: flex; align-items: center; gap: 14px; min-height: 52px; padding: 0 20px; font-size: 16px; border-top: 1px solid rgba(255,255,255,0.06); flex-shrink: 0 }
div.m-srow:active { background: var(--m-hi) }
div.m-srow > span.m-ic { width: 22px; height: 22px; color: var(--m-sub) }
div.m-srow > div.m-srow-l { flex-grow: 1 }
div.m-srow > div.m-srow-v { font-size: 13px; color: var(--m-sub) }
div.m-tile { background: var(--m-card); border: 1px solid var(--m-line2); border-radius: 14px; display: flex; flex-direction: column }
div.m-sheet div.m-tile { background: rgba(255,255,255,0.05); border-color: rgba(255,255,255,0.07) }
div.m-toast { position: fixed; left: 16px; right: 16px; z-index: 70; border-radius: 14px; display: flex; align-items: center; gap: 12px; padding: 12px 8px 12px 16px; font-size: 15px }
div.m-toast > div.m-toast-tx { flex-grow: 1; white-space: normal }
button.m-toast-act { font-weight: 600; padding: 8px 10px; border-radius: 8px; color: var(--m-txt) }
button.m-fab { position: fixed; right: 18px; z-index: 65; width: 58px; height: 58px; padding: 17px; border-radius: 29px; background: var(--m-strong); color: var(--m-on-strong); box-shadow: 0 10px 26px rgba(0,0,0,0.55) }
div.m-drawer { position: fixed; top: 0; bottom: 0; z-index: 60; width: 300px; border-radius: 0 22px 22px 0; padding: env(safe-area-inset-top, 0px) 12px env(safe-area-inset-bottom, 0px); display: flex; flex-direction: column }
div.m-dhead { display: flex; align-items: center; gap: 12px; padding: 14px 12px 10px; flex-shrink: 0 }
div.m-dhead > div.m-dhead-t { display: flex; flex-direction: column; flex-grow: 1; min-width: 0 }
div.m-dhead div.m-dname { font-size: 17px; font-weight: 600 }
div.m-dhead div.m-dsub { display: flex; align-items: center; gap: 6px; font-size: 13px; color: var(--m-sub) }
div.m-ditem { display: flex; align-items: center; gap: 14px; height: 48px; padding: 0 12px; border-radius: 12px; font-size: 16px; color: #d6d6d6; flex-shrink: 0 }
div.m-ditem.on { background: var(--m-hi); color: var(--m-txt); font-weight: 600 }
div.m-ditem > span.m-ic { width: 22px; height: 22px }
div.m-ditem > div.m-dl { flex-grow: 1; min-width: 0 }
div.m-ditem > div.m-dn { font-size: 13px; color: var(--m-sub) }
div.m-dsec { font-size: 12px; color: rgba(255,255,255,0.45); font-weight: 600; padding: 14px 12px 6px; flex-shrink: 0 }
div.m-dfill { flex-grow: 1 }
div.m-rail { position: fixed; right: 0; top: env(safe-area-inset-top, 0px); bottom: 0; width: 64px; z-index: 10; background: var(--m-bar); border-left: 1px solid var(--m-line); display: flex; flex-direction: column; align-items: center; gap: 6px; padding: 10px 0 env(safe-area-inset-bottom, 0px) }
button.m-ritem { width: 48px; height: 48px; padding: 12px; border-radius: 14px; color: var(--m-dim); position: relative; flex-shrink: 0 }
button.m-ritem.on { background: #262626; color: var(--m-txt) }
button.m-ritem > span.m-dot { position: absolute; top: 9px; right: 9px }
div.m-composer { flex-shrink: 0; display: flex; align-items: flex-end; gap: 6px; padding: 8px 10px; background: var(--m-bar); border-top: 1px solid var(--m-line) }
div.m-composer > button.m-ibtn { width: 40px; height: 40px; padding: 8px; color: var(--m-sub) }
div.m-inp { flex-grow: 1; min-width: 0; min-height: 40px; padding: 9px 14px; border-radius: 20px; background: var(--m-card); border: 1px solid var(--m-line2); font-size: 16px;
            --placeholder: var(--m-dim); --caret: var(--m-txt); --selection: rgba(255,255,255,0.25) }
button.m-send { width: 40px; height: 40px; padding: 10px; border-radius: 20px; background: var(--m-strong); color: var(--m-on-strong); flex-shrink: 0 }
button.m-send.off { background: #2a2a2a; color: var(--m-dim) }
div.m-hero { display: flex; flex-direction: column; align-items: center; padding: 12px 20px 18px; flex-shrink: 0 }
div.m-critter { width: 96px; height: 96px; border-radius: 30px; background: #262626; border: 1px solid #333333; position: relative }
div.m-critter > span.m-dot { position: absolute; right: -2px; bottom: -2px; width: 16px; height: 16px; border-radius: 8px; border: 3px solid var(--m-bg) }
div.m-h2 { font-size: 22px; font-weight: 700; margin-top: 12px }
div.m-st { display: flex; align-items: center; gap: 7px; color: var(--m-sub); font-size: 14px; margin-top: 3px }
div.m-qa { display: flex; gap: 10px; padding: 0 16px 14px; flex-shrink: 0 }
button.m-q { flex-grow: 1; flex-basis: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 6px; height: 68px; border-radius: 14px; background: var(--m-card); border: 1px solid var(--m-line2); font-size: 12px; color: var(--m-txt); font-weight: 500 }
button.m-q > span.m-ic { width: 22px; height: 22px }
div.m-kv { margin: 0 16px 12px; flex-shrink: 0 }
div.m-kvr { display: flex; align-items: center; min-height: 48px; padding: 0 14px; gap: 12px; font-size: 15px; border-top: 1px solid var(--m-line) }
div.m-sbar { display: flex; align-items: center; gap: 10px; padding: 8px 16px; flex-shrink: 0; background: var(--m-bg) }
div.m-sbar > div.m-sfield { flex-grow: 1; min-width: 0; margin-top: 0 }
input.m-sinput { flex-grow: 1; min-width: 0; height: 24px; padding: 0; border-width: 0; background: transparent; color: var(--m-txt); font-size: 16px }
button.m-sclear { width: 26px; height: 26px; padding: 5px; border-radius: 13px; flex-shrink: 0; color: var(--m-sub); background: var(--m-press) }
button.m-scancel { height: 38px; padding: 0 2px; flex-shrink: 0; color: var(--m-txt); font-size: 16px }
div.m-vlist { flex-grow: 1; min-height: 0; display: flex; flex-direction: column; overflow: hidden; background: var(--m-bg) }
div.m-vbody { flex-shrink: 0; display: flex; flex-direction: column }
div.m-vpad { flex-shrink: 0 }
div.m-vrow { flex-shrink: 0; display: flex; flex-direction: column; overflow: hidden }
div.m-vrow > div.m-row, div.m-vrow > div.m-swipe { flex-grow: 1; flex-shrink: 1; min-height: 0 }
div.m-empty { padding: 48px 24px; text-align: center; color: var(--m-sub); font-size: 15px; white-space: normal }
div.m-kvr.first { border-top: 0 }
div.m-kvr > div.m-k { flex-grow: 1; color: var(--m-sub); min-width: 0 }
div.m-kvr > div.m-v { color: var(--m-txt); text-align: right; min-width: 0 }
div.m-kvr > div.m-v.mono { font-family: monospace; font-size: 13px }
button.m-toggle { width: 51px; height: 31px; padding: 2px; border-radius: 16px; background: #3a3a3a; --knob: #bdbdbd; flex-shrink: 0 }
button.m-toggle:checked { background: #e6e6e6; --knob: #111111 }
@media (hover: hover) {
div.m-row:hover { background: #181818 }
button.m-tab:hover { color: var(--m-sub) }
button.m-tab.on:hover { color: var(--m-txt) }
div.m-ditem:hover { background: rgba(255,255,255,0.06) }
div.m-srow:hover { background: rgba(255,255,255,0.05) }
button.m-ibtn:hover { background: var(--m-press) }
button.m-q:hover { background: #222222 }
}
div.m-screen div.k-msg.me { background: #2c2c2c; color: var(--m-txt); border: 1px solid #363636 }
div.m-screen div.k-msg.them { background: var(--m-card); color: var(--m-txt); border: 1px solid var(--m-line) }
div.m-screen div.k-msg > div.k-who { color: var(--m-sub) }
div.m-screen div.k-msg div.k-time { color: var(--m-dim) }
div.m-screen div.k-msg.me div.k-body a.link { color: var(--m-txt) }
"#;

/// [`MOBILE_CSS`] after the desktop kit ([`super::domkit::KIT_CSS`], touch sizes) and
/// before the app's own rules, against the current media environment.
pub fn sheet_with(app_css: &str) -> Result<Sheet, StyleError> {
    Sheet::parse(&[super::domkit::KIT_CSS, super::domkit::KIT_TOUCH_CSS, MOBILE_CSS, app_css].concat())
}

/// A 24×24 stroke icon at the mobile weight (1.7) from its path markup.
pub fn icon(body: &str) -> String {
    format!(r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">{body}</svg>"#)
}

fn ic(body: &str) -> El {
    El::new("span.m-ic").svg(icon(body))
}

/// The mobile icon set (path bodies for [`icon`]).
pub mod icons {
    pub const CHAT: &str = r#"<path d="M4 5h16v11H9l-5 4z"/>"#;
    pub const TASKS: &str = r#"<path d="M4 6l2 2 3-3M4 13l2 2 3-3M12 7h8M12 14h8M4 20h16"/>"#;
    pub const MACHINE: &str = r#"<rect x="3" y="4" width="18" height="6" rx="2"/><rect x="3" y="14" width="18" height="6" rx="2"/><path d="M7 7h.01M7 17h.01"/>"#;
    pub const VAULT: &str = r#"<rect x="4" y="10" width="16" height="10" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0v3M12 14v2"/>"#;
    pub const GEAR: &str = r#"<circle cx="12" cy="12" r="3"/><path d="M12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.3 5.3l2.1 2.1M16.6 16.6l2.1 2.1M5.3 18.7l2.1-2.1M16.6 7.4l2.1-2.1"/>"#;
    pub const SEARCH: &str = r#"<circle cx="11" cy="11" r="6"/><path d="M20 20l-4.5-4.5"/>"#;
    pub const PLUS: &str = r#"<path d="M12 5v14M5 12h14"/>"#;
    pub const BACK: &str = r#"<path d="M15 4l-8 8 8 8"/>"#;
    pub const MORE: &str = r#"<circle cx="5" cy="12" r="1.3"/><circle cx="12" cy="12" r="1.3"/><circle cx="19" cy="12" r="1.3"/>"#;
    pub const PIN: &str = r#"<path d="M9 4h6l-1 6 3 3H7l3-3zM12 13v7"/>"#;
    pub const ARCHIVE: &str = r#"<rect x="3" y="4" width="18" height="5" rx="1"/><path d="M5 9v11h14V9M10 13h4"/>"#;
    pub const TRASH: &str = r#"<path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13"/>"#;
    pub const BELL: &str = r#"<path d="M6 16V11a6 6 0 0 1 12 0v5l2 2H4zM10 20a2 2 0 0 0 4 0"/>"#;
    pub const BELL_OFF: &str = r#"<path d="M6 16V11a6 6 0 0 1 9-5.2M18 11v5l2 2H8M10 20a2 2 0 0 0 4 0M3 3l18 18"/>"#;
    pub const SEND: &str = r#"<path d="M12 19V5M6 11l6-6 6 6"/>"#;
    pub const MIC: &str = r#"<rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3"/>"#;
    pub const MENU: &str = r#"<path d="M4 7h16M4 12h16M4 17h16"/>"#;
    pub const CHECK: &str = r#"<path d="M5 12l5 5 9-10"/>"#;
    pub const READ: &str = r#"<path d="M2 12l4 4 8-9M10 16l1 1 8-9"/>"#;
    pub const EDIT: &str = r#"<path d="M4 20h4l11-11-4-4L4 16z"/>"#;
    pub const STOP: &str = r#"<rect x="6" y="6" width="12" height="12" rx="2"/>"#;
    pub const TERM: &str = r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 9l3 3-3 3M12 15h5"/>"#;
    pub const KEY: &str = r#"<circle cx="8" cy="15" r="4"/><path d="M11 12l9-9M16 7l3 3"/>"#;
    pub const USER: &str = r#"<circle cx="12" cy="8" r="4"/><path d="M4 21a8 8 0 0 1 16 0"/>"#;
    pub const CLOSE: &str = r#"<path d="M6 6l12 12M18 6L6 18"/>"#;
    pub const CHEV: &str = r#"<path d="M9 5l7 7-7 7"/>"#;
    pub const SPARK: &str = r#"<path d="M12 3v4M12 17v4M3 12h4M17 12h4M6 6l2.5 2.5M15.5 15.5L18 18M6 18l2.5-2.5M15.5 8.5L18 6"/>"#;
    pub const GLOBE: &str = r#"<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18"/>"#;
    pub const COPY: &str = r#"<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3"/>"#;
    pub const REPLY: &str = r#"<path d="M10 8L4 13l6 5M4 13h10a6 6 0 0 1 6 6"/>"#;
}

/// A small status dot: `"ok"`, `"warn"`, `"err"`, anything else is the muted grey.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Dot {
    #[default]
    None,
    Idle,
    Ok,
    Warn,
    Err,
}

impl Dot {
    fn class(self) -> &'static str {
        match self {
            Dot::Ok => "span.m-dot.ok",
            Dot::Warn => "span.m-dot.warn",
            Dot::Err => "span.m-dot.err",
            _ => "span.m-dot",
        }
    }
    fn el(self) -> Option<El> {
        (self != Dot::None).then(|| El::new(self.class()))
    }
}

fn with_dot(e: El, d: Dot) -> El {
    match d.el() {
        Some(x) => e.child(x),
        None => e,
    }
}

/// The whole phone screen: a column with the top safe area padded (`div.m-safe-top`)
/// when `pad_top`, then `body` in order.
pub fn screen(pad_top: bool, body: impl IntoIterator<Item = El>) -> El {
    El::new("div.m-screen").child_if(pad_top, || El::new("div.m-safe-top")).children(body)
}

/// A screen's large title with round buttons on its right (`(key, icon, name)`) and,
/// with `search`, the search pill under it: `Some(placeholder)` shows the hint
/// (`{id}:search` is its hit); a host field goes in the `@{id}:search` slot when
/// `search_slot` is set instead.
pub fn large_title(id: &str, title: &str, actions: &[(&str, &str, &str)], search: Option<&str>, search_slot: bool) -> El {
    let row = El::new("div.m-top-row").child(El::new("div.m-h1").text(title)).children(actions.iter().map(|(k, svg, name)| {
        El::new("button.m-ibtn.fill").id(format!("{id}:{k}")).tab().name(*name).svg(icon(svg))
    }));
    let s = search.map(|ph| {
        let pill = El::new("div.m-search").id(format!("{id}:search")).name(ph).child(ic(icons::SEARCH));
        if search_slot {
            pill.child(El::new("div.m-field").slot(format!("@{id}:search")))
        } else {
            pill.child(El::new("div.m-ph").text(ph))
        }
    });
    let top = El::new("div.m-top").child(row);
    match s {
        Some(s) => top.child(s),
        None => top,
    }
}

/// The top bar of a pushed screen: `‹ back` (`{id}:back`), the title with an optional
/// status line (a dot and its text), and round buttons on the right (`{id}:{key}`).
pub fn top_bar(id: &str, back: &str, title: &str, sub: Option<(Dot, &str)>, actions: &[(&str, &str, &str)]) -> El {
    let ttl = El::new("div.m-ttl").child(El::new("div.m-ttl-main").text(title)).child_if(sub.is_some(), || {
        let (d, t) = sub.unwrap_or_default();
        with_dot(El::new("div.m-ttl-sub"), if d == Dot::None { Dot::None } else { d }).child(El::new("span").text(t))
    });
    El::new("div.m-nav")
        .child(El::new("button.m-back").id(format!("{id}:back")).tab().name(back).child(ic(icons::BACK)).child(El::new("span").text(back)))
        .child(ttl)
        .child(El::new("div.m-nav-end").children(actions.iter().map(|(k, svg, name)| El::new("button.m-ibtn").id(format!("{id}:{k}")).tab().name(*name).svg(icon(svg)))))
}

/// One bottom tab.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tab {
    pub icon: &'static str,
    pub label: String,
    /// An unread count (monochrome badge, `99+` past 99); 0 = none.
    pub badge: u32,
    /// A status dot (only colour on the bar).
    pub dot: Dot,
}

impl Tab {
    pub fn new(icon: &'static str, label: impl Into<String>) -> Self {
        Tab { icon, label: label.into(), ..Default::default() }
    }
    pub fn badge(mut self, n: u32) -> Self {
        self.badge = n;
        self
    }
    pub fn dot(mut self, d: Dot) -> Self {
        self.dot = d;
        self
    }
}

fn count(n: u32) -> String {
    if n > 99 { "99+".into() } else { n.to_string() }
}

/// The bottom tab bar (opaque), padded by the bottom safe area: `{id}:{i}` per tab.
pub fn tab_bar(id: &str, tabs: &[Tab], active: usize) -> El {
    El::new("div.m-tabs").children(tabs.iter().enumerate().map(|(i, t)| {
        let b = El::new(if i == active { "button.m-tab.on" } else { "button.m-tab" })
            .id(format!("{id}:{i}"))
            .tab()
            .name(t.label.clone())
            .child(ic(t.icon))
            .child(El::new("div").text(t.label.clone()));
        if t.badge > 0 {
            b.child(El::new("span.m-tbadge").text(count(t.badge)))
        } else {
            with_dot(b, t.dot)
        }
    }))
}

/// One list row (chats, tasks, machines).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    /// The avatar: a letter, or empty for a host slot `@{id}:av` (a critter, a picture).
    pub avatar: String,
    /// Square avatar (machines, spaces).
    pub square: bool,
    /// Status dot on the avatar.
    pub dot: Dot,
    pub title: String,
    pub time: String,
    pub sub: String,
    /// Unread count (0 = none); `muted` greys it.
    pub count: u32,
    pub muted: bool,
    /// A trailing icon when there is no count (a pin), or a chevron for drill-down rows.
    pub trail: Option<&'static str>,
    /// One-line dense row (52px, no avatar when `avatar` is `"-"`).
    pub dense: bool,
    pub on: bool,
}

impl Row {
    pub fn new(title: impl Into<String>) -> Self {
        Row { title: title.into(), ..Default::default() }
    }
}

/// A list row `id`. The avatar is the first letter given, or the slot `@{id}:av`.
pub fn row(id: &str, r: &Row) -> El {
    let spec = match (r.dense, r.on) {
        (true, true) => "div.m-row.dense.on",
        (true, false) => "div.m-row.dense",
        (false, true) => "div.m-row.on",
        (false, false) => "div.m-row",
    };
    let av = if r.avatar == "-" {
        None
    } else {
        let a = El::new(if r.square { "div.m-av.sq" } else { "div.m-av" });
        let a = if r.avatar.is_empty() { a.slot(format!("@{id}:av")) } else { a.text(r.avatar.clone()) };
        Some(with_dot(a, r.dot))
    };
    let l1 = El::new("div.m-l1").child(El::new("div.m-t").text(r.title.clone())).child_if(!r.time.is_empty(), || El::new("div.m-time").text(r.time.clone()));
    let mut l2 = El::new("div.m-l2").child(El::new("div.m-s").text(r.sub.clone()));
    if r.count > 0 {
        l2 = l2.child(El::new(if r.muted { "span.m-cnt.muted" } else { "span.m-cnt" }).text(count(r.count)));
    } else if let Some(t) = r.trail.filter(|_| !r.dense) {
        l2 = l2.child(El::new("span.m-chev").svg(icon(t)));
    }
    let main = El::new("div.m-main").child(l1).child_if(!r.sub.is_empty() || r.count > 0, || l2);
    let mut e = El::new(spec).id(id).name(r.title.clone());
    if let Some(a) = av {
        e = e.child(a);
    }
    e = e.child(main);
    if r.dense {
        if let Some(t) = r.trail {
            e = e.child(El::new("span.m-chev").svg(icon(t)));
        }
    }
    e
}

/// A section label over a list (`ЗАКРЕПЛЁННЫЕ`, `УЗЛЫ`).
pub fn section(label: &str) -> El {
    El::new("div.m-sec").text(label)
}

/// One swipe action: `(key, icon, label, look)` with look `""`, `"alt"`, `"strong"`.
pub type SwipeAct<'a> = (&'a str, &'a str, &'a str, &'a str);

/// Width of one swipe action button (`button.m-sact`).
pub const SWIPE_ACT_W: f64 = 78.0;

/// A row with swipe actions behind it: `left` show when the row moves right, `right`
/// when it moves left. `state.offset` moves the row (negative = left). Hits are
/// `{id}:act:{key}`; the row keeps its own id. Only the side being revealed is built.
pub fn swipe_row(id: &str, row_el: El, left: &[SwipeAct<'_>], right: &[SwipeAct<'_>], state: &SwipeState) -> El {
    let off = state.offset;
    let acts = |side: &str, list: &[SwipeAct<'_>]| {
        El::new(if side == "l" { "div.m-acts.l" } else { "div.m-acts.r" }).children(list.iter().map(|(k, svg, label, look)| {
            let spec = match *look {
                "alt" => "button.m-sact.alt",
                "strong" => "button.m-sact.strong",
                _ => "button.m-sact",
            };
            El::new(spec).id(format!("{id}:act:{k}")).tab().name(*label).child(ic(svg)).child(El::new("div").text(*label))
        }))
    };
    let shown_r = off < -0.5;
    let shown_l = off > 0.5;
    El::new("div.m-swipe")
        .child_if(shown_r, || acts("r", right))
        .child_if(shown_l, || acts("l", left))
        .child(row_el.css(format!("left: {}px; z-index: 1", fmt(off))))
}

fn fmt(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r.fract() == 0.0 { format!("{}", r as i64) } else { format!("{r}") }
}

/// One drawer entry.
#[derive(Clone, Debug, PartialEq)]
pub enum DrawerItem {
    /// `(key, icon, label, count text, on)`.
    Item(String, &'static str, String, String, bool),
    /// A section label.
    Section(String),
    /// Pushes the following items to the bottom.
    Fill,
}

/// The edge drawer (glass) over a scrim: `progress` 0 (closed, nothing built) … 1 (open),
/// `head` an optional account header (`(avatar letter, name, dot, sub)`).
/// Hits `{id}:scrim`, `{id}:{key}`, `{id}:head`.
pub fn drawer(id: &str, progress: f64, head: Option<(&str, &str, Dot, &str)>, items: &[DrawerItem]) -> Option<El> {
    if progress <= 0.001 {
        return None;
    }
    let p = progress.clamp(0.0, 1.0);
    let w = DRAWER_W;
    let mut d = El::new("div.m-drawer.m-glass").css(format!("left: {}px", fmt(-(1.0 - p) * w)));
    if let Some((a, name, dot, sub)) = head {
        d = d.child(
            El::new("div.m-dhead")
                .id(format!("{id}:head"))
                .child(El::new("div.m-av.sq").text(a))
                .child(El::new("div.m-dhead-t").child(El::new("div.m-dname").text(name)).child(with_dot(El::new("div.m-dsub"), dot).child(El::new("span").text(sub)))),
        );
    }
    for it in items {
        d = d.child(match it {
            DrawerItem::Item(k, svg, label, n, on) => El::new(if *on { "div.m-ditem.on" } else { "div.m-ditem" })
                .id(format!("{id}:{k}"))
                .tab()
                .name(label.clone())
                .child(ic(svg))
                .child(El::new("div.m-dl").text(label.clone()))
                .child_if(!n.is_empty(), || El::new("div.m-dn").text(n.clone())),
            DrawerItem::Section(s) => El::new("div.m-dsec").text(s.clone()),
            DrawerItem::Fill => El::new("div.m-dfill"),
        });
    }
    Some(El::new("div").child(El::new("div.m-scrim").id(format!("{id}:scrim")).css(format!("opacity: {}", fmt(p)))).child(d))
}

/// The drawer's width (`div.m-drawer`).
pub const DRAWER_W: f64 = 300.0;

/// A right side rail of icon buttons (opaque): `(icon, name, dot)`; `{id}:{i}`.
pub fn side_rail(id: &str, items: &[(&'static str, &str, Dot)], active: usize) -> El {
    El::new("div.m-rail").children(items.iter().enumerate().map(|(i, (svg, name, dot))| {
        with_dot(El::new(if i == active { "button.m-ritem.on" } else { "button.m-ritem" }).id(format!("{id}:{i}")).tab().name(*name).svg(icon(svg)), *dot)
    }))
}

/// The floating action button, `bottom` px above the viewport's bottom edge (over the
/// tab bar: tab bar height + 16).
pub fn fab(id: &str, svg: &str, name: &str, bottom: f64) -> El {
    El::new("button.m-fab").id(id).tab().name(name).svg(icon(svg)).css(format!("bottom: {}px", fmt(bottom)))
}

/// One action of the action modal: `(key, icon, label, hint)`.
pub type Action<'a> = (&'a str, &'a str, &'a str, &'a str);

/// The FAB's action modal: an opaque sheet from the bottom over a scrim, with a title,
/// a close button and one row per action. Opaque so content under it (a bottom sheet)
/// does not show through. Hits `{id}:scrim`, `{id}:close`, `{id}:{key}`.
pub fn action_modal(id: &str, title: &str, close: &str, actions: &[Action<'_>]) -> El {
    let sheet = El::new("div.m-sheet.m-solid")
        .css("bottom: 0")
        .child(El::new("div.m-grab").child(El::new("div.m-grab-bar")))
        .child(El::new("div.m-sh").child(El::new("div.m-sh-t").text(title)).child(El::new("button.m-ibtn.fill").id(format!("{id}:close")).tab().name(close).svg(icon(icons::CLOSE))))
        .children(actions.iter().map(|(k, svg, label, hint)| {
            El::new("div.m-srow")
                .id(format!("{id}:{k}"))
                .tab()
                .name(*label)
                .child(ic(svg))
                .child(El::new("div.m-srow-l").text(*label))
                .child_if(!hint.is_empty(), || El::new("div.m-srow-v").text(*hint))
        }));
    El::new("div").child(El::new("div.m-scrim").id(format!("{id}:scrim"))).child(sheet)
}

/// The bottom sheet at its current height ([`SheetState::height`]) in a viewport of
/// height `vh`: the grab handle (`{id}:grab`, drag it), then `head` and `body`. Over
/// half height a scrim (`{id}:scrim`) dims the screen. `None` when closed.
pub fn bottom_sheet(id: &str, st: &SheetState, vh: f64, head: Option<El>, body: El) -> Option<El> {
    let h = st.height(vh);
    if h <= 0.5 {
        return None;
    }
    let half = SheetState::detent_height(Detent::Half, vh, st.peek);
    let dim = ((h - st.peek) / (half - st.peek).max(1.0)).clamp(0.0, 1.0);
    let sheet = El::new("div.m-sheet.m-glass")
        .css(format!("top: {}px; height: {}px", fmt(vh - h), fmt(h + 1.0)))
        .child(El::new("div.m-grab").id(format!("{id}:grab")).child(El::new("div.m-grab-bar")))
        .child_if(head.is_some(), || head.unwrap_or_else(|| El::new("div")))
        .child(El::new("div.m-sheet-body").child(body));
    Some(El::new("div").child_if(dim > 0.01, || El::new("div.m-scrim").id(format!("{id}:scrim")).css(format!("opacity: {}", fmt(dim)))).child(sheet))
}

/// A toast `bottom` px above the viewport bottom (glass), with an optional action
/// (`{id}:act`).
pub fn toast(id: &str, text: &str, action: Option<&str>, bottom: f64) -> El {
    El::new("div.m-toast.m-glass")
        .id(id)
        .css(format!("bottom: {}px", fmt(bottom)))
        .child(El::new("div.m-toast-tx").text(text))
        .child_if(action.is_some(), || El::new("button.m-toast-act").id(format!("{id}:act")).tab().text(action.unwrap_or_default()))
}

/// A segmented control: `(label, count)` (count `None` = no number); `{id}:{i}`.
pub fn segmented(id: &str, items: &[(&str, Option<u32>)], active: usize) -> El {
    El::new("div.m-seg").children(items.iter().enumerate().map(|(i, (label, n))| {
        El::new(if i == active { "button.m-seg-b.on" } else { "button.m-seg-b" })
            .id(format!("{id}:{i}"))
            .tab()
            .name(*label)
            .child(El::new("span").text(*label))
            .child_if(n.is_some(), || El::new("span.m-seg-n").text(n.unwrap_or(0).to_string()))
    }))
}

/// What fills the composer's field.
#[derive(Clone, Debug, PartialEq)]
pub enum ComposerField {
    /// The DOM's own single-line input (`Control::Input`).
    Input(Edit),
    /// An embed slot `@{id}:field` of this height: the host builds its multi-line text
    /// area there (`uzor_framework::widgets::kit::area`).
    Slot(f64),
}

/// The mobile composer: `+` (`{id}:plus`), the field (`{id}:field`), and send — an
/// arrow when there is text (`has_text`), else a mic (`.off`) — (`{id}:send`). The host
/// places it at [`composer_bottom`] so it rides above the soft keyboard.
pub fn composer(id: &str, field: ComposerField, has_text: bool, names: (&str, &str, &str)) -> El {
    let f = El::new("div.m-inp").id(format!("{id}:field")).name(names.1);
    let f = match field {
        ComposerField::Input(e) => f.control(Control::input(e)),
        ComposerField::Slot(h) => f.child(El::new("div").slot(format!("@{id}:field")).css(format!("height: {}px", fmt(h)))),
    };
    El::new("div.m-composer")
        .child(El::new("button.m-ibtn").id(format!("{id}:plus")).tab().name(names.0).svg(icon(icons::PLUS)))
        .child(f)
        .child(El::new(if has_text { "button.m-send" } else { "button.m-send.off" }).id(format!("{id}:send")).tab().name(names.2).svg(icon(if has_text { icons::SEND } else { icons::MIC })))
}

/// The y the composer's bottom edge sits at in a viewport `[top, top + height)`: over the
/// soft keyboard when it is up, else over the bottom safe area (home indicator).
pub fn composer_bottom(top: f64, height: f64, keyboard: f64, safe_bottom: f64) -> f64 {
    top + height - keyboard.max(safe_bottom).max(0.0)
}

/// How far (px, positive = scroll content up) a focused field at `field` must move so
/// its bottom clears the keyboard by `margin` in a viewport `[top, top + height)`; a
/// field above the top is brought down to `top + margin` (negative). 0 when it is visible.
pub fn reveal_above(field_top: f64, field_bottom: f64, top: f64, height: f64, keyboard: f64, margin: f64) -> f64 {
    let visible_bottom = top + height - keyboard.max(0.0) - margin;
    if field_bottom > visible_bottom {
        // Its bottom over the keyboard, but never its top past the viewport top.
        (field_bottom - visible_bottom).min((field_top - top - margin).max(0.0))
    } else if field_top < top + margin {
        field_top - top - margin
    } else {
        0.0
    }
}

/// The agent card's hero: a 96px critter slot `@{id}:critter` with a status dot, the name
/// and its status line.
pub fn hero(id: &str, name: &str, dot: Dot, status: &str) -> El {
    El::new("div.m-hero")
        .child(with_dot(El::new("div.m-critter").slot(format!("@{id}:critter")), dot))
        .child(El::new("div.m-h2").text(name))
        .child(with_dot(El::new("div.m-st"), dot).child(El::new("span").text(status)))
}

/// The card's quick actions (`(key, icon, label)`), `{id}:{key}`.
pub fn quick_actions(id: &str, items: &[(&str, &str, &str)]) -> El {
    El::new("div.m-qa").children(items.iter().map(|(k, svg, label)| El::new("button.m-q").id(format!("{id}:{k}")).tab().name(*label).child(ic(svg)).child(El::new("div").text(*label))))
}

/// One key–value row of a card group.
#[derive(Clone, Debug, PartialEq)]
pub enum Kv {
    /// `(key, value, monospace)`.
    Text(String, String, bool),
    /// `(id, label, on)`: a switch (`button.m-toggle`).
    Switch(String, String, bool),
    /// `(id, label, value)`: a drill-down row with a chevron.
    Link(String, String, String),
}

/// A tile of key–value rows (the card's facts and settings).
pub fn kv_group(rows: &[Kv]) -> El {
    El::new("div.m-kv.m-tile").children(rows.iter().enumerate().map(|(i, r)| {
        let spec = if i == 0 { "div.m-kvr.first" } else { "div.m-kvr" };
        match r {
            Kv::Text(k, v, mono) => El::new(spec).child(El::new("div.m-k").text(k.clone())).child(El::new(if *mono { "div.m-v.mono" } else { "div.m-v" }).text(v.clone())),
            Kv::Switch(id, label, on) => El::new(spec).id(id.clone()).name(label.clone()).child(El::new("div.m-k").text(label.clone())).child(El::new("button.m-toggle").control(Control::switch(*on))),
            Kv::Link(id, label, v) => El::new(spec)
                .id(id.clone())
                .name(label.clone())
                .child(El::new("div.m-k").text(label.clone()))
                .child(El::new("div.m-v").text(v.clone()))
                .child(El::new("span.m-chev").svg(icon(icons::CHEV))),
        }
    }))
}

/// A mobile button: look `""`, `"pri"`, `"ghost"`; `block` stretches it.
pub fn btn(id: &str, label: &str, look: &str, block: bool) -> El {
    let spec = match (look, block) {
        ("pri", true) => "button.m-btn.pri.block",
        ("pri", false) => "button.m-btn.pri",
        ("ghost", true) => "button.m-btn.ghost.block",
        ("ghost", false) => "button.m-btn.ghost",
        (_, true) => "button.m-btn.block",
        _ => "button.m-btn",
    };
    El::new(spec).id(id).tab().text(label)
}

// ---- motion: pure state machines a host drives from gestures or mouse drags ----

/// Move `cur` toward `target` over `dt` seconds with a critically damped ease
/// (`rate` 1/s, ~0.25 s to settle at 18). Lands exactly when within half a pixel.
pub fn approach(cur: f64, target: f64, dt: f64, rate: f64) -> f64 {
    let k = 1.0 - (-rate * dt.max(0.0)).exp();
    let next = cur + (target - cur) * k;
    if (target - next).abs() < 0.5 { target } else { next }
}

/// A finger or mouse release this fast (px/s) flicks to the next stop instead of the nearest.
pub const FLICK_SPEED: f64 = 600.0;

/// A bottom sheet's stops.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Detent {
    #[default]
    Closed,
    /// A strip with the head only.
    Peek,
    /// Half the viewport.
    Half,
    /// All but a gap under the status bar.
    Full,
}

/// A bottom sheet: the stop it rests at, the finger's height while dragged and the
/// height shown (easing to the stop after a release).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SheetState {
    pub detent: Detent,
    /// Height of the peek stop, px.
    pub peek: f64,
    /// The gap kept above the full sheet (status bar + a margin), px.
    pub top_gap: f64,
    /// Whether the sheet may close by a drag down (otherwise Peek is the lowest stop).
    pub closable: bool,
    drag: Option<f64>,
    shown: Option<f64>,
}

impl Default for SheetState {
    fn default() -> Self {
        SheetState { detent: Detent::Closed, peek: 120.0, top_gap: 54.0, closable: true, drag: None, shown: None }
    }
}

impl SheetState {
    /// The height of stop `d` in a viewport `vh` tall.
    pub fn detent_height(d: Detent, vh: f64, peek: f64) -> f64 {
        match d {
            Detent::Closed => 0.0,
            Detent::Peek => peek.min(vh),
            Detent::Half => (vh * 0.5).max(peek),
            Detent::Full => (vh - 54.0).max(peek),
        }
    }

    fn stop_h(&self, d: Detent, vh: f64) -> f64 {
        match d {
            Detent::Full => (vh - self.top_gap).max(self.peek),
            d => Self::detent_height(d, vh, self.peek),
        }
    }

    /// Open (or move) to `d` (eases there).
    pub fn open(&mut self, d: Detent) {
        self.detent = d;
        self.drag = None;
    }

    /// The height to draw now.
    pub fn height(&self, vh: f64) -> f64 {
        self.drag.or(self.shown).unwrap_or_else(|| self.stop_h(self.detent, vh))
    }

    /// Whether a drag is in progress.
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// The finger moved the sheet by `dy` (down positive). Past the full stop it
    /// resists (rubber band).
    pub fn drag_by(&mut self, dy: f64, vh: f64) {
        let h = self.drag.unwrap_or_else(|| self.height(vh));
        let full = self.stop_h(Detent::Full, vh);
        let mut next = h - dy;
        if next > full {
            next = full + (next - full) * 0.35;
        }
        self.drag = Some(next.max(0.0));
        self.shown = self.drag;
    }

    /// The finger lifted moving at `vy` px/s (down positive): snap to the nearest stop,
    /// or to the next one in the flick's direction. Returns the stop.
    pub fn release(&mut self, vy: f64, vh: f64) -> Detent {
        let h = self.drag.take().unwrap_or_else(|| self.height(vh));
        let mut stops: Vec<Detent> = vec![Detent::Peek, Detent::Half, Detent::Full];
        if self.closable {
            stops.insert(0, Detent::Closed);
        }
        let hs: Vec<f64> = stops.iter().map(|d| self.stop_h(*d, vh)).collect();
        let pick = if vy.abs() >= FLICK_SPEED {
            if vy > 0.0 {
                // Down: the highest stop below the finger.
                (0..stops.len()).rev().find(|&i| hs[i] < h - 1.0).unwrap_or(0)
            } else {
                (0..stops.len()).find(|&i| hs[i] > h + 1.0).unwrap_or(stops.len() - 1)
            }
        } else {
            (0..stops.len()).min_by(|&a, &b| (hs[a] - h).abs().total_cmp(&(hs[b] - h).abs())).unwrap_or(0)
        };
        self.detent = stops[pick];
        self.shown = Some(h);
        self.detent
    }

    /// Advance the ease by `dt` s; true while it still moves (request a frame).
    pub fn step(&mut self, dt: f64, vh: f64) -> bool {
        if self.drag.is_some() {
            return false;
        }
        let target = self.stop_h(self.detent, vh);
        match self.shown {
            Some(h) => {
                let n = approach(h, target, dt, 18.0);
                self.shown = if n == target { None } else { Some(n) };
                self.shown.is_some()
            }
            None => false,
        }
    }

    /// Show from the current height (call before [`SheetState::open`] for a slide-in).
    pub fn animate_from(&mut self, h: f64) {
        self.shown = Some(h);
    }
}

/// An edge drawer: open progress 0 … 1, following a finger from the edge.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DrawerState {
    pub open: bool,
    drag: Option<f64>,
    shown: f64,
}

impl DrawerState {
    /// 0 (closed) … 1 (open), as drawn now.
    pub fn progress(&self) -> f64 {
        self.drag.unwrap_or(self.shown).clamp(0.0, 1.0)
    }
    pub fn set(&mut self, open: bool) {
        self.open = open;
        self.drag = None;
    }
    pub fn toggle(&mut self) {
        self.set(!self.open);
    }
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }
    /// A pan moved the drawer by `dx` (right positive) for a drawer `w` wide.
    pub fn drag_by(&mut self, dx: f64, w: f64) {
        let p = self.drag.unwrap_or(self.shown) + dx / w.max(1.0);
        self.drag = Some(p.clamp(0.0, 1.0));
    }
    /// The finger lifted at `vx` px/s: a flick decides, else past half opens.
    pub fn release(&mut self, vx: f64) -> bool {
        let p = self.drag.take().unwrap_or(self.shown);
        self.shown = p;
        self.open = if vx.abs() >= FLICK_SPEED { vx > 0.0 } else { p >= 0.5 };
        self.open
    }
    /// Advance the ease by `dt`; true while moving.
    pub fn step(&mut self, dt: f64, w: f64) -> bool {
        if self.drag.is_some() {
            return false;
        }
        let target = if self.open { 1.0 } else { 0.0 };
        let px = approach(self.shown * w, target * w, dt, 18.0);
        self.shown = px / w.max(1.0);
        (self.shown - target).abs() > 1e-9
    }
}

/// A swipe row: the row's offset (negative = moved left, right-side actions show).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SwipeState {
    pub offset: f64,
    rest: f64,
    drag: bool,
}

impl SwipeState {
    /// The finger moved the row by `dx`; it stops `left_w` right / `right_w` left of
    /// home, resisting past that.
    pub fn drag_by(&mut self, dx: f64, left_w: f64, right_w: f64) {
        self.drag = true;
        let mut o = self.offset + dx;
        if o < -right_w {
            o = -right_w + (o + right_w) * 0.35;
        }
        if o > left_w {
            o = left_w + (o - left_w) * 0.35;
        }
        if right_w == 0.0 && o < 0.0 {
            o = 0.0;
        }
        if left_w == 0.0 && o > 0.0 {
            o = 0.0;
        }
        self.offset = o;
    }
    /// Released at `vx`: open the side shown past a third of its width (or flicked
    /// open), else close.
    pub fn release(&mut self, vx: f64, left_w: f64, right_w: f64) -> f64 {
        self.drag = false;
        let o = self.offset;
        self.rest = if o < 0.0 && right_w > 0.0 && (o < -right_w / 3.0 || vx <= -FLICK_SPEED) && vx < FLICK_SPEED {
            -right_w
        } else if o > 0.0 && left_w > 0.0 && (o > left_w / 3.0 || vx >= FLICK_SPEED) && vx > -FLICK_SPEED {
            left_w
        } else {
            0.0
        };
        self.rest
    }
    /// Snap closed (after an action, or a tap elsewhere).
    pub fn close(&mut self) {
        self.drag = false;
        self.rest = 0.0;
    }
    pub fn is_open(&self) -> bool {
        self.rest != 0.0
    }
    pub fn dragging(&self) -> bool {
        self.drag
    }
    /// Advance the ease; true while moving.
    pub fn step(&mut self, dt: f64) -> bool {
        if self.drag {
            return false;
        }
        self.offset = approach(self.offset, self.rest, dt, 20.0);
        self.offset != self.rest
    }
}

#[cfg(all(test, feature = "flow-flex"))]
mod tests;
