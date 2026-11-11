//! The tray widgets ([`uzor::ui::tray`]) on one page: every widget in the glass
//! flyout look over a wallpaper, the opaque thumbnail look, and the icon-size
//! variants (a usage ring, a status badge), dark or light, Russian or English.
//! [`TrayGallery::tree`] is pure: the page renders live ([`TrayPage`]), in tests and
//! in the `tray-widgets-frames` PNG bin. The `tray-widgets` bin also puts a real tray
//! icon up with the flyout and the taskbar thumbnail.

use uzor::render::RenderContext;
use uzor::ui::dash::{Meter, TaskState};
use uzor::ui::dom::El;
use uzor::ui::tray::{self, TrayAgent, TrayLabels, TrayLook, TrayModel, TrayStatus, TrayTask, TrayWidget};
use uzor::ui::widgets::atomic::critter::{CritterFamily, CritterState};
use uzor::Rect;

/// Gallery state.
#[derive(Clone, Debug, PartialEq)]
pub struct TrayGallery {
    pub ru: bool,
    pub dark: bool,
    /// The widget the tray shows (the bin's flyout).
    pub pick: TrayWidget,
}

impl Default for TrayGallery {
    fn default() -> Self {
        TrayGallery { ru: true, dark: true, pick: TrayWidget::Combined }
    }
}

/// The page around the cards.
pub const PAGE_CSS: &str = r#"
div.tg-page { display: flex; flex-direction: column; gap: 14px; padding: 18px 20px }
div.tg-bar { display: flex; align-items: center; gap: 8px }
div.tg-h1 { font-size: 15px; font-weight: 600; flex-grow: 1; color: var(--tg-fg) }
div.tg-sec { color: var(--tg-sub); font-size: 11px; font-weight: 600 }
div.tg-row { display: flex; gap: 18px; flex-wrap: wrap; align-items: flex-start }
div.tg-cell { display: flex; flex-direction: column; gap: 6px }
div.tg-cap { color: var(--tg-sub); font-size: 11px }
span.tg-icon { width: 32px; height: 32px }
span.tg-icon.big { width: 64px; height: 64px }
button.tg-btn { height: 28px; padding: 0 12px; border-radius: 8px; background: var(--tg-btn); color: var(--tg-fg); border: 1px solid var(--tg-line); font-size: 12px }
button.tg-btn.on { border-color: var(--tg-fg) }
"#;

impl TrayGallery {
    /// Kit + dash + tray rules and the page's.
    pub fn sheet() -> uzor::ui::style::Sheet {
        uzor::ui::style::Sheet::parse(&[uzor::ui::domkit::KIT_CSS, uzor::ui::dash::DASH_CSS, tray::TRAY_CSS, PAGE_CSS].concat()).expect("tray gallery css")
    }

    /// The labels of the gallery language.
    pub fn labels(&self) -> TrayLabels {
        if self.ru {
            TrayLabels::ru()
        } else {
            TrayLabels::en()
        }
    }

    /// A made-up HQ: three tasks running, two limits, five agents.
    pub fn model(&self) -> TrayModel {
        let ru = self.ru;
        let s = |r: &str, e: &str| if ru { r.to_string() } else { e.to_string() };
        TrayModel {
            title: "Hatchery".into(),
            status: TrayStatus::Live,
            tasks: vec![
                TrayTask::new("t1", s("Релиз uzor 1.5.33", "Release uzor 1.5.33"), TaskState::Running).side("scout"),
                TrayTask::new("t2", s("Виджеты в трее", "Tray widgets"), TaskState::Running).side("knyaz"),
                TrayTask::new("t3", s("Проверка захвата роутера", "Router capture check"), TaskState::Open).side(s("2 ч", "2 h")),
                TrayTask::new("t4", s("Документация HQ", "HQ docs"), TaskState::Done),
            ],
            counts: (2, 5, 12),
            meters: vec![
                Meter::new("5h", 0.34).note(s("сброс через 2 ч", "resets in 2 h")),
                Meter::new(s("неделя", "week"), 0.61).note(s("сброс в пн", "resets Mon")),
                Meter::new("codex", 0.12).note("5h"),
            ],
            agents: vec![
                TrayAgent::new("a1", "scout").sub(s("гоняет тесты", "running tests")).state(CritterState::Working).status(TrayStatus::Live),
                TrayAgent::new("a2", "knyaz").sub(s("пишет виджеты", "writing widgets")).state(CritterState::Thinking).status(TrayStatus::Live),
                TrayAgent::new("a3", "rev").sub(s("ждёт ответа", "waits for you")).state(CritterState::Attention).status(TrayStatus::Attention),
                TrayAgent::new("a4", "lead").sub(s("свободен", "idle")).family(CritterFamily::Agent),
                TrayAgent::new("a5", "box-7").sub(s("нода", "node")).family(CritterFamily::Node).state(CritterState::Off),
            ],
            foot: String::new(),
        }
    }

    fn vars(&self) -> &'static str {
        if self.dark {
            "--tg-fg: #f2f2f2; --tg-sub: #9a9a9a; --tg-btn: #232325; --tg-line: #333336"
        } else {
            "--tg-fg: #18181b; --tg-sub: #6b6b70; --tg-btn: #ffffff; --tg-line: #dcdce0"
        }
    }

    fn label(&self, w: TrayWidget) -> String {
        let l = self.labels();
        match w {
            TrayWidget::Tasks => l.tasks,
            TrayWidget::Limits => l.limits,
            TrayWidget::Agents => l.agents,
            TrayWidget::Combined => if self.ru { "Всё вместе".into() } else { "Combined".into() },
        }
    }

    /// The page: a bar (language, theme, the tray's pick), the flyout cards, the
    /// thumbnail cards and the icons. Cards carry ids `card:{key}` / `solid:{key}`.
    pub fn tree(&self, w: f64) -> El {
        let m = self.model();
        let l = self.labels();
        let flyout = TrayLook::flyout(self.dark);
        let solid = TrayLook::solid(self.dark);
        let ru = self.ru;
        let s = |r: &str, e: &str| if ru { r.to_string() } else { e.to_string() };
        El::new("div.tg-page").css(format!("{}; width: {w}px", self.vars()))
            .child(
                El::new("div.tg-bar")
                    .child(El::new("div.tg-h1").text(s("Виджеты трея", "Tray widgets")))
                    .children(TrayWidget::ALL.iter().map(|k| {
                        El::new(if *k == self.pick { "button.tg-btn.on" } else { "button.tg-btn" }).id(format!("pick:{}", k.key())).tab().text(self.label(*k))
                    }))
                    .child(El::new("button.tg-btn").id("lang").tab().text(if self.ru { "EN" } else { "RU" }))
                    .child(El::new("button.tg-btn").id("theme").tab().text(if self.dark { s("Светлая", "Light") } else { s("Тёмная", "Dark") })),
            )
            .child(El::new("div.tg-sec").text(s("Окно над треем · стекло", "Tray flyout · glass")))
            .child(El::new("div.tg-row").children(TrayWidget::ALL.iter().map(|k| {
                El::new("div.tg-cell").child(tray::tray_widget(*k, &m, &l, &flyout).id(format!("card:{}", k.key()))).child(El::new("div.tg-cap").text(self.label(*k)))
            })))
            .child(El::new("div.tg-sec").text(s("Превью на панели задач · непрозрачные", "Taskbar thumbnail · opaque")))
            .child(El::new("div.tg-row").children([TrayWidget::Tasks, TrayWidget::Combined].iter().map(|k| {
                El::new("div.tg-cell").child(tray::tray_widget(*k, &m, &l, &solid).id(format!("solid:{}", k.key()))).child(El::new("div.tg-cap").text(self.label(*k)))
            })))
            .child(El::new("div.tg-sec").text(s("Значки: кольцо лимита и точка статуса", "Icons: limit ring and status dot")))
            .child(
                El::new("div.tg-row")
                    .child(El::new("span.tg-icon").slot("@icon:ring:32"))
                    .child(El::new("span.tg-icon.big").slot("@icon:ring:64"))
                    .child(El::new("span.tg-icon").slot("@icon:badge:live"))
                    .child(El::new("span.tg-icon").slot("@icon:badge:attention"))
                    .child(El::new("span.tg-icon").slot("@icon:badge:error")),
            )
    }

    /// The window background: a soft wallpaper so the glass shows.
    pub fn backdrop(&self, ctx: &mut dyn RenderContext, r: Rect) {
        ctx.set_fill_color(if self.dark { "#101012" } else { "#e9e9ee" });
        ctx.fill_rect(r.x, r.y, r.width, r.height);
        let blobs: [(f64, f64, f64, &str, &str); 4] = [
            (0.18, 0.30, 0.22, "#3b4a6b", "#a9c1ff"),
            (0.55, 0.25, 0.18, "#5a3d5c", "#f3b6d8"),
            (0.80, 0.55, 0.20, "#2f5546", "#a8e6c9"),
            (0.35, 0.75, 0.16, "#5b4a2e", "#f5d9a0"),
        ];
        for (x, y, rad, dk, lt) in blobs {
            ctx.set_fill_color(if self.dark { dk } else { lt });
            ctx.begin_path();
            ctx.arc(r.x + r.width * x, r.y + r.height * y, r.width.min(r.height) * rad * 1.6, 0.0, std::f64::consts::TAU);
            ctx.fill();
        }
    }

    /// Paint one slot of the page: critters of the cards and the icons.
    pub fn paint_slot(&self, ctx: &mut dyn RenderContext, key: &str, r: Rect, t: f64) {
        if let Some(rest) = key.strip_prefix("@icon:") {
            let icon = match rest {
                "ring:32" => tray::ring_icon(64, 0.61, self.dark, TrayStatus::Live),
                "ring:64" => tray::ring_icon(128, 0.34, self.dark, TrayStatus::Attention),
                b => {
                    let st = match b {
                        "badge:live" => TrayStatus::Live,
                        "badge:attention" => TrayStatus::Attention,
                        _ => TrayStatus::Error,
                    };
                    tray::badge(&demo_logo(64, self.dark), st.rgba())
                }
            };
            if let Some(p) = ctx.image_painter() {
                p.draw_image_rgba(&icon.pixels, icon.width, icon.height, r.x, r.y, r.width, r.height);
            }
            return;
        }
        tray::paint_slot(ctx, key, r, &self.model(), t, false);
    }

    /// A click on `id`.
    pub fn click(&mut self, id: &str) {
        match id {
            "lang" => self.ru = !self.ru,
            "theme" => self.dark = !self.dark,
            k => {
                if let Some(w) = k.strip_prefix("pick:").and_then(TrayWidget::from_key) {
                    self.pick = w;
                }
            }
        }
    }
}

/// A stand-in app icon: a monochrome rounded square with a ring (the badge demo).
pub fn demo_logo(px: u32, dark: bool) -> uzor::RgbaIcon {
    let mut i = tray::ring_icon(px, 1.0, dark, TrayStatus::Idle);
    // Clear the centre dot: a plain ring reads as a logo.
    let c = px as f64 / 2.0;
    for y in 0..px {
        for x in 0..px {
            let d = ((x as f64 + 0.5 - c).powi(2) + (y as f64 + 0.5 - c).powi(2)).sqrt();
            if d < px as f64 * 0.2 {
                let k = ((y * px + x) * 4) as usize;
                i.pixels[k + 3] = 0;
            }
        }
    }
    i
}

/// The gallery as a live page.
pub struct TrayPage {
    /// The gallery state.
    pub g: TrayGallery,
    sheet: uzor::ui::style::Sheet,
    /// Last click, for tests.
    pub last_click: Option<String>,
    /// Clickable ids and where they sit, from the last paint.
    pub hits: Vec<(String, Rect)>,
    /// The pick changed since `on_change` last ran.
    pub changed: bool,
    /// Called after a change (the native bin feeds its tray widget from here).
    pub on_change: Option<Box<dyn FnMut(&TrayGallery)>>,
}

impl Default for TrayPage {
    fn default() -> Self {
        TrayPage {
            g: TrayGallery::default(),
            sheet: TrayGallery::sheet(),
            last_click: None,
            hits: Vec::new(),
            changed: true,
            on_change: None,
        }
    }
}

impl crate::page_app::Page for TrayPage {
    fn title(&self) -> &str {
        "Tray widgets"
    }
    fn size(&self) -> (u32, u32) {
        (1400, 760)
    }
    fn paint(&mut self, cx: &mut uzor_framework::PanelCx<'_, crate::page_app::PageSpec>, now_ms: u64) {
        use uzor_framework::widgets::kit;
        let area = cx.rect;
        self.g.backdrop(cx.render, area);
        let resp = kit::dom(area, &self.sheet, self.g.tree(area.width)).build(cx);
        let t = now_ms as f64 / 1000.0;
        for (key, r) in resp.dom.slots() {
            self.g.paint_slot(cx.render, key, r, t);
        }
        self.hits = resp.dom.hits().map(|(id, r)| (id.to_string(), r)).collect();
        if let Some(c) = resp.clicked.clone() {
            self.g.click(&c);
            self.last_click = Some(c);
            self.changed = true;
        }
        if self.changed {
            if let Some(f) = self.on_change.as_mut() {
                f(&self.g);
                self.changed = false;
            }
        }
        // Working critters move.
        cx.out.request_frame();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page_app::testing::{click_at, open, Host};

    fn click(h: &mut Host<TrayPage>, id: &str) {
        let hits: Vec<(String, Rect)> = h.runtime().app().page.hits.clone();
        let r = hits.iter().find(|(i, _)| i == id).map(|(_, r)| *r).unwrap_or_else(|| panic!("no hit {id}: {hits:?}"));
        click_at(h, r.x + r.width / 2.0, r.y + r.height / 2.0);
    }

    #[test]
    fn picks_language_and_theme() {
        let mut h = open(TrayPage::default());
        click(&mut h, "pick:agents");
        assert_eq!(h.runtime().app().page.g.pick, TrayWidget::Agents);
        click(&mut h, "lang");
        assert!(!h.runtime().app().page.g.ru);
        click(&mut h, "theme");
        assert!(!h.runtime().app().page.g.dark);
    }

    #[test]
    fn every_card_is_on_the_page() {
        let h = open(TrayPage::default());
        let hits = &h.runtime().app().page.hits;
        for w in TrayWidget::ALL {
            let r = hits.iter().find(|(i, _)| *i == format!("card:{}", w.key())).map(|(_, r)| *r).expect("card");
            assert_eq!((r.width, r.height), w.size());
        }
    }
}
