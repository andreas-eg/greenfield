//! Beargarden's spots are closed: a skater pushing flat out in any direction from anywhere
//! on a spot meets a wall, a building or grass, never the edge of the world ("off the
//! spot"). Found in a player test in the lens (2026-11-07): Plattan's upper terrace ended
//! in nothing.

use sk8::input::btn;
use sk8::math::*;
use sk8::sim::Event;
use sk8::{Difficulty, Sim, VirtualController, World};

#[test]
fn no_spot_has_an_edge_to_ride_off() {
    for spot in sk8::spots::all().into_iter().chain(sk8::spots::surveyed()) {
        let world = World::from_spot(&spot);
        let mut starts = vec![world.spawn];
        for m in &spot.marks {
            starts.push(v3(m.pos[0], m.pos[1], m.pos[2]));
        }
        for at in starts {
            for k in 0..16 {
                let yaw = k as f32 / 16.0 * std::f32::consts::TAU;
                let mut s = Sim::new(World::from_spot(&spot), Difficulty::Normal);
                s.place(at, v3(yaw.sin(), 0.0, yaw.cos()));
                let mut off = None;
                for t in 0..60 * 25 {
                    // A push every half second, held for a tenth.
                    let push = t % 30 < 6;
                    let c = VirtualController::default().with(btn::A, push);
                    s.step(&[c]);
                    for e in s.take_events() {
                        if let Event::Bail { reason } = e {
                            if reason == "off the spot" {
                                off = Some(s.p.pos);
                            }
                        }
                    }
                    if off.is_some() {
                        break;
                    }
                }
                assert!(off.is_none(), "{}: off the spot from {:?} heading {:.0} deg, at {:?}", spot.name, at, yaw.to_degrees(), off.unwrap());
            }
        }
    }
}

/// Plattan's first line, as a player rides it from the start: push, ollie onto the east
/// bench, 50-50, and back on the ground (player test 2026-11-07: after leaning off, the
/// board hung over the bench in the air and never landed).
#[test]
fn plattan_first_line_lands_off_the_bench() {
    let spot = sk8::spots::all().into_iter().find(|s| s.name == "plattan").unwrap();
    let mut s = Sim::new(World::from_spot(&spot), Difficulty::Normal);
    s.t.apply_text(&std::fs::read_to_string("../world/skate/tuning.txt").unwrap());
    let w = s.world.spawn;
    s.place(w, v3(0.0, 0.0, -1.0));
    let mut ev = Vec::new();
    let step = |s: &mut Sim, n: usize, c: VirtualController, ev: &mut Vec<Event>| {
        for _ in 0..n {
            s.step(&[c]);
            ev.extend(s.take_events());
        }
    };
    let pad = VirtualController::default();
    for _ in 0..3 {
        step(&mut s, 12, pad.with(btn::A, true), &mut ev);
        step(&mut s, 18, pad, &mut ev);
    }
    while s.p.pos.z > 8.3 {
        step(&mut s, 1, pad, &mut ev);
    }
    step(&mut s, 15, VirtualController { ry: -1.0, ..pad }, &mut ev);
    step(&mut s, 5, VirtualController { ry: 1.0, ..pad }, &mut ev);
    let mut ended = None;
    let mut landed = None;
    for t in 0..600 {
        step(&mut s, 1, pad, &mut ev);
        if ended.is_none() && ev.iter().any(|e| matches!(e, Event::GrindEnd { .. })) {
            ended = Some(t);
        }
        if ended.is_some() && landed.is_none() && ev.iter().any(|e| matches!(e, Event::Land { .. })) {
            landed = Some(t);
        }
    }
    // Off the bench and down within half a second, as a hop off a 0.42 m ledge is.
    let (e, l) = (ended.expect("the grind ends"), landed.expect("a landing"));
    assert!(l - e < 30, "{} ticks from the grind's end to the ground", l - e);
    let said: Vec<String> = ev.iter().filter(|e| !matches!(e, Event::Push)).map(|e| format!("{e:?}")).collect();
    assert!(said.iter().any(|e| e.contains("GrindStart")), "{said:?}");
    assert_eq!(s.p.state, sk8::sim::State::Ground, "at {:?} vel {:?}: {said:?}", s.p.pos, s.p.vel);
}

/// Björns trädgård's big bowl as a player first meets it: roll off the start into it at a
/// walking push's speed, across the floor and up the far wall, and back, never bailing.
#[test]
fn bjorns_big_bowl_drops_in_and_rides_across() {
    let spot = sk8::spots::all().into_iter().find(|s| s.name == "bjorns").unwrap();
    let mut s = Sim::new(World::from_spot(&spot), Difficulty::Normal);
    s.t.apply_text_styled(&std::fs::read_to_string("../world/skate/tuning.txt").unwrap(), "arcade");
    s.place(s.world.spawn, v3(0.0, 0.0, -1.0));
    s.p.grace = 0.0;
    s.p.vel = v3(0.0, 0.0, -4.0);
    s.p.speed_target = 4.0;
    let (mut low, mut north, mut bails) = (0.0f32, 0.0f32, Vec::new());
    for _ in 0..60 * 4 {
        s.step(&[VirtualController::default()]);
        for e in s.take_events() {
            if let Event::Bail { reason } = e {
                bails.push(reason);
            }
        }
        low = low.min(s.p.pos.y);
        north = north.min(s.p.pos.z);
    }
    assert!(bails.is_empty(), "bailed: {bails:?}");
    assert!(low < -1.2, "down to {low} m, the floor is 1.35 m down");
    assert!(north < -4.0, "got as far north as z = {north}");
}

/// The city spike (tools/city_spot.py, SODER-LENS-S272): the real park's bowls as GeoGen made
/// them, ridden as the hand-built Björns spot is above: from the park's start, rolling north at
/// 4 m/s into the big bowl. Records how deep it goes and how far, and what bails.
#[test]
fn medborgarplatsen_bowls_from_geogen_drop_in_and_ride_across() {
    let Some(spot) = sk8::spots::surveyed().into_iter().find(|s| s.name == "medborgarplatsen") else {
        return eprintln!("no surveyed Medborgarplatsen (sk8/spots/): not ridden");
    };
    let mut s = Sim::new(World::from_spot(&spot), Difficulty::Normal);
    s.t.apply_text_styled(&std::fs::read_to_string("../world/skate/tuning.txt").unwrap(), "arcade");
    s.place(s.world.spawn, v3(0.0, 0.0, -1.0));
    s.p.grace = 0.0;
    s.p.vel = v3(0.0, 0.0, -4.0);
    s.p.speed_target = 4.0;
    let start = s.p.pos;
    let (mut low, mut north, mut bails) = (start.y, start.z, Vec::new());
    for _ in 0..60 * 4 {
        s.step(&[VirtualController::default()]);
        for e in s.take_events() {
            if let Event::Bail { reason } = e {
                bails.push(reason);
            }
        }
        low = low.min(s.p.pos.y);
        north = north.min(s.p.pos.z);
    }
    println!("medborgarplatsen bowls: down {:.2} m, {:.1} m north, bails {bails:?}", start.y - low, start.z - north);
    assert!(bails.is_empty(), "bailed: {bails:?}");
    assert!(start.y - low > 1.0, "down only {:.2} m", start.y - low);
    assert!(start.z - north > 4.0, "only {:.1} m north", start.z - north);
}

/// Rides `spot` from `from`, heading `yaw` (0 = +z) at a cruising 6 m/s, up a bank whose deck
/// is `deck_y` up, and turns on the deck to ride along it toward `along`.
/// The deck holds it: it ends rolling on the deck at the deck's height, turned along it,
/// having pushed on along it.
/// @req SODER-LENS-S292
fn up_the_bank_and_along_the_deck(name: &str, from: Vec3, yaw: f32, deck_y: f32, along: Vec3) {
    let spot = sk8::spots::all().into_iter().find(|s| s.name == name).unwrap();
    let mut s = Sim::new(World::from_spot(&spot), Difficulty::Normal);
    s.t.apply_text(&std::fs::read_to_string("../world/skate/tuning.txt").unwrap());
    let f = v3(yaw.sin(), 0.0, yaw.cos());
    s.place(from, f);
    s.p.grace = 0.0;
    s.p.vel = f * 6.0;
    s.p.speed_target = 6.0;
    s.step(&[VirtualController::default()]);
    // The board rides this far over the ground it stands on.
    let ride = s.p.pos.y - from.y;
    let deck_y = deck_y + ride;
    let mut bails = Vec::new();
    let mut crest = None;
    let mut low_on_deck = f32::MAX;
    for t in 0..60 * 8 {
        if t % 20 == 0 && std::env::var("SK8_TRACE").is_ok() {
            eprintln!("{t} pos {:?} speed {:.2} fwd {:?} {:?}", s.p.pos, s.p.speed(), s.p.fwd, s.p.state);
        }
        // Once up, steer along the deck until the board points along it, then push on.
        let mut c = VirtualController::default();
        if crest.is_some() {
            let fwd = s.p.fwd.planar().norm();
            if fwd.dot(along) < deg(8.0).cos() {
                let right = v3(-fwd.z, 0.0, fwd.x);
                c.lx = if right.dot(along) > 0.0 { 1.0 } else { -1.0 };
            } else {
                c = c.with(btn::A, t % 30 < 6);
            }
            low_on_deck = low_on_deck.min(s.p.pos.y);
        }
        s.step(&[c]);
        for e in s.take_events() {
            if let Event::Bail { reason } = e {
                bails.push(reason);
            }
        }
        if crest.is_none() && s.p.pos.y > deck_y - 0.05 {
            crest = Some(s.p.pos);
        }
    }
    let crest = crest.unwrap_or_else(|| panic!("{name}: never got up the bank, at {:?}", s.p.pos));
    let end = s.p.pos;
    let gone = (end - crest).dot(along);
    assert!(bails.is_empty(), "{name}: bailed {bails:?}");
    assert_eq!(s.p.state, sk8::sim::State::Ground, "{name}: at {end:?}");
    assert!((end.y - deck_y).abs() < 0.05, "{name}: ended at y = {} off the {deck_y} m deck, at {end:?}", end.y);
    assert!(low_on_deck > deck_y - 0.3, "{name}: dropped to {low_on_deck} after cresting");
    assert!(s.p.fwd.planar().norm().dot(along) > deg(20.0).cos(), "{name}: not turned along the deck, fwd {:?}", s.p.fwd);
    assert!(gone > 5.0, "{name}: only {gone:.1} m along the deck from {crest:?} to {end:?}");
}

/// Plattan's north bank: from between the granite benches up 1.2 m onto the street-level
/// deck, a left turn and east along it.
#[test]
fn plattan_bank_up_to_the_north_deck_and_along_it() {
    up_the_bank_and_along_the_deck("plattan", v3(6.0, 0.0, 12.0), 0.0, 1.2, v3(1.0, 0.0, 0.0));
}

/// Övningsplanen's west bank: across the plaza north of the bowl, up onto the 5 m deck, and
/// north along it.
#[test]
fn ovningsplanen_bank_up_to_the_west_deck_and_along_it() {
    up_the_bank_and_along_the_deck("ovningsplanen", v3(-32.0, 0.0, 20.0), -std::f32::consts::FRAC_PI_2, 1.2, v3(0.0, 0.0, 1.0));
}

/// Rålis's south bank: past the mini ramp's west side, up onto the 5 m deck, and west along it.
#[test]
fn ralis_bank_up_to_the_south_deck_and_along_it() {
    up_the_bank_and_along_the_deck("ralis", v3(-4.0, 0.0, -9.0), std::f32::consts::PI, 1.2, v3(-1.0, 0.0, 0.0));
}

/// Flat out on the scooter across Plattan, up the east bank and over the deck: the rider stops
/// at the parapet on the deck's outer edge, short of the buildings' faces (x 46) where no body
/// may stand. Without it the scooter ran into the building edge at x 45.75 and the lens put
/// the rider back on their feet (SODER-BUG-657).
/// @bug SODER-BUG-657
#[test]
fn plattan_east_deck_stops_a_full_throttle_scooter_at_its_parapet() {
    let spot = sk8::spots::all().into_iter().find(|s| s.name == "plattan").unwrap();
    let mut s = Sim::new(World::from_spot(&spot), Difficulty::Normal);
    s.set_tuning(Difficulty::Normal, &std::fs::read_to_string("../world/skate/tuning.txt").unwrap(), "arcade");
    s.set_vehicle(sk8::Vehicle::Scooter);
    s.place(v3(16.0, 0.0, 0.0), v3(1.0, 0.0, 0.0));
    s.p.grace = 0.0;
    s.set_stick_push(1.0);
    let mut furthest = f32::MIN;
    let mut top = 0.0f32;
    for _ in 0..60 * 10 {
        s.step(&[VirtualController::default()]);
        s.take_events();
        furthest = furthest.max(s.p.pos.x);
        top = top.max(s.p.speed());
    }
    assert!(top > 9.0, "never got up to speed: {top:.1} m/s");
    assert!(furthest > 40.0, "never got onto the east deck: furthest x {furthest:.2}");
    assert!(furthest < 45.6, "past the parapet into the buildings: x {furthest:.2}");
}
