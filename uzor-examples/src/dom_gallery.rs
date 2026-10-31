//! The DOM widget catalog ([`uzor::ui::domkit`]) on one page: controls, the select
//! with its fixed popover, chat building blocks, a virtualized message list and a
//! fixed side panel. [`DomGallery::tree`] is pure, so the same page renders in a kit
//! panel ([`uzor_framework::widgets::kit::dom`]), a headless test, or the
//! `dom-gallery-frames` PNG bin.

use uzor::ui::dom::{Edit, El};
use uzor::ui::domkit::{self as k, icons, SelectOpt, VirtualList};
use uzor::Rect;

/// Gallery state a host (or a scripted demo) changes between frames.
#[derive(Clone, Debug)]
pub struct DomGallery {
    /// Switch states.
    pub notify: bool,
    /// Second switch.
    pub compact: bool,
    /// Checkbox.
    pub agree: bool,
    /// The nick input.
    pub nick: Edit,
    /// The select is open.
    pub open: bool,
    /// Chosen option.
    pub level: usize,
    /// Hovered message (shows its action row).
    pub hover_msg: Option<usize>,
    /// The collapsible card is open.
    pub card_open: bool,
    /// Active chip.
    pub chip: usize,
    /// The side panel is shown.
    pub side: bool,
    /// Message list scroll.
    pub scroll: f64,
    /// Seconds, for the spinner.
    pub t: f64,
    /// The closable error banner was dismissed.
    pub closed_403: bool,
}

impl Default for DomGallery {
    fn default() -> Self {
        DomGallery {
            notify: true,
            compact: false,
            agree: false,
            nick: Edit::new("lavr").placeholder("Nick"),
            open: false,
            level: 1,
            hover_msg: None,
            card_open: true,
            chip: 0,
            side: false,
            scroll: 0.0,
            t: 0.0,
            closed_403: false,
        }
    }
}

/// The select's options.
pub fn levels() -> Vec<SelectOpt> {
    vec![
        SelectOpt::new("Quiet").sub("only mentions"),
        SelectOpt::new("Normal").sub("mentions and replies"),
        SelectOpt::new("Everything").sub("every message"),
        SelectOpt::new("Admin feed").sub("needs the admin right").disabled(true),
    ]
}

/// Demo messages (markdown bodies).
pub const MESSAGES: &[(bool, &str, &str)] = &[
    (false, "mira", "Morning! The **DOM** widgets are in — try the _select_ in Settings."),
    (true, "lavr", "Looks good. Does `aspect-ratio` work now?"),
    (false, "mira", "Yes, and *italic*, `% widths`, dashed borders and `.a > .b` selectors."),
    (true, "lavr", "Ping @knyaz when the gallery is recorded: https://uzor.dev/gallery"),
    (false, "knyaz", "On it. Long chats only lay out the **visible** rows now."),
    (true, "lavr", "Nice — _no more_ frame drops in 5k-message rooms."),
    (false, "mira", "Side panel and popover are `position: fixed`, so they don't scroll."),
    (true, "lavr", "Ship it as **1.5.21**."),
];

const PAGE_CSS: &str = r#"
div.g-page { display: flex; gap: 16px; padding: 16px; background: var(--k-bg) }
div.g-col { display: flex; flex-direction: column; gap: 12px; width: 300px }
div.g-sec { color: var(--k-muted); font-size: 11px; font-weight: 600 }
div.g-row { display: flex; align-items: center; gap: 10px }
div.g-label { flex-grow: 1 }
div.g-chat { display: flex; flex-direction: column; gap: 6px; width: 440px; height: 520px; overflow: hidden; padding: 10px;
             border-radius: 12px; border: 1px solid var(--k-line); background: #141518 }
div.g-thumb { width: 100%; aspect-ratio: 16 / 9; border-radius: 10px; border: 2px dotted var(--k-line); background: #1b1d21 }
div.g-wide { width: 50%; height: 6px; border-radius: 3px; background: var(--k-accent) }
div.g-list { display: flex; flex-direction: column; gap: 6px; flex-shrink: 0 }
div.k-vspace { flex-shrink: 0 }
"#;

impl DomGallery {
    /// The page sheet: [`KIT_CSS`](uzor::ui::domkit::KIT_CSS) plus the gallery layout.
    pub fn sheet() -> uzor::ui::style::Sheet {
        k::sheet_with(PAGE_CSS).expect("gallery css")
    }

    /// Where the select trigger sits (for the popover anchor), given the laid-out page.
    pub fn trigger_rect(dom: &uzor::ui::dom::Dom) -> Option<Rect> {
        dom.rect("level")
    }

    /// The page for a `w × h` viewport. `anchor` is the select trigger's rect from the
    /// previous layout (the popover opens below it).
    pub fn tree(&self, list: &VirtualList, w: f64, h: f64, anchor: Option<Rect>) -> El {
        let vp = Rect::new(0.0, 0.0, w, h);
        let lv = levels();
        let controls = El::new("div.g-col")
            .child(El::new("div.g-sec").text("CONTROLS"))
            .child(row("Notifications", k::switch("notify", self.notify, "Notifications")))
            .child(row("Compact mode", k::switch("compact", self.compact, "Compact mode")))
            .child(k::checkbox("agree", self.agree, "I read the rules"))
            .child(k::input("nick", self.nick.clone()))
            .child(El::new("div.g-row").child(El::new("div.g-label").text("Notify me")).child(k::select_trigger("level", &lv[self.level].label, self.open, "Notify me")))
            .child(El::new("div.g-row").child(k::spinner(self.t)).child(El::new("div.g-label").text("Syncing…")))
            .child(El::new("div.g-row").children([k::button("save", "Save").class_hint("primary"), k::button("cancel", "Cancel"), k::button("room", "Room…")]))
            .child(El::new("div.g-sec").text("CSS"))
            .child(El::new("div.g-thumb"))
            .child(El::new("div.g-wide"));
        let blocks = El::new("div.g-col")
            .child(El::new("div.g-sec").text("CHAT BLOCKS"))
            .child(El::new("div.g-row").children(
                ["All", "Mentions", "Files"].iter().enumerate().map(|(i, l)| k::chip(&format!("chip:{i}"), l, i == self.chip)),
            ))
            .child_if(!self.closed_403, || k::banner("b403", "error", "this nick has no right for this action", None, true))
            .child(k::banner("bsync", "warn", "Reconnecting to the node…", Some("Retry"), false))
            .child(k::card("card", "Pinned: release checklist", self.card_open, El::new("div").runs(k::inline_md("bump to **1.5.21**, _changelog_, `cargo package`"))))
            .child(k::slot("@kit-chart", 300.0, 60.0));
        let win = list.window(self.scroll, 500.0, 200.0);
        let inner = El::new("div.g-list").css(format!("margin-top: {}px", -self.scroll.min(list.total())));
        let rows = list.el(inner, win, |i| {
            let (mine, who, body) = MESSAGES[i % MESSAGES.len()];
            let time = format!("#{} · {:02}:{:02}", i + 1, 9 + i / 60 % 12, i % 60);
            let mut b = k::bubble(&format!("m{i}"), mine, (!mine).then_some(who), body, Some(&time));
            if self.hover_msg == Some(i) {
                b = b.child(k::action_row(&format!("m{i}"), &[("reply", icons::REPLY, "Reply"), ("copy", icons::COPY, "Copy"), ("react", icons::SMILE, "React")]));
            }
            b
        });
        let chat = El::new("div.g-chat").id("chat").child(rows);
        let mut page = El::new("div.k-root").child(El::new("div.g-page").children([controls, blocks, chat]));
        if self.open {
            let a = anchor.unwrap_or(Rect::new(150.0, 250.0, 150.0, 32.0));
            page = page.child(k::select_popover("level", &lv, Some(self.level), k::popover_at(a, vp, 220.0, 190.0)));
        }
        if self.side {
            page = page.child(k::side_panel("side", "Room settings", [
                row("Mute room", k::switch("side:mute", false, "Mute room")),
                row("Show previews", k::switch("side:prev", true, "Show previews")),
                k::input("side:topic", Edit::new("").placeholder("Room topic")),
            ]));
        }
        page
    }
}

fn row(label: &str, control: El) -> El {
    El::new("div.g-row").child(El::new("div.g-label").text(label)).child(control)
}

trait ClassHint {
    fn class_hint(self, c: &str) -> El;
}
impl ClassHint for El {
    /// Add a class to the element's spec.
    fn class_hint(mut self, c: &str) -> El {
        self.spec = format!("{}.{c}", self.spec);
        self
    }
}

/// The gallery as a live [`Page`](crate::page_app::Page): every control reacts (switches,
/// checkbox, the nick input, the select, chips, the card, banners, the side panel opened
/// by "Room…"), hovering a message shows its action row, the wheel scrolls the
/// virtualized chat, and the `@kit-chart` slot is painted by kit code every frame.
pub struct DomGalleryPage {
    /// The gallery state.
    pub g: DomGallery,
    list: VirtualList,
    anchor: Option<Rect>,
    sheet: uzor::ui::style::Sheet,
    /// Last click, for tests.
    pub last_click: Option<String>,
    hits: Vec<(String, Rect)>,
}

impl Default for DomGalleryPage {
    fn default() -> Self {
        let mut list = VirtualList::new(MESSAGES.len() * 50, 64.0);
        list.gap = 6.0;
        DomGalleryPage { g: DomGallery::default(), list, anchor: None, sheet: DomGallery::sheet(), last_click: None, hits: Vec::new() }
    }
}

impl DomGalleryPage {
    /// Paint into `area` at frame time `now_ms`.
    pub fn paint_in<S: uzor_framework::Spec>(&mut self, cx: &mut uzor_framework::PanelCx<'_, S>, area: Rect, now_ms: u64) {
        use uzor_framework::widgets::kit;
        self.g.t = now_ms as f64 / 1000.0;
        // The chat scrolls with the wheel over it (kept in range by the list).
        let max = (self.list.total() - 500.0).max(0.0);
        let tree = self.g.tree(&self.list, area.width, area.height, self.anchor.map(|a| Rect::new(a.x - area.x, a.y - area.y, a.width, a.height)));
        let resp = kit::dom(area, &self.sheet, tree).build(cx);
        self.hits = resp.dom.hits().map(|(id, r)| (id.to_string(), r)).collect();
        let over_chat = resp.hovered.as_deref().is_some_and(|h| h == "chat" || (h.starts_with('m') && h[1..].chars().next().is_some_and(|c| c.is_ascii_digit())));
        let dy = cx.view.scroll().1;
        if over_chat && dy != 0.0 {
            let px = if cx.view.scroll_precise() { dy } else { dy * 48.0 };
            self.g.scroll = (self.g.scroll - px).clamp(0.0, max);
            cx.out.request_frame();
        }
        let win = self.list.window(self.g.scroll, 500.0, 200.0);
        if self.list.measure(&resp.dom, win, |i| format!("m{i}")) {
            cx.out.request_frame();
        }
        self.anchor = DomGallery::trigger_rect(&resp.dom).or(self.anchor);
        for s in resp.dom.slots() {
            if s.0 == "@kit-chart" {
                paint_chart(cx.render, s.1, self.g.t);
            }
        }
        // Hovering a message (or its action row) shows the row.
        let hm = resp.hovered.as_deref().and_then(|h| h.strip_prefix('m')).and_then(|r| r.split(':').next()?.parse().ok());
        if hm.is_some() || resp.hovered.is_none() {
            self.g.hover_msg = hm;
        }
        for (id, v) in &resp.edited {
            if id == "nick" {
                self.g.nick.value = v.clone();
            }
        }
        if let Some(c) = resp.clicked.clone() {
            self.click(&c);
            self.last_click = Some(c);
        }
        // The spinner and the chart animate.
        cx.out.request_frame();
    }

    /// React to a click on element `c`.
    pub fn click(&mut self, c: &str) {
        let g = &mut self.g;
        let was_open = g.open;
        g.open = false;
        match c {
            "notify" => g.notify = !g.notify,
            "compact" => g.compact = !g.compact,
            "agree" => g.agree = !g.agree,
            "level" => g.open = !was_open,
            "card" => g.card_open = !g.card_open,
            "b403:close" => g.closed_403 = true,
            "room" => g.side = true,
            "side:close" => g.side = false,
            _ => {
                if let Some(i) = k::picked_opt("level", c) {
                    if levels().get(i).is_some_and(|o| !o.disabled) {
                        g.level = i;
                    }
                } else if let Some(i) = k::picked("chip", "", c) {
                    g.chip = i;
                }
            }
        }
    }
}

impl crate::page_app::Page for DomGalleryPage {
    fn title(&self) -> &str {
        "DOM gallery"
    }
    fn size(&self) -> (u32, u32) {
        (1280, 760)
    }
    fn paint(&mut self, cx: &mut uzor_framework::PanelCx<'_, crate::page_app::PageSpec>, now_ms: u64) {
        let area = cx.rect;
        self.paint_in(cx, area, now_ms);
    }
}

/// A kit-drawn bar chart: what goes in the `@kit-chart` slot.
pub fn paint_chart(ctx: &mut dyn uzor::render::RenderContext, r: Rect, t: f64) {
    let n = 24;
    let bw = r.width / n as f64;
    ctx.set_fill_color("#3b82f6");
    for i in 0..n {
        let v = 0.5 + 0.45 * ((i as f64 * 0.5 + t * 2.0).sin());
        let h = (r.height - 8.0) * v;
        ctx.fill_rect(r.x + i as f64 * bw + 2.0, r.y + r.height - 4.0 - h, bw - 4.0, h);
    }
}

#[cfg(test)]
mod page_tests {
    use super::*;
    use crate::page_app::testing::{click_at, open, Host};

    fn click(h: &mut Host<DomGalleryPage>, id: &str) {
        let hits: Vec<(String, Rect)> = h.runtime().app().page.hits.clone();
        let r = hits.iter().find(|(i, _)| i == id).map(|(_, r)| *r).unwrap_or_else(|| panic!("no hit {id}"));
        click_at(h, r.x + r.width / 2.0, r.y + r.height / 2.0);
    }

    #[test]
    fn controls_react() {
        let mut h = open(DomGalleryPage::default());
        click(&mut h, "compact");
        assert!(h.runtime().app().page.g.compact);
        click(&mut h, "level");
        assert!(h.runtime().app().page.g.open);
        click(&mut h, "level:opt:2");
        let g = &h.runtime().app().page.g;
        assert_eq!((g.level, g.open), (2, false));
        click(&mut h, "chip:1");
        click(&mut h, "b403:close");
        click(&mut h, "room");
        let g = &h.runtime().app().page.g;
        assert_eq!((g.chip, g.closed_403, g.side), (1, true, true));
        click(&mut h, "side:close");
        assert!(!h.runtime().app().page.g.side);
    }
}
