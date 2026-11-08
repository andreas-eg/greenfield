//! The left stick as the push on glass (SODER-LENS-S290) and the scooter (SODER-LENS-S291),
//! with Beargarden's arcade tuning (`world/skate/tuning.txt`): a light thumb rolls slowly, a
//! full one pushes hard, centred nothing pushes; the scooter's motor opens with the stick to
//! about 11 m/s, and its tricks score by a scooter rider's names.
//! @req SODER-LENS-S290 @req SODER-LENS-S291

use sk8::build::Builder;
use sk8::input::{btn, stick_push};
use sk8::math::*;
use sk8::sim::{scooter_name, Event};
use sk8::world::{Surface, World};
use sk8::{Difficulty, Sim, Vehicle, VirtualController};

fn flat() -> World {
    let mut b = Builder::new();
    b.ground(Surface::Smooth, -200.0, -200.0, 200.0, 200.0, 0.0);
    World::from_spot(&b.finish("flat", "flat", ZERO, 0.0))
}

fn arcade(v: Vehicle) -> Sim {
    let mut s = Sim::new(flat(), Difficulty::Normal);
    let text = std::fs::read_to_string("../world/skate/tuning.txt").unwrap();
    s.set_tuning(Difficulty::Normal, &text, "arcade");
    s.set_vehicle(v);
    // From standing, along -z from the far edge: room for a long roll.
    s.place(v3(0.0, 0.0, 150.0), v3(0.0, 0.0, -1.0));
    s.p.grace = 0.0;
    s
}

fn run(s: &mut Sim, ticks: usize, c: VirtualController) -> Vec<Event> {
    let mut ev = Vec::new();
    for _ in 0..ticks {
        s.step(&[c]);
        ev.extend(s.take_events());
    }
    ev
}

/// The speed after `secs` with the glass stick held forward at `y`, from standing.
fn speed_after(v: Vehicle, y: f32, secs: f32) -> f32 {
    let mut s = arcade(v);
    run(&mut s, 30, VirtualController::default());
    s.set_stick_push(stick_push(y));
    run(&mut s, (secs * 60.0) as usize, VirtualController::default());
    s.p.speed()
}

#[test]
fn the_stick_push_has_a_small_dead_zone_and_no_push_pulled_back() {
    assert_eq!(stick_push(0.05), 0.0);
    assert_eq!(stick_push(-1.0), 0.0);
    assert_eq!(stick_push(1.0), 1.0);
    assert!((stick_push(0.55) - 0.5).abs() < 1e-5);
}

#[test]
fn a_board_rolls_faster_the_further_the_stick_is_out() {
    let at = |y| speed_after(Vehicle::Board, y, 8.0);
    let (none, light, half, full) = (at(0.0), at(0.3), at(0.6), at(1.0));
    println!("board after 8 s: centred {none:.2}, 0.3 {light:.2}, 0.6 {half:.2}, 1.0 {full:.2} m/s");
    assert!(none < 0.05, "centred, no push: {none}");
    assert!(light > 1.0, "a light thumb still rolls: {light}");
    assert!(light + 1.0 < half && half + 1.0 < full, "{light} < {half} < {full}");
    // A full stick pushes as the button does.
    let mut s = arcade(Vehicle::Board);
    run(&mut s, 30, VirtualController::default());
    run(&mut s, 480, VirtualController::default().with(btn::A, true));
    assert!((s.p.speed() - full).abs() < 1.0, "button {} against a full stick {full}", s.p.speed());
}

#[test]
fn a_light_thumb_pushes_less_often() {
    let pushes = |y: f32| {
        let mut s = arcade(Vehicle::Board);
        run(&mut s, 30, VirtualController::default());
        s.set_stick_push(stick_push(y));
        run(&mut s, 360, VirtualController::default()).iter().filter(|e| matches!(e, Event::Push)).count()
    };
    let (light, full) = (pushes(0.3), pushes(1.0));
    assert!(light < full && light > 0, "pushes in 6 s: light {light}, full {full}");
}

#[test]
fn the_stick_let_go_stops_the_pushes() {
    let mut s = arcade(Vehicle::Board);
    run(&mut s, 30, VirtualController::default());
    s.set_stick_push(1.0);
    run(&mut s, 180, VirtualController::default());
    s.set_stick_push(0.0);
    run(&mut s, 60, VirtualController::default());
    let ev = run(&mut s, 240, VirtualController::default());
    assert!(!ev.iter().any(|e| matches!(e, Event::Push)), "{ev:?}");
}

#[test]
fn a_scooter_cruises_at_eleven_and_its_throttle_is_the_stick() {
    let at = |y| speed_after(Vehicle::Scooter, y, 10.0);
    let (none, light, half, full) = (at(0.0), at(0.3), at(0.6), at(1.0));
    println!("scooter after 10 s: centred {none:.2}, 0.3 {light:.2}, 0.6 {half:.2}, 1.0 {full:.2} m/s");
    assert!(none < 0.05, "{none}");
    assert!(light + 1.5 < half && half + 1.5 < full, "{light} < {half} < {full}");
    assert!((full - 11.0).abs() < 0.8, "full throttle cruises about 11 m/s: {full}");
    // Faster than the board, and no push clip: a motor.
    let mut s = arcade(Vehicle::Scooter);
    s.set_stick_push(1.0);
    let ev = run(&mut s, 300, VirtualController::default());
    assert!(!ev.iter().any(|e| matches!(e, Event::Push)) && !s.p.pushing);
    assert!(full > speed_after(Vehicle::Board, 1.0, 10.0) + 1.0);
}

#[test]
fn a_scooter_brakes() {
    let mut s = arcade(Vehicle::Scooter);
    s.set_stick_push(1.0);
    run(&mut s, 360, VirtualController::default());
    let fast = s.p.speed();
    s.set_stick_push(0.0);
    run(&mut s, 120, VirtualController::default().with(btn::B, true));
    assert!(s.p.speed() < fast - 5.0, "{fast} braked to {}", s.p.speed());
}

#[test]
fn a_tailwhip_lands_and_scores_by_its_name() {
    let mut s = arcade(Vehicle::Scooter);
    s.set_stick_push(0.6);
    run(&mut s, 180, VirtualController::default());
    s.set_stick_push(0.0);
    // The kickflip's flick: down, then curved out and up.
    let rs = |x, y| VirtualController { rx: x, ry: y, ..Default::default() };
    let mut ev = run(&mut s, 2, rs(0.0, -0.95));
    ev.extend(run(&mut s, 1, rs(-0.55, -0.75)));
    ev.extend(run(&mut s, 1, rs(-0.9, 0.45)));
    ev.extend(run(&mut s, 150, VirtualController::default()));
    assert!(!ev.iter().any(|e| matches!(e, Event::Bail { .. })), "{ev:?}");
    assert!(ev.iter().any(|e| matches!(e, Event::Trick { name } if name == "Tailwhip")), "{ev:?}");
    let banked = ev.iter().find_map(|e| if let Event::Banked { names, points, .. } = e { Some((names.clone(), *points)) } else { None });
    let (names, points) = banked.expect("nothing banked");
    assert!(names.iter().any(|n| n.contains("Tailwhip")) && !names.iter().any(|n| n.contains("Kickflip")), "{names:?}");
    assert!(points >= 250.0, "{points}");
}

#[test]
fn board_names_become_scooter_names() {
    assert_eq!(scooter_name("Ollie"), "Bunny Hop");
    assert_eq!(scooter_name("Nollie"), "Nose Hop");
    assert_eq!(scooter_name("Switch FS 180 Kickflip Indy"), "Switch FS 180 Tailwhip Indy");
    assert_eq!(scooter_name("Heelflip"), "Reverse Tailwhip");
    assert_eq!(scooter_name("Pop Shuvit"), "Barspin");
    assert_eq!(scooter_name("FS Pop Shuvit"), "Reverse Barspin");
    assert_eq!(scooter_name("360 Flip"), "Double Whip");
    assert_eq!(scooter_name("360 Pop Shuvit"), "360 Barspin");
    assert_eq!(scooter_name("Varial Kickflip"), "Tailwhip Barspin");
    assert_eq!(scooter_name("Boardslide"), "Deckslide");
    assert_eq!(scooter_name("FS 50-50"), "FS 50-50");
    assert_eq!(scooter_name("Manual"), "Manual");
}
