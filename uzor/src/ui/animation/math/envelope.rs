//! Envelopes and small motion primitives over a normalised progress `p` (0..1): the
//! building blocks of procedural character motion (tricks that start and end at rest).
//!
//! All are pure `f64 -> f64` and never snap: windows ([`bell`], [`arc`]) are 0 outside
//! `a..b`, [`hold`] and [`ring`] are 0 at both ends.

use std::f64::consts::{PI, TAU};

/// Smoothstep: 0 → 1 with zero slope at both ends (input clamped to 0..1).
pub fn smoothstep(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Ease-out with overshoot `c` (back-out; `c = 1.2` overshoots about 5%).
pub fn back_out(x: f64, c: f64) -> f64 {
    let x = x.clamp(0.0, 1.0) - 1.0;
    1.0 + (c + 1.0) * x * x * x + c * x * x
}

/// Smooth 0 → 1 → 0 over `p` in `a..b` (sin²), 0 outside.
pub fn bell(p: f64, a: f64, b: f64) -> f64 {
    if p <= a || p >= b {
        0.0
    } else {
        (PI * (p - a) / (b - a)).sin().powi(2)
    }
}

/// Jump height: a half-sine over `a..b` (steep take-off and landing), 0 outside.
pub fn arc(p: f64, a: f64, b: f64) -> f64 {
    if p <= a || p >= b {
        0.0
    } else {
        (PI * (p - a) / (b - a)).sin()
    }
}

/// Attack / hold / release: a [`smoothstep`]-started [`back_out`] (overshoot `c`, no jump
/// in speed at rest) over the first `attack`, 1, then a [`smoothstep`] back to 0 over the
/// last `release` of the progress.
pub fn hold(p: f64, attack: f64, release: f64, c: f64) -> f64 {
    if p < attack {
        back_out(smoothstep(p / attack), c)
    } else if p < 1.0 - release {
        1.0
    } else {
        1.0 - smoothstep((p - (1.0 - release)) / release)
    }
}

/// Damped oscillation: `cycles` swings over the progress, fading in fast and out with
/// `(1 - p)^decay`; 0 at both ends.
pub fn ring(p: f64, cycles: f64, decay: f64) -> f64 {
    (cycles * TAU * p).sin() * (1.0 - p).powf(decay) * smoothstep(p * 6.0)
}

/// Squash and stretch of a jump in the air over `a..b`: a crouch before take-off, a
/// stretch on take-off and a squash after landing, gain `k`; returns `(sx, sy)`.
pub fn squash_stretch(p: f64, a: f64, b: f64, k: f64) -> (f64, f64) {
    let s = bell(p, a - 0.18, a) + bell(p, b, b + 0.2) * 1.2;
    let st = bell(p, a, a + 0.18) * 0.6;
    (1.0 + k * s - 0.5 * k * st, 1.0 - k * s + k * st)
}

/// The shift that turns a body by `rot` about a pivot at (`px`, `py`) from its centre
/// instead of about the centre, for a body drawn translate → rotate → scale (`sx`, `sy`).
pub fn pivot_shift(px: f64, py: f64, sx: f64, sy: f64, rot: f64) -> (f64, f64) {
    let (vx, vy) = (px * sx, py * sy);
    let (s, c) = rot.sin_cos();
    (px - (vx * c - vy * s), py - (vx * s + vy * c))
}

/// How far a rolling ellipse (semi-axes `a` across, `b` up, resting on its long side)
/// lifts its centre at turn `rot`: 0 at rest, `a - b` standing on end.
pub fn ellipse_lift(a: f64, b: f64, rot: f64) -> f64 {
    (a * rot.sin()).hypot(b * rot.cos()) - b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ends_at_rest() {
        assert_eq!(smoothstep(0.0), 0.0);
        assert_eq!(smoothstep(1.0), 1.0);
        assert!((back_out(1.0, 1.2) - 1.0).abs() < 1e-12 && back_out(0.0, 1.2).abs() < 1e-12);
        assert!((0..=100).map(|i| back_out(i as f64 / 100.0, 1.2)).fold(0.0, f64::max) > 1.04);
        for f in [bell(0.5, 0.0, 1.0), arc(0.5, 0.0, 1.0)] {
            assert!((f - 1.0).abs() < 1e-12);
        }
        assert_eq!(bell(1.2, 0.0, 1.0), 0.0);
        assert!(hold(0.0, 0.3, 0.3, 1.2).abs() < 1e-12 && hold(1.0, 0.3, 0.3, 1.2).abs() < 1e-12);
        assert_eq!(hold(0.5, 0.3, 0.3, 1.2), 1.0);
        assert!(ring(0.0, 3.0, 1.4).abs() < 1e-12 && ring(1.0, 3.0, 1.4).abs() < 1e-12);
        assert_eq!(squash_stretch(0.0, 0.3, 0.6, 0.2), (1.0, 1.0));
    }

    #[test]
    fn pivot_stays_put() {
        let (px, py, sx, sy, r) = (0.0, -25.0, 1.1, 0.9, 0.4);
        let (dx, dy) = pivot_shift(px, py, sx, sy, r);
        let (s, c) = f64::sin_cos(r);
        let (x, y) = (dx + (px * sx * c - py * sy * s), dy + (px * sx * s + py * sy * c));
        assert!((x - px).abs() < 1e-12 && (y - py).abs() < 1e-12);
    }
}
