//! Every number in one place (spec C.1), with the difficulty sets of B.15.
//!
//! Difficulty is three data sets selected by index, never multipliers. Any field can be
//! overridden by name from a `key = value` text (the realm ships `world/skate/tuning.txt`),
//! so feel work is data, not code (D.2 ordering rule 2).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    Normal,
    Hardcore,
}

impl Difficulty {
    pub fn from_index(i: u32) -> Difficulty {
        match i {
            0 => Difficulty::Easy,
            2 => Difficulty::Hardcore,
            _ => Difficulty::Normal,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Normal => "normal",
            Difficulty::Hardcore => "hardcore",
        }
    }
}

macro_rules! tuning {
    ($( $(#[$m:meta])* $name:ident : $e:expr, $n:expr, $h:expr ;)*) => {
        /// All tunables. Values are `[default]` unless the spec marks them `[code]`.
        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct Tuning {
            pub difficulty: Difficulty,
            $( $(#[$m])* pub $name: f32, )*
        }
        impl Tuning {
            pub fn new(d: Difficulty) -> Tuning {
                let i = d as usize;
                Tuning { difficulty: d, $( $name: [$e, $n, $h][i], )* }
            }
            /// Set a field by its name; false when there is no such field.
            pub fn set(&mut self, key: &str, value: f32) -> bool {
                match key { $( stringify!($name) => { self.$name = value; true } )* _ => false }
            }
            pub fn get(&self, key: &str) -> Option<f32> {
                match key { $( stringify!($name) => Some(self.$name), )* _ => None }
            }
            pub fn names() -> &'static [&'static str] {
                &[ $( stringify!($name), )* ]
            }
        }
    };
}

tuning! {
    // Camera (B.16): the chase camera's distance from slow to 10 m/s, its height as an
    // angle, how far ahead it looks, its vertical field of view.
    cam_distance_slow: 2.0, 2.0, 2.0;
    cam_distance_fast: 2.6, 2.6, 2.6;
    cam_elevation_deg: 12.0, 12.0, 12.0;
    cam_look_ahead: 0.5, 0.5, 0.5;
    cam_fov_deg: 70.0, 70.0, 70.0;
    // Input (B.4)
    flick_low: 1.75, 1.75, 1.5;
    flick_high: 4.4, 4.4, 3.0;
    pattern_tolerance: 0.4, 0.4, 0.4;
    // Ground (B.5)
    stick_dist: 0.12, 0.12, 0.12;
    natural_air_max_dist: 0.12, 0.12, 0.12;
    natural_air_min_dist: 0.04, 0.04, 0.04;
    natural_air_time: 0.1, 0.1, 0.1;
    ground_n_smoothing: 0.3, 0.3, 0.3;
    // Speed (B.6.1)
    max_grav_accel: 6.0, 6.0, 6.0;
    max_speed_change: 0.3, 0.3, 0.3;
    rolling_friction_base: 0.35, 0.35, 0.35;
    rolling_friction_per_speed: 0.055, 0.055, 0.055;
    /// Not in the spec: a quadratic term so a 10 deg slope settles near 9 m/s (C.3 step 1);
    /// the linear `[default]` friction alone settles far faster.
    rolling_drag: 0.0035, 0.0035, 0.0035;
    // Push (B.6.2)
    max_pushable_speed: 8.5, 8.5, 8.5;
    push_dv_rest: 2.6, 2.0, 1.6;
    push_dv_max: 1.3, 1.0, 0.8;
    push_clip: 0.65, 0.65, 0.65;
    push_contact: 0.25, 0.25, 0.25;
    auto_push: 1.0, 1.0, 0.0;
    // Steer (B.6.3)
    yaw_rate_max: 2.4, 2.4, 2.4;
    grip_base: 12.0, 12.0, 12.0;
    straighten_gain: 4.0, 4.0, 4.0;
    wobble_speed: 12.0, 10.0, 8.0;
    wobble_amp: 0.1, 0.15, 0.25;
    // Brake (B.6.5)
    brake_decel: 5.0, 5.0, 5.0;
    // Slide (B.7.1)
    slide_min_speed: 2.5, 2.5, 2.5;
    /// Scrub = base + side * sin(slip). The spec's 2.2 + 3.5 takes a 90 deg slide from 8 m/s
    /// to under 3 in a second; its own check wants about 4, which 1.2 + 3.5 gives.
    slide_scrub_base: 1.2, 1.2, 1.2;
    slide_scrub_side: 3.5, 3.5, 3.5;
    // Manual (B.7.3)
    manual_max_tilt_deg: 14.0, 14.0, 14.0;
    manual_noise_deg: 1.0, 2.0, 3.5;
    // Pump (B.7.5)
    pump_factor: 1.3, 1.0, 0.6;
    pump_max_accel: 3.0, 2.2, 1.5;
    pump_unintentional: 0.5, 0.3, 0.0;
    // Pop (B.8.1)
    ollie_min_h: 0.30, 0.25, 0.20;
    ollie_max_h: 1.20, 0.95, 0.80;
    manual_min_h: 0.20, 0.15, 0.10;
    // Air (B.8.2 to B.8.6)
    traj_adjust: 1.2, 0.6, 0.2;
    max_spin_deg: 540.0, 540.0, 540.0;
    spin_accel_deg: 2200.0, 2200.0, 2200.0;
    max_auto_spin_deg: 360.0, 180.0, 0.0;
    flip_time: 0.42, 0.42, 0.42;
    // Landing and bails (B.9.3)
    land_angle_rest_deg: 70.0, 60.0, 45.0;
    land_angle_fast_deg: 45.0, 35.0, 25.0;
    impact_bail: 9.0, 7.0, 5.5;
    closing_h: 6.0, 4.5, 3.5;
    closing_v: 8.0, 6.0, 5.0;
    flip_catch_deg: 120.0, 90.0, 60.0;
    // Grind (B.10)
    grind_lock_dist: 0.9, 0.5, 0.1;
    grind_pop_min: 2.8, 2.6, 2.2;
    grind_pop_max: 4.5, 4.1, 3.6;
    grind_max_impact: 7.0, 5.5, 4.5;
    grind_lean_time: 4.0, 2.5, 1.5;
    grind_drag_base: 0.9, 0.9, 0.9;
    // Scoring (B.14.3)
    line_time: 5.0, 5.0, 5.0;
}

impl Default for Tuning {
    fn default() -> Tuning {
        Tuning::new(Difficulty::Normal)
    }
}

impl Tuning {
    /// Apply `key = value` lines; `#` starts a comment; `[easy]`/`[normal]`/`[hardcore]`
    /// sections apply only to that difficulty. Returns the keys it did not know.
    pub fn apply_text(&mut self, text: &str) -> Vec<String> {
        let mut unknown = Vec::new();
        let mut section: Option<Difficulty> = None;
        for raw in text.lines() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                section = match &line[1..line.len() - 1] {
                    "easy" => Some(Difficulty::Easy),
                    "normal" => Some(Difficulty::Normal),
                    "hardcore" => Some(Difficulty::Hardcore),
                    _ => None,
                };
                continue;
            }
            if let Some(d) = section {
                if d != self.difficulty {
                    continue;
                }
            }
            if let Some((k, v)) = line.split_once('=') {
                let k = k.trim();
                match v.trim().parse::<f32>() {
                    Ok(v) if self.set(k, v) => {}
                    _ => unknown.push(k.to_string()),
                }
            }
        }
        unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sections_and_overrides() {
        let mut t = Tuning::new(Difficulty::Hardcore);
        assert_eq!(t.ollie_max_h, 0.80);
        let unknown = t.apply_text("grip_base = 10\n[easy]\ngrip_base = 99\n[hardcore]\nollie_max_h=0.7 # lower\nnope = 1");
        assert_eq!(t.grip_base, 10.0);
        assert_eq!(t.ollie_max_h, 0.7);
        assert_eq!(unknown, vec!["nope".to_string()]);
    }
}
