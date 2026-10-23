//! Critter avatar: a soft body shape with two tilted eyes (Hatchery HQ style).
//!
//! - [`CritterFamily`] picks the shape set; chat kinds differ by shape, not by label.
//! - [`critter_view`] derives shape, colour and eye height from a stable name.
//! - [`draw_critter`] paints into any [`RenderContext`]; [`register_critter`] declares the hit rect.
//! - [`draw_critter_motion`] paints one animation frame of a [`CritterState`] at a time `t`
//!   (the HQ style.css `.av.st-*` keyframes); [`critter_next_frame`] says when the next frame
//!   is due, so an idle screen repaints only for blinks; [`critter_look`] is the eye offset
//!   toward the pointer.

mod tokens;

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

/// Paint a critter scaled into `rect` (square fit, centred), at rest.
pub fn draw_critter(ctx: &mut dyn RenderContext, rect: Rect, view: &CritterView) {
    draw_critter_motion(ctx, rect, view, &CritterMotion { reduced: true, ..Default::default() });
}

/// Animation state (HQ `avatarState`: idle / working / thinking / attention / off).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CritterState {
    /// Blinks and breathes.
    #[default]
    Idle,
    /// Pulses and wobbles, eyes roll.
    Working,
    /// Bobs, eyes roll, two thought dots fade.
    Thinking,
    /// Bounces inside an expanding ring.
    Attention,
    /// Eyes almost shut, faded; no motion.
    Off,
}

/// One frame of a critter's motion.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CritterMotion {
    /// State to animate.
    pub state: CritterState,
    /// Seconds on any monotonic clock (all critters share the phase).
    pub t: f64,
    /// Pointer in the same space as the paint rect: the eyes look at it ([`critter_look`]).
    pub look_at: Option<(f64, f64)>,
    /// `prefers-reduced-motion`: the state's pose at rest, no keyframes.
    pub reduced: bool,
}

/// CSS `ease-in-out` between keyframes, close enough at avatar size.
fn ease(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// 0 → 1 → 0 over `period` (the `0%,100% {a} 50% {b}` keyframes).
fn swing(t: f64, period: f64) -> f64 {
    let f = (t / period).rem_euclid(1.0);
    if f < 0.5 { ease(f * 2.0) } else { ease((1.0 - f) * 2.0) }
}

/// `blink 5.5s`: scaleY 1 until 92%, 0.1 at 95%, 1 at 100%.
fn blink(t: f64) -> f64 {
    let f = (t / 5.5).rem_euclid(1.0);
    if f < 0.92 {
        1.0
    } else if f < 0.95 {
        1.0 - 0.9 * ease((f - 0.92) / 0.03)
    } else {
        0.1 + 0.9 * ease((f - 0.95) / 0.05)
    }
}

/// `roll`: (0,0) (3,-2) (0,-3) (-3,-1) (0,0) at 0/25/50/75/100%.
fn roll(t: f64, period: f64) -> (f64, f64) {
    const K: [(f64, f64); 5] = [(0.0, 0.0), (3.0, -2.0), (0.0, -3.0), (-3.0, -1.0), (0.0, 0.0)];
    let f = (t / period).rem_euclid(1.0) * 4.0;
    let i = (f as usize).min(3);
    let k = ease(f - i as f64);
    (K[i].0 + (K[i + 1].0 - K[i].0) * k, K[i].1 + (K[i + 1].1 - K[i].1) * k)
}

/// Eye offset toward `pointer` from the critter centre (HQ: up to 4 units, full at 300px).
pub fn critter_look(center: (f64, f64), pointer: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = (pointer.0 - center.0, pointer.1 - center.1);
    let d = dx.hypot(dy).max(1.0);
    let k = (d / 300.0).min(1.0) * 4.0;
    (dx / d * k, dy / d * k)
}

/// Seconds until the next frame of `state` at `t` changes visibly; `None` for a still pose.
/// Idle wakes only for the blink (30 fps inside it); the breath is under a pixel at avatar
/// size, so it rides along on frames painted for other reasons instead of costing its own.
pub fn critter_next_frame(state: CritterState, t: f64, reduced: bool) -> Option<f64> {
    const FAST: f64 = 1.0 / 30.0;
    if reduced {
        return None;
    }
    match state {
        CritterState::Off => None,
        CritterState::Idle => {
            let f = (t / 5.5).rem_euclid(1.0) * 5.5;
            let start = 0.92 * 5.5;
            if f >= start { Some(FAST) } else { Some((start - f).max(FAST)) }
        }
        _ => Some(FAST),
    }
}

/// Paint one animation frame of a critter (HQ style.css `.av.st-*` keyframes).
pub fn draw_critter_motion(ctx: &mut dyn RenderContext, rect: Rect, view: &CritterView, m: &CritterMotion) {
    use CritterState::*;
    let st = if view.off { Off } else { m.state };
    let t = if m.reduced { 0.0 } else { m.t };
    let s = rect.width.min(rect.height) / 64.0;
    let ox = rect.x + (rect.width - 64.0 * s) / 2.0;
    let oy = rect.y + (rect.height - 64.0 * s) / 2.0;
    let about = |ctx: &mut dyn RenderContext, cx: f64, cy: f64, f: &dyn Fn(&mut dyn RenderContext)| {
        ctx.translate(cx, cy);
        f(ctx);
        ctx.translate(-cx, -cy);
    };
    ctx.save();
    ctx.translate(ox, oy);
    ctx.scale(s, s);
    if st == Off {
        ctx.set_global_alpha(0.5);
    }
    // svg-level motion: pulse (working), bounce (attention)
    if !m.reduced {
        match st {
            Working => {
                let k = 1.0 + 0.05 * swing(t, 1.3);
                about(ctx, 32.0, 32.0, &|c| c.scale(k, k));
            }
            Attention => ctx.translate(0.0, -3.0 * swing(t, 0.9)),
            _ => {}
        }
    }
    // .bodyg: breathe (idle), wobble (working), bob (thinking)
    ctx.save();
    if !m.reduced {
        match st {
            Idle => {
                let p = swing(t, 5.0);
                about(ctx, 32.0, 32.0, &|c| c.scale(1.0 + 0.03 * p, 1.0 - 0.02 * p));
            }
            Working => {
                let p = swing(t, 1.4);
                ctx.translate(0.0, -2.0 * p);
                about(ctx, 32.0, 32.0, &|c| c.rotate((-4.0 + 8.0 * p).to_radians()));
            }
            Thinking => ctx.translate(0.0, -3.0 * swing(t, 2.4)),
            _ => {}
        }
    }
    emit_svg_path(ctx, CRITTER_SHAPES[view.shape % CRITTER_SHAPES.len()]);
    ctx.set_fill_color(view.color);
    ctx.fill();
    // .look (pointer) then .eyes (roll) then each .eye
    let look = m.look_at.map(|p| critter_look((rect.x + rect.width / 2.0, rect.y + rect.height / 2.0), p));
    let (mut ex, mut ey) = if st == Off || m.reduced { (0.0, 0.0) } else { look.unwrap_or((0.0, 0.0)) };
    if !m.reduced {
        match st {
            Working => {
                let r = roll(t, 2.2);
                ex += r.0;
                ey += r.1;
            }
            Thinking => {
                let r = roll(t, 3.0);
                ex += r.0;
                ey += r.1;
            }
            _ => {}
        }
    }
    // An animated `.eye` transform (blink, off) replaces the SVG rotate(-12); otherwise it tilts.
    let (sy, tilt) = match st {
        Idle => (if m.reduced { 1.0 } else { blink(t) }, 0.0),
        Off => (0.12, 0.0),
        _ => (1.0, (-12.0f64).to_radians()),
    };
    ctx.set_fill_color(tokens::EYE);
    for cx in [23.0, 41.0] {
        ctx.begin_path();
        ctx.ellipse(cx + ex, view.eye_y + ey, 4.0, 6.0 * sy, tilt, 0.0, std::f64::consts::TAU);
        ctx.fill();
    }
    ctx.restore();
    // extras: thought dots, attention ring
    match st {
        Thinking => {
            ctx.set_fill_color(tokens::THOUGHT);
            for (i, (cx, cy, r)) in [(50.0, 8.0, 2.4), (56.0, 3.0, 1.8)].into_iter().enumerate() {
                let a = if m.reduced { 1.0 } else { 0.2 + 0.8 * swing(t - 0.5 * i as f64, 1.6) };
                ctx.set_global_alpha(a);
                ctx.begin_path();
                ctx.arc(cx, cy, r, 0.0, std::f64::consts::TAU);
                ctx.fill();
            }
        }
        Attention => {
            let f = if m.reduced { 0.0 } else { (t / 1.4).rem_euclid(1.0) };
            let e = 1.0 - (1.0 - f).powi(3); // ease-out
            ctx.set_global_alpha(0.9 * (1.0 - e));
            ctx.set_stroke_color(tokens::ATTENTION);
            ctx.set_stroke_width(2.5);
            ctx.begin_path();
            ctx.arc(32.0, 32.0, 30.0 * (1.0 + 0.25 * e), 0.0, std::f64::consts::TAU);
            ctx.stroke();
        }
        _ => {}
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

    #[test]
    fn motion_keyframes_and_cadence() {
        // blink closes at 95% of 5.5 s, open otherwise
        assert_eq!(blink(1.0), 1.0);
        assert!((blink(0.95 * 5.5) - 0.1).abs() < 1e-9);
        // idle: slow steps outside the blink, fast inside, nothing when off / reduced
        assert!((critter_next_frame(CritterState::Idle, 1.0, false).unwrap() - (0.92 * 5.5 - 1.0)).abs() < 1e-9);
        assert_eq!(critter_next_frame(CritterState::Idle, 5.2, false), Some(1.0 / 30.0));
        assert_eq!(critter_next_frame(CritterState::Off, 1.0, false), None);
        assert_eq!(critter_next_frame(CritterState::Working, 1.0, true), None);
        // roll passes its keyframes; look caps at 4 units
        assert_eq!(roll(0.0, 2.0), (0.0, 0.0));
        assert_eq!(roll(1.0, 2.0), (0.0, -3.0));
        let l = critter_look((0.0, 0.0), (900.0, 0.0));
        assert!((l.0 - 4.0).abs() < 1e-9 && l.1.abs() < 1e-9);
    }
}
