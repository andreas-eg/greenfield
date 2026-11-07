//! What a renderer needs from one tick (B.1 outputs): the board's transform with its flip
//! animation, the rider's pose inputs, the camera, and the HUD's numbers. A plain C struct
//! so the lens (Rust), a Swift shell and JavaScript can all read it.

use crate::math::*;
use crate::sim::{Sim, State};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Frame {
    pub tick: u64,
    /// Board deck centre and frame.
    pub pos: [f32; 3],
    pub fwd: [f32; 3],
    pub up: [f32; 3],
    /// Visual-only board motion: flip about the long axis and shuvit about up (radians),
    /// manual pitch about the board's right.
    pub flip_angle: f32,
    pub shuv_angle: f32,
    pub pitch: f32,
    /// Rider: where the body is (the board when riding), its up, crouch 0..1, lean (rad),
    /// body flip (rad), and the clip the animation should play (`clip_name()`).
    pub rider_pos: [f32; 3],
    pub rider_up: [f32; 3],
    pub crouch: f32,
    pub lean: f32,
    pub body_flip: f32,
    pub clip: u32,
    pub goofy: u32,
    pub fakie: u32,
    pub state: u32,
    pub speed: f32,
    /// Camera position, target and vertical field of view in degrees.
    pub cam_pos: [f32; 3],
    pub cam_target: [f32; 3],
    pub cam_fov: f32,
    /// HUD.
    pub score: f32,
    pub line: f32,
    pub line_timer: f32,
    pub multiplier: f32,
    pub sequence: f32,
}

fn a(v: Vec3) -> [f32; 3] {
    [v.x, v.y, v.z]
}

/// The animation minimum set (B.17) as numbers.
pub const CLIPS: &[&str] = &[
    "ride", "push", "push_mongo", "brake", "manual", "nose_manual", "powerslide", "kick_turn", "ollie", "air", "grab",
    "land", "land_sketchy", "grind_5050", "grind_50", "grind_nosegrind", "boardslide", "noseslide", "tailslide",
    "darkslide", "revert", "bail", "run_out", "walk", "crouch", "wallride", "handplant", "bail_cannonball",
    "bail_swan_dive", "bail_judo_kick", "bail_torpedo",
];

pub fn clip_name(i: u32) -> &'static str {
    CLIPS.get(i as usize).copied().unwrap_or("ride")
}

fn clip_index(name: &str) -> u32 {
    CLIPS.iter().position(|c| *c == name).unwrap_or(0) as u32
}

impl Frame {
    pub fn of(sim: &Sim) -> Frame {
        let p = &sim.p;
        let (flip_angle, shuv_angle) = match p.flip {
            Some(f) => {
                let k = smoothstep(f.t / f.dur);
                (f.flip_turns * k * std::f32::consts::TAU, deg(f.shuv_deg) * k)
            }
            None => (0.0, 0.0),
        };
        let clip = match p.state {
            State::Ground => {
                if let Some(m) = p.manual {
                    if m.nose {
                        "nose_manual"
                    } else {
                        "manual"
                    }
                } else if p.pushing {
                    "push"
                } else if p.crouch > 0.5 {
                    "crouch"
                } else if p.ground_time < 0.3 && p.landing == Some(crate::sim::Landing::Sketchy) {
                    "land_sketchy"
                } else if p.ground_time < 0.3 {
                    "land"
                } else {
                    "ride"
                }
            }
            State::Powerslide => "powerslide",
            State::Revert => "revert",
            State::Air | State::KnownAir => {
                if p.grab.is_some() {
                    "grab"
                } else if p.air_time < 0.25 {
                    "ollie"
                } else {
                    "air"
                }
            }
            State::Grind => match p.grind.as_ref().map(|g| (g.family, g.nose)) {
                Some((crate::grind::GrindFamily::FiftyFifty, _)) => "grind_5050",
                Some((crate::grind::GrindFamily::FiveO, true)) => "grind_nosegrind",
                Some((crate::grind::GrindFamily::FiveO, false)) => "grind_50",
                Some((crate::grind::GrindFamily::Boardslide, _)) => "boardslide",
                Some((crate::grind::GrindFamily::Tipslide, true)) => "noseslide",
                Some((crate::grind::GrindFamily::Tipslide, false)) => "tailslide",
                Some((crate::grind::GrindFamily::Darkslide, _)) => "darkslide",
                None => "ride",
            },
            State::Wipeout => match p.wipe.map_or(0, |w| w.pose) {
                1 => "bail_cannonball",
                2 => "bail_swan_dive",
                3 => "bail_judo_kick",
                4 => "bail_torpedo",
                _ => "bail",
            },
            State::WallRide => "wallride",
            State::Handplant => "handplant",
            State::OffBoard => {
                if p.off.map_or(false, |o| o.stumble > 0.0) {
                    "run_out"
                } else {
                    "walk"
                }
            }
        };
        let rider_pos = match p.state {
            State::Wipeout => p.wipe.map_or(p.pos, |w| w.body_pos),
            State::OffBoard => p.off.map_or(p.pos, |o| o.rider_pos),
            _ => p.pos,
        };
        let seq: f32 = sim.score.sequence.tricks.iter().map(|t| t.points).sum();
        Frame {
            tick: sim.tick,
            pos: a(p.pos),
            fwd: a(p.fwd),
            up: a(p.up),
            flip_angle,
            shuv_angle,
            pitch: p.pitch,
            rider_pos: a(rider_pos),
            rider_up: a(p.rider_up),
            crouch: p.crouch,
            lean: p.lean,
            body_flip: if p.state == State::Wipeout { p.wipe.map_or(0.0, |w| w.spin) } else { p.body_flip },
            clip: clip_index(clip),
            goofy: p.goofy as u32,
            fakie: p.fakie as u32,
            state: p.state as u32,
            speed: p.speed(),
            cam_pos: a(sim.cam.pos),
            cam_target: a(sim.cam.target),
            cam_fov: sim.cam.fov_y_deg,
            score: sim.score.score,
            line: sim.score.line_total,
            line_timer: sim.score.line_timer.max(0.0),
            multiplier: sim.score.multiplier,
            sequence: seq,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::offset_of;
    /// play/main.js reads these offsets straight out of wasm memory.
    #[test]
    fn layout_is_what_the_web_reads() {
        assert_eq!(std::mem::size_of::<Frame>(), 160);
        assert_eq!(offset_of!(Frame, pos), 8);
        assert_eq!(offset_of!(Frame, flip_angle), 44);
        assert_eq!(offset_of!(Frame, rider_pos), 56);
        assert_eq!(offset_of!(Frame, crouch), 80);
        assert_eq!(offset_of!(Frame, clip), 92);
        assert_eq!(offset_of!(Frame, state), 104);
        assert_eq!(offset_of!(Frame, cam_pos), 112);
        assert_eq!(offset_of!(Frame, cam_fov), 136);
        assert_eq!(offset_of!(Frame, sequence), 156);
        assert_eq!(std::mem::size_of::<crate::input::VirtualController>(), 28);
    }
}
