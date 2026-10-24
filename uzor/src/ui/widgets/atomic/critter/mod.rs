//! Critter avatar: a soft body shape with two tilted eyes (Hatchery HQ style).
//!
//! - [`CritterFamily`] picks the shape set; chat kinds differ by shape, not by label.
//! - [`critter_view`] derives shape, colour, eye height and blink phase from a stable name.
//! - [`draw_critter`] paints into any [`RenderContext`]; [`register_critter`] declares the hit rect.
//! - [`CritterPose`] is one frame (rotation, squash/stretch, body morph, 3/4 turn, eye
//!   size/shape/offset, lids); [`critter_pose`] animates a [`CritterState`] with tricks
//!   picked per critter; [`draw_critter_pose`] paints a pose and [`draw_critter_motion`]
//!   paints a state at a time `t`. [`critter_next_frame`] says when the next frame is due
//!   (idle and off are still); [`critter_look`] is the eye offset toward the pointer.

mod motion;
mod tokens;

pub use motion::{
    critter_pose, trick_for, trick_pose, CritterPose, CritterTrick, ATTENTION_TRICKS, TRICK_SLOT, WORKING_TRICKS,
};

use crate::input::core::coordinator::LayerId;
use crate::input::{InputCoordinator, Sense, WidgetKind};
use crate::render::{emit_svg_path, RenderContext};
use crate::types::{Rect, WidgetId};

/// Body outlines in a 64x64 box (SVG path data): circle, egg, pear, rounded square,
/// octagon, pebble, flower, ghost, rounded hexagon, squircle, blob, capsule, rounded
/// triangle. Every outline covers both eyes (x 19..45, y 20..36).
pub const CRITTER_SHAPES: [&str; 13] = [
    "M32 5c15 0 27 11 27 27S47 59 32 59 5 47 5 32 17 5 32 5z",
    "M32 4c16 0 28 10 28 26 0 18-10 30-28 30S4 48 4 30C4 14 16 4 32 4z",
    "M32 6c5 0 26 38 26 46 0 6-6 8-26 8S6 58 6 52C6 44 27 6 32 6z",
    "M16 6h32c6 0 10 4 10 10v32c0 6-4 10-10 10H16C10 58 6 54 6 48V16C6 10 10 6 16 6z",
    "M23 5h18l17 17v20L41 59H23L6 42V22z",
    "M30 5c14-2 29 9 28 27-1 17-13 27-28 26C14 57 4 46 6 30 7 16 17 6 30 5z",
    "M32 4c6 0 9 6 15 8s12 4 12 12c0 6-4 9-4 15 0 12-9 21-23 21S9 51 9 39c0-6-4-9-4-15 0-8 6-10 12-12s9-8 15-8z",
    "M32 6c13 0 24 11 24 24v26l-6-4-6 4-6-4-6 4-6-4-6 4-6-4-6 4V30C8 17 19 6 32 6z",
    "M27 7.9q5-2.9 10 0l15.4 8.9q5 2.9 5 8.7v17q0 5.8-5 8.7L37 60.1q-5 2.9-10 0l-15.4-8.9q-5-2.9-5-8.7v-17q0-5.8 5-8.7z",
    "M32 5c22 0 27 5 27 27S54 59 32 59 5 54 5 32 10 5 32 5z",
    "M34 5c12 1 24 9 24 22 0 9-5 13-4 20 1 8-8 13-20 12-14-1-26-8-27-22C6 22 18 4 34 5z",
    "M32 5c12 0 20 8 20 18v18c0 10-8 18-20 18S12 51 12 41V23C12 13 20 5 32 5z",
    "M27 9q5-7 10 0l20 35q4 8-5 9H12q-9-1-5-9z",
];

/// Body colours.
pub const CRITTER_PALETTE: [&str; 10] = tokens::PALETTE;

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
            CritterFamily::Dm => tokens::DM_SHAPES,
            CritterFamily::Group => &[3, 8],
            CritterFamily::Agent => &[1, 2, 6, 7, 10, 12],
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
    /// Seconds added to the motion clock, so a list of critters does not blink in step.
    pub phase: f64,
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
        phase: (h >> 16) as f64 / 65536.0 * BLINK,
    }
}

/// Paint a critter scaled into `rect` (square fit, centred), at rest.
pub fn draw_critter(ctx: &mut dyn RenderContext, rect: Rect, view: &CritterView) {
    draw_critter_pose(ctx, rect, view, &CritterPose::REST);
}

/// Animation state (HQ `avatarState`). State shows only by motion, never by rings,
/// glows or pulses ([`critter_pose`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CritterState {
    /// Still.
    #[default]
    Idle,
    /// Plays tricks without a pause (tilts, turns, spins, somersaults, hops).
    Working,
    /// The working tricks, slower, eyes raised.
    Thinking,
    /// A calmer set: looks at the viewer, small hops.
    Attention,
    /// Eyes almost shut, faded; no motion.
    Off,
}

/// One frame of a critter's motion.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CritterMotion {
    /// State to animate.
    pub state: CritterState,
    /// Seconds on any monotonic clock (each critter keeps its own phase via its seed).
    pub t: f64,
    /// Pointer in the same space as the paint rect: the eyes look at it ([`critter_look`]).
    pub look_at: Option<(f64, f64)>,
    /// `prefers-reduced-motion`: a gentle tilt only.
    pub reduced: bool,
}

/// Span of [`CritterView::phase`], seconds.
pub const BLINK: f64 = 7.5;

/// Eye offset toward `pointer` from the critter centre (HQ: up to 4 units, full at 300px).
pub fn critter_look(center: (f64, f64), pointer: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = (pointer.0 - center.0, pointer.1 - center.1);
    let d = dx.hypot(dy).max(1.0);
    let k = (d / 300.0).min(1.0) * 4.0;
    (dx / d * k, dy / d * k)
}

/// Seconds until the next frame of `state` changes visibly; `None` for a still pose
/// (idle, off, and reduced-motion idle). Moving states animate at 30 fps.
pub fn critter_next_frame(state: CritterState, _t: f64, _reduced: bool) -> Option<f64> {
    match state {
        CritterState::Idle | CritterState::Off => None,
        _ => Some(1.0 / 30.0),
    }
}

/// The per-critter seed of [`critter_pose`] for a view (keeps a list out of step).
pub fn critter_seed(view: &CritterView) -> u32 {
    (view.phase * 8191.0) as u32 ^ (view.shape as u32).wrapping_mul(0x9E37_79B9)
}

/// Paint one animation frame of a critter: [`critter_pose`] for its state, plus the
/// pointer look.
pub fn draw_critter_motion(ctx: &mut dyn RenderContext, rect: Rect, view: &CritterView, m: &CritterMotion) {
    let st = if view.off { CritterState::Off } else { m.state };
    let mut pose = critter_pose(st, m.t, critter_seed(view), m.reduced);
    if let (Some(p), false) = (m.look_at, st == CritterState::Off || m.reduced) {
        let (lx, ly) = critter_look((rect.x + rect.width / 2.0, rect.y + rect.height / 2.0), p);
        pose.eye_dx += lx;
        pose.eye_dy += ly;
    }
    draw_critter_pose(ctx, rect, view, &pose);
}

/// Round teardrop the body morphs toward (64-box).
const BLOB: &str = "M32 6C40 14 58 24 58 39C58 52 46 60 32 60S6 52 6 39C6 24 24 14 32 6z";

/// Paint a critter in `pose`, scaled into `rect` (square fit, centred).
pub fn draw_critter_pose(ctx: &mut dyn RenderContext, rect: Rect, view: &CritterView, pose: &CritterPose) {
    let s = rect.width.min(rect.height) / 64.0;
    let off = view.off;
    let yaw = pose.yaw.clamp(-1.0, 1.0);
    let m = pose.morph.clamp(0.0, 1.0);
    let about = |ctx: &mut dyn RenderContext, k: f64, cy: f64, d: &str| {
        ctx.save();
        ctx.translate(32.0, cy);
        ctx.scale(k, k);
        ctx.translate(-32.0, -cy);
        emit_svg_path(ctx, d);
        ctx.fill();
        ctx.restore();
    };
    ctx.save();
    ctx.translate(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    ctx.scale(s, s);
    if off {
        ctx.set_global_alpha(0.5);
    }
    ctx.translate(pose.dx, pose.dy);
    ctx.rotate(pose.rot);
    ctx.scale(pose.sx * (1.0 - 0.14 * yaw.abs()), pose.sy);
    ctx.translate(-32.0, -32.0);
    // Body: the shape shrinks into a growing blob of the same colour (their union reads
    // as one morphing body).
    ctx.set_fill_color(view.color);
    about(ctx, 1.0 - 0.3 * m, 32.0, CRITTER_SHAPES[view.shape % CRITTER_SHAPES.len()]);
    if m > 0.0 {
        about(ctx, m.powf(0.7), 36.0, BLOB);
    }
    // Eyes: tilted ovals at rest, upright pills in a turn; the far eye narrows.
    let half = 9.0 * (1.0 - 0.35 * yaw.abs());
    let lid = if off { 0.12 } else { (1.0 - 0.9 * pose.blink.clamp(0.0, 1.0)).max(0.08) };
    let pill = pose.pill.clamp(0.0, 1.0);
    ctx.set_fill_color(tokens::EYE);
    for side in [-1.0, 1.0] {
        let far = if side * yaw < 0.0 { 1.0 - 0.2 * yaw.abs() } else { 1.0 };
        let rx = 4.0 * pose.eye_w * (1.0 - 0.22 * pill) * far;
        let ry = 6.0 * pose.eye_h * (1.0 + 0.15 * pill) * lid;
        ctx.begin_path();
        ctx.ellipse(
            32.0 + side * half + pose.eye_dx,
            view.eye_y + pose.eye_dy,
            rx.max(0.5),
            ry.max(0.4),
            (-12.0f64).to_radians() * (1.0 - pill),
            0.0,
            std::f64::consts::TAU,
        );
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
        assert!(CritterFamily::Group.shapes().contains(&critter_view("x", CritterFamily::Group).shape));
        assert!((26.0..=30.0).contains(&a.eye_y));
    }

    #[test]
    fn cadence_and_look() {
        assert_eq!(critter_next_frame(CritterState::Idle, 1.0, false), None);
        assert_eq!(critter_next_frame(CritterState::Off, 1.0, false), None);
        assert_eq!(critter_next_frame(CritterState::Working, 1.0, false), Some(1.0 / 30.0));
        let l = critter_look((0.0, 0.0), (900.0, 0.0));
        assert!((l.0 - 4.0).abs() < 1e-9 && l.1.abs() < 1e-9);
    }
}
