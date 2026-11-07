//! A small builder for skate spots: ground, boxes, ledges, rails, banks, quarter-pipes,
//! bowls and stair sets, each with its collision triangles, surface category and rail
//! polylines (spec B.2.1, B.3). The spots in `spots.rs` are made with it.

use crate::math::*;
use crate::poly::{self, P2};
use crate::world::*;

#[derive(Default)]
pub struct Builder {
    meshes: Vec<SpotMesh>,
    rails: Vec<SpotRail>,
    marks: Vec<SpotMark>,
}

fn arr(v: Vec3) -> [f32; 3] {
    [v.x, v.y, v.z]
}

impl Builder {
    pub fn new() -> Builder {
        Builder::default()
    }

    fn mesh(&mut self, s: Surface) -> &mut SpotMesh {
        if let Some(i) = self.meshes.iter().position(|m| m.surface == s) {
            return &mut self.meshes[i];
        }
        self.meshes.push(SpotMesh { surface: s, vertices: Vec::new(), triangles: Vec::new() });
        self.meshes.last_mut().unwrap()
    }

    /// A triangle whose normal faces `out` (winding fixed if needed).
    pub fn tri(&mut self, s: Surface, a: Vec3, b: Vec3, c: Vec3, out: Vec3) {
        let n = (b - a).cross(c - a);
        let (b, c) = if n.dot(out) < 0.0 { (c, b) } else { (b, c) };
        let m = self.mesh(s);
        let i = m.vertices.len() as u32;
        m.vertices.extend_from_slice(&[arr(a), arr(b), arr(c)]);
        m.triangles.push([i, i + 1, i + 2]);
    }

    /// A quad a-b-c-d (in order round its edge) facing `out`.
    pub fn quad(&mut self, s: Surface, a: Vec3, b: Vec3, c: Vec3, d: Vec3, out: Vec3) {
        self.tri(s, a, b, c, out);
        self.tri(s, a, c, d, out);
    }

    /// Flat ground from (x0, z0) to (x1, z1) at height y, in tiles no bigger than 8 m.
    pub fn ground(&mut self, s: Surface, x0: f32, z0: f32, x1: f32, z1: f32, y: f32) {
        let nx = ((x1 - x0) / 8.0).ceil().max(1.0) as i32;
        let nz = ((z1 - z0) / 8.0).ceil().max(1.0) as i32;
        for i in 0..nx {
            for j in 0..nz {
                let ax = lerp(x0, x1, i as f32 / nx as f32);
                let bx = lerp(x0, x1, (i + 1) as f32 / nx as f32);
                let az = lerp(z0, z1, j as f32 / nz as f32);
                let bz = lerp(z0, z1, (j + 1) as f32 / nz as f32);
                self.quad(s, v3(ax, y, az), v3(bx, y, az), v3(bx, y, bz), v3(ax, y, bz), UP);
            }
        }
    }

    /// Ground over a rectangle with a rectangular hole (for a sunken bowl).
    pub fn ground_around(&mut self, s: Surface, outer: (f32, f32, f32, f32), hole: (f32, f32, f32, f32), y: f32) {
        let (x0, z0, x1, z1) = outer;
        let (hx0, hz0, hx1, hz1) = hole;
        self.ground(s, x0, z0, x1, hz0, y);
        self.ground(s, x0, hz1, x1, z1, y);
        self.ground(s, x0, hz0, hx0, hz1, y);
        self.ground(s, hx1, hz0, x1, hz1, y);
    }

    /// A box standing on y0: centre (x, z), size w (local x) by d (local z), height h,
    /// turned `yaw` radians. Top and four sides. Returns the four top corners in order.
    pub fn block(&mut self, s: Surface, x: f32, z: f32, y0: f32, w: f32, d: f32, h: f32, yaw: f32) -> [Vec3; 4] {
        let (sn, cs) = yaw.sin_cos();
        let ax = v3(cs, 0.0, -sn) * (w * 0.5);
        let az = v3(sn, 0.0, cs) * (d * 0.5);
        let c = v3(x, y0, z);
        let b = [c - ax - az, c + ax - az, c + ax + az, c - ax + az];
        let t: Vec<Vec3> = b.iter().map(|p| *p + UP * h).collect();
        self.quad(s, t[0], t[1], t[2], t[3], UP);
        for i in 0..4 {
            let j = (i + 1) % 4;
            let mid = (b[i] + b[j]) * 0.5;
            let out = (mid - c).planar().norm();
            self.quad(s, b[i], b[j], t[j], t[i], out);
        }
        [t[0], t[1], t[2], t[3]]
    }

    pub fn rail(&mut self, kind: RailKind, material: RailMaterial, points: &[Vec3], closed: bool) {
        self.rails.push(SpotRail { kind, material, closed, points: points.iter().map(|p| arr(*p)).collect() });
    }

    /// A ledge: a block whose two long top edges are grindable.
    pub fn ledge(&mut self, s: Surface, mat: RailMaterial, x: f32, z: f32, y0: f32, w: f32, d: f32, h: f32, yaw: f32) {
        let t = self.block(s, x, z, y0, w, d, h, yaw);
        let kind = if h < 0.22 { RailKind::Curb } else { RailKind::Ledge };
        // The long sides: the pair of edges along the longer dimension.
        if d >= w {
            self.rail(kind, mat, &[t[1], t[2]], false);
            self.rail(kind, mat, &[t[0], t[3]], false);
        } else {
            self.rail(kind, mat, &[t[0], t[1]], false);
            self.rail(kind, mat, &[t[3], t[2]], false);
        }
    }

    /// A free-standing rail (no collision of its own: posts are thin enough to ignore).
    pub fn flat_rail(&mut self, a: Vec3, b: Vec3, round: bool) {
        let kind = if round { RailKind::ThinRail } else { RailKind::FatRail };
        self.rail(kind, RailMaterial::Metal, &[a, b], false);
    }

    /// A flat bank: from ground at the low edge to height h at the high edge, w wide, run
    /// metres long, then a deck `top` metres deep, with a back wall. `dir` is the way up.
    pub fn bank(&mut self, s: Surface, x: f32, z: f32, y0: f32, w: f32, run: f32, h: f32, top: f32, yaw: f32) {
        let (sn, cs) = yaw.sin_cos();
        let fw = v3(sn, 0.0, cs);
        let side = v3(cs, 0.0, -sn) * (w * 0.5);
        let c = v3(x, y0, z);
        let a0 = c - side;
        let a1 = c + side;
        let b0 = a0 + fw * run + UP * h;
        let b1 = a1 + fw * run + UP * h;
        let up_n = (b0 - a0).cross(a1 - a0).norm();
        let up_n = if up_n.y < 0.0 { -up_n } else { up_n };
        self.quad(s, a0, a1, b1, b0, up_n);
        let d0 = b0 + fw * top;
        let d1 = b1 + fw * top;
        self.quad(s, b0, b1, d1, d0, UP);
        // Back and sides.
        let g0 = d0 - UP * h;
        let g1 = d1 - UP * h;
        self.quad(s, d0, d1, g1, g0, fw);
        let bb0 = b0 - UP * h;
        let bb1 = b1 - UP * h;
        self.tri(s, a0, b0, bb0, -side);
        self.quad(s, b0, d0, g0, bb0, -side);
        self.tri(s, a1, b1, bb1, side);
        self.quad(s, b1, d1, g1, bb1, side);
        // The top edge of the bank is a ledge you can grind.
        self.rail(RailKind::Ledge, RailMaterial::Concrete, &[d0, d1], false);
    }

    /// The profile of a transition: points (horizontal distance from the lip wall,
    /// height) from the flat bottom up to vertical, then `vert` metres of vertical.
    fn transition_profile(radius: f32, vert: f32, segs: usize) -> Vec<(f32, f32)> {
        let mut pts = Vec::new();
        for i in 0..=segs {
            let a = (i as f32 / segs as f32) * std::f32::consts::FRAC_PI_2;
            // Circle centre at (0, radius) relative to the wall foot; from the bottom (a = 0)
            // to the wall (a = 90 deg).
            pts.push((radius - radius * a.sin(), radius - radius * a.cos()));
        }
        if vert > 0.0 {
            pts.push((0.0, radius + vert));
        }
        pts
    }

    /// A quarter-pipe: its flat bottom meets ground y0 at (x, z), it rises along `yaw`, w
    /// wide, transition `radius`, `vert` of vertical, a `deck` behind the coping.
    pub fn quarter_pipe(&mut self, s: Surface, x: f32, z: f32, y0: f32, w: f32, radius: f32, vert: f32, deck: f32, yaw: f32) {
        let (sn, cs) = yaw.sin_cos();
        let fw = v3(sn, 0.0, cs);
        let side = v3(cs, 0.0, -sn) * (w * 0.5);
        // The wall foot is `radius` forward of the bottom edge.
        let foot = v3(x, y0, z) + fw * radius;
        let prof = Builder::transition_profile(radius, vert, 14);
        let pt = |d: f32, h: f32, sgn: f32| foot - fw * d + UP * h + side * sgn;
        for i in 0..prof.len() - 1 {
            let (d0, h0) = prof[i];
            let (d1, h1) = prof[i + 1];
            let a0 = pt(d0, h0, -1.0);
            let a1 = pt(d0, h0, 1.0);
            let b0 = pt(d1, h1, -1.0);
            let b1 = pt(d1, h1, 1.0);
            // Facing back toward the rider (into the curve's centre).
            let mid_d = (d0 + d1) * 0.5;
            let mid_h = (h0 + h1) * 0.5;
            let centre = foot - fw * radius + UP * radius;
            let out = (centre - (foot - fw * mid_d + UP * mid_h)).norm_or(-fw);
            self.quad(s, a0, a1, b1, b0, out);
        }
        let top = radius + vert;
        // Deck.
        let l0 = pt(0.0, top, -1.0);
        let l1 = pt(0.0, top, 1.0);
        let e0 = l0 + fw * deck;
        let e1 = l1 + fw * deck;
        self.quad(s, l0, l1, e1, e0, UP);
        // Back wall and sides.
        let g0 = e0 - UP * top;
        let g1 = e1 - UP * top;
        self.quad(s, e0, e1, g1, g0, fw);
        for sgn in [-1.0f32, 1.0] {
            let out = side * sgn;
            let mut prev = pt(prof[0].0, prof[0].1, sgn);
            for &(d, h) in prof.iter().skip(1) {
                let cur = pt(d, h, sgn);
                let below_prev = v3(prev.x, y0, prev.z);
                let below_cur = v3(cur.x, y0, cur.z);
                self.quad(s, prev, cur, below_cur, below_prev, out);
                prev = cur;
            }
            let lip = pt(0.0, top, sgn);
            let edge = lip + fw * deck;
            self.quad(s, lip, edge, v3(edge.x, y0, edge.z), v3(lip.x, y0, lip.z), out);
        }
        // Coping: a pipe along the lip, standing 4 cm proud of the wall.
        let c0 = pt(0.04, top, -1.0);
        let c1 = pt(0.04, top, 1.0);
        self.rail(RailKind::Coping, RailMaterial::Metal, &[c0, c1], false);
    }

    /// A sunken round bowl: floor radius `floor`, transition `radius`, `vert`, deck at y0;
    /// returns the hole to leave in the ground (square of half size `half`).
    pub fn bowl(&mut self, s: Surface, x: f32, z: f32, y0: f32, floor: f32, radius: f32, vert: f32, half: f32) -> (f32, f32, f32, f32) {
        let segs = 40;
        let depth = radius + vert;
        let c = v3(x, y0 - depth, z);
        let prof = Builder::transition_profile(radius, vert, 12);
        let rim = floor + radius;
        let dir = |i: usize| {
            let a = i as f32 / segs as f32 * std::f32::consts::TAU;
            v3(a.cos(), 0.0, a.sin())
        };
        for i in 0..segs {
            let d0 = dir(i);
            let d1 = dir(i + 1);
            // Floor wedge.
            self.tri(s, c, c + d0 * floor, c + d1 * floor, UP);
            // Walls.
            for k in 0..prof.len() - 1 {
                let (a, ha) = prof[k];
                let (b, hb) = prof[k + 1];
                let ra = rim - a;
                let rb = rim - b;
                let p00 = c + d0 * ra + UP * ha;
                let p01 = c + d1 * ra + UP * ha;
                let p10 = c + d0 * rb + UP * hb;
                let p11 = c + d1 * rb + UP * hb;
                let mid = (d0 + d1).norm();
                let centre = c + mid * floor + UP * radius;
                let out = (centre - (p00 + p11) * 0.5).norm_or(UP);
                self.quad(s, p00, p01, p11, p10, out);
            }
            // Deck ring from the rim out to the square.
            let sq = |d: Vec3| d * (half / d.x.abs().max(d.z.abs()));
            let r0 = v3(x, y0, z) + d0 * rim;
            let r1 = v3(x, y0, z) + d1 * rim;
            let o0 = v3(x, y0, z) + sq(d0);
            let o1 = v3(x, y0, z) + sq(d1);
            self.quad(s, r0, r1, o1, o0, UP);
        }
        // The square's corners are covered by the ring because sq() follows the square.
        let pts: Vec<Vec3> = (0..segs).map(|i| v3(x, y0, z) + dir(i) * (rim - 0.04)).collect();
        self.rail(RailKind::Coping, RailMaterial::Metal, &pts, true);
        (x - half, z - half, x + half, z + half)
    }

    /// Flat ground at height `y` over `outer` (x, z points round its edge) less `holes`.
    pub fn polygon(&mut self, s: Surface, outer: &[P2], holes: &[Vec<P2>], y: f32) {
        for t in poly::triangulate(outer, holes) {
            let p = |q: P2| v3(q.0, y, q.1);
            self.tri(s, p(t[0]), p(t[1]), p(t[2]), UP);
        }
    }

    /// A sunken bowl of any shape: its rim the closed curve `rim` (x, z) at the deck's height
    /// `y0`; its wall a quarter circle of radius `depth`, vertical at the lip and flat at the
    /// bottom, `depth` down, in `rings` steps; a flat floor; metal coping along the rim. The
    /// deck round it is the caller's (`polygon` with the rim as a hole). Returns the floor's
    /// edge, for a check that the wall did not fold over itself.
    pub fn freeform_bowl(&mut self, s: Surface, rim: &[P2], y0: f32, depth: f32, rings: usize) -> Vec<P2> {
        let n = rim.len();
        let inward = poly::inward_normals(rim);
        let ring = |k: usize| -> (Vec<P2>, f32) {
            let th = k as f32 / rings as f32 * std::f32::consts::FRAC_PI_2;
            let d = depth * (1.0 - th.cos());
            let pts = (0..n).map(|i| (rim[i].0 + inward[i].0 * d, rim[i].1 + inward[i].1 * d)).collect();
            (pts, y0 - depth * th.sin())
        };
        for k in 0..rings {
            let ((a, ya), (b, yb)) = (ring(k), ring(k + 1));
            let mid = (k as f32 + 0.5) / rings as f32 * std::f32::consts::FRAC_PI_2;
            for i in 0..n {
                let j = (i + 1) % n;
                let nm = v3(inward[i].0 + inward[j].0, 0.0, inward[i].1 + inward[j].1).norm_or(UP);
                let out = (nm * mid.cos() + UP * mid.sin()).norm_or(UP);
                self.quad(s, v3(a[i].0, ya, a[i].1), v3(a[j].0, ya, a[j].1), v3(b[j].0, yb, b[j].1), v3(b[i].0, yb, b[i].1), out);
            }
        }
        let (floor, yf) = ring(rings);
        self.polygon(s, &floor, &[], yf);
        let coping: Vec<Vec3> = (0..n).map(|i| v3(rim[i].0 + inward[i].0 * 0.04, y0, rim[i].1 + inward[i].1 * 0.04)).collect();
        self.rail(RailKind::Coping, RailMaterial::Metal, &coping, true);
        floor
    }

    /// A flight of `n` steps down from y0, going along `yaw`, each `rise` by `tread`, w wide,
    /// with a handrail on the right at 0.8 m and a landing at the bottom left to the caller.
    pub fn stairs(&mut self, x: f32, z: f32, y0: f32, w: f32, n: usize, rise: f32, tread: f32, yaw: f32, handrail: bool) {
        let (sn, cs) = yaw.sin_cos();
        let fw = v3(sn, 0.0, cs);
        let side = v3(cs, 0.0, -sn);
        let base = y0 - rise * n as f32;
        for i in 0..n.saturating_sub(1) {
            let top = y0 - rise * (i + 1) as f32;
            let c = v3(x, 0.0, z) + fw * (tread * (i as f32 + 0.5));
            self.block(Surface::Stairs, c.x, c.z, base, w, tread, top - base, yaw);
        }
        if handrail {
            let a = v3(x, y0, z) + side * (w * 0.5 + 0.2) + UP * 0.8 - fw * 0.3;
            let b = v3(x, y0 - rise * n as f32, z) + side * (w * 0.5 + 0.2) + fw * (tread * n as f32 + 0.3) + UP * 0.8;
            self.rail(RailKind::ThinRail, RailMaterial::Metal, &[a, b], false);
        }
    }

    pub fn mark(&mut self, name: &str, pos: Vec3, heading: f32) {
        self.marks.push(SpotMark { name: name.to_string(), pos: arr(pos), heading });
    }

    pub fn finish(self, name: &str, title: &str, spawn: Vec3, heading: f32) -> SpotFile {
        SpotFile { name: name.to_string(), title: title.to_string(), spawn: arr(spawn), spawn_heading: heading, meshes: self.meshes, rails: self.rails, marks: self.marks }
    }
}
