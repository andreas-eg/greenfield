//! The spec's acceptance checks (C.3), ridden headless with scripted controllers on small
//! test worlds. Where a check is "by playing" in the spec, the number here is what the ride
//! must show; where the spec's own numbers disagree with each other, the comment says so.

use sk8::build::Builder;
use sk8::input::btn;
use sk8::math::*;
use sk8::sim::{Event, Landing, State, DT};
use sk8::world::{RailMaterial, Surface, World};
use sk8::{Difficulty, Sim, VirtualController};

fn pad() -> VirtualController {
    VirtualController::default()
}
fn ls(x: f32, y: f32) -> VirtualController {
    VirtualController { lx: x, ly: y, ..pad() }
}
fn rs(x: f32, y: f32) -> VirtualController {
    VirtualController { rx: x, ry: y, ..pad() }
}

fn flat_world() -> World {
    let mut b = Builder::new();
    b.ground(Surface::Smooth, -100.0, -100.0, 100.0, 100.0, 0.0);
    World::from_spot(&b.finish("flat", "flat", ZERO, 0.0))
}

fn sim(w: World) -> Sim {
    Sim::new(w, Difficulty::Normal)
}

/// Put the board at `pos` heading `yaw` (radians, 0 = +z) rolling at `speed`.
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

/// A quick ollie flick: pull down, then snap up in two samples.
fn ollie(s: &mut Sim) -> Vec<Event> {
    let mut ev = run(s, 3, rs(0.0, -1.0));
    ev.extend(run(s, 1, rs(0.0, 0.0)));
    ev.extend(run(s, 2, rs(0.0, 1.0)));
    ev.extend(run(s, 1, pad()));
    ev
}

fn landed(ev: &[Event]) -> Option<Landing> {
    ev.iter().find_map(|e| if let Event::Land { quality, .. } = e { Some(*quality) } else { None })
}
fn bailed(ev: &[Event]) -> Option<String> {
    ev.iter().find_map(|e| if let Event::Bail { reason } = e { Some(reason.clone()) } else { None })
}

// ---- step 1: rolling ------------------------------------------------------------------

#[test]
fn c3_1_coast_down_10_degree_slope_settles_near_9() {
    let mut b = Builder::new();
    let h = 500.0 * deg(10.0).tan();
    b.quad(Surface::Smooth, v3(-5.0, h, 0.0), v3(5.0, h, 0.0), v3(5.0, 0.0, 500.0), v3(-5.0, 0.0, 500.0), UP);
    let mut s = sim(World::from_spot(&b.finish("hill", "hill", v3(0.0, h, 2.0), 0.0)));
    start(&mut s, v3(0.0, h, 2.0), 0.0, 0.0);
    run(&mut s, 60 * 40, pad());
    let v = s.p.speed();
    assert_eq!(s.p.state, State::Ground);
    assert!((8.0..10.5).contains(&v), "terminal speed {v}");
}

#[test]
fn c3_1_carve_radius_about_4m_at_6() {
    let mut s = sim(flat_world());
    start(&mut s, ZERO, 0.0, 6.0);
    run(&mut s, 30, ls(1.0, 0.0));
    let v = s.p.speed();
    let r = v / s.p.yaw_rate.abs();
    assert!((3.0..5.5).contains(&r), "radius {r} at {v} m/s");
    // Turning right: the board yaws clockwise seen from above.
    assert!(s.p.fwd.x < 0.0, "turned the wrong way: fwd {:?}", s.p.fwd);
}

#[test]
fn c3_1_up_a_bank_and_back() {
    let mut b = Builder::new();
    b.ground(Surface::Smooth, -10.0, -10.0, 10.0, 10.0, 0.0);
    b.bank(Surface::Smooth, 0.0, 10.0, 0.0, 8.0, 8.0, 2.0, 2.0, 0.0);
    let mut s = sim(World::from_spot(&b.finish("bank", "bank", ZERO, 0.0)));
    start(&mut s, v3(0.0, 0.0, 4.0), 0.0, 4.5);
    let mut top: f32 = 0.0;
    let mut air = false;
    for _ in 0..330 {
        s.step(&[pad()]);
        top = top.max(s.p.pos.y);
        air |= matches!(s.p.state, State::Air | State::KnownAir);
    }
    assert!(!air, "left the bank");
    assert!(top > 0.5, "got up {top}");
    assert!(s.p.vel.z < -0.5 && s.p.pos.z < 10.0, "rolled back: vel {:?} pos {:?}", s.p.vel, s.p.pos);
    assert!(s.p.fakie, "rolling back down is fakie");
}

fn bowl_world() -> World {
    let mut b = Builder::new();
    let hole = b.bowl(Surface::Smooth, 0.0, 0.0, 0.0, 1.5, 2.4, 0.3, 5.0);
    b.ground_around(Surface::Smooth, (-20.0, -20.0, 20.0, 20.0), hole, 0.0);
    World::from_spot(&b.finish("bowl", "bowl", v3(0.0, -2.7, 0.0), 0.0))
}

#[test]
fn c3_1_bowl_wall_to_vertical_and_back() {
    let mut s = sim(bowl_world());
    start(&mut s, v3(0.0, -2.7, -1.0), 0.0, 5.6);
    let mut steepest: f32 = 1.0;
    let mut left = false;
    let mut back_down = false;
    for _ in 0..150 {
        s.step(&[pad()]);
        steepest = steepest.min(s.p.ground_n.y);
        left |= matches!(s.p.state, State::Air | State::KnownAir);
        back_down |= steepest < 0.3 && s.p.pos.y < -2.5;
    }
    assert!(steepest < 0.3, "never got near vertical: n.y {steepest}");
    assert!(!left, "a false natural air on a smooth bowl wall");
    assert!(back_down, "never came back down to the floor");
}

// ---- step 2: push, brake, powerslide, kick turn -----------------------------------------

#[test]
fn c3_2_pushes_to_top_speed() {
    let mut s = sim(flat_world());
    start(&mut s, ZERO, 0.0, 0.0);
    let mut pushes = 0;
    for _ in 0..60 * 8 {
        s.step(&[pad().with(btn::A, true)]);
        pushes += s.take_events().iter().filter(|e| matches!(e, Event::Push)).count();
        if s.p.speed() >= 7.5 {
            break;
        }
    }
    // The spec says four pushes; its own push numbers (2.0 at rest falling to 1.0 at 8.5,
    // code test fixture) add up to at most 6.7 m/s in four, so it takes six or seven.
    assert!(s.p.speed() >= 7.5, "only {} m/s", s.p.speed());
    assert!(pushes <= 7, "{pushes} pushes");
}

#[test]
fn c3_2_brake_from_8_in_about_1_5_s() {
    let mut s = sim(flat_world());
    start(&mut s, ZERO, 0.0, 8.0);
    let mut t = 0.0;
    while s.p.speed() > 0.01 && t < 4.0 {
        s.step(&[pad().with(btn::B, true)]);
        t += DT;
    }
    assert!((1.2..2.0).contains(&t), "stopped in {t} s");
}

#[test]
fn c3_2_powerslide_90_from_8_ends_near_4() {
    let mut s = sim(flat_world());
    start(&mut s, ZERO, 0.0, 8.0);
    run(&mut s, 5, pad());
    // Left stick hard down and to the right: a slide to the right.
    let c = ls(0.6, -0.8);
    let mut slid = false;
    for _ in 0..60 {
        s.step(&[c]);
        slid |= s.p.state == State::Powerslide;
    }
    run(&mut s, 20, pad());
    assert!(slid, "never slid");
    let v = s.p.speed();
    assert!((2.5..5.5).contains(&v), "after the slide {v} m/s");
}

#[test]
fn c3_2_kick_turn_at_rest() {
    let mut s = sim(flat_world());
    start(&mut s, ZERO, 0.0, 0.0);
    let f0 = s.p.fwd;
    run(&mut s, 31, ls(1.0, 0.0));
    let a = angle_between(f0, s.p.fwd).to_degrees();
    assert!(a > 150.0, "kick turn turned {a} deg");
}

// ---- step 3: the ollie ------------------------------------------------------------------

/// The height above take-off and the pop's strength.
fn ollie_height(path: &[VirtualController]) -> (f32, f32) {
    let mut s = sim(flat_world());
    start(&mut s, ZERO, 0.0, 6.0);
    run(&mut s, 10, pad());
    let y0 = s.p.pos.y;
    let mut top = y0;
    let mut strength = -1.0;
    for (i, c) in path.iter().enumerate() {
        // Keep the speed the same however long the crouch is held.
        if i + 4 < path.len() {
            s.p.vel = s.p.fwd * 6.0;
            s.p.speed_target = 6.0;
        }
        s.step(&[*c]);
        top = top.max(s.p.pos.y);
        for e in s.take_events() {
            if let Event::Pop { strength: st, .. } = e {
                strength = st;
            }
        }
    }
    for _ in 0..90 {
        s.step(&[pad()]);
        top = top.max(s.p.pos.y);
    }
    (top - y0, strength)
}

#[test]
fn c3_3_flick_speed_is_height() {
    let quick = ollie_height(&[rs(0.0, -1.0), rs(0.0, -1.0), rs(0.0, 0.0), rs(0.0, 1.0), rs(0.0, 1.0)]);
    let mut held: Vec<VirtualController> = vec![rs(0.0, -1.0); 120];
    held.extend_from_slice(&[rs(0.0, 0.0), rs(0.0, 1.0), rs(0.0, 1.0)]);
    let held = ollie_height(&held);
    let lazy: Vec<VirtualController> = (0..=8).map(|i| rs(0.0, -1.0 + 1.5 * i as f32 / 8.0)).chain([rs(0.0, 1.0)]).collect();
    let lazy = ollie_height(&lazy);
    assert!((0.85..1.05).contains(&quick.0), "full snap {quick:?}");
    assert_eq!(held.1, quick.1, "a held crouch changed the strength");
    assert!((held.0 - quick.0).abs() < 0.02, "a held crouch changed the height: {held:?} vs {quick:?}");
    assert!(lazy.0 < 0.3, "lazy flick {lazy:?}");
}

// ---- step 4: air, prediction, landing ---------------------------------------------------

#[test]
fn c3_4_off_a_1m_bank_lands_clean() {
    let mut b = Builder::new();
    b.ground(Surface::Smooth, -10.0, -10.0, 10.0, 40.0, 0.0);
    b.bank(Surface::Smooth, 0.0, 6.0, 0.0, 6.0, 6.0, 1.0, 3.0, 0.0);
    let mut s = sim(World::from_spot(&b.finish("bank", "bank", ZERO, 0.0)));
    start(&mut s, v3(0.0, 0.0, 2.0), 0.0, 7.0);
    let ev = run(&mut s, 300, pad());
    assert_eq!(bailed(&ev), None);
    let q = landed(&ev);
    assert!(matches!(q, Some(Landing::Clean) | Some(Landing::Adjust)), "{q:?}");
}

#[test]
fn c3_4_quarter_pipe_air_keeps_speed() {
    let mut b = Builder::new();
    b.ground(Surface::Smooth, -10.0, -30.0, 10.0, 10.0, 0.0);
    b.quarter_pipe(Surface::Smooth, 0.0, 6.0, 0.0, 8.0, 2.4, 0.3, 2.0, 0.0);
    let mut s = sim(World::from_spot(&b.finish("qp", "qp", ZERO, 0.0)));
    // 7 m/s climbs only about 2.5 m (v^2 / 2g): it takes the top pushing speed to air out.
    start(&mut s, v3(0.0, 0.0, 0.0), 0.0, 8.5);
    let mut flew = false;
    let mut ev = Vec::new();
    for _ in 0..240 {
        s.step(&[pad()]);
        ev.extend(s.take_events());
        flew |= matches!(s.p.state, State::Air | State::KnownAir) && s.p.pos.y > 2.8;
        if flew && s.p.state == State::Ground && s.p.pos.y < 0.2 {
            break;
        }
    }
    assert!(flew, "never aired above the lip");
    assert_eq!(bailed(&ev), None, "{ev:?}");
    let v = s.p.speed();
    assert!(v > 0.75 * 8.5, "came back in at {v} m/s");
}

fn land_at(angle_deg: f32, d: Difficulty) -> Vec<Event> {
    let mut s = Sim::new(flat_world(), d);
    start(&mut s, ZERO, 0.0, 5.0);
    run(&mut s, 5, pad());
    // Just above the ground, falling, the board turned off the velocity: too late for auto
    // heading (which straightens a crooked air on easy and normal) to fix it.
    s.p.pos = v3(0.0, 0.2, 0.0);
    s.p.vel = v3(0.0, -1.0, 5.0);
    let f = v3(deg(angle_deg).sin(), 0.0, deg(angle_deg).cos());
    s.p.fwd = f;
    s.p.heading = f;
    s.p.air_time = 0.5;
    s.p.set_state(State::Air);
    run(&mut s, 60, pad())
}

#[test]
fn c3_4_sideways_is_sketchy_and_50_degrees_bails() {
    let ev = land_at(25.0, Difficulty::Hardcore);
    assert_eq!(bailed(&ev), None, "{ev:?}");
    assert_eq!(landed(&ev), Some(Landing::Sketchy));
    let ev = land_at(50.0, Difficulty::Normal);
    assert_eq!(bailed(&ev).as_deref(), Some("landing angle"));
}

// ---- step 5: flips and spins -------------------------------------------------------------

#[test]
fn c3_5_kickflip_from_its_shape() {
    let mut s = sim(flat_world());
    start(&mut s, ZERO, 0.0, 5.0);
    run(&mut s, 5, pad());
    let mut ev = run(&mut s, 2, rs(0.0, -0.95));
    ev.extend(run(&mut s, 1, rs(-0.55, -0.75)));
    ev.extend(run(&mut s, 1, rs(-0.9, 0.45)));
    ev.extend(run(&mut s, 100, pad()));
    assert_eq!(bailed(&ev), None, "{ev:?}");
    let named = ev.iter().any(|e| matches!(e, Event::Trick { name } if name.contains("Kickflip")));
    assert!(named, "{ev:?}");
}

#[test]
fn c3_5_left_stick_180_lands_fakie() {
    let mut s = sim(flat_world());
    start(&mut s, ZERO, 0.0, 5.0);
    run(&mut s, 5, pad());
    let mut ev = ollie(&mut s);
    // Spin right for a moment, then let auto heading finish it.
    ev.extend(run(&mut s, 18, ls(1.0, 0.0)));
    ev.extend(run(&mut s, 90, pad()));
    assert_eq!(bailed(&ev), None, "{ev:?}");
    assert!(s.p.fakie, "not fakie after a 180");
    let named = ev.iter().any(|e| matches!(e, Event::Trick { name } if name.contains("180")));
    assert!(named, "{ev:?}");
}

// ---- step 6: rails ----------------------------------------------------------------------

fn ledge_world(len: f32) -> World {
    let mut b = Builder::new();
    b.ground(Surface::Smooth, -20.0, -20.0, 20.0, 40.0, 0.0);
    // Along z from 0 to len, centred on x = 0.2 so its west edge runs along x = 0.
    b.ledge(Surface::Smooth, RailMaterial::Concrete, 0.2, len * 0.5, 0.0, 0.4, len, 0.45, 0.0);
    World::from_spot(&b.finish("ledge", "ledge", ZERO, 0.0))
}

fn grind_names(ev: &[Event]) -> Vec<String> {
    ev.iter().filter_map(|e| if let Event::GrindStart { name } = e { Some(name.clone()) } else { None }).collect()
}

#[test]
fn c3_6_ollie_from_0_4m_off_line_locks_a_5050() {
    let mut s = sim(ledge_world(8.0));
    // Riding along the ledge 0.4 m off its west edge.
    start(&mut s, v3(-0.4, 0.0, -3.0), 0.0, 4.5);
    run(&mut s, 10, pad());
    let mut ev = ollie(&mut s);
    ev.extend(run(&mut s, 60, pad()));
    let names = grind_names(&ev);
    assert!(names.iter().any(|n| n.ends_with("50-50")), "grinds {names:?}, events {ev:?}");
}

#[test]
fn c3_6_across_the_ledge_is_a_boardslide() {
    let mut s = sim(ledge_world(8.0));
    // Heading east across the ledge.
    start(&mut s, v3(-2.6, 0.0, 4.0), std::f32::consts::FRAC_PI_2, 3.5);
    run(&mut s, 4, pad());
    let mut ev = ollie(&mut s);
    ev.extend(run(&mut s, 60, pad()));
    let names = grind_names(&ev);
    assert!(names.iter().any(|n| n.ends_with("Boardslide") || n.ends_with("Lipslide")), "grinds {names:?}, events {ev:?}");
}

fn into_5050(len: f32, speed: f32) -> Sim {
    let mut s = sim(ledge_world(len));
    start(&mut s, v3(-0.3, 0.0, -3.0), 0.0, speed);
    run(&mut s, 10, pad());
    ollie(&mut s);
    for _ in 0..70 {
        s.step(&[pad()]);
        if s.p.state == State::Grind {
            break;
        }
    }
    assert_eq!(s.p.state, State::Grind, "not grinding");
    s
}

#[test]
fn c3_6_lean_past_0_9_falls_off() {
    let mut s = into_5050(10.0, 4.5);
    let ev = run(&mut s, 10, ls(1.0, 0.0));
    assert!(ev.iter().any(|e| matches!(e, Event::GrindEnd { .. })), "{ev:?}");
    assert_ne!(s.p.state, State::Grind);
}

#[test]
fn c3_6_long_ledge_leans_you_off_at_about_4_5_s() {
    let mut s = into_5050(12.0, 4.0);
    let t0 = s.p.grind.as_ref().unwrap().t;
    let mut t = t0;
    for _ in 0..60 * 7 {
        s.step(&[pad()]);
        if s.p.state != State::Grind {
            break;
        }
        t = s.p.grind.as_ref().unwrap().t;
    }
    assert_ne!(s.p.state, State::Grind, "still grinding after 7 s");
    assert!((4.0..5.0).contains(&t), "leaned off after {t} s");
}

#[test]
fn c3_6_grind_pop_clears_0_8m() {
    let mut s = into_5050(10.0, 5.0);
    run(&mut s, 10, pad());
    let rail_y = 0.45;
    let mut ev = ollie(&mut s);
    let mut top: f32 = 0.0;
    for _ in 0..60 {
        s.step(&[pad()]);
        ev.extend(s.take_events());
        top = top.max(s.p.pos.y - 0.075 - rail_y);
    }
    assert!(top >= 0.8, "cleared {top} m: {ev:?}");
}

// ---- step 7: pumping ---------------------------------------------------------------------

fn mini_ramp_speed(pump: bool) -> f32 {
    let mut b = Builder::new();
    b.ground(Surface::Smooth, -10.0, -20.0, 10.0, 20.0, 0.0);
    b.quarter_pipe(Surface::Smooth, 0.0, 3.0, 0.0, 6.0, 2.0, 0.0, 1.5, 0.0);
    b.quarter_pipe(Surface::Smooth, 0.0, -3.0, 0.0, 6.0, 2.0, 0.0, 1.5, std::f32::consts::PI);
    let mut s = sim(World::from_spot(&b.finish("mini", "mini", ZERO, 0.0)));
    start(&mut s, ZERO, 0.0, 4.0);
    let mut best: f32 = 0.0;
    for i in 0..60 * 8 {
        // Crouch on the flat, stand up as the wall comes.
        let on_wall = s.p.ground_n.y < 0.97;
        let c = if pump && !on_wall { VirtualController { lt: 1.0, ..pad() } } else { pad() };
        s.step(&[c]);
        if s.p.pos.y < 0.1 && i > 60 * 6 {
            best = best.max(s.p.speed());
        }
    }
    best
}

#[test]
fn c3_7_pumping_gains_speed() {
    let with = mini_ramp_speed(true);
    let without = mini_ramp_speed(false);
    assert!(with > without + 1.0, "pumped {with} m/s, coasting {without} m/s");
}

// ---- step 8: bails and respawn -----------------------------------------------------------

#[test]
fn c3_8_wall_at_speed_is_a_run_out_and_bail_chord_respawns() {
    let mut b = Builder::new();
    b.ground(Surface::Smooth, -10.0, -10.0, 10.0, 20.0, 0.0);
    b.block(Surface::Smooth, 0.0, 8.0, 0.0, 8.0, 1.0, 2.0, 0.0);
    let mut s = sim(World::from_spot(&b.finish("wall", "wall", ZERO, 0.0)));
    start(&mut s, ZERO, 0.0, 6.0);
    let ev = run(&mut s, 90, pad());
    assert!(ev.iter().any(|e| matches!(e, Event::RunOut)), "{ev:?}");
    assert_eq!(s.p.state, State::OffBoard);

    start(&mut s, ZERO, 0.0, 3.0);
    let chord = VirtualController { lt: 1.0, rt: 1.0, ..pad() }.with(btn::L3, true);
    run(&mut s, 2, chord);
    let ev = run(&mut s, 1, chord.with(btn::R3, true));
    assert!(bailed(&ev).is_some(), "{ev:?}");
    run(&mut s, 100, pad());
    let ev = run(&mut s, 5, pad().with(btn::A, true));
    assert!(ev.iter().any(|e| matches!(e, Event::Respawn)), "{ev:?}");
    assert_eq!(s.p.state, State::Ground);
}

// ---- step 9: scoring ----------------------------------------------------------------------

#[test]
fn c3_9_a_landed_kickflip_banks() {
    let mut s = sim(flat_world());
    start(&mut s, ZERO, 0.0, 5.0);
    run(&mut s, 5, pad());
    let mut ev = run(&mut s, 2, rs(0.0, -0.95));
    ev.extend(run(&mut s, 1, rs(-0.55, -0.75)));
    ev.extend(run(&mut s, 1, rs(-0.9, 0.45)));
    ev.extend(run(&mut s, 120, pad()));
    let banked = ev.iter().find_map(|e| if let Event::Banked { names, points, .. } = e { Some((names.clone(), *points)) } else { None });
    let (names, points) = banked.expect("nothing banked");
    assert!(names.iter().any(|n| n.contains("Kickflip")), "{names:?}");
    assert!(points >= 250.0, "{points}");
}

#[test]
fn spots_load_and_spawn_on_the_ground() {
    for f in sk8::spots::all() {
        let name = f.name.clone();
        let w = World::from_spot(&f);
        assert!(!w.tris.is_empty() && !w.chords.is_empty(), "{name}");
        let mut s = Sim::new(w, Difficulty::Normal);
        run(&mut s, 120, pad());
        assert_eq!(s.p.state, State::Ground, "{name} spawn");
    }
}

/// A ledge ridden slowly to its stall: off it, once, not locked again where it stopped
/// (player test 2026-11-07: a stalled 50-50 at Plattan re-locked every half second).
#[test]
fn a_stalled_grind_comes_off_and_does_not_lock_again() {
    let mut s = sim(ledge_world(8.0));
    start(&mut s, v3(-0.4, 0.0, -1.0), 0.0, 2.7);
    run(&mut s, 10, pad());
    let mut ev = ollie(&mut s);
    ev.extend(run(&mut s, 600, pad()));
    let names = grind_names(&ev);
    let ends: Vec<_> = ev.iter().filter_map(|e| if let Event::GrindEnd { reason, .. } = e { Some(reason.clone()) } else { None }).collect();
    assert!(names.len() <= 1, "grinds {names:?}, ends {ends:?}, at {:?} state {:?}", s.p.pos, s.p.state);
    // And back on the ground, beside the ledge or on it, not hanging in the air over it.
    assert_eq!(s.p.state, State::Ground, "ends {ends:?}, at {:?} vel {:?}", s.p.pos, s.p.vel);
}
