//! Critter avatar: a soft body shape with two tilted eyes (Hatchery HQ style).
//!
//! - [`CritterFamily`] picks the shape set; chat kinds differ by shape, not by label.
//! - [`critter_view`] derives shape, colour and eye height from a stable name.
//! - [`draw_critter`] paints into any [`RenderContext`]; [`register_critter`] declares the hit rect.

use crate::input::core::coordinator::LayerId;
use crate::input::{InputCoordinator, Sense, WidgetKind};
use crate::render::{emit_svg_path, RenderContext};
use crate::types::{Rect, WidgetId};

/// Body outlines in a 64x64 box (SVG path data).
pub const CRITTER_SHAPES: [&str; 7] = [
    "M32 5c15 0 27 11 27 27S47 59 32 59 5 47 5 32 17 5 32 5z",
    "M32 4c16 0 28 10 28 26 0 18-10 30-28 30S4 48 4 30C4 14 16 4 32 4z",
    "M32 6c5 0 26 38 26 46 0 6-6 8-26 8S6 58 6 52C6 44 27 6 32 6z",
    "M16 6h32c6 0 10 4 10 10v32c0 6-4 10-10 10H16C10 58 6 54 6 48V16C6 10 10 6 16 6z",
    "M23 5h18l17 17v20L41 59H23L6 42V22z",
    "M30 5c14-2 29 9 28 27-1 17-13 27-28 26C14 57 4 46 6 30 7 16 17 6 30 5z",
    "M32 4c6 0 9 6 15 8s12 4 12 12c0 6-4 9-4 15 0 12-9 21-23 21S9 51 9 39c0-6-4-9-4-15 0-8 6-10 12-12s9-8 15-8z",
];

/// Body colours.
pub const CRITTER_PALETTE: [&str; 10] = [
    "#1677ff", "#ff7a1a", "#ff3d8b", "#8b5cf6", "#14b8a6", "#a855f7", "#22c55e", "#f5b014",
    "#ef4444", "#06b6d4",
];

/// Which kind of chat the critter stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CritterFamily {
    /// Direct message with a person.
    Dm,
    /// Group room.
    Group,
    /// Agent / PTY session.
    Agent,
    /// Node.
    Node,
}

impl CritterFamily {
    /// Shape indices this family draws from.
    pub fn shapes(self) -> &'static [usize] {
        match self {
            CritterFamily::Dm => &[0, 5],
            CritterFamily::Group => &[3],
            CritterFamily::Agent => &[1, 2, 6],
            CritterFamily::Node => &[4],
        }
    }
}

/// Plain view data for one critter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CritterView {
    /// Index into [`CRITTER_SHAPES`].
    pub shape: usize,
    /// Body fill colour (CSS).
    pub color: &'static str,
    /// Eye centre y in the 64-box.
    pub eye_y: f64,
    /// Half-closed, faded (session off).
    pub off: bool,
}

/// FNV-1a over UTF-16 units (matches the HQ JS `hashOf`).
pub fn critter_hash(name: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for c in name.encode_utf16() {
        h = (h ^ c as u32).wrapping_mul(16777619);
    }
    h
}

fn color_hash(name: &str) -> u32 {
    let mut h: u32 = 0;
    for c in name.encode_utf16() {
        h = h.wrapping_mul(31).wrapping_add(c as u32);
    }
    h
}

/// Stable view from a name and family (same picks as the HQ JS `avatar`).
pub fn critter_view(name: &str, family: CritterFamily) -> CritterView {
    let h = critter_hash(name);
    let set = family.shapes();
    CritterView {
        shape: set[((h >> 8) as usize) % set.len()],
        color: CRITTER_PALETTE[(color_hash(name) as usize) % CRITTER_PALETTE.len()],
        eye_y: 28.0 + ((h >> 4) % 5) as f64 - 2.0,
        off: false,
    }
}

/// Paint a critter scaled into `rect` (square fit, centred).
pub fn draw_critter(ctx: &mut dyn RenderContext, rect: Rect, view: &CritterView) {
    let s = rect.width.min(rect.height) / 64.0;
    let ox = rect.x + (rect.width - 64.0 * s) / 2.0;
    let oy = rect.y + (rect.height - 64.0 * s) / 2.0;
    ctx.save();
    ctx.translate(ox, oy);
    ctx.scale(s, s);
    if view.off {
        ctx.set_global_alpha(0.5);
    }
    emit_svg_path(ctx, CRITTER_SHAPES[view.shape % CRITTER_SHAPES.len()]);
    ctx.set_fill_color(view.color);
    ctx.fill();
    ctx.set_fill_color("#0b0b0d");
    let ry = if view.off { 0.72 } else { 6.0 };
    for cx in [23.0, 41.0] {
        ctx.begin_path();
        ctx.ellipse(cx, view.eye_y, 4.0, ry, -12f64.to_radians(), 0.0, std::f64::consts::TAU);
        ctx.fill();
    }
    ctx.restore();
}

/// Declare the critter's click rect for this frame.
pub fn register_critter(coord: &mut InputCoordinator, id: impl Into<WidgetId>, rect: Rect, layer: &LayerId) {
    coord.register_atomic(id, WidgetKind::Button, rect, Sense::CLICK, layer);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn view_is_stable_and_in_family() {
        let a = critter_view("pty:node-a-1", CritterFamily::Agent);
        assert_eq!(a, critter_view("pty:node-a-1", CritterFamily::Agent));
        assert!(CritterFamily::Agent.shapes().contains(&a.shape));
        assert_eq!(critter_view("x", CritterFamily::Group).shape, 3);
        assert!((26.0..=30.0).contains(&a.eye_y));
    }
}
