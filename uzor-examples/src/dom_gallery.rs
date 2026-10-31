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
            .child(El::new("div.g-row").children([k::button("save", "Save").class_hint("primary"), k::button("cancel", "Cancel")]))
            .child(El::new("div.g-sec").text("CSS"))
            .child(El::new("div.g-thumb"))
            .child(El::new("div.g-wide"));
        let blocks = El::new("div.g-col")
            .child(El::new("div.g-sec").text("CHAT BLOCKS"))
            .child(El::new("div.g-row").children(
                ["All", "Mentions", "Files"].iter().enumerate().map(|(i, l)| k::chip(&format!("chip:{i}"), l, i == self.chip)),
            ))
            .child(k::banner("b403", "error", "this nick has no right for this action", None, true))
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
