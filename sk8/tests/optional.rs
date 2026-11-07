//! The spec's optional systems (D.1): kerb ride-over and wall rides (B.6.6), the handplant
//! (B.11 point 8), and the Hall of Meat wipeout (B.13.1).

use sk8::build::Builder;
use sk8::input::btn;
use sk8::math::*;
use sk8::sim::{Event, State};
use sk8::world::{Surface, World};
use sk8::{Difficulty, Sim, VirtualController};

fn pad() -> VirtualController {
    VirtualController::default()
}

fn start(s: &mut Sim, pos: Vec3, yaw: f32, speed: f32) {
    let f = v3(yaw.sin(), 0.0, yaw.cos());
    s.place(pos, f);
    s.p.grace = 0.0;
    s.p.vel = f * speed;
    s.p.speed_target = speed;
}

fn run(s: &mut Sim, ticks: usize, c: VirtualController) -> Vec<Event> {
    let mut ev = Vec::new();
    for _ in 0..ticks {
        s.step(&[c]);
        ev.extend(s.take_events());
    }
    ev
}

fn bailed(ev: &[Event]) -> Option<String> {
    ev.iter().find_map(|e| if let Event::Bail { reason } = e { Some(reason.clone()) } else { None })
}

#[test]
fn a_low_kerb_is_ridden_over() {
    let mut b = Builder::new();
    b.ground(Surface::Smooth, -10.0, -10.0, 10.0, 30.0, 0.0);
    // A 6 cm kerb across the way, then the raised side.
    b.block(Surface::Smooth, 0.0, 15.0, 0.0, 20.0, 20.0, 0.06, 0.0);
    let mut s = Sim::new(World::from_spot(&b.finish("kerb", "kerb", ZERO, 0.0)), Difficulty::Normal);
    start(&mut s, v3(0.0, 0.0, 0.0), 0.0, 3.0);
    let ev = run(&mut s, 150, pad());
    assert_eq!(bailed(&ev), None, "{ev:?}");
    // Over the kerb's face (z = 5) and still rolling: coasting with no input slows a board
    // from 3 m/s by about 0.7 m/s a second (B.6.1), the kerb takes nothing.
    assert!(s.p.pos.z > 5.3 && s.p.speed() > 1.0, "stopped at the kerb: {:?} at {} m/s", s.p.pos, s.p.speed());
    assert!((s.p.pos.y - 0.135).abs() < 0.03, "not up on it: {:?}", s.p.pos);
    assert_eq!(s.p.state, State::Ground);
}

#[test]
fn a_wall_met_at_a_shallow_angle_is_ridden() {
    let mut b = Builder::new();
    b.ground(Surface::Smooth, -10.0, -10.0, 10.0, 40.0, 0.0);
    // A wall 3 m high along z, its face at x = 1.
    b.block(Surface::Smooth, 1.5, 15.0, 0.0, 1.0, 30.0, 3.0, 0.0);
    let mut s = Sim::new(World::from_spot(&b.finish("wall", "wall", ZERO, 0.0)), Difficulty::Normal);
    start(&mut s, v3(0.0, 0.0, 2.0), 0.0, 6.0);
    // Popped off the ground beside the wall, drifting into it.
    s.p.pos = v3(0.65, 1.0, 2.0);
    s.p.vel = v3(1.2, 2.0, 6.0);
    s.p.air_time = 0.3;
    s.p.set_state(State::Air);
    let mut rode = 0;
    let mut ev = Vec::new();
    for _ in 0..120 {
        s.step(&[pad()]);
        ev.extend(s.take_events());
        if s.p.state == State::WallRide {
            rode += 1;
        }
    }
    assert!(ev.iter().any(|e| matches!(e, Event::Trick { name } if name == "Wallride")), "{ev:?}");
    assert!(rode > 10 && rode <= 49, "rode the wall for {rode} ticks");
    assert_eq!(bailed(&ev), None, "{ev:?}");
    assert_ne!(s.p.state, State::WallRide);
}

#[test]
fn rb_up_the_vert_is_a_handplant() {
    let mut b = Builder::new();
    b.ground(Surface::Smooth, -10.0, -30.0, 10.0, 10.0, 0.0);
    b.quarter_pipe(Surface::Smooth, 0.0, 6.0, 0.0, 8.0, 2.4, 0.3, 2.0, 0.0);
    let mut s = Sim::new(World::from_spot(&b.finish("qp", "qp", ZERO, 0.0)), Difficulty::Normal);
    start(&mut s, v3(0.0, 0.0, 0.0), 0.0, 7.5);
    let rb = pad().with(btn::RB, true);
    let mut planted = 0;
    let mut ev = Vec::new();
    for _ in 0..300 {
        s.step(&[rb]);
        ev.extend(s.take_events());
        if s.p.state == State::Handplant {
            planted += 1;
        }
    }
    assert!(ev.iter().any(|e| matches!(e, Event::Trick { name } if name == "Handplant")), "{ev:?}");
    assert!((50..=56).contains(&planted), "planted for {planted} ticks (0.9 s is 54)");
    assert_eq!(bailed(&ev), None, "{ev:?}");
    assert!(s.p.pos.y < 0.3 && s.p.state == State::Ground, "not back down: {:?} {:?}", s.p.state, s.p.pos);
}

#[test]
fn hall_of_meat_poses_and_push_off() {
    let mut b = Builder::new();
    b.ground(Surface::Smooth, -50.0, -50.0, 50.0, 50.0, 0.0);
    let mut s = Sim::new(World::from_spot(&b.finish("flat", "flat", ZERO, 0.0)), Difficulty::Normal);
    start(&mut s, ZERO, 0.0, 6.0);
    let chord = VirtualController { lt: 1.0, rt: 1.0, ..pad() }.with(btn::L3, true);
    run(&mut s, 2, chord);
    run(&mut s, 1, chord.with(btn::R3, true));
    assert_eq!(s.p.state, State::Wipeout);
    // Right stick pulled down: a cannonball.
    run(&mut s, 5, VirtualController { ry: -1.0, ..pad() });
    assert_eq!(s.p.wipe.unwrap().pose, 1);
    assert_eq!(sk8::frame::clip_name(sk8::Frame::of(&s).clip), "bail_cannonball");
    let spin0 = s.p.wipe.unwrap().spin;
    run(&mut s, 30, pad());
    assert!(s.p.wipe.unwrap().spin - spin0 > 2.0, "a cannonball tumbles");
    // Lying on the ground, a full trigger pushes off it.
    run(&mut s, 30, pad());
    let vy0 = s.p.wipe.unwrap().body_vel.y;
    run(&mut s, 1, VirtualController { rt: 1.0, ..pad() });
    assert!(s.p.wipe.unwrap().body_vel.y > vy0 + 2.0, "no push off: {:?}", s.p.wipe.unwrap().body_vel);
}
