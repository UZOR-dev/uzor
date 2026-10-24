//! Fake 3D for flat drawings: turn a 2D shape about any axis in space and project it
//! back onto the screen.
//!
//! A [`Rot3`] is a rotation (a unit quaternion). [`Rot3::project`] takes a point of the
//! drawing's plane (z = 0, y down as on screen), rotates it and projects it, with an
//! optional camera distance for a little perspective. A polygon stays a polygon under
//! it (a hexagon foreshortens into a flatter hexagon, turns edge-on and flips over), so a
//! path keeps its outline and only its points move. [`Rot3::facing`] says how squarely the
//! plane faces the viewer (1 face-on, 0 edge-on, negative from behind), for shading.
//!
//! ```
//! use uzor::ui::animation::math::spatial::Rot3;
//! use std::f64::consts::PI;
//! // a quarter turn about the vertical axis: the plane is edge-on, x collapses
//! let r = Rot3::about([0.0, 1.0, 0.0], PI / 2.0);
//! let (x, y) = r.project(10.0, 5.0, 0.0);
//! assert!(x.abs() < 1e-9 && (y - 5.0).abs() < 1e-9);
//! assert!(r.facing().abs() < 1e-9);
//! ```

/// A rotation in 3D: a unit quaternion (w, x, y, z).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rot3 {
    /// Scalar part.
    pub w: f64,
    /// Vector part.
    pub x: f64,
    /// Vector part.
    pub y: f64,
    /// Vector part.
    pub z: f64,
}

impl Default for Rot3 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Rot3 {
    /// No rotation.
    pub const IDENTITY: Rot3 = Rot3 { w: 1.0, x: 0.0, y: 0.0, z: 0.0 };

    /// Turn by `angle` radians about `axis` (any length; x right, y down, z toward the
    /// viewer). A zero axis gives no rotation.
    pub fn about(axis: [f64; 3], angle: f64) -> Rot3 {
        let n = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
        if n < 1e-12 {
            return Self::IDENTITY;
        }
        let (s, c) = (angle / 2.0).sin_cos();
        let k = s / n;
        Rot3 { w: c, x: axis[0] * k, y: axis[1] * k, z: axis[2] * k }
    }

    /// Turn by `angle` about an axis lying in the drawing's plane at `dir` radians from the
    /// x axis (clockwise on screen), tilted `lift` toward the viewer (0 = in the plane).
    pub fn about_dir(dir: f64, lift: f64, angle: f64) -> Rot3 {
        let (s, c) = dir.sin_cos();
        Self::about([c, s, lift], angle)
    }

    /// `self` first, then `next`.
    pub fn then(&self, next: &Rot3) -> Rot3 {
        let (a, b) = (next, self);
        Rot3 {
            w: a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
            x: a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
            y: a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
            z: a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
        }
    }

    /// Blend toward `o` by `k` (0 = self, 1 = `o`) along the shorter way round
    /// (normalised lerp: smooth, and exact at both ends).
    pub fn nlerp(&self, o: &Rot3, k: f64) -> Rot3 {
        let d = self.w * o.w + self.x * o.x + self.y * o.y + self.z * o.z;
        let s = if d < 0.0 { -1.0 } else { 1.0 };
        let l = |a: f64, b: f64| a + (s * b - a) * k;
        let r = [l(self.w, o.w), l(self.x, o.x), l(self.y, o.y), l(self.z, o.z)];
        let n = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2] + r[3] * r[3]).sqrt().max(1e-12);
        Rot3 { w: r[0] / n, x: r[1] / n, y: r[2] / n, z: r[3] / n }
    }

    /// Rotate a vector.
    pub fn rotate(&self, v: [f64; 3]) -> [f64; 3] {
        let (w, a, b, c) = (self.w, self.x, self.y, self.z);
        let t = [2.0 * (b * v[2] - c * v[1]), 2.0 * (c * v[0] - a * v[2]), 2.0 * (a * v[1] - b * v[0])];
        [
            v[0] + w * t[0] + (b * t[2] - c * t[1]),
            v[1] + w * t[1] + (c * t[0] - a * t[2]),
            v[2] + w * t[2] + (a * t[1] - b * t[0]),
        ]
    }

    /// Screen position of the plane point (`x`, `y`) after the rotation, relative to the
    /// rotation centre. `camera`: viewer distance in the same units (nearer parts grow);
    /// 0 projects orthographically.
    pub fn project(&self, x: f64, y: f64, camera: f64) -> (f64, f64) {
        let [px, py, pz] = self.rotate([x, y, 0.0]);
        let k = if camera > 0.0 { camera / (camera - pz).max(camera * 0.1) } else { 1.0 };
        (px * k, py * k)
    }

    /// How squarely the plane faces the viewer: the z of its turned normal (1 face-on,
    /// 0 edge-on, -1 seen from behind).
    pub fn facing(&self) -> f64 {
        1.0 - 2.0 * (self.x * self.x + self.y * self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn half_turn_about_a_symmetry_axis_lands_on_the_mirror_point() {
        let r = Rot3::about_dir(0.0, 0.0, PI);
        let (x, y) = r.project(3.0, 4.0, 0.0);
        assert!((x - 3.0).abs() < 1e-9 && (y + 4.0).abs() < 1e-9);
        assert!((r.facing() + 1.0).abs() < 1e-9);
    }

    #[test]
    fn composition_and_blend_are_exact_at_the_ends() {
        let a = Rot3::about([1.0, 0.0, 0.0], 0.3);
        let b = Rot3::about([0.0, 0.0, 1.0], 0.8);
        let ab = a.then(&b);
        let v = ab.rotate([1.0, 2.0, 0.0]);
        let w = b.rotate(a.rotate([1.0, 2.0, 0.0]));
        assert!((0..3).all(|i| (v[i] - w[i]).abs() < 1e-9));
        let s = a.nlerp(&b, 0.0);
        assert!((s.w - a.w).abs() < 1e-12 && (s.x - a.x).abs() < 1e-12);
        let e = a.nlerp(&b, 1.0);
        assert!((e.w - b.w).abs() < 1e-9 && (e.z - b.z).abs() < 1e-9);
    }

    #[test]
    fn perspective_grows_the_near_side() {
        let r = Rot3::about([1.0, 0.0, 0.0], 0.5);
        let (_, near) = r.project(0.0, 10.0, 100.0);
        let (_, flat) = r.project(0.0, 10.0, 0.0);
        assert!(near.abs() > flat.abs());
    }
}
