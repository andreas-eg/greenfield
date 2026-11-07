//! Flat shapes for the spot builder: a closed Catmull-Rom curve through control points, a
//! polygon offset inward, and ear-clipping triangulation of a polygon with holes. Points are
//! (x, z) in the ground plane. Spots are built offline (`cargo run --bin spots`), so this is
//! written for clarity over speed: a few hundred points each.

pub type P2 = (f32, f32);

fn sub(a: P2, b: P2) -> P2 {
    (a.0 - b.0, a.1 - b.1)
}
fn cross(a: P2, b: P2) -> f32 {
    a.0 * b.1 - a.1 * b.0
}

/// Twice the signed area: positive when the points go round counter-clockwise (x right, z up).
pub fn area2(p: &[P2]) -> f32 {
    (0..p.len()).map(|i| cross(p[i], p[(i + 1) % p.len()])).sum()
}

/// A closed Catmull-Rom curve through `ctrl`, `per` points to each span.
pub fn catmull_rom_closed(ctrl: &[P2], per: usize) -> Vec<P2> {
    let n = ctrl.len();
    let mut out = Vec::with_capacity(n * per);
    for i in 0..n {
        let (p0, p1, p2, p3) = (ctrl[(i + n - 1) % n], ctrl[i], ctrl[(i + 1) % n], ctrl[(i + 2) % n]);
        for k in 0..per {
            let t = k as f32 / per as f32;
            let (t2, t3) = (t * t, t * t * t);
            let f = |a: f32, b: f32, c: f32, d: f32| 0.5 * (2.0 * b + (c - a) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (3.0 * b - a - 3.0 * c + d) * t3);
            out.push((f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1)));
        }
    }
    out
}

/// The unit normal at each point pointing into the polygon.
pub fn inward_normals(p: &[P2]) -> Vec<P2> {
    let n = p.len();
    let ccw = area2(p) > 0.0;
    (0..n)
        .map(|i| {
            let t = sub(p[(i + 1) % n], p[(i + n - 1) % n]);
            let l = (t.0 * t.0 + t.1 * t.1).sqrt().max(1e-6);
            // Left of the direction of travel is inside for a counter-clockwise polygon.
            if ccw { (-t.1 / l, t.0 / l) } else { (t.1 / l, -t.0 / l) }
        })
        .collect()
}

/// Distance from `q` to the closed polyline `p`.
pub fn distance_to(p: &[P2], q: P2) -> f32 {
    let n = p.len();
    (0..n)
        .map(|i| {
            let (a, b) = (p[i], p[(i + 1) % n]);
            let ab = sub(b, a);
            let t = ((q.0 - a.0) * ab.0 + (q.1 - a.1) * ab.1) / (ab.0 * ab.0 + ab.1 * ab.1).max(1e-12);
            let t = t.clamp(0.0, 1.0);
            let c = (a.0 + ab.0 * t, a.1 + ab.1 * t);
            ((q.0 - c.0).powi(2) + (q.1 - c.1).powi(2)).sqrt()
        })
        .fold(f32::MAX, f32::min)
}

fn in_tri(p: P2, a: P2, b: P2, c: P2) -> bool {
    let (d1, d2, d3) = (cross(sub(b, a), sub(p, a)), cross(sub(c, b), sub(p, b)), cross(sub(a, c), sub(p, c)));
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}

fn same(a: P2, b: P2) -> bool {
    (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6
}

/// Splice each hole into the outer ring through a bridge from its rightmost point, giving one
/// ring with no holes (each bridge's two ends appear twice).
fn bridge(outer: &[P2], holes: &[Vec<P2>]) -> Vec<P2> {
    let mut ring: Vec<P2> = outer.to_vec();
    if area2(&ring) < 0.0 {
        ring.reverse();
    }
    let mut hs: Vec<Vec<P2>> = holes
        .iter()
        .map(|h| {
            let mut h = h.clone();
            if area2(&h) > 0.0 {
                h.reverse();
            }
            h
        })
        .collect();
    hs.sort_by(|a, b| {
        let mx = |h: &Vec<P2>| h.iter().map(|p| p.0).fold(f32::MIN, f32::max);
        mx(b).total_cmp(&mx(a))
    });
    for h in hs {
        let (mi, m) = h.iter().copied().enumerate().max_by(|a, b| a.1 .0.total_cmp(&b.1 .0)).unwrap();
        // The nearest ring edge the ray from m to +x crosses.
        let n = ring.len();
        let mut best: Option<(f32, usize)> = None;
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            if (a.1 > m.1) == (b.1 > m.1) {
                continue;
            }
            let x = a.0 + (m.1 - a.1) / (b.1 - a.1) * (b.0 - a.0);
            if x >= m.0 && best.map_or(true, |(bx, _)| x < bx) {
                best = Some((x, i));
            }
        }
        let (ix, ei) = best.expect("a hole outside its ring");
        let i_pt = (ix, m.1);
        let (a, b) = (ring[ei], ring[(ei + 1) % n]);
        let mut pi = if a.0 > b.0 { ei } else { (ei + 1) % n };
        // A ring point inside the triangle m, i, p would block the bridge: take the one at the
        // smallest angle from the ray instead.
        let p = ring[pi];
        let mut best_ang = f32::MAX;
        for (k, &q) in ring.iter().enumerate() {
            if k == pi || same(q, p) || q.0 < m.0 {
                continue;
            }
            if in_tri(q, m, i_pt, p) || in_tri(q, m, p, i_pt) {
                let ang = ((q.1 - m.1).abs()).atan2(q.0 - m.0);
                if ang < best_ang {
                    best_ang = ang;
                    pi = k;
                }
            }
        }
        let mut next = Vec::with_capacity(ring.len() + h.len() + 2);
        next.extend_from_slice(&ring[..=pi]);
        for k in 0..=h.len() {
            next.push(h[(mi + k) % h.len()]);
        }
        next.push(ring[pi]);
        next.extend_from_slice(&ring[pi + 1..]);
        ring = next;
    }
    ring
}

/// Triangles covering `outer` less `holes`, each counter-clockwise.
pub fn triangulate(outer: &[P2], holes: &[Vec<P2>]) -> Vec<[P2; 3]> {
    let mut ring = bridge(outer, holes);
    let mut out = Vec::new();
    let mut guard = 0;
    while ring.len() > 3 && guard < 100_000 {
        guard += 1;
        let n = ring.len();
        let mut clipped = false;
        for i in 0..n {
            let (a, b, c) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
            if cross(sub(b, a), sub(c, b)) <= 1e-9 {
                continue;
            }
            let blocked = ring.iter().any(|&q| !same(q, a) && !same(q, b) && !same(q, c) && in_tri(q, a, b, c));
            if blocked {
                continue;
            }
            out.push([a, b, c]);
            ring.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            // Only collinear or degenerate points left: drop the flattest.
            let n = ring.len();
            let i = (0..n)
                .min_by(|&i, &j| {
                    let f = |i: usize| cross(sub(ring[i], ring[(i + n - 1) % n]), sub(ring[(i + 1) % n], ring[i])).abs();
                    f(i).total_cmp(&f(j))
                })
                .unwrap();
            ring.remove(i);
        }
    }
    if ring.len() == 3 && cross(sub(ring[1], ring[0]), sub(ring[2], ring[1])) > 0.0 {
        out.push([ring[0], ring[1], ring[2]]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area_of(t: &[[P2; 3]]) -> f32 {
        t.iter().map(|t| area2(t) * 0.5).sum()
    }

    #[test]
    fn a_square_with_a_square_hole_is_covered_once() {
        let outer = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let hole = vec![(3.0, 3.0), (3.0, 6.0), (6.0, 6.0), (6.0, 3.0)];
        let t = triangulate(&outer, &[hole]);
        assert!((area_of(&t) - 91.0).abs() < 1e-3, "area {}", area_of(&t));
    }

    #[test]
    fn a_concave_ring_with_two_holes() {
        // An L, two holes in its arms.
        let outer = [(0.0, 0.0), (12.0, 0.0), (12.0, 4.0), (4.0, 4.0), (4.0, 12.0), (0.0, 12.0)];
        let h1 = vec![(6.0, 1.0), (9.0, 1.0), (9.0, 3.0), (6.0, 3.0)];
        let h2 = vec![(1.0, 6.0), (3.0, 6.0), (3.0, 9.0), (1.0, 9.0)];
        let t = triangulate(&outer, &[h1, h2]);
        assert!((area_of(&t) - (80.0 - 6.0 - 6.0)).abs() < 1e-3, "area {}", area_of(&t));
    }

    #[test]
    fn a_curve_through_points_passes_through_them() {
        let ctrl = [(0.0, 0.0), (4.0, 0.0), (4.0, 3.0), (0.0, 3.0)];
        let c = catmull_rom_closed(&ctrl, 8);
        assert_eq!(c.len(), 32);
        assert_eq!(c[8], (4.0, 0.0));
        assert!(distance_to(&c, (2.0, 1.5)) > 1.0);
    }
}
