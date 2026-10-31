//! The component catalog on one panel, on the DOM engine: tabs, a text field,
//! buttons with a tooltip, a list with a context menu, checkbox and switch, a table,
//! a select with its popover, a dialog, a side drawer and toasts — every piece from
//! [`uzor::ui::domkit`], themed by the shared `--k-*` look. The one piece the DOM
//! has no element for, the draggable splitter between the two panes, is the kit
//! widget ([`kit::splitter`]) laid over the DOM gutter.
//!
//! Call [`Gallery::paint`] from an [`App::panel`](uzor_framework::App::panel) hook.
//! The gallery owns the values (text, choices, sizes) and reads the clicks back from
//! the [`DomResponse`](kit::DomResponse).

use uzor::ui::dom::{Edit, El};
use uzor::ui::domkit::{self as k, icons, MenuItem, SelectOpt};
use uzor::ui::style::Sheet;
use uzor::Rect;
use uzor_framework::widgets::kit::{self, Axis};
use uzor_framework::{PanelCx, Spec};

use crate::look;

/// How long a toast stays, in ms.
const TOAST_MS: u64 = 3000;

impl crate::page_app::Page for Gallery {
    fn title(&self) -> &str {
        "Kit gallery"
    }
    fn size(&self) -> (u32, u32) {
        (1100, 720)
    }
    fn paint(&mut self, cx: &mut PanelCx<'_, crate::page_app::PageSpec>, now_ms: u64) {
        let area = cx.rect;
        self.paint_in(cx, area, now_ms);
    }
}

/// The values the gallery's pieces edit.
pub struct Gallery {
    /// Text of the name field.
    pub name: String,
    /// Active tab.
    pub tab: usize,
    /// Chosen option of the select.
    pub level: usize,
    /// The select's popover is open.
    pub select_open: bool,
    /// Selected list row.
    pub row: Option<usize>,
    /// Left pane right edge (window x), moved by the splitter; 0 = not placed yet.
    pub split: f64,
    /// The context menu is open.
    pub menu_open: bool,
    /// The dialog is open.
    pub dialog_open: bool,
    /// Toasts on screen: text and the time it was shown (ms).
    pub toasts: Vec<(String, u64)>,
    /// How many times "Save" was pressed.
    pub saves: u32,
    /// The last menu item picked.
    pub last_action: Option<usize>,
    /// The notify checkbox.
    pub notify: bool,
    /// The compact switch (it opens the drawer).
    pub compact: bool,
    /// Selected table row.
    pub table_row: Option<usize>,
    /// The drawer is open.
    pub drawer_open: bool,
    /// Dark look.
    pub dark: bool,
    /// The element under the pointer on the last frame.
    pub hovered: Option<String>,
    /// Where the context menu opens (window coordinates).
    menu_at: Rect,
    /// The select trigger on the last frame.
    anchor: Option<Rect>,
    /// Hit rects of the last frame (test door, see [`Gallery::hit`]).
    hits: Vec<(String, Rect)>,
    sheet: Sheet,
}

impl Default for Gallery {
    fn default() -> Self {
        Gallery {
            name: String::new(),
            tab: 0,
            level: 1,
            select_open: false,
            row: None,
            split: 0.0,
            menu_open: false,
            dialog_open: false,
            toasts: Vec::new(),
            saves: 0,
            last_action: None,
            notify: true,
            compact: false,
            table_row: None,
            drawer_open: false,
            dark: true,
            hovered: None,
            menu_at: Rect::new(0.0, 0.0, 0.0, 0.0),
            anchor: None,
            hits: Vec::new(),
            sheet: look::sheet(CSS),
        }
    }
}

const TABS: [&str; 3] = ["Profile", "Limits", "Nodes"];
const LEVELS: [&str; 3] = ["Low", "Normal", "High"];
const ROWS: [&str; 6] = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta"];
const MENU: [&str; 3] = ["Rename", "Duplicate", "Delete"];
const LIMITS: [(&str, f64); 3] = [("CPU", 0.42), ("Memory", 0.68), ("Disk", 0.15)];

const CSS: &str = r#"
div.g-split { display: flex; background: var(--k-bg) }
div.g-pane { display: flex; flex-direction: column; gap: 10px; padding: 14px; overflow: hidden; flex-shrink: 0 }
div.g-gutter { width: 9px; flex-shrink: 0; display: flex; justify-content: center }
div.g-gutter > div { width: 1px; background: var(--k-line) }
div.g-right { flex-grow: 1; display: flex; flex-direction: column; gap: 10px; padding: 14px; overflow: hidden }
div.g-lim { display: flex; align-items: center; gap: 10px; flex-shrink: 0 }
div.g-lim > div.g-lbl { width: 70px; color: var(--k-muted) }
div.g-lim > div.k-progress { flex-grow: 1 }
div.g-lim > div.g-pct { width: 40px; text-align: right }
div.g-keys { display: flex; flex-direction: column; gap: 6px; flex-shrink: 0 }
"#;

fn item(i: usize) -> MenuItem {
    let it = MenuItem::new(MENU[i]);
    match i {
        0 => it.icon(icons::EDIT).kbd("F2"),
        1 => it.icon(icons::COPY).kbd("Ctrl+D"),
        _ => it.icon(icons::TRASH).kbd("Del").danger(true),
    }
}

impl Gallery {
    /// Where element `id` was laid out on the last frame (window coordinates).
    pub fn hit(&self, id: &str) -> Option<Rect> {
        self.hits.iter().find(|(h, _)| h == id).map(|(_, r)| *r)
    }

    /// The page for `area` (window coordinates); `now_ms` ages the toasts.
    pub fn tree(&self, area: Rect, now_ms: u64) -> El {
        let origin = |r: Rect| Rect::new(r.x - area.x, r.y - area.y, r.width, r.height);
        let vp = Rect::new(0.0, 0.0, area.width, area.height);
        let left_w = (self.split - area.x).max(160.0);
        let body = match self.tab {
            0 => El::new("div.p-card")
                .child(k::field("Name", k::input("gallery:name", Edit::new(&self.name).placeholder("Name"))))
                .child(
                    El::new("div.p-row")
                        .child(El::new("button.k-btn.primary").id("gallery:save").tab().text("Save"))
                        .child(k::button("gallery:open", "Details…"))
                        .child(El::new("div.p-grow"))
                        .child(El::new("div.p-muted").text(format!("saved {}×", self.saves))),
                )
                .child(
                    El::new("div.p-row")
                        .child(k::checkbox("gallery:notify", self.notify, "Notify"))
                        .child(El::new("div.p-grow"))
                        .child(El::new("div").text("Compact"))
                        .child(k::switch("gallery:compact", self.compact, "Compact")),
                ),
            1 => El::new("div.p-card")
                .children(LIMITS.iter().map(|(l, v)| {
                    El::new("div.g-lim")
                        .child(El::new("div.g-lbl").text(*l))
                        .child(k::progress(*v))
                        .child(El::new("div.g-pct").text(format!("{:.0}%", v * 100.0)))
                }))
                .child(k::field("Level", k::segmented("gallery:seg", &LEVELS, self.level))),
            _ => {
                let rows: Vec<Vec<String>> = ROWS.iter().enumerate().map(|(i, r)| vec![r.to_string(), (i * 7).to_string()]).collect();
                El::new("div.p-card").child(k::table("gallery:table", &["Name", "Count"], "1fr 80px", &[1], &rows, self.table_row))
            }
        };
        let list_rows: Vec<(&str, Option<&str>)> = ROWS.iter().enumerate().map(|(i, r)| (*r, Some(["node", "edge", "hub"][i % 3]))).collect();
        let left = El::new("div.g-pane")
            .css(format!("width: {left_w}px; height: {}px", area.height))
            .child(
                El::new("div.p-row")
                    .child(look::ico(icons::BOX, true))
                    .child(El::new("div.p-h1").text("Kit gallery"))
                    .child(El::new("div.p-grow"))
                    .child(k::badge("DOM + kit", "info")),
            )
            .child(k::tabs("gallery:tabs", &TABS, self.tab))
            .child(body)
            .child(El::new("div.p-sec").text("ROWS — CLICK ONE FOR ITS MENU"))
            .child(k::list("gallery:list", &list_rows, self.row));
        let action = self.last_action.map(|i| MENU[i]).unwrap_or("—");
        let right = El::new("div.g-right")
            .child(El::new("div.p-sec").text("SELECT"))
            .child(k::select_trigger("gallery:level", LEVELS[self.level], self.select_open, "Level"))
            .child(El::new("div.p-sec").text("STATUS"))
            .child(
                El::new("div.p-grid2")
                    .child(k::stat("gallery:saves", "SAVES", &self.saves.to_string(), "toasts on save"))
                    .child(k::stat("gallery:action", "LAST ACTION", action, "from the row menu")),
            )
            .child(El::new("div.p-sec").text("SPLITTER"))
            .child(El::new("div.p-muted").css("white-space: normal").text("The bar on the left is the kit splitter over a DOM gutter: drag it."))
            .child(
                El::new("div.g-keys")
                    .child(El::new("div.p-row").child(k::kbd("Tab")).child(El::new("div.p-muted").text("next control")))
                    .child(El::new("div.p-row").child(k::kbd("Space")).child(El::new("div.p-muted").text("press / toggle"))),
            );
        let mut page = look::root(self.dark, area.height).child(
            El::new("div.g-split")
                .css(format!("height: {}px", area.height))
                .child(left)
                .child(El::new("div.g-gutter").child(El::new("div")))
                .child(right),
        );
        if self.select_open {
            let a = origin(self.anchor.unwrap_or(Rect::new(left_w + 20.0, 60.0, 200.0, 32.0)));
            let opts: Vec<SelectOpt> = LEVELS.iter().map(|l| SelectOpt::new(*l)).collect();
            page = page.child(k::select_popover("gallery:level", &opts, Some(self.level), k::popover_at(a, vp, 200.0, 130.0)));
        }
        if self.menu_open {
            let items: Vec<MenuItem> = (0..MENU.len()).map(item).collect();
            let at = origin(self.menu_at);
            page = page.child(k::menu("gallery", Some("Row"), &items, k::popover_at(at, vp, 200.0, 130.0)));
        }
        if self.hovered.as_deref() == Some("gallery:save") {
            if let Some(r) = self.hit("gallery:save") {
                page = page.child(k::tooltip("Saves the name", origin(r)));
            }
        }
        if self.drawer_open {
            page = page.child(k::side_panel(
                "gallery:drawer",
                "Settings",
                [
                    El::new("div.p-muted").css("white-space: normal").text("A side drawer: position: fixed, above the page."),
                    k::checkbox("gallery:notify", self.notify, "Notify"),
                    El::new("button.k-btn.primary").id("gallery:drawer:done").tab().text("Done"),
                ],
            ));
        }
        if self.dialog_open {
            let d = k::centered(vp, 360.0, 220.0);
            let msg = format!("Name: {}\nLevel: {}\nSaves: {}", if self.name.is_empty() { "—" } else { &self.name }, LEVELS[self.level], self.saves);
            page = page.child(k::dialog("gallery:dialog", "Details", El::new("div").css("white-space: normal").text(msg), &["OK", "Cancel"], d));
        }
        let live: Vec<(&str, &str)> = self.toasts.iter().filter(|(_, t)| now_ms.saturating_sub(*t) < TOAST_MS).map(|(m, _)| ("ok", m.as_str())).collect();
        if !live.is_empty() {
            page = page.child(k::toasts("gallery:toasts", &live));
        }
        page
    }

    /// Paint every piece into a dock leaf below its header. `now_ms` drives the toasts.
    pub fn paint<S: Spec>(&mut self, cx: &mut PanelCx<'_, S>, now_ms: u64) {
        let area = look::leaf_body(cx.rect);
        self.paint_in(cx, area, now_ms);
    }

    /// Paint every piece into `area` (window coordinates).
    pub fn paint_in<S: Spec>(&mut self, cx: &mut PanelCx<'_, S>, area: Rect, now_ms: u64) {
        if self.split <= 0.0 {
            self.split = area.x + area.width * 0.6;
        }
        self.split = self.split.clamp(area.x + 280.0, (area.x + area.width - 220.0).max(area.x + 280.0));
        self.toasts.retain(|(_, t)| now_ms.saturating_sub(*t) < TOAST_MS);
        let tree = self.tree(area, now_ms);
        let resp = kit::dom(area, &self.sheet, tree).build(cx);
        self.hits = resp.dom.hits().map(|(id, r)| (id.to_string(), r)).collect();
        self.anchor = resp.dom.rect("gallery:level");
        // The splitter: a kit widget over the DOM gutter (DOM has no drag). It registers
        // after the page, so it would sit above the page's overlays: leave it out while
        // one is open.
        let overlay = self.dialog_open || self.drawer_open || self.menu_open || self.select_open;
        if !overlay {
            kit::splitter("gallery:split", Axis::X, area, &mut self.split)
                .range(area.x + 280.0, area.x + area.width - 220.0)
                .build(cx);
        }
        if resp.hovered != self.hovered {
            self.hovered = resp.hovered.clone();
            cx.out.request_frame();
        }
        for (id, v) in &resp.edited {
            if id == "gallery:name" {
                self.name = v.clone();
            }
        }
        if let Some(c) = resp.clicked.as_deref() {
            self.click(c, now_ms);
            cx.out.request_frame();
        }
        if !self.toasts.is_empty() {
            cx.out.request_frame();
        }
    }

    /// React to a click on element `c`.
    pub fn click(&mut self, c: &str, now_ms: u64) {
        let was_menu = self.menu_open;
        let was_select = self.select_open;
        self.menu_open = false;
        self.select_open = false;
        if let Some(i) = k::picked("gallery:tabs", "tab", c) {
            self.tab = i;
        } else if let Some(i) = k::picked("gallery:list", "row", c) {
            self.row = Some(i);
            if let Some(r) = self.hit(c) {
                self.menu_at = Rect::new(r.x + r.width * 0.4, r.y + r.height * 0.5, 0.0, 0.0);
            }
            self.menu_open = true;
        } else if let Some(i) = k::picked("gallery", "item", c) {
            if was_menu {
                self.last_action = Some(i);
                self.toasts.push((format!("{} {}", MENU[i], ROWS[self.row.unwrap_or(0)]), now_ms));
            }
        } else if let Some(i) = k::picked_opt("gallery:level", c) {
            self.level = i;
        } else if let Some(i) = k::picked("gallery:seg", "", c) {
            self.level = i;
        } else if let Some(i) = k::picked("gallery:table", "row", c) {
            self.table_row = Some(i);
        } else {
            match c {
                "gallery:level" => self.select_open = !was_select,
                "gallery:save" => {
                    self.saves += 1;
                    self.toasts.push(("Saved".into(), now_ms));
                }
                "gallery:open" => self.dialog_open = true,
                "gallery:notify" => self.notify = !self.notify,
                "gallery:compact" => {
                    self.compact = !self.compact;
                    self.drawer_open = self.compact;
                }
                "gallery:drawer:done" | "gallery:drawer:close" => {
                    self.drawer_open = false;
                    self.compact = false;
                }
                c if c.starts_with("gallery:dialog:") => self.dialog_open = false,
                c if c.starts_with("gallery:toasts:") && c.ends_with(":close") => {
                    if let Some(i) = c.split(':').nth(2).and_then(|i| i.parse::<usize>().ok()) {
                        if i < self.toasts.len() {
                            self.toasts.remove(i);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use uzor::input::{ModifierKeys, MouseButton};
    use uzor_framework::{HeadlessHost, InputEvent, Point, PointerInput};

    use crate::page_app::{PageApp, PageSpec};

    type Host = HeadlessHost<PageSpec, PageApp<Gallery>>;

    fn open() -> Host {
        let app = PageApp::new(Gallery::default());
        let cfg = app.runtime_config();
        let (mut host, _) = HeadlessHost::new(app, cfg);
        for _ in 0..6 {
            host.advance(0.02);
        }
        host
    }

    fn click_id(host: &mut Host, id: &str) {
        let r = host.runtime().app().page.hit(id).unwrap_or_else(|| panic!("{id} not laid out"));
        let p = Point::new(r.x + r.width * 0.5, r.y + r.height * 0.5);
        let win = host.snapshot().windows[0].id;
        let mods = ModifierKeys::default();
        host.input(win, InputEvent::Pointer(PointerInput::Moved { pos: p, mods }));
        host.advance(0.02);
        host.input(win, InputEvent::Pointer(PointerInput::Down { pos: p, button: MouseButton::Left, mods }));
        host.advance(0.02);
        host.input(win, InputEvent::Pointer(PointerInput::Up { pos: p, button: MouseButton::Left, mods }));
        for _ in 0..3 {
            host.advance(0.02);
        }
    }

    #[test]
    fn gallery_lays_out_every_piece() {
        let host = open();
        let g = &host.runtime().app().page;
        for id in ["gallery:tabs:tab:2", "gallery:name", "gallery:save", "gallery:open", "gallery:list:row:5", "gallery:notify", "gallery:compact", "gallery:level", "gallery:saves"] {
            assert!(g.hit(id).is_some(), "{id} not laid out");
        }
        let save = g.hit("gallery:save").unwrap();
        let level = g.hit("gallery:level").unwrap();
        assert!(level.x > save.x + save.width, "the select sits in the right pane");
    }

    #[test]
    fn gallery_clicks_drive_every_overlay() {
        let mut host = open();
        click_id(&mut host, "gallery:save");
        assert_eq!(host.runtime().app().page.saves, 1);
        assert!(host.runtime().app().page.hit("gallery:toasts").is_some(), "save shows a toast");

        click_id(&mut host, "gallery:list:row:2");
        assert!(host.runtime().app().page.menu_open, "a row opens its menu");
        click_id(&mut host, "gallery:item:2");
        assert_eq!(host.runtime().app().page.last_action, Some(2), "menu pick");

        click_id(&mut host, "gallery:level");
        assert!(host.runtime().app().page.select_open);
        click_id(&mut host, "gallery:level:opt:2");
        assert_eq!(host.runtime().app().page.level, 2);

        click_id(&mut host, "gallery:open");
        assert!(host.runtime().app().page.dialog_open);
        click_id(&mut host, "gallery:dialog:foot:0");
        assert!(!host.runtime().app().page.dialog_open);

        click_id(&mut host, "gallery:compact");
        assert!(host.runtime().app().page.drawer_open, "compact opens the drawer");
        click_id(&mut host, "gallery:drawer:done");
        assert!(!host.runtime().app().page.drawer_open);

        click_id(&mut host, "gallery:tabs:tab:2");
        click_id(&mut host, "gallery:table:row:3");
        assert_eq!(host.runtime().app().page.table_row, Some(3));
    }
}
