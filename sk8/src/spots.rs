//! Greenfield's spots. The test plaza is the spec's Phase 0 level (D.2): every feature the
//! acceptance tests ride. The Stockholm spots are schematic layouts of real places with
//! real-world dimensions where they are known; their survey-accurate geometry is a GeoGen
//! request (greenfield/feature_requests.md, GF-REQ-GEOGEN-1).

use crate::build::Builder;
use crate::math::*;
use crate::world::{RailKind, RailMaterial, SpotFile, Surface};

/// Övningsplanen: the test plaza on the green field. A bank, a 0.45 m ledge, a flat rail,
/// a quarter-pipe with coping, a 2.4 m bowl, a manual pad, a curb, a stair set with a
/// handrail, and a long 10 degree hill for the coasting test.
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
    // The green field round the plaza, so riding off the concrete is grass, not the edge.
    grass_round(&mut b, 40.0, 40.0, 70.0);
    b.mark("start", v3(0.0, 0.0, 0.0), 0.0);
    b.mark("hill_top", v3(32.0, h, -36.0), 0.0);
    b.mark("bowl_deck", v3(-24.0, 0.0, -17.2), 0.0);
    b.mark("ledge_line", v3(-7.6, 0.0, -2.0), 0.0);
    b.finish("ovningsplanen", "Övningsplanen", v3(0.0, 0.0, 0.0), 0.0)
}

/// Plattan, Sergels torg: the sunken plaza with its triangle paving, the stairs up to the
/// upper level with their handrails, and the granite benches along the edges. Stockholm's
/// street spot since the eighties.
pub fn plattan() -> SpotFile {
    let mut b = Builder::new();
    let s = Surface::Smooth;
    // The plaza floor (the black-and-white paving), 64 by 40 m, and the street level
    // round it to the buildings.
    b.ground(s, -32.0, -22.0, 32.0, 20.0, 0.0);
    b.ground(Surface::Rough, -46.0, 20.0, 46.0, 34.0, 0.0);
    b.ground(Surface::Rough, -46.0, -30.0, -32.0, 20.0, 0.0);
    b.ground(Surface::Rough, 32.0, -30.0, 46.0, 20.0, 0.0);
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
/// pyramid with a rail, ledges and flat rails, with the park's grass round it.
pub fn ralis() -> SpotFile {
    let mut b = Builder::new();
    let c = Surface::Smooth;
    let hole = b.bowl(c, -14.0, 0.0, 0.0, 2.5, 2.2, 0.4, 6.5);
    b.ground_around(c, (-22.0, -16.0, 22.0, 16.0), hole, 0.0);
    // Grass round the concrete, the park's.
    grass_round(&mut b, 22.0, 16.0, 70.0);
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
    vec![ovningsplanen(), plattan(), ralis(), fryshuset()]
}
