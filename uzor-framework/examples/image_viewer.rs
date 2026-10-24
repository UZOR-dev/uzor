//! Image viewer (lightbox) demo over a gallery of PNGs.
//!
//! ```text
//! cargo run -p uzor-framework --example image_viewer --features native -- a.png b.png …
//! ```
//!
//! Click a tile to open; Left / Right or a thumbnail switches, Esc / X /
//! backdrop closes, the download button prints the file path.

use std::sync::{Arc, Mutex};
use uzor::input::{Sense, WidgetKind};
use uzor::input::keyboard::KeyCode;
use uzor::layout::docking::DockPanel;
use uzor::render::{RenderContext, TextAlign, TextBaseline};
use uzor::tokens::{BuiltinSet, Tokens};
use uzor::ui::widgets::composite::image_viewer::{self as iv, ViewerImage, ViewerStyle};
use uzor::{Rect, WidgetId};
use uzor_framework::host::native::run_native_default;
use uzor_framework::{
    App, AppCommand, ChromeModel, DockTarget, FloatingControls, Handle, InitCx, Intent, IntentCx, KeyState, LayoutCmd, OverlayCx,
    OverlayModel, PanelCx, PanelHome, RuntimeConfig, SizePx, Spec, UnhandledInput, WindowControlsStyle, WindowId, WindowSpec,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Root;
impl DockPanel for Root {
    fn title(&self) -> &str { "Viewer" }
    fn type_id(&self) -> &'static str { "viewer" }
    fn shows_header(&self) -> bool { false }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Ov {}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Act {}
struct S;
impl Spec for S {
    type Panel = Root;
    type Overlay = Ov;
    type Action = Act;
    fn decode_panel(_h: PanelHome, _t: &str) -> Option<Root> { Some(Root) }
}

struct Img { name: String, path: String, rgba: Vec<u8>, w: u32, h: u32 }

struct Demo {
    imgs: Vec<Img>,
    open: Arc<Mutex<Option<usize>>>,
    handle: Option<Handle<S>>,
    win: Option<WindowId>,
}

fn controls() -> FloatingControls {
    FloatingControls { border: Some("#353535".into()), ..FloatingControls::new(WindowControlsStyle::default().with_fill(Some("#0e0e0e"))) }
}

fn hit(cx: &mut PanelCx<'_, S>, wid: String, r: Rect) {
    let layer = cx.widgets.layer.clone();
    cx.widgets.coord.register_atomic(WidgetId::from(wid.as_str()), WidgetKind::Button, r, Sense::CLICK, &layer);
}

impl Demo {
    fn repaint(&self) {
        if let (Some(h), Some(w)) = (&self.handle, self.win) { let _ = h.request_repaint(w); }
    }
}

impl App for Demo {
    type Spec = S;
    fn init(&mut self, cx: &mut InitCx<'_, S>) { self.handle = Some(cx.handle()); }
    fn chrome_model(&self, _w: WindowId) -> ChromeModel { ChromeModel { floating: Some(controls()), ..Default::default() } }
    fn overlay_model(&self, _w: WindowId, _id: Ov) -> OverlayModel { OverlayModel::default() }
    fn overlay_body(&mut self, _cx: &mut OverlayCx<'_, S>) {}

    fn panel(&mut self, cx: &mut PanelCx<'_, S>) {
        let r = cx.rect;
        let st = ViewerStyle::default();
        let n = self.imgs.len();
        let reserve = controls().style.button_w * 3.0;
        let top = controls().style.button_h;
        let clicks: Vec<String> = cx.view.clicks().iter().map(|(w, _)| w.as_str().to_string()).collect();
        let mut open = *self.open.lock().unwrap();
        for c in &clicks {
            if let Some(i) = c.strip_prefix("tile:") { open = i.parse().ok(); }
            match c.as_str() {
                "iv:close" | "iv:backdrop" => open = None,
                "iv:download" => if let Some(i) = open { println!("download {}", self.imgs[i].path); },
                c if c.starts_with("iv:thumb:") => open = c["iv:thumb:".len()..].parse().ok(),
                _ => {}
            }
        }
        *self.open.lock().unwrap() = open;
        if !clicks.is_empty() { cx.out.request_frame(); }
        let ctx: &mut dyn RenderContext = cx.render;
        // Gallery underneath.
        ctx.set_fill_color("#0b0b0b");
        ctx.fill_rect(r.x, r.y, r.width, r.height);
        ctx.set_font("600 15px Inter");
        ctx.set_fill_color("#e6e6e6");
        ctx.set_text_align(TextAlign::Left);
        ctx.set_text_baseline(TextBaseline::Middle);
        ctx.fill_text("Library", r.x + 24.0, r.y + 22.0);
        let (tw, gap) = (220.0, 16.0);
        let cols = ((r.width - 48.0 + gap) / (tw + gap)).floor().max(1.0) as usize;
        let mut tiles = Vec::new();
        for (i, im) in self.imgs.iter().enumerate() {
            let th = tw * im.h as f64 / im.w.max(1) as f64;
            let t = Rect::new(r.x + 24.0 + (i % cols) as f64 * (tw + gap), r.y + 56.0 + (i / cols) as f64 * (tw * 0.7 + 40.0), tw, th.min(tw * 0.7));
            uzor::ui::widgets::composite::frame_view::draw_frame(ctx, t, &im.rgba, im.w, im.h, "#151515");
            ctx.set_font("400 12px Inter");
            ctx.set_fill_color("#9a9a9a");
            ctx.fill_text(&im.name, t.x, t.y + t.height + 12.0);
            tiles.push(t);
        }
        let Some(sel) = open.filter(|&s| s < n) else {
            for (i, t) in tiles.into_iter().enumerate() { hit(cx, format!("tile:{i}"), t); }
            return;
        };
        let views: Vec<ViewerImage<'_>> = self.imgs.iter().map(|m| ViewerImage { rgba: &m.rgba, w: m.w, h: m.h, alt: &m.name }).collect();
        let l = iv::layout(r, (self.imgs[sel].w, self.imgs[sel].h), n, reserve, top, &st);
        let hov = cx.view.raw_pointer().map(|p| iv::hit(&l, (p.0, p.1)));
        iv::draw(cx.render, r, &l, &views, sel, hov, "400 13px Inter", &st);
        // Backdrop first so the parts registered after it win.
        hit(cx, "iv:backdrop".into(), r);
        hit(cx, "iv:image".into(), l.image);
        for (i, t) in l.thumbs.iter().enumerate() { hit(cx, format!("iv:thumb:{i}"), *t); }
        hit(cx, "iv:download".into(), l.download);
        hit(cx, "iv:close".into(), l.close);
    }

    fn intent(&mut self, intent: Intent<S>, cx: &mut IntentCx<'_, S>) {
        match intent {
            Intent::Window(uzor_framework::WindowIntent::Opened { win, .. }) => {
                cx.command(AppCommand::Layout(LayoutCmd::SetChrome { win, visible: false, height: 0.0 }));
                cx.command(AppCommand::Layout(LayoutCmd::OpenPanel { win, panel: Root, at: DockTarget::Root }));
                self.win = Some(win);
            }
            Intent::Unhandled(UnhandledInput::Key { key, .. }) if key.state != KeyState::Up => {
                let mut o = self.open.lock().unwrap();
                if let Some(s) = *o {
                    *o = iv::key_nav(s, self.imgs.len(), key.code);
                    drop(o);
                    self.repaint();
                } else if key.code == KeyCode::Enter && !self.imgs.is_empty() {
                    *o = Some(0);
                    drop(o);
                    self.repaint();
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), uzor_framework::FrameworkError> {
    let mut imgs = Vec::new();
    let mut start = None;
    for a in std::env::args().skip(1) {
        if let Some(v) = a.strip_prefix("--open=") { start = v.parse().ok(); continue; }
        let Ok(p) = tiny_skia::Pixmap::load_png(&a) else { eprintln!("skip {a}: not a PNG"); continue };
        let name = std::path::Path::new(&a).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        imgs.push(Img { name, path: a.clone(), w: p.width(), h: p.height(), rgba: p.take() });
    }
    let mut s = WindowSpec::new("main", "Image viewer", SizePx::new(1280, 800));
    s.decorations = false;
    s.border_color = Some(0x353535);
    s.transparent = true;
    s.corner_style = uzor::CornerStyle::Rounded;
    let cfg = RuntimeConfig { windows: vec![s], tokens: Tokens::builtin(BuiltinSet::Dark), dark: true, ..RuntimeConfig::default() };
    run_native_default(Demo { imgs, open: Arc::new(Mutex::new(start)), handle: None, win: None }, cfg)
}
