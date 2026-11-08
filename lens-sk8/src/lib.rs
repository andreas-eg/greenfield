//! Beargarden's boards as a lens game mode: a ridden thing whose drive says `"sim": "sk8"` is
//! ridden by the sk8 simulation (`../sk8`, the sk8r specification), not by the lens's own
//! board.
//!
//! sk8 owns the board while it is ridden: physics at a fixed 1/60 s, the Flickit gestures,
//! grinds, landings, bails, the score of a line. The lens hands it the player's hands each
//! frame (lens-mode's `Input`), draws what it says (the board with its flip, the rider in
//! its clip, its camera), tells the world where the board is as for any ride, and passes on
//! the `skate ...` lines, which the realm scores (`world/support/skate_rules.c`).
//!
//! The spot's ridden surfaces are the chart's `"skate"` file, beside the chart.
//! @req GF-REQ-LENS-1 @req GF-REQ-LENS-2 @req GF-REQ-LENS-4 @req SODER-LENS-S260

use lens_mode::{Attrs, Camera, Frame, Input, Mat, Registration, Ride, Spectator, Start};
use sk8::input::{btn, VirtualController};
use sk8::math::v3;
use sk8::{Difficulty, Event, Sim, State, World};
use std::collections::{HashMap, HashSet};

/// The mode as the lens is given it.
pub fn registration() -> Registration {
    Registration { sim: "sk8", ride: start, spectator: spectator }
}

fn start(s: &Start) -> Result<Box<dyn Ride>, String> {
    Ok(Box::new(Skate::start(s)?))
}

fn spectator() -> Box<dyn Spectator> {
    Box::new(Others::default())
}

/// The realm's numbers for sk8 (the board's `sim_tuning`), as Beargarden keeps them.
const TUNING: &str = "skate/tuning.txt";

/// sk8's deck centre is this far over the surface (sk8::sim::DECK_H); the board mesh
/// (`sth_skateboard`) stands on its wheels at its origin, its deck's top this far up.
pub const DECK_H: f32 = 0.075;
pub const DECK_TOP: f32 = 0.11;

/// The skate clips the bodies carry (tools/skate_clips.py), for each clip sk8 names
/// (sk8::frame::CLIPS). One clip stands for several where the authored set has one.
pub fn body_clip(sk8: &str) -> &'static str {
    match sk8 {
        "push" | "push_mongo" => "skate_push",
        "brake" | "crouch" | "kick_turn" => "skate_crouch",
        "manual" | "nose_manual" => "skate_manual",
        "powerslide" | "revert" => "skate_powerslide",
        "ollie" => "skate_ollie",
        "air" => "skate_air",
        "grab" => "skate_grab",
        "land" | "land_sketchy" => "skate_land",
        "grind_5050" | "grind_50" | "grind_nosegrind" | "boardslide" | "noseslide" | "tailslide" | "darkslide" => "skate_grind",
        "bail" | "run_out" | "bail_cannonball" | "bail_swan_dive" | "bail_judo_kick" | "bail_torpedo" => "skate_bail",
        "wallride" => "skate_wallride",
        "handplant" => "skate_handplant",
        "walk" => "walk",
        _ => "skate_ride",
    }
}

/// Played once and held on the last frame; the rest loop.
pub fn one_shot(clip: &str) -> bool {
    matches!(clip, "skate_ollie" | "skate_land" | "skate_bail")
}

/// What is drawn this frame, between the last two ticks.
#[derive(Clone, Copy, Debug)]
struct Pose {
    /// Board deck centre, its nose and its up.
    pos: glam::Vec3,
    fwd: glam::Vec3,
    up: glam::Vec3,
    /// About the long axis, about up, about the right (radians).
    flip: f32,
    shuv: f32,
    pitch: f32,
    rider: glam::Vec3,
    body_flip: f32,
    clip: &'static str,
    goofy: bool,
    on_board: bool,
    speed: f32,
}

/// The rider's stance, difficulty and style as the world shows them, with today's defaults:
/// arcade unless the rider chose realistic.
fn settings(attrs: &Attrs) -> (bool, String, &'static str) {
    let get = |k: &str| attrs.get(k).map(String::as_str).unwrap_or("");
    let style = if get("skate_style") == "realistic" { "realistic" } else { "arcade" };
    (get("skate_stance") == "goofy", get("skate_difficulty").to_string(), style)
}

fn level(difficulty: &str) -> Difficulty {
    match difficulty {
        "easy" => Difficulty::Easy,
        "hardcore" => Difficulty::Hardcore,
        _ => Difficulty::Normal,
    }
}

struct Skate {
    sim: Sim,
    prev: sk8::Frame,
    cur: sk8::Frame,
    realm: std::path::PathBuf,
    /// The rider's stance, difficulty and style it was made with: a change retunes it.
    settings: String,
    /// The keys as sticks, ramped (0 to 1 in 0.12 s, back in 0.08 s, spec C.2).
    left: glam::Vec2,
    right: glam::Vec2,
    /// The mouse dragged as the right stick: integrated, springing back at rest.
    mouse: glam::Vec2,
    mouse_rest: f32,
    /// The right thumb on glass: how far it has gone from where it came down, in pixels
    /// (a floating stick, spec C.2), while it is down.
    thumb: Option<glam::Vec2>,
    /// The OLLIE button (glass, or the lens's TRICK): an ollie's flick played for the
    /// thumb, from when it went down; and whether it was down last frame.
    ollie_t: Option<f32>,
    trick_was: bool,
    /// The trick button pressed in the air and still held: a grab.
    grabbing: bool,
    /// The state last written to the record, and the time since its last sample.
    logged_state: String,
    sample_t: f32,
    /// Seconds since the ride began, for the record.
    clock: f32,
    /// The clip the world was last told the rider is in, and how long ago: others see the
    /// rider by it (`skate clip`, GF-REQ-LENS-6).
    told: (&'static str, f32),
    /// The flip sk8 is turning the board through, as last told: other players see it from
    /// the realm (`skate board ...`, shown on the rider as `skate_board`).
    flip_told: Option<f32>,
    /// The samples since the last tick: a frame drawn between ticks keeps its sample for the
    /// next tick (B.4.1, every sample), and the frame rate is the sample rate.
    pending: Vec<VirtualController>,
    rate: f32,
    /// Where the rider stood last frame: off the board they face the way they move.
    last_rider: Option<glam::Vec3>,
    lines: Vec<String>,
    record: Vec<String>,
    events: Vec<(String, serde_json::Value)>,
}

fn g(v: [f32; 3]) -> glam::Vec3 {
    glam::Vec3::from_array(v)
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> glam::Vec3 {
    g(a).lerp(g(b), t)
}

/// A key's stick axis eased toward where the keys hold it.
fn ramp(v: f32, target: f32, dt: f32) -> f32 {
    if target != 0.0 && (v == 0.0 || v.signum() == target.signum()) {
        (v + target.signum() * dt / 0.12).clamp(-1.0, 1.0)
    } else {
        let d = dt / 0.08;
        if v.abs() <= d {
            if target == 0.0 {
                0.0
            } else {
                target.signum() * d
            }
        } else {
            v - v.signum() * d
        }
    }
}

/// `LENS_SK8_TIMESCALE`: the board's time slowed (0.25 a quarter speed), for looking at a
/// trick's poses in the air; 1 otherwise.
fn time_scale() -> f32 {
    static SCALE: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *SCALE.get_or_init(|| std::env::var("LENS_SK8_TIMESCALE").ok().and_then(|v| v.parse().ok()).filter(|v: &f32| *v > 0.0).unwrap_or(1.0))
}

fn trace() -> bool {
    std::env::var_os("LENS_SK8_TRACE").is_some()
}

impl Skate {
    /// The board for the start's chart, if its chart has a skate file, put down where the
    /// rider stands.
    fn start(s: &Start) -> Result<Skate, String> {
        let realm = std::path::PathBuf::from(&s.realm);
        let charts = realm.join("charts");
        let doc = std::fs::read_to_string(charts.join(format!("{}.json", s.chart))).map_err(|e| format!("{}.json: {e}", s.chart))?;
        let doc: serde_json::Value = serde_json::from_str(&doc).map_err(|e| format!("{}.json: {e}", s.chart))?;
        let file = doc.get("skate").and_then(|s| s.as_str()).ok_or_else(|| format!("{}.json names no skate file", s.chart))?;
        let text = std::fs::read_to_string(charts.join(file)).map_err(|e| format!("{file}: {e}"))?;
        let world = World::from_json(&text)?;
        // Stance and style are set by retune, below.
        let (_, difficulty, _) = settings(&s.attrs);
        let sim = Sim::new(world, level(&difficulty));
        let f = sk8::Frame::of(&sim);
        let mut k = Skate {
            sim,
            prev: f,
            cur: f,
            realm,
            settings: String::new(),
            left: glam::Vec2::ZERO,
            right: glam::Vec2::ZERO,
            mouse: glam::Vec2::ZERO,
            mouse_rest: 0.0,
            thumb: None,
            ollie_t: None,
            trick_was: false,
            grabbing: false,
            logged_state: String::new(),
            sample_t: 0.0,
            clock: 0.0,
            told: ("", 0.0),
            flip_told: None,
            pending: Vec::new(),
            rate: 60.0,
            last_rider: None,
            lines: Vec::new(),
            record: Vec::new(),
            events: Vec::new(),
        };
        k.retune(&s.attrs);
        k.place(s.at[0], s.at[1], s.at[2], s.yaw);
        println!("  sk8: on the board at {}, ({:.1}, {:.1}), {}", s.chart, s.at[0], s.at[2], k.settings);
        let mut f = here(&k.sim);
        f["chart"] = serde_json::json!(s.chart);
        f["settings"] = serde_json::json!(k.settings);
        k.log("on_board", f);
        Ok(k)
    }

    /// Put the board down at x, z (on whatever surface is under it), heading the lens's way:
    /// `yaw` 0 east (+x), its forward (cos, 0, sin). At the spot's start (where the world
    /// puts a skater who rides there) the board faces the way the spot's start does, toward
    /// its features; anywhere else, the way the walker faced. Put down facing a wall at
    /// Fryshuset, the first push went into it.
    fn place(&mut self, x: f32, y: f32, z: f32, yaw: f32) {
        let w = &self.sim.world;
        let at_start = (w.spawn.x - x).hypot(w.spawn.z - z) < 1.0;
        let fwd = if at_start { v3(w.spawn_heading.sin(), 0.0, w.spawn_heading.cos()) } else { v3(yaw.cos(), 0.0, yaw.sin()) };
        self.sim.place(v3(x, y + 0.5, z), fwd);
        let f = sk8::Frame::of(&self.sim);
        self.prev = f;
        self.cur = f;
    }

    /// The ride's record: one JSON object a line, with the time and the kind.
    fn log(&mut self, kind: &str, mut fields: serde_json::Value) {
        if let Some(o) = fields.as_object_mut() {
            o.insert("t".into(), serde_json::json!((self.clock as f64 * 1000.0).round() / 1000.0));
            o.insert("kind".into(), serde_json::json!(kind));
        }
        let line = fields.to_string();
        if trace() {
            println!("  sk8 {line}");
        }
        self.record.push(line);
    }

    /// This frame's controller from the player's hands: the pad's sticks and buttons, the
    /// keys (WASD the left stick, W and Space push, S and C brake, the arrows the right stick
    /// (the flick), Z and G the triggers (grabs), H and Y a manual and a nose manual, T on and
    /// off the board, E the X button, Q and R the shoulders, Shift crouch), the mouse dragged,
    /// and the glass: its stick and pedals (x steer, y +1 PUSH, -1 FOOT BRAKE), OLLIE (the
    /// lens's TRICK), which flicks an ollie for the thumb, and the right thumb's drag.
    fn sample(&mut self, i: &Input, dt: f32) -> VirtualController {
        if i.mouse != [0.0, 0.0] {
            self.mouse += glam::Vec2::new(i.mouse[0], -i.mouse[1]) / 120.0;
            self.mouse = self.mouse.clamp_length_max(1.0);
            self.mouse_rest = 0.0;
        }
        if i.thumb != [0.0, 0.0] {
            *self.thumb.get_or_insert(glam::Vec2::ZERO) += glam::Vec2::new(i.thumb[0], -i.thumb[1]);
        }
        // Where the thumb was when it lifted counts for this frame: a fast flick ends with
        // the finger off the glass in the same frame as its last move.
        let thumb_stick = self.thumb.map(|t| (t / i.thumb_radius.max(1.0)).clamp_length_max(1.0)).unwrap_or_default();
        if trace() && (i.thumb_down || self.thumb.is_some()) {
            println!("  sk8 thumb: down={} stick=({:.2}, {:.2})", i.thumb_down, thumb_stick.x, thumb_stick.y);
        }
        if !i.thumb_down {
            self.thumb = None;
        }
        let keys: HashSet<&str> = i.keys.iter().map(String::as_str).collect();
        let key = |c: &str| keys.contains(c);
        let k = |c: &str| key(c) as i32 as f32;
        let lx = k("KeyD") - k("KeyA");
        let ly = k("KeyW") - k("KeyS");
        let rx = k("ArrowRight") - k("ArrowLeft");
        let ry = k("ArrowUp") - k("ArrowDown");
        // H and Y hold the right stick three quarters back or forward: a manual and a nose
        // manual (B.7.3 wants 0.45 to 0.92 of the stick, which an arrow's full 1 is not).
        let hold = 0.75 * (k("KeyY") - k("KeyH"));
        self.left = glam::Vec2::new(ramp(self.left.x, lx, dt), ramp(self.left.y, ly, dt));
        // The flick keys are where the stick is, at once: an eased key reads to the
        // recognizer as a slow flick (an ollie from the arrows popped 0.46 m, a pad's 0.88 m,
        // player test 2026-11-07), and a key has no half way to ease through anyway.
        self.right = glam::Vec2::new(rx, ry);
        if ry == 0.0 && hold != 0.0 {
            self.right.y = hold;
        }
        self.mouse_rest += dt;
        if self.mouse_rest > 0.03 {
            self.mouse = glam::Vec2::ZERO;
        }
        // A script's pad stands for the rest while it holds.
        if let Some(p) = &i.script_pad {
            let a = p.axes;
            let mut c = VirtualController { lx: a[0], ly: a[1], rx: a[2], ry: a[3], lt: a[4], rt: a[5], buttons: 0 };
            for n in &p.buttons {
                let b = match n.as_str() {
                    "A" => btn::A,
                    "B" => btn::B,
                    "X" => btn::X,
                    "Y" => btn::Y,
                    "LB" => btn::LB,
                    "RB" => btn::RB,
                    "L3" => btn::L3,
                    "R3" => btn::R3,
                    _ => continue,
                };
                c = c.with(b, true);
            }
            return c;
        }
        let pick = |a: f32, b: f32| if b.abs() > a.abs() { b } else { a };
        let trick = i.trick;
        // The ollie button: down and back up through the stick in a tenth of a second; off
        // the board (a run-out, walking) it is Y, back on.
        let off = self.sim.p.state == State::OffBoard;
        // In the air the same button grabs, held (a phone has no triggers): it pops on the
        // ground and grabs in the air.
        let airborne = matches!(self.sim.p.state, State::Air | State::KnownAir);
        // Pressed afresh in the air, not the ollie's press still held.
        if trick && !self.trick_was && airborne {
            self.grabbing = true;
        }
        if !trick || !airborne {
            self.grabbing = false;
        }
        let grab = self.grabbing;
        if trick && !self.trick_was && !off && !airborne {
            self.ollie_t = Some(0.0);
        }
        self.trick_was = trick;
        let flick = self.ollie_t.map(|t| if t < 0.07 { -1.0 } else { 1.0 });
        self.ollie_t = self.ollie_t.map(|t| t + dt).filter(|t| *t < 0.13);
        let stick = i.stick.unwrap_or_default();
        let (ls, rs) = (i.pad.left, i.pad.right);
        let held = |b: &str| i.pad.held.iter().any(|h| h == b);
        let mut c = VirtualController {
            lx: pick(pick(self.left.x, ls[0]), stick[0]),
            ly: pick(self.left.y, ls[1]),
            rx: pick(pick(pick(self.right.x, self.mouse.x), rs[0]), thumb_stick.x),
            ry: flick.unwrap_or_else(|| pick(pick(pick(self.right.y, self.mouse.y), rs[1]), thumb_stick.y)),
            lt: if held("L2") || key("KeyZ") || grab { 1.0 } else { 0.0 },
            rt: if held("R2") || key("KeyG") { 1.0 } else { 0.0 },
            buttons: 0,
        };
        for (on, b) in [
            // W and S are the board's PUSH and FOOT BRAKE as the lens's hints say (the
            // world's controls); they are the left stick's forward and back as well.
            (held("South") || key("Space") || key("KeyW") || stick[1] > 0.5, btn::A),
            // S alone: with A or D it is the left stick back and to the side, the powerslide.
            (held("East") || key("KeyC") || (lx == 0.0 && key("KeyS")) || stick[1] < -0.5, btn::B),
            (held("West") || key("KeyE"), btn::X),
            (held("L1") || key("KeyQ"), btn::LB),
            (held("R1") || key("KeyR"), btn::RB),
            (held("L3") || key("ShiftLeft"), btn::L3),
            (held("R3"), btn::R3),
            // Y: off the board and on again (B.12); T on the keys.
            (held("North") || key("KeyT") || (off && trick), btn::Y),
        ] {
            c = c.with(b, on);
        }
        c
    }

    /// Real time on: as many fixed ticks as fit, every sample given.
    fn advance(&mut self, dt: f32, samples: &[VirtualController]) {
        // The recognizer counts samples, so it is told how many a second it gets: the frame
        // rate, eased, at least 60 (a frame sampled once).
        if dt > 1e-5 {
            self.rate += ((1.0 / dt).clamp(60.0, 1000.0) - self.rate) * 0.05;
            self.sim.set_sample_rate(self.rate);
        }
        self.pending.extend_from_slice(samples);
        // Never more than a tenth of a second of them.
        let keep = (self.rate * 0.1) as usize + 1;
        if self.pending.len() > keep {
            self.pending.drain(..self.pending.len() - keep);
        }
        let ticks = self.sim.advance(dt, &self.pending);
        if ticks == 0 {
            return;
        }
        self.pending.clear();
        // The record: every change of state, and the ride twice a second with the ground
        // under it.
        let now = format!("{:?}", self.sim.p.state);
        if now != self.logged_state {
            self.log("state", here(&self.sim));
            self.logged_state = now;
        }
        self.sample_t += dt;
        if self.sample_t >= 0.5 {
            self.sample_t = 0.0;
            self.log("sample", here(&self.sim));
        }
        // Under the spot's lowest floor is through the world: said, and back to the start.
        let floor = self.sim.world.floor_y();
        if self.sim.p.pos.y < floor - 2.0 {
            let mut f = here(&self.sim);
            f["floor"] = serde_json::json!((floor as f64 * 100.0).round() / 100.0);
            self.log("fell_through", f.clone());
            self.events.push(("sk8_fell_through".into(), f));
            let (w, h) = (self.sim.world.spawn, self.sim.world.spawn_heading);
            self.sim.place(v3(w.x, w.y + 0.5, w.z), v3(h.sin(), 0.0, h.cos()));
        }
        // A flip begun (or a late one replacing it) is told once, at its start.
        match self.sim.p.flip {
            Some(f) if self.flip_told.map_or(true, |t| f.t < t) => {
                self.lines.push(format!("skate board {:.2}_{:.0}_{:.2}", f.flip_turns, f.shuv_deg, f.dur));
                self.flip_told = Some(f.t);
            }
            Some(f) => self.flip_told = Some(f.t),
            None => self.flip_told = None,
        }
        self.prev = self.sim.prev_frame.unwrap_or(self.cur);
        self.cur = sk8::Frame::of(&self.sim);
        for e in self.sim.take_events() {
            if let Some(line) = line_for(&e) {
                self.lines.push(line);
            }
            if !matches!(e, Event::Push | Event::Marker) {
                println!("  sk8: {}", serde_json::to_string(&e).unwrap_or_default());
                let mut f = here(&self.sim);
                f["event"] = serde_json::to_value(&e).unwrap_or_default();
                self.log("event", f);
            }
            // What a player met, for the game's feedback from phones (the lens sends a mode's
            // events on to Firebase): a bail and its reason, a run-out.
            if let Event::Bail { reason } = &e {
                let mut f = here(&self.sim);
                f["reason"] = serde_json::json!(reason);
                self.events.push(("sk8_bail".into(), f));
            }
            if matches!(e, Event::RunOut) {
                self.events.push(("sk8_run_out".into(), here(&self.sim)));
            }
            if matches!(e, Event::Bail { .. }) {
                let p = self.sim.p.pos;
                println!("  sk8: bailed at ({:.1}, {:.1}, {:.1}), {:.1} m/s", p.x, p.y, p.z, self.sim.p.speed());
            }
        }
    }

    /// Tell the world the rider's clip when it changes (at most four times a second) and
    /// again every two seconds, so the others see a skater, not a walker on a board.
    fn tell_clip(&mut self, clip: &'static str, dt: f32) {
        self.told.1 += dt;
        if (clip != self.told.0 && self.told.1 >= 0.25) || self.told.1 >= 2.0 {
            self.told = (clip, 0.0);
            self.lines.push(format!("skate clip {clip}"));
        }
    }

    /// Between the last two ticks, as far as the time since the last one says.
    fn pose(&self) -> Pose {
        let a = (self.sim.accumulator / sk8::sim::DT).clamp(0.0, 1.0);
        let (p, c) = (&self.prev, &self.cur);
        Pose {
            pos: mix(p.pos, c.pos, a),
            fwd: mix(p.fwd, c.fwd, a).normalize_or(g(c.fwd)),
            up: mix(p.up, c.up, a).normalize_or(g(c.up)),
            flip: c.flip_angle,
            shuv: c.shuv_angle,
            pitch: c.pitch,
            rider: mix(p.rider_pos, c.rider_pos, a),
            body_flip: c.body_flip,
            clip: body_clip(sk8::frame::clip_name(c.clip)),
            goofy: c.goofy != 0,
            on_board: !matches!(self.sim.p.state, State::Wipeout | State::OffBoard),
            speed: c.speed,
        }
    }

    /// sk8's own camera (B.16), between its last two ticks.
    fn camera(&self) -> Camera {
        let a = (self.sim.accumulator / sk8::sim::DT).clamp(0.0, 1.0);
        Camera {
            eye: mix(self.prev.cam_pos, self.cur.cam_pos, a).to_array(),
            target: mix(self.prev.cam_target, self.cur.cam_target, a).to_array(),
            fov: self.cur.cam_fov,
        }
    }
}

impl Ride for Skate {
    fn step(&mut self, dt: f32, input: &Input) -> Frame {
        self.clock += dt;
        let sample = self.sample(input, dt);
        self.advance(dt * time_scale(), &[sample]);
        let p = self.pose();
        self.tell_clip(p.clip, dt);
        // On the board the way it points; off it (a bail, walking) the way the rider moves.
        let fwd = if p.on_board { p.fwd } else { self.last_rider.map_or(p.fwd, |l| (p.rider - l).with_y(0.0).normalize_or(p.fwd)) };
        self.last_rider = Some(p.rider);
        // On the board the feet are on the deck, the regular rider's left foot to the nose
        // (the clips are authored so), turned round for goofy; off it, where sk8 has the body.
        let body = if p.on_board {
            let stance = if p.goofy { glam::Mat4::from_rotation_y(std::f32::consts::PI) } else { glam::Mat4::IDENTITY };
            board_matrix(p.pos + p.up * (DECK_TOP - DECK_H), p.fwd, p.up) * stance
        } else {
            let flat = p.fwd.with_y(0.0).normalize_or(glam::Vec3::X);
            board_matrix(p.rider, flat, glam::Vec3::Y) * glam::Mat4::from_rotation_z(-p.body_flip)
        };
        Frame {
            rider: p.rider.to_array(),
            facing: fwd.to_array(),
            speed: p.speed,
            mount: board_matrix(p.pos - p.up * DECK_H, p.fwd, p.up).to_cols_array(),
            trick: board_trick(&p).to_cols_array(),
            body: body.to_cols_array(),
            clip: p.clip.to_string(),
            clip_once: one_shot(p.clip),
            camera: Some(self.camera()),
            // Rolling, the world's offers would cover the line ahead: only getting off stays.
            hide_offers: self.sim.p.vel.len() > 0.7,
        }
    }

    /// A new stance, difficulty or style tunes the ride where it is, mid-air or on a wall.
    fn retune(&mut self, attrs: &Attrs) {
        let (goofy, difficulty, style) = settings(attrs);
        let now = format!("{} {} {style}", attrs.get("skate_stance").map(String::as_str).unwrap_or(""), difficulty);
        if now == self.settings {
            return;
        }
        let overrides = std::fs::read_to_string(self.realm.join(TUNING)).unwrap_or_default();
        self.sim.set_tuning(level(&difficulty), &overrides, style);
        self.sim.p.goofy = goofy;
        if !self.settings.is_empty() {
            println!("  sk8: retuned, {now}");
        }
        self.settings = now;
    }

    fn lines(&mut self) -> Vec<String> {
        std::mem::take(&mut self.lines)
    }

    fn record(&mut self) -> Vec<String> {
        std::mem::take(&mut self.record)
    }

    fn events(&mut self) -> Vec<(String, serde_json::Value)> {
        std::mem::take(&mut self.events)
    }

    fn end(&mut self) -> Vec<String> {
        println!("  sk8: off the board");
        let f = here(&self.sim);
        self.log("off_board", f);
        std::mem::take(&mut self.record)
    }
}

/// What the world hears (feature_requests.md, GF-REQ-LENS-2).
pub fn line_for(e: &Event) -> Option<String> {
    Some(match e {
        Event::Banked { names, points, multiplier } => {
            format!("skate banked {} {:.1} {}", points.round() as i64, multiplier, names.join("+").replace(' ', "_"))
        }
        Event::LineEnded { points } => format!("skate line {}", points.round() as i64),
        Event::Bail { reason } => format!("skate bail {}", reason.replace(' ', "_")),
        Event::Land { quality, .. } => format!("skate land {}", format!("{quality:?}").to_lowercase()),
        Event::GrindStart { name } => format!("skate grind {}", name.replace(' ', "_")),
        _ => return None,
    })
}

/// Where the rider is, for the record: position, velocity, state, and the ground under them.
fn here(sim: &Sim) -> serde_json::Value {
    let p = &sim.p;
    let r = |x: f32| (x as f64 * 100.0).round() / 100.0;
    let ground = sim.world.raycast(p.pos + v3(0.0, 0.5, 0.0), v3(0.0, -1.0, 0.0), 30.0).map(|h| r(h.point.y));
    serde_json::json!({
        "pos": [r(p.pos.x), r(p.pos.y), r(p.pos.z)],
        "vel": [r(p.vel.x), r(p.vel.y), r(p.vel.z)],
        "state": format!("{:?}", p.state),
        "ground": ground,
    })
}

/// The board's frame as a matrix in the lens's model convention: +X its nose, +Y up, +Z its
/// right.
pub fn board_matrix(pos: glam::Vec3, fwd: glam::Vec3, up: glam::Vec3) -> glam::Mat4 {
    let right = fwd.cross(up).normalize_or(glam::Vec3::Z);
    let up = right.cross(fwd).normalize_or(glam::Vec3::Y);
    glam::Mat4::from_cols(fwd.extend(0.0), up.extend(0.0), right.extend(0.0), pos.extend(1.0))
}

/// The board under the feet with its trick on it: the flip about its long axis (+X), the
/// shuvit about up, the manual's pitch about its right, turned about the deck's middle a
/// little above it.
fn board_trick(p: &Pose) -> glam::Mat4 {
    let lift = glam::Vec3::Y * 0.05;
    glam::Mat4::from_translation(lift)
        * glam::Mat4::from_rotation_y(-p.shuv)
        * glam::Mat4::from_rotation_x(p.flip)
        * glam::Mat4::from_rotation_z(p.pitch)
        * glam::Mat4::from_translation(-lift)
}

/// Other riders as their realm shows them: their board flips (`skate_board`,
/// "<count> <turns>_<shuvit deg>_<seconds>") and their stance (`skate_stance`).
#[derive(Default)]
struct Others {
    /// The value last seen, when it changed, and the flip it says. Seen first, a value is
    /// old news and not played.
    flips: HashMap<String, (String, f32, [f32; 3])>,
    goofy: HashSet<String>,
}

impl Spectator for Others {
    fn heard(&mut self, who: &str, attrs: &Attrs, now: f32) {
        if let Some(v) = attrs.get("skate_stance") {
            if v == "goofy" {
                self.goofy.insert(who.to_string());
            } else {
                self.goofy.remove(who);
            }
        }
        let Some(value) = attrs.get("skate_board") else { return };
        let seen = self.flips.get(who).map(|(v, ..)| v.as_str());
        if seen == Some(value.as_str()) {
            return;
        }
        let n: Vec<f32> = value.split(|c: char| c.is_whitespace() || c == '_').skip(1).filter_map(|w| w.parse().ok()).collect();
        let Some(&[turns, shuv, dur]) = n.get(..3) else { return };
        let start = if seen.is_none() { f32::NEG_INFINITY } else { now };
        if seen.is_some() && trace() {
            println!("  sk8: {who}'s board flips: {turns} turns, {shuv} deg, {dur} s");
        }
        self.flips.insert(who.to_string(), (value.clone(), start, [turns, shuv, dur.max(0.1)]));
    }

    /// The board under `who` turned as far through its flip as `now` is: about its length
    /// (the mesh's +X) and about up, round the deck's middle, as sk8 turns the rider's own.
    fn trick(&self, who: &str, now: f32) -> Option<Mat> {
        let (_, start, [turns, shuv, dur]) = self.flips.get(who)?;
        let t = (now - start) / dur;
        if !(0.0..1.0).contains(&t) {
            return None;
        }
        let k = t * t * (3.0 - 2.0 * t);
        let lift = glam::Vec3::Y * 0.05;
        let m = glam::Mat4::from_translation(lift)
            * glam::Mat4::from_rotation_y(-(shuv * k).to_radians())
            * glam::Mat4::from_rotation_x(turns * k * std::f32::consts::TAU)
            * glam::Mat4::from_translation(-lift);
        Some(m.to_cols_array())
    }

    /// On a skateboard in a skate clip: stood on the deck, regular with the left foot to the
    /// nose or goofy turned round, as the rider's own lens draws them; not sat on a seat the
    /// board does not have.
    fn body(&self, who: &str, mesh: &str, clip: &str) -> Option<Mat> {
        if mesh != "sth_skateboard" || !clip.starts_with("skate_") {
            return None;
        }
        let stance = if self.goofy.contains(who) { glam::Mat4::from_rotation_y(std::f32::consts::PI) } else { glam::Mat4::IDENTITY };
        Some((glam::Mat4::from_translation(glam::Vec3::Y * DECK_TOP) * stance).to_cols_array())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sk8_clip_has_a_body_clip() {
        for c in sk8::frame::CLIPS {
            let b = body_clip(c);
            assert!(b.starts_with("skate_") || b == "walk", "{c} -> {b}");
        }
        assert_eq!(body_clip("grind_5050"), "skate_grind");
        assert_eq!(body_clip("push"), "skate_push");
    }

    #[test]
    fn the_board_matrix_has_its_nose_on_x() {
        let m = board_matrix(glam::Vec3::new(1.0, 2.0, 3.0), glam::Vec3::Z, glam::Vec3::Y);
        assert!((m.transform_vector3(glam::Vec3::X) - glam::Vec3::Z).length() < 1e-5);
        assert!((m.transform_vector3(glam::Vec3::Z) - glam::Vec3::NEG_X).length() < 1e-5, "right of +z is -x");
        assert!((m.transform_point3(glam::Vec3::ZERO) - glam::Vec3::new(1.0, 2.0, 3.0)).length() < 1e-5);
    }

    #[test]
    fn events_become_the_lines_the_realm_reads() {
        let e = Event::Banked { names: vec!["Kickflip".into(), "50-50".into()], points: 812.4, multiplier: 1.5 };
        assert_eq!(line_for(&e).unwrap(), "skate banked 812 1.5 Kickflip+50-50");
        assert_eq!(line_for(&Event::LineEnded { points: 99.6 }).unwrap(), "skate line 100");
        assert!(line_for(&Event::Push).is_none());
    }

    fn plattan() -> Start {
        let realm = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../world");
        // Plattan's start, as the world puts a skater who rides there.
        let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(realm.join("charts/plattan.skate.json")).unwrap()).unwrap();
        let s = &doc["spawn"];
        let at = [s[0].as_f64().unwrap() as f32, s[1].as_f64().unwrap() as f32, s[2].as_f64().unwrap() as f32];
        Start { sim: "sk8".into(), realm: realm.display().to_string(), chart: "plattan".into(), at, yaw: 0.0, attrs: Attrs::new() }
    }

    #[test]
    fn a_ride_on_plattan_pushes_off_and_tells_the_world() {
        let m = lens_mode::find(&[registration()], "sk8").copied().expect("registered as sk8");
        let mut ride = (m.ride)(&plattan()).expect("plattan has a skate file");
        let push = Input { keys: vec!["KeyW".into()], ..Input::default() };
        let mut f = ride.step(1.0 / 60.0, &push);
        for _ in 0..120 {
            f = ride.step(1.0 / 60.0, &push);
        }
        assert!(f.speed > 1.0, "two seconds of pushing rolls: {} m/s", f.speed);
        assert!(f.hide_offers, "rolling hides the offers");
        assert!(f.clip.starts_with("skate_"), "{}", f.clip);
        assert!(f.camera.is_some());
        let lines = ride.lines();
        assert!(lines.iter().any(|l| l.starts_with("skate clip ")), "{lines:?}");
        let record = ride.record();
        assert!(record.iter().any(|l| l.contains("\"on_board\"")), "{record:?}");
        assert!(ride.end().iter().any(|l| l.contains("\"off_board\"")));
    }

    #[test]
    fn riding_into_plattans_far_edge_is_a_run_out_the_phone_reports() {
        let mut ride = start(&plattan()).unwrap();
        let push = Input { keys: vec!["KeyW".into()], ..Input::default() };
        let mut events = Vec::new();
        for _ in 0..(60 * 15) {
            ride.step(1.0 / 60.0, &push);
            events.extend(ride.events());
        }
        assert!(events.iter().any(|(k, _)| k == "sk8_run_out"), "{events:?}");
    }

    #[test]
    fn a_chart_without_a_skate_file_leaves_the_lens_its_own_board() {
        let mut s = plattan();
        s.chart = "no_such_spot".into();
        assert!(start(&s).is_err());
    }

    #[test]
    fn others_flip_their_boards_and_stand_goofy() {
        let mut o = Others::default();
        let mut a = Attrs::new();
        a.insert("skate_board".into(), "1 1.00_0_0.50".into());
        a.insert("skate_stance".into(), "goofy".into());
        o.heard("Erik", &a, 0.0);
        assert!(o.trick("Erik", 0.1).is_none(), "the first value seen is old news");
        a.insert("skate_board".into(), "2 1.00_180_0.50".into());
        o.heard("Erik", &a, 10.0);
        assert!(o.trick("Erik", 10.25).is_some(), "a new flip plays");
        assert!(o.trick("Erik", 10.6).is_none(), "and ends");
        let goofy = glam::Mat4::from_cols_array(&o.body("Erik", "sth_skateboard", "skate_ride").unwrap());
        assert!((goofy.transform_vector3(glam::Vec3::X) + glam::Vec3::X).length() < 1e-5, "goofy turned round");
        assert!(o.body("Erik", "sth_scooter", "skate_ride").is_none());
        assert!(o.body("Erik", "sth_skateboard", "walk").is_none());
    }
}
