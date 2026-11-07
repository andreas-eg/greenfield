//! Input layer (spec B.2.3, B.4): the virtual controller every backend fills, button edges,
//! the Flickit gesture recognizer with flick strength, stance mirroring, and the intents
//! derived every tick.

use crate::math::*;
use serde::{Deserialize, Serialize};

/// Button indices into `VirtualController::buttons`.
pub mod btn {
    pub const A: usize = 0;
    pub const B: usize = 1;
    pub const X: usize = 2;
    pub const Y: usize = 3;
    pub const LB: usize = 4;
    pub const RB: usize = 5;
    pub const L3: usize = 6;
    pub const R3: usize = 7;
    pub const DUP: usize = 8;
    pub const DDOWN: usize = 9;
    pub const DLEFT: usize = 10;
    pub const DRIGHT: usize = 11;
    pub const COUNT: usize = 12;
}

/// One sample of the pad, raw (before the dead zone). +y is up on the stick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[repr(C)]
pub struct VirtualController {
    pub lx: f32,
    pub ly: f32,
    pub rx: f32,
    pub ry: f32,
    pub lt: f32,
    pub rt: f32,
    /// Bit i set = button i down (see `btn`).
    pub buttons: u32,
}

impl VirtualController {
    pub fn down(&self, b: usize) -> bool {
        self.buttons & (1 << b) != 0
    }
    pub fn with(mut self, b: usize, on: bool) -> Self {
        if on {
            self.buttons |= 1 << b;
        } else {
            self.buttons &= !(1 << b);
        }
        self
    }
}

/// Dead zone (B.2.3) `[code]`: 0.25 dead, 0.95 saturates.
pub fn dead_zone(x: f32, y: f32) -> (f32, f32) {
    let len = (x * x + y * y).sqrt();
    if len < 0.001 {
        return (0.0, 0.0);
    }
    let f = clamp((len - 0.25) * 1.428_571_4, 0.0, 1.0) / len;
    (x * f, y * f)
}

/// Stick angle (B.2.3): 0 = pulled down (toward the tail), +pi/2 right, +/-pi pushed up.
pub fn stick_angle(x: f32, y: f32) -> f32 {
    x.atan2(-y)
}

// ---- patterns ------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Gesture {
    Ollie,
    Nollie,
    Kickflip,
    Heelflip,
    PopShuvit,
    FsPopShuvit,
    VarialKickflip,
    VarialHeelflip,
    Hardflip,
    InwardHeelflip,
    PopShuvit360,
    FsPopShuvit360,
    Flip360,
    Laserflip,
    NKickflip,
    NHeelflip,
    NPopShuvit,
    NFsPopShuvit,
    NVarialKickflip,
    NVarialHeelflip,
    NHardflip,
    NInwardHeelflip,
    N360Flip,
    NLaserflip,
    LateKickflip,
    LateHeelflip,
    LateShuvit,
    LateFsShuvit,
    FrontFlip,
    BackFlip,
    SlideFs180,
    SlideBs180,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Family {
    Ollie,
    Nollie,
    Air,
    LeftStick,
}

impl Gesture {
    pub fn family(self) -> Family {
        use Gesture::*;
        match self {
            Ollie | Kickflip | Heelflip | PopShuvit | FsPopShuvit | VarialKickflip | VarialHeelflip | Hardflip
            | InwardHeelflip | PopShuvit360 | FsPopShuvit360 | Flip360 | Laserflip => Family::Ollie,
            Nollie | NKickflip | NHeelflip | NPopShuvit | NFsPopShuvit | NVarialKickflip | NVarialHeelflip | NHardflip
            | NInwardHeelflip | N360Flip | NLaserflip => Family::Nollie,
            LateKickflip | LateHeelflip | LateShuvit | LateFsShuvit | FrontFlip | BackFlip => Family::Air,
            SlideFs180 | SlideBs180 => Family::LeftStick,
        }
    }
    /// The ollie-family twin of a nollie-family gesture and back (fakie swap, B.4.5).
    pub fn pop_twin(self) -> Gesture {
        use Gesture::*;
        match self {
            Ollie => Nollie,
            Nollie => Ollie,
            Kickflip => NKickflip,
            NKickflip => Kickflip,
            Heelflip => NHeelflip,
            NHeelflip => Heelflip,
            PopShuvit => NPopShuvit,
            NPopShuvit => PopShuvit,
            FsPopShuvit => NFsPopShuvit,
            NFsPopShuvit => FsPopShuvit,
            VarialKickflip => NVarialKickflip,
            NVarialKickflip => VarialKickflip,
            VarialHeelflip => NVarialHeelflip,
            NVarialHeelflip => VarialHeelflip,
            Hardflip => NHardflip,
            NHardflip => Hardflip,
            InwardHeelflip => NInwardHeelflip,
            NInwardHeelflip => InwardHeelflip,
            Flip360 => N360Flip,
            N360Flip => Flip360,
            Laserflip => NLaserflip,
            NLaserflip => Laserflip,
            g => g,
        }
    }
    /// Goofy / switch mirroring by name pairs (B.4.5).
    pub fn mirrored(self) -> Gesture {
        use Gesture::*;
        match self {
            Kickflip => Heelflip,
            Heelflip => Kickflip,
            PopShuvit => FsPopShuvit,
            FsPopShuvit => PopShuvit,
            VarialKickflip => VarialHeelflip,
            VarialHeelflip => VarialKickflip,
            Hardflip => InwardHeelflip,
            InwardHeelflip => Hardflip,
            Flip360 => Laserflip,
            Laserflip => Flip360,
            PopShuvit360 => FsPopShuvit360,
            FsPopShuvit360 => PopShuvit360,
            NKickflip => NHeelflip,
            NHeelflip => NKickflip,
            NPopShuvit => NFsPopShuvit,
            NFsPopShuvit => NPopShuvit,
            NVarialKickflip => NVarialHeelflip,
            NVarialHeelflip => NVarialKickflip,
            NHardflip => NInwardHeelflip,
            NInwardHeelflip => NHardflip,
            N360Flip => NLaserflip,
            NLaserflip => N360Flip,
            LateKickflip => LateHeelflip,
            LateHeelflip => LateKickflip,
            LateShuvit => LateFsShuvit,
            LateFsShuvit => LateShuvit,
            g => g,
        }
    }
    /// The flip the board does: (flip turns about the long axis, +1 kick / -1 heel; shuv
    /// degrees about the vertical, + backside).
    pub fn board_motion(self) -> (f32, f32) {
        use Gesture::*;
        match self {
            Kickflip | NKickflip | LateKickflip => (1.0, 0.0),
            Heelflip | NHeelflip | LateHeelflip => (-1.0, 0.0),
            PopShuvit | NPopShuvit | LateShuvit => (0.0, 180.0),
            FsPopShuvit | NFsPopShuvit | LateFsShuvit => (0.0, -180.0),
            VarialKickflip | NVarialKickflip => (1.0, 180.0),
            VarialHeelflip | NVarialHeelflip => (-1.0, -180.0),
            Hardflip | NHardflip => (1.0, -180.0),
            InwardHeelflip | NInwardHeelflip => (-1.0, 180.0),
            PopShuvit360 => (0.0, 360.0),
            FsPopShuvit360 => (0.0, -360.0),
            Flip360 | N360Flip => (1.0, 360.0),
            Laserflip | NLaserflip => (-1.0, -360.0),
            _ => (0.0, 0.0),
        }
    }
    pub fn is_flip_trick(self) -> bool {
        let (f, s) = self.board_motion();
        f != 0.0 || s != 0.0
    }
    pub fn trick_name(self) -> &'static str {
        use Gesture::*;
        match self {
            Ollie => "Ollie",
            Nollie => "Nollie",
            Kickflip | NKickflip => "Kickflip",
            Heelflip | NHeelflip => "Heelflip",
            PopShuvit | NPopShuvit => "Pop Shuvit",
            FsPopShuvit | NFsPopShuvit => "FS Pop Shuvit",
            VarialKickflip | NVarialKickflip => "Varial Kickflip",
            VarialHeelflip | NVarialHeelflip => "Varial Heelflip",
            Hardflip | NHardflip => "Hardflip",
            InwardHeelflip | NInwardHeelflip => "Inward Heelflip",
            PopShuvit360 => "360 Pop Shuvit",
            FsPopShuvit360 => "FS 360 Pop Shuvit",
            Flip360 | N360Flip => "360 Flip",
            Laserflip | NLaserflip => "Laser Flip",
            LateKickflip => "Late Kickflip",
            LateHeelflip => "Late Heelflip",
            LateShuvit => "Late Shuvit",
            LateFsShuvit => "Late FS Shuvit",
            FrontFlip => "Front Flip",
            BackFlip => "Back Flip",
            SlideFs180 => "FS Revert",
            SlideBs180 => "BS Revert",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Pattern {
    pub gesture: Gesture,
    pub points: Vec<Vec2>,
    pub tolerance: f32,
    /// Which stick feeds it.
    pub left: bool,
}

fn pat(g: Gesture, pts: &[(f32, f32)]) -> Pattern {
    Pattern { gesture: g, points: pts.iter().map(|&(x, y)| v2(x, y)).collect(), tolerance: 0.4, left: false }
}

/// The pattern set of B.4.4, authored for a regular rider in stick space (+y = pulled down).
pub fn pattern_set() -> Vec<Pattern> {
    use Gesture::*;
    let ollie: Vec<(Gesture, Gesture, Vec<(f32, f32)>)> = vec![
        (Ollie, Nollie, vec![(0.0, 0.9), (0.0, -0.9)]),
        (Kickflip, NKickflip, vec![(0.0, 0.9), (-0.55, 0.75), (-0.9, -0.45)]),
        (Heelflip, NHeelflip, vec![(0.0, 0.9), (0.55, 0.75), (0.9, -0.45)]),
        (PopShuvit, NPopShuvit, vec![(0.0, 0.9), (0.85, 0.45), (0.3, -0.9)]),
        (FsPopShuvit, NFsPopShuvit, vec![(0.0, 0.9), (-0.85, 0.45), (-0.3, -0.9)]),
        (VarialKickflip, NVarialKickflip, vec![(0.4, 0.85), (0.9, 0.2), (0.0, -0.95)]),
        (VarialHeelflip, NVarialHeelflip, vec![(-0.4, 0.85), (-0.9, 0.2), (0.0, -0.95)]),
        (Hardflip, NHardflip, vec![(0.0, 0.9), (0.6, 0.7), (0.9, -0.2), (0.3, -0.9)]),
        (InwardHeelflip, NInwardHeelflip, vec![(0.0, 0.9), (-0.6, 0.7), (-0.9, -0.2), (-0.3, -0.9)]),
        (Flip360, N360Flip, vec![(-0.97, 0.27), (-0.5, 0.84), (0.21, 0.99), (0.91, -0.41)]),
        (Laserflip, NLaserflip, vec![(0.97, 0.27), (0.5, 0.84), (-0.21, 0.99), (-0.91, -0.41)]),
    ];
    let mut out = Vec::new();
    for (g, ng, pts) in &ollie {
        out.push(pat(*g, pts));
        // Nollie twins: y negated (start at the top, flick down).
        let neg: Vec<(f32, f32)> = pts.iter().map(|&(x, y)| (x, -y)).collect();
        out.push(pat(*ng, &neg));
    }
    out.push(pat(PopShuvit360, &[(0.0, 0.9), (0.9, 0.3), (0.5, -0.8), (-0.4, -0.85)]));
    out.push(pat(FsPopShuvit360, &[(0.0, 0.9), (-0.9, 0.3), (-0.5, -0.8), (0.4, -0.85)]));
    // Late flips: the shapes without their first point, matched only in the air.
    out.push(pat(LateKickflip, &[(-0.55, 0.75), (-0.9, -0.45)]));
    out.push(pat(LateHeelflip, &[(0.55, 0.75), (0.9, -0.45)]));
    out.push(pat(LateShuvit, &[(0.85, 0.45), (0.3, -0.9)]));
    out.push(pat(LateFsShuvit, &[(-0.85, 0.45), (-0.3, -0.9)]));
    out.push(pat(FrontFlip, &[(0.0, -0.9), (0.0, 0.9)]));
    out.push(pat(BackFlip, &[(0.0, 0.9), (0.0, -0.9)]));
    // Left stick half circles round the bottom of the disc (reverts).
    let mut fs = pat(SlideFs180, &[(-0.9, 0.0), (-0.6, 0.65), (0.0, 0.9), (0.6, 0.65), (0.9, 0.0)]);
    fs.left = true;
    let mut bs = pat(SlideBs180, &[(0.9, 0.0), (0.6, 0.65), (0.0, 0.9), (-0.6, 0.65), (-0.9, 0.0)]);
    bs.left = true;
    out.push(fs);
    out.push(bs);
    out
}

#[derive(Clone, Copy, Debug, Default)]
struct MatchState {
    active: bool,
    next: usize,
    elapsed: u32,
    misses: u32,
    dist_sum: f32,
    matched: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Completion {
    pub gesture: Gesture,
    /// Flick strength 0..1 (B.4.3).
    pub strength: f32,
    pub elapsed: u32,
}

/// Which patterns may complete now (the state machine says where the board is).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context {
    Ground,
    Air,
    Other,
}

pub struct Recognizer {
    pub patterns: Vec<Pattern>,
    states: Vec<MatchState>,
    cooldown: u32,
    /// The pattern completed last and still held (stick within tolerance of its last point).
    pub held: Option<Gesture>,
    held_idx: Option<usize>,
    /// Samples per point (low, high) for flick strength, for 60 Hz sampling.
    pub window: (f32, f32),
    /// 2.0 when fed 120 samples a second (B.4.1).
    pub rate_scale: f32,
    /// Recent right stick samples in stick space, for the visualiser.
    pub trail: Vec<Vec2>,
    prev_l: Vec2,
    prev_r: Vec2,
}

impl Recognizer {
    pub fn new(window: (f32, f32)) -> Recognizer {
        let patterns = pattern_set();
        let n = patterns.len();
        Recognizer { patterns, states: vec![MatchState::default(); n], cooldown: 0, held: None, held_idx: None, window, rate_scale: 1.0, trail: Vec::new(), prev_l: Vec2::default(), prev_r: Vec2::default() }
    }

    pub fn reset(&mut self) {
        self.states.iter_mut().for_each(|s| *s = MatchState::default());
        self.held = None;
        self.held_idx = None;
    }

    /// Feed one sample of both sticks (dead zone already applied, +y up). Returns the
    /// winning completion on this sample, if any.
    pub fn sample(&mut self, left: (f32, f32), right: (f32, f32), ctx: Context) -> Option<Completion> {
        let prep = |(x, y): (f32, f32)| {
            let (mut x, mut y) = (x, -y);
            if x.abs() < 0.1 {
                x = 0.0;
            }
            if y.abs() < 0.1 {
                y = 0.0;
            }
            v2(x, y)
        };
        let l = prep(left);
        let r = prep(right);
        self.trail.push(r);
        if self.trail.len() > 30 {
            self.trail.remove(0);
        }
        if let Some(i) = self.held_idx {
            let p = &self.patterns[i];
            let s = if p.left { l } else { r };
            if s.dist(*p.points.last().unwrap()) > p.tolerance {
                self.held = None;
                self.held_idx = None;
            }
        }
        let max_misses = (3.0 * self.rate_scale).round() as u32;
        let mut done: Vec<(usize, f32)> = Vec::new();
        for (i, p) in self.patterns.iter().enumerate() {
            let allowed = match p.gesture.family() {
                Family::Ollie | Family::Nollie => ctx == Context::Ground,
                Family::Air => ctx == Context::Air,
                Family::LeftStick => ctx != Context::Other,
            };
            let st = &mut self.states[i];
            if !allowed {
                *st = MatchState::default();
                continue;
            }
            let s = if p.left { l } else { r };
            let prev = if p.left { self.prev_l } else { self.prev_r };
            if !st.active {
                let d = s.dist(p.points[0]);
                if d <= p.tolerance {
                    *st = MatchState { active: true, next: 1, elapsed: 1, misses: 0, dist_sum: d, matched: 1 };
                }
            } else {
                let d = s.dist(p.points[st.next]);
                if d <= p.tolerance {
                    st.next += 1;
                    st.misses = 0;
                    st.elapsed += 1;
                    st.dist_sum += d;
                    st.matched += 1;
                    if st.next == p.points.len() {
                        let n = p.points.len() as f32;
                        let mean = clamp(st.dist_sum / st.matched as f32, 0.001, 0.15);
                        let score = n.powi(4) / (mean * st.elapsed.max(1) as f32);
                        done.push((i, score));
                        st.active = false;
                    }
                } else if !(st.next == 1 && s.dist(p.points[0]) <= p.tolerance) {
                    st.elapsed += 1;
                    // A sample still closing on the next point is slow, not wrong: only a
                    // sample that does not approach it is a miss (so a lazy flick is a weak
                    // pop rather than none, B.4.3's strength window).
                    if s.dist(p.points[st.next]) >= prev.dist(p.points[st.next]) - 0.01 {
                        st.misses += 1;
                    }
                    if st.misses > max_misses {
                        st.active = false;
                    }
                } else if s.dist(prev) > 0.08 {
                    // Holding the first point still does not age the pattern (a long crouch
                    // costs nothing), but moving slowly through it does. The spec's matcher
                    // never ages there at all, which cannot give its own acceptance numbers
                    // (C.3 step 3: a 150 ms lazy push is a hop under 0.3 m).
                    st.elapsed += 1;
                }
            }
        }
        self.prev_l = l;
        self.prev_r = r;
        if self.cooldown > 0 {
            self.cooldown -= 1;
            return None;
        }
        let (i, _) = done.into_iter().fold(None, |best: Option<(usize, f32)>, c| match best {
            Some(b) if b.1 >= c.1 => Some(b),
            _ => Some(c),
        })?;
        self.cooldown = 1;
        let p = &self.patterns[i];
        let elapsed = self.states[i].elapsed;
        let ratio = elapsed as f32 / self.rate_scale / p.points.len() as f32;
        let (low, high) = self.window;
        let strength = if ratio <= low {
            1.0
        } else if ratio >= high {
            0.0
        } else {
            (high - ratio) / (high - low)
        };
        // A completion resets every other pattern that started inside it.
        for (j, st) in self.states.iter_mut().enumerate() {
            if j != i && self.patterns[j].left == p.left {
                *st = MatchState::default();
            }
        }
        self.held = Some(p.gesture);
        self.held_idx = Some(i);
        Some(Completion { gesture: p.gesture, strength, elapsed })
    }
}

/// Apply stance (B.4.5): goofy or switch mirror the name; fakie swaps ollie and nollie.
pub fn apply_stance(g: Gesture, goofy: bool, switch: bool, fakie: bool) -> Gesture {
    let mut g = g;
    if goofy != switch {
        g = g.mirrored();
    }
    if fakie && matches!(g.family(), Family::Ollie | Family::Nollie) {
        g = g.pop_twin();
    }
    g
}

// ---- intents -------------------------------------------------------------------------

/// What the player means this tick (B.4.6), from the dead-zoned controller.
#[derive(Clone, Copy, Debug, Default)]
pub struct Intents {
    pub turn: f32,
    pub hard_turn: f32,
    pub hard_turn_crouch: f32,
    pub body_spin: f32,
    pub kick_turn: f32,
    /// +1 right, -1 left, 0 none: a powerslide start.
    pub slide_start: i32,
    pub slide: f32,
    pub push_new: bool,
    pub push_held: bool,
    pub mongo: bool,
    pub crouch: f32,
    pub antic_mag: f32,
    pub manual: f32,
    pub manual_brake: f32,
    pub grind_balance: f32,
    pub grind_translate: f32,
    pub grind_nudge: f32,
    pub grind_up_down: f32,
    pub left_grab: bool,
    pub right_grab: bool,
    pub grab_dir: f32,
    pub grab_mag: f32,
    pub brake: bool,
    pub dismount: bool,
    pub toggle_off: bool,
    pub grab_world: bool,
    pub set_marker: bool,
    pub return_marker: bool,
    pub bail: bool,
    pub recover: bool,
    pub stick_input: bool,
    pub dark: bool,
    /// The raw left stick after the dead zone.
    pub lx: f32,
    pub ly: f32,
    pub rx: f32,
    pub ry: f32,
}

/// Per-button timers and the previous sample, for edges and holds.
#[derive(Clone, Debug, Default)]
pub struct Buttons {
    prev: u32,
    hold: [f32; btn::COUNT],
    since_release: [f32; btn::COUNT],
}

impl Buttons {
    pub fn update(&mut self, c: &VirtualController, dt: f32) -> (u32, u32) {
        let pressed = c.buttons & !self.prev;
        let released = !c.buttons & self.prev;
        for i in 0..btn::COUNT {
            if c.down(i) {
                self.hold[i] += dt;
            } else {
                if released & (1 << i) != 0 {
                    self.since_release[i] = 0.0;
                }
                self.hold[i] = 0.0;
                self.since_release[i] += dt;
            }
        }
        self.prev = c.buttons;
        (pressed, released)
    }
    pub fn hold(&self, b: usize) -> f32 {
        self.hold[b]
    }
    pub fn since_release(&self, b: usize) -> f32 {
        self.since_release[b]
    }
}

pub fn derive_intents(c: &VirtualController, buttons: &Buttons, pressed: u32, auto_push: bool) -> Intents {
    let (lx, ly) = dead_zone(c.lx, c.ly);
    let (rx, ry) = dead_zone(c.rx, c.ry);
    let mut it = Intents { lx, ly, rx, ry, ..Default::default() };
    let a = stick_angle(lx, ly);
    let len = (lx * lx + ly * ly).sqrt();
    let aa = a.abs();
    it.turn = if ly > 0.0 {
        if aa > 3.05 {
            0.0
        } else if aa > 2.35 {
            (3.05 - aa) * 1.4286 * sign(lx) * len
        } else {
            sign(lx) * len
        }
    } else {
        lx
    };
    if len > 0.9 && (0.91..=1.39).contains(&aa) {
        it.hard_turn = ((aa - 1.39) * -2.0833).max(0.001) * sign(lx);
    }
    it.hard_turn_crouch = it.hard_turn.abs() * 0.85;
    it.body_spin = lx;
    it.kick_turn = lx;
    let none_held = !c.down(btn::A) && !c.down(btn::B) && !c.down(btn::X);
    if len > 0.9 && none_held {
        if a > 0.0 && a < 0.91 {
            it.slide_start = 1;
        } else if a < 0.0 && a > -0.91 {
            it.slide_start = -1;
        }
    }
    it.slide = if a >= 0.0 { clamp((2.0 - a) * 0.28, -1.0, 1.0) } else { clamp(-((a + 2.0) * 0.28), -1.0, 1.0) };
    if len < 0.5 {
        it.slide = 0.0;
    }
    let pa = pressed & (1 << btn::A) != 0;
    let px = pressed & (1 << btn::X) != 0;
    let both = c.down(btn::A) && c.down(btn::X);
    let auto = auto_push && len > 0.8 && aa >= std::f32::consts::PI - 0.6;
    it.push_new = !both && (pa || px);
    it.push_held = !both && (c.down(btn::A) || c.down(btn::X) || auto);
    it.mongo = c.down(btn::X) && !c.down(btn::A);
    let rlen = (rx * rx + ry * ry).sqrt();
    let ra = stick_angle(rx, ry).abs();
    it.manual = sign(ry) * rlen;
    if ry.abs() < 1e-6 {
        it.manual = 0.0;
    }
    it.manual_brake = if rlen > 0.9 { (rlen - 0.9) * 10.0 } else { 0.0 };
    it.crouch = c.lt.max(c.rt).max(it.hard_turn_crouch);
    if ra <= 0.52 || ra >= 2.62 {
        it.antic_mag = rlen;
    }
    it.grind_balance = -lx;
    it.grind_translate = clamp(lx + rx, -1.0, 1.0);
    it.grind_nudge = lx;
    it.grind_up_down = ry;
    it.left_grab = c.lt > 0.0;
    it.right_grab = c.rt > 0.0;
    it.grab_dir = rx.atan2(-ry);
    it.grab_mag = rlen;
    it.brake = c.down(btn::B);
    it.dismount = pressed & (1 << btn::B) != 0;
    it.toggle_off = pressed & (1 << btn::Y) != 0 && !c.down(btn::LB);
    it.grab_world = c.down(btn::RB);
    it.set_marker = c.down(btn::LB) && pressed & (1 << btn::DDOWN) != 0;
    it.return_marker = c.down(btn::LB) && c.down(btn::DUP) && buttons.hold(btn::DUP) >= 0.2;
    let chord = c.lt >= 1.0 && c.rt >= 1.0 && c.down(btn::L3) && c.down(btn::R3);
    it.bail = chord && pressed & ((1 << btn::L3) | (1 << btn::R3)) != 0;
    it.recover = pressed & ((1 << btn::A) | (1 << btn::X)) != 0;
    it.stick_input = len > 0.0 || rlen > 0.0;
    it.dark = c.down(btn::B) || c.down(btn::RB);
    it
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feed a stick path (in raw +y-up stick coordinates) one sample per tick.
    fn run(r: &mut Recognizer, path: &[(f32, f32)]) -> Option<Completion> {
        let mut out = None;
        for &p in path {
            if let Some(c) = r.sample((0.0, 0.0), p, Context::Ground) {
                out = Some(c);
            }
        }
        out
    }

    #[test]
    fn quick_ollie_is_full_strength() {
        let mut r = Recognizer::new((1.75, 4.4));
        // Pull down (stick -y), then snap up.
        let c = run(&mut r, &[(0.0, -1.0), (0.0, -1.0), (0.0, 0.0), (0.0, 1.0)]).unwrap();
        assert_eq!(c.gesture, Gesture::Ollie);
        assert_eq!(c.strength, 1.0);
    }

    #[test]
    fn long_crouch_costs_nothing() {
        let mut r = Recognizer::new((1.75, 4.4));
        let mut path = vec![(0.0, -1.0); 120];
        path.extend_from_slice(&[(0.0, 0.0), (0.0, 1.0)]);
        let c = run(&mut r, &path).unwrap();
        assert_eq!(c.strength, 1.0);
    }

    #[test]
    fn lazy_ollie_is_weak() {
        let mut r = Recognizer::new((1.75, 4.4));
        // Completed in 9 samples (150 ms): from the bottom to the top zone's edge.
        let path: Vec<(f32, f32)> = (0..=8).map(|i| (0.0, -1.0 + 1.5 * i as f32 / 8.0)).collect();
        let c = run(&mut r, &path).unwrap();
        assert_eq!(c.gesture, Gesture::Ollie);
        assert!(c.strength < 0.1, "strength {}", c.strength);
    }

    #[test]
    fn kickflip_shape() {
        let mut r = Recognizer::new((1.75, 4.4));
        // Down, curve toward the left (stick -x), flick up-left.
        let c = run(&mut r, &[(0.0, -0.95), (-0.55, -0.75), (-0.9, 0.45)]).unwrap();
        assert_eq!(c.gesture, Gesture::Kickflip);
    }

    #[test]
    fn stance_mirrors_and_fakie_swaps() {
        assert_eq!(apply_stance(Gesture::Kickflip, true, false, false), Gesture::Heelflip);
        assert_eq!(apply_stance(Gesture::Kickflip, true, true, false), Gesture::Kickflip);
        assert_eq!(apply_stance(Gesture::Ollie, false, false, true), Gesture::Nollie);
    }

    #[test]
    fn dead_zone_values() {
        assert_eq!(dead_zone(0.2, 0.0), (0.0, 0.0));
        let (x, _) = dead_zone(0.95, 0.0);
        assert!((x - 1.0).abs() < 1e-4);
    }
}
