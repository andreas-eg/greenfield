//! The player state machine and physics (spec B.5 to B.13, B.16): one controlled board body
//! with wheel probes, the target-speed model, pushing, steering with grip, braking,
//! powerslide, revert, manual, kick turn, crouch and pumping, pop along the normal,
//! trajectory prediction with grind magnetism, air orientation and spin, touchdown with
//! speed preservation, landing quality, bails with the run-out rule, wipeout and respawn.
//! Grinding lives in `grind.rs`.

use crate::grind::{self, GrindState};
use crate::input::{self, Buttons, Completion, Context, Family, Gesture, Intents, Recognizer, VirtualController};
use crate::math::*;
use crate::scoring::Scoring;
use crate::tuning::{Difficulty, Tuning};
use crate::world::{Surface, World};
use serde::{Deserialize, Serialize};

pub const DT: f32 = 1.0 / 60.0;
pub const G: f32 = 9.8;
pub const GRAVITY: Vec3 = v3(0.0, -9.8, 0.0);
pub const HALF_WB: f32 = 0.243;
pub const HALF_TRACK: f32 = 0.095;
pub const WHEEL_R: f32 = 0.031;
pub const DECK_H: f32 = 0.075;
const PROBE_LIFT: f32 = 0.3;
const PROBE_LEN: f32 = 0.2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    Ground,
    Powerslide,
    Revert,
    Air,
    KnownAir,
    Grind,
    Wipeout,
    OffBoard,
    WallRide,
    Handplant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Landing {
    Clean,
    Adjust,
    Sketchy,
    Spin,
}

impl Landing {
    pub fn factor(self, side_speed: f32, spin_out: f32) -> f32 {
        match self {
            Landing::Clean => 1.1,
            Landing::Adjust => 1.0,
            Landing::Sketchy => {
                if side_speed > 1.5 && spin_out.abs() > 0.5 {
                    0.5
                } else {
                    0.8
                }
            }
            Landing::Spin => 0.75,
        }
    }
}

/// Something the host should hear about: sounds, haptics, HUD, and the lines it sends the
/// world (`skate ...` commands, beargarden/world/players/player.c).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Push,
    Pop { height: f32, strength: f32 },
    Trick { name: String },
    Land { quality: Landing, factor: f32 },
    GrindStart { name: String },
    GrindEnd { name: String, metres: f32, reason: String },
    Manual { nose: bool },
    Bail { reason: String },
    RunOut,
    Respawn,
    Banked { names: Vec<String>, points: f32, multiplier: f32 },
    LineEnded { points: f32 },
    Marker,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WheelHit {
    pub hit: bool,
    pub point: Vec3,
    pub normal: Vec3,
    pub dist: f32,
    pub surface: Option<Surface>,
}

#[derive(Clone, Copy, Debug)]
pub struct Traj {
    pub landing_point: Vec3,
    pub landing_normal: Vec3,
    /// Air time at which the landing happens (measured from the start of the air).
    pub landing_time: f32,
    pub apex_time: f32,
    pub landing_heading: Vec3,
    pub grind_chord: Option<u32>,
    pub grind_across: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Flip {
    pub gesture: Gesture,
    pub t: f32,
    pub dur: f32,
    pub flip_turns: f32,
    pub shuv_deg: f32,
    pub late: bool,
    pub dark: bool,
}

impl Flip {
    pub fn remaining_deg(&self) -> f32 {
        let left = sat(1.0 - self.t / self.dur);
        left * (self.flip_turns.abs() * 360.0).max(self.shuv_deg.abs())
    }
    pub fn done(&self) -> bool {
        self.t >= self.dur
    }
}

#[derive(Clone, Debug)]
pub struct Grab {
    pub name: String,
    pub t: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Manual {
    pub nose: bool,
    pub t: f32,
    pub slow_t: f32,
    pub over_t: f32,
    pub out_t: f32,
    pub metres: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Wipe {
    pub t: f32,
    pub body_pos: Vec3,
    pub body_vel: Vec3,
    pub recover_in: i32,
    /// Hall of Meat (B.13.1): 0 none, 1 cannonball, 2 swan dive, 3 judo kick, 4 torpedo.
    pub pose: u32,
    /// The tumbling body's turn, radians, and how fast it turns.
    pub spin: f32,
    pub spin_rate: f32,
    pub push_cool: f32,
}

/// A handplant (B.11 point 8): a curve from the wall over the coping and back.
#[derive(Clone, Copy, Debug)]
pub struct Plant {
    pub t: f32,
    pub dur: f32,
    pub p0: Vec3,
    pub c: Vec3,
    pub p2: Vec3,
    pub axis: Vec3,
    pub speed: f32,
    pub fwd0: Vec3,
    pub up0: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct Off {
    pub rider_pos: Vec3,
    pub rider_vel: Vec3,
    pub heading: Vec3,
    pub stumble: f32,
    pub vy: f32,
}

/// What the scoring needs to remember about the air in progress.
#[derive(Clone, Debug, Default)]
pub struct AirLog {
    pub pop: Option<Gesture>,
    pub flips: Vec<Gesture>,
    pub grabs: Vec<(String, f32)>,
    pub takeoff_y: f32,
    pub max_y: f32,
    pub takeoff: Vec3,
    pub from_manual: bool,
    pub from_grind: bool,
    pub stationary: bool,
    pub switch: bool,
    pub fakie: bool,
    pub body_flip: Option<Gesture>,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub pos: Vec3,
    pub vel: Vec3,
    pub fwd: Vec3,
    pub up: Vec3,
    pub yaw_rate: f32,
    pub pitch: f32,
    pub pitch_rate: f32,
    pub(crate) pitch_int: f32,
    pub(crate) pitch_prev_err: f32,
    pub wheels: [WheelHit; 4],
    pub ground_n: Vec3,
    pub rider_up: Vec3,
    pub wheels_down: u32,
    pub surface: Surface,
    pub goofy: bool,
    pub switch: bool,
    pub fakie: bool,
    pub crouch: f32,
    pub com_height: f32,
    pub(crate) prev_com: f32,
    pub(crate) rising_s: f32,
    pub(crate) ang_speed_s: f32,
    pub(crate) prev_ground_n: Vec3,
    pub heading: Vec3,
    pub spin: f32,
    pub spin_rate: f32,
    pub state: State,
    pub state_time: f32,
    pub air_time: f32,
    pub ground_time: f32,
    pub speed_target: f32,
    pub(crate) damped_turn: f32,
    pub lean: f32,
    pub(crate) push_t: f32,
    pub(crate) push_hold: f32,
    pub(crate) push_queued: bool,
    pub(crate) push_applied: bool,
    pub(crate) push_steer: f32,
    pub pushing: bool,
    pub(crate) brake_t: f32,
    pub(crate) kick: Option<(f32, Vec3, f32)>,
    pub(crate) slide: Option<(f32, f32, f32)>,
    pub(crate) post_slide: f32,
    pub wall: Option<(f32, Vec3)>,
    pub plant: Option<Plant>,
    pub(crate) slide_recover: f32,
    pub(crate) revert: Option<(f32, Vec3)>,
    pub manual: Option<Manual>,
    pub(crate) manual_band: u32,
    pub traj: Option<Traj>,
    pub(crate) jump_vel: Vec3,
    pub(crate) jump_ticks: u32,
    pub tricks_disabled: f32,
    pub flip: Option<Flip>,
    pub grab: Option<Grab>,
    pub(crate) grab_block: u32,
    pub body_flip: f32,
    pub(crate) body_flip_rate: f32,
    pub grind: Option<GrindState>,
    pub grind_block: u32,
    /// The rail a grind just fell off (leaned off, stalled, lost it): not locked onto again
    /// until a pop or the ground away from it, or a stalled ledge re-locks where it stopped.
    pub fell_off_rail: Option<u32>,
    pub anti_flicker: f32,
    pub anti_flicker_block: u32,
    pub jump_energy: f32,
    pub wipe: Option<Wipe>,
    pub off: Option<Off>,
    pub(crate) checkpoints: Vec<(Vec3, Vec3, Surface, u64)>,
    pub marker: Option<(Vec3, Vec3)>,
    pub grace: f32,
    pub(crate) no_input: f32,
    pub(crate) natural_air: f32,
    pub(crate) stairs_ticks: u32,
    pub(crate) hang_ticks: u32,
    pub air: AirLog,
    pub last_landing: f32,
    pub landing: Option<Landing>,
    pub(crate) absorb: f32,
    pub(crate) wobble_t: f32,
    pub last_bail: String,
    pub prev_carrier_ground: bool,
    pub approach_side: Vec3,
}

impl Player {
    pub fn new(pos: Vec3, heading: f32) -> Player {
        let fwd = v3(heading.sin(), 0.0, heading.cos());
        Player {
            pos,
            vel: ZERO,
            fwd,
            up: UP,
            yaw_rate: 0.0,
            pitch: 0.0,
            pitch_rate: 0.0,
            pitch_int: 0.0,
            pitch_prev_err: 0.0,
            wheels: [WheelHit::default(); 4],
            ground_n: UP,
            rider_up: UP,
            wheels_down: 0,
            surface: Surface::Smooth,
            goofy: false,
            switch: false,
            fakie: false,
            crouch: 0.05,
            com_height: 1.0,
            prev_com: 1.0,
            rising_s: 0.0,
            ang_speed_s: 0.0,
            prev_ground_n: UP,
            heading: fwd,
            spin: 0.0,
            spin_rate: 0.0,
            state: State::Ground,
            state_time: 0.0,
            air_time: 0.0,
            ground_time: 0.0,
            speed_target: 0.0,
            damped_turn: 0.0,
            lean: 0.0,
            push_t: -1.0,
            push_hold: 0.0,
            push_queued: false,
            push_applied: false,
            push_steer: 1.0,
            pushing: false,
            brake_t: 0.0,
            kick: None,
            slide: None,
            post_slide: 0.0,
            wall: None,
            plant: None,
            slide_recover: 0.0,
            revert: None,
            manual: None,
            manual_band: 0,
            traj: None,
            jump_vel: ZERO,
            jump_ticks: 0,
            tricks_disabled: 0.0,
            flip: None,
            grab: None,
            grab_block: 0,
            body_flip: 0.0,
            body_flip_rate: 0.0,
            grind: None,
            grind_block: 0,
            fell_off_rail: None,
            anti_flicker: 0.0,
            anti_flicker_block: 0,
            jump_energy: 1.0,
            wipe: None,
            off: None,
            checkpoints: Vec::new(),
            marker: None,
            grace: 0.4,
            no_input: 0.0,
            natural_air: 0.0,
            stairs_ticks: 0,
            hang_ticks: 0,
            air: AirLog::default(),
            last_landing: 1.0,
            landing: None,
            absorb: 0.0,
            wobble_t: 0.0,
            last_bail: String::new(),
            prev_carrier_ground: false,
            approach_side: ZERO,
        }
    }

    pub fn right(&self) -> Vec3 {
        self.fwd.cross(self.up).norm_or(v3(1.0, 0.0, 0.0))
    }
    /// The way the rider's chest faces (A.2): the board's right for a regular rider riding
    /// normally, its left for goofy; switch flips it.
    pub fn chest(&self) -> Vec3 {
        let s = if self.goofy != self.switch { -1.0 } else { 1.0 };
        self.right() * s
    }
    pub fn speed(&self) -> f32 {
        self.vel.len()
    }
    pub fn set_state(&mut self, s: State) {
        if self.state != s {
            self.state = s;
            self.state_time = 0.0;
        }
    }
    pub fn on_board(&self) -> bool {
        !matches!(self.state, State::Wipeout | State::OffBoard)
    }
}

/// The chase camera (B.16).
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub pos: Vec3,
    pub target: Vec3,
    pub heading: Vec3,
    pub elevation: f32,
    pub distance: f32,
    pub fov_y_deg: f32,
    anchor: Vec3,
    drop_tilt: f32,
}

impl Camera {
    fn new(p: &Player) -> Camera {
        let h = p.fwd.planar().norm_or(v3(0.0, 0.0, 1.0));
        Camera { pos: p.pos - h * 2.0 + UP * 0.9, target: p.pos, heading: h, elevation: deg(12.0), distance: 2.0, fov_y_deg: 70.0, anchor: p.pos, drop_tilt: 0.0 }
    }
}

pub struct Sim {
    pub world: World,
    pub t: Tuning,
    pub p: Player,
    pub rec: Recognizer,
    pub buttons: Buttons,
    pub score: Scoring,
    pub cam: Camera,
    pub events: Vec<Event>,
    pub tick: u64,
    pub last_input: VirtualController,
    pub intents: Intents,
    pub completion: Option<Completion>,
    pub accumulator: f32,
    pub prev_frame: Option<crate::frame::Frame>,
}

impl Sim {
    pub fn new(world: World, difficulty: Difficulty) -> Sim {
        let t = Tuning::new(difficulty);
        let mut p = Player::new(world.spawn, world.spawn_heading);
        let rec = Recognizer::new((t.flick_low, t.flick_high));
        let w = &world;
        settle(w, &mut p);
        let cam = Camera::new(&p);
        Sim {
            world,
            t,
            p,
            rec,
            buttons: Buttons::default(),
            score: Scoring::new(),
            cam,
            events: Vec::new(),
            tick: 0,
            last_input: VirtualController::default(),
            intents: Intents::default(),
            completion: None,
            accumulator: 0.0,
            prev_frame: None,
        }
    }

    pub fn set_difficulty(&mut self, d: Difficulty, overrides: &str) {
        self.set_tuning(d, overrides, "");
    }

    /// The difficulty, and a realm's `key = value` overrides with its `style`'s sections
    /// (Beargarden's `[arcade]` and `[realistic]`).
    pub fn set_tuning(&mut self, d: Difficulty, overrides: &str, style: &str) {
        self.t = Tuning::new(d);
        self.t.apply_text_styled(overrides, style);
        self.rec.window = (self.t.flick_low, self.t.flick_high);
    }

    /// Input sampled at `hz` per second: the recognizer's windows scale with it (B.4.1).
    pub fn set_sample_rate(&mut self, hz: f32) {
        self.rec.rate_scale = (hz / 60.0).max(1.0);
    }

    /// Advance by real time `dt`, stepping the fixed 1/60 tick as often as it fits (B.1).
    /// `samples` are every controller sample since the last call, oldest first.
    pub fn advance(&mut self, real_dt: f32, samples: &[VirtualController]) -> u32 {
        self.accumulator += real_dt.min(0.1);
        let mut n = 0;
        let mut rest = samples;
        while self.accumulator >= DT {
            // Hand every pending sample to the first tick; later ticks repeat the latest.
            let take = if n == 0 { rest } else { &rest[rest.len().saturating_sub(1)..] };
            self.step(take);
            rest = if n == 0 && !rest.is_empty() { &rest[rest.len() - 1..] } else { rest };
            self.accumulator -= DT;
            n += 1;
        }
        if n == 0 && !samples.is_empty() {
            // Keep the recognizer fed even on a frame with no tick.
            self.last_input = *samples.last().unwrap();
        }
        n
    }

    /// One fixed tick of 1/60 s.
    pub fn step(&mut self, samples: &[VirtualController]) {
        self.prev_frame = Some(crate::frame::Frame::of(self));
        let samples: Vec<VirtualController> = if samples.is_empty() { vec![self.last_input] } else { samples.to_vec() };
        let ctx = match self.p.state {
            State::Ground | State::Powerslide | State::Revert | State::Grind => Context::Ground,
            State::Air | State::KnownAir => Context::Air,
            _ => Context::Other,
        };
        let mut pressed = 0u32;
        let mut completion = None;
        let sdt = DT / samples.len() as f32;
        for c in &samples {
            let (pr, _) = self.buttons.update(c, sdt);
            pressed |= pr;
            let (lx, ly) = input::dead_zone(c.lx, c.ly);
            let (rx, ry) = input::dead_zone(c.rx, c.ry);
            if let Some(done) = self.rec.sample((lx, ly), (rx, ry), ctx) {
                completion = Some(done);
            }
        }
        let c = *samples.last().unwrap();
        self.last_input = c;
        let it = input::derive_intents(&c, &self.buttons, pressed, self.t.auto_push > 0.5);
        self.intents = it;
        self.completion = completion;
        self.tick += 1;
        let p = &mut self.p;
        p.state_time += DT;
        p.grace = (p.grace - DT).max(0.0);
        p.tricks_disabled = (p.tricks_disabled - DT).max(0.0);
        p.grind_block = p.grind_block.saturating_sub(1);
        p.anti_flicker = (p.anti_flicker - 1.0).max(0.0);
        p.anti_flicker_block = p.anti_flicker_block.saturating_sub(1);
        p.jump_energy = (p.jump_energy + 0.0035).min(1.0);
        if it.stick_input || it.push_held || it.crouch > 0.05 {
            p.no_input = 0.0;
        } else {
            p.no_input += DT;
        }

        if it.bail && p.on_board() {
            self.bail("bail chord");
        }
        if it.set_marker && self.p.state == State::Ground && self.p.wheels_down >= 2 && self.p.up.y > 0.71 {
            let f = if self.p.speed() > 0.5 { self.p.vel.planar().norm_or(self.p.fwd) } else { self.p.fwd };
            self.p.marker = Some((self.p.pos + UP * 0.2, f));
            self.events.push(Event::Marker);
        }
        if it.return_marker && self.p.on_board() {
            if let Some((mp, mf)) = self.p.marker {
                self.place(mp, mf);
            }
        }

        if matches!(self.p.state, State::Ground | State::Air | State::KnownAir) {
            grind::detect(self, &it);
        }
        match self.p.state {
            // Handplant on the coping (B.11 point 8), else riding.
            State::Ground => {
                if !self.try_handplant(&it) {
                    self.ground(&it, completion)
                }
            }
            State::Powerslide => self.powerslide(&it, completion),
            State::Revert => self.revert_tick(&it),
            State::Air | State::KnownAir => self.air(&it, completion),
            State::Grind => grind::grind_tick(self, &it, completion),
            State::Wipeout => self.wipeout(&it),
            State::OffBoard => self.off_board(&it),
            State::WallRide => self.wall_ride(&it, completion),
            State::Handplant => self.handplant(),
        }

        // Scoring (B.14): carriers keep a sequence open.
        let p = &self.p;
        let carrier = p.manual.is_some()
            || matches!(p.state, State::Grind | State::Powerslide | State::Revert | State::Air | State::KnownAir | State::WallRide | State::Handplant)
            || p.wheels_down < 4 && p.state == State::Ground && p.ground_time < 0.05;
        let manualing = p.manual.is_some();
        let lf = p.last_landing;
        self.score.tick(DT, carrier, manualing, lf, self.t.line_time);
        for e in self.score.events.drain(..) {
            match e {
                crate::scoring::ScoreEvent::Banked { names, multiplier, points, .. } => {
                    self.events.push(Event::Banked { names, points, multiplier })
                }
                crate::scoring::ScoreEvent::LineEnded { points } => self.events.push(Event::LineEnded { points }),
                crate::scoring::ScoreEvent::Lost { .. } => {}
            }
        }
        self.camera();
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    // ---- ground -------------------------------------------------------------------------

    fn ground(&mut self, it: &Intents, completion: Option<Completion>) {
        let t = self.t.clone();
        let w = &self.world;
        let p = &mut self.p;
        p.ground_time += DT;
        p.air_time = 0.0;
        sense(w, p, &t);
        update_rider_up(p);
        if p.wheels_down == 0 {
            p.natural_air += DT;
        }
        if p.surface == Surface::Stairs {
            p.stairs_ticks = 9;
        } else {
            p.stairs_ticks = p.stairs_ticks.saturating_sub(1);
        }
        if p.surface == Surface::Unrideable && p.grace <= 0.0 {
            self.bail("unrideable");
            return;
        }
        let tangent = orthonormal_fwd(p.fwd, p.ground_n);
        let signed = p.vel.dot(tangent);
        let surf = signed.abs();
        let mut travel = if surf > 0.05 { tangent * sign(signed) } else if p.fakie { -tangent } else { tangent };
        // Rolling tail first is fakie, whichever way the board got turned round.
        if surf > 0.3 && p.state == State::Ground {
            p.fakie = signed < 0.0;
        }
        // At rest on a slope the board rolls whichever way is downhill.
        if surf < 0.05 && tangent.y.abs() > 0.02 {
            travel = if tangent.y < 0.0 { tangent } else { -tangent };
        }

        // Pop (B.8.1): an ollie- or nollie-family gesture on the ground.
        if let Some(c) = completion {
            let g = input::apply_stance(c.gesture, p.goofy, p.switch, p.fakie);
            if matches!(c.gesture.family(), Family::Ollie | Family::Nollie)
                && p.tricks_disabled <= 0.0
                && p.stairs_ticks == 0
                && p.kick.is_none()
                && (p.wheels_down >= 2 || p.manual.is_some())
            {
                let from_manual = p.manual.is_some();
                if let Some(m) = p.manual.take() {
                    end_manual(&mut self.score, &mut self.events, p, m);
                }
                let dark = it.dark && matches!(g, Gesture::Kickflip | Gesture::Heelflip);
                let h = pop(p, &t, c.strength, from_manual, it);
                begin_air(p, g, from_manual, false, dark, &t);
                self.events.push(Event::Pop { height: h, strength: c.strength });
                p.fell_off_rail = None;
                predict(w, p, &t, true, true);
                return;
            }
            // Revert (B.7.2): left-stick half circle soon after landing or on a steep wall.
            if c.gesture.family() == Family::LeftStick && (p.ground_time < 0.3 || p.ground_n.y < deg(20.0).cos()) {
                let target = -p.vel.on_plane(p.ground_n).norm_or(-p.fwd);
                p.revert = Some((0.0, target));
                p.set_state(State::Revert);
                return;
            }
        }

        // Powerslide (B.7.1).
        if it.slide_start != 0 && surf > t.slide_min_speed && p.wheels_down == 4 && p.manual.is_none() && p.kick.is_none() {
            p.slide = Some((0.0, it.slide_start as f32, 0.0));
            p.set_state(State::Powerslide);
            return;
        }

        // Kick turn (B.6.4).
        if p.kick.is_none() && surf < 1.5 && it.kick_turn.abs() > 0.5 && p.manual.is_none() && p.wheels_down >= 2 {
            p.kick = Some((0.0, p.fwd, it.kick_turn));
        }
        if let Some((kt, start, amt)) = p.kick {
            let kt = kt + DT;
            let s = smoothstep(kt / 0.5);
            let ang = -amt * std::f32::consts::PI * s;
            p.fwd = orthonormal_fwd(rotate(start, p.up, ang), p.up);
            p.pitch = deg(12.0) * if kt < 0.15 { kt / 0.15 } else { sat((0.5 - kt) / 0.35) };
            p.yaw_rate = 0.0;
            if kt >= 0.5 {
                p.kick = None;
                p.pitch = 0.0;
                if p.vel.len() > 0.2 && p.vel.dot(p.fwd) < 0.0 {
                    p.fakie = !p.fakie;
                    p.fwd = -p.fwd;
                }
            } else {
                p.kick = Some((kt, start, amt));
            }
            p.speed_target = surf;
            // Coast with the speed frozen while the board pivots.
            integrate_ground(w, p, &t);
            return;
        }

        // Manual (B.7.3).
        let m = it.manual;
        let in_band = (0.45..=0.92).contains(&m.abs());
        if p.manual.is_none() {
            if in_band && p.wheels_down == 4 && surf > 0.8 && completion.is_none() {
                p.manual_band += 1;
                if p.manual_band >= 4 {
                    p.manual = Some(Manual { nose: m > 0.0, t: 0.0, slow_t: 0.0, over_t: 0.0, out_t: 0.0, metres: 0.0 });
                    p.pitch_int = 0.0;
                    p.pitch_prev_err = 0.0;
                    self.events.push(Event::Manual { nose: m > 0.0 });
                }
            } else {
                p.manual_band = 0;
            }
        }
        if let Some(mut man) = p.manual {
            man.t += DT;
            man.metres += surf * DT;
            let sgn = if man.nose { 1.0 } else { -1.0 };
            let max_tilt = deg(t.manual_max_tilt_deg);
            let noise_amp = deg(t.manual_noise_deg) * if surf < 3.0 { 1.0 } else { lerp(1.0, 0.5, sat((surf - 3.0) / 5.0)) };
            let held = if m.abs() > 0.05 && sign(m) == sgn { m.abs() } else { 0.0 };
            let mut target = if held > 0.0 { sgn * max_tilt * (0.25 + 0.75 * held.min(1.0)) } else { 0.0 };
            target += noise_amp * (man.t * 0.7 * 2.0 * std::f32::consts::PI).sin();
            let err = target - p.pitch;
            p.pitch_int = p.pitch_int * 0.95 + err * 0.05;
            let deriv = clamp((err - p.pitch_prev_err) / DT, -20.0, 20.0);
            p.pitch_prev_err = err;
            p.pitch_rate += (err * 60.0 + p.pitch_int * 8.0 + deriv * 1.5) * DT;
            p.pitch_rate *= 0.85;
            p.pitch += p.pitch_rate * DT;
            let mut end = false;
            if held < 0.05 {
                man.out_t += DT;
                if man.out_t >= 0.166 {
                    end = true;
                }
            } else {
                man.out_t = 0.0;
            }
            if p.pitch.abs() > max_tilt + deg(10.0) && surf > 2.0 {
                man.over_t += DT;
                if man.over_t > 0.25 {
                    if t.difficulty == Difficulty::Hardcore {
                        self.bail("manual over");
                        return;
                    }
                    end = true;
                }
            } else {
                man.over_t = 0.0;
            }
            if surf < 1.2 {
                man.slow_t += DT;
                if man.slow_t > 1.0 {
                    self.bail("manual too slow");
                    return;
                }
            }
            // Tail brake past 0.9.
            if it.manual_brake > 0.0 {
                let a = 2.5 * it.manual_brake;
                p.vel -= travel * (a * DT).min(surf);
                p.speed_target = p.vel.dot(travel).max(0.0);
            }
            if end || p.wheels_down < 2 {
                p.manual = None;
                p.pitch = 0.0;
                end_manual(&mut self.score, &mut self.events, p, man);
            } else {
                p.manual = Some(man);
            }
        } else if p.kick.is_none() {
            p.pitch *= 0.8;
        }
        let manualing = p.manual.is_some();

        // Speed model (B.6.1).
        let grav = clamp(-travel.y * G, -t.max_grav_accel, t.max_grav_accel) * DT;
        let mut fric = -(t.rolling_friction_base + t.rolling_friction_per_speed * surf + t.rolling_drag * surf * surf)
            * p.surface.friction()
            * DT;
        if !p.pushing && p.no_input > 1.5 {
            fric *= 1.5;
        }
        if manualing {
            fric *= 2.2;
        }
        let delta = if grav > 0.0 { fric + grav } else { fric.min(grav) };
        p.speed_target += delta;
        p.speed_target = clamp(p.speed_target, 0.0, surf + t.max_speed_change);
        p.speed_target = p.speed_target.max(surf - t.max_speed_change);
        let err = p.speed_target - surf;
        let gain = if err > 0.0 { 0.5 } else { 0.8 };
        let dv = gain * clamp(err, -t.max_speed_change, t.max_speed_change);
        // Rolling backwards uphill past zero is handled by `travel` flipping at rest.
        p.vel += travel * dv;

        // Pushing (B.6.2).
        let can_push = p.wheels_down >= 2 && !manualing && !it.brake;
        let ts = self.buttons.since_release(input::btn::A).min(self.buttons.since_release(input::btn::X));
        if it.push_new {
            p.push_queued = true;
        }
        if p.push_t >= 0.0 {
            p.push_t += DT;
            if it.push_held {
                p.push_hold += DT;
            }
            if !p.push_applied && p.push_t >= t.push_contact && can_push {
                p.push_applied = true;
                let along = p.vel.dot(travel).max(0.0);
                let dv_limit = lerp(t.push_dv_rest, t.push_dv_max, sat(along / t.max_pushable_speed));
                let hold = clamp(0.6 + p.push_hold / 0.3 * 0.4, 0.6, 1.0);
                let dv = clamp(dv_limit * hold, 0.0, (t.max_pushable_speed - along).max(0.0));
                let dir = if surf < 0.05 { if p.fakie { -tangent } else { tangent } } else { travel };
                p.vel += dir * dv;
                p.speed_target = p.vel.dot(dir).abs();
            }
            let quick = p.push_queued && ts < 0.3;
            if p.push_t >= t.push_clip || (quick && p.push_t >= 0.45) {
                p.push_t = -1.0;
                p.pushing = false;
            }
        }
        if p.push_t < 0.0 && can_push && (p.push_queued || it.push_held) {
            p.push_t = 0.0;
            p.push_hold = 0.0;
            p.push_applied = false;
            p.push_queued = false;
            p.pushing = true;
            self.events.push(Event::Push);
        }
        if p.push_t > 0.1 && p.push_t < 0.5 {
            p.push_steer = (p.push_steer - 0.08).max(0.4);
        } else {
            p.push_steer = (p.push_steer + 0.04).min(1.0);
        }

        // Foot brake (B.6.5).
        if it.brake && !manualing {
            p.brake_t += DT;
            p.push_t = -1.0;
            p.pushing = false;
            if p.brake_t > 0.12 {
                let a = t.brake_decel * p.surface.brake();
                let along = p.vel.dot(travel);
                p.vel -= travel * (a * DT).min(along.max(0.0));
                if p.vel.on_plane(p.ground_n).len() < 0.6 {
                    p.vel = p.ground_n * p.vel.dot(p.ground_n);
                    p.speed_target = 0.0;
                } else {
                    p.speed_target = p.vel.dot(travel).abs();
                }
            }
        } else {
            p.brake_t = 0.0;
        }

        // Steering (B.6.3).
        p.damped_turn = lerp(p.damped_turn, it.turn, 0.2);
        if it.hard_turn != 0.0 {
            p.damped_turn = sign(it.hard_turn);
        }
        let x = p.damped_turn.abs();
        let steer_in = x * x * (3.0 - 2.0 * x);
        let steer_speed = clamp(1.0 - 0.06 * surf, 0.35, 1.0);
        let mut tilt = steer_in * steer_speed * (1.0 + 0.5 * it.hard_turn.abs()) * p.push_steer * sign(p.damped_turn);
        // Speed wobble.
        if surf > t.wobble_speed {
            p.wobble_t += DT;
            let k = sat((surf - t.wobble_speed) / 6.0);
            let amp = t.wobble_amp * k * if p.crouch > 0.5 { 0.5 } else { 1.0 };
            let f = lerp(2.0, 5.0, k);
            tilt += amp * (2.0 * std::f32::consts::PI * f * p.wobble_t).sin();
        }
        let manual_spin = if manualing { it.lx * deg(120.0) } else { 0.0 };
        let mut yaw_target = if manualing { manual_spin } else { tilt * t.yaw_rate_max };
        // Straighten out toward the velocity; the stick overrides it.
        if surf > 0.3 {
            let ref_dir = if p.vel.dot(p.fwd) >= 0.0 { p.fwd } else { -p.fwd };
            let ang = -signed_angle(ref_dir, p.vel, p.up);
            let gain = t.straighten_gain * if p.post_slide > 0.0 { 0.16 } else { 1.0 } * (1.0 - it.turn.abs());
            yaw_target += ang * gain;
        }
        p.post_slide = (p.post_slide - DT).max(0.0);
        // Slope torque: a board left alone on a steep slope turns downhill.
        let slope = p.ground_n.y.min(1.0).acos();
        if slope > deg(8.0) && it.turn.abs() < 0.1 && surf > 0.5 {
            let downhill = (-UP).on_plane(p.ground_n).norm();
            // Toward the fall line, whichever way along it is nearer: a board climbing a
            // bank straightens up it, it does not turn round.
            let mut ang = -signed_angle(travel, downhill, p.up);
            if ang.abs() > std::f32::consts::FRAC_PI_2 {
                ang -= std::f32::consts::PI * sign(ang);
            }
            yaw_target += ang * 0.8;
        }
        if p.wheels_down >= 2 && p.slide_recover <= 0.0 {
            p.yaw_rate = lerp(p.yaw_rate, yaw_target, 0.25);
        }
        let lean_target = clamp(tilt, -1.0, 1.0) * deg(25.0);
        p.lean += clamp(lean_target - p.lean, -deg(4.0), deg(4.0));

        // Coming out of a powerslide (B.7.1 exit): the board swings back onto the velocity's
        // nearest axis while grip ramps back over 0.3 s.
        let mut grip_scale = 1.0;
        if p.slide_recover > 0.0 {
            p.slide_recover = (p.slide_recover - DT).max(0.0);
            grip_scale = 1.0 - p.slide_recover / 0.3;
            let vd = p.vel.on_plane(p.up);
            if vd.len() > 0.3 {
                let ax = if vd.dot(p.fwd) >= 0.0 { p.fwd } else { -p.fwd };
                let err = -signed_angle(ax, vd, p.up);
                p.yaw_rate = err * 9.0;
            }
        }
        // Lateral grip.
        let right = tangent.cross(p.ground_n).norm_or(p.right());
        let side = p.vel.dot(right);
        let along = p.vel.dot(tangent);
        let grip = t.grip_base * p.surface.grip() * (1.0 - 0.6 * sat(side.abs() / 4.0)) * grip_scale;
        let k = sat(grip * DT);
        let new_side = side * (1.0 - k);
        let conserve = if p.slide_recover > 0.0 { 1.0 } else { 1.0 - sat(side.abs() / 3.0) };
        let mag2 = along * along + side * side - new_side * new_side;
        let new_along = lerp(along.abs(), mag2.max(0.0).sqrt(), conserve) * sign(along);
        p.vel += right * (new_side - side) + tangent * (new_along - along);

        // Crouch and pumping (B.7.4, B.7.5).
        crouch_tick(p, it, manualing);
        pump(p, &t, it);

        // Bumps and hang-ups (B.6.6).
        if p.speed() < 1.1 && p.up.y < 0.9 && p.wheels_down < 4 {
            p.hang_ticks += 1;
            if p.hang_ticks == 20 {
                let away = p.ground_n.planar().norm_or(-p.fwd);
                p.vel += away * 0.6 + UP * 2.0;
            }
            if p.hang_ticks > 40 {
                p.hang_ticks = 0;
                self.bail("hung up");
                return;
            }
        } else {
            p.hang_ticks = 0;
        }

        // Checkpoints for respawn (B.13.2).
        if p.ground_time > 15.0 * DT && self.tick % 20 == 0 && p.wheels_down >= 2 && p.surface.respawn_ok() {
            let far = p.checkpoints.last().map_or(true, |c| (c.0 - p.pos).len() > 1.5);
            if far {
                p.checkpoints.push((p.pos, p.fwd, p.surface, self.tick));
                if p.checkpoints.len() > 32 {
                    p.checkpoints.remove(0);
                }
            }
        }

        let left = integrate_ground(w, p, &t);
        if let Some(reason) = wall_contacts(w, p, &t) {
            self.bail(&reason);
            return;
        }
        if left {
            // Natural air (B.5.5).
            let p = &mut self.p;
            if let Some(m) = p.manual.take() {
                end_manual(&mut self.score, &mut self.events, p, m);
            }
            if p.ground_n.y < 0.35 {
                let n = p.ground_n;
                let vn = p.vel.dot(n);
                p.vel -= n * (vn * 0.15);
            }
            begin_air(p, Gesture::Ollie, false, false, false, &t);
            p.air.pop = None;
            predict(&self.world, p, &t, false, true);
        }
    }

    fn powerslide(&mut self, it: &Intents, completion: Option<Completion>) {
        let t = self.t.clone();
        let w = &self.world;
        let p = &mut self.p;
        sense(w, p, &t);
        update_rider_up(p);
        let (mut st, dir, mut no_slip) = p.slide.unwrap_or((0.0, 1.0, 0.0));
        st += DT;
        let amount = sat(it.slide.abs());
        let blend = sat(st / 0.5);
        let desired = dir * lerp(deg(60.0), deg(120.0), amount) * blend;
        let pv = p.vel.on_plane(p.ground_n);
        let speed = pv.len();
        let vdir = pv.norm_or(p.fwd);
        // Right slide: the board's nose swings right of the velocity (negative about up).
        let target_fwd = rotate(vdir, p.up, -desired);
        let err = -signed_angle(p.fwd, target_fwd, p.up);
        p.yaw_rate = err * 9.0 - p.yaw_rate * 0.3;
        p.fwd = orthonormal_fwd(rotate(p.fwd, p.up, -p.yaw_rate * DT), p.up);
        let slip = angle_between(p.fwd, vdir).min(angle_between(-p.fwd, vdir));
        let scrub = (t.slide_scrub_base + t.slide_scrub_side * slip.sin()) * p.surface.slide();
        if speed > 1e-3 {
            p.vel -= vdir * (scrub * DT).min(speed);
        }
        if slip < deg(5.0) {
            no_slip += DT;
        } else {
            no_slip = 0.0;
        }
        p.slide = Some((st, dir, no_slip));
        crouch_tick(p, it, false);
        let left = integrate_ground(w, p, &t);
        let in_zone = it.slide_start != 0 || it.slide.abs() > 0.3;
        let speed = p.vel.on_plane(p.ground_n).len();
        if (!in_zone && st > 0.25) || speed < 0.8 || no_slip > 0.3 || left {
            let metres = st * speed;
            p.slide = None;
            if p.vel.dot(p.fwd) < 0.0 && speed > 0.3 {
                p.fakie = !p.fakie;
            }
            p.post_slide = 0.5;
            p.slide_recover = 0.3;
            p.speed_target = speed;
            p.set_state(State::Ground);
            if st > 0.3 {
                self.score.add_value("Powerslide", "Powerslide", metres, 0.0, 1.0);
            }
        }
        let _ = completion;
    }

    fn revert_tick(&mut self, it: &Intents) {
        let t = self.t.clone();
        let w = &self.world;
        let p = &mut self.p;
        sense(w, p, &t);
        update_rider_up(p);
        let (mut rt, target) = p.revert.unwrap_or((0.0, -p.fwd));
        rt += DT;
        let err = -signed_angle(p.fwd, target, p.up);
        let rate = clamp(err * 20.0, -deg(720.0), deg(720.0));
        p.fwd = orthonormal_fwd(rotate(p.fwd, p.up, -rate * DT), p.up);
        p.revert = Some((rt, target));
        integrate_ground(w, p, &t);
        crouch_tick(p, it, false);
        if err.abs() < deg(10.0) || rt > 0.45 {
            p.revert = None;
            p.fakie = !p.fakie;
            p.set_state(State::Ground);
            self.score.add_value("Revert", "Revert", 0.0, 0.0, 1.0);
        }
    }

    // ---- air ----------------------------------------------------------------------------

    fn air(&mut self, it: &Intents, completion: Option<Completion>) {
        let t = self.t.clone();
        let p = &mut self.p;
        p.air_time += DT;
        p.pitch *= 0.8;
        p.air.max_y = p.air.max_y.max(p.pos.y);
        // Late flips, body flips (B.8.6, B.8.5).
        if let Some(c) = completion {
            let g = input::apply_stance(c.gesture, p.goofy, p.switch, p.fakie);
            let ttl = p.traj.map_or(1.0, |tr| tr.landing_time - p.air_time);
            match c.gesture {
                Gesture::LateKickflip | Gesture::LateHeelflip | Gesture::LateShuvit | Gesture::LateFsShuvit => {
                    if p.air_time > 0.1 && ttl > t.flip_time * 0.6 && p.flip.as_ref().map_or(true, |f| f.done()) && p.tricks_disabled <= 0.0 {
                        start_flip(p, g, true, false, &t);
                        p.air.flips.push(g);
                        self.events.push(Event::Trick { name: g.trick_name().to_string() });
                    }
                }
                Gesture::FrontFlip | Gesture::BackFlip => {
                    if p.grab.is_some() && ttl > 0.6 && p.air.body_flip.is_none() {
                        p.air.body_flip = Some(c.gesture);
                        let dir = if c.gesture == Gesture::FrontFlip { 1.0 } else { -1.0 };
                        p.body_flip_rate = dir * 2.4 * std::f32::consts::PI / ttl;
                    }
                }
                _ => {}
            }
        }
        if let Some(f) = p.flip.as_mut() {
            f.t += DT;
        }
        if p.body_flip_rate != 0.0 {
            p.body_flip += p.body_flip_rate * DT;
            if p.body_flip.abs() >= 2.0 * std::f32::consts::PI {
                p.body_flip = 0.0;
                p.body_flip_rate = 0.0;
            }
        }
        // Grabs (B.8.5).
        if it.left_grab || it.right_grab {
            if p.grab.is_none() {
                p.grab = Some(Grab { name: grab_name(p, it), t: 0.0 });
                p.grab_block = 3;
            } else if let Some(g) = p.grab.as_mut() {
                g.t += DT;
            }
        } else if let Some(g) = p.grab.take() {
            if g.t > 0.15 {
                p.air.grabs.push((g.name, g.t));
            }
        }
        // Body spin.
        let spin_in = if p.grab_block > 0 { 0.0 } else { it.body_spin };
        p.grab_block = p.grab_block.saturating_sub(1);
        let max_spin = deg(t.max_spin_deg);
        let input_target = spin_in * max_spin * sat(p.air_time * 2.0);
        let mut target = input_target;
        if let Some(tr) = p.traj {
            let ttl = tr.landing_time - p.air_time;
            if spin_in.abs() < 0.1 && ttl > 0.05 {
                let mut err = -signed_angle(p.fwd.on_plane(p.rider_up), tr.landing_heading, p.rider_up);
                if err.abs() > deg(90.0) {
                    err = -signed_angle((-p.fwd).on_plane(p.rider_up), tr.landing_heading, p.rider_up);
                }
                let mut auto = err / (ttl - 0.032).max(0.032) * 1.2;
                let ma = deg(t.max_auto_spin_deg);
                auto = clamp(auto, -ma, ma);
                if auto * p.spin_rate < 0.0 && p.spin_rate.abs() > deg(30.0) {
                    auto *= 0.005;
                }
                target = auto;
            }
        }
        let reversing = target * p.spin_rate < 0.0;
        let acc = deg(if reversing { 900.0 } else { t.spin_accel_deg });
        p.spin_rate = move_toward(p.spin_rate, target, acc * DT);

        // Orientation (B.8.4).
        if let Some(tr) = p.traj {
            let ttl = tr.landing_time - p.air_time;
            let remaining = ((ttl * 60.0).round() as i32).max(1);
            let wgt = 1.0 / ((remaining - 3).max(2) as f32);
            p.rider_up = rotate_toward(p.rider_up, tr.landing_normal, deg(8.0), wgt);
        } else if angle_between(p.rider_up, UP) < deg(60.0) {
            p.rider_up = rotate_toward(p.rider_up, UP, deg(3.0), 1.0);
        }
        p.up = p.rider_up;
        p.heading = orthonormal_fwd(rotate(p.heading, p.rider_up, -p.spin_rate * DT), p.rider_up);
        p.fwd = p.heading;
        p.spin += p.spin_rate * DT;

        // Integrate (B.5.6) with the jump hold.
        let prev = p.pos;
        p.vel += GRAVITY * DT;
        if p.jump_ticks > 0 {
            p.jump_ticks -= 1;
            let k = (12 - p.jump_ticks) as f32 * DT;
            let want = p.jump_vel + GRAVITY * k;
            let e = p.vel - want;
            if e.y > -3.0 && e.planar().len() < 3.0 {
                p.vel = want;
            } else {
                p.jump_ticks = 0;
            }
        }
        p.pos += p.vel * DT;

        // Prediction refresh (B.8.3).
        if p.state_time > 0.0 && (self.tick % 7 == 0) {
            // Every run of the prediction may bend the path a little (B.8.2), which is how
            // the air finds a rail it was not aimed at when it popped.
            let popped = p.air.pop.is_some();
            predict(&self.world, p, &t, popped, true);
        }
        if let (State::KnownAir, Some(tr)) = (p.state, p.traj) {
            if tr.landing_time - p.air_time < -0.2 {
                p.traj = None;
                p.set_state(State::Air);
            }
        }
        // Legs.
        let ttl = p.traj.map_or(1.0, |tr| tr.landing_time - p.air_time);
        p.com_height = if ttl < 0.25 { lerp(p.com_height, 1.0, 0.2) } else { lerp(p.com_height, 0.75, 0.1) };
        p.crouch = sat((1.0 - p.com_height) / 0.38);

        // Off the edge of the spot: nothing below will ever catch it (a kill volume, B.2.1).
        if p.pos.y < self.world.floor_y() - 3.0 {
            self.bail("off the spot");
            return;
        }
        // Respawn if the air goes on and on (C.1: 5 s).
        if p.air_time > 5.0 {
            self.bail("lost in the air");
            return;
        }

        // Touchdown (B.9.1).
        let settle_ok = if p.state == State::KnownAir { p.state_time > 0.04 } else { p.air_time > 0.08 };
        let w = &self.world;
        let mut contact: Option<(Vec3, Surface, f32)> = None;
        if settle_ok {
            // The contact point: the wheels' bottom under the deck centre.
            let c0 = prev - p.up * DECK_H;
            let c1 = p.pos - p.up * DECK_H;
            let d = c1 - c0;
            if d.len() > 1e-5 {
                // Under the wheels: only a surface the board's underside faces. Sliding down
                // a vert wall past the lip, this point is behind the board and must not land
                // on the deck beside the coping.
                if let Some(h) = w.raycast(c0, d.norm(), d.len() + 0.01) {
                    if h.normal.dot(p.vel) < 0.0 && h.normal.dot(p.up) > 0.5 {
                        p.pos = h.point + p.up * DECK_H + h.normal * 0.005;
                        contact = Some((h.normal, h.surface, 0.0));
                    }
                }
            }
            // The deck centre itself reaching a surface is a touchdown whatever the board's
            // angle (and a bail when it is on its side).
            let dc = p.pos - prev;
            if contact.is_none() && dc.len() > 1e-5 {
                if let Some(h) = w.raycast(prev, dc.norm(), dc.len() + 0.01) {
                    if h.normal.dot(p.vel) < 0.0 {
                        p.pos = h.point + h.normal * 0.01;
                        contact = Some((h.normal, h.surface, 0.0));
                    }
                }
            }
            if contact.is_none() {
                let wh = probe(w, p);
                let mut n = ZERO;
                let mut cnt = 0;
                let mut surf = Surface::Smooth;
                for h in wh.iter() {
                    if h.hit && h.dist <= 0.01 && h.normal.dot(p.vel) < 0.0 {
                        n += h.normal;
                        cnt += 1;
                        surf = h.surface.unwrap_or(Surface::Smooth);
                    }
                }
                if cnt > 0 {
                    contact = Some((n.norm(), surf, 0.0));
                }
            }
        }
        if let Some((n, surface, _)) = contact {
            self.touchdown(n, surface);
            return;
        }
        // A wall met at a shallow angle with speed along it is ridden (B.6.6 wall ride).
        if self.try_wall_ride() {
            return;
        }
        // Hitting something with the body in the air.
        if let Some(reason) = wall_contacts(&self.world, &mut self.p, &t) {
            self.bail(&reason);
        }
    }

    /// B.6.6: a surface with |normal.y| < 0.5 met in the air, the board travelling along it
    /// faster than into it, is ridden. The spec asks for the board already turned to the wall
    /// (dot(up, normal) > 0.71); here meeting it at a shallow angle turns it, which is how a
    /// wallride is set up on a pad.
    fn try_wall_ride(&mut self) -> bool {
        let p = &mut self.p;
        if p.air_time < 0.15 || p.grab.is_some() || p.flip.map_or(false, |f| !f.done()) {
            return false;
        }
        // An air the prediction aimed at a rail goes to the rail.
        if p.traj.map_or(false, |t| t.grind_chord.is_some()) {
            return false;
        }
        let mut cs = Vec::new();
        self.world.sphere_contacts(p.pos + p.rider_up * 0.15, 0.3, &mut cs);
        for (_, depth, surf, tn) in cs {
            if tn.y.abs() >= 0.5 || surf == Surface::Unrideable {
                continue;
            }
            let along = p.vel.on_plane(tn);
            let into = -p.vel.dot(tn);
            // A wall, not a ledge's side: it must stand above the rider's waist here.
            let tall = self.world.raycast(p.pos + UP * 0.8 + tn * 0.4, -tn, 0.7).map_or(false, |h| h.normal.dot(tn) > 0.8);
            if tall && along.len() > 2.5 && into >= 0.0 && into < along.len() {
                p.pos += tn * (depth + 0.02);
                // Wheels to the wall: the deck centre DECK_H off its face.
                if let Some(h) = self.world.raycast(p.pos + tn * 0.5, -tn, 1.5) {
                    p.pos = h.point + tn * (DECK_H + 0.01) + (p.pos - h.point).on_plane(tn);
                }
                p.vel = along;
                p.up = tn;
                p.rider_up = rotate_toward(p.rider_up, tn, deg(60.0), 1.0);
                p.fwd = orthonormal_fwd(along, tn);
                p.wall = Some((0.0, tn));
                p.traj = None;
                p.set_state(State::WallRide);
                self.events.push(Event::Trick { name: "Wallride".into() });
                return true;
            }
        }
        false
    }

    fn wall_ride(&mut self, _it: &Intents, completion: Option<Completion>) {
        let t = self.t.clone();
        let p = &mut self.p;
        let Some((wt, n)) = p.wall else {
            p.set_state(State::Air);
            return;
        };
        let wt = wt + DT;
        // Gravity is cancelled at first and less so as the 0.8 s run out.
        let anti = G * sat(1.0 - wt / 0.8);
        p.vel += (GRAVITY + UP * anti) * DT;
        p.vel = p.vel.on_plane(n);
        p.pos += p.vel * DT;
        p.fwd = orthonormal_fwd(p.vel.norm_or(p.fwd), n);
        p.up = n;
        let metres = p.vel.len() * DT;
        // Still on the wall?
        let on = self.world.raycast(p.pos + n * 0.3, -n, 0.3 + DECK_H + 0.15).map_or(false, |h| h.normal.dot(n) > 0.8);
        // The ground coming up under it ends the ride.
        let floor = p.vel.y < 0.0 && self.world.raycast(p.pos + UP * 0.1, -UP, 0.35).map_or(false, |h| h.normal.y > 0.7);
        let pop = completion.map_or(false, |c| matches!(c.gesture.family(), Family::Ollie | Family::Nollie));
        if wt >= 0.8 || !on || pop || floor {
            p.wall = None;
            p.vel += n * if pop { 2.5 } else { 2.0 } + if pop { UP * 2.5 } else { ZERO };
            p.rider_up = UP.lerp(p.rider_up, 0.3).norm();
            begin_air(p, Gesture::Ollie, false, false, false, &t);
            p.air.pop = None;
            let dist = wt * p.vel.len();
            self.score.add_value("Wallride", "Wallride", dist, 0.0, 1.0);
            predict(&self.world, &mut self.p, &t, false, true);
            let _ = metres;
            return;
        }
        p.wall = Some((wt, n));
    }

    /// B.11 point 8: holding RB going up a steep wall at speed plants a hand on the highest
    /// coping in reach; the board goes over the apex and back into the wall in 0.9 s.
    fn try_handplant(&mut self, it: &Intents) -> bool {
        let p = &self.p;
        if !it.grab_world || p.vel.y <= 0.0 || p.speed() <= 3.0 || p.ground_n.y > deg(50.0).cos() {
            return false;
        }
        let lo = p.pos - v3(2.0, 0.5, 2.0);
        let hi = p.pos + v3(2.0, 4.0, 2.0);
        let mut best: Option<Vec3> = None;
        for id in self.world.rails_in_box(lo, hi) {
            let c = &self.world.chords[id as usize];
            if c.kind != crate::world::RailKind::Coping {
                continue;
            }
            let s = clamp((p.pos - c.a).dot(c.dir), 0.0, c.len);
            let q = c.a + c.dir * s;
            if q.y > p.pos.y && best.map_or(true, |b| q.y > b.y) {
                best = Some(q);
            }
        }
        let Some(cop) = best else { return false };
        let inward = p.ground_n.planar().norm_or(-p.fwd.planar().norm());
        let apex = cop + UP * 0.76 - inward * 0.24;
        let p0 = p.pos;
        let p2 = p.pos + UP * 0.1;
        let c = apex * 2.0 - (p0 + p2) * 0.5;
        let axis = UP.cross(inward).norm_or(p.right());
        let plant = Plant { t: 0.0, dur: 0.9, p0, c, p2, axis, speed: p.speed(), fwd0: p.fwd, up0: p.up };
        let p = &mut self.p;
        p.plant = Some(plant);
        p.manual = None;
        p.set_state(State::Handplant);
        self.events.push(Event::Trick { name: "Handplant".into() });
        true
    }

    fn handplant(&mut self) {
        let t = self.t.clone();
        let p = &mut self.p;
        let Some(mut pl) = p.plant else {
            p.set_state(State::Air);
            return;
        };
        pl.t += DT;
        let s = sat(pl.t / pl.dur);
        let a = 1.0 - s;
        p.pos = pl.p0 * (a * a) + pl.c * (2.0 * a * s) + pl.p2 * (s * s);
        // The board goes over with the body, all the way round about the coping's axis, and
        // comes back wheels to the wall: down tail first, which the landing calls fakie.
        let ang = std::f32::consts::TAU * smoothstep(s);
        p.up = rotate(pl.up0, pl.axis, ang);
        p.fwd = orthonormal_fwd(rotate(pl.fwd0, pl.axis, ang), p.up);
        p.rider_up = p.up;
        if s >= 1.0 {
            p.plant = None;
            // Back into the wall going down, fakie, with most of the speed.
            let tangent = (pl.p2 - pl.c).norm_or(-UP);
            p.vel = tangent * (pl.speed * 0.85);
            begin_air(p, Gesture::Ollie, false, false, false, &t);
            p.air.pop = None;
            p.air_time = 0.1;
            self.score.add_value("Handplant", "Handplant", 0.0, pl.dur, 1.0);
            predict(&self.world, &mut self.p, &t, false, true);
            return;
        }
        p.plant = Some(pl);
    }

    fn touchdown(&mut self, n: Vec3, surface: Surface) {
        let t = self.t.clone();
        let p = &mut self.p;
        let vel_before = p.vel;
        // Falling upside down, board on its side, flips not caught (B.9.3).
        let dark = p.flip.map_or(false, |f| f.dark);
        let mut reason: Option<&str> = None;
        if p.up.dot(n) < 0.5 && !dark {
            reason = Some("landing primo");
        }
        if surface == Surface::Unrideable {
            reason = Some("unrideable");
        }
        let into = -vel_before.dot(n);
        let impact = if surface == Surface::Stairs { t.impact_bail.min(5.5) } else { t.impact_bail };
        if into > impact {
            reason = Some("impact");
        }
        if let Some(f) = p.flip {
            if f.remaining_deg() > t.flip_catch_deg {
                reason = Some("trick not caught");
            }
        }
        if p.body_flip.abs() > deg(60.0) && p.body_flip.abs() < deg(300.0) {
            reason = Some("flip");
        }
        // Speed preservation.
        let tang = vel_before.on_plane(n);
        let scale = if n.y >= 0.5 { 1.0 } else { lerp(1.0, 1.05, sat((0.5 - n.y) / 0.5)) };
        p.vel = tang * scale;
        let speed = p.vel.len();
        // Landing angle.
        let vh = p.vel.on_plane(n).norm_or(p.fwd);
        let fh = p.fwd.on_plane(n).norm_or(p.fwd);
        let ang = angle_between(fh, vh).min(angle_between(-fh, vh));
        let allowed = deg(lerp(t.land_angle_rest_deg, t.land_angle_fast_deg, sat(speed / 8.0)));
        if ang > allowed && speed > 1.0 {
            reason = reason.or(Some("landing angle"));
        }
        if p.grace > 0.0 {
            reason = None;
        }
        p.up = n;
        p.rider_up = rotate_toward(p.rider_up, n, deg(30.0), 1.0);
        p.ground_n = n;
        p.prev_ground_n = n;
        p.fwd = orthonormal_fwd(p.fwd, n);
        if let Some(r) = reason {
            self.bail(r);
            return;
        }
        // Landing quality (B.9.2).
        let right = p.right();
        let side_spd = p.vel.dot(right).abs();
        let spin_rate = p.spin_rate;
        let mut kind = Landing::Clean;
        let mut spin_out = 0.0;
        if speed >= 2.0 {
            let vhat = p.vel.planar().norm_or(p.fwd);
            let fhat = p.fwd.planar().norm_or(p.fwd);
            let fh2 = if fhat.dot(vhat) >= 0.0 { fhat } else { -fhat };
            let orient = fh2.cross(vhat).y * if p.switch { -1.0 } else { 1.0 };
            let spin_in = sat((spin_rate.abs() - 0.1) / 6.9);
            let side_in = sat((side_spd - 0.1) / 9.9);
            let away = spin_rate.abs() > 0.1 && sign(-spin_rate) != sign(orient);
            kind = if spin_rate.abs() <= 0.1 { Landing::Sketchy } else if away { Landing::Spin } else { Landing::Sketchy };
            spin_out = sign(spin_rate) * spin_in.max(side_in);
            if orient.abs() < 0.2 && (kind != Landing::Spin || spin_out.abs() <= 0.3) {
                kind = if orient.abs() >= 0.05 { Landing::Adjust } else { Landing::Clean };
                spin_out = 0.0;
            }
        }
        if let Some(f) = p.flip {
            if !f.done() && kind == Landing::Clean {
                kind = Landing::Sketchy;
            }
        }
        let factor = kind.factor(side_spd, spin_out);
        p.last_landing = factor;
        p.landing = Some(kind);
        p.fakie = p.vel.dot(p.fwd) < 0.0 && speed > 0.3;
        p.absorb = sat(into * 0.3).min(0.7);
        p.crouch = p.crouch.max(p.absorb);
        p.speed_target = speed;
        p.ground_time = 0.0;
        p.tricks_disabled = 0.5;
        p.spin_rate = 0.0;
        p.yaw_rate = 0.0;
        p.traj = None;
        p.natural_air = 0.0;
        p.surface = surface;
        p.body_flip = 0.0;
        p.body_flip_rate = 0.0;
        if let Some(g) = p.grab.take() {
            if g.t > 0.15 {
                p.air.grabs.push((g.name, g.t));
            }
        }
        p.flip = None;
        p.set_state(State::Ground);
        self.events.push(Event::Land { quality: kind, factor });
        self.score_air();
    }

    /// Put the air's tricks into the open sequence (B.14.1).
    fn score_air(&mut self) {
        let p = &self.p;
        let air = p.air.clone();
        let spin_deg = p.spin.abs().to_degrees();
        let spins = ((spin_deg + 45.0) / 180.0).floor() as i32;
        let mut parts: Vec<String> = Vec::new();
        let mut modifier = 1.0;
        if air.switch {
            parts.push("Switch".into());
            modifier *= 1.5;
        } else if air.fakie {
            parts.push("Fakie".into());
            modifier *= 1.2;
        }
        if air.from_manual {
            modifier *= 1.2;
        }
        if air.from_grind {
            modifier *= 1.3;
        }
        if air.stationary {
            modifier *= 0.2;
        }
        let mut spin_key = None;
        if spins > 0 {
            let fs = spin_frontside(p, self.p.spin);
            let deg_name = (spins.min(4) * 180).to_string();
            parts.push(format!("{} {}", if fs { "FS" } else { "BS" }, deg_name));
            spin_key = Some(deg_name);
        }
        let mut keys: Vec<(String, f32)> = Vec::new();
        let mut trick_names: Vec<String> = Vec::new();
        if let Some(g) = air.pop {
            if g.is_flip_trick() || spins == 0 && air.flips.is_empty() && air.grabs.is_empty() {
                let mut n = g.trick_name().to_string();
                if g.family() == Family::Nollie && g != Gesture::Nollie {
                    n = format!("Nollie {}", n);
                }
                trick_names.push(n);
                keys.push((g.trick_name().to_string(), 0.0));
            }
        }
        for g in &air.flips {
            trick_names.push(g.trick_name().to_string());
            keys.push((g.trick_name().to_string(), 0.0));
        }
        for (gname, gt) in &air.grabs {
            trick_names.push(gname.clone());
            keys.push(("Grab".to_string(), *gt));
        }
        if let Some(bf) = air.body_flip {
            trick_names.push(bf.trick_name().to_string());
            keys.push((bf.trick_name().to_string(), 0.0));
        }
        if trick_names.is_empty() && spins == 0 && air.pop.is_none() {
            // A natural air with nothing in it: only the metrics, if any.
        }
        parts.extend(trick_names.iter().cloned());
        let name = parts.join(" ");
        let s = &mut self.score;
        let mut first = true;
        for (k, secs) in keys {
            let shown = if first { name.clone() } else { k.clone() };
            first = false;
            s.add_value(&shown, &k, 0.0, secs, modifier);
        }
        if let Some(k) = spin_key {
            let shown = if first { name.clone() } else { format!("{} spin", k) };
            s.add_value(&shown, &k, 0.0, 0.0, modifier);
        }
        // Air metrics.
        let height = (air.max_y - air.takeoff_y).max(0.0);
        let dist = (p.pos - air.takeoff).planar().len();
        let metric = 100.0 * height + 20.0 * (dist - 2.0).max(0.0);
        if (air.pop.is_some() || spins > 0) && metric > 1.0 {
            s.add("Air", "Air", metric, 1.0);
        }
        if !name.is_empty() {
            self.events.push(Event::Trick { name });
        }
    }

    // ---- failure ------------------------------------------------------------------------

    pub fn bail(&mut self, reason: &str) {
        let p = &mut self.p;
        if !p.on_board() {
            return;
        }
        // The run-out rule (B.9.3): closing speed alone, upright, no grab, no flip.
        if reason == "closing speed" && p.grab.is_none() && p.body_flip == 0.0 && p.rider_up.y.abs() > 0.81 {
            p.off = Some(Off { rider_pos: p.pos, rider_vel: p.vel.planar() * 0.5, heading: p.fwd.planar().norm_or(p.fwd), stumble: 1.5, vy: 0.0 });
            p.vel *= 0.5;
            p.set_state(State::OffBoard);
            p.grind = None;
            p.manual = None;
            self.score.bail();
            self.events.push(Event::RunOut);
            return;
        }
        p.last_bail = reason.to_string();
        let mut bv = p.vel;
        if bv.len() > 20.0 {
            bv = bv.norm() * 20.0;
        }
        p.wipe = Some(Wipe { t: 0.0, body_pos: p.pos + p.rider_up * 0.9, body_vel: bv, recover_in: -1, pose: 0, spin: 0.0, spin_rate: 3.0, push_cool: 0.0 });
        p.grind = None;
        p.manual = None;
        p.flip = None;
        p.grab = None;
        p.traj = None;
        p.set_state(State::Wipeout);
        self.score.bail();
        self.events.push(Event::Bail { reason: reason.to_string() });
    }

    fn wipeout(&mut self, it: &Intents) {
        let w = &self.world;
        let p = &mut self.p;
        let Some(mut wp) = p.wipe else { return };
        wp.t += DT;
        // Hall of Meat (B.13.1): the right stick picks how the body flies (dead zone 0.2), the
        // left stick spins it and drifts it, a trigger pulled all the way pushes off a surface.
        if it.grab_mag > 0.2 {
            let a = it.grab_dir;
            wp.pose = if a.abs() > 2.36 {
                2 // pushed up: swan dive
            } else if a.abs() < 0.79 {
                1 // pulled down: cannonball
            } else if a < 0.0 {
                3 // left: judo kick
            } else {
                4 // right: torpedo
            };
        }
        let (rate, drag) = match wp.pose {
            1 => (9.0, 0.995),
            2 => (1.5, 0.99),
            3 => (5.0, 0.99),
            4 => (0.3, 0.998),
            _ => (3.0, 0.99),
        };
        wp.spin_rate = lerp(wp.spin_rate, rate + it.lx * 4.0, 0.1);
        wp.spin += wp.spin_rate * DT;
        let side = wp.body_vel.planar().norm_or(p.fwd.planar()).cross(UP);
        wp.body_vel += (side * -it.lx + wp.body_vel.planar().norm_or(ZERO) * it.ly) * (1.5 * DT);
        wp.body_vel *= drag;
        wp.push_cool = (wp.push_cool - DT).max(0.0);
        // A tumbling body (B.13.1): ballistic with ground contact and friction.
        wp.body_vel += GRAVITY * DT;
        let next = wp.body_pos + wp.body_vel * DT;
        let mut cs = Vec::new();
        w.sphere_contacts(next, 0.3, &mut cs);
        wp.body_pos = next;
        let push = (self.last_input.rt >= 1.0 || self.last_input.lt >= 1.0) && wp.push_cool <= 0.0;
        for (n, depth, _, _) in &cs {
            wp.body_pos += *n * *depth;
            let vn = wp.body_vel.dot(*n);
            if vn < 0.0 {
                wp.body_vel -= *n * vn;
                wp.body_vel *= 0.92;
            }
            if push {
                wp.body_vel += *n * 4.0;
                wp.push_cool = 0.5;
            }
        }
        // The board: a free object that rolls to a stop.
        p.vel *= 0.98;
        p.vel += GRAVITY * DT;
        let bnext = p.pos + p.vel * DT;
        if let Some(h) = w.raycast(p.pos + UP * 0.3, -UP, 0.3 + (p.pos.y - bnext.y).max(0.0) + 0.1) {
            if bnext.y < h.point.y + 0.05 {
                p.pos = v3(bnext.x, h.point.y + 0.05, bnext.z);
                p.vel.y = 0.0;
            } else {
                p.pos = bnext;
            }
        } else {
            p.pos = bnext;
        }
        let slow = wp.body_vel.len() < 0.5 && wp.t > 0.6;
        let can = slow || wp.t > 1.6;
        if can && it.recover && wp.recover_in < 0 {
            wp.recover_in = 2;
        }
        if wp.recover_in > 0 {
            wp.recover_in -= 1;
        }
        let stuck = cs.iter().any(|c| c.1 > 0.25) && wp.t > 0.5;
        p.wipe = Some(wp);
        if wp.recover_in == 0 || wp.t > 6.0 || stuck {
            self.respawn();
        }
    }

    /// Pick where to put the rider back (B.13.2) and do it.
    pub fn respawn(&mut self) {
        let w = &self.world;
        let p = &self.p;
        let body = p.wipe.map(|wp| wp.body_pos).unwrap_or(p.pos);
        let mut spot: Option<(Vec3, Vec3)> = None;
        if let Some(h) = w.raycast(body + UP * 0.5, -UP, 10.0) {
            let drop = body.y - h.point.y;
            if h.normal.y >= 0.75 && drop <= 1.0 && w.rail_distance(h.point) > 0.3 && h.surface.respawn_ok() {
                let mut cs = Vec::new();
                w.sphere_contacts(h.point + UP * 0.9, 0.5, &mut cs);
                if cs.iter().all(|c| c.3.y > 0.7) {
                    let f = p.checkpoints.last().map_or(p.fwd, |c| c.1);
                    spot = Some((h.point, f));
                }
            }
        }
        if spot.is_none() {
            let n = p.checkpoints.len();
            let mut best: Option<(f32, Vec3, Vec3)> = None;
            for (i, c) in p.checkpoints.iter().enumerate() {
                let age = n - 1 - i;
                let mut s = c.2.checkpoint_score();
                if age == 0 {
                    s -= 100.0;
                }
                if age >= 5 {
                    s -= 200.0;
                }
                if best.map_or(true, |b| s > b.0) {
                    best = Some((s, c.0, c.1));
                }
            }
            spot = best.map(|b| (b.1, b.2));
        }
        let (pos, fwd) = spot.unwrap_or((w.spawn, v3(w.spawn_heading.sin(), 0.0, w.spawn_heading.cos())));
        self.place(pos, fwd);
        self.events.push(Event::Respawn);
    }

    /// Put the board at `pos` facing `fwd`, still, on the ground.
    pub fn place(&mut self, pos: Vec3, fwd: Vec3) {
        let goofy = self.p.goofy;
        let marker = self.p.marker;
        let cps = std::mem::take(&mut self.p.checkpoints);
        let heading = fwd.x.atan2(fwd.z);
        self.p = Player::new(pos + UP * 0.2, heading);
        self.p.goofy = goofy;
        self.p.marker = marker;
        self.p.checkpoints = cps;
        self.p.grace = 0.4;
        settle(&self.world, &mut self.p);
        self.rec.reset();
    }

    fn off_board(&mut self, it: &Intents) {
        let w = &self.world;
        let p = &mut self.p;
        let Some(mut o) = p.off else {
            p.set_state(State::Ground);
            return;
        };
        o.stumble = (o.stumble - DT).max(0.0);
        // The board rolls on with friction.
        let bs = p.vel.planar();
        let bl = bs.len();
        if bl > 0.0 {
            p.vel = p.vel - bs.norm() * (1.5 * DT).min(bl);
        }
        p.pos += p.vel.planar() * DT;
        if let Some(h) = w.raycast(p.pos + UP * 0.5, -UP, 2.0) {
            p.pos.y = h.point.y + DECK_H;
        }
        // The rider walks (B.12): left stick, A sprints, X jumps.
        if o.stumble <= 0.0 {
            let fwd = o.heading;
            let side = fwd.cross(UP);
            let mut want = (fwd * it.ly + side * it.lx) * if self.last_input.down(input::btn::A) { 5.5 } else { 2.5 };
            if want.len() > 5.5 {
                want = want.norm() * 5.5;
            }
            o.rider_vel = o.rider_vel.lerp(want, 0.1);
            if it.lx.abs() + it.ly.abs() > 0.2 {
                o.heading = o.heading.lerp(want.norm_or(o.heading), 0.15).norm_or(o.heading);
            }
            if self.last_input.down(input::btn::X) && o.vy == 0.0 {
                o.vy = (2.0 * G * 0.8).sqrt();
            }
        } else {
            o.rider_vel *= 0.97;
        }
        o.vy -= G * DT;
        o.rider_pos += o.rider_vel * DT + UP * o.vy * DT;
        if let Some(h) = w.raycast(o.rider_pos + UP * 1.0, -UP, 3.0) {
            if o.rider_pos.y <= h.point.y {
                o.rider_pos.y = h.point.y;
                o.vy = 0.0;
            }
        }
        p.off = Some(o);
        let near = (o.rider_pos - p.pos).planar().len() < 1.2;
        if it.toggle_off || (near && it.push_new && o.stumble <= 0.0) {
            let f = o.heading;
            let pos = if near { p.pos } else { o.rider_pos };
            let vel = o.rider_vel;
            self.place(pos, if near { self.p.fwd } else { f });
            self.p.vel = vel;
            self.p.speed_target = vel.len();
        }
    }

    // ---- camera -------------------------------------------------------------------------

    fn camera(&mut self) {
        let w = &self.world;
        let p = &self.p;
        let t = &self.t;
        let c = &mut self.cam;
        let (subject, vel) = match p.state {
            State::Wipeout => (p.wipe.map_or(p.pos, |wp| wp.body_pos), p.wipe.map_or(ZERO, |wp| wp.body_vel)),
            State::OffBoard => (p.off.map_or(p.pos, |o| o.rider_pos + UP * 0.6), p.off.map_or(ZERO, |o| o.rider_vel)),
            _ => (p.pos + p.rider_up * 0.5, p.vel),
        };
        let mut anchor = subject;
        if let (State::KnownAir, Some(tr)) = (p.state, p.traj) {
            let drop = p.pos.y - tr.landing_point.y;
            if drop < 3.0 {
                let k = sat(p.air_time / tr.landing_time.max(0.1));
                anchor.y = lerp(subject.y, tr.landing_point.y + 0.5, k * 0.5) - 0.33;
            }
        }
        c.anchor = c.anchor.lerp(anchor, 0.35);
        let speed = vel.planar().len();
        let vdir = vel.planar().norm_or(c.heading);
        let vert_air = matches!(p.state, State::Air | State::KnownAir)
            && p.traj.map_or(false, |tr| tr.landing_normal.y < deg(30.0).cos().min(0.87))
            && p.air.takeoff_y < p.air.max_y;
        if !vert_air {
            if speed > 0.3 {
                let dead = deg(if speed < 2.0 { 6.0 } else { lerp(6.0, 0.0, sat((speed - 2.0) / 4.0)) });
                let ang = signed_angle(c.heading, vdir, UP);
                if ang.abs() > dead {
                    let want = ang - dead * sign(ang);
                    let step = clamp(want * 0.8 * 4.0 * DT, -deg(180.0) * DT, deg(180.0) * DT);
                    c.heading = rotate(c.heading, UP, step).norm();
                }
            } else if p.on_board() {
                let f = if p.fakie { -p.fwd } else { p.fwd };
                let f = f.planar().norm_or(c.heading);
                c.heading = c.heading.lerp(f, DT).norm_or(c.heading);
            }
        }
        c.distance = lerp(t.cam_distance_slow, t.cam_distance_fast, sat(speed / 10.0));
        // Drop predictor.
        let mut tilt = 0.0;
        if p.state == State::Ground && speed > 0.5 {
            let mut depth: f32 = 0.0;
            for i in 1..=5 {
                let ahead = p.pos + vdir * (speed * 1.2 * i as f32 / 5.0) + UP * 1.0;
                match w.raycast(ahead, -UP, 20.0) {
                    Some(h) => depth = depth.max(p.pos.y - h.point.y),
                    None => depth = depth.max(20.0),
                }
            }
            if depth > 1.5 {
                tilt = (depth / c.distance).atan().min(deg(70.0));
                if tilt < deg(15.0) {
                    tilt = 0.0;
                }
                tilt *= lerp(1.0, 0.9, sat((speed - 4.0) / 4.0));
            }
        }
        c.drop_tilt += clamp(tilt - c.drop_tilt, -deg(1.6), deg(1.6));
        c.elevation = deg(t.cam_elevation_deg) + c.drop_tilt;
        let back = -c.heading * c.elevation.cos() + UP * c.elevation.sin();
        let mut cam = c.anchor + back * c.distance;
        // Pull in when something is between the rider and the camera.
        let d = cam - c.anchor;
        if let Some(h) = w.raycast(c.anchor, d.norm(), d.len()) {
            cam = c.anchor + d.norm() * (h.t - 0.15).max(0.3);
            // Pulled in close (a wall, stairs behind): rise over the rider rather than sit
            // inside them, as far as there is room overhead.
            let near = (cam - c.anchor).len();
            if near < 1.6 {
                let lift = (1.6 - near) * 1.2;
                let room = w.raycast(cam, UP, lift + 0.2).map_or(lift, |h| (h.t - 0.2).max(0.0));
                cam = cam + UP * lift.min(room);
            }
        }
        c.pos = cam;
        c.target = c.anchor + c.heading * t.cam_look_ahead - UP * c.drop_tilt.sin() * 1.5;
        c.fov_y_deg = t.cam_fov_deg;
    }
}

// ---- free helpers (they borrow the world and the player separately) -------------------------

/// Cast the four wheel probes (B.5.2) along the board's -up.
pub fn probe(w: &World, p: &Player) -> [WheelHit; 4] {
    let right = p.right();
    let mut out = [WheelHit::default(); 4];
    let mut i = 0;
    for fz in [HALF_WB, -HALF_WB] {
        for fx in [HALF_TRACK, -HALF_TRACK] {
            let wc = p.pos + p.fwd * fz + right * fx + p.up * (-DECK_H + WHEEL_R);
            let o = wc + p.up * PROBE_LIFT;
            if let Some(h) = w.raycast(o, -p.up, PROBE_LIFT + PROBE_LEN + WHEEL_R + 0.1) {
                out[i] = WheelHit { hit: true, point: h.point, normal: h.normal, dist: h.t - PROBE_LIFT - WHEEL_R, surface: Some(h.surface) };
            }
            i += 1;
        }
    }
    out
}

/// Update wheel hits, the smoothed ground normal and the surface under the board.
fn sense(w: &World, p: &mut Player, t: &Tuning) {
    p.wheels = probe(w, p);
    let down: Vec<&WheelHit> = p.wheels.iter().filter(|h| h.hit && h.dist <= 0.02).collect();
    p.wheels_down = down.len() as u32;
    if p.manual.is_some() && p.wheels_down >= 2 {
        p.wheels_down = 4;
    }
    if !down.is_empty() {
        let closest = down.iter().max_by(|a, b| a.normal.dot(p.rider_up).partial_cmp(&b.normal.dot(p.rider_up)).unwrap()).unwrap().normal;
        let mut n = ZERO;
        for h in &down {
            if angle_between(h.normal, closest) <= deg(45.0) {
                n += h.normal;
            }
        }
        let raw = n.norm_or(p.ground_n);
        p.prev_ground_n = p.ground_n;
        p.ground_n = p.ground_n.lerp(raw, t.ground_n_smoothing).norm_or(raw);
        p.surface = down[0].surface.unwrap_or(Surface::Smooth);
    }
}

/// B.5.3: the rider's up, a rate-limited blend between world up and the ground normal.
fn update_rider_up(p: &mut Player) {
    let slope = clamp(p.ground_n.y, -1.0, 1.0).acos() / std::f32::consts::PI;
    let w = if p.wheels_down >= 2 {
        if slope <= 0.15 {
            slope / 0.15 * 0.6
        } else {
            lerp(0.6, 1.0, sat((slope - 0.15) / 0.15))
        }
    } else {
        0.0
    };
    let target = UP.lerp(p.ground_n, w).norm_or(UP);
    let max = deg(if p.speed() < 2.0 { 2.5 } else { 6.0 });
    p.rider_up = rotate_toward(p.rider_up, target, max, 1.0);
}

/// B.5.4 steps 3 to 5: integrate, yaw, re-probe and snap. Returns true when the board left
/// the ground.
fn integrate_ground(w: &World, p: &mut Player, t: &Tuning) -> bool {
    p.pos += p.vel * DT;
    p.fwd = orthonormal_fwd(rotate(p.fwd, p.up, -p.yaw_rate * DT), p.up);
    let hits = probe(w, p);
    let near: Vec<&WheelHit> = hits.iter().filter(|h| h.hit && h.dist <= t.stick_dist).collect();
    let mut n = ZERO;
    for h in &near {
        n += h.normal;
    }
    let n = n.norm_or(p.ground_n);
    let away = p.vel.dot(n);
    let min_dist = hits.iter().filter(|h| h.hit).map(|h| h.dist).fold(f32::MAX, f32::min);
    if min_dist > t.natural_air_min_dist && min_dist <= t.natural_air_max_dist {
        p.natural_air += DT;
    } else if min_dist <= t.natural_air_min_dist {
        p.natural_air = 0.0;
    }
    if near.len() >= 2 && away < 1.0 && p.natural_air < t.natural_air_time {
        let avg = near.iter().map(|h| h.dist).sum::<f32>() / near.len() as f32;
        p.pos -= p.up * avg;
        let speed = p.vel.len();
        let vn = p.vel.dot(n);
        p.vel -= n * vn;
        // Rolling into a concave surface turns the velocity, it does not eat it.
        if vn < 0.0 {
            let l = p.vel.len();
            if l > 1e-4 {
                p.vel = p.vel * (speed / l).min(1.02);
            }
        }
        p.up = p.up.lerp(n, 0.5).norm_or(n);
        p.fwd = orthonormal_fwd(p.fwd, p.up);
        false
    } else {
        true
    }
}

/// Body and deck contacts with steep faces (B.6.6, B.9.3 closing speed). Removes the
/// velocity into them; returns a bail reason when the closing speed is too high.
fn wall_contacts(w: &World, p: &mut Player, t: &Tuning) -> Option<String> {
    let mut cs = Vec::new();
    let flip_scale = if p.flip.map_or(false, |f| !f.done()) { 0.3 } else { 1.0 };
    let mut reason = None;
    for (h, r) in [(0.25, 0.18), (0.9, 0.25)] {
        let c = p.pos + p.rider_up * h;
        w.sphere_contacts(c, r, &mut cs);
        for (n, depth, _, tn) in cs.iter() {
            // Floors are the wheels' business.
            if tn.dot(p.up) > 0.7 || tn.y > 0.7 && p.on_board() && h < 0.5 {
                continue;
            }
            let vn = p.vel.dot(*n);
            p.pos += *n * *depth;
            if vn < 0.0 {
                let closing_h = (*n * vn).planar().len();
                let closing_v = (n.y * vn).abs();
                if p.grace <= 0.0 && (closing_h > t.closing_h * flip_scale || closing_v > t.closing_v * flip_scale) {
                    reason = Some("closing speed".to_string());
                }
                p.vel -= *n * vn;
                p.speed_target = p.vel.len();
            }
        }
    }
    reason
}

/// Drop the board onto whatever is under it.
fn settle(w: &World, p: &mut Player) {
    if let Some(h) = w.raycast(p.pos + UP * 0.5, -UP, 20.0) {
        p.pos = h.point + h.normal * DECK_H;
        p.up = h.normal;
        p.ground_n = h.normal;
        p.prev_ground_n = h.normal;
        p.fwd = orthonormal_fwd(p.fwd, p.up);
        p.heading = p.fwd;
        p.surface = h.surface;
        p.wheels_down = 4;
    }
}

fn crouch_tick(p: &mut Player, it: &Intents, manualing: bool) {
    let input = it.crouch.max(it.hard_turn_crouch).max(if manualing { 0.4 } else { 0.0 }).max(it.antic_mag * 0.6);
    let mut target = input.max(0.05);
    if p.absorb > 0.0 {
        target = target.max(p.absorb);
        p.absorb = (p.absorb - DT * 3.5).max(0.0);
    }
    p.crouch += clamp(target - p.crouch, -0.08, 0.06);
    p.prev_com = p.com_height;
    p.com_height = lerp(1.0, 0.62, p.crouch);
}

/// B.7.5: standing up through a concave surface adds speed; over a hump it costs.
fn pump(p: &mut Player, t: &Tuning, it: &Intents) {
    let rising_raw = ((p.com_height - p.prev_com) / DT).max(0.0);
    // Smoothed more than the spec's 0.3 so one stand-up spreads over the wall instead of
    // spiking into the max-accel clamp for a few ticks (C.3 step 7 wants about +1 m/s a pump).
    p.rising_s = lerp(p.rising_s, rising_raw, 0.12);
    let rising = if p.rising_s < 0.05 { 0.0 } else { p.rising_s.min(2.0) };
    let travel = p.vel.on_plane(p.ground_n).norm_or(p.fwd);
    let axis = travel.cross(p.rider_up).norm_or(p.right());
    // Positive when the surface rotates up toward the rider (concave).
    let ang = -signed_angle(p.prev_ground_n, p.ground_n, axis) / DT;
    p.ang_speed_s = lerp(p.ang_speed_s, ang, 0.25);
    let speed = p.vel.len();
    let pump_vs_vel = sat((speed - 1.0) / 2.0);
    let pmp = p.ang_speed_s * pump_vs_vel;
    let factor = if pmp > 0.0 { t.pump_factor } else { t.pump_factor };
    let mut accel = clamp(pmp * rising * factor, -t.pump_max_accel, t.pump_max_accel);
    let intentional = it.crouch > 0.1 || p.crouch > 0.3;
    if !intentional {
        accel *= t.pump_unintentional;
    }
    if accel.abs() > 1e-4 {
        p.vel += travel * accel * DT;
        p.speed_target = p.vel.on_plane(p.ground_n).len();
    }
}

/// B.8.1. Returns the pop height. The board must be on the ground (or manualing).
fn pop(p: &mut Player, t: &Tuning, strength: f32, from_manual: bool, it: &Intents) -> f32 {
    let n = p.ground_n;
    let planar_v = p.vel.on_plane(n);
    let surf = planar_v.len();
    let sf = sat(surf / 6.0);
    let abs_min = 0.12;
    let min_h = if from_manual { t.manual_min_h } else { t.ollie_min_h };
    let low = abs_min + lerp(0.8, 1.0, sf) * (min_h - abs_min);
    let high = abs_min + lerp(0.85, 1.0, sf) * (t.ollie_max_h - abs_min);
    let height = lerp(low, high, strength);
    let vresp = lerp(0.9, 1.0, n.y.max(0.0));
    let v_up = (2.0 * G * height).sqrt() * vresp;
    let mut vel = planar_v + n * v_up;
    if n.y < 0.5 {
        vel.y += clamp(0.15 * vel.y, 0.0, 0.6);
    }
    let tangent = orthonormal_fwd(p.fwd, n);
    let right = tangent.cross(n);
    vel += right * (it.lx * 0.15 * v_up) + tangent * (it.ly * 0.10 * v_up);
    // Vert alignment.
    if n.y < 0.35 {
        let d = vel.norm();
        if d.y > 0.5 {
            let lean_into = 0.0;
            let want = (UP - n * (0.25 + 0.5 * lean_into)).norm();
            let nd = d.lerp(want, 0.35).norm();
            vel = nd * vel.len();
        }
    }
    p.vel = vel;
    p.jump_vel = vel;
    p.jump_ticks = 12;
    height
}

pub fn begin_air_pub(p: &mut Player, g: Gesture, from_manual: bool, from_grind: bool, dark: bool, t: &Tuning) {
    begin_air(p, g, from_manual, from_grind, dark, t)
}

/// Enter the air after a pop (or natural air) with the scoring log started.
fn begin_air(p: &mut Player, g: Gesture, from_manual: bool, from_grind: bool, dark: bool, t: &Tuning) {
    p.air = AirLog {
        pop: Some(g),
        flips: Vec::new(),
        grabs: Vec::new(),
        takeoff_y: p.pos.y,
        max_y: p.pos.y,
        takeoff: p.pos,
        from_manual,
        from_grind,
        stationary: p.vel.on_plane(p.ground_n).len() < 1.0,
        switch: p.switch,
        fakie: p.fakie,
        body_flip: None,
    };
    p.air_time = 0.0;
    p.spin = 0.0;
    p.heading = orthonormal_fwd(p.fwd, p.rider_up);
    p.manual = None;
    p.pitch = 0.0;
    p.kick = None;
    p.push_t = -1.0;
    p.pushing = false;
    p.natural_air = 0.0;
    p.flip = None;
    if g.is_flip_trick() {
        start_flip(p, g, false, dark, t);
    }
    p.set_state(State::Air);
}

fn start_flip(p: &mut Player, g: Gesture, late: bool, dark: bool, t: &Tuning) {
    let (turns, shuv) = g.board_motion();
    let dur = if turns != 0.0 {
        t.flip_time
    } else if shuv.abs() > 200.0 {
        0.5
    } else {
        0.35
    };
    p.flip = Some(Flip { gesture: g, t: 0.0, dur, flip_turns: turns, shuv_deg: shuv, late, dark });
}

fn end_manual(score: &mut Scoring, events: &mut Vec<Event>, _p: &mut Player, m: Manual) {
    if m.t > 0.3 {
        let key = if m.nose { "Nose Manual" } else { "Manual" };
        score.add_value(key, key, m.metres, m.t, 1.0);
        let _ = events;
    }
}

fn grab_name(p: &Player, it: &Intents) -> String {
    // Up = nose, down = tail; sideways: toe side (the chest's side) or heel side.
    let ry = it.ry;
    let rx = it.rx;
    if it.grab_mag > 0.3 && ry.abs() > rx.abs() {
        return if ry > 0.0 { "Nosegrab".into() } else { "Tailgrab".into() };
    }
    let chest_right = p.chest().dot(p.right()) > 0.0;
    let toe = if rx.abs() > 0.3 { (rx > 0.0) == chest_right } else { true };
    // The back hand is the right hand for a regular rider riding forward.
    let regular_fwd = p.goofy == p.switch;
    let right_hand = it.right_grab && !it.left_grab;
    let back_hand = right_hand == regular_fwd;
    match (back_hand, toe) {
        (true, true) => "Indy",
        (true, false) => "Stalefish",
        (false, true) => "Mute",
        (false, false) => "Melon",
    }
    .into()
}

/// Was the air's rotation frontside (the chest leading)?
fn spin_frontside(p: &Player, spin: f32) -> bool {
    // Clockwise (positive) rotation moves the chest by -up x chest.
    let travel = p.air.takeoff.lerp(p.pos, 1.0) - p.air.takeoff;
    let travel = travel.planar().norm_or(p.fwd);
    let chest = p.chest().planar().norm_or(p.right());
    let motion = -UP.cross(chest) * sign(spin);
    motion.dot(travel) > 0.0
}

/// Trajectory prediction with grind magnetism (B.8.2, B.10.3). `adjust` applies the
/// velocity correction (at the pop and at natural air) and enters KnownAir.
pub fn predict(w: &World, p: &mut Player, t: &Tuning, popped: bool, adjust: bool) {
    let base = p.vel;
    let mut cands = vec![base];
    if popped {
        let speed = base.len();
        let cx = deg(4.0);
        let cz = deg(4.0) * clamp(speed * 0.05, 0.5, 2.0);
        let right = p.right();
        let fwd = p.fwd;
        for (ax, a) in [(right, cx), (right, -cx), (fwd, cz), (fwd, -cz)] {
            cands.push(rotate(base, ax, a));
        }
        cands.push(rotate(rotate(base, right, cx), fwd, cz));
        cands.push(rotate(rotate(base, right, -cx), fwd, -cz));
    }
    let mut best: Option<(f32, Vec3, Traj)> = None;
    let start = p.pos;
    let t0 = p.air_time;
    for (ci, c) in cands.iter().enumerate() {
        let Some(mut land) = sweep_parabola(w, start, *c, 3.0) else { continue };
        if land.0 < 0.1 {
            continue;
        }
        let (lt, lp, ln, surf) = land;
        let cn = c.norm();
        let tb = if lt < 0.25 { 0.0 } else if lt <= 1.5 { 1.0 } else { lerp(1.0, -2.0, sat((lt - 1.5) / 1.5)) };
        let vel_at = *c + GRAVITY * lt;
        let mut score = tb + 1.0 + ln.cross(vel_at.norm()).len() * 2.0 + ln.dot(-vel_at.norm()) * -1.0;
        if ci == 0 {
            score += 1.5;
        }
        if surf == Surface::Unrideable {
            score -= 1000.0;
        }
        if ln.y < 0.3 && ln.dot(-vel_at.norm()) < 0.0 {
            score -= 1000.0;
        }
        if ln.y < 0.9 && vel_at.y > 0.0 {
            score -= 500.0;
        }
        let _ = cn;
        let mut traj = Traj {
            landing_point: lp,
            landing_normal: ln,
            landing_time: t0 + lt,
            apex_time: t0 + (c.y / G).max(0.0),
            landing_heading: vel_at.on_plane(ln).norm_or(p.fwd),
            grind_chord: None,
            grind_across: false,
        };
        let mut vel_c = *c;
        // Grind magnetism.
        if let Some((chord, corr, dist, across)) = grind::magnet(w, p, t, start, *c, lp) {
            let dv = corr;
            let lim = t.traj_adjust * 1.5;
            let dv = if dv.len() > lim { dv.norm() * lim } else { dv };
            vel_c = *c + dv;
            if angle_between(vel_c, *c) <= deg(12.0) {
                if let Some(l2) = sweep_parabola(w, start, vel_c, 3.0) {
                    land = l2;
                    traj.landing_point = land.1;
                    traj.landing_time = t0 + land.0;
                }
                traj.grind_chord = Some(chord);
                traj.grind_across = across;
                score += if ci == 0 { 3.0 } else { -dist * 2.0 };
            } else {
                vel_c = *c;
            }
        }
        if best.as_ref().map_or(true, |b| score > b.0) {
            best = Some((score, vel_c, traj));
        }
    }
    match best {
        Some((s, v, tr)) if s > -400.0 => {
            if adjust {
                let mut dv = v - base;
                let lim = if tr.grind_chord.is_some() { t.traj_adjust * 1.5 } else { t.traj_adjust };
                if dv.len() > lim {
                    dv = dv.norm() * lim;
                }
                p.vel += dv;
                p.jump_vel += dv;
            }
            p.traj = Some(tr);
            if p.state == State::Air || adjust {
                p.set_state(State::KnownAir);
            }
        }
        _ => {
            p.traj = None;
            if p.state == State::KnownAir {
                p.set_state(State::Air);
            }
        }
    }
}

/// Follow p(t) = start + v t + g t^2 / 2 in 1/60 steps; the first surface the wheel-bottom
/// point crosses: (time, point, normal, surface).
pub fn sweep_parabola(w: &World, start: Vec3, v: Vec3, max_t: f32) -> Option<(f32, Vec3, Vec3, Surface)> {
    let off = UP * DECK_H;
    let mut prev = start - off;
    let steps = (max_t / DT) as i32;
    for i in 1..=steps {
        let tt = i as f32 * DT;
        let pnt = start + v * tt + GRAVITY * (0.5 * tt * tt) - off;
        let d = pnt - prev;
        let l = d.len();
        if l > 1e-6 && tt > 0.05 {
            if let Some(h) = w.raycast(prev, d / l, l) {
                let mut n = ZERO;
                let mut cs = Vec::new();
                w.sphere_contacts(h.point + h.normal * 0.01, 0.3, &mut cs);
                for c in &cs {
                    // Faces near the one hit: a ledge's side must not tilt its top's normal.
                    if (c.3.y > 0.7 || c.3.dot(-d) > 0.0) && c.3.dot(h.normal) > 0.7 {
                        n += c.3;
                    }
                }
                let n = n.norm_or(h.normal);
                return Some((tt - DT + h.t / l * DT, h.point + off, n, h.surface));
            }
        }
        prev = pnt;
    }
    None
}
