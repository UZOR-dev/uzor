//! The dashboard widgets ([`uzor::ui::dash`]) on one page, in English or Russian
//! through [`uzor::ui::i18n`]: a space switcher, summary tiles with mini histograms,
//! live agents by space and node, a task tree, limit rings, a task card, a mini board,
//! 24h activity, mini panels (widget, collapsed, rail) and glass overlays (the space
//! menu, a popover, a dialog). [`WidgetsGallery::tree`] is pure: the same page renders
//! live ([`WidgetsPage`]), in tests and in the `dom-widgets-frames` PNG bin.

use std::collections::HashSet;

use uzor::ui::dash::{self as d, Agent, AgentGroup, BoardCard, BoardCol, Meter, MiniPanel, NodeAgents, PanelMode, SpaceItem, TaskCard, TaskNode, TaskState, Tile};
use uzor::ui::dom::El;
use uzor::ui::domkit as k;
use uzor::ui::i18n::{t, Translate};
use uzor::Rect;

/// Gallery languages (index 0, English, is the fallback).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    En = 0,
    Ru = 1,
}

impl Lang {
    /// Make this the current [`uzor::ui::i18n`] language.
    pub fn set(self) {
        uzor::ui::i18n::set_lang_index(self as u8);
    }
}

macro_rules! texts {
    ($($k:ident => $en:literal, $ru:literal;)*) => {
        /// Gallery strings.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Tx { $($k),* }
        impl Translate for Tx {
            fn translate(self, lang: usize) -> &'static str {
                match self { $(Tx::$k => if lang == 1 && !$ru.is_empty() { $ru } else { $en }),* }
            }
        }
    };
}

texts! {
    Space => "Space", "Пространство";
    Spaces => "Spaces", "Пространства";
    NewSpace => "New space", "Новое пространство";
    Agents => "Agents", "Агенты";
    LiveAgents => "Live agents", "Агенты онлайн";
    Tasks => "Tasks", "Задачи";
    MyTasks => "My tasks", "Мои задачи";
    Nodes => "Nodes", "Узлы";
    Done => "Done", "Готово";
    InWork => "In work", "В работе";
    Open => "Open", "Открытые";
    Review => "Review", "Проверка";
    Limits => "Limits", "Лимиты";
    Activity => "Activity · 24h", "Активность · 24ч";
    Board => "Board", "Доска";
    Today => "today", "сегодня";
    Week => "this week", "за неделю";
    NoAgents => "No agents online", "Нет агентов онлайн";
    Session => "Session", "Сессия";
    Agent => "Agent", "Агент";
    Messages => "messages", "сообщения";
    Tokens => "tokens", "токены";
    Uptime => "uptime", "аптайм";
    Subtasks => "subtasks", "подзадачи";
    Assignee => "Assignee", "Исполнитель";
    Node => "Node", "Узел";
    Started => "Started", "Начата";
    Branch => "Branch", "Ветка";
    OpenAct => "Open", "Открыть";
    Reassign => "Reassign", "Переназначить";
    Expand => "Expand", "Развернуть";
    Rail => "Rail", "Свернуть в полосу";
    Close => "Close", "Закрыть";
    Running => "running", "в работе";
    Resets => "resets in 3h", "сброс через 3ч";
    ResetsWeek => "resets Mon", "сброс в пн";
    Widget => "Widget", "Виджет";
    Collapsed => "Collapsed", "Свёрнутая";
    RailMode => "Rail", "Полоса";
    LastEvent => "last event 2 min ago", "последнее событие 2 мин назад";
    Archive => "Archive the task?", "Архивировать задачу?";
    ArchiveBody => "Subtasks move to the archive with it. Agents on them are stopped.", "Подзадачи уйдут в архив вместе с ней. Агенты на них будут остановлены.";
    Cancel => "Cancel", "Отмена";
    ArchiveBtn => "Archive", "В архив";
    Filters => "Filters", "Фильтры";
    Mine => "Only mine", "Только мои";
    WithSub => "With subtasks", "С подзадачами";
    Release => "Release 1.5.26", "Релиз 1.5.26";
    Changelog => "Changelog and notes", "Changelog и заметки";
    Widgets => "DOM widgets", "DOM-виджеты";
    GlassPass => "Glass overlay pass", "Проход по стеклу оверлеев";
    Package => "cargo package check", "Проверка cargo package";
    Board2 => "Task board trees", "Деревья на доске задач";
    Icons => "Provider glyphs", "Символы провайдеров";
    Fold => "Fold and progress", "Сворачивание и прогресс";
    Docs => "Docs for dash", "Документация dash";
    Bug => "Fix ring overflow", "Починить переполнение кольца";
    TileAgentsNote => "3 nodes", "3 узла";
    TileTasksNote => "4 in work", "4 в работе";
    TileDoneNote => "+5 today", "+5 сегодня";
    TileSpend => "Spend", "Расход";
    TileSpendNote => "of $50 limit", "из лимита $50";
    Phone => "phone", "телефон";
}

/// Gallery state a host (or a scripted shot) changes.
#[derive(Clone, Debug, Default)]
pub struct WidgetsGallery {
    pub lang: Lang,
    /// Current space.
    pub space: usize,
    /// The space menu is open.
    pub menu: bool,
    /// The filter popover is open.
    pub popover: bool,
    /// The archive dialog is open.
    pub dialog: bool,
    /// Folded task keys.
    pub folded: HashSet<String>,
    /// Selected task.
    pub sel: Option<String>,
    /// The demo session panel's mode.
    pub panel: PanelMode,
}

/// The page layout on top of [`d::DASH_CSS`].
pub const PAGE_CSS: &str = r#"
div.w-page { display: grid; grid-template-columns: 300px 1fr 300px; gap: 14px; padding: 16px; background: var(--d-bg) }
div.w-col { display: flex; flex-direction: column; gap: 12px; min-width: 0 }
div.w-head { display: flex; align-items: center; gap: 10px }
div.w-h1 { font-size: 15px; font-weight: 600; flex-grow: 1 }
div.w-sec { color: var(--d-faint); font-size: 11px; font-weight: 600; padding: 4px 2px 0 }
div.w-panels { display: flex; gap: 14px; padding: 16px; background: var(--d-bg); align-items: flex-start }
div.w-full { width: 340px; display: flex; flex-direction: column; gap: 10px; background: var(--d-card); border: 1px solid var(--d-line);
             border-radius: 12px; padding: 12px 14px; height: 420px }
div.w-chat { flex-grow: 1; display: flex; flex-direction: column; gap: 10px; background: var(--d-bg); border: 1px solid var(--d-line); border-radius: 12px; padding: 14px; height: 520px }
div.w-msg { color: #c8c8c8; white-space: normal; max-width: 520px }
div.w-cap { color: var(--d-sub); font-size: 11px; font-weight: 600 }
"#;

impl WidgetsGallery {
    /// [`d::DASH_CSS`] plus the page layout.
    pub fn sheet() -> uzor::ui::style::Sheet {
        d::sheet_with(PAGE_CSS).expect("widgets css")
    }

    /// Demo spaces.
    pub fn spaces(&self) -> Vec<SpaceItem> {
        vec![
            SpaceItem::new("hatchery", "Hatchery").sub(format!("3 {}", t(Tx::Nodes).to_lowercase())).live(true),
            SpaceItem::new("uzor", "Uzor").sub(format!("1 {}", t(Tx::Node).to_lowercase())).live(true),
            SpaceItem::new("home", "Home").sub(t(Tx::Phone)),
        ]
    }

    /// Demo task tree.
    pub fn tasks(&self) -> Vec<TaskNode> {
        use TaskState::*;
        vec![
            TaskNode::new("rel", t(Tx::Release), Running).provider("claude").side("knyaz").kids([
                TaskNode::new("w", t(Tx::Widgets), Running).provider("codex").kids([
                    TaskNode::new("g", t(Tx::GlassPass), Done).provider("codex"),
                    TaskNode::new("r", t(Tx::Bug), Done).provider("grok"),
                    TaskNode::new("dd", t(Tx::Docs), Open),
                ]),
                TaskNode::new("c", t(Tx::Changelog), Done).provider("kimi"),
                TaskNode::new("p", t(Tx::Package), Open).provider("opencode"),
            ]),
            TaskNode::new("b", t(Tx::Board2), Running).provider("claude").side("stepan").kids([
                TaskNode::new("i", t(Tx::Icons), Done).provider("gemini"),
                TaskNode::new("f", t(Tx::Fold), Running).provider("claude"),
            ]),
        ]
    }

    fn agents(&self) -> Vec<AgentGroup> {
        vec![
            AgentGroup {
                title: "Hatchery".into(),
                sub: "4".into(),
                nodes: vec![
                    NodeAgents {
                        node: "box-7 · linux".into(),
                        agents: vec![
                            Agent::new("knyaz", "knyaz").provider("claude").sub(t(Tx::Widgets)).live(true).side("2h"),
                            Agent::new("stepan", "stepan").provider("codex").sub(t(Tx::GlassPass)).live(true).side("40m"),
                        ],
                    },
                    NodeAgents { node: "mac-mini".into(), agents: vec![Agent::new("vasilisa", "vasilisa").provider("grok").sub(t(Tx::Bug)).live(true).side("12m")] },
                ],
            },
            AgentGroup {
                title: "Uzor".into(),
                sub: "2".into(),
                nodes: vec![NodeAgents {
                    node: String::new(),
                    agents: vec![
                        Agent::new("kimi", "miron").provider("kimi").sub(t(Tx::Changelog)).side("1d"),
                        Agent::new("oc", "fedot").provider("opencode").sub(t(Tx::Package)).live(true).side("5m"),
                    ],
                }],
            },
        ]
    }

    fn tiles(&self) -> Vec<Tile> {
        vec![
            Tile::new(t(Tx::Agents), "5").note(t(Tx::TileAgentsNote)).hist(vec![2.0, 3.0, 3.0, 4.0, 2.0, 5.0, 4.0, 5.0, 6.0, 5.0, 5.0, 5.0]),
            Tile::new(t(Tx::Tasks), "12").note(t(Tx::TileTasksNote)).hist(vec![8.0, 9.0, 9.0, 10.0, 11.0, 10.0, 12.0, 12.0, 11.0, 13.0, 12.0, 12.0]),
            Tile::new(t(Tx::Done), "37").note(t(Tx::TileDoneNote)).hist(vec![1.0, 0.0, 2.0, 3.0, 1.0, 4.0, 2.0, 3.0, 5.0, 2.0, 4.0, 5.0]),
            Tile::new(t(Tx::TileSpend), "$18.40").note(t(Tx::TileSpendNote)).hist(vec![1.0, 2.0, 1.5, 3.0, 2.0, 2.5, 4.0, 3.0, 2.0, 3.5, 3.0, 2.0]),
        ]
    }

    fn meters(&self) -> Vec<Meter> {
        vec![
            Meter::new("5h", 0.62).note(t(Tx::Resets)),
            Meter::new(t(Tx::Week).to_string(), 0.34).note(t(Tx::ResetsWeek)),
            Meter::new("opus", 0.81).note(t(Tx::ResetsWeek)),
        ]
    }

    fn board(&self) -> Vec<BoardCol> {
        let c = |key: &str, title: Tx, p: &str, meta: &str, kids: (u32, u32), live: bool| BoardCard {
            key: key.into(),
            title: t(title).into(),
            provider: p.into(),
            meta: meta.into(),
            kids,
            live,
        };
        vec![
            BoardCol { title: t(Tx::Open).into(), cards: vec![c("p", Tx::Package, "opencode", "fedot", (0, 0), false), c("dd", Tx::Docs, "", "—", (0, 0), false)] },
            BoardCol {
                title: t(Tx::InWork).into(),
                cards: vec![c("rel", Tx::Release, "claude", "knyaz", (4, 7), true), c("w", Tx::Widgets, "codex", "stepan", (2, 3), true), c("f", Tx::Fold, "claude", "knyaz", (0, 0), true)],
            },
            BoardCol { title: t(Tx::Review).into(), cards: vec![c("b", Tx::Board2, "claude", "stepan", (1, 2), false)] },
            BoardCol { title: t(Tx::Done).into(), cards: vec![c("g", Tx::GlassPass, "codex", "stepan", (0, 0), false), c("r", Tx::Bug, "grok", "vasilisa", (0, 0), false), c("c", Tx::Changelog, "kimi", "miron", (0, 0), false), c("i", Tx::Icons, "gemini", "—", (0, 0), false)] },
        ]
    }

    /// The demo session's metrics.
    pub fn session(&self) -> MiniPanel {
        MiniPanel {
            kind: t(Tx::Session).into(),
            title: "knyaz · hatchery".into(),
            live: true,
            provider: "claude".into(),
            kpis: vec![(t(Tx::Messages).into(), "148".into()), (t(Tx::Tokens).into(), "1.2M".into()), (t(Tx::Uptime).into(), "2h 14m".into())],
            meters: vec![Meter::new("5h", 0.62), Meter::new(t(Tx::Week).to_string(), 0.34)],
            trend: vec![2.0, 4.0, 3.0, 6.0, 5.0, 7.0, 4.0, 8.0, 6.0, 9.0, 7.0, 8.0, 10.0, 6.0, 7.0],
            foot: t(Tx::LastEvent).into(),
        }
    }

    fn space_panel(&self) -> MiniPanel {
        MiniPanel {
            kind: t(Tx::Space).into(),
            title: "Hatchery".into(),
            live: true,
            provider: String::new(),
            kpis: vec![(t(Tx::Agents).to_lowercase(), "5".into()), (t(Tx::Tasks).to_lowercase(), "12".into()), (t(Tx::Nodes).to_lowercase(), "3".into())],
            meters: vec![],
            trend: vec![3.0, 4.0, 4.0, 5.0, 6.0, 5.0, 7.0, 6.0, 8.0, 7.0, 9.0, 8.0],
            foot: String::new(),
        }
    }

    fn agent_panel(&self) -> MiniPanel {
        MiniPanel {
            kind: t(Tx::Agent).into(),
            title: "stepan".into(),
            live: true,
            provider: "codex".into(),
            kpis: vec![(t(Tx::Tasks).to_lowercase(), "3".into()), (t(Tx::Tokens).into(), "420k".into()), (t(Tx::Uptime).into(), "40m".into())],
            meters: vec![Meter::new("5h", 0.18)],
            trend: vec![],
            foot: String::new(),
        }
    }

    fn labels() -> [&'static str; 3] {
        [t(Tx::OpenAct), t(Tx::Expand), t(Tx::Rail)]
    }

    /// The catalog page for a `w × h` viewport.
    pub fn tree(&self, w: f64, h: f64) -> El {
        self.lang.set();
        let spaces = self.spaces();
        let cur = &spaces[self.space.min(spaces.len() - 1)];
        let rows = d::flatten_tree(&self.tasks(), &self.folded);
        let left = El::new("div.w-col")
            .child(
                El::new("div.w-head")
                    .child(d::space_switcher("space", cur, t(Tx::Space)).css("flex-grow: 1"))
                    .child(El::new("button.d-btn").id("lang").tab().text(if self.lang == Lang::En { "RU" } else { "EN" })),
            )
            .child(d::section(t(Tx::LiveAgents), "5", d::live_agents("agents", &self.agents(), t(Tx::NoAgents))))
            .child(d::section(t(Tx::MyTasks), "7", d::task_list("tasks", &rows, self.sel.as_deref())));
        let mid = El::new("div.w-col")
            .child(d::tiles("tiles", &self.tiles(), 4))
            .child(d::activity(
                "act",
                t(Tx::Activity),
                "214",
                &[2.0, 1.0, 0.0, 0.0, 1.0, 0.0, 3.0, 6.0, 9.0, 12.0, 14.0, 11.0, 8.0, 13.0, 15.0, 12.0, 10.0, 9.0, 7.0, 11.0, 13.0, 9.0, 6.0, 4.0],
                Some(21),
                &["00", "06", "12", "18", "24"],
            ))
            .child(d::section(t(Tx::Board), "", d::mini_board("board", &self.board(), 3)));
        let card = TaskCard {
            title: t(Tx::Release).into(),
            status: t(Tx::Running).into(),
            state: TaskState::Running,
            provider: "claude".into(),
            facts: vec![
                (t(Tx::Assignee).into(), "knyaz".into()),
                (t(Tx::Node).into(), "box-7".into()),
                (t(Tx::Started).into(), "14:02".into()),
                (t(Tx::Branch).into(), "widgets-1.5.26".into()),
            ],
            body: String::new(),
            progress: Some((4, 7, t(Tx::Subtasks).into())),
            actions: vec![("open".into(), t(Tx::OpenAct).into()), ("reassign".into(), t(Tx::Reassign).into())],
        };
        let right = El::new("div.w-col")
            .child(d::section(t(Tx::Limits), "", d::rings("limits", &self.meters())))
            .child(d::task_card("card", &card))
            .child(d::mini_panel("session", &self.session(), PanelMode::Widget, Self::labels()));
        let mut page = El::new("div.k-root.d-root").css(format!("height: {h}px; background: var(--d-bg)")).child(El::new("div.w-page").children([left, mid, right]));
        let vp = Rect::new(0.0, 0.0, w, h);
        if self.menu {
            page = page.child(d::space_menu("space", t(Tx::Spaces), &spaces, self.space, Some(t(Tx::NewSpace)), Rect::new(16.0, 54.0, 300.0, 0.0)));
        }
        if self.popover {
            let body = El::new("div")
                .css("display: flex; flex-direction: column; gap: 8px; padding: 2px 8px 8px")
                .child(k::checkbox("f:mine", true, t(Tx::Mine)))
                .child(k::checkbox("f:sub", false, t(Tx::WithSub)))
                .child(d::rings("f:rings", &[Meter::new("5h", 0.62)]));
            page = page.child(d::glass(d::popover("filters", t(Tx::Filters), body, Rect::new(w - 316.0 - 250.0, 250.0, 240.0, 0.0), t(Tx::Close))));
        }
        if self.dialog {
            let body = El::new("div").text(t(Tx::ArchiveBody));
            page = page.child(d::glass(k::dialog("arch", t(Tx::Archive), body, &[t(Tx::ArchiveBtn), t(Tx::Cancel)], k::centered(vp, 420.0, 190.0))));
        }
        page
    }

    /// Mini panels in every mode, next to a full panel and a chat that embeds one.
    pub fn panels_tree(&self, h: f64) -> El {
        self.lang.set();
        let cap = |s: &str| El::new("div.w-cap").text(s);
        let full = El::new("div.w-full")
            .child(cap(t(Tx::Widget)))
            .child(d::mini_panel("p:space", &self.space_panel(), PanelMode::Widget, Self::labels()))
            .child(d::mini_panel("p:agent", &self.agent_panel(), PanelMode::Widget, Self::labels()));
        let chat = El::new("div.w-chat")
            .child(cap(t(Tx::Session)))
            .child(El::new("div.w-msg").text(t(Tx::ArchiveBody)))
            .child(d::mini_panel("p:chat", &self.session(), PanelMode::Widget, Self::labels()).css("max-width: 420px"))
            .child(El::new("div.w-msg").text(t(Tx::LastEvent)));
        let collapsed = El::new("div.w-col")
            .css("width: 260px")
            .child(cap(t(Tx::Collapsed)))
            .child(d::mini_panel("p:col", &self.session(), PanelMode::Collapsed, Self::labels()).css("height: 470px"));
        let rail = El::new("div.w-col").child(cap(t(Tx::RailMode))).child(d::mini_panel("p:rail", &self.session(), PanelMode::Rail, Self::labels()).css("height: 470px"));
        El::new("div.k-root.d-root").css(format!("height: {h}px; background: var(--d-bg)")).child(El::new("div.w-panels").children([full, chat, collapsed, rail]))
    }

    /// React to a click.
    pub fn click(&mut self, c: &str) {
        let was_menu = self.menu;
        self.menu = false;
        if c == "lang" {
            self.lang = if self.lang == Lang::En { Lang::Ru } else { Lang::En };
        } else if c == "space" {
            self.menu = !was_menu;
        } else if let Some(i) = d::picked_space("space", c) {
            self.space = i;
        } else if d::toggle_fold("tasks", c, &mut self.folded) {
        } else if let Some(k) = c.strip_prefix("tasks:row:") {
            self.sel = Some(k.to_string());
        } else if let Some(m) = self.panel.after("session", c) {
            self.panel = m;
        } else if c == "filters:close" {
            self.popover = false;
        } else if c.starts_with("arch:") {
            self.dialog = false;
        } else if c == "card:act:reassign" {
            self.popover = true;
        } else if c == "card:act:open" {
            self.dialog = true;
        }
    }
}

/// The widgets gallery as a live page.
pub struct WidgetsPage {
    pub g: WidgetsGallery,
    sheet: uzor::ui::style::Sheet,
    /// Last click, for tests.
    pub last_click: Option<String>,
    hits: Vec<(String, Rect)>,
}

impl Default for WidgetsPage {
    fn default() -> Self {
        WidgetsPage { g: WidgetsGallery::default(), sheet: WidgetsGallery::sheet(), last_click: None, hits: Vec::new() }
    }
}

impl crate::page_app::Page for WidgetsPage {
    fn title(&self) -> &str {
        "DOM widgets"
    }
    fn size(&self) -> (u32, u32) {
        (1280, 900)
    }
    fn paint(&mut self, cx: &mut uzor_framework::PanelCx<'_, crate::page_app::PageSpec>, _now_ms: u64) {
        use uzor_framework::widgets::kit;
        let area = cx.rect;
        let tree = self.g.tree(area.width, area.height);
        let resp = kit::dom(area, &self.sheet, tree).build(cx);
        self.hits = resp.dom.hits().map(|(id, r)| (id.to_string(), r)).collect();
        if let Some(c) = resp.clicked.clone() {
            self.g.click(&c);
            self.last_click = Some(c);
            cx.out.request_frame();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page_app::testing::{click_at, open, Host};

    fn click(h: &mut Host<WidgetsPage>, id: &str) {
        let hits: Vec<(String, Rect)> = h.runtime().app().page.hits.clone();
        let r = hits.iter().find(|(i, _)| i == id).map(|(_, r)| *r).unwrap_or_else(|| panic!("no hit {id}: {hits:?}"));
        click_at(h, r.x + r.width / 2.0, r.y + r.height / 2.0);
    }

    #[test]
    fn space_menu_tree_folding_and_overlays() {
        let mut h = open(WidgetsPage::default());
        click(&mut h, "space");
        assert!(h.runtime().app().page.g.menu);
        click(&mut h, "space:item:1");
        let g = &h.runtime().app().page.g;
        assert_eq!((g.space, g.menu), (1, false));
        click(&mut h, "tasks:fold:w");
        assert!(h.runtime().app().page.g.folded.contains("w"));
        click(&mut h, "tasks:row:rel");
        assert_eq!(h.runtime().app().page.g.sel.as_deref(), Some("rel"));
        click(&mut h, "lang");
        assert_eq!(h.runtime().app().page.g.lang, Lang::Ru);
        click(&mut h, "card:act:open");
        assert!(h.runtime().app().page.g.dialog);
    }

    #[test]
    fn russian_texts_are_complete() {
        // Every key has a Russian cell distinct from or equal to English on purpose.
        let g = WidgetsGallery { lang: Lang::Ru, ..Default::default() };
        let _ = g.tree(1280.0, 900.0);
        assert_eq!(t(Tx::MyTasks), "Мои задачи");
        Lang::En.set();
        assert_eq!(t(Tx::MyTasks), "My tasks");
    }
}
