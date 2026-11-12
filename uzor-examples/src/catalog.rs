//! The example index: every uzor example as a card on one DOM page, with a filter, a
//! search box, a detail pane (what it shows, how to run it) and a Launch button that
//! starts the example's binary when it sits next to the launcher.
//!
//! Adding an example to the set is one [`Example`] entry in [`EXAMPLES`]; the cards, the
//! filter counts and the detail pane follow. [`PLANNED`] lists the demos that come next.
//!
//! ```sh
//! cargo build -p uzor-examples --release --features native,png --bins
//! cargo build -p uzor-framework --release --features native --examples
//! cargo run -p uzor-examples --release --features native --bin uzor-launcher
//! ```

use std::path::PathBuf;

use uzor::ui::dom::{Edit, El};
use uzor::ui::domkit::{self as k, icons};
use uzor::ui::style::Sheet;
use uzor::Rect;
use uzor_framework::widgets::kit;
use uzor_framework::{PanelCx, Spec};

use crate::look;
use crate::page_app::{Page, PageSpec};

/// How an example runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A native window (winit + a GPU or CPU backend).
    Native,
    /// A wasm build drawn into a `<canvas>`.
    Web,
    /// Renders to files, no window.
    Offscreen,
}

impl Kind {
    /// Filter label.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Native => "Native",
            Kind::Web => "Web",
            Kind::Offscreen => "Offscreen",
        }
    }
}

/// One entry of the index.
#[derive(Clone, Copy, Debug)]
pub struct Example {
    /// Card title.
    pub name: &'static str,
    /// Binary (or wasm crate) name: what Launch looks for next to the launcher.
    pub bin: &'static str,
    /// Where it lives in the repository.
    pub path: &'static str,
    /// How it runs.
    pub kind: Kind,
    /// Card icon (an [`icons`] body).
    pub icon: &'static str,
    /// One line for the card.
    pub blurb: &'static str,
    /// What it shows off, for the detail pane.
    pub shows: &'static [&'static str],
    /// The command that builds and runs it.
    pub cmd: &'static str,
    /// Arguments Launch passes.
    pub args: &'static [&'static str],
}

/// Every example. Add a line here and it gets a card.
pub const EXAMPLES: &[Example] = &[
    Example {
        name: "Dashboard",
        bin: "l4-dashboard",
        path: "uzor-examples/src/dashboard.rs",
        kind: Kind::Native,
        icon: icons::GRID,
        blurb: "A docked grid of live chart cards and a DOM settings page.",
        shows: &["Docking: splits, tabs, new windows", "DOM cards with kit charts in slots", "Theme switch, on-demand vs 60 fps"],
        cmd: "cargo run -p uzor-examples --release --features native --bin l4-dashboard",
        args: &[],
    },
    Example {
        name: "Kit gallery",
        bin: "kit-gallery",
        path: "uzor-examples/src/kit_gallery.rs",
        kind: Kind::Native,
        icon: icons::BOX,
        blurb: "Every domkit control and overlay, wired up.",
        shows: &["Tabs, fields, select, segmented, table, list", "Menu, dialog, drawer, tooltip, toasts", "A kit splitter over the DOM gutter"],
        cmd: "cargo run -p uzor-examples --release --features native --bin kit-gallery",
        args: &[],
    },
    Example {
        name: "DOM gallery",
        bin: "dom-gallery",
        path: "uzor-examples/src/dom_gallery.rs",
        kind: Kind::Native,
        icon: icons::LIST,
        blurb: "Chat blocks, markdown and a 400-row virtualized list.",
        shows: &["Inline markdown, bubbles, action rows", "VirtualList: only visible rows laid out", "Fixed side panel and select popover", "A kit chart in a DOM slot"],
        cmd: "cargo run -p uzor-examples --release --features native --bin dom-gallery",
        args: &[],
    },
    Example {
        name: "DOM gallery frames",
        bin: "dom-gallery-frames",
        path: "uzor-examples/src/bin/dom_gallery_frames.rs",
        kind: Kind::Offscreen,
        icon: icons::DOWNLOAD,
        blurb: "Renders a scripted walk-through to PNG frames on the CPU.",
        shows: &["tiny-skia CPU backend, no window", "Scripted hover, focus, caret, selection", "Join frames into a video with ffmpeg"],
        cmd: "cargo run -p uzor-examples --release --features png --bin dom-gallery-frames -- out 30",
        args: &["uzor-dom-gallery-frames", "30"],
    },
    Example {
        name: "Tray widgets",
        bin: "tray-widgets",
        path: "uzor-examples/src/tray_widgets.rs",
        kind: Kind::Native,
        icon: icons::PIN,
        blurb: "Compact cards for the tray flyout and the taskbar thumbnail.",
        shows: &["Tasks, limits rings, live agents, a combined card", "Glass flyout over the tray icon, status dot badge", "Windows taskbar thumbnail drawn from the widget"],
        cmd: "cargo run -p uzor-examples --release --features native --bin tray-widgets",
        args: &[],
    },
    Example {
        name: "Tray widget frames",
        bin: "tray-widgets-frames",
        path: "uzor-examples/src/bin/tray_widgets_frames.rs",
        kind: Kind::Offscreen,
        icon: icons::DOWNLOAD,
        blurb: "Every tray widget, dark and light, rendered to PNG on the CPU.",
        shows: &["Flyout glass and solid thumbnail looks", "RU / EN labels", "The gallery page"],
        cmd: "cargo run -p uzor-examples --release --features png --bin tray-widgets-frames -- out",
        args: &["uzor-tray-widgets"],
    },
    Example {
        name: "Paint slot frames",
        bin: "dom-slots-frames",
        path: "uzor-examples/src/dom_slots.rs",
        kind: Kind::Offscreen,
        icon: icons::DOWNLOAD,
        blurb: "Slots painted and driven by registered painters, rendered to PNG on the CPU.",
        shows: &["SlotRegistry: a painter per slot key", "A sparkline that tracks the pointer, a dial on wheel, drag and keys", "Slot a11y role and name from the painter", "Dark / light from the sheet, RU / EN"],
        cmd: "cargo run -p uzor-examples --release --features png --bin dom-slots-frames -- out",
        args: &["uzor-dom-slots"],
    },
    Example {
        name: "Image frames",
        bin: "dom-images-frames",
        path: "uzor-examples/src/dom_images.rs",
        kind: Kind::Offscreen,
        icon: icons::DOWNLOAD,
        blurb: "Raster images in the DOM, rendered to PNG on the CPU.",
        shows: &["El::image with RGBA pixels or a backend image id", "object-fit cover / contain, round avatar by border-radius", "The box background while an image is not loaded"],
        cmd: "cargo run -p uzor-examples --release --features png --bin dom-images-frames -- out",
        args: &["uzor-dom-images"],
    },
    Example {
        name: "Framework demo",
        bin: "demo_native",
        path: "uzor-framework/examples/demo",
        kind: Kind::Native,
        icon: icons::BOLT,
        blurb: "The framework tour: panels, overlays, actions.",
        shows: &["App / Spec vocabulary", "Overlays and actions", "Same code on the web build"],
        cmd: "cargo run -p uzor-framework --release --features native --example demo_native",
        args: &[],
    },
    Example {
        name: "Image viewer",
        bin: "image_viewer",
        path: "uzor-framework/examples/image_viewer.rs",
        kind: Kind::Native,
        icon: icons::IMAGE,
        blurb: "Thumbnails, zoom and pan; images drawn in DOM slots.",
        shows: &["Raster images through slots", "Wheel zoom, drag pan", "DOM toolbar and thumbnail strip"],
        cmd: "cargo run -p uzor-framework --release --features native --example image_viewer",
        args: &[],
    },
    Example {
        name: "Two windows",
        bin: "native_two_windows",
        path: "uzor-framework/examples/native_two_windows.rs",
        kind: Kind::Native,
        icon: icons::WINDOW,
        blurb: "Two OS windows sharing one app state.",
        shows: &["Multi-window runtime", "Cross-window state and intents", "Per-window chrome"],
        cmd: "cargo run -p uzor-framework --release --features native --example native_two_windows",
        args: &[],
    },
    Example {
        name: "Demo (web)",
        bin: "demo_web",
        path: "uzor-framework/examples/demo/main_web.rs",
        kind: Kind::Web,
        icon: icons::PLAY,
        blurb: "The framework demo compiled to wasm in a canvas.",
        shows: &["Web host: DOM events to uzor input", "Same App as the native demo", "On-demand repaint in the browser"],
        cmd: "cargo build -p uzor-framework --release --target wasm32-unknown-unknown --features web --example demo_web && wasm-bindgen --target web",
        args: &[],
    },
    Example {
        name: "Monitor (web)",
        bin: "monitor_web",
        path: "uzor-framework/examples/monitor",
        kind: Kind::Web,
        icon: icons::TERMINAL,
        blurb: "A live ops monitor: virtualized lists fed from JS.",
        shows: &["Data pushed from JavaScript", "Virtualized, selectable rows", "Kit scroll views with DOM rows"],
        cmd: "cargo build -p uzor-framework --release --target wasm32-unknown-unknown --features monitor --example monitor_web && wasm-bindgen --target web",
        args: &[],
    },
];

/// Demos that come next (shown as planned on the index).
pub const PLANNED: &[&str] = &[
    "Chat on VirtualList",
    "Settings & forms",
    "Data table explorer",
    "Markdown & typography",
    "Charts in slots",
    "3D embed",
    "Animation showcase",
    "TUI backend",
    "A11y inspector",
    "Theme playground",
];

const FILTERS: &[&str] = &["All", "Native", "Web", "Offscreen"];

const CSS: &str = r#"
div.c-head { display: flex; align-items: center; gap: 10px; flex-shrink: 0 }
div.c-body { display: flex; gap: 14px; flex-grow: 1 }
div.c-main { display: flex; flex-direction: column; gap: 12px; flex-grow: 1 }
div.c-grid { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px; flex-shrink: 0 }
div.c-card { display: flex; flex-direction: column; gap: 8px; padding: 12px; height: 150px; border-radius: 12px;
             background: var(--k-panel); border: 1px solid var(--k-line) }
div.c-card:hover { border-color: var(--k-muted) }
div.c-card.on { border: 1px solid var(--k-accent) }
div.c-title { font-weight: 600; flex-grow: 1 }
div.c-blurb { color: var(--k-muted); height: 36px; white-space: normal }
div.c-tags { display: flex; gap: 6px; flex-wrap: wrap }
div.c-tag { padding: 1px 7px; border-radius: 9px; font-size: 11px; color: var(--k-muted); border: 1px solid var(--k-line) }
div.c-side { display: flex; flex-direction: column; gap: 10px; width: 330px; padding: 14px; border-radius: 12px;
             background: var(--k-panel); border: 1px solid var(--k-line); flex-shrink: 0 }
div.c-code { white-space: normal; padding: 8px 10px; border-radius: 8px; background: var(--k-bg); border: 1px solid var(--k-line); font-size: 12px }
div.c-li { color: var(--k-fg) }
div.c-status { white-space: normal; color: var(--k-muted); font-size: 12px }
div.c-plan { display: flex; gap: 8px; flex-wrap: wrap; flex-shrink: 0 }
div.c-planned { padding: 4px 10px; border-radius: 12px; border: 1px dashed var(--k-line); color: var(--k-muted); font-size: 12px }
div.c-empty { color: var(--k-muted); padding: 24px }
"#;

/// The index page.
pub struct Catalog {
    /// Active filter (index into All / Native / Web / Offscreen).
    pub filter: usize,
    /// Search text.
    pub query: String,
    /// Selected example (index into [`EXAMPLES`]).
    pub selected: usize,
    /// Dark theme.
    pub dark: bool,
    /// The status line under Launch.
    pub status: String,
    /// Launched processes (kept so they are not zombies until the launcher exits).
    children: Vec<std::process::Child>,
    sheet: Sheet,
    hits: Vec<(String, Rect)>,
    /// Actually spawn binaries (tests turn it off).
    pub spawn: bool,
}

impl Default for Catalog {
    fn default() -> Self {
        Catalog {
            filter: 0,
            query: String::new(),
            selected: 0,
            dark: true,
            status: String::new(),
            children: Vec::new(),
            sheet: look::sheet(CSS),
            hits: Vec::new(),
            spawn: true,
        }
    }
}

impl Catalog {
    /// The examples the filter and the search let through, with their [`EXAMPLES`] index.
    pub fn visible(&self) -> Vec<(usize, &'static Example)> {
        let q = self.query.to_lowercase();
        EXAMPLES
            .iter()
            .enumerate()
            .filter(|(_, e)| self.filter == 0 || e.kind.label() == FILTERS[self.filter])
            .filter(|(_, e)| {
                q.is_empty()
                    || e.name.to_lowercase().contains(&q)
                    || e.blurb.to_lowercase().contains(&q)
                    || e.shows.iter().any(|s| s.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// The rect of hit `id` in the last frame.
    pub fn hit(&self, id: &str) -> Option<Rect> {
        self.hits.iter().find(|(h, _)| h == id).map(|(_, r)| *r)
    }

    /// Where `bin` would be next to the running launcher (`target/<profile>/` for bins,
    /// `target/<profile>/examples/` for framework examples), if it is built.
    pub fn locate(bin: &str) -> Option<PathBuf> {
        let exe = std::env::current_exe().ok()?;
        let dir = exe.parent()?;
        let name = format!("{bin}{}", std::env::consts::EXE_SUFFIX);
        [dir.join(&name), dir.join("examples").join(&name), dir.parent()?.join(&name)].into_iter().find(|p| p.is_file())
    }

    /// The page tree for `area`.
    pub fn tree(&self, area: Rect) -> El {
        let vis = self.visible();
        let counts: Vec<usize> =
            FILTERS.iter().enumerate().map(|(i, f)| EXAMPLES.iter().filter(|e| i == 0 || e.kind.label() == *f).count()).collect();
        let labels: Vec<String> = FILTERS.iter().zip(&counts).map(|(f, n)| format!("{f} {n}")).collect();
        let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        let head = El::new("div.c-head")
            .child(look::ico(icons::GRID, true))
            .child(El::new("div.p-h1").text("uzor examples"))
            .child(k::badge(&format!("{} examples", EXAMPLES.len()), "info"))
            .child(El::new("div.p-grow"))
            .child(k::input("cat:search", Edit::new(&self.query).placeholder("Search examples")).css("width: 220px"))
            .child(k::segmented("cat:filter", &label_refs, self.filter))
            .child(k::button("cat:theme", if self.dark { "Light" } else { "Dark" }));
        let mut grid = El::new("div.c-grid");
        for (i, e) in &vis {
            let kind = match e.kind {
                Kind::Native => "ok",
                Kind::Web => "info",
                Kind::Offscreen => "warn",
            };
            let card = El::new(if *i == self.selected { "div.c-card.on" } else { "div.c-card" })
                .id(format!("cat:card:{i}"))
                .tab()
                .name(e.name)
                .child(El::new("div.p-row").child(look::ico(e.icon, *i == self.selected)).child(El::new("div.c-title").text(e.name)).child(k::badge(e.kind.label(), kind)))
                .child(El::new("div.c-blurb").text(e.blurb))
                .child(El::new("div.c-tags").children(e.shows.iter().take(2).map(|s| El::new("div.c-tag").text(s.split([':', ',']).next().unwrap_or(s)))));
            grid = grid.child(card);
        }
        let mut main = El::new("div.c-main").css(format!("width: {}px", (area.width - 28.0 - 14.0 - 330.0).max(200.0))).child(El::new("div.p-sec").text("EXAMPLES"));
        main = if vis.is_empty() { main.child(El::new("div.c-empty").text("Nothing matches the search.")) } else { main.child(grid) };
        main = main
            .child(El::new("div.p-sec").text("NEXT UP"))
            .child(El::new("div.c-plan").children(PLANNED.iter().map(|p| El::new("div.c-planned").text(*p))));
        let body = El::new("div.c-body").child(main).child(self.detail());
        look::root(self.dark, area.height).child(El::new("div.p-page").css(format!("height: {}px", area.height)).child(head).child(body))
    }

    fn detail(&self) -> El {
        let e = &EXAMPLES[self.selected.min(EXAMPLES.len() - 1)];
        let built = Self::locate(e.bin).is_some();
        let (launch, state) = match e.kind {
            Kind::Web => ("Open build notes", k::badge("wasm", "info")),
            _ if built => ("Launch", k::badge("built", "ok")),
            _ => ("Launch", k::badge("not built", "warn")),
        };
        El::new("div.c-side")
            .id("cat:detail")
            .child(El::new("div.p-row").child(look::ico(e.icon, true)).child(El::new("div.p-h1.p-grow").text(e.name)).child(state))
            .child(El::new("div.p-muted").css("white-space: normal").text(e.blurb))
            .child(El::new("div.p-sec").text("SHOWS"))
            .children(e.shows.iter().map(|s| El::new("div.c-li").text(format!("• {s}"))))
            .child(El::new("div.p-sec").text("SOURCE"))
            .child(El::new("div.c-code").text(e.path))
            .child(El::new("div.p-sec").text("RUN"))
            .child(El::new("div.c-code").id("cat:cmd").text(e.cmd))
            .child(El::new("div.p-row").child(El::new("button.k-btn.primary").id("cat:launch").tab().text(launch)).child(El::new("div.p-muted").text("or double-click a card")))
            .child(El::new("div.c-status").id("cat:status").text(&self.status))
    }

    /// Start the selected example (or say how to build it).
    pub fn launch(&mut self) {
        let e = EXAMPLES[self.selected.min(EXAMPLES.len() - 1)];
        if e.kind == Kind::Web {
            self.status = format!("Web example: build with the command above, then serve the pkg dir and open {}.html.", e.bin.trim_end_matches("_web"));
            return;
        }
        let Some(path) = Self::locate(e.bin) else {
            self.status = format!("{} is not built next to the launcher. Run the command above.", e.bin);
            return;
        };
        if !self.spawn {
            self.status = format!("Would start {}", path.display());
            return;
        }
        let args: Vec<String> = e
            .args
            .iter()
            .map(|a| if a.starts_with("uzor-") { std::env::temp_dir().join(a).display().to_string() } else { a.to_string() })
            .collect();
        match std::process::Command::new(&path).args(&args).spawn() {
            Ok(c) => {
                self.status = if e.kind == Kind::Offscreen {
                    format!("Rendering frames to {} (pid {})", args.first().map(String::as_str).unwrap_or("."), c.id())
                } else {
                    format!("Started {} (pid {})", e.name, c.id())
                };
                self.children.push(c);
            }
            Err(err) => self.status = format!("Could not start {}: {err}", e.bin),
        }
    }

    /// React to a click on `c`.
    pub fn click(&mut self, c: &str) {
        if let Some(i) = k::picked("cat:card", "", c) {
            if i == self.selected && !self.status.is_empty() {
                return;
            }
            self.selected = i;
            self.status.clear();
        } else if let Some(i) = k::picked("cat:filter", "", c) {
            self.filter = i;
            if let Some((first, _)) = self.visible().first() {
                if !self.visible().iter().any(|(j, _)| *j == self.selected) {
                    self.selected = *first;
                    self.status.clear();
                }
            }
        } else if c == "cat:launch" {
            self.launch();
        } else if c == "cat:theme" {
            self.dark = !self.dark;
        }
    }

    /// Paint into `area`.
    pub fn paint_in<S: Spec>(&mut self, cx: &mut PanelCx<'_, S>, area: Rect) {
        // Reap finished children.
        self.children.retain_mut(|c| matches!(c.try_wait(), Ok(None)));
        let resp = kit::dom(area, &self.sheet, self.tree(area)).build(cx);
        self.hits = resp.dom.hits().map(|(id, r)| (id.to_string(), r)).collect();
        for (id, v) in &resp.edited {
            if id == "cat:search" {
                self.query = v.clone();
                if let Some((first, _)) = self.visible().first() {
                    if !self.visible().iter().any(|(j, _)| *j == self.selected) {
                        self.selected = *first;
                    }
                }
                cx.out.request_frame();
            }
        }
        if let Some(c) = resp.clicked.as_deref() {
            // A second click on the selected card launches it (a double click).
            let again = k::picked("cat:card", "", c) == Some(self.selected) && cx.view.click_count(&c.into()) >= 2;
            self.click(c);
            if again {
                self.launch();
            }
            cx.out.request_frame();
        }
        if resp.hovered.is_some() {
            cx.out.request_frame();
        }
    }
}

impl Page for Catalog {
    fn title(&self) -> &str {
        "Examples"
    }
    fn size(&self) -> (u32, u32) {
        (1280, 800)
    }
    fn paint(&mut self, cx: &mut PanelCx<'_, PageSpec>, _now_ms: u64) {
        let area = cx.rect;
        self.paint_in(cx, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page_app::testing::{click_at, open, Host};

    fn host() -> Host<Catalog> {
        let mut c = Catalog::default();
        c.spawn = false;
        open(c)
    }

    fn click(h: &mut Host<Catalog>, id: &str) {
        let r = h.runtime().app().page.hit(id).unwrap_or_else(|| panic!("no hit {id}"));
        click_at(h, r.x + r.width / 2.0, r.y + r.height / 2.0);
    }

    fn page(h: &Host<Catalog>) -> &Catalog {
        &h.runtime().app().page
    }

    #[test]
    fn every_example_has_a_card() {
        let h = host();
        for i in 0..EXAMPLES.len() {
            assert!(page(&h).hit(&format!("cat:card:{i}")).is_some(), "card {i}");
        }
        assert!(page(&h).hit("cat:launch").is_some());
    }

    #[test]
    fn filter_select_and_launch() {
        let mut h = host();
        click(&mut h, "cat:filter:2");
        assert_eq!(page(&h).filter, 2);
        let web: Vec<usize> = EXAMPLES.iter().enumerate().filter(|(_, e)| e.kind == Kind::Web).map(|(i, _)| i).collect();
        assert_eq!(page(&h).selected, web[0]);
        assert!(page(&h).hit("cat:card:0").is_none(), "native cards filtered out");
        click(&mut h, &format!("cat:card:{}", web[1]));
        assert_eq!(page(&h).selected, web[1]);
        click(&mut h, "cat:launch");
        assert!(page(&h).status.contains("Web example"), "{}", page(&h).status);
        click(&mut h, "cat:filter:0");
        click(&mut h, "cat:card:1");
        click(&mut h, "cat:launch");
        assert!(page(&h).status.contains("kit-gallery"), "{}", page(&h).status);
    }

    #[test]
    fn search_narrows_the_cards() {
        let mut c = Catalog::default();
        c.query = "virtualized".into();
        let names: Vec<&str> = c.visible().iter().map(|(_, e)| e.name).collect();
        assert!(names.contains(&"DOM gallery") && names.contains(&"Monitor (web)"), "{names:?}");
        assert!(!names.contains(&"Two windows"));
    }
}
