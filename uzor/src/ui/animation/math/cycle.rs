//! Phase sequencing for code-driven motion: a run of short "tricks", each from rest
//! back to rest, picked per time slot without stored state.
//!
//! [`TrickCycle`] cuts time into slots and says which slot `t` falls in and how far the
//! trick of that slot has run ([`TrickCycle::at`]); a stateless [`slot_hash`] of a seed
//! and the slot picks which trick plays, so two instances with different seeds never move
//! in step. Since every trick starts and ends at rest, slot changes never jump. The
//! shaping helpers are what such tricks are made of: [`back_in_out`] (ease with a small
//! overshoot either side), [`breathe`] (contract, then rebound past rest, then settle) and
//! [`turn_delta`] (the shortest turn between two angles of an n-fold symmetric shape, so
//! a blend between two counter-spinning poses never spins the long way round).
//!
//! ```
//! use uzor::ui::animation::math::cycle::{breathe, TrickCycle};
//! let c = TrickCycle { slot: 2.0, play: 0.5 };
//! assert_eq!(c.at(3.0), (1, None)); // second slot, its trick is over (resting)
//! assert_eq!(c.at(2.5), (1, Some(0.5)));
//! assert_eq!(breathe(0.0, 0.2), 1.0);
//! assert!(breathe(0.3, 0.2) < 0.85);
//! ```

use std::f64::consts::PI;

/// Time cut into equal slots; a trick plays over the first `play` fraction of each.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrickCycle {
    /// Seconds per slot.
    pub slot: f64,
    /// Fraction of the slot the trick runs (1 = back to back), the rest holds still.
    pub play: f64,
}

impl TrickCycle {
    /// Slot index at `t` seconds and the trick's progress 0..1 in it (`None` once the
    /// trick of this slot is over).
    pub fn at(&self, t: f64) -> (u32, Option<f64>) {
        let f = t / self.slot.max(1e-6);
        let slot = f.floor();
        let u = (f - slot) / self.play.clamp(1e-6, 1.0);
        (slot as i64 as u32, (u < 1.0).then_some(u.max(0.0)))
    }
}

/// A 32-bit hash of (`seed`, `slot`): which trick a slot plays, which way it turns.
pub fn slot_hash(seed: u32, slot: u32) -> u32 {
    let mut h = seed.wrapping_mul(0x9E37_79B1) ^ slot.wrapping_mul(0x85EB_CA77);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846C_A68B);
    h ^ (h >> 16)
}

/// Ease in and out with an overshoot of strength `s` past both ends (0 = none, about
/// 1 = clearly springy); 0 at 0, 1 at 1.
pub fn back_in_out(u: f64, s: f64) -> f64 {
    let s = s * 1.525;
    let u = u.clamp(0.0, 1.0);
    if u < 0.5 {
        let v = 2.0 * u;
        v * v * ((s + 1.0) * v - s) / 2.0
    } else {
        let v = 2.0 * u - 2.0;
        (v * v * ((s + 1.0) * v + s) + 2.0) / 2.0
    }
}

/// Smoothstep.
pub fn smooth(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// A breath over `u` 0..1 as a scale: contracts by `depth` (most at 0.3), rebounds a
/// third of that past rest (at 0.8), settles at 1. 1 at both ends.
pub fn breathe(u: f64, depth: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    if u < 0.6 {
        1.0 - depth * smooth((PI * u / 0.6).sin())
    } else {
        1.0 + depth * 0.35 * (PI * (u - 0.6) / 0.4).sin()
    }
}

/// The turn from angle `a` to `b` (radians) for a shape with `fold`-fold rotational
/// symmetry, taken the short way: in -π/fold ..= π/fold.
pub fn turn_delta(a: f64, b: f64, fold: u32) -> f64 {
    let p = 2.0 * PI / fold.max(1) as f64;
    let d = b - a;
    d - p * (d / p).round()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tricks_start_and_end_at_rest() {
        for s in [0.0, 0.6, 1.1] {
            assert!(back_in_out(0.0, s).abs() < 1e-12 && (back_in_out(1.0, s) - 1.0).abs() < 1e-12);
        }
        assert!(back_in_out(0.1, 1.0) < 0.0, "pulls back before it goes");
        assert!((breathe(1.0, 0.3) - 1.0).abs() < 1e-9);
        assert!(breathe(0.8, 0.3) > 1.0, "rebounds past rest");
    }

    #[test]
    fn slots_and_hash() {
        let c = TrickCycle { slot: 1.25, play: 1.0 };
        assert_eq!(c.at(0.0), (0, Some(0.0)));
        assert_eq!(c.at(2.5).0, 2);
        assert_ne!(slot_hash(7, 1), slot_hash(7, 2));
        assert_ne!(slot_hash(7, 1), slot_hash(8, 1));
        // same numbers as the JS prototype's 32-bit hash
        assert_eq!([slot_hash(7, 0), slot_hash(7, 1), slot_hash(7, 12345)], [1431726239, 3423267222, 602759169]);
    }

    #[test]
    fn symmetric_turns_take_the_short_way() {
        use std::f64::consts::PI;
        assert!((turn_delta(0.0, PI / 2.0 + 0.1, 4) - 0.1).abs() < 1e-9);
        assert!(turn_delta(0.0, -3.0 * PI + 0.2, 4).abs() <= PI / 4.0 + 1e-9);
        assert!((turn_delta(1.0, 1.0 + 4.0 * PI / 2.0, 4)).abs() < 1e-9);
    }
}
