//! Greenfield's spots are closed: a skater pushing flat out in any direction from anywhere
//! on a spot meets a wall, a building or grass, never the edge of the world ("off the
//! spot"). Found in a player test in the lens (2026-11-07): Plattan's upper terrace ended
//! in nothing.

use sk8::input::btn;
use sk8::math::*;
use sk8::sim::Event;
use sk8::{Difficulty, Sim, VirtualController, World};

#[test]
fn no_spot_has_an_edge_to_ride_off() {
    for spot in sk8::spots::all() {
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
