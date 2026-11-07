//! The world a board rides on: collision triangles with surface categories and rail
//! polylines (spec B.2.1, B.3), with the spatial queries of B.3.3.
//!
//! A spot is loaded from a `*.skate.json` file the realm ships beside its chart
//! (`beargarden/world/charts/<spot>.skate.json`); the format is `SpotFile` below.

use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Surface {
    Smooth,
    Rough,
    Slow,
    Slippery,
    VerySlow,
    Stairs,
    Unrideable,
}

impl Surface {
    /// Rolling friction multiplier, braking multiplier, grip (B.2.1 table).
    pub fn friction(self) -> f32 {
        match self {
            Surface::Smooth => 1.0,
            Surface::Rough => 1.8,
            Surface::Slow => 3.5,
            Surface::Slippery => 0.8,
            Surface::VerySlow => 6.0,
            Surface::Stairs => 2.0,
            Surface::Unrideable => 6.0,
        }
    }
    pub fn brake(self) -> f32 {
        match self {
            Surface::Smooth => 1.0,
            Surface::Rough => 1.1,
            Surface::Slow => 1.3,
            Surface::Slippery => 0.6,
            Surface::VerySlow => 1.5,
            Surface::Stairs => 1.0,
            Surface::Unrideable => 1.0,
        }
    }
    pub fn grip(self) -> f32 {
        match self {
            Surface::Smooth => 1.0,
            Surface::Rough => 0.9,
            Surface::Slow => 0.6,
            Surface::Slippery => 0.5,
            Surface::VerySlow => 0.4,
            Surface::Stairs => 0.5,
            Surface::Unrideable => 0.0,
        }
    }
    pub fn respawn_ok(self) -> bool {
        matches!(self, Surface::Smooth | Surface::Rough | Surface::Slow | Surface::Slippery)
    }
    /// Checkpoint score (B.13.2).
    pub fn checkpoint_score(self) -> f32 {
        match self {
            Surface::Smooth => 100.0,
            Surface::Rough => 50.0,
            Surface::Slow => 20.0,
            _ => -1000.0,
        }
    }
    /// The powerslide scrub multiplier: grip-like, slippery slides further.
    pub fn slide(self) -> f32 {
        match self {
            Surface::Slippery => 0.6,
            Surface::Slow | Surface::VerySlow => 1.6,
            Surface::Rough => 1.15,
            _ => 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RailKind {
    ThinRail,
    FatRail,
    Ledge,
    Curb,
    Coping,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RailMaterial {
    Metal,
    Concrete,
    Stone,
    Wood,
    Plastic,
}

impl RailMaterial {
    /// Grind drag ratio (B.10.5).
    pub fn drag(self) -> f32 {
        match self {
            RailMaterial::Metal => 1.0,
            RailMaterial::Concrete => 1.4,
            RailMaterial::Stone => 2.0,
            RailMaterial::Plastic => 0.6,
            RailMaterial::Wood => 3.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Triangle {
    pub p: [Vec3; 3],
    pub normal: Vec3,
    pub surface: Surface,
}

#[derive(Clone, Copy, Debug)]
pub struct RailChord {
    pub a: Vec3,
    pub b: Vec3,
    pub dir: Vec3,
    pub len: f32,
    /// The upright normal: perpendicular to the chord, as close to world up as it can be.
    pub normal: Vec3,
    pub rail: u32,
    /// Index of this chord within its rail.
    pub index: u32,
    pub kind: RailKind,
    pub material: RailMaterial,
}

#[derive(Clone, Debug)]
pub struct Rail {
    pub chords: Vec<u32>,
    pub closed: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub point: Vec3,
    pub normal: Vec3,
    pub t: f32,
    pub surface: Surface,
    pub tri: u32,
}

// ---- the file format ------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpotMesh {
    pub surface: Surface,
    pub vertices: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpotRail {
    pub kind: RailKind,
    pub material: RailMaterial,
    #[serde(default)]
    pub closed: bool,
    pub points: Vec<[f32; 3]>,
}

/// A named place inside the spot a session or the world can refer to.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpotMark {
    pub name: String,
    pub pos: [f32; 3],
    #[serde(default)]
    pub heading: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpotFile {
    pub name: String,
    #[serde(default)]
    pub title: String,
    pub spawn: [f32; 3],
    #[serde(default)]
    pub spawn_heading: f32,
    pub meshes: Vec<SpotMesh>,
    #[serde(default)]
    pub rails: Vec<SpotRail>,
    #[serde(default)]
    pub marks: Vec<SpotMark>,
}

// ---- the world ------------------------------------------------------------------------

const CELL: f32 = 2.0;

pub struct World {
    pub name: String,
    pub tris: Vec<Triangle>,
    pub chords: Vec<RailChord>,
    pub rails: Vec<Rail>,
    pub spawn: Vec3,
    pub spawn_heading: f32,
    pub marks: Vec<SpotMark>,
    min: Vec3,
    nx: i32,
    nz: i32,
    cells: Vec<Vec<u32>>,
    stamp: std::cell::RefCell<(u32, Vec<u32>)>,
}

fn a3(p: [f32; 3]) -> Vec3 {
    v3(p[0], p[1], p[2])
}

impl World {
    pub fn from_json(text: &str) -> Result<World, String> {
        let f: SpotFile = serde_json::from_str(text).map_err(|e| e.to_string())?;
        Ok(World::from_spot(&f))
    }

    pub fn from_spot(f: &SpotFile) -> World {
        let mut tris = Vec::new();
        for m in &f.meshes {
            for t in &m.triangles {
                let p = [a3(m.vertices[t[0] as usize]), a3(m.vertices[t[1] as usize]), a3(m.vertices[t[2] as usize])];
                let n = (p[1] - p[0]).cross(p[2] - p[0]);
                if n.len2() < 1e-12 {
                    continue;
                }
                tris.push(Triangle { p, normal: n.norm(), surface: m.surface });
            }
        }
        let mut chords = Vec::new();
        let mut rails = Vec::new();
        for (ri, r) in f.rails.iter().enumerate() {
            let mut ids = Vec::new();
            let n = r.points.len();
            let segs = if r.closed { n } else { n.saturating_sub(1) };
            for i in 0..segs {
                let a = a3(r.points[i]);
                let b = a3(r.points[(i + 1) % n]);
                let d = b - a;
                if d.len() < 1e-4 {
                    continue;
                }
                let dir = d.norm();
                // B.2.1: normalize(cross(cross(UP, dir), dir)), flipped so normal.y >= 0.
                let mut normal = UP.cross(dir).cross(dir).norm_or(UP);
                if normal.y < 0.0 {
                    normal = -normal;
                }
                ids.push(chords.len() as u32);
                chords.push(RailChord {
                    a,
                    b,
                    dir,
                    len: d.len(),
                    normal,
                    rail: ri as u32,
                    index: ids.len() as u32 - 1,
                    kind: r.kind,
                    material: r.material,
                });
            }
            rails.push(Rail { chords: ids, closed: r.closed });
        }
        World::build(f.name.clone(), tris, chords, rails, a3(f.spawn), f.spawn_heading, f.marks.clone())
    }

    fn build(
        name: String,
        tris: Vec<Triangle>,
        chords: Vec<RailChord>,
        rails: Vec<Rail>,
        spawn: Vec3,
        spawn_heading: f32,
        marks: Vec<SpotMark>,
    ) -> World {
        let mut lo = v3(f32::MAX, f32::MAX, f32::MAX);
        let mut hi = v3(f32::MIN, f32::MIN, f32::MIN);
        for t in &tris {
            for p in t.p {
                lo = lo.min(p);
                hi = hi.max(p);
            }
        }
        if tris.is_empty() {
            lo = ZERO;
            hi = ZERO;
        }
        let nx = (((hi.x - lo.x) / CELL).ceil() as i32).max(1);
        let nz = (((hi.z - lo.z) / CELL).ceil() as i32).max(1);
        let mut cells = vec![Vec::new(); (nx * nz) as usize];
        for (i, t) in tris.iter().enumerate() {
            let tl = t.p[0].min(t.p[1]).min(t.p[2]);
            let th = t.p[0].max(t.p[1]).max(t.p[2]);
            let (x0, z0) = cell_of(lo, tl, nx, nz);
            let (x1, z1) = cell_of(lo, th, nx, nz);
            for cz in z0..=z1 {
                for cx in x0..=x1 {
                    cells[(cz * nx + cx) as usize].push(i as u32);
                }
            }
        }
        let n = tris.len();
        World { name, tris, chords, rails, spawn, spawn_heading, marks, min: lo, nx, nz, cells, stamp: std::cell::RefCell::new((0, vec![0; n])) }
    }

    /// Every triangle whose cell overlaps the box (each once).
    pub fn tris_in_box(&self, lo: Vec3, hi: Vec3, out: &mut Vec<u32>) {
        out.clear();
        let (x0, z0) = cell_of(self.min, lo, self.nx, self.nz);
        let (x1, z1) = cell_of(self.min, hi, self.nx, self.nz);
        let mut st = self.stamp.borrow_mut();
        st.0 = st.0.wrapping_add(1);
        if st.0 == 0 {
            st.1.iter_mut().for_each(|s| *s = 0);
            st.0 = 1;
        }
        let mark = st.0;
        for cz in z0..=z1 {
            for cx in x0..=x1 {
                for &ti in &self.cells[(cz * self.nx + cx) as usize] {
                    if st.1[ti as usize] != mark {
                        st.1[ti as usize] = mark;
                        let t = &self.tris[ti as usize];
                        let tl = t.p[0].min(t.p[1]).min(t.p[2]);
                        let th = t.p[0].max(t.p[1]).max(t.p[2]);
                        if th.x >= lo.x && tl.x <= hi.x && th.y >= lo.y && tl.y <= hi.y && th.z >= lo.z && tl.z <= hi.z {
                            out.push(ti);
                        }
                    }
                }
            }
        }
    }

    /// First hit of the ray from `o` along unit `d` within `max` metres; only faces whose
    /// normal opposes the ray (single-sided).
    pub fn raycast(&self, o: Vec3, d: Vec3, max: f32) -> Option<Hit> {
        let e = o + d * max;
        let pad = v3(0.01, 0.01, 0.01);
        let mut ids = Vec::new();
        self.tris_in_box(o.min(e) - pad, o.max(e) + pad, &mut ids);
        let mut best: Option<Hit> = None;
        for ti in ids {
            let t = &self.tris[ti as usize];
            if t.normal.dot(d) >= 0.0 {
                continue;
            }
            if let Some(tt) = ray_tri(o, d, t) {
                if tt <= max && best.map_or(true, |b| tt < b.t) {
                    best = Some(Hit { point: o + d * tt, normal: t.normal, t: tt, surface: t.surface, tri: ti });
                }
            }
        }
        best
    }

    /// The deepest contact of a sphere with the world: (normal out of the surface, depth, surface).
    pub fn sphere_contacts(&self, c: Vec3, r: f32, out: &mut Vec<(Vec3, f32, Surface, Vec3)>) {
        out.clear();
        let pad = v3(r, r, r);
        let mut ids = Vec::new();
        self.tris_in_box(c - pad, c + pad, &mut ids);
        for ti in ids {
            let t = &self.tris[ti as usize];
            let q = closest_on_tri(c, t);
            let d = c - q;
            let dist = d.len();
            if dist < r {
                // Single-sided: only the front of a face pushes.
                if t.normal.dot(c - t.p[0]) < -0.02 {
                    continue;
                }
                let n = if dist > 1e-5 { d / dist } else { t.normal };
                out.push((n, r - dist, t.surface, t.normal));
            }
        }
    }

    /// A sphere moving from `a` to `b`: the first contact (B.3.3 sweepSphere, sampled).
    pub fn sweep_sphere(&self, a: Vec3, b: Vec3, r: f32) -> Option<Hit> {
        let d = b - a;
        let len = d.len();
        if len > 1e-6 {
            let dir = d / len;
            // A ray along the path, lowered by r toward any surface below, covers fast motion.
            if let Some(h) = self.raycast(a, dir, len + r) {
                if h.t <= len + r {
                    return Some(h);
                }
            }
        }
        let mut cs = Vec::new();
        self.sphere_contacts(b, r, &mut cs);
        let mut best: Option<(Vec3, f32, Surface, Vec3)> = None;
        for c in cs {
            if best.map_or(true, |bb| c.1 > bb.1) {
                best = Some(c);
            }
        }
        best.map(|(n, depth, s, tn)| Hit { point: b - n * (r - depth), normal: tn, t: len, surface: s, tri: 0 })
    }

    /// Up to 40 chords whose bounds overlap the box (B.3.3).
    pub fn rails_in_box(&self, lo: Vec3, hi: Vec3) -> Vec<u32> {
        let mut out = Vec::new();
        for (i, c) in self.chords.iter().enumerate() {
            let cl = c.a.min(c.b);
            let ch = c.a.max(c.b);
            if ch.x >= lo.x && cl.x <= hi.x && ch.y >= lo.y && cl.y <= hi.y && ch.z >= lo.z && cl.z <= hi.z {
                out.push(i as u32);
                if out.len() >= 40 {
                    break;
                }
            }
        }
        out
    }

    /// The next chord along a rail from chord `c`, forward (`+1`) or back (`-1`).
    pub fn next_chord(&self, c: u32, step: i32) -> Option<u32> {
        let ch = &self.chords[c as usize];
        let rail = &self.rails[ch.rail as usize];
        let n = rail.chords.len() as i32;
        let i = ch.index as i32 + step;
        if i >= 0 && i < n {
            Some(rail.chords[i as usize])
        } else if rail.closed {
            Some(rail.chords[((i % n + n) % n) as usize])
        } else {
            None
        }
    }

    /// Distance from `p` to the nearest rail chord.
    pub fn rail_distance(&self, p: Vec3) -> f32 {
        let mut best = f32::MAX;
        for c in &self.chords {
            let s = clamp((p - c.a).dot(c.dir), 0.0, c.len);
            best = best.min((c.a + c.dir * s - p).len());
        }
        best
    }

    /// The lowest point of the spot: a board below it has left the world.
    pub fn floor_y(&self) -> f32 {
        self.min.y
    }

    pub fn mark(&self, name: &str) -> Option<&SpotMark> {
        self.marks.iter().find(|m| m.name == name)
    }
}

fn cell_of(min: Vec3, p: Vec3, nx: i32, nz: i32) -> (i32, i32) {
    let cx = ((p.x - min.x) / CELL).floor() as i32;
    let cz = ((p.z - min.z) / CELL).floor() as i32;
    (cx.clamp(0, nx - 1), cz.clamp(0, nz - 1))
}

fn ray_tri(o: Vec3, d: Vec3, t: &Triangle) -> Option<f32> {
    let e1 = t.p[1] - t.p[0];
    let e2 = t.p[2] - t.p[0];
    let h = d.cross(e2);
    let a = e1.dot(h);
    if a.abs() < 1e-9 {
        return None;
    }
    let f = 1.0 / a;
    let s = o - t.p[0];
    let u = f * s.dot(h);
    if !(-1e-5..=1.0 + 1e-5).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = f * d.dot(q);
    if v < -1e-5 || u + v > 1.0 + 1e-5 {
        return None;
    }
    let tt = f * e2.dot(q);
    if tt >= -1e-4 {
        Some(tt.max(0.0))
    } else {
        None
    }
}

/// Closest point on a triangle to `p` (Ericson, Real-Time Collision Detection 5.1.5).
pub fn closest_on_tri(p: Vec3, t: &Triangle) -> Vec3 {
    let (a, b, c) = (t.p[0], t.p[1], t.p[2]);
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denom = 1.0 / (va + vb + vc);
    a + ab * (vb * denom) + ac * (vc * denom)
}
