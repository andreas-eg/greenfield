//! Beargarden's two styles, as its realm tunes them (`world/skate/tuning.txt`): arcade (big
//! airs, grinds that find you, wide landings) and realistic (a knee-high flat ollie, grinds you
//! aim for, square landings). Each check is a number a player would notice.

use sk8::build::Builder;
use sk8::math::*;
use sk8::sim::{Event, Landing};
use sk8::world::{RailMaterial, Surface, World};
use sk8::{Difficulty, Sim, VirtualController};

fn styled(w: World, style: &str) -> Sim {
    let mut s = Sim::new(w, Difficulty::Normal);
    let text = std::fs::read_to_string("../world/skate/tuning.txt").unwrap();
    s.set_tuning(Difficulty::Normal, &text, style);
    s
}

fn flat() -> World {
    let mut b = Builder::new();
    b.ground(Surface::Smooth, -100.0, -100.0, 100.0, 100.0, 0.0);
    World::from_spot(&b.finish("flat", "flat", ZERO, 0.0))
}

fn ledge() -> World {
    let mut b = Builder::new();
    b.ground(Surface::Smooth, -20.0, -20.0, 20.0, 40.0, 0.0);
    b.ledge(Surface::Smooth, RailMaterial::Concrete, 0.2, 4.0, 0.0, 0.4, 8.0, 0.45, 0.0);
    World::from_spot(&b.finish("ledge", "ledge", ZERO, 0.0))
}

fn start(s: &mut Sim, pos: Vec3, yaw: f32, speed: f32) {
    let f = v3(yaw.sin(), 0.0, yaw.cos());
    s.place(pos, f);
    s.p.grace = 0.0;
    s.p.vel = f * speed;
    s.p.speed_target = speed;
}

fn run(s: &mut Sim, n: usize, c: VirtualController) -> Vec<Event> {
    let mut ev = Vec::new();
    for _ in 0..n {
        s.step(&[c]);
        ev.extend(s.take_events());
    }
    ev
}

fn rs(y: f32) -> VirtualController {
    VirtualController { ry: y, ..Default::default() }
}

/// A full flick: down, then snapped up.
fn ollie(s: &mut Sim) -> Vec<Event> {
    let mut ev = run(s, 3, rs(-1.0));
    ev.extend(run(s, 1, rs(0.0)));
    ev.extend(run(s, 2, rs(1.0)));
    ev.extend(run(s, 1, VirtualController::default()));
    ev
}

fn pop_height(style: &str) -> f32 {
    let mut s = styled(flat(), style);
    start(&mut s, ZERO, 0.0, 3.0);
    run(&mut s, 10, VirtualController::default());
    let ev = ollie(&mut s);
    ev.iter().find_map(|e| if let Event::Pop { height, .. } = e { Some(*height) } else { None }).expect("a pop")
}

#[test]
fn a_full_flick_on_the_flat_is_knee_high_realistic_and_big_arcade() {
    let (r, a) = (pop_height("realistic"), pop_height("arcade"));
    assert!((0.45..0.65).contains(&r), "realistic pops {r} m");
    assert!((0.95..1.2).contains(&a), "arcade pops {a} m");
}

#[test]
fn arcade_rolls_further() {
    let coast = |style: &str| {
        let mut s = styled(flat(), style);
        start(&mut s, ZERO, 0.0, 4.0);
        run(&mut s, 180, VirtualController::default());
        s.p.vel.len()
    };
    let (r, a) = (coast("realistic"), coast("arcade"));
    assert!(a > r + 0.2, "after 3 s from 4 m/s: arcade {a}, realistic {r}");
    assert!(a > 3.0, "arcade keeps {a} of 4 m/s over 3 s");
}

/// Riding along the ledge `off` metres from its edge, then an ollie: is a grind found?
fn locks(style: &str, off: f32) -> bool {
    let mut s = styled(ledge(), style);
    start(&mut s, v3(-off, 0.0, -2.0), 0.0, 4.5);
    run(&mut s, 10, VirtualController::default());
    let mut ev = ollie(&mut s);
    ev.extend(run(&mut s, 60, VirtualController::default()));
    ev.iter().any(|e| matches!(e, Event::GrindStart { .. }))
}

#[test]
fn arcade_grinds_find_you_and_realistic_ones_are_aimed() {
    assert!(locks("arcade", 0.8), "arcade from 0.8 m off the line");
    assert!(locks("realistic", 0.1), "realistic on the line");
    assert!(!locks("realistic", 0.6), "realistic from 0.6 m off the line");
}

/// A flat air with the left stick held round for `ticks` after the pop, then let go: a spin
/// started and not finished. How it lands.
fn spin_part(style: &str, ticks: usize) -> Option<Landing> {
    let mut s = styled(flat(), style);
    start(&mut s, v3(0.0, 0.0, 0.0), 0.0, 4.0);
    run(&mut s, 10, VirtualController::default());
    let mut ev = ollie(&mut s);
    ev.extend(run(&mut s, ticks, VirtualController { lx: 1.0, ..Default::default() }));
    ev.extend(run(&mut s, 120, VirtualController::default()));
    if std::env::var_os("SHOW").is_some() {
        eprintln!("{style} {ticks}: {:?}", ev.iter().filter(|e| !matches!(e, Event::Push)).collect::<Vec<_>>());
    }
    ev.iter().find_map(|e| if let Event::Land { quality, .. } = e { Some(*quality) } else { None })
}

#[test]
fn a_spin_left_short_is_finished_in_arcade_and_bails_realistic() {
    // 0.15 s of stick: arcade squares it up, realistic lands it crooked.
    assert_eq!(spin_part("arcade", 9), Some(Landing::Clean));
    assert_eq!(spin_part("realistic", 9), Some(Landing::Sketchy));
    // 0.2 s: arcade carries it round to a 180, realistic is left 45 deg off and bails.
    assert!(spin_part("arcade", 12).is_some(), "arcade rides away");
    assert_eq!(spin_part("realistic", 12), None);
}
