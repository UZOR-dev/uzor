//! Ring mark: a regular polygon ring around an n-point star, animated in fake 3D.
//!
//! The ring tumbles about axes in space (flips over a symmetry axis, full tumbles about
//! a tilted axis, a coin wobble, a flat spin) and is projected back onto the screen, so
//! it foreshortens, goes edge-on and lands on its back face looking exactly as it started.
//! The star stays flat and turns the other way; on a breath both contract and rebound.
//! Every trick runs from rest to rest and is picked per time slot by a hash of a seed
//! ([`TrickCycle`], [`slot_hash`]), so a pose is a pure function of time.
//!
//! - [`RingMarkStyle`]: the drawing (sides, radii, colours) in a 64-unit box.
//! - [`ring_mark_pose`]: the pose of a [`RingMarkMode`] at `t` seconds.
//! - [`RingMarkPose::blend`]: cross-fade two poses (mode changes).
//! - [`draw_ring_mark`]: paint a pose into any [`RenderContext`].

use std::f64::consts::PI;

use crate::render::parse_color;
use crate::render::RenderContext;
use crate::types::Rect;
use crate::ui::animation::math::cycle::{back_in_out, breathe, slot_hash, smooth, turn_delta, TrickCycle};
use crate::ui::animation::math::spatial::Rot3;

/// How busy the mark looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RingMarkMode {
    /// Now and then a wobble, a breath or a small spin; mostly at rest.
    Idle,
    /// Flips, wobbles, a tumble, a breath, back to back.
    Loading,
    /// Faster, more tumbles.
    Busy,
}

/// One trick of the ring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RingTrick {
    /// Hold still.
    Rest,
    /// Half turn about an in-plane symmetry axis (lands on the same outline).
    Flip,
    /// Full turn about an axis tilted out of the plane.
    Tumble,
    /// Coin wobble: the tilt axis goes once around.
    Wobble,
    /// Contract, rebound, settle.
    Breathe,
    /// Flat spin by one side.
    Spin,
}

use RingTrick::*;

impl RingMarkMode {
    /// Slot timing of the mode.
    pub fn cycle(self) -> TrickCycle {
        match self {
            Self::Idle => TrickCycle { slot: 3.2, play: 0.55 },
            Self::Loading => TrickCycle { slot: 1.25, play: 1.0 },
            Self::Busy => TrickCycle { slot: 0.95, play: 1.0 },
        }
    }

    /// Tricks the mode picks from.
    pub fn tricks(self) -> &'static [RingTrick] {
        match self {
            Self::Idle => &[Wobble, Breathe, Rest, Spin, Wobble, Rest],
            Self::Loading => &[Flip, Flip, Wobble, Flip, Tumble, Breathe],
            Self::Busy => &[Flip, Tumble, Flip, Breathe, Tumble, Spin],
        }
    }

    /// Steady counter-spin of the star, radians per second.
    pub fn drift(self) -> f64 {
        match self {
            Self::Idle => -0.25,
            Self::Loading => -0.9,
            Self::Busy => -1.6,
        }
    }
}

/// One frame of the mark.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingMarkPose {
    /// Ring rotation in space.
    pub turn: Rot3,
    /// Star angle, radians (clockwise).
    pub star: f64,
    /// Ring scale.
    pub ring: f64,
    /// Star scale.
    pub core: f64,
}

impl RingMarkPose {
    /// At rest.
    pub const REST: RingMarkPose = RingMarkPose { turn: Rot3::IDENTITY, star: 0.0, ring: 1.0, core: 1.0 };

    /// Blend toward `o` by `k` 0..1 (smoothed); the star takes the short way round its
    /// `points`-fold symmetry.
    pub fn blend(&self, o: &RingMarkPose, k: f64, points: u32) -> RingMarkPose {
        let k = smooth(k);
        RingMarkPose {
            turn: self.turn.nlerp(&o.turn, k),
            star: self.star + turn_delta(self.star, o.star, points) * k,
            ring: self.ring + (o.ring - self.ring) * k,
            core: self.core + (o.core - self.core) * k,
        }
    }
}

/// The drawing, in a 64-unit box centred at (32, 32).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingMarkStyle {
    /// Ring sides (first corner points up).
    pub sides: u32,
    /// Ring centreline radius (to a corner).
    pub radius: f64,
    /// Ring band width.
    pub width: f64,
    /// Star points (first points up).
    pub points: u32,
    /// Star tip radius.
    pub star_outer: f64,
    /// Star waist radius.
    pub star_inner: f64,
    /// Ring colour (`#rrggbb`).
    pub ring_color: &'static str,
    /// Star colour.
    pub star_color: &'static str,
    /// What the mark sits on: an edge-on ring fades a little toward it.
    pub background: &'static str,
    /// Camera distance for perspective (0 = none).
    pub camera: f64,
}

/// Pose of a trick at progress `u` 0..1; `h` picks the axis and the direction.
pub fn ring_trick_pose(trick: RingTrick, u: f64, h: u32, sides: u32, points: u32) -> RingMarkPose {
    let n = sides.max(3) as f64;
    // in-plane symmetry axes: through corners and edge midpoints
    let axis = -PI / 2.0 + (h % sides.max(3)) as f64 * PI / n;
    let dir = if (h >> 8) & 1 == 1 { 1.0 } else { -1.0 };
    let step = 2.0 * PI / points.max(1) as f64;
    let mut p = RingMarkPose::REST;
    match trick {
        Rest => {}
        Flip => {
            let e = back_in_out(u, 1.1);
            p.turn = Rot3::about_dir(axis, 0.0, dir * PI * e);
            p.star = -dir * step * e;
        }
        Tumble => {
            let e = back_in_out(u, 0.8);
            p.turn = Rot3::about_dir(axis, 0.55, dir * 2.0 * PI * e);
            p.star = -dir * 2.0 * step * e;
        }
        Wobble => {
            let s = (PI * u).sin();
            p.turn = Rot3::about_dir(axis + dir * 2.0 * PI * smooth(u), 0.0, 0.42 * s * s);
            p.star = -dir * step * smooth(u);
        }
        Breathe => {
            let b = breathe(u, 0.16);
            p.star = -dir * step * smooth(u);
            p.ring = b;
            // the star breathes deeper, so it never touches the ring's inner edge
            p.core = if b < 1.0 { 1.0 - (1.0 - b) * 1.6 } else { b };
        }
        Spin => {
            let e = back_in_out(u, 0.6);
            p.turn = Rot3::about([0.0, 0.0, 1.0], dir * 2.0 * PI / n * e);
            p.star = -dir * step * e;
        }
    }
    p
}

/// Pose of `mode` at `t` seconds. `seed` decorrelates marks shown side by side.
pub fn ring_mark_pose(mode: RingMarkMode, t: f64, seed: u32, style: &RingMarkStyle) -> RingMarkPose {
    let (slot, u) = mode.cycle().at(t);
    let h = slot_hash(seed, slot);
    let tricks = mode.tricks();
    let mut p = match u {
        Some(u) => ring_trick_pose(tricks[h as usize % tricks.len()], u, h >> 4, style.sides, style.points),
        None => RingMarkPose::REST,
    };
    p.star += mode.drift() * t;
    p
}

/// `#rrggbb` of `a` moved toward `b` by `k`.
fn mix(a: &str, b: &str, k: f64) -> [u8; 7] {
    let (a, b) = (parse_color(a), parse_color(b));
    let mut out = *b"#000000";
    for (i, (x, y)) in [(a.0, b.0), (a.1, b.1), (a.2, b.2)].into_iter().enumerate() {
        let v = (x as f64 + (y as f64 - x as f64) * k + 0.5) as u8;
        out[1 + 2 * i] = b"0123456789abcdef"[(v >> 4) as usize];
        out[2 + 2 * i] = b"0123456789abcdef"[(v & 15) as usize];
    }
    out
}

/// Paint `pose` into the square centred in `rect`.
pub fn draw_ring_mark(ctx: &mut dyn RenderContext, rect: Rect, style: &RingMarkStyle, pose: &RingMarkPose) {
    let size = rect.width.min(rect.height);
    let s = size / 64.0;
    let (cx, cy) = (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    let n = style.sides.max(3);
    let corner = (PI / n as f64).cos();
    // depth cue: the ring fades a little toward the background as it turns edge-on,
    // the same for both faces (a flip lands looking exactly like the rest pose)
    let ring = mix(style.ring_color, style.background, 0.22 * (1.0 - pose.turn.facing().abs()));
    let ring = core::str::from_utf8(&ring).unwrap_or(style.ring_color);
    let path = |ctx: &mut dyn RenderContext, r: f64, rev: bool| {
        for i in 0..=n {
            let j = if rev { n - i % n } else { i } % n;
            let a = -PI / 2.0 + 2.0 * PI * j as f64 / n as f64;
            let (x, y) = pose.turn.project(r * a.cos(), r * a.sin(), style.camera);
            let (x, y) = (cx + x * pose.ring * s, cy + y * pose.ring * s);
            if i == 0 { ctx.move_to(x, y) } else { ctx.line_to(x, y) }
        }
        ctx.close_path();
    };
    // band: outer outline, inner one the other way round (a hole under nonzero fill);
    // both outlines stroked too, which rounds the corners and keeps an edge-on ring
    // visible as a line (the band is narrowed by half the stroke to keep its width)
    let line = (1.6 * s).max(1.0);
    let half = (style.width - line / s) / 2.0 / corner;
    ctx.begin_path();
    path(ctx, style.radius + half, false);
    path(ctx, style.radius - half, true);
    ctx.set_fill_color(ring);
    ctx.fill();
    ctx.set_stroke_color(ring);
    ctx.set_stroke_width(line);
    ctx.set_line_join("round");
    ctx.stroke();
    // star: flat, turning against the ring
    let k = 2 * style.points.max(2);
    ctx.begin_path();
    for i in 0..k {
        let r = if i % 2 == 0 { style.star_outer } else { style.star_inner } * pose.core * s;
        let a = -PI / 2.0 + pose.star + PI * i as f64 / style.points.max(2) as f64;
        let (x, y) = (cx + r * a.cos(), cy + r * a.sin());
        if i == 0 { ctx.move_to(x, y) } else { ctx.line_to(x, y) }
    }
    ctx.close_path();
    ctx.set_fill_color(style.star_color);
    ctx.fill();
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEX: RingMarkStyle = RingMarkStyle {
        sides: 6, radius: 27.0, width: 7.5, points: 4, star_outer: 16.5, star_inner: 5.94,
        ring_color: "#15171c", star_color: "#f5a524", background: "#f4f2ee", camera: 150.0,
    };

    fn corners(p: &RingMarkPose) -> Vec<(f64, f64)> {
        (0..6)
            .map(|i| {
                let t = -PI / 2.0 + PI / 3.0 * i as f64;
                let (x, y) = p.turn.project(27.0 * t.cos(), 27.0 * t.sin(), 150.0);
                (x * p.ring, y * p.ring)
            })
            .collect()
    }

    /// How far apart two poses look, box units: ring corners (each to the nearest corner
    /// of the other: a flip lands with corners swapped, which looks the same) and star tips.
    fn far(a: &RingMarkPose, b: &RingMarkPose) -> f64 {
        let cb = corners(b);
        let d = corners(a).iter().map(|p| cb.iter().map(|q| (p.0 - q.0).hypot(p.1 - q.1)).fold(f64::MAX, f64::min)).fold(0.0, f64::max);
        let star = turn_delta(a.star, b.star, 4).abs() * 16.5;
        d.max(star).max((a.core - b.core).abs() * 16.5)
    }

    /// How far the outline of `a` is from the rest outline (corners may swap places).
    fn off_rest(a: &RingMarkPose) -> f64 {
        let rest = corners(&RingMarkPose::REST);
        let ring = corners(a).iter().map(|p| rest.iter().map(|q| (p.0 - q.0).hypot(p.1 - q.1)).fold(f64::MAX, f64::min)).fold(0.0, f64::max);
        ring.max(turn_delta(a.star, 0.0, 4).abs() * 16.5).max((a.core - 1.0).abs())
    }

    #[test]
    fn tricks_rest_at_both_ends_on_the_same_outline() {
        for tr in [Flip, Tumble, Wobble, Breathe, Spin] {
            for h in [0u32, 1, 2, 3, 4, 5, 256, 257] {
                for u in [0.0, 1.0] {
                    let p = ring_trick_pose(tr, u, h, 6, 4);
                    assert!(off_rest(&p) < 1e-6, "{tr:?} h={h} u={u}");
                }
            }
        }
    }

    #[test]
    fn no_jumps_frame_to_frame() {
        // 120 fps through every mode: no frame moves much more than its neighbours (a pop
        // would stand out against both), and nothing moves faster than 6 box units a frame
        for mode in [RingMarkMode::Idle, RingMarkMode::Loading, RingMarkMode::Busy] {
            let pose = |i: i32| ring_mark_pose(mode, i as f64 / 120.0, 7, &HEX);
            let steps: Vec<f64> = (0..2400).map(|i| far(&pose(i), &pose(i + 1))).collect();
            for i in 1..steps.len() - 1 {
                let near = steps[i - 1].max(steps[i + 1]);
                assert!(steps[i] < 6.0 && steps[i] <= 2.0 * near + 0.2, "{mode:?} at {}: {} vs {near}", i as f64 / 120.0, steps[i]);
            }
        }
    }
}
