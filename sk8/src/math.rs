//! Small vector maths. Right-handed, Y up, metres and radians (spec B, "Units").

use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub const fn v3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3 { x, y, z }
}

pub const UP: Vec3 = v3(0.0, 1.0, 0.0);
pub const ZERO: Vec3 = v3(0.0, 0.0, 0.0);

impl Vec3 {
    pub fn dot(self, o: Vec3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: Vec3) -> Vec3 {
        v3(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }
    pub fn len(self) -> f32 {
        self.dot(self).sqrt()
    }
    pub fn len2(self) -> f32 {
        self.dot(self)
    }
    pub fn norm(self) -> Vec3 {
        let l = self.len();
        if l < 1e-9 {
            ZERO
        } else {
            self / l
        }
    }
    /// Normalised, or `fallback` when too short to have a direction.
    pub fn norm_or(self, fallback: Vec3) -> Vec3 {
        let l = self.len();
        if l < 1e-6 {
            fallback
        } else {
            self / l
        }
    }
    /// The horizontal part (y removed).
    pub fn planar(self) -> Vec3 {
        v3(self.x, 0.0, self.z)
    }
    /// The part of `self` lying in the plane with normal `n` (unit).
    pub fn on_plane(self, n: Vec3) -> Vec3 {
        self - n * self.dot(n)
    }
    pub fn lerp(self, o: Vec3, t: f32) -> Vec3 {
        self + (o - self) * t
    }
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
    pub fn min(self, o: Vec3) -> Vec3 {
        v3(self.x.min(o.x), self.y.min(o.y), self.z.min(o.z))
    }
    pub fn max(self, o: Vec3) -> Vec3 {
        v3(self.x.max(o.x), self.y.max(o.y), self.z.max(o.z))
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f32) -> Vec3 {
        v3(self.x * s, self.y * s, self.z * s)
    }
}
impl Div<f32> for Vec3 {
    type Output = Vec3;
    fn div(self, s: f32) -> Vec3 {
        v3(self.x / s, self.y / s, self.z / s)
    }
}
impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        v3(-self.x, -self.y, -self.z)
    }
}
impl AddAssign for Vec3 {
    fn add_assign(&mut self, o: Vec3) {
        *self = *self + o;
    }
}
impl SubAssign for Vec3 {
    fn sub_assign(&mut self, o: Vec3) {
        *self = *self - o;
    }
}
impl MulAssign<f32> for Vec3 {
    fn mul_assign(&mut self, s: f32) {
        *self = *self * s;
    }
}

pub fn clamp(x: f32, lo: f32, hi: f32) -> f32 {
    x.max(lo).min(hi)
}
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
pub fn sign(x: f32) -> f32 {
    if x < 0.0 {
        -1.0
    } else {
        1.0
    }
}
/// 0 when x <= 0, 1 when x >= 1, linear between.
pub fn sat(x: f32) -> f32 {
    clamp(x, 0.0, 1.0)
}
pub fn smoothstep(x: f32) -> f32 {
    let x = sat(x);
    x * x * (3.0 - 2.0 * x)
}
pub fn move_toward(x: f32, target: f32, step: f32) -> f32 {
    if x < target {
        (x + step).min(target)
    } else {
        (x - step).max(target)
    }
}
pub fn deg(d: f32) -> f32 {
    d.to_radians()
}

/// Rodrigues rotation of `v` about the unit `axis` by `angle`.
pub fn rotate(v: Vec3, axis: Vec3, angle: f32) -> Vec3 {
    let (s, c) = angle.sin_cos();
    v * c + axis.cross(v) * s + axis * (axis.dot(v) * (1.0 - c))
}

/// Unsigned angle between two vectors.
pub fn angle_between(a: Vec3, b: Vec3) -> f32 {
    let d = clamp(a.norm().dot(b.norm()), -1.0, 1.0);
    d.acos()
}

/// Angle from `a` to `b` about `axis`, in (-pi, pi]; positive is counter-clockwise looking down `axis`.
pub fn signed_angle(a: Vec3, b: Vec3, axis: Vec3) -> f32 {
    let a = a.on_plane(axis);
    let b = b.on_plane(axis);
    if a.len2() < 1e-12 || b.len2() < 1e-12 {
        return 0.0;
    }
    let s = axis.dot(a.cross(b));
    let c = a.dot(b);
    s.atan2(c)
}

/// Rotate unit `from` toward unit `to` by at most `max_angle`, or by `fraction` of the gap
/// when that is smaller (rate-limited slerp, B.5.3 and B.8.4).
pub fn rotate_toward(from: Vec3, to: Vec3, max_angle: f32, fraction: f32) -> Vec3 {
    let ang = angle_between(from, to);
    if ang < 1e-5 {
        return to.norm_or(from);
    }
    let step = (ang * fraction).min(max_angle);
    let mut axis = from.cross(to);
    if axis.len2() < 1e-12 {
        // Opposite: any perpendicular will do.
        axis = from.cross(v3(1.0, 0.0, 0.0));
        if axis.len2() < 1e-12 {
            axis = from.cross(v3(0.0, 0.0, 1.0));
        }
    }
    rotate(from, axis.norm(), step).norm()
}

/// `fwd` made perpendicular to `up` and unit length.
pub fn orthonormal_fwd(fwd: Vec3, up: Vec3) -> Vec3 {
    let f = fwd.on_plane(up);
    if f.len2() < 1e-10 {
        // Degenerate: pick something perpendicular.
        let alt = if up.y.abs() < 0.9 { UP } else { v3(0.0, 0.0, 1.0) };
        return alt.on_plane(up).norm();
    }
    f.norm()
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}
pub const fn v2(x: f32, y: f32) -> Vec2 {
    Vec2 { x, y }
}
impl Vec2 {
    pub fn len(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    pub fn dist(self, o: Vec2) -> f32 {
        v2(self.x - o.x, self.y - o.y).len()
    }
}

/// Closest points between segments p1-q1 and p2-q2: returns (s, t) parameters in [0,1].
pub fn closest_segment_segment(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> (f32, f32) {
    let d1 = q1 - p1;
    let d2 = q2 - p2;
    let r = p1 - p2;
    let a = d1.dot(d1);
    let e = d2.dot(d2);
    let f = d2.dot(r);
    if a <= 1e-9 && e <= 1e-9 {
        return (0.0, 0.0);
    }
    if a <= 1e-9 {
        return (0.0, sat(f / e));
    }
    let c = d1.dot(r);
    if e <= 1e-9 {
        return (sat(-c / a), 0.0);
    }
    let b = d1.dot(d2);
    let denom = a * e - b * b;
    let mut s = if denom > 1e-9 { sat((b * f - c * e) / denom) } else { 0.0 };
    let mut t = (b * s + f) / e;
    if t < 0.0 {
        t = 0.0;
        s = sat(-c / a);
    } else if t > 1.0 {
        t = 1.0;
        s = sat((b - c) / a);
    }
    (s, t)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_angle_about_up() {
        let a = v3(0.0, 0.0, 1.0);
        let b = v3(1.0, 0.0, 0.0);
        // Looking down +Y, +Z to +X is clockwise in a right-handed frame: negative? cross(z,x)=+y so positive.
        assert!((signed_angle(a, b, UP) - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        let r = rotate(a, UP, std::f32::consts::FRAC_PI_2);
        assert!((r - b).len() < 1e-5);
    }
    #[test]
    fn rotate_toward_limits() {
        let r = rotate_toward(UP, v3(1.0, 0.0, 0.0), deg(10.0), 1.0);
        assert!((angle_between(UP, r) - deg(10.0)).abs() < 1e-4);
    }
}
