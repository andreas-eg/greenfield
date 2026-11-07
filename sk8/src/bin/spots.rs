//! Write every spot into the realm: `<dir>/<name>.skate.json` (collision triangles, rails
//! and marks, read by sk8 in the lens and in play/), `<dir>/<name>.json` (the chart the
//! room names with set_chart: one instance, the spot's mesh, a walkmask, and "skate": the
//! file above) and `<meshes>/spot_<name>.glb` (what the lens draws: the same triangles and
//! the rails, coloured by surface from a small palette texture, until GeoGen has surveyed
//! meshes for the spots, GF-REQ-GEOGEN-1).
//!
//!     cargo run --bin spots -- ../world/charts ../conditioned/beargarden

use sk8::world::{RailKind, RailMaterial, SpotFile};

fn bounds(f: &SpotFile) -> [f32; 4] {
    let mut b = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for m in &f.meshes {
        for v in &m.vertices {
            b[0] = b[0].min(v[0]);
            b[1] = b[1].min(v[2]);
            b[2] = b[2].max(v[0]);
            b[3] = b[3].max(v[2]);
        }
    }
    b
}

/// The palette: one texel a surface, four by four.
const PALETTE: [(&str, [u8; 3]); 10] = [
    ("smooth", [188, 184, 176]),
    ("rough", [138, 134, 128]),
    ("slow", [96, 132, 70]),
    ("slippery", [176, 196, 210]),
    ("veryslow", [112, 92, 66]),
    ("stairs", [204, 198, 186]),
    ("unrideable", [78, 82, 96]),
    ("metal", [196, 120, 52]),
    ("edge", [236, 232, 222]),
    ("coping", [150, 156, 164]),
];

fn texel(name: &str) -> [f32; 2] {
    let i = PALETTE.iter().position(|(n, _)| *n == name).unwrap_or(0);
    [((i % 4) as f32 + 0.5) / 4.0, ((i / 4) as f32 + 0.5) / 4.0]
}

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

/// The palette as a 4x4 texture.
fn palette_png() -> Vec<u8> {
    png(4, 4, |x, y| PALETTE.get(y * 4 + x).map(|p| p.1).unwrap_or([128, 128, 128]))
}

/// A spot's floor, tiled: what the ground it is laid in looks like (`pattern`), as a
/// 64x64 texture repeating every `FLOOR_TILE` metres.
const FLOOR_TILE: f32 = 2.4;

fn floor_png(pattern: &str) -> Vec<u8> {
    // A little grain so a large floor is not one flat colour.
    let grain = |x: usize, y: usize| -> i32 { (((x * 7919 + y * 104_729) ^ (x * y * 31)) % 13) as i32 - 6 };
    let shade = |c: [u8; 3], d: i32| [(c[0] as i32 + d).clamp(0, 255) as u8, (c[1] as i32 + d).clamp(0, 255) as u8, (c[2] as i32 + d).clamp(0, 255) as u8];
    match pattern {
        // Sergels torg: black and white triangles, two to a square, the squares alternating.
        // Rows of triangles, black pointing up and white pointing down, each row half a
        // triangle along from the last.
        "plattan" => png(64, 64, |x, y| {
            let row = y / 16;
            let fy = (y % 16) as f32 / 16.0;
            let u = x as f32 / 16.0 + if row % 2 == 1 { 0.5 } else { 0.0 };
            let f = u - u.floor();
            let dark = (f - 0.5).abs() * 2.0 < fy;
            shade(if dark { [38, 38, 42] } else { [222, 220, 214] }, grain(x, y))
        }),
        // Cast concrete in slabs with saw-cut joints.
        "slabs" => png(64, 64, |x, y| {
            let joint = x % 32 == 0 || y % 32 == 0;
            shade(if joint { [118, 116, 112] } else { [176, 172, 164] }, grain(x, y) * 2)
        }),
        // Asphalt: dark, with a coarse grain.
        _ => png(64, 64, |x, y| shade([84, 84, 88], grain(x, y) * 3)),
    }
}

/// An RGB PNG of w x h, stored (uncompressed) deflate.
fn png(w: usize, h: usize, at: impl Fn(usize, usize) -> [u8; 3]) -> Vec<u8> {
    let mut raw = Vec::new();
    for y in 0..h {
        raw.push(0u8);
        for x in 0..w {
            raw.extend_from_slice(&at(x, y));
        }
    }
    assert!(raw.len() < 65_536, "one stored block");
    let mut z = vec![0x78, 0x01, 0x01];
    z.extend_from_slice(&(raw.len() as u16).to_le_bytes());
    z.extend_from_slice(&(!(raw.len() as u16)).to_le_bytes());
    z.extend_from_slice(&raw);
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut png = vec![0x89, b'P', b'N', b'G', 13, 10, 26, 10];
    let mut chunk = |kind: &[u8], data: &[u8]| {
        png.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        png.extend_from_slice(&body);
        png.extend_from_slice(&crc32(&body).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    png
}

#[derive(Default)]
struct Geo {
    pos: Vec<[f32; 3]>,
    nrm: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
}

impl Geo {
    fn tri(&mut self, a: [f32; 3], b: [f32; 3], c: [f32; 3], uv: [f32; 2]) {
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let mut n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if l < 1e-9 {
            return;
        }
        n = [n[0] / l, n[1] / l, n[2] / l];
        for p in [a, b, c] {
            self.pos.push(p);
            self.nrm.push(n);
            self.uv.push(uv);
        }
    }
    fn quad(&mut self, a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3], uv: [f32; 2]) {
        self.tri(a, b, c, uv);
        self.tri(a, c, d, uv);
    }
    /// A square tube of half-width `r` from a to b (a rail, a ledge's edge).
    fn tube(&mut self, a: [f32; 3], b: [f32; 3], r: f32, uv: [f32; 2]) {
        let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if l < 1e-4 {
            return;
        }
        let f = [d[0] / l, d[1] / l, d[2] / l];
        let up = if f[1].abs() > 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
        let mut s = [f[1] * up[2] - f[2] * up[1], f[2] * up[0] - f[0] * up[2], f[0] * up[1] - f[1] * up[0]];
        let sl = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
        s = [s[0] / sl * r, s[1] / sl * r, s[2] / sl * r];
        let mut t = [f[1] * s[2] - f[2] * s[1], f[2] * s[0] - f[0] * s[2], f[0] * s[1] - f[1] * s[0]];
        let tl = (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt();
        t = [t[0] / tl * r, t[1] / tl * r, t[2] / tl * r];
        let corner = |p: [f32; 3], i: usize| {
            let (ks, kt) = [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)][i];
            [p[0] + s[0] * ks + t[0] * kt, p[1] + s[1] * ks + t[1] * kt, p[2] + s[2] * ks + t[2] * kt]
        };
        for i in 0..4 {
            let j = (i + 1) % 4;
            self.quad(corner(a, i), corner(b, i), corner(b, j), corner(a, j), uv);
        }
    }
}

/// Whether a triangle is the spot's floor: level, at the floor's height, ridden smooth.
fn is_floor(a: [f32; 3], b: [f32; 3], c: [f32; 3], floor: f32) -> bool {
    [a, b, c].iter().all(|p| (p[1] - floor).abs() < 0.02)
}

/// The spot's floor (its own mesh, tiled with `pattern`) and everything else (the palette).
fn spot_glbs(f: &SpotFile, floor_y: f32) -> (Vec<u8>, Vec<u8>) {
    let mut g = Geo::default();
    let mut floor = Geo::default();
    for m in &f.meshes {
        let surface = serde_json::to_value(m.surface).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_else(|| "smooth".into());
        let uv = texel(&surface);
        for t in &m.triangles {
            let p = |i: u32| m.vertices[i as usize];
            if surface == "smooth" && is_floor(p(t[0]), p(t[1]), p(t[2]), floor_y) {
                floor.tri(p(t[0]), p(t[1]), p(t[2]), [0.0, 0.0]);
            } else {
                g.tri(p(t[0]), p(t[1]), p(t[2]), uv);
            }
        }
    }
    for r in &f.rails {
        let mut pts = r.points.clone();
        if r.closed && !pts.is_empty() {
            pts.push(pts[0]);
        }
        let (radius, uv) = match (r.kind, r.material) {
            (RailKind::Coping, _) => (0.035, texel("coping")),
            (_, RailMaterial::Metal) => (0.025, texel("metal")),
            (RailKind::ThinRail | RailKind::FatRail, _) => (0.025, texel("edge")),
            _ => (0.012, texel("edge")),
        };
        for w in pts.windows(2) {
            g.tube(w[0], w[1], radius, uv);
        }
    }
    let pattern = match f.name.as_str() {
        "plattan" => "plattan",
        "ralis" => "asphalt",
        _ => "slabs",
    };
    (glb(&g, palette_png(), &format!("spot_{}", f.name), None), glb(&floor, floor_png(pattern), &format!("spot_{}_floor", f.name), Some(FLOOR_TILE)))
}

/// One mesh with one texture; `tile`, its UVs from where it is (the lens's
/// `uv_from_position`, metres a repeat).
fn glb(g: &Geo, png: Vec<u8>, name: &str, tile: Option<f32>) -> Vec<u8> {
    // u32 indices: a bowl is more than a u16 holds. The lens reads them as u16 per
    // primitive, so the triangles are split into primitives of at most 65 535 vertices.
    let n = g.pos.len();
    let mut bin: Vec<u8> = Vec::new();
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    let mut prims = Vec::new();
    let mut push = |bin: &mut Vec<u8>, bytes: &[u8], target: Option<u32>| -> usize {
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let mut v = serde_json::json!({"buffer": 0, "byteOffset": bin.len(), "byteLength": bytes.len()});
        if let Some(t) = target {
            v["target"] = t.into();
        }
        bin.extend_from_slice(bytes);
        views.push(v);
        views.len() - 1
    };
    let img_view = push(&mut bin, &png, None);
    let chunk = 65_535 / 3 * 3;
    let mut start = 0;
    while start < n {
        let end = (start + chunk).min(n);
        let count = end - start;
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        let mut pb = Vec::with_capacity(count * 12);
        for p in &g.pos[start..end] {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
                pb.extend_from_slice(&p[k].to_le_bytes());
            }
        }
        let nb: Vec<u8> = g.nrm[start..end].iter().flat_map(|p| p.iter().flat_map(|x| x.to_le_bytes())).collect();
        let ub: Vec<u8> = g.uv[start..end].iter().flat_map(|p| p.iter().flat_map(|x| x.to_le_bytes())).collect();
        let ib: Vec<u8> = (0..count as u16).flat_map(|i| i.to_le_bytes()).collect();
        let pv = push(&mut bin, &pb, Some(34962));
        let nv = push(&mut bin, &nb, Some(34962));
        let uvv = push(&mut bin, &ub, Some(34962));
        let iv = push(&mut bin, &ib, Some(34963));
        let a0 = accessors.len();
        accessors.push(serde_json::json!({"bufferView": pv, "componentType": 5126, "count": count, "type": "VEC3", "min": lo, "max": hi}));
        accessors.push(serde_json::json!({"bufferView": nv, "componentType": 5126, "count": count, "type": "VEC3"}));
        accessors.push(serde_json::json!({"bufferView": uvv, "componentType": 5126, "count": count, "type": "VEC2"}));
        accessors.push(serde_json::json!({"bufferView": iv, "componentType": 5123, "count": count, "type": "SCALAR"}));
        let mut prim = serde_json::json!({"attributes": {"POSITION": a0, "NORMAL": a0 + 1, "TEXCOORD_0": a0 + 2}, "indices": a0 + 3, "material": 0});
        if let Some(t) = tile {
            prim["attributes"].as_object_mut().unwrap().remove("TEXCOORD_0");
            prim["extras"] = serde_json::json!({"uv_from_position": t});
        }
        prims.push(prim);
        start = end;
    }
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    let doc = serde_json::json!({
        "asset": {"version": "2.0", "generator": "beargarden/sk8 spots"},
        "scene": 0, "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0, "name": name}],
        "meshes": [{"primitives": prims}],
        "materials": [{"pbrMetallicRoughness": {"baseColorTexture": {"index": 0}, "metallicFactor": 0.0}}],
        "samplers": [{"magFilter": 9728, "minFilter": 9728, "wrapS": 10497, "wrapT": 10497}],
        "textures": [{"source": 0, "sampler": 0}],
        "images": [{"bufferView": img_view, "mimeType": "image/png"}],
        "buffers": [{"byteLength": bin.len()}],
        "bufferViews": views,
        "accessors": accessors,
    });
    let mut js = serde_json::to_vec(&doc).unwrap();
    while js.len() % 4 != 0 {
        js.push(b' ');
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&((12 + 8 + js.len() + 8 + bin.len()) as u32).to_le_bytes());
    out.extend_from_slice(&(js.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&js);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&bin);
    out
}

/// The walkable ground as the walkmask's decks: every surface but an unrideable one that
/// faces up more than it faces sideways, one deck a triangle, its plane through its corners,
/// and none buried under another. Off the board a body walks on these, so stairs, ledges,
/// the upper level and bowls hold it where sk8's surfaces are. With a flat floor at 0, or the
/// plaza's ground left in under the stairs, a body put down off the board stood in the stairs
/// or inside the upper level (iPhone simulator, 2026-11-07).
fn decks(f: &sk8::world::SpotFile) -> Vec<serde_json::Value> {
    // (corners in x, z; height at the first; slope in x and z)
    let mut tris: Vec<([[f32; 2]; 3], f32, [f32; 2])> = Vec::new();
    for m in f.meshes.iter().filter(|m| m.surface != sk8::world::Surface::Unrideable) {
        for t in &m.triangles {
            let [a, b, c] = t.map(|i| m.vertices[i as usize]);
            let (u, v) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
            let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len < 1e-6 || (n[1] / len).abs() < 0.3 {
                continue;
            }
            tris.push(([[a[0], a[2]], [b[0], b[2]], [c[0], c[2]]], a[1], [-n[0] / n[1], -n[2] / n[1]]));
        }
    }
    let height = |t: &([[f32; 2]; 3], f32, [f32; 2]), x: f32, z: f32| t.1 + t.2[0] * (x - t.0[0][0]) + t.2[1] * (z - t.0[0][1]);
    let inside = |t: &[[f32; 2]; 3], x: f32, z: f32| {
        let s = |p: [f32; 2], q: [f32; 2]| (q[0] - p[0]) * (z - p[1]) - (q[1] - p[1]) * (x - p[0]);
        let (d1, d2, d3) = (s(t[0], t[1]), s(t[1], t[2]), s(t[2], t[0]));
        !((d1 < -1e-4 || d2 < -1e-4 || d3 < -1e-4) && (d1 > 1e-4 || d2 > 1e-4 || d3 > 1e-4))
    };
    let bbox = |t: &[[f32; 2]; 3]| {
        let xs = t.map(|p| p[0]);
        let zs = t.map(|p| p[1]);
        [xs.iter().cloned().fold(f32::MAX, f32::min), zs.iter().cloned().fold(f32::MAX, f32::min), xs.iter().cloned().fold(f32::MIN, f32::max), zs.iter().cloned().fold(f32::MIN, f32::max)]
    };
    let boxes: Vec<[f32; 4]> = tris.iter().map(|t| bbox(&t.0)).collect();
    // A triangle is buried when any of its corners (pulled a little in) or its middle has
    // another surface over it by more than a step.
    let buried = |i: usize| {
        let t = &tris[i];
        let (cx, cz) = ((t.0[0][0] + t.0[1][0] + t.0[2][0]) / 3.0, (t.0[0][1] + t.0[1][1] + t.0[2][1]) / 3.0);
        let mut probes = vec![(cx, cz)];
        for p in t.0 {
            probes.push((p[0] + (cx - p[0]) * 0.1, p[1] + (cz - p[1]) * 0.1));
        }
        probes.iter().all(|&(x, z)| {
            let mine = height(t, x, z);
            tris.iter().enumerate().any(|(j, o)| {
                let b = boxes[j];
                j != i && x >= b[0] && x <= b[2] && z >= b[1] && z <= b[3] && inside(&o.0, x, z) && height(o, x, z) > mine + 0.3
            })
        })
    };
    let r = |x: f32| (x * 1000.0).round() / 1000.0;
    let mut kept: Vec<usize> = (0..tris.len()).filter(|&i| !buried(i)).collect();
    // Highest first: a body put down with no level of its own takes the first deck under it.
    kept.sort_by(|&a, &b| {
        let top = |i: usize| tris[i].0.iter().map(|p| height(&tris[i], p[0], p[1])).fold(f32::MIN, f32::max);
        top(b).total_cmp(&top(a))
    });
    kept.into_iter()
        .map(|i| {
            let t = &tris[i];
            serde_json::json!({
                "corners": [[r(t.0[0][0]), r(t.0[0][1])], [r(t.0[1][0]), r(t.0[1][1])], [r(t.0[2][0]), r(t.0[2][1])]],
                "y": r(t.1),
                "slope": [r(t.2[0]), r(t.2[1])],
            })
        })
        .collect()
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "../world/charts".into());
    let meshes = std::env::args().nth(2).unwrap_or_else(|| "../conditioned/beargarden".into());
    std::fs::create_dir_all(&meshes).unwrap();
    for f in sk8::spots::all() {
        let skate = format!("{}/{}.skate.json", dir, f.name);
        std::fs::write(&skate, serde_json::to_string(&f).unwrap()).unwrap();
        let world = sk8::World::from_spot(&f);
        let (things, floor) = spot_glbs(&f, world.floor_y());
        let mesh = format!("{}/spot_{}.glb", meshes, f.name);
        std::fs::write(&mesh, &things).unwrap();
        std::fs::write(format!("{}/spot_{}_floor.glb", meshes, f.name), &floor).unwrap();
        let glb = things;
        let b = bounds(&f);
        let chart = serde_json::json!({
            "_comment": format!("Written by beargarden/sk8 (cargo run --bin spots): {}, a skate spot. The lens draws spot_{}.glb (the same surfaces, coloured by kind) and rides {}.skate.json with sk8 (GF-REQ-LENS-1). Do not edit.", f.title, f.name, f.name),
            "assets": "conditioned/beargarden",
            "atmosphere": { "color": [0.72, 0.8, 0.88, 1.0], "start": 80.0, "end": 320.0 },
            "instances": [
                { "mesh": format!("spot_{}_floor", f.name), "pos": [0.0, 0.0, 0.0], "rot_y": 0.0, "scale": 1.0 },
                { "mesh": format!("spot_{}", f.name), "pos": [0.0, 0.0, 0.0], "rot_y": 0.0, "scale": 1.0 }
            ],
            "skate": format!("{}.skate.json", f.name),
            "walkmask": {
                "extent": [b[0] + 0.5, b[1] + 0.5, b[2] - 0.5, b[3] - 0.5],
                "floor_y": 0.0,
                "eye_height": 1.65,
                "body_radius": 0.35,
                "blocked": [],
                "decks": decks(&f)
            }
        });
        let path = format!("{}/{}.json", dir, f.name);
        std::fs::write(&path, serde_json::to_string_pretty(&chart).unwrap() + "\n").unwrap();
        let tris: usize = f.meshes.iter().map(|m| m.triangles.len()).sum();
        println!("{}: {} triangles, {} rails, {} marks -> {} ({} bytes)", f.name, tris, f.rails.len(), f.marks.len(), mesh, glb.len());
    }
}
