//! Critter motion: a [`CritterPose`] (what one frame looks like) and the trick animator
//! [`critter_pose`] (which pose a critter takes at a time).
//!
//! State shows only by motion: no rings, glows or pulses. Working runs tricks without a
//! pause (tilts, 3/4 turns, spins and somersaults with a mid-spin body morph, hops with
//! squash and stretch, blinks, wobbles); Attention runs a calmer set (looks at the
//! viewer, small hops); Idle and Off are still. Each critter picks its next trick from a
//! stateless hash of its seed and the time slot, so a list of critters never moves in step.
//! Every trick starts and ends at rest and is eased with a little overshoot, so slot
//! changes blend and nothing snaps. Reduced motion: a gentle tilt only.

use super::CritterState;
use std::f64::consts::{PI, TAU};

/// One frame of a critter, as offsets from its rest pose (64-unit box, centre 32,32).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CritterPose {
    /// Body shift, box units.
    pub dx: f64,
    /// Body shift, box units (negative is up).
    pub dy: f64,
    /// Rotation about the centre, radians (clockwise positive).
    pub rot: f64,
    /// Horizontal scale (squash / stretch).
    pub sx: f64,
    /// Vertical scale.
    pub sy: f64,
    /// Body morph toward a round teardrop blob, 0..1.
    pub morph: f64,
    /// Head turn, -1 (3/4 to the left) ..= 1 (3/4 to the right).
    pub yaw: f64,
    /// Eye width factor (1 = rest).
    pub eye_w: f64,
    /// Eye height factor (1 = rest).
    pub eye_h: f64,
    /// Eye shape: 0 tilted oval (rest), 1 upright pill.
    pub pill: f64,
    /// Eye offset, box units (look direction).
    pub eye_dx: f64,
    /// Eye offset, box units.
    pub eye_dy: f64,
    /// Lids, 0 open ..= 1 shut.
    pub blink: f64,
}

impl Default for CritterPose {
    fn default() -> Self {
        Self::REST
    }
}

impl CritterPose {
    /// The still pose.
    pub const REST: CritterPose = CritterPose {
        dx: 0.0,
        dy: 0.0,
        rot: 0.0,
        sx: 1.0,
        sy: 1.0,
        morph: 0.0,
        yaw: 0.0,
        eye_w: 1.0,
        eye_h: 1.0,
        pill: 0.0,
        eye_dx: 0.0,
        eye_dy: 0.0,
        blink: 0.0,
    };

    /// Linear blend toward `o` by `k` (0 = self, 1 = `o`).
    pub fn lerp(&self, o: &CritterPose, k: f64) -> CritterPose {
        let l = |a: f64, b: f64| a + (b - a) * k;
        CritterPose {
            dx: l(self.dx, o.dx),
            dy: l(self.dy, o.dy),
            rot: l(self.rot, o.rot),
            sx: l(self.sx, o.sx),
            sy: l(self.sy, o.sy),
            morph: l(self.morph, o.morph),
            yaw: l(self.yaw, o.yaw),
            eye_w: l(self.eye_w, o.eye_w),
            eye_h: l(self.eye_h, o.eye_h),
            pill: l(self.pill, o.pill),
            eye_dx: l(self.eye_dx, o.eye_dx),
            eye_dy: l(self.eye_dy, o.eye_dy),
            blink: l(self.blink, o.blink),
        }
    }
}

/// A short animation a critter plays from rest back to rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CritterTrick {
    /// Lean to the left and back.
    TiltLeft,
    /// Lean to the right and back.
    TiltRight,
    /// Eyes sweep around.
    LookAround,
    /// 3/4 turn to the left: body narrows, eyes slide and turn to pills.
    TurnLeft,
    /// 3/4 turn to the right.
    TurnRight,
    /// Full turn in place; the body rounds to a blob mid-spin.
    Spin,
    /// Hop with a full forward roll and the mid-spin morph.
    Somersault,
    /// Hop: crouch, stretch in the air, squash on landing.
    Hop,
    /// Two quick blinks.
    DoubleBlink,
    /// Shaky wobble that settles.
    Wobble,
    /// Turn the eyes to the viewer: big round eyes, a small lean in.
    LookAtViewer,
    /// A small hop.
    SmallHop,
}

use CritterTrick::*;

/// Tricks a working critter cycles through.
pub const WORKING_TRICKS: &[CritterTrick] =
    &[TiltLeft, TiltRight, LookAround, TurnLeft, TurnRight, Spin, Somersault, Hop, DoubleBlink, Wobble];
/// The calmer set for a critter waiting on the user.
pub const ATTENTION_TRICKS: &[CritterTrick] = &[LookAtViewer, SmallHop, LookAtViewer, DoubleBlink, SmallHop];
/// Seconds per trick slot (working, attention).
pub const TRICK_SLOT: (f64, f64) = (1.6, 2.2);

fn ease(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Ease-out with a small overshoot (about 8%).
fn back_out(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0) - 1.0;
    let c = 1.2;
    1.0 + (c + 1.0) * x * x * x + c * x * x
}

/// 0 → 1 (with overshoot) → hold → 0: the envelope of a pose that is held for a moment.
fn hold(p: f64) -> f64 {
    if p < 0.3 {
        back_out(p / 0.3)
    } else if p < 0.7 {
        1.0
    } else {
        1.0 - ease((p - 0.7) / 0.3)
    }
}

/// Smooth 0 → 1 → 0 over p in a..b.
fn bell(p: f64, a: f64, b: f64) -> f64 {
    if p <= a || p >= b {
        0.0
    } else {
        (PI * (p - a) / (b - a)).sin().powi(2)
    }
}

/// Jump height over p (0 before `a`, a parabola-like arc to `b`).
fn arc(p: f64, a: f64, b: f64) -> f64 {
    if p <= a || p >= b {
        0.0
    } else {
        (PI * (p - a) / (b - a)).sin()
    }
}

/// Crouch before take-off and squash after landing: (sx, sy).
fn squash(p: f64, a: f64, b: f64, k: f64) -> (f64, f64) {
    let s = bell(p, a - 0.18, a) + bell(p, b, b + 0.2) * 1.2;
    let st = bell(p, a, a + 0.18) * 0.6; // stretch on take-off
    (1.0 + k * s - 0.5 * k * st, 1.0 - k * s + k * st)
}

/// The pose of `trick` at progress `p` (0..1); the pose is the rest pose at 0 and 1.
pub fn trick_pose(trick: CritterTrick, p: f64) -> CritterPose {
    let p = p.clamp(0.0, 1.0);
    let mut o = CritterPose::REST;
    match trick {
        TiltLeft | TiltRight => {
            let s = if trick == TiltLeft { -1.0 } else { 1.0 };
            let h = hold(p);
            o.rot = s * 0.32 * h;
            o.dx = s * 2.0 * h;
            o.eye_dx = s * 2.0 * h;
            o.eye_dy = 1.0 * h;
        }
        LookAround => {
            let e = hold(p);
            o.eye_dx = 4.0 * (TAU * p).sin() * e;
            o.eye_dy = -2.5 * (TAU * p).cos().max(0.0) * e;
            o.eye_w = 1.0 + 0.15 * e;
            o.eye_h = 1.0 - 0.1 * e;
        }
        TurnLeft | TurnRight => {
            let s = if trick == TurnLeft { -1.0 } else { 1.0 };
            let h = hold(p);
            o.yaw = s * h;
            o.pill = h;
            o.eye_dx = s * 6.0 * h;
            o.rot = s * 0.08 * h;
        }
        Spin => {
            o.rot = TAU * ease(p);
            let m = bell(p, 0.1, 0.9);
            o.morph = 0.9 * m;
            o.sx = 1.0 + 0.08 * m;
            o.sy = 1.0 - 0.06 * m;
            o.eye_w = 1.0 - 0.3 * m;
            o.eye_h = 1.0 - 0.45 * m;
        }
        Somersault => {
            let (a, b) = (0.2, 0.75);
            let q = ((p - a) / (b - a)).clamp(0.0, 1.0);
            o.dy = -9.0 * arc(p, a, b);
            o.rot = TAU * ease(q);
            let m = bell(p, a, b);
            o.morph = m;
            o.eye_h = 1.0 - 0.5 * m;
            let (x, y) = squash(p, a, b, 0.14);
            o.sx = x;
            o.sy = y;
        }
        Hop => {
            let (a, b) = (0.2, 0.65);
            o.dy = -8.0 * arc(p, a, b);
            let (x, y) = squash(p, a, b, 0.16);
            o.sx = x;
            o.sy = y;
            o.eye_dy = -1.5 * arc(p, a, b);
            o.eye_h = 1.0 + 0.2 * arc(p, a, b);
        }
        DoubleBlink => {
            o.blink = bell(p, 0.25, 0.4) + bell(p, 0.5, 0.65);
            o.eye_dy = 0.5 * bell(p, 0.2, 0.7);
        }
        Wobble => {
            let d = 1.0 - ease(p);
            o.rot = 0.22 * (3.0 * TAU * p).sin() * d * ease(p * 5.0);
            o.sx = 1.0 + 0.05 * (2.0 * TAU * p).sin() * d;
            o.sy = 2.0 - o.sx;
        }
        LookAtViewer => {
            let h = hold(p);
            o.eye_w = 1.0 + 0.3 * h;
            o.eye_h = 1.0 - 0.1 * h;
            o.eye_dy = 1.5 * h;
            o.sx = 1.0 + 0.04 * h;
            o.sy = 1.0 + 0.04 * h;
            o.blink = bell(p, 0.55, 0.65);
        }
        SmallHop => {
            let (a, b) = (0.25, 0.6);
            o.dy = -4.0 * arc(p, a, b);
            let (x, y) = squash(p, a, b, 0.08);
            o.sx = x;
            o.sy = y;
        }
    }
    o
}

/// Stateless hash for (seed, slot): picks the trick of a slot.
fn mix(seed: u32, slot: i64) -> u32 {
    let mut h = seed ^ (slot as u32).wrapping_mul(0x9E37_79B9) ^ ((slot >> 32) as u32);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846C_A68B);
    h ^ (h >> 16)
}

/// Shuffled order of `n` tricks for cycle `c` (Fisher-Yates on the hash stream).
fn order(n: usize, seed: u32, c: i64) -> [u8; 16] {
    let mut o = [0u8; 16];
    for (i, v) in o.iter_mut().enumerate() {
        *v = i as u8;
    }
    let mut h = mix(seed, c);
    for i in (1..n.min(16)).rev() {
        h = mix(h, i as i64);
        o.swap(i, h as usize % (i + 1));
    }
    o
}

/// The trick a critter with `seed` plays in time slot `slot`: every trick of the set
/// once per cycle, in a per-critter shuffled order, never the same trick twice in a row.
pub fn trick_for(set: &[CritterTrick], seed: u32, slot: i64) -> CritterTrick {
    let n = set.len().clamp(1, 16) as i64;
    let (c, i) = (slot.div_euclid(n), slot.rem_euclid(n) as usize);
    let mut cur = order(n as usize, seed, c);
    if n > 1 && cur[0] == order(n as usize, seed, c - 1)[n as usize - 1] {
        cur.swap(0, 1);
    }
    set[cur[i] as usize]
}

/// The pose of a critter in `state` at `t` seconds. `seed` keeps critters out of step
/// (any stable per-critter number, such as [`super::critter_hash`] of its name).
pub fn critter_pose(state: CritterState, t: f64, seed: u32, reduced: bool) -> CritterPose {
    use CritterState::*;
    let (set, slot) = match state {
        Idle | Off => return CritterPose::REST,
        Working => (WORKING_TRICKS, TRICK_SLOT.0),
        Thinking => (WORKING_TRICKS, TRICK_SLOT.0 * 1.4),
        Attention => (ATTENTION_TRICKS, TRICK_SLOT.1),
    };
    let off = (seed % 997) as f64 / 997.0 * slot;
    if reduced {
        let mut o = CritterPose::REST;
        o.rot = 0.07 * (TAU * (t + off) / (2.0 * slot)).sin();
        return o;
    }
    let u = (t + off) / slot;
    let n = u.floor();
    let mut o = trick_pose(trick_for(set, seed, n as i64), u - n);
    if state == Thinking {
        o.eye_dy -= 1.5;
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &CritterPose, b: &CritterPose) -> bool {
        let f = |x: f64, y: f64| (x - y).abs() < 1e-6;
        f(a.dx, b.dx) && f(a.dy, b.dy) && f(a.rot.rem_euclid(TAU) % (TAU - 1e-9), b.rot.rem_euclid(TAU) % (TAU - 1e-9))
            && f(a.sx, b.sx) && f(a.sy, b.sy) && f(a.morph, b.morph) && f(a.yaw, b.yaw)
            && f(a.eye_w, b.eye_w) && f(a.eye_h, b.eye_h) && f(a.pill, b.pill)
            && f(a.eye_dx, b.eye_dx) && f(a.eye_dy, b.eye_dy) && f(a.blink, b.blink)
    }

    #[test]
    fn every_trick_starts_and_ends_at_rest() {
        for &k in WORKING_TRICKS.iter().chain(ATTENTION_TRICKS) {
            assert!(close(&trick_pose(k, 0.0), &CritterPose::REST), "{k:?} at 0");
            assert!(close(&trick_pose(k, 1.0), &CritterPose::REST), "{k:?} at 1");
        }
    }

    #[test]
    fn tricks_never_snap() {
        // Small time steps give small pose steps (no jumps, also across slot changes).
        for seed in [1u32, 77, 4242] {
            for st in [CritterState::Working, CritterState::Attention] {
                let mut prev = critter_pose(st, 0.0, seed, false);
                for i in 1..2000 {
                    let p = critter_pose(st, i as f64 / 240.0, seed, false);
                    let dr = (p.rot - prev.rot).rem_euclid(TAU);
                    let d = dr.min(TAU - dr) + (p.dy - prev.dy).abs() + (p.eye_dx - prev.eye_dx).abs()
                        + (p.morph - prev.morph).abs() + (p.sx - prev.sx).abs() + (p.blink - prev.blink).abs();
                    assert!(d < 0.6, "{st:?} seed {seed} jumps at step {i}: {d}");
                    prev = p;
                }
            }
        }
    }

    #[test]
    fn states_and_sets() {
        assert_eq!(critter_pose(CritterState::Idle, 3.3, 9, false), CritterPose::REST);
        assert_eq!(critter_pose(CritterState::Off, 3.3, 9, false), CritterPose::REST);
        // Reduced motion: only a gentle tilt.
        let r = critter_pose(CritterState::Working, 1.0, 9, true);
        assert!(r.rot.abs() <= 0.07 && r.morph == 0.0 && r.dy == 0.0);
        // Attention draws only from its calm set; no spins there.
        for s in 0..200 {
            assert!(ATTENTION_TRICKS.contains(&trick_for(ATTENTION_TRICKS, 5, s)));
            assert_ne!(trick_for(WORKING_TRICKS, 5, s), trick_for(WORKING_TRICKS, 5, s - 1), "slot {s}");
        }
        // Working uses the whole set, and two critters are out of step.
        let used: Vec<_> = (0..400).map(|s| trick_for(WORKING_TRICKS, 5, s)).collect();
        assert!(WORKING_TRICKS.iter().all(|k| used.contains(k)));
        let other: Vec<_> = (0..40).map(|s| trick_for(WORKING_TRICKS, 6, s)).collect();
        assert_ne!(used[..40], other[..]);
        // A spin rounds the body mid-turn and turns fully.
        let mid = trick_pose(CritterTrick::Spin, 0.5);
        assert!(mid.morph > 0.8 && (mid.rot - PI).abs() < 1e-6);
        // A hop lifts, then squashes on landing.
        assert!(trick_pose(CritterTrick::Hop, 0.42).dy < -7.0);
        let land = trick_pose(CritterTrick::Hop, 0.75);
        assert!(land.sx > 1.05 && land.sy < 0.95);
    }
}
