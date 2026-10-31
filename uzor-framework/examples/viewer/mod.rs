//! The image viewer app: a DOM library (header, tile-size switch, a card grid) and a
//! DOM lightbox (top bar, prev / next, a thumbnail strip). The DOM has no raster
//! images, so every picture is an embed slot (`k::slot`) the app paints with the
//! render context's image painter after the tree (cover-cropped tiles, a contained
//! full image).
#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use uzor::input::keyboard::KeyCode;
use uzor::layout::docking::DockPanel;
use uzor::render::{InvalidateBits, RenderContext, TickRate};
use uzor::tokens::{BuiltinSet, Tokens};
use uzor::ui::dom::El;
use uzor::ui::domkit::{self as k, icons};
use uzor::ui::style::Sheet;
use uzor::ui::widgets::composite::image_viewer as iv;
use uzor::Rect;
use uzor_framework::widgets::kit;
use uzor_framework::{
    App, AppCommand, CadenceCmd, ChromeModel, DockTarget, FloatingControls, Handle, InitCx, Intent, IntentCx, KeyState, LayoutCmd,
    OverlayCx, OverlayModel, PanelCx, PanelHome, RuntimeConfig, SizePx, Spec, UnhandledInput, WindowControlsStyle,
    WindowId, WindowSpec,
};

#[path = "../common/look.rs"]
mod look;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Root;
impl DockPanel for Root {
    fn title(&self) -> &str {
        "Viewer"
    }
    fn type_id(&self) -> &'static str {
        "viewer"
    }
    fn shows_header(&self) -> bool {
        false
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ov {}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Act {}
pub struct S;
impl Spec for S {
    type Panel = Root;
    type Overlay = Ov;
    type Action = Act;
    fn decode_panel(_h: PanelHome, _t: &str) -> Option<Root> {
        Some(Root)
    }
}

/// One picture.
pub struct Img {
    pub name: String,
    pub path: String,
    pub rgba: Vec<u8>,
    pub w: u32,
    pub h: u32,
}

const CSS: &str = r#"
div.v-head { display: flex; align-items: center; gap: 12px; height: 40px; flex-shrink: 0 }
div.v-grid { display: grid; gap: 16px }
button.v-card { display: flex; flex-direction: column; gap: 0px; border-radius: 12px; background: var(--k-panel);
                border: 1px solid var(--k-line); overflow: hidden; padding: 0px }
button.v-card:hover { border: 1px solid var(--k-accent) }
button.v-card:focus-visible { border: 2px solid var(--k-accent) }
div.v-meta { display: flex; align-items: center; gap: 8px; padding: 10px 12px }
div.v-name { font-weight: 600; flex-grow: 1 }
div.v-dim { color: var(--k-muted); font-size: 12px }
div.lb { display: flex; flex-direction: column; background: #060708; color: #e8e8ea }
div.lb-top { display: flex; align-items: center; gap: 12px; height: 52px; padding: 0px 16px; flex-shrink: 0 }
div.lb-stage { display: flex; align-items: center; gap: 12px; padding: 0px 16px; flex-grow: 1 }
button.lb-nav { width: 44px; height: 44px; border-radius: 22px; background: #1b1d22; color: #e8e8ea; flex-shrink: 0;
                display: flex; align-items: center; justify-content: center }
button.lb-nav:hover { background: #2a2d34 }
button.lb-ib { width: 34px; height: 34px; border-radius: 8px; color: #e8e8ea; display: flex; align-items: center; justify-content: center }
button.lb-ib:hover { background: #22252b }
div.lb-strip { display: flex; justify-content: center; gap: 8px; height: 92px; padding: 12px 0px; flex-shrink: 0 }
button.lb-thumb { border-radius: 8px; border: 2px solid transparent; overflow: hidden; padding: 0px; opacity: 0.6 }
button.lb-thumb:hover { opacity: 1 }
button.lb-thumb.on { border: 2px solid var(--k-accent); opacity: 1 }
span.lb-i { width: 18px; height: 18px }
div.k-slot { border: none; background: transparent }
"#;

const SIZES: [(&str, f64); 3] = [("S", 180.0), ("M", 250.0), ("L", 340.0)];

/// The viewer app.
pub struct Viewer {
    pub imgs: Vec<Img>,
    pub open: Arc<Mutex<Option<usize>>>,
    pub size: usize,
    handle: Option<Handle<S>>,
    win: Option<WindowId>,
    sheet: Sheet,
    samples: bool,
}

fn controls() -> FloatingControls {
    FloatingControls {
        border: Some("#353535".into()),
        ..FloatingControls::new(WindowControlsStyle::default().with_fill(Some("#0e0e0e")))
    }
}

fn ib(id: &str, name: &str, body: &str, class: &str) -> El {
    El::new(format!("button.{class}")).id(id).tab().name(name).child(El::new("span.lb-i").svg(k::icon(body)))
}

/// Cover `r` with a `w × h` picture (cropped by the caller's clip).
fn cover(r: Rect, w: u32, h: u32) -> Rect {
    let s = (r.width / w.max(1) as f64).max(r.height / h.max(1) as f64);
    let (dw, dh) = (w as f64 * s, h as f64 * s);
    Rect::new(r.x + (r.width - dw) / 2.0, r.y + (r.height - dh) / 2.0, dw, dh)
}

/// Fit a `w × h` picture inside `r`.
fn contain(r: Rect, w: u32, h: u32) -> Rect {
    let s = (r.width / w.max(1) as f64).min(r.height / h.max(1) as f64);
    let (dw, dh) = (w as f64 * s, h as f64 * s);
    Rect::new(r.x + (r.width - dw) / 2.0, r.y + (r.height - dh) / 2.0, dw, dh)
}

fn draw(ctx: &mut dyn RenderContext, clip: Rect, at: Rect, im: &Img) {
    ctx.save();
    ctx.begin_path();
    ctx.rect(clip.x, clip.y, clip.width, clip.height);
    ctx.clip();
    if let Some(ip) = ctx.image_painter() {
        ip.draw_image_rgba(&im.rgba, im.w, im.h, at.x, at.y, at.width, at.height);
    }
    ctx.restore();
}

impl Viewer {
    pub fn new(imgs: Vec<Img>, open: Option<usize>, samples: bool) -> Self {
        Self { imgs, open: Arc::new(Mutex::new(open)), size: 1, handle: None, win: None, sheet: look::sheet(CSS), samples }
    }

    pub fn config() -> RuntimeConfig {
        let mut s = WindowSpec::new("main", "Image viewer", SizePx::new(1280, 800));
        s.decorations = false;
        s.border_color = Some(0x353535);
        s.transparent = true;
        s.corner_style = uzor::CornerStyle::Rounded;
        // Paint on demand: input and the app's invalidates schedule frames, idle costs nothing.
        RuntimeConfig { windows: vec![s], tokens: Tokens::builtin(BuiltinSet::Dark), dark: true, tick: TickRate::Dirty, ..RuntimeConfig::default() }
    }

    fn repaint(&self) {
        if let (Some(h), Some(w)) = (&self.handle, self.win) {
            let _ = h.request_repaint(w);
        }
    }

    fn repaint_now(&self, cx: &mut IntentCx<'_, S>) {
        if let Some(win) = self.win {
            cx.command(AppCommand::Cadence(CadenceCmd::Invalidate { win, region: None, bits: InvalidateBits::ALL }));
        }
    }

    fn reserve() -> f64 {
        controls().style.button_w * 3.0 + 12.0
    }

    /// The library page `w × h`.
    pub fn library(&self, w: f64, h: f64) -> El {
        let gap = 16.0;
        let want = SIZES[self.size].1;
        let inner = w - 48.0;
        let cols = ((inner + gap) / (want + gap)).floor().max(1.0) as usize;
        let cw = ((inner - gap * (cols - 1) as f64) / cols as f64).floor();
        let mut grid = El::new("div.v-grid").css(format!("grid-template-columns: {}", vec!["1fr"; cols].join(" ")));
        for (i, im) in self.imgs.iter().enumerate() {
            grid = grid.child(
                El::new("button.v-card").id(format!("tile:{i}")).tab().name(im.name.clone())
                    .child(k::slot(&format!("tile:{i}"), cw - 2.0, (cw * 0.62).round()))
                    .child(
                        El::new("div.v-meta")
                            .child(El::new("div.v-name").text(im.name.clone()))
                            .child(El::new("div.v-dim").text(format!("{} × {}", im.w, im.h))),
                    ),
            );
        }
        let sub = if self.samples { "built-in samples · pass PNG paths to view yours" } else { "from the command line" };
        look::root(true, h).child(
            El::new("div.p-page").css(format!("height: {h}px; padding: 16px 24px; gap: 16px"))
                .child(
                    El::new("div.v-head").css(format!("padding-right: {}px", Self::reserve()))
                        .child(El::new("span.p-icon.accent").css("width: 22px; height: 22px").svg(k::icon(icons::IMAGE)))
                        .child(El::new("div.p-h1").css("font-size: 20px").text("Library"))
                        .child(k::badge(&format!("{} images", self.imgs.len()), "info"))
                        .child(El::new("div.p-muted").text(sub))
                        .child(El::new("div.p-grow"))
                        .child(El::new("div.p-muted").text("Tile"))
                        .child(k::segmented("iv-size", &SIZES.map(|s| s.0), self.size)),
                )
                .child_if(self.imgs.is_empty(), || El::new("div.p-muted").text("No images: pass PNG paths on the command line."))
                .child(grid),
        )
    }

    /// The lightbox page over picture `sel`.
    pub fn lightbox(&self, sel: usize, w: f64, h: f64) -> El {
        let im = &self.imgs[sel];
        let n = self.imgs.len();
        let stage_h = h - 52.0 - 92.0;
        let img_w = w - 32.0 - 2.0 * 44.0 - 24.0;
        let mut strip = El::new("div.lb-strip");
        for (i, t) in self.imgs.iter().enumerate() {
            let tw = (64.0 * t.w as f64 / t.h.max(1) as f64).clamp(48.0, 112.0).round();
            strip = strip.child(
                El::new(if i == sel { "button.lb-thumb.on" } else { "button.lb-thumb" }).id(format!("iv:thumb:{i}")).tab()
                    .name(t.name.clone()).child(k::slot(&format!("thumb:{i}"), tw, 64.0)),
            );
        }
        El::new("div.k-root").css(format!("height: {h}px")).child(
            El::new("div.lb").id("iv:backdrop").css(format!("height: {h}px"))
                .child(
                    El::new("div.lb-top").css(format!("padding-right: {}px", Self::reserve()))
                        .child(ib("iv:back", "Back to the library", icons::LEFT, "lb-ib"))
                        .child(El::new("div").css("font-weight: 600").text(im.name.clone()))
                        .child(El::new("div").css("color: #8b8f98").text(format!("{} × {} · {} / {n}", im.w, im.h, sel + 1)))
                        .child(El::new("div.p-grow"))
                        .child(ib("iv:download", "Download", icons::DOWNLOAD, "lb-ib"))
                        .child(ib("iv:close", "Close", icons::CLOSE, "lb-ib")),
                )
                .child(
                    El::new("div.lb-stage").css(format!("height: {stage_h}px"))
                        .child(ib("iv:prev", "Previous", icons::LEFT, "lb-nav"))
                        .child(k::slot(&format!("full:{sel}"), img_w, stage_h).id("iv:image"))
                        .child(ib("iv:next", "Next", icons::RIGHT, "lb-nav")),
                )
                .child(strip),
        )
    }

    fn paint_slots(&self, ctx: &mut dyn RenderContext, dom: &uzor::ui::dom::Dom, clip: Rect) {
        for (key, r) in dom.slots() {
            let Some((kind, i)) = key.split_once(':') else { continue };
            let Some(im) = i.parse::<usize>().ok().and_then(|i| self.imgs.get(i)) else { continue };
            let vis = r.intersect(clip);
            if vis.width <= 0.0 || vis.height <= 0.0 {
                continue;
            }
            match kind {
                "full" => draw(ctx, vis, contain(r, im.w, im.h), im),
                _ => draw(ctx, vis, cover(r, im.w, im.h), im),
            }
        }
    }
}

impl App for Viewer {
    type Spec = S;
    fn init(&mut self, cx: &mut InitCx<'_, S>) {
        self.handle = Some(cx.handle());
    }
    fn chrome_model(&self, _w: WindowId) -> ChromeModel {
        ChromeModel { floating: Some(controls()), ..Default::default() }
    }
    fn overlay_model(&self, _w: WindowId, _id: Ov) -> OverlayModel {
        OverlayModel::default()
    }
    fn overlay_body(&mut self, _cx: &mut OverlayCx<'_, S>) {}

    fn panel(&mut self, cx: &mut PanelCx<'_, S>) {
        let r = cx.rect;
        let n = self.imgs.len();
        let open = (*self.open.lock().unwrap()).filter(|&s| s < n);
        let tree = match open {
            Some(sel) => self.lightbox(sel, r.width, r.height),
            None => self.library(r.width, r.height),
        };
        let resp = kit::dom(r, &self.sheet, tree).build(cx);
        self.paint_slots(cx.render, &resp.dom, r);
        let mut next = open;
        if let Some(c) = resp.clicked.as_deref() {
            if let Some(i) = c.strip_prefix("tile:") {
                next = i.parse().ok();
            } else if let Some(i) = c.strip_prefix("iv:thumb:") {
                next = i.parse().ok();
            } else if let Some(i) = k::picked("iv-size", "", c) {
                self.size = i;
            } else {
                match (c, open) {
                    ("iv:close" | "iv:back" | "iv:backdrop", _) => next = None,
                    ("iv:prev", Some(s)) => next = Some(s.saturating_sub(1)),
                    ("iv:next", Some(s)) => next = Some((s + 1).min(n - 1)),
                    ("iv:download", Some(s)) => println!("download {}", self.imgs[s].path),
                    _ => {}
                }
            }
            *self.open.lock().unwrap() = next;
            cx.out.request_frame();
        }
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
                    self.repaint_now(cx);
                } else if key.code == KeyCode::Enter && !self.imgs.is_empty() {
                    *o = Some(0);
                    drop(o);
                    self.repaint_now(cx);
                }
            }
            _ => {}
        }
    }
}

// -- built-in samples ---------------------------------------------------------

fn mix(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    let t = t.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn gen(name: &str, w: u32, h: u32, f: impl Fn(f64, f64) -> [f64; 3]) -> Img {
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let c = f(x as f64 / w as f64, y as f64 / h as f64);
            rgba.extend_from_slice(&[c[0].clamp(0.0, 255.0) as u8, c[1].clamp(0.0, 255.0) as u8, c[2].clamp(0.0, 255.0) as u8, 255]);
        }
    }
    Img { name: name.into(), path: format!("sample:{name}"), rgba, w, h }
}

/// Eight generated pictures (gradients, ridges, rings) so the viewer shows something
/// without arguments.
pub fn samples() -> Vec<Img> {
    use std::f64::consts::PI;
    vec![
        gen("Aurora", 640, 360, |x, y| {
            let band = ((x * 6.0 + (y * 3.0).sin() * 1.5).sin() * 0.5 + 0.5) * (1.0 - y).powi(2);
            mix(mix([8.0, 12.0, 40.0], [10.0, 60.0, 90.0], y), [60.0, 230.0, 160.0], band * 0.8)
        }),
        gen("Dunes", 600, 400, |x, y| {
            let sky = mix([255.0, 140.0, 60.0], [90.0, 40.0, 120.0], y * 1.6);
            let ridge = 0.62 + (x * 7.0).sin() * 0.05 + (x * 17.0).sin() * 0.02;
            if y > ridge { mix([120.0, 50.0, 40.0], [40.0, 15.0, 30.0], (y - ridge) * 3.0) } else { sky }
        }),
        gen("Rings", 420, 420, |x, y| {
            let d = ((x - 0.5).powi(2) + (y - 0.5).powi(2)).sqrt();
            let t = (d * 40.0).sin() * 0.5 + 0.5;
            mix([20.0, 24.0, 40.0], mix([90.0, 140.0, 255.0], [255.0, 110.0, 180.0], d * 1.6), t * (1.0 - d))
        }),
        gen("Plasma", 560, 420, |x, y| {
            let v = (x * 10.0).sin() + (y * 8.0 + x * 4.0).sin() + ((x * x + y * y).sqrt() * 14.0).sin();
            let t = v / 6.0 + 0.5;
            [128.0 + 127.0 * (PI * t * 2.0).sin(), 128.0 + 127.0 * (PI * t * 2.0 + 2.0).sin(), 128.0 + 127.0 * (PI * t * 2.0 + 4.0).sin()]
        }),
        gen("Peaks", 720, 320, |x, y| {
            let sky = mix([170.0, 200.0, 235.0], [240.0, 225.0, 210.0], y * 1.4);
            let far = 0.45 + (x * 9.0).sin().abs() * -0.12 + (x * 23.0).sin() * 0.03;
            let near = 0.7 + ((x * 5.0 + 1.0).sin()) * 0.1;
            if y > near { [40.0, 55.0, 70.0] } else if y > far { [95.0, 115.0, 140.0] } else { sky }
        }),
        gen("Tiles", 400, 400, |x, y| {
            let (cx, cy) = ((x * 8.0) as i32, (y * 8.0) as i32);
            let on = (cx + cy) % 2 == 0;
            let hue = mix([60.0, 200.0, 190.0], [250.0, 190.0, 70.0], (cx + cy) as f64 / 14.0);
            if on { hue } else { [28.0, 30.0, 36.0] }
        }),
        gen("Bloom", 360, 500, |x, y| {
            let d = ((x - 0.5).powi(2) + (y - 0.4).powi(2) * 0.6).sqrt();
            let petals = ((y - 0.4).atan2(x - 0.5) * 6.0).cos() * 0.08;
            if d < 0.22 + petals { mix([255.0, 240.0, 120.0], [240.0, 90.0, 140.0], d * 4.0) } else { mix([30.0, 60.0, 50.0], [10.0, 20.0, 18.0], y) }
        }),
        gen("Horizon", 640, 360, |x, y| {
            if y < 0.5 {
                let sun = ((x - 0.5).powi(2) + (y - 0.42).powi(2) * 3.0).sqrt() < 0.12;
                if sun { [255.0, 200.0, 90.0] } else { mix([20.0, 10.0, 50.0], [230.0, 70.0, 140.0], y * 2.0) }
            } else {
                let gy = 1.0 / (y - 0.48);
                let line = (gy * 2.0).fract() < 0.08 || (((x - 0.5) * gy * 2.0).fract().abs() < 0.04);
                if line { [255.0, 90.0, 200.0] } else { [15.0, 8.0, 30.0] }
            }
        }),
    ]
}

/// Load PNGs from `args` (`--open=N` opens picture N); built-in samples when none.
pub fn from_args(args: impl Iterator<Item = String>) -> Viewer {
    let mut imgs = Vec::new();
    let mut start = None;
    for a in args {
        if let Some(v) = a.strip_prefix("--open=") {
            start = v.parse().ok();
            continue;
        }
        let Ok(p) = tiny_skia::Pixmap::load_png(&a) else {
            eprintln!("skip {a}: not a PNG");
            continue;
        };
        let name = std::path::Path::new(&a).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        imgs.push(Img { name, path: a.clone(), w: p.width(), h: p.height(), rgba: p.take() });
    }
    let generated = imgs.is_empty();
    if generated {
        imgs = samples();
    }
    Viewer::new(imgs, start, generated)
}
