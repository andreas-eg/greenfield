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
