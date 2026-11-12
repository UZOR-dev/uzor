//! The mobile shell widgets ([`uzor::ui::mobile`]) as phone screens: the tab bar over a
//! chat list with swipe actions and a floating button, a pushed chat with the top bar
//! and the composer over the keyboard, the edge drawer, the right rail, the action
//! modal, the bottom sheet at its three stops, a toast, the segmented control and the
//! agent card. [`MobileGallery::tree`] is pure: the `mobile-gallery-frames` bin renders
//! it to PNGs, a kit panel ([`uzor_framework::widgets::kit::dom`]) shows it live, and the
//! tests below lay it out headless.

use uzor::ui::dom::El;
use uzor::ui::mobile::{self as m, icons, Detent, Dot, DrawerItem, DrawerState, Kv, Row, SheetState, SwipeState, Tab};

/// The screens, in the concept's order.
pub const SCREENS: [&str; 15] = [
    "01-tab-bar-chat-list",
    "02-top-bar-back-chat",
    "03-composer-keyboard",
    "04-drawer-edge-swipe",
    "05-side-rail-right",
    "06a-fab",
    "06b-fab-modal",
    "07a-bottom-sheet-peek",
    "07b-bottom-sheet-half",
    "07c-bottom-sheet-full",
    "08-swipe-actions",
    "09-toast",
    "10-segmented-control",
    "11a-search-virtual-list",
    "11b-search-typed",
];

/// 500 made-up instruments for the long list: (ticker, market).
pub fn instruments() -> Vec<(String, &'static str)> {
    const BASES: [&str; 25] = ["BTC", "ETH", "SOL", "XRP", "ADA", "DOGE", "TON", "TRX", "DOT", "LINK", "AVAX", "MATIC", "LTC", "BCH", "ATOM", "NEAR", "APT", "ARB", "OP", "FIL", "ETC", "XLM", "UNI", "AAVE", "SUI"];
    const QUOTES: [(&str, &str); 20] = [("USDT", "спот"), ("USDC", "спот"), ("BTC", "спот"), ("ETH", "спот"), ("EUR", "спот"), ("RUB", "спот"), ("TRY", "спот"), ("BRL", "спот"), ("FDUSD", "спот"), ("DAI", "спот"), ("USDT-PERP", "фьючерс"), ("USDC-PERP", "фьючерс"), ("USD-QTR", "фьючерс"), ("USD-BI", "фьючерс"), ("USDT-1H", "опцион"), ("USDT-1D", "опцион"), ("USDT-1W", "опцион"), ("USDT-1M", "опцион"), ("BUSD", "спот"), ("JPY", "спот")];
    BASES.iter().flat_map(|b| QUOTES.iter().filter(move |q| q.0 != *b).map(move |(q, k)| (format!("{b}/{q}"), *k))).collect()
}

/// Made-up chats: (avatar, name, dot, time, last line, unread).
pub const CHATS: [(&str, &str, Dot, &str, &str, u32); 8] = [
    ("Г", "Grok · основной", Dot::Ok, "13:48", "Готово: ветка mobile-gaps собрана для Android и iOS", 3),
    ("Х", "Hatchery", Dot::Ok, "13:31", "Схема ключей: Secure Enclave + Keystore", 0),
    ("К", "knyaz-tishini", Dot::None, "12:05", "ок, релиз вечером", 0),
    ("R", "edge-relay", Dot::Err, "11:40", "Проверка TLS не прошла: сертификат истёк", 1),
    ("И", "Исследователь", Dot::Warn, "вчера", "Ждёт подтверждения: открыть сайт в браузере", 0),
    ("Д", "Дизайн HQ", Dot::None, "вчера", "Концепты мобильных виджетов готовы", 0),
    ("Н", "Ночной дайджест", Dot::Ok, "пн", "Сводка за неделю: 14 задач закрыто", 0),
    ("A", "Агент сборки", Dot::None, "вс", "cargo check: 0 ошибок", 0),
];

/// Row height of the long list.
pub const LIST_ROW: f64 = 56.0;

/// Gallery state a host (or the frames bin) changes between frames.
#[derive(Clone, Debug, Default)]
pub struct MobileGallery {
    pub screen: usize,
    pub sheet: SheetState,
    pub drawer: DrawerState,
    pub swipe: SwipeState,
    /// Viewport.
    pub w: f64,
    pub h: f64,
    /// Soft keyboard height (the composer screen).
    pub keyboard: f64,
    /// Bottom safe area.
    pub safe_bottom: f64,
    /// The long list's search query and scroll offset.
    pub query: String,
    pub list_scroll: f64,
}

impl MobileGallery {
    /// A 390×844 phone with a 34px home indicator, on `screen`.
    pub fn phone(screen: usize) -> Self {
        let mut g = MobileGallery { screen, w: 390.0, h: 844.0, safe_bottom: 34.0, ..Default::default() };
        match SCREENS.get(screen).copied().unwrap_or("") {
            "03-composer-keyboard" => g.keyboard = 336.0,
            "04-drawer-edge-swipe" => {
                g.drawer.set(true);
                while g.drawer.step(1.0 / 60.0, m::DRAWER_W) {}
            }
            "11a-search-virtual-list" => g.list_scroll = 200.0 * LIST_ROW,
            "11b-search-typed" => g.query = "eth usd".into(),
            "07a-bottom-sheet-peek" => g.sheet.open(Detent::Peek),
            "07b-bottom-sheet-half" => g.sheet.open(Detent::Half),
            "07c-bottom-sheet-full" => g.sheet.open(Detent::Full),
            "08-swipe-actions" => {
                g.swipe.drag_by(-234.0, 0.0, 234.0);
                g.swipe.release(0.0, 0.0, 234.0);
                while g.swipe.step(1.0 / 60.0) {}
            }
            _ => {}
        }
        g
    }

    /// The page sheet.
    pub fn sheet() -> uzor::ui::style::Sheet {
        m::sheet_with("").expect("mobile css")
    }

    fn tabs(&self, on: usize) -> El {
        let tabs = [
            Tab::new(icons::CHAT, "Чаты").badge(4),
            Tab::new(icons::TASKS, "Задачи"),
            Tab::new(icons::MACHINE, "Машины").dot(Dot::Err),
            Tab::new(icons::VAULT, "Волт"),
            Tab::new(icons::GEAR, "Настройки"),
        ];
        m::tab_bar("tabs", &tabs, on)
    }

    fn chat_list(&self, swipe_first: bool) -> El {
        El::new("div.m-list").children(CHATS.iter().enumerate().map(|(i, (a, n, d, t, s, c))| {
            let r = Row { avatar: a.to_string(), dot: *d, title: n.to_string(), time: t.to_string(), sub: s.to_string(), count: *c, muted: i >= 4, trail: (i < 2 && *c == 0).then_some(icons::PIN), ..Default::default() };
            let el = m::row(&format!("chat:{i}"), &r);
            if swipe_first && i == 1 {
                m::swipe_row("sw", el, &[("unread", icons::READ, "Прочит.", "")], &[("pin", icons::PIN, "Закрепить", ""), ("mute", icons::BELL_OFF, "Без звука", "alt"), ("del", icons::TRASH, "Удалить", "strong")], &self.swipe)
            } else {
                el
            }
        }))
    }

    fn chats_screen(&self, swipe: bool) -> Vec<El> {
        vec![
            m::large_title("top", "Чаты", &[("new", icons::EDIT, "Новый чат")], Some("Поиск"), false),
            El::new("div.m-content").child(self.chat_list(swipe)),
            self.tabs(0),
        ]
    }

    fn chat_screen(&self) -> Vec<El> {
        let msgs = El::new("div.m-content").css("padding: 10px 12px; gap: 6px").children([
            El::new("div.k-msg.me").child(El::new("div.k-body").text("Закрой мобильные пробелы в uzor: сворачивание, клавиатура, свайпы")),
            El::new("div.k-msg.them").child(El::new("div.k-body").text("Беру. Начну с жизненного цикла: на Android поверхность пересоздаётся после возврата из фона.")),
            El::new("div.k-msg.them").child(El::new("div.k-body").text("Готово: прокрутка одним пальцем, инерция, долгое нажатие, отступы под вырез и клавиатуру.")),
            El::new("div.k-msg.me").child(El::new("div.k-body").text("Отлично. Покажи концепты виджетов")),
        ]);
        let pad = self.keyboard.max(self.safe_bottom);
        vec![
            m::top_bar("nav", "Чаты", "Grok · основной", Some((Dot::Ok, "работает · mobile-gaps")), &[("more", icons::MORE, "Ещё")]),
            msgs,
            m::composer("comp", m::ComposerField::Input(uzor::ui::dom::Edit::new(if self.keyboard > 0.0 { "Покажи ещё нижний лист" } else { "" }).placeholder("Сообщение")), self.keyboard > 0.0, ("Вложение", "Сообщение", "Отправить")),
            El::new("div").css(format!("height: {pad}px; flex-shrink: 0; background: var(--m-bar)")),
        ]
    }

    fn tasks_screen(&self) -> Vec<El> {
        let tasks = ["Мобильные пробелы uzor", "Ключи: кроссплатформенная схема", "Концепты мобильных виджетов", "edge-relay: продлить TLS", "Шаблон Kotlin-хоста", "Шаблон XcodeGen", "Релиз 1.5.30"];
        let dots = [Dot::Ok, Dot::Warn, Dot::Ok, Dot::Err, Dot::Idle, Dot::Idle, Dot::Ok];
        vec![
            m::large_title("top", "Задачи", &[], Some("Поиск"), false),
            El::new("div.m-content").child(El::new("div.m-list").children(tasks.iter().zip(dots).enumerate().map(|(i, (t, d))| {
                let r = Row { avatar: "-".into(), dense: true, title: t.to_string(), trail: Some(icons::CHEV), dot: d, ..Default::default() };
                m::row(&format!("task:{i}"), &r).child(El::new("span").css("order: -1")).css("gap: 10px")
            }))),
            self.tabs(1),
        ]
    }

    fn machines_screen(&self) -> Vec<El> {
        let row = |i: usize, name: &str, sub: &str, side: &str, d: Dot| {
            let r = Row { avatar: name.chars().next().unwrap_or('?').to_uppercase().to_string(), square: true, dot: d, title: name.into(), sub: sub.into(), time: side.into(), dense: true, ..Default::default() };
            m::row(&format!("mach:{i}"), &r)
        };
        vec![
            m::large_title("top", "Машины", &[("add", icons::PLUS, "Добавить")], None, false),
            El::new("div").css("padding: 0 16px 8px; flex-shrink: 0").child(m::segmented("seg", &[("Все", Some(5)), ("В сети", Some(3)), ("Проблемы", Some(1))], 0)),
            El::new("div.m-content").children([
                m::section("РЕЛЕ"),
                row(0, "edge-1", "relay · 2 узла", "в сети", Dot::Ok),
                row(1, "edge-relay", "TLS истёк", "ошибка", Dot::Err),
                m::section("УЗЛЫ"),
                row(2, "box", "linux · node", "в сети", Dot::Ok),
                row(3, "mac-studio", "macOS · node", "в сети", Dot::Ok),
                row(4, "old-nuc", "linux · node", "не в сети", Dot::Idle),
            ]),
            self.tabs(2),
        ]
    }

    fn search_screen(&self) -> Vec<El> {
        let all = instruments();
        let labels: Vec<&str> = all.iter().map(|i| i.0.as_str()).collect();
        let hits = m::search_filter(&labels, &self.query);
        let mut edit = uzor::ui::dom::Edit::new(self.query.clone()).placeholder("Тикер или рынок");
        edit.caret = (!self.query.is_empty()).then_some(self.query.chars().count());
        // The list's height on a 390×844 phone (a host takes it from the last frame).
        let vh = self.h - 47.0 - 44.0 - 54.0 - (6.0 + 49.0 + self.safe_bottom + 1.0);
        let v = m::VirtualList::new(LIST_ROW);
        let list = v.el("list", hits.len(), self.list_scroll, vh, Some("Ничего не найдено"), |i| {
            let (t, k) = &all[hits[i]];
            let r = Row { avatar: t.chars().next().unwrap_or('?').to_string(), square: true, title: t.clone(), sub: format!("{k} · #{}", hits[i] + 1), time: format!("{}", 1000 + hits[i] * 37 % 9000), dense: true, ..Default::default() };
            m::row(&format!("inst:{}", hits[i]), &r)
        });
        vec![
            m::top_bar("nav", "Рынки", &format!("Инструменты · {}", hits.len()), None, &[]),
            m::search_bar("q", edit, "Поиск инструмента", Some("Отмена")),
            list,
            self.tabs(1),
        ]
    }

    fn card_body(&self) -> (El, El) {
        let head = El::new("div.m-sh").child(El::new("div.m-av.sq").text("Г")).child(El::new("div.m-sh-t").text("Grok · основной"));
        let body = El::new("div").css("display: flex; flex-direction: column").children([
            m::quick_actions("qa", &[("chat", icons::CHAT, "Чат"), ("term", icons::TERM, "Терминал"), ("tasks", icons::TASKS, "Задачи"), ("stop", icons::STOP, "Стоп")]),
            m::kv_group(&[
                Kv::Text("Модель".into(), "grok-4".into(), false),
                Kv::Text("Машина".into(), "box · eu-1".into(), false),
                Kv::Text("Ветка".into(), "mobile-gaps".into(), true),
                Kv::Text("Задачи".into(), "3 активны".into(), false),
            ]),
            m::kv_group(&[Kv::Switch("notify".into(), "Уведомления".into(), true), Kv::Switch("ask".into(), "Спрашивать перед отправкой".into(), true)]),
            El::new("div").css("display: flex; gap: 10px; padding: 0 16px").child(m::btn("pause", "Пауза", "", true)).child(m::btn("open", "Открыть чат", "pri", true)),
        ]);
        (head, body)
    }

    /// The tree of the current screen in a `w`×`h` viewport.
    pub fn tree(&self) -> El {
        let name = SCREENS.get(self.screen).copied().unwrap_or(SCREENS[0]);
        let tab_h = 6.0 + 49.0 + self.safe_bottom + 1.0;
        let mut overlays: Vec<El> = Vec::new();
        let body: Vec<El> = match name {
            "02-top-bar-back-chat" | "03-composer-keyboard" => self.chat_screen(),
            "04-drawer-edge-swipe" => {
                let items = [
                    DrawerItem::Item("chats".into(), icons::CHAT, "Чаты".into(), "4".into(), true),
                    DrawerItem::Item("tasks".into(), icons::TASKS, "Задачи".into(), "12".into(), false),
                    DrawerItem::Item("machines".into(), icons::MACHINE, "Машины".into(), String::new(), false),
                    DrawerItem::Item("vault".into(), icons::VAULT, "Волт".into(), String::new(), false),
                    DrawerItem::Item("settings".into(), icons::GEAR, "Настройки".into(), String::new(), false),
                    DrawerItem::Section("ПРОСТРАНСТВА".into()),
                    DrawerItem::Item("sp:0".into(), icons::USER, "Личное".into(), String::new(), false),
                    DrawerItem::Item("sp:1".into(), icons::SPARK, "uzor".into(), String::new(), false),
                    DrawerItem::Fill,
                    DrawerItem::Item("sp:new".into(), icons::PLUS, "Новое пространство".into(), String::new(), false),
                ];
                overlays.extend(m::drawer("dr", self.drawer.progress(), Some(("HQ", "ЛАВР", Dot::Ok, "в сети · 3 машины")), &items));
                self.chats_screen(false)
            }
            "05-side-rail-right" => {
                overlays.push(m::side_rail("rail", &[(icons::CHAT, "Чаты", Dot::None), (icons::TASKS, "Задачи", Dot::None), (icons::MACHINE, "Машины", Dot::Err), (icons::VAULT, "Волт", Dot::None), (icons::GEAR, "Настройки", Dot::None)], 0));
                vec![m::large_title("top", "Чаты", &[], Some("Поиск"), false).css("padding-right: 80px"), El::new("div.m-content").css("padding-right: 64px").child(self.chat_list(false))]
            }
            "06a-fab" | "06b-fab-modal" => {
                if name == "06b-fab-modal" {
                    overlays.push(m::action_modal("new", "Создать", "Закрыть", &[("chat", icons::CHAT, "Новый чат", "с агентом"), ("task", icons::TASKS, "Задача", ""), ("machine", icons::MACHINE, "Добавить машину", "по SSH"), ("secret", icons::KEY, "Секрет в волт", ""), ("agent", icons::SPARK, "Новый агент", "")]));
                } else {
                    overlays.push(m::fab("fab", icons::PLUS, "Создать", tab_h + 16.0));
                }
                self.tasks_screen()
            }
            "07a-bottom-sheet-peek" | "07b-bottom-sheet-half" | "07c-bottom-sheet-full" => {
                let (head, body) = self.card_body();
                overlays.extend(m::bottom_sheet("card", &self.sheet, self.h, Some(head), body));
                self.chats_screen(false)
            }
            "08-swipe-actions" => self.chats_screen(true),
            "09-toast" => {
                overlays.push(m::toast("toast", "Чат «Hatchery» в архиве", Some("Отменить"), tab_h + 12.0));
                self.chats_screen(false)
            }
            "10-segmented-control" => self.machines_screen(),
            "11a-search-virtual-list" | "11b-search-typed" => self.search_screen(),
            _ => self.chats_screen(false),
        };
        m::screen(true, body).css(format!("height: {}px", self.h)).children(overlays)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uzor::ui::style::State;
    use uzor::Rect;

    #[test]
    fn every_screen_lays_out_on_a_phone() {
        let env = uzor::ui::media::MediaEnv::touch(390.0, 844.0, uzor::ui::media::Edges::new(47.0, 0.0, 34.0, 0.0));
        let sheet = uzor::ui::style::Sheet::parse_with(&[uzor::ui::domkit::KIT_CSS, uzor::ui::domkit::KIT_TOUCH_CSS, m::MOBILE_CSS].concat(), &env).unwrap();
        let mut ctx = uzor::testing::RecordingRenderContext::new();
        for i in 0..SCREENS.len() {
            let g = MobileGallery::phone(i);
            let d = uzor::ui::dom::lay_out_in(&mut ctx, &sheet, &g.tree(), 0.0, 0.0, 390.0, Some(Rect::new(0.0, 0.0, 390.0, 844.0)), &|_| State::NONE);
            assert!(d.hits().count() > 3, "{}", SCREENS[i]);
            match SCREENS[i] {
                "06a-fab" => assert!(d.rect("fab").is_some()),
                "06b-fab-modal" => assert!(d.rect("new:machine").is_some()),
                "07b-bottom-sheet-half" => assert!(d.rect("card:grab").is_some() && d.rect("qa:chat").is_some()),
                "08-swipe-actions" => assert!(d.rect("sw:act:del").is_some()),
                "10-segmented-control" => assert!(d.rect("seg:2").is_some()),
                "11a-search-virtual-list" => {
                    assert!(instruments().len() >= 480);
                    assert!(d.rect("inst:200").is_some() && d.rect("inst:0").is_none() && d.rect("q:clear").is_none());
                    assert!(d.hits().filter(|h| h.0.starts_with("inst:")).count() < 25, "only a screenful");
                }
                "11b-search-typed" => assert!(d.rect("q:clear").is_some() && d.rect("inst:20").is_some()),
                "01-tab-bar-chat-list" => assert!(d.rect("tabs:4").unwrap().y > 700.0),
                _ => {}
            }
        }
    }
}
