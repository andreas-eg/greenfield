//! Beargarden's spots. The test plaza is the spec's Phase 0 level (D.2): every feature the
//! acceptance tests ride. The Stockholm spots are schematic layouts of real places with
//! real-world dimensions where they are known; their survey-accurate geometry is a GeoGen
//! request (beargarden/feature_requests.md, GF-REQ-GEOGEN-1).

use crate::build::Builder;
use crate::math::*;
use crate::world::{RailKind, RailMaterial, SpotFile, Surface};

/// Övningsplanen: the test plaza on the green field. A bank, a 0.45 m ledge, a flat rail,
/// a quarter-pipe with coping, a 2.4 m bowl, a manual pad, a curb, a stair set with a
/// handrail, a long 10 degree hill for the coasting test, and banks up to a deck all round.
pub fn ovningsplanen() -> SpotFile {
    let mut b = Builder::new();
    let s = Surface::Smooth;
    // The bowl sits in the west; leave its square out of the ground.
    let hole = b.bowl(s, -24.0, -12.0, 0.0, 1.5, 2.4, 0.3, 5.5);
    b.ground_around(s, (-40.0, -40.0, 40.0, 40.0), hole, 0.0);
    // Bank: rises north to 1 m over 6 m (9.5 deg), then a 4 m deck.
    b.bank(s, 0.0, 18.0, 0.0, 12.0, 6.0, 1.0, 4.0, 0.0);
    // Ledge 0.45 m high, 6 m long along z, 0.4 m wide.
    b.ledge(s, RailMaterial::Concrete, -8.0, 8.0, 0.0, 0.4, 6.0, 0.45, 0.0);
    // Flat rail, round, 0.35 m high.
    b.flat_rail(v3(-3.0, 0.35, 5.0), v3(-3.0, 0.35, 11.0), true);
    // Manual pad and a curb.
    b.ledge(s, RailMaterial::Concrete, 7.0, 8.0, 0.0, 2.0, 4.0, 0.15, 0.0);
    b.ledge(s, RailMaterial::Concrete, 12.0, 8.0, 0.0, 0.3, 6.0, 0.12, 0.0);
    // Quarter-pipe in the south, rising south: 2.4 m radius, 0.3 m vert, coping, 2 m deck.
    b.quarter_pipe(s, 0.0, -12.0, 0.0, 8.0, 2.4, 0.3, 2.0, std::f32::consts::PI);
    // Stairs: a 1.2 m terrace in the east with five steps down to the west and a handrail.
    b.block(s, 24.0, -6.0, 0.0, 8.0, 8.0, 1.2, 0.0);
    b.stairs(20.0, -6.0, 1.2, 4.0, 6, 0.2, 0.4, -std::f32::consts::FRAC_PI_2, true);
    // The hill: 10 deg, 60 m long, falling north along x = 32.
    let h = 60.0 * deg(10.0).tan();
    b.quad(s, v3(28.0, h, -38.0), v3(36.0, h, -38.0), v3(36.0, 0.0, 22.0), v3(28.0, 0.0, 22.0), UP);
    // Banks up to a deck all round the 80 m concrete, then the green field, so riding off the
    // concrete is grass, not the edge.
    platforms(&mut b, s, 40.0, 40.0);
    b.mark("start", v3(0.0, 0.0, 0.0), 0.0);
    b.mark("hill_top", v3(32.0, h, -36.0), 0.0);
    b.mark("bowl_deck", v3(-24.0, 0.0, -17.2), 0.0);
    b.mark("ledge_line", v3(-7.6, 0.0, -2.0), 0.0);
    b.finish("ovningsplanen", "Övningsplanen", v3(0.0, 0.0, 0.0), 0.0)
}

/// Plattan, Sergels torg: the sunken plaza with its triangle paving, the stairs up to the
/// upper level with their handrails, and the granite benches along the edges. Stockholm's
/// street spot since the eighties. The street level round the sunken plaza is a deck 1.2 m
/// up, banks running up to it on the west, north and east, joined to the upper level.
pub fn plattan() -> SpotFile {
    let mut b = Builder::new();
    let s = Surface::Smooth;
    // The plaza floor (the black-and-white paving), 64 by 40 m.
    b.ground(s, -32.0, -22.0, 32.0, 20.0, 0.0);
    // The street level round it to the buildings, 1.2 m up: banks rise to it from the plaza's
    // west, north and east edges over 7 m (9.7 deg), then a 7 m deck to the buildings, so a
    // rider can roll up at cruising speed, turn on the deck, ride along it and drop back in.
    // The banks meet in mitred corners and the deck runs round both north corners.
    // @req SODER-LENS-S292
    b.bank_ring(s, (-32.0, -22.0, 32.0, 20.0), 0.0, 7.0, 1.2, 7.0, [true, true, false, true]);
    // South of the side decks, beside the upper level, the deck carries on to the south
    // buildings, and a 6 m bank (11 deg) climbs the last 1.2 m onto the upper level: the
    // platforms go all the way round, the upper level their south side.
    b.look(Some("deck"));
    for sx in [-1.0f32, 1.0] {
        b.block(s, sx * 39.0, -26.0, 0.0, 14.0, 8.0, 1.2, 0.0);
        let (lo, hi) = (sx * 38.0, sx * 32.0);
        b.quad(s, v3(lo, 1.2, -30.0), v3(lo, 1.2, -22.0), v3(hi, 2.4, -22.0), v3(hi, 2.4, -30.0), UP);
        b.tri(s, v3(lo, 1.2, -22.0), v3(hi, 2.4, -22.0), v3(hi, 1.2, -22.0), v3(0.0, 0.0, 1.0));
    }
    b.look(None);
    // A 0.9 m parapet along the decks' outer edge, as Rålis and Övningsplanen have: at full
    // speed a rider stops at it, not in the buildings' faces where no body may stand
    // (SODER-BUG-657).
    // @bug SODER-BUG-657
    let (t, ph) = (0.3, 1.2 + 0.9);
    for (x, z, w, d) in [(-46.0 + t * 0.5, 2.0, t, 64.0), (46.0 - t * 0.5, 2.0, t, 64.0), (0.0, 34.0 - t * 0.5, 92.0, t), (-39.0, -30.0 + t * 0.5, 14.0, t), (39.0, -30.0 + t * 0.5, 14.0, t)] {
        b.block(Surface::Unrideable, x, z, 0.0, w, d, ph, 0.0);
    }
    // The buildings that close the square: Drottninggatan's corner to the north, the
    // blocks to the east and west.
    for (x, z, w, d) in [(0.0, 38.0, 92.0, 8.0), (-50.0, 0.0, 8.0, 84.0), (50.0, 0.0, 8.0, 84.0), (0.0, -34.0, 108.0, 8.0)] {
        b.block(Surface::Unrideable, x, z, 0.0, w, d, 14.0, 0.0);
    }
    // The upper level along the south, 2.4 m up, reached by a wide flight of 12 steps.
    b.block(s, 0.0, -26.0, 0.0, 64.0, 8.0, 2.4, 0.0);
    for x in [-14.0f32, 0.0, 14.0] {
        b.stairs(x, -22.0, 2.4, 10.0, 12, 0.2, 0.35, 0.0, true);
    }
    // Granite benches: low ledges along the plaza's north edge and round the middle.
    for i in 0..5 {
        let x = -24.0 + i as f32 * 12.0;
        b.ledge(s, RailMaterial::Stone, x, 16.0, 0.0, 5.0, 0.6, 0.48, 0.0);
    }
    b.ledge(s, RailMaterial::Stone, -8.0, 4.0, 0.0, 0.6, 6.0, 0.42, 0.0);
    b.ledge(s, RailMaterial::Stone, 8.0, 4.0, 0.0, 0.6, 6.0, 0.42, 0.0);
    // Kulturhuset's side: a long kerb-high step the length of the plaza.
    b.ledge(s, RailMaterial::Stone, -30.0, 0.0, 0.0, 1.2, 36.0, 0.18, 0.0);
    // The glass obelisk's base on the upper level, not rideable.
    b.block(Surface::Unrideable, 0.0, -27.0, 2.4, 6.0, 6.0, 1.0, 0.0);
    // The start looks down the east bench's line, 0.4 m off it: push twice and ollie for a
    // first 50-50, rather than facing the open plaza with every ledge behind (player test
    // 2026-11-07).
    b.mark("start", v3(7.6, 0.0, 13.0), std::f32::consts::PI);
    b.mark("stairs_top", v3(14.0, 2.4, -24.0), 0.0);
    b.finish("plattan", "Plattan, Sergels torg", v3(7.6, 0.0, 13.0), std::f32::consts::PI)
}

/// Rålis, the concrete park in Rålambshovsparken: a bowl, a mini ramp, banks, a hubba, a
/// pyramid with a rail, ledges and flat rails, banks up to a deck all round the concrete, and
/// the park's grass round it.
pub fn ralis() -> SpotFile {
    let mut b = Builder::new();
    let c = Surface::Smooth;
    let hole = b.bowl(c, -14.0, 0.0, 0.0, 2.5, 2.2, 0.4, 6.5);
    b.ground_around(c, (-22.0, -16.0, 22.0, 16.0), hole, 0.0);
    // Banks up to a deck round the 44 by 32 m concrete, then the park's grass.
    platforms(&mut b, c, 22.0, 16.0);
    // Mini ramp: two 1.6 m quarters facing each other over 5 m of flat.
    b.quarter_pipe(c, 4.0, -9.5, 0.0, 6.0, 1.6, 0.0, 1.5, std::f32::consts::PI);
    b.quarter_pipe(c, 4.0, -4.5, 0.0, 6.0, 1.6, 0.0, 1.5, 0.0);
    // A pyramid: four banks round a 3 m top, 0.8 m high, with a rail down its west face.
    let (px, pz) = (12.0, 8.0);
    b.block(c, px, pz, 0.0, 3.0, 3.0, 0.8, 0.0);
    b.bank(c, px, pz + 4.0, 0.0, 3.0, 2.5, 0.8, 0.0, std::f32::consts::PI);
    b.bank(c, px, pz - 4.0, 0.0, 3.0, 2.5, 0.8, 0.0, 0.0);
    b.bank(c, px + 4.0, pz, 0.0, 3.0, 2.5, 0.8, 0.0, -std::f32::consts::FRAC_PI_2);
    b.bank(c, px - 4.0, pz, 0.0, 3.0, 2.5, 0.8, 0.0, std::f32::consts::FRAC_PI_2);
    b.flat_rail(v3(px - 1.5, 1.1, pz + 1.2), v3(px - 4.0, 0.3, pz + 1.2), true);
    // A hubba: a ledge running down beside a three-step set.
    b.block(c, 16.0, -8.0, 0.0, 6.0, 4.0, 0.6, 0.0);
    b.stairs(16.0, -6.0, 0.6, 3.0, 3, 0.2, 0.4, 0.0, false);
    b.rail(RailKind::Ledge, RailMaterial::Concrete, &[v3(18.0, 0.95, -6.0), v3(18.0, 0.35, -4.8)], false);
    // Ledges and a flat bar along the north.
    b.ledge(c, RailMaterial::Concrete, -2.0, 10.0, 0.0, 6.0, 0.5, 0.4, 0.0);
    b.ledge(c, RailMaterial::Concrete, 4.0, 13.0, 0.0, 6.0, 0.5, 0.55, 0.0);
    b.flat_rail(v3(-4.0, 0.3, 5.0), v3(2.0, 0.3, 5.0), false);
    b.mark("start", v3(0.0, 0.0, 0.0), 0.0);
    b.mark("bowl_deck", v3(-14.0, 0.0, -5.5), 0.0);
    b.mark("mini_ramp", v3(4.0, 1.6, -12.5), 0.0);
    b.finish("ralis", "Rålis, Rålambshovsparken", v3(0.0, 0.0, 0.0), 0.0)
}

/// Fryshuset: the indoor hall at Hammarby, a vert ramp 10 m wide with 3 m transitions and
/// 0.8 m of vert over 4 m of flat, a mini ramp beside it, wooden surfaces, walls round it.
pub fn fryshuset() -> SpotFile {
    let mut b = Builder::new();
    let s = Surface::Smooth;
    b.ground(s, -20.0, -24.0, 20.0, 24.0, 0.0);
    // Vert: two quarters facing each other along z.
    b.quarter_pipe(s, -6.0, -2.0, 0.0, 10.0, 3.0, 0.8, 2.5, std::f32::consts::PI);
    b.quarter_pipe(s, -6.0, 2.0, 0.0, 10.0, 3.0, 0.8, 2.5, 0.0);
    // Mini ramp.
    b.quarter_pipe(s, 9.0, -2.5, 0.0, 6.0, 1.4, 0.0, 1.5, std::f32::consts::PI);
    b.quarter_pipe(s, 9.0, 2.5, 0.0, 6.0, 1.4, 0.0, 1.5, 0.0);
    // A box and a rail on the floor at the far end.
    b.ledge(s, RailMaterial::Wood, 0.0, 16.0, 0.0, 1.2, 4.0, 0.4, 0.0);
    b.flat_rail(v3(6.0, 0.35, 14.0), v3(6.0, 0.35, 20.0), true);
    // The hall's walls.
    for (x, z, w, d) in [(0.0, -24.5, 40.0, 1.0), (0.0, 24.5, 40.0, 1.0), (-20.5, 0.0, 1.0, 50.0), (20.5, 0.0, 1.0, 50.0)] {
        b.block(Surface::Unrideable, x, z, 0.0, w, d, 6.0, 0.0);
    }
    // The start is the vert's flat, facing the north wall: push, and it is up the wall.
    b.mark("start", v3(-6.0, 0.0, 0.0), 0.0);
    b.mark("vert_deck", v3(-6.0, 3.8, -6.5), 0.0);
    b.mark("floor", v3(0.0, 0.0, 12.0), std::f32::consts::PI);
    b.finish("fryshuset", "Fryshuset", v3(-6.0, 0.0, 0.0), 0.0)
}

/// Björns trädgård: the three concrete bowls in the park on Södermalm (STHLM Royalty's park
/// model, asset BT_Skate_Basins_01 v005): a big lobed bowl 1.35 m deep and two ovals, 1.10
/// and 1.25 m, sunk into one deck, apart with no hips between. Rims traced from the 2023
/// orthophoto (±1 m), depths estimated from photos (±0.4 m), each wall a quarter circle of its
/// depth's radius, vertical at the lip, as the model has them. The model's metres east and
/// north are x and -z here, its origin the deck's middle, the deck at y = 0.
pub fn bjorns() -> SpotFile {
    let mut b = Builder::new();
    let s = Surface::Smooth;
    let here = |pts: &[(f32, f32)]| -> Vec<(f32, f32)> { pts.iter().map(|&(e, n)| (e, -n)).collect() };
    let large = here(&[(-10.66, 3.61), (-10.20, 5.21), (-8.26, 6.47), (-6.09, 7.84), (-3.35, 8.86), (-0.95, 8.17), (0.42, 6.68), (1.10, 4.63), (0.64, 3.60), (1.90, 2.68), (3.15, 1.19), (3.26, -0.41), (1.89, -2.24), (-0.17, -3.49), (-2.45, -3.38), (-4.27, -2.23), (-4.61, -0.63), (-6.67, -1.31), (-8.50, -1.20), (-9.98, -0.17), (-10.78, 1.67)]);
    let north_east = here(&[(2.59, 9.08), (2.70, 10.57), (4.19, 11.71), (6.70, 12.17), (9.10, 11.82), (10.47, 10.56), (10.92, 8.85), (10.01, 7.13), (8.06, 5.76), (5.89, 5.19), (3.95, 5.77), (3.04, 7.03)]);
    let south_east = here(&[(4.29, 1.31), (4.98, 3.02), (6.92, 4.05), (9.55, 4.39), (11.72, 3.58), (12.97, 1.87), (13.31, -0.53), (12.39, -2.93), (10.45, -4.30), (7.82, -4.53), (5.77, -3.27), (4.63, -1.32)]);
    let deck = here(&[(-12.13, -10.30), (-16.38, 8.06), (12.37, 14.89), (16.66, -3.35), (4.44, -6.50), (-0.51, -7.31)]);
    let mut rims = Vec::new();
    for (ctrl, depth) in [(&large, 1.35), (&north_east, 1.10), (&south_east, 1.25)] {
        let rim = crate::poly::catmull_rom_closed(ctrl, 8);
        b.freeform_bowl(s, &rim, 0.0, depth, 10);
        rims.push(rim);
    }
    b.polygon(s, &deck, &rims, 0.0);
    // The park round the deck: grass to a fence 30 m out each way.
    let out = 30.0;
    b.polygon(Surface::Slow, &[(-out, -out), (out, -out), (out, out), (-out, out)], &[deck.clone()], 0.0);
    let (w, h, l) = (0.3, 1.4, 2.0 * out);
    for (x, z, bw, bd) in [(0.0, -out, l, w), (0.0, out, l, w), (-out, 0.0, w, l), (out, 0.0, w, l)] {
        b.block(Surface::Unrideable, x, z, 0.0, bw, bd, h, 0.0);
    }
    // The start is the deck south of the big bowl, facing north into it: push, and drop in.
    let start = v3(-1.5, 0.0, 5.4);
    b.mark("start", start, std::f32::consts::PI);
    b.mark("ovals", v3(14.8, 0.0, 0.0), -std::f32::consts::FRAC_PI_2);
    b.finish("bjorns", "Björns trädgård", start, std::f32::consts::PI)
}

/// Banks up to a deck all round the concrete from -hw..hw by -hd..hd: 1.2 m up over 7 m
/// (9.7 deg, a roll up at cruising speed), a deck 5 m deep to turn on and ride along, a
/// 0.9 m parapet on its outer edge, and the grass and fence beyond (`grass_round`).
/// @req SODER-LENS-S292
fn platforms(b: &mut Builder, s: Surface, hw: f32, hd: f32) {
    let (x0, z0, x1, z1) = b.bank_ring(s, (-hw, -hd, hw, hd), 0.0, 7.0, 1.2, 5.0, [true; 4]);
    let (t, h) = (0.3, 1.2 + 0.9);
    let (w, d) = (x1 - x0 + 2.0 * t, z1 - z0 + 2.0 * t);
    for (x, z, bw, bd) in [(0.0, z1 + t * 0.5, w, t), (0.0, z0 - t * 0.5, w, t), (x0 - t * 0.5, 0.0, t, d), (x1 + t * 0.5, 0.0, t, d)] {
        b.block(Surface::Unrideable, x, z, 0.0, bw, bd, h, 0.0);
    }
    grass_round(b, x1 + t, z1 + t, 70.0);
}

/// Grass from a hw x hd rectangle round the middle out to `out` metres each way, and a
/// park's fence round it: a spot has no edge to ride off (tests/spots.rs).
fn grass_round(b: &mut Builder, hw: f32, hd: f32, out: f32) {
    let g = Surface::Slow;
    b.ground(g, -out, -out, out, -hd, 0.0);
    b.ground(g, -out, hd, out, out, 0.0);
    b.ground(g, -out, -hd, -hw, hd, 0.0);
    b.ground(g, hw, -hd, out, hd, 0.0);
    let (w, h, l) = (0.3, 1.4, 2.0 * out);
    for (x, z, bw, bd) in [(0.0, -out, l, w), (0.0, out, l, w), (-out, 0.0, w, l), (out, 0.0, w, l)] {
        b.block(Surface::Unrideable, x, z, 0.0, bw, bd, h, 0.0);
    }
}

pub fn all() -> Vec<SpotFile> {
    vec![ovningsplanen(), plattan(), ralis(), fryshuset(), bjorns()]
}

/// Spots made from surveyed ground rather than built here: the SpotFiles in `sk8/spots/`
/// (tools/city_spot.py writes Medborgarplatsen and Björns trädgård from GeoGen's island, the
/// city spike). Read at run time, so a missing or unreadable file is left out and said.
#[cfg(not(target_arch = "wasm32"))]
pub fn surveyed() -> Vec<SpotFile> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("spots");
    let mut files: Vec<_> = std::fs::read_dir(&dir).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect()).unwrap_or_default();
    files.sort();
    files
        .iter()
        .filter_map(|p| match std::fs::read_to_string(p).map_err(|e| e.to_string()).and_then(|t| serde_json::from_str(&t).map_err(|e| e.to_string())) {
            Ok(f) => Some(f),
            Err(why) => {
                eprintln!("{}: {why}", p.display());
                None
            }
        })
        .collect()
}
