//! DevTools-like inspector overlay (DOM-like roadmap item 3). Compiled only with the
//! `inspector` cargo feature, so a release/wasm build without it carries none of this.
//!
//! The app records the boxes it lays out each frame ([`Inspector::record`]); the
//! inspector picks the innermost box under the pointer, paints the box model in Chrome's
//! colours (margin / border / padding / content), pins a box on click, and draws a side
//! panel with the id, kind, rect, notes (e.g. the resolved style trace from
//! [`crate::ui::style::Sheet::resolve_traced`]) and the hit chain (outermost first).

use crate::Rect;
use crate::render::{RenderContext, TextAlign, TextBaseline};

/// One laid-out box.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InspectBox {
    pub id: String,
    pub kind: String,
    /// Border box.
    pub rect: Rect,
    /// top, right, bottom, left
    pub padding: [f64; 4],
    pub margin: [f64; 4],
    pub border: f64,
    /// Extra lines for the panel (style trace, scroll offsets, ...).
    pub notes: Vec<String>,
}

impl InspectBox {
    pub fn new(id: impl Into<String>, kind: impl Into<String>, rect: Rect) -> Self {
        InspectBox { id: id.into(), kind: kind.into(), rect, ..Default::default() }
    }
    fn contains(&self, x: f64, y: f64) -> bool {
        let r = self.rect;
        x >= r.x && y >= r.y && x < r.x + r.width && y < r.y + r.height
    }
}

/// Inspector state; keep one per window and feed it input before the app sees it.
#[derive(Clone, Debug, Default)]
pub struct Inspector {
    pub enabled: bool,
    boxes: Vec<InspectBox>,
    pointer: Option<(f64, f64)>,
    pinned: Option<InspectBox>,
}

const MARGIN_C: &str = "rgba(246,178,107,0.66)";
const BORDER_C: &str = "rgba(255,229,153,0.66)";
const PADDING_C: &str = "rgba(147,196,125,0.55)";
const CONTENT_C: &str = "rgba(111,168,220,0.66)";
const PANEL_BG: &str = "rgba(20,20,24,0.94)";
const PANEL_TXT: &str = "#e8eaed";
const PANEL_SUB: &str = "#9aa0a6";

impl Inspector {
    /// F12: toggle; unpins when turned off.
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
        if !self.enabled {
            self.pinned = None;
        }
    }
    /// Call at the start of each frame, before the app records its boxes.
    pub fn begin_frame(&mut self) {
        self.boxes.clear();
    }
    pub fn record(&mut self, b: InspectBox) {
        if self.enabled {
            self.boxes.push(b);
        }
    }
    pub fn pointer_move(&mut self, x: f64, y: f64) {
        self.pointer = Some((x, y));
    }
    /// A click while enabled pins (or unpins) the box under the pointer and is swallowed:
    /// returns `true` when the app must not see it.
    pub fn click(&mut self, x: f64, y: f64) -> bool {
        if !self.enabled {
            return false;
        }
        let hit = self.pick(x, y).cloned();
        self.pinned = if hit.as_ref().map(|h| &h.id) == self.pinned.as_ref().map(|p| &p.id) { None } else { hit };
        true
    }
    /// The innermost box at a point: the smallest area, later recordings win ties.
    pub fn pick(&self, x: f64, y: f64) -> Option<&InspectBox> {
        let mut best: Option<&InspectBox> = None;
        for b in self.boxes.iter().filter(|b| b.contains(x, y)) {
            if best.is_none_or(|o| b.rect.width * b.rect.height <= o.rect.width * o.rect.height) {
                best = Some(b);
            }
        }
        best
    }
    /// Every box under a point, outermost first (the hit chain).
    pub fn chain(&self, x: f64, y: f64) -> Vec<&InspectBox> {
        let mut v: Vec<&InspectBox> = self.boxes.iter().filter(|b| b.contains(x, y)).collect();
        v.sort_by(|a, b| (b.rect.width * b.rect.height).total_cmp(&(a.rect.width * a.rect.height)));
        v
    }
    /// The box the overlay describes: the pinned one, else the one under the pointer.
    pub fn target(&self) -> Option<&InspectBox> {
        self.pinned.as_ref().or_else(|| self.pointer.and_then(|(x, y)| self.pick(x, y)))
    }

    /// Paint the overlay on top of the frame; `view` is the window rect.
    pub fn paint(&self, ctx: &mut dyn RenderContext, view: Rect) {
        if !self.enabled {
            return;
        }
        let Some(b) = self.target() else { return };
        let r = b.rect;
        let [mt, mr, mb, ml] = b.margin;
        let [pt, pr, pb, pl] = b.padding;
        let bw = b.border;
        // Each layer is the ring between an outer and an inner rect.
        let ring = |ctx: &mut dyn RenderContext, o: Rect, i: Rect, c: &str| {
            ctx.set_fill_color(c);
            ctx.fill_rect(o.x, o.y, o.width, i.y - o.y);
            ctx.fill_rect(o.x, i.y + i.height, o.width, o.y + o.height - i.y - i.height);
            ctx.fill_rect(o.x, i.y, i.x - o.x, i.height);
            ctx.fill_rect(i.x + i.width, i.y, o.x + o.width - i.x - i.width, i.height);
        };
        let inset = |r: Rect, t: f64, rt: f64, bt: f64, l: f64| Rect::new(r.x + l, r.y + t, (r.width - l - rt).max(0.0), (r.height - t - bt).max(0.0));
        let outer = Rect::new(r.x - ml, r.y - mt, r.width + ml + mr, r.height + mt + mb);
        let pad = inset(r, bw, bw, bw, bw);
        let content = inset(pad, pt, pr, pb, pl);
        ring(ctx, outer, r, MARGIN_C);
        ring(ctx, r, pad, BORDER_C);
        ring(ctx, pad, content, PADDING_C);
        ctx.set_fill_color(CONTENT_C);
        ctx.fill_rect(content.x, content.y, content.width, content.height);

        let mut lines: Vec<(String, bool)> = vec![
            (format!("{}#{}{}", b.kind, b.id, if self.pinned.is_some() { "  (pinned)" } else { "" }), true),
            (format!("{:.1} × {:.1} at ({:.1}, {:.1})", r.width, r.height, r.x, r.y), false),
            (format!("padding {pt} {pr} {pb} {pl} · border {bw} · margin {mt} {mr} {mb} {ml}"), false),
        ];
        lines.extend(b.notes.iter().map(|n| (n.clone(), false)));
        let (cx, cy) = (r.x + r.width / 2.0, r.y + r.height / 2.0);
        let chain: Vec<String> = self.chain(cx, cy).iter().map(|c| format!("{}#{}", c.kind, c.id)).collect();
        if !chain.is_empty() {
            lines.push((format!("chain: {}", chain.join(" › ")), false));
        }
        let (pw, lh) = (360.0f64.min(view.width), 17.0);
        let ph = 12.0 + lines.len() as f64 * lh;
        let (px, py) = (view.x + view.width - pw - 8.0, view.y + view.height - ph - 8.0);
        ctx.set_fill_color(PANEL_BG);
        ctx.fill_rect(px, py, pw, ph);
        ctx.set_text_align(TextAlign::Left);
        ctx.set_text_baseline(TextBaseline::Top);
        for (i, (t, head)) in lines.iter().enumerate() {
            ctx.set_font(if *head { "600 12px Inter" } else { "400 11px Inter" });
            ctx.set_fill_color(if *head { PANEL_TXT } else { PANEL_SUB });
            ctx.fill_text(t, px + 8.0, py + 6.0 + i as f64 * lh);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insp() -> Inspector {
        let mut i = Inspector::default();
        i.toggle();
        i.begin_frame();
        i.record(InspectBox::new("modal", "card", Rect::new(0.0, 0.0, 400.0, 300.0)));
        i.record(InspectBox::new("roles", "grid", Rect::new(10.0, 50.0, 380.0, 200.0)));
        i.record(InspectBox::new("dev", "card", Rect::new(20.0, 60.0, 180.0, 100.0)));
        i
    }

    #[test]
    fn picks_innermost_and_reports_chain() {
        let i = insp();
        assert_eq!(i.pick(30.0, 70.0).unwrap().id, "dev");
        assert_eq!(i.pick(300.0, 70.0).unwrap().id, "roles");
        assert_eq!(i.pick(5.0, 5.0).unwrap().id, "modal");
        assert!(i.pick(500.0, 5.0).is_none());
        let ids: Vec<_> = i.chain(30.0, 70.0).iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, ["modal", "roles", "dev"]);
    }

    #[test]
    fn click_pins_and_is_swallowed_only_when_enabled() {
        let mut i = insp();
        i.pointer_move(300.0, 70.0);
        assert_eq!(i.target().unwrap().id, "roles");
        assert!(i.click(30.0, 70.0));
        i.pointer_move(5.0, 5.0);
        assert_eq!(i.target().unwrap().id, "dev", "pinned box survives pointer moves");
        assert!(i.click(30.0, 70.0), "clicking the pinned box unpins");
        assert_eq!(i.target().unwrap().id, "modal");
        i.toggle();
        assert!(!i.click(30.0, 70.0));
        i.begin_frame();
        i.record(InspectBox::new("x", "y", Rect::new(0.0, 0.0, 1.0, 1.0)));
        assert!(i.pick(0.5, 0.5).is_none(), "nothing is recorded while disabled");
    }
}
