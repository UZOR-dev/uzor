use super::*;
use crate::ui::dom::lay_out_in;
use crate::ui::media::{Edges, MediaEnv};
use crate::ui::style::State;
use crate::Rect;

fn phone() -> MediaEnv {
    MediaEnv::touch(390.0, 844.0, Edges::new(47.0, 0.0, 34.0, 0.0))
}

fn sheet_for(env: &MediaEnv) -> Sheet {
    Sheet::parse_with(&[super::super::domkit::KIT_CSS, super::super::domkit::KIT_TOUCH_CSS, MOBILE_CSS].concat(), env).expect("MOBILE_CSS parses")
}

fn lay(root: &El, env: &MediaEnv) -> crate::ui::dom::Dom {
    let mut ctx = crate::testing::RecordingRenderContext::new();
    let sh = sheet_for(env);
    lay_out_in(&mut ctx, &sh, root, 0.0, 0.0, env.width, Some(Rect::new(0.0, 0.0, env.width, env.height)), &|_| State::NONE)
}

fn tabs() -> Vec<Tab> {
    vec![Tab::new(icons::CHAT, "Чаты").badge(4), Tab::new(icons::TASKS, "Задачи"), Tab::new(icons::MACHINE, "Машины").dot(Dot::Err), Tab::new(icons::VAULT, "Волт"), Tab::new(icons::GEAR, "Настройки")]
}

#[test]
fn sheet_parses_on_phone_and_desktop() {
    sheet_for(&phone());
    sheet_for(&MediaEnv::desktop(1280.0, 800.0));
    sheet_with("div.app { color: #fff }").expect("with app rules");
}

#[test]
fn only_status_dots_carry_colour() {
    // The status colours are used by the dot rules alone: no coloured pills or text.
    for var in ["var(--m-ok)", "var(--m-warn)", "var(--m-err)"] {
        for line in MOBILE_CSS.lines().filter(|l| l.contains(var)) {
            assert!(line.trim_start().starts_with("span.m-dot."), "{var} outside a dot: {line}");
        }
    }
    // Every other literal colour is a grey (r = g = b) or a black/white alpha.
    let mut rest = MOBILE_CSS;
    while let Some(i) = rest.find('#') {
        let hex: String = rest[i + 1..].chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        rest = &rest[i + 1..];
        if hex.len() == 6 && !["5fd08a", "d8b45a", "d07070"].contains(&hex.as_str()) {
            let (r, g, b) = (&hex[0..2], &hex[2..4], &hex[4..6]);
            assert!(r == g && g == b, "#{hex} is not a grey");
        }
    }
}

#[test]
fn tab_bar_fills_the_width_and_pads_the_home_indicator() {
    let env = phone();
    let root = screen(true, [El::new("div.m-content"), tab_bar("tabs", &tabs(), 0)]).css("height: 844px");
    let d = lay(&root, &env);
    let t0 = d.rect("tabs:0").unwrap();
    let t4 = d.rect("tabs:4").unwrap();
    assert!((t0.width - t4.width).abs() < 0.5, "equal tabs");
    assert!(t4.x + t4.width > 380.0);
    // 6px top padding + 49px tabs + 34px safe area + 1px border = bar height 90 at the bottom.
    assert!((t0.y + t0.height - (844.0 - 34.0)).abs() < 1.0, "tabs end above the home indicator: {t0:?}");
    assert!(t0.height >= 44.0, "touch-sized");
}

#[test]
fn top_bar_back_and_title() {
    let d = lay(&top_bar("nav", "Чаты", "Grok · основной", Some((Dot::Ok, "работает")), &[("more", icons::MORE, "Ещё")]), &phone());
    let back = d.rect("nav:back").unwrap();
    assert!(back.height >= 44.0 && back.x < 20.0);
    let more = d.rect("nav:more").unwrap();
    assert!(more.x + more.width > 370.0 && more.width >= 44.0);
}

#[test]
fn rows_and_swipe_actions() {
    let mut r = Row::new("Hatchery");
    r.avatar = "Х".into();
    r.dot = Dot::Ok;
    r.sub = "Схема ключей".into();
    r.time = "13:31".into();
    r.count = 3;
    let mut st = SwipeState::default();
    let right = [("pin", icons::PIN, "Закрепить", ""), ("read", icons::READ, "Прочитано", "alt"), ("del", icons::TRASH, "Удалить", "strong")];
    // Closed: no actions built.
    let d = lay(&swipe_row("s", row("chat:1", &r), &[], &right, &st), &phone());
    assert!(d.rect("s:act:del").is_none());
    let home = d.rect("chat:1").unwrap();
    assert!(home.height >= 68.0 && home.x == 0.0);
    // Dragged left past a third: the actions show and it rests open at their width.
    st.drag_by(-120.0, 0.0, 3.0 * SWIPE_ACT_W);
    assert_eq!(st.release(0.0, 0.0, 3.0 * SWIPE_ACT_W), -3.0 * SWIPE_ACT_W);
    while st.step(1.0 / 60.0) {}
    let d = lay(&swipe_row("s", row("chat:1", &r), &[], &right, &st), &phone());
    let moved = d.rect("chat:1").unwrap();
    assert_eq!(moved.x, -234.0);
    let del = d.rect("s:act:del").unwrap();
    assert!((del.x + del.width - 390.0).abs() < 0.5 && del.width == SWIPE_ACT_W);
    // A short drag closes again; no left actions means no rightward motion.
    let mut s2 = SwipeState::default();
    s2.drag_by(-20.0, 0.0, 234.0);
    assert_eq!(s2.release(0.0, 0.0, 234.0), 0.0);
    s2.drag_by(50.0, 0.0, 234.0);
    assert_eq!(s2.offset, 0.0);
    // A flick opens even when short.
    let mut s3 = SwipeState::default();
    s3.drag_by(-30.0, 0.0, 234.0);
    assert_eq!(s3.release(-900.0, 0.0, 234.0), -234.0);
}

#[test]
fn bottom_sheet_snaps_to_stops() {
    let vh = 844.0;
    let mut st = SheetState { peek: 120.0, ..SheetState::default() };
    st.open(Detent::Peek);
    assert_eq!(st.height(vh), 120.0);
    // Dragged up to ~400 and let go slowly: nearest is half (422).
    st.drag_by(-280.0, vh);
    assert!(st.dragging());
    assert_eq!(st.release(0.0, vh), Detent::Half);
    while st.step(1.0 / 60.0, vh) {}
    assert_eq!(st.height(vh), 422.0);
    // A flick up from half goes full, a flick down from full goes half, then peek, then closed.
    st.drag_by(-10.0, vh);
    assert_eq!(st.release(-1200.0, vh), Detent::Full);
    while st.step(1.0 / 60.0, vh) {}
    assert_eq!(st.height(vh), vh - 54.0);
    for want in [Detent::Half, Detent::Peek, Detent::Closed] {
        st.drag_by(10.0, vh);
        assert_eq!(st.release(1500.0, vh), want);
        while st.step(1.0 / 60.0, vh) {}
    }
    assert_eq!(st.height(vh), 0.0);
    // Not closable: peek is the floor.
    let mut nc = SheetState { closable: false, ..SheetState::default() };
    nc.open(Detent::Peek);
    nc.drag_by(100.0, vh);
    assert_eq!(nc.release(2000.0, vh), Detent::Peek);
    // Past full the drag resists.
    let mut f = SheetState::default();
    f.open(Detent::Full);
    f.drag_by(-100.0, vh);
    assert!(f.height(vh) < vh - 54.0 + 40.0);
}

#[test]
fn bottom_sheet_lays_out_at_its_height_with_a_scrim_over_half() {
    let env = phone();
    let mut st = SheetState::default();
    st.open(Detent::Half);
    let el = bottom_sheet("card", &st, 844.0, Some(El::new("div.m-sh").child(El::new("div.m-sh-t").text("Grok"))), kv_group(&[Kv::Text("Модель".into(), "grok-4".into(), false)])).unwrap();
    let d = lay(&el, &env);
    let g = d.rect("card:grab").unwrap();
    assert!((g.y - 422.0 - 8.0).abs() < 2.0, "grab at the sheet top: {g:?}");
    assert!(d.rect("card:scrim").is_some());
    st.open(Detent::Peek);
    let d = lay(&bottom_sheet("card", &st, 844.0, None, El::new("div")).unwrap(), &env);
    assert!(d.rect("card:scrim").is_none(), "a peeking sheet does not dim");
    st.open(Detent::Closed);
    assert!(bottom_sheet("card", &st, 844.0, None, El::new("div")).is_none());
}

#[test]
fn drawer_follows_the_edge_pan_and_settles() {
    let mut d = DrawerState::default();
    assert!(drawer("dr", d.progress(), None, &[]).is_none());
    d.drag_by(90.0, DRAWER_W);
    assert!((d.progress() - 0.3).abs() < 1e-9);
    assert!(!d.release(0.0), "under half closes");
    d.drag_by(100.0, DRAWER_W);
    assert!(d.release(800.0), "a flick opens");
    while d.step(1.0 / 60.0, DRAWER_W) {}
    assert_eq!(d.progress(), 1.0);
    let items = [DrawerItem::Item("chats".into(), icons::CHAT, "Чаты".into(), "4".into(), true), DrawerItem::Section("Пространства".into()), DrawerItem::Fill, DrawerItem::Item("new".into(), icons::PLUS, "Новое пространство".into(), String::new(), false)];
    let el = drawer("dr", d.progress(), Some(("Л", "ЛАВР", Dot::Ok, "в сети")), &items).unwrap();
    let dom = lay(&el, &phone());
    let chats = dom.rect("dr:chats").unwrap();
    assert!(chats.x >= 12.0 && chats.x + chats.width <= DRAWER_W);
    let new = dom.rect("dr:new").unwrap();
    assert!(new.y > 700.0, "Fill pushes the last item down: {new:?}");
    assert!(dom.rect("dr:scrim").is_some());
    // Half open: the panel is half off-screen.
    let half = drawer("dr", 0.5, None, &items).unwrap();
    let dom = lay(&half, &phone());
    let x = dom.rect("dr:chats").unwrap().x;
    assert!((x - (chats.x - 150.0)).abs() < 1.0, "{x} vs {}", chats.x);
}

#[test]
fn fab_modal_toast_segmented_rail() {
    let env = phone();
    let d = lay(&fab("fab", icons::PLUS, "Создать", 106.0), &env);
    let f = d.rect("fab").unwrap();
    assert_eq!((f.width, f.y + f.height, f.x + f.width), (58.0, 844.0 - 106.0, 390.0 - 18.0));
    let d = lay(&action_modal("new", "Создать", "Закрыть", &[("chat", icons::CHAT, "Новый чат", "с агентом"), ("task", icons::TASKS, "Задача", "")]), &env);
    let chat = d.rect("new:chat").unwrap();
    let task = d.rect("new:task").unwrap();
    assert!(task.y > chat.y && task.y + task.height <= 844.0 - 34.0 + 0.5);
    assert!(d.rect("new:scrim").unwrap().height == 844.0);
    let d = lay(&toast("t", "Чат в архиве", Some("Отменить"), 100.0), &env);
    assert!(d.rect("t:act").is_some());
    let d = lay(&segmented("seg", &[("Все", Some(5)), ("В сети", Some(3)), ("Проблемы", Some(1))], 0), &env);
    let s0 = d.rect("seg:0").unwrap();
    let s2 = d.rect("seg:2").unwrap();
    assert!((s0.width - s2.width).abs() < 0.5);
    let d = lay(&side_rail("rail", &[(icons::CHAT, "Чаты", Dot::None), (icons::MACHINE, "Машины", Dot::Err)], 0), &env);
    let r1 = d.rect("rail:1").unwrap();
    assert!(r1.x >= 390.0 - 64.0 && r1.y > 47.0);
}

#[test]
fn composer_rides_above_the_keyboard() {
    assert_eq!(composer_bottom(0.0, 844.0, 0.0, 34.0), 810.0);
    assert_eq!(composer_bottom(0.0, 844.0, 336.0, 34.0), 508.0);
    let d = lay(&composer("c", ComposerField::Slot(22.0), false, ("Вложение", "Сообщение", "Голос")), &phone());
    for k in ["c:plus", "c:field", "c:send"] {
        assert!(d.rect(k).is_some(), "{k}");
    }
    assert!(d.slots().any(|(k, _)| k == "@c:field"));
    let d = lay(&composer("c", ComposerField::Input(Edit::new("").placeholder("Сообщение")), true, ("+", "msg", "send")), &phone());
    assert!(d.rect("c:field").unwrap().height >= 40.0);
}

#[test]
fn reveal_above_moves_a_field_out_from_under_the_keyboard() {
    // Viewport 0..844, keyboard 336: visible to 508 (minus 8 margin = 500).
    assert_eq!(reveal_above(600.0, 644.0, 0.0, 844.0, 336.0, 8.0), 144.0);
    assert_eq!(reveal_above(100.0, 144.0, 0.0, 844.0, 336.0, 8.0), 0.0);
    // Above the top: scroll down (negative).
    assert_eq!(reveal_above(-30.0, 14.0, 0.0, 844.0, 0.0, 8.0), -38.0);
    // Taller than the space left: its top stops at the margin.
    assert_eq!(reveal_above(300.0, 900.0, 0.0, 844.0, 336.0, 8.0), 292.0);
}

#[test]
fn agent_card_pieces() {
    let env = phone();
    let card = El::new("div")
        .child(hero("ag", "Grok · основной", Dot::Ok, "работает · 38 мин"))
        .child(quick_actions("qa", &[("chat", icons::CHAT, "Чат"), ("term", icons::TERM, "Терминал"), ("tasks", icons::TASKS, "Задачи"), ("stop", icons::STOP, "Стоп")]))
        .child(kv_group(&[Kv::Text("Ключ".into(), "ed25519:7c3a…".into(), true), Kv::Switch("notify".into(), "Уведомления".into(), true), Kv::Link("space".into(), "Пространство".into(), "uzor".into())]));
    let d = lay(&card, &env);
    assert!(d.slots().any(|(k, r)| k == "@ag:critter" && r.width >= 90.0));
    let q = d.rect("qa:chat").unwrap();
    let s = d.rect("qa:stop").unwrap();
    assert!((q.width - s.width).abs() < 0.5 && q.height == 68.0);
    assert!(d.rect("notify").unwrap().height >= 48.0);
}

#[test]
fn viewport_units_and_safe_area_reach_the_widgets() {
    let env = phone();
    let sh = Sheet::parse_with("div.full { height: 100vh; width: 50vw; padding-top: env(safe-area-inset-top) }", &env).unwrap();
    let e = sh.el("div.full");
    assert_eq!((e.height, e.width), (Some(844.0), Some(195.0)));
    assert_eq!(e.padding.map(|p| p[0]), Some(47.0));
}

#[test]
fn approach_lands() {
    let mut x = 0.0;
    let mut n = 0;
    while x != 100.0 {
        x = approach(x, 100.0, 1.0 / 60.0, 18.0);
        n += 1;
        assert!(n < 120);
    }
    assert!(n > 5, "eases, does not jump");
}

#[test]
fn drawer_avatar_paints_inside_the_drawer() {
    // The avatar is `position: relative` (for its dot); the drawer's z-index makes it a
    // stacking context, so the avatar paints over the drawer, not under it.
    let el = drawer("dr", 1.0, Some(("Л", "ЛАВР", Dot::Ok, "в сети")), &[]).unwrap();
    let dom = lay(&el, &phone());
    let at = |f: &dyn Fn(&crate::ui::dom::Placed) -> bool| dom.boxes.iter().position(|b| f(b)).unwrap();
    let av = at(&|b| b.text.as_deref() == Some("Л"));
    let panel = at(&|b| b.style.backdrop_blur.is_some());
    let scrim = at(&|b| b.id.as_deref() == Some("dr:scrim"));
    assert!(scrim < panel && panel < av, "scrim {scrim}, panel {panel}, avatar {av}");
}

// ---- virtual list + search bar ----

fn items() -> Vec<String> {
    let mut v: Vec<String> = (0..480).map(|i| format!("Instrument {i:03}")).collect();
    v.extend(["RSI_14", "rsi-divergence", "Moving average", "Индекс относительной силы", "Объём", "BTC/USDT", "ETH/USDT", "Bollinger bands", "Stoch RSI", "MACD", "Ichimoku", "VWAP", "ATR", "OBV", "Pivot points", "Parabolic SAR", "Супертренд", "Keltner", "Donchian", "CCI"].map(String::from));
    v
}

#[test]
fn virtual_list_ranges_are_the_visible_rows_plus_overscan() {
    let v = VirtualList::new(50.0).overscan(2);
    assert_eq!(v.content_height(500), 25_000.0);
    assert_eq!(v.max_scroll(500, 600.0), 24_400.0);
    assert_eq!(v.visible(500, 0.0, 600.0), 0..12);
    assert_eq!(v.range(500, 0.0, 600.0), 0..14);
    assert_eq!(v.visible(500, 125.0, 600.0), 2..15, "partly visible rows count");
    assert_eq!(v.range(500, 125.0, 600.0), 0..17);
    assert_eq!(v.range(500, 1e9, 600.0), 486..500, "clamped to the end");
    assert_eq!(v.range(500, -40.0, 600.0), 0..14, "and to the start");
    assert_eq!(v.range(500, f64::NAN, 600.0), 0..14);
    assert_eq!(v.range(0, 0.0, 600.0), 0..0);
    assert_eq!(v.range(3, 0.0, 600.0), 0..3, "fewer rows than fit");
    assert_eq!(v.max_scroll(3, 600.0), 0.0);
}

#[test]
fn virtual_list_reveal_moves_as_little_as_needed() {
    let v = VirtualList::new(50.0);
    assert_eq!(v.reveal(500, 3, 0.0, 600.0), 0.0, "already visible");
    assert_eq!(v.reveal(500, 20, 0.0, 600.0), 450.0, "brought to the bottom edge");
    assert_eq!(v.reveal(500, 5, 1000.0, 600.0), 250.0, "brought to the top edge");
    assert_eq!(v.reveal(500, 499, 0.0, 600.0), v.max_scroll(500, 600.0));
    assert_eq!(v.reveal(500, 9999, 0.0, 600.0), v.max_scroll(500, 600.0), "out of range is the last row");
}

#[test]
fn search_matches_any_case_and_script_and_ranks_prefixes_first() {
    let l = items();
    let hits = search_filter(&l, "rsi");
    let names: Vec<&str> = hits.iter().map(|&i| l[i].as_str()).collect();
    assert_eq!(names, ["RSI_14", "rsi-divergence", "Stoch RSI"], "prefix, then word prefix");
    assert_eq!(search_filter(&l, "индекс").len(), 1, "Cyrillic, case-insensitive");
    assert_eq!(search_filter(&l, "ОБЪЁМ").len(), 1);
    assert_eq!(search_rank("BTC/USDT", "btc usdt"), Some(0), "/ counts as a space");
    assert_eq!(search_rank("Moving average", "av mov"), Some(1), "every word starts a word, any order");
    assert_eq!(search_rank("Moving average", "erag"), Some(2), "inside a word");
    assert_eq!(search_rank("Moving average", "rsi"), None);
    assert_eq!(search_filter(&l, "  ").len(), l.len(), "empty query keeps everything, in order");
    assert_eq!(search_filter(&l, "  ")[..3], [0, 1, 2]);
    let multi = search_filter(&l, "instrument 4");
    assert_eq!(multi[..80], (400..480).collect::<Vec<_>>()[..], "multi-word: the word prefixes 400..479 first");
    assert!(multi[80..].iter().all(|&i| search_rank(&l[i], "instrument 4") == Some(2)), "then the ones with a 4 inside");
    assert!(search_filter(&l, "zzz").is_empty());
}

fn list_screen(env: &MediaEnv, query: &str, scroll: f64, vh: f64, labels: &[String], v: &VirtualList) -> (El, Vec<usize>) {
    let hits = search_filter(labels, query);
    let mut edit = Edit::new(query).placeholder("Поиск");
    edit.caret = (!query.is_empty()).then_some(query.chars().count());
    let list = v.el("list", hits.len(), scroll, vh, Some("Ничего не найдено"), |i| {
        let mut r = Row::new(labels[hits[i]].clone());
        r.sub = format!("#{}", hits[i]);
        row(&format!("list:row:{}", hits[i]), &r)
    });
    let root = screen(true, [top_bar("nav", "Назад", "Инструменты", None, &[]), search_bar("q", edit, "Поиск", Some("Отмена")), list, tab_bar("tabs", &tabs(), 0)]).css("height: 844px");
    (root, hits)
}

#[test]
fn virtual_list_with_search_fits_a_phone_and_lays_out_only_what_shows() {
    let env = phone();
    let labels = items();
    assert!(labels.len() >= 480);
    let v = VirtualList::new(64.0);
    let (root, hits) = list_screen(&env, "", 0.0, env.height, &labels, &v);
    let d = lay(&root, &env);

    let q = d.rect("q").expect("search input");
    let nav = d.rect("nav:back").unwrap();
    assert!(nav.y >= 47.0, "nav below the notch: {nav:?}");
    assert!(q.y >= nav.y + nav.height - 0.5, "search bar under the nav: {q:?}");
    assert!(q.x >= 16.0 && q.x + q.width <= 390.0, "input inside the screen: {q:?}");
    assert!(d.rect("q:clear").is_none(), "no clear button while empty");
    let cancel = d.rect("q:cancel").expect("cancel button");
    assert!(cancel.x + cancel.width <= 390.0 - 15.5, "cancel not clipped: {cancel:?}");

    let list = d.rect("list").unwrap();
    let tab = d.rect("tabs:0").unwrap();
    assert!(list.y >= q.y + q.height, "list under the search bar");
    assert!(list.y + list.height <= tab.y + 0.5, "list ends above the tab bar: {list:?} {tab:?}");
    assert!(tab.y + tab.height <= 844.0 - 34.0 + 0.5, "tabs above the home indicator: {tab:?}");
    let body = d.get("list").unwrap();
    assert!((body.scroll_height - v.content_height(hits.len())).abs() < 1.0, "scroll height is the whole list: {}", body.scroll_height);

    let laid: Vec<&str> = d.hits().filter(|(id, _)| id.starts_with("list:row:")).map(|(id, _)| id).collect();
    assert!(!laid.is_empty() && laid.len() <= v.range(hits.len(), 0.0, env.height).len(), "only a screenful: {}", laid.len());
    let r0 = d.rect("list:row:0").unwrap();
    assert!((r0.height - 64.0).abs() < 0.5 && (r0.y - list.y).abs() < 0.5, "first row at the top, full height: {r0:?}");
    assert!(r0.x >= list.x && r0.x + r0.width <= list.x + list.width + 0.5, "row inside the list horizontally");

    // At the very end the last row sits whole on the bottom edge of the list, above the tabs.
    let vh = list.height;
    // The host scrolls with the list's height from the last frame.
    let (root, hits) = list_screen(&env, "", v.max_scroll(hits.len(), vh), vh, &labels, &v);
    let d = lay(&root, &env);
    let last = *hits.last().unwrap();
    let lr = d.rect(&format!("list:row:{last}")).expect("last row laid out");
    assert!((lr.y + lr.height - (list.y + list.height)).abs() < 0.5, "last row on the list's bottom edge: {lr:?} vs {list:?}");
    assert!(lr.y + lr.height <= tab.y + 0.5, "and not under the tab bar");
    assert!(d.rect("list:row:0").is_none(), "the top rows are gone");
}

#[test]
fn searching_narrows_the_list_and_shows_clear() {
    let env = phone();
    let labels = items();
    let v = VirtualList::new(64.0);
    let (root, hits) = list_screen(&env, "rsi", 0.0, env.height, &labels, &v);
    assert_eq!(hits.len(), 3);
    let d = lay(&root, &env);
    let q = d.rect("q").unwrap();
    let clear = d.rect("q:clear").expect("clear button once typed");
    assert!(clear.x >= q.x + q.width - 0.5 && clear.x + clear.width <= d.rect("q:cancel").unwrap().x, "clear between the field and cancel");
    let rows: Vec<&str> = d.hits().filter(|(id, _)| id.starts_with("list:row:")).map(|(id, _)| id).collect();
    assert_eq!(rows.len(), 3);
    let rsi = labels.iter().position(|l| l == "RSI_14").unwrap();
    assert!(d.rect(&format!("list:row:{rsi}")).unwrap().y < d.rect(&format!("list:row:{}", hits[2])).unwrap().y, "best match first");

    let (root, hits) = list_screen(&env, "nothing like it", 0.0, env.height, &labels, &v);
    assert!(hits.is_empty());
    let d = lay(&root, &env);
    let list = d.rect("list").unwrap();
    let empty = d.boxes.iter().find(|p| p.text.as_deref() == Some("Ничего не найдено")).expect("the empty line");
    assert!(empty.rect.y >= list.y && empty.rect.x >= list.x && empty.rect.x + empty.rect.width <= list.x + list.width + 0.5, "empty line inside the list: {:?}", empty.rect);
    assert_eq!(d.hits().filter(|(id, _)| id.starts_with("list:row:")).count(), 0);
}
