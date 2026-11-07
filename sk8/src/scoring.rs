//! Scoring (spec B.14): trick values, sequences held open by carriers, landing factors,
//! lines with a timer and multiplier, and the "already been done" repetition rule.
//!
//! The lens runs this for instant feedback; the realm re-scores what it is told
//! (greenfield/world/support/skate_rules.c) and is the authority for anything kept.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Base points and points per metre / per second of a trick by its scoring key.
pub fn trick_value(key: &str) -> (f32, f32, f32) {
    match key {
        "Ollie" => (50.0, 0.0, 0.0),
        "Nollie" => (70.0, 0.0, 0.0),
        "Pop Shuvit" | "FS Pop Shuvit" => (150.0, 0.0, 0.0),
        "Kickflip" | "Heelflip" => (250.0, 0.0, 0.0),
        "Varial Kickflip" | "Varial Heelflip" => (400.0, 0.0, 0.0),
        "Hardflip" | "Inward Heelflip" => (500.0, 0.0, 0.0),
        "360 Pop Shuvit" | "FS 360 Pop Shuvit" => (400.0, 0.0, 0.0),
        "360 Flip" | "Laser Flip" => (800.0, 0.0, 0.0),
        "Late Kickflip" | "Late Heelflip" => (450.0, 0.0, 0.0),
        "Late Shuvit" | "Late FS Shuvit" => (350.0, 0.0, 0.0),
        "Front Flip" | "Back Flip" => (1500.0, 0.0, 0.0),
        "180" => (100.0, 0.0, 0.0),
        "360" => (350.0, 0.0, 0.0),
        "540" => (800.0, 0.0, 0.0),
        "720" => (1500.0, 0.0, 0.0),
        "Grab" => (150.0, 0.0, 120.0),
        "Manual" | "Nose Manual" => (100.0, 50.0, 30.0),
        "50-50" => (150.0, 60.0, 0.0),
        "5-0" | "Nosegrind" => (250.0, 70.0, 0.0),
        "Boardslide" | "Lipslide" => (200.0, 60.0, 0.0),
        "Noseslide" | "Tailslide" => (300.0, 70.0, 0.0),
        "Crooked" | "Overcrook" | "Smith" | "Feeble" | "Salad" | "Willy" | "Over Willy" => (450.0, 80.0, 0.0),
        "Bluntslide" | "Noseblunt" => (600.0, 90.0, 0.0),
        "Darkslide" => (700.0, 90.0, 0.0),
        "Powerslide" | "Revert" => (50.0, 20.0, 0.0),
        "Drop In" | "Stall" => (100.0, 0.0, 0.0),
        "Handplant" => (500.0, 0.0, 200.0),
        "Wallride" => (300.0, 60.0, 0.0),
        _ => (100.0, 0.0, 0.0),
    }
}

/// Keys exempt from repetition (B.14.2).
fn exempt(key: &str) -> bool {
    matches!(key, "Air" | "Handplant")
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ScoredTrick {
    /// The full display name ("Switch FS 180 Kickflip Indy").
    pub name: String,
    /// The scoring key ("Kickflip").
    pub key: String,
    pub points: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Sequence {
    pub tricks: Vec<ScoredTrick>,
    pub open: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ScoreEvent {
    /// A sequence banked into the line.
    Banked { names: Vec<String>, raw: f32, landing: f32, multiplier: f32, points: f32 },
    /// A sequence lost to a bail.
    Lost { names: Vec<String> },
    /// The line ended: its total became score.
    LineEnded { points: f32 },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Scoring {
    pub sequence: Sequence,
    pub line_total: f32,
    pub line_timer: f32,
    pub line_live: bool,
    pub multiplier: f32,
    pub used: HashMap<String, u32>,
    pub score: f32,
    pub best_line: f32,
    pub events: Vec<ScoreEvent>,
    idle_ticks: u32,
}

impl Scoring {
    pub fn new() -> Scoring {
        Scoring { multiplier: 1.0, ..Default::default() }
    }

    /// Repetition factor for `key` given uses in the line and in the open sequence.
    fn repetition(&self, key: &str) -> f32 {
        if exempt(key) {
            return 1.0;
        }
        let n = self.used.get(key).copied().unwrap_or(0) + self.sequence.tricks.iter().filter(|t| t.key == key).count() as u32;
        0.6f32.powi(n as i32)
    }

    /// Add a trick to the open sequence (opening one if needed). `modifier` is the product of
    /// switch / fakie / out-of-manual / out-of-grind / stationary factors.
    pub fn add(&mut self, name: &str, key: &str, points: f32, modifier: f32) -> f32 {
        let p = points * modifier * self.repetition(key);
        self.sequence.open = true;
        self.sequence.tricks.push(ScoredTrick { name: name.to_string(), key: key.to_string(), points: p });
        self.idle_ticks = 0;
        p
    }

    pub fn add_value(&mut self, name: &str, key: &str, metres: f32, seconds: f32, modifier: f32) -> f32 {
        let (base, per_m, per_s) = trick_value(key);
        self.add(name, key, base + per_m * metres + per_s * seconds, modifier)
    }

    /// Called every tick with whether any carrier is active (manual, grind, powerslide,
    /// revert, handplant, air). Returns true when the sequence closes by itself.
    pub fn tick(&mut self, dt: f32, carrier: bool, manualing: bool, landing_factor: f32, line_time: f32) {
        if self.sequence.open {
            if carrier {
                self.idle_ticks = 0;
            } else {
                self.idle_ticks += 1;
                if self.idle_ticks >= 3 {
                    self.bank(landing_factor, line_time);
                }
            }
        }
        if self.line_live {
            self.line_timer -= dt * if manualing { 0.5 } else { 1.0 };
            if self.line_timer <= 0.0 {
                self.end_line();
            }
        }
    }

    pub fn bank(&mut self, landing_factor: f32, line_time: f32) {
        if !self.sequence.open {
            return;
        }
        let raw: f32 = self.sequence.tricks.iter().map(|t| t.points).sum();
        let points = raw * landing_factor * self.multiplier;
        let names: Vec<String> = self.sequence.tricks.iter().map(|t| t.name.clone()).collect();
        for t in &self.sequence.tricks {
            *self.used.entry(t.key.clone()).or_insert(0) += 1;
        }
        self.sequence = Sequence::default();
        self.idle_ticks = 0;
        if points <= 0.0 {
            return;
        }
        self.events.push(ScoreEvent::Banked { names, raw, landing: landing_factor, multiplier: self.multiplier, points });
        self.line_total += points;
        if self.line_live || points >= 500.0 {
            self.line_live = true;
            self.line_timer = line_time;
        }
        // The multiplier rises with the line's points and never drops while it lives.
        let m = if self.line_total >= 8000.0 {
            3.0
        } else if self.line_total >= 4000.0 {
            2.0
        } else if self.line_total >= 1500.0 {
            1.5
        } else {
            1.0
        };
        self.multiplier = self.multiplier.max(m);
        if !self.line_live {
            // A small sequence with no line running is score at once, said as a line of its
            // own: the realm scores only what a line's end tells it.
            self.end_line();
        }
    }

    /// A bail: the open sequence is lost and the line ends.
    pub fn bail(&mut self) {
        if self.sequence.open {
            let names = self.sequence.tricks.iter().map(|t| t.name.clone()).collect();
            self.events.push(ScoreEvent::Lost { names });
        }
        self.sequence = Sequence::default();
        self.end_line();
    }

    pub fn end_line(&mut self) {
        if self.line_live || self.line_total > 0.0 {
            self.score += self.line_total;
            self.best_line = self.best_line.max(self.line_total);
            self.events.push(ScoreEvent::LineEnded { points: self.line_total });
        }
        self.line_total = 0.0;
        self.line_timer = 0.0;
        self.line_live = false;
        self.multiplier = 1.0;
        self.used.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_small_sequence_with_no_line_ends_as_a_line_of_its_own() {
        let mut s = Scoring::new();
        s.add_value("Ollie", "Ollie", 0.0, 0.0, 1.0);
        s.bank(1.0, 5.0);
        assert!(!s.line_live);
        assert!(s.score > 0.0);
        assert!(s.events.iter().any(|e| matches!(e, ScoreEvent::LineEnded { points } if *points == s.score)));
    }
    #[test]
    fn repeat_in_line_scores_sixty_percent() {
        let mut s = Scoring::new();
        s.add_value("Kickflip", "Kickflip", 0.0, 0.0, 1.0);
        s.add_value("FS 50-50", "50-50", 3.0, 0.0, 1.0);
        s.add_value("Manual", "Manual", 2.0, 1.0, 1.0);
        s.bank(1.0, 5.0);
        assert!(s.line_live);
        let p = s.add_value("Kickflip", "Kickflip", 0.0, 0.0, 1.0);
        assert!((p - 150.0).abs() < 1e-3);
    }
    #[test]
    fn bail_loses_sequence() {
        let mut s = Scoring::new();
        s.add_value("Kickflip", "Kickflip", 0.0, 0.0, 1.0);
        s.bail();
        assert_eq!(s.score, 0.0);
    }
}
