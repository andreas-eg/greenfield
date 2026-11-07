//! Grinding (spec B.10): contact features and families, air attraction ("magnetism"),
//! entry, the pin, drag, lean and balance, exits and the six-fact name.

use crate::input::{self, Completion, Family, Intents};
use crate::math::*;
use crate::sim::{predict, Event, Player, Sim, State, DECK_H, DT, G, HALF_TRACK, HALF_WB};
use crate::tuning::Tuning;
use crate::world::{RailChord, RailKind, Surface, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrindFamily {
    FiftyFifty,
    FiveO,
    Tipslide,
    Boardslide,
    Darkslide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    DropIn,
    RideIntoCoping,
    RideFromBelow,
    RideFromAbove,
    AirToGrind,
}

#[derive(Clone, Debug)]
pub struct GrindState {
    pub chord: u32,
    pub family: GrindFamily,
    pub kind: RailKind,
    pub entry: Entry,
    pub t: f32,
    /// Which end touches for 5-0 and tipslides: true = nose end.
    pub nose: bool,
    pub approach_fs: bool,
    /// Horizontal unit vector from the rail toward the side the rider came from.
    pub came_from: Vec3,
    /// Horizontal unit vector from the rail toward a ledge's top (zero for rails).
    pub top_side: Vec3,
    pub lean: f32,
    pub slope_sine: f32,
    pub metres: f32,
    pub name: String,
    pub stall_t: f32,
    pub lost: u32,
    pub along_sign: f32,
}

/// The board features of B.10.2 as world segments, with their depth limits.
struct Features {
    front: (Vec3, Vec3),
    rear: (Vec3, Vec3),
    middle: (Vec3, Vec3),
    nose: (Vec3, Vec3),
    tail: (Vec3, Vec3),
}

fn features(p: &Player) -> Features {
    let r = p.right();
    let base = p.pos - p.up * DECK_H;
    let f = p.fwd;
    Features {
        front: (base + f * HALF_WB - r * HALF_TRACK, base + f * HALF_WB + r * HALF_TRACK),
        rear: (base - f * HALF_WB - r * HALF_TRACK, base - f * HALF_WB + r * HALF_TRACK),
        middle: (base - f * HALF_WB, base + f * HALF_WB),
        nose: (base + f * HALF_WB, base + f * 0.458),
        tail: (base - f * 0.458, base - f * HALF_WB),
    }
}

/// Does a feature segment touch the chord? (B.10.2): the feature may sit up to `depth`
/// below the rail top and a little above it, with the rail passing under it.
fn touches(seg: (Vec3, Vec3), c: &RailChord, up: Vec3, depth: f32, above: f32) -> bool {
    let (s, t) = closest_segment_segment(seg.0, seg.1, c.a, c.b);
    let fp = seg.0 + (seg.1 - seg.0) * s;
    let rp = c.a + (c.b - c.a) * t;
    let d = fp - rp;
    let h = d.dot(up);
    let lateral = (d - up * h).len();
    h >= -depth && h <= above && lateral <= 0.06
}

/// Classify what touches which chord (B.10.2), in the spec's priority order.
fn classify(w: &World, p: &Player, it: &Intents, current: Option<&GrindState>, above: f32) -> Option<(u32, GrindFamily, bool)> {
    let f = features(p);
    let lo = p.pos - v3(1.2, 1.2, 1.2);
    let hi = p.pos + v3(1.2, 1.2, 1.2);
    let ids = w.rails_in_box(lo, hi);
    let inverted = p.up.y < -0.45;
    let mut found: Option<(u32, GrindFamily, bool)> = None;
    for id in ids {
        let c = &w.chords[id as usize];
        let mut fr = touches(f.front, c, p.up, 0.13, above);
        let mut rr = touches(f.rear, c, p.up, 0.13, above);
        // Landing flat along a rail, the trucks arrive a tick apart: the second one a few
        // centimetres up still counts, or every 50-50 from the air would start as a 5-0.
        if fr != rr && current.is_none() && p.right().dot(c.dir).abs() < 0.5 {
            fr = fr || touches(f.front, c, p.up, 0.13, above + 0.1);
            rr = rr || touches(f.rear, c, p.up, 0.13, above + 0.1);
        }
        let mid = touches(f.middle, c, p.up, 0.06, above);
        let ns = touches(f.nose, c, p.up, 0.08, above);
        let tl = touches(f.tail, c, p.up, 0.08, above);
        let across = p.fwd.dot(c.dir).abs() < deg(50.0).cos();
        let mut fam: Option<(GrindFamily, bool)> = None;
        if inverted {
            let dark_mid = touches((p.pos - p.fwd * HALF_WB, p.pos + p.fwd * HALF_WB), c, -p.up, 0.075, above);
            if dark_mid && (-p.up).dot(c.normal) > 0.45 {
                fam = Some((GrindFamily::Darkslide, false));
            }
        } else if fr && rr && it.grind_nudge.abs() < 0.9 && it.grind_translate.abs() < 0.9 {
            fam = Some((GrindFamily::FiftyFifty, false));
        } else if (fr ^ rr) && p.right().dot(c.dir).abs() < 0.906 && p.up.dot(c.dir).abs() < 0.46 {
            fam = Some((GrindFamily::FiveO, fr));
        } else if (ns || tl) && !mid && across {
            fam = Some((GrindFamily::Tipslide, ns));
        } else if mid && c.normal.dot(p.up) > 0.65 && across {
            fam = Some((GrindFamily::Boardslide, false));
        }
        if let Some((fm, nose)) = fam {
            // The current family on the current rail wins while it holds.
            if let Some(g) = current {
                if w.chords[g.chord as usize].rail == c.rail && fm == g.family {
                    return Some((id, fm, nose));
                }
            }
            if found.is_none() {
                found = Some((id, fm, nose));
            }
        }
    }
    found
}

/// Look for a grind to start (every tick, all on-board states except grinding).
pub fn detect(sim: &mut Sim, it: &Intents) {
    let p = &sim.p;
    if p.grind_block > 0 || p.anti_flicker_block > 0 || !p.on_board() {
        return;
    }
    let in_air = matches!(p.state, State::Air | State::KnownAir);
    if in_air && p.air_time < 0.08 {
        return;
    }
    if !in_air && p.state != State::Ground {
        return;
    }
    let above = if in_air { 0.08 } else { 0.03 };
    let Some((chord, family, nose)) = classify(&sim.world, p, it, None, above) else { return };
    let c = sim.world.chords[chord as usize];
    // Coming down onto the rail from the air, never rising through it; and with the board
    // roughly upright to it, unless the prediction aimed at this very rail. A vert air
    // falling back past the coping re-enters the wall, it does not stall on the lip.
    if in_air && p.vel.dot(c.normal) > 0.5 {
        return;
    }
    let aimed = p.traj.and_then(|t| t.grind_chord).map_or(false, |a| sim.world.chords[a as usize].rail == c.rail);
    if in_air && !aimed && p.up.dot(c.normal) < 0.3 {
        return;
    }
    enter(sim, chord, family, nose, in_air);
}

fn enter(sim: &mut Sim, chord: u32, family: GrindFamily, nose: bool, from_air: bool) {
    let t = sim.t.clone();
    let w = &sim.world;
    let p = &mut sim.p;
    let c = w.chords[chord as usize];
    let along = c.dir * p.vel.dot(c.dir);
    let across = p.vel - along;
    let speed = p.vel.len();
    let slope_sine = if across.len() > 1e-3 { (across.y / across.len()).abs() } else { 0.0 };
    let entry = if from_air {
        Entry::AirToGrind
    } else if speed < 1.45 {
        Entry::DropIn
    } else if slope_sine > 0.96 {
        Entry::RideIntoCoping
    } else if slope_sine > 0.25 {
        Entry::RideFromBelow
    } else {
        Entry::RideFromAbove
    };
    if entry == Entry::RideFromAbove {
        let hv = p.vel.planar();
        if angle_between(hv, c.dir.planar()).min(angle_between(hv, -c.dir.planar())) > deg(17.0) {
            return;
        }
    }
    if matches!(entry, Entry::RideFromBelow | Entry::RideIntoCoping) {
        let ratio = if speed > 1e-3 { across.len() / speed } else { 0.0 };
        if across.len() > (1.0 - ratio) * 2.4 + 2.8 {
            return;
        }
    }
    // Anti-flicker (B.10.2).
    p.anti_flicker += match family {
        GrindFamily::Boardslide | GrindFamily::Tipslide | GrindFamily::Darkslide => 50.0,
        GrindFamily::FiveO => 20.0,
        _ => 0.0,
    };
    if p.anti_flicker > 151.0 {
        p.anti_flicker = 0.0;
        p.anti_flicker_block = 90;
        return;
    }
    if p.up.dot(c.dir).abs() > 0.5 && family != GrindFamily::Tipslide {
        sim.bail("bad grind entry");
        return;
    }
    let keep = if from_air {
        match family {
            GrindFamily::Boardslide | GrindFamily::Darkslide => 0.8,
            GrindFamily::Tipslide => 0.2,
            _ => 0.6,
        }
    } else {
        1.0 - match family {
            GrindFamily::Boardslide => 0.1,
            GrindFamily::Tipslide => 0.4,
            _ => 0.2,
        }
    };
    let impact = across.len();
    if impact * (1.0 - keep) > t.grind_max_impact && p.grace <= 0.0 {
        sim.bail("slammed into rail");
        return;
    }
    if across.planar().len() > 7.0 && c.kind == RailKind::ThinRail && p.grace <= 0.0 {
        sim.bail("slammed into rail");
        return;
    }
    // Where the rider came from, and a ledge's top side.
    let rp = c.a + c.dir * clamp((p.pos - c.a).dot(c.dir), 0.0, c.len);
    let from = if from_air { p.air.takeoff } else { p.pos - p.vel * 0.5 };
    let perp = |v: Vec3| v.planar().on_plane(c.dir.planar().norm_or(c.dir)).norm_or(ZERO);
    let came_from = perp(from - rp);
    let side = perp(c.normal.cross(c.dir));
    let mut top_side = ZERO;
    if matches!(c.kind, RailKind::Ledge | RailKind::Curb | RailKind::Coping) {
        let mut tops = [false, false];
        for (i, s) in [side, -side].iter().enumerate() {
            if let Some(h) = w.raycast(rp + *s * 0.15 + UP * 0.2, -UP, 0.35) {
                tops[i] = h.point.y > rp.y - 0.06;
            }
        }
        top_side = match tops {
            [true, false] => side,
            [false, true] => -side,
            _ => ZERO,
        };
    }
    let approach_fs = {
        let rail_to_rider = (from - rp).planar();
        p.chest().planar().dot(rail_to_rider) < 0.0
    };
    p.vel = along + across * keep;
    p.speed_target = 0.0;
    p.manual = None;
    let along_sign = if p.fwd.dot(c.dir) >= 0.0 { 1.0 } else { -1.0 };
    let mut g = GrindState {
        chord,
        family,
        kind: c.kind,
        entry,
        t: 0.0,
        nose,
        approach_fs,
        came_from,
        top_side,
        lean: 0.0,
        slope_sine,
        metres: 0.0,
        name: String::new(),
        stall_t: 0.0,
        lost: 0,
        along_sign,
    };
    g.name = name(p, &g, &c);
    p.flip = None;
    p.grab = None;
    let name = g.name.clone();
    p.grind = Some(g);
    p.set_state(State::Grind);
    sim.events.push(Event::GrindStart { name });
    let _ = DT;
}

/// The six-fact grind name (B.10.7).
pub fn name(p: &Player, g: &GrindState, c: &RailChord) -> String {
    let fsbs = if g.approach_fs { "FS" } else { "BS" };
    let fd = p.fwd.on_plane(c.normal).norm_or(p.fwd);
    let straight = fd.dot(c.dir).abs() > 0.9397;
    let low = c.normal.dot(p.fwd * if g.nose { 1.0 } else { -1.0 }) <= -0.1;
    let base = match g.family {
        GrindFamily::FiftyFifty => "50-50".to_string(),
        GrindFamily::Darkslide => "Darkslide".to_string(),
        GrindFamily::Boardslide => {
            // The nose crossed over the rail to get on: boardslide; the tail: lipslide.
            let nose_side = (p.pos + p.fwd * 0.3 - c.a).planar().dot(g.came_from);
            if nose_side < 0.0 { "Boardslide".into() } else { "Lipslide".into() }
        }
        GrindFamily::Tipslide => match (g.nose, low) {
            (true, false) => "Noseslide".into(),
            (true, true) => "Noseblunt".into(),
            (false, false) => "Tailslide".into(),
            (false, true) => "Bluntslide".into(),
        },
        GrindFamily::FiveO => {
            if g.nose {
                if low {
                    "Willy".into()
                } else if straight {
                    "Nosegrind".into()
                } else {
                    // Tail to the heel side: crooked; to the toe side: overcrook.
                    let tail_side = (-p.fwd).dot(p.chest());
                    if tail_side < 0.0 { "Crooked".into() } else { "Overcrook".into() }
                }
            } else if low {
                // The front truck hangs on the near side: smith; the far side: feeble.
                let front_side = (p.fwd).planar().dot(g.came_from);
                if front_side > 0.0 { "Smith".into() } else { "Feeble".into() }
            } else if straight {
                "5-0".into()
            } else {
                "Salad".into()
            }
        }
    };
    format!("{} {}", fsbs, base)
}

/// The scoring key: the name without FS/BS.
pub fn key(name: &str) -> &str {
    name.split_once(' ').map(|(_, k)| k).unwrap_or(name)
}

fn base_drag(g: &GrindState) -> f32 {
    let thin = g.kind == RailKind::ThinRail;
    match g.family {
        GrindFamily::FiftyFifty | GrindFamily::FiveO => {
            if thin {
                0.9
            } else {
                0.75
            }
        }
        GrindFamily::Boardslide | GrindFamily::Darkslide => {
            if g.kind == RailKind::Ledge {
                1.1
            } else {
                1.0
            }
        }
        GrindFamily::Tipslide => 1.1,
    }
}

/// One tick of grinding (B.10.5) and its exits (B.10.6).
pub fn grind_tick(sim: &mut Sim, it: &Intents, completion: Option<Completion>) {
    let t = sim.t.clone();
    let Some(mut g) = sim.p.grind.clone() else {
        sim.p.set_state(State::Air);
        return;
    };
    let w = &sim.world;
    let mut c = w.chords[g.chord as usize];
    g.t += DT;

    // Pop out (B.10.6).
    if let Some(cmp) = completion {
        let ge = input::apply_stance(cmp.gesture, sim.p.goofy, sim.p.switch, sim.p.fakie);
        if matches!(cmp.gesture.family(), Family::Ollie | Family::Nollie) {
            let p = &mut sim.p;
            let mut n = (UP * 0.5 + c.normal * 0.5).norm();
            if g.family == GrindFamily::Tipslide {
                n = (n + g.came_from * -0.29).norm();
            }
            let (lo, hi) = match g.family {
                GrindFamily::Boardslide => (t.grind_pop_min - 0.2, t.grind_pop_max - 0.3),
                GrindFamily::Tipslide => (t.grind_pop_min - 0.4, t.grind_pop_max - 0.6),
                _ => (t.grind_pop_min, t.grind_pop_max),
            };
            let mut strength = cmp.strength;
            let mut bal = it.grind_balance;
            if p.jump_energy < 0.31 {
                bal = 0.42 * sign(bal);
                strength *= 0.5;
            }
            p.jump_energy -= 0.6;
            let vj = lerp(lo, hi, strength);
            let side = if g.kind == RailKind::Ledge && matches!(g.family, GrindFamily::Boardslide | GrindFamily::Tipslide) { 0.9 } else { 0.5 };
            let across_dir = c.normal.cross(c.dir).norm();
            let mut lateral = across_dir * (bal * 1.8) - g.top_side * side;
            if lateral.len() > 1.8 {
                lateral = lateral.norm() * 1.8;
            }
            p.vel = p.vel.on_plane(n) + n * vj * 1.03 + lateral;
            finish(sim, &g, &c, "pop");
            let p = &mut sim.p;
            p.grind_block = 10;
            p.ground_n = UP;
            let dark = false;
            crate::sim::begin_air_pub(p, ge, false, true, dark, &t);
            p.jump_ticks = 0;
            sim.events.push(Event::Pop { height: vj * vj / (2.0 * G), strength });
            predict(&sim.world, &mut sim.p, &t, true, true);
            return;
        }
    }

    let p = &mut sim.p;
    // Family changes on the same rail by lifting an end (within 24 deg).
    if g.family == GrindFamily::FiftyFifty && it.grind_up_down.abs() > 0.6 {
        g.family = GrindFamily::FiveO;
        g.nose = it.grind_up_down > 0.0;
        g.name = name(p, &g, &c);
        sim.events.push(Event::GrindStart { name: g.name.clone() });
    } else if g.family == GrindFamily::FiveO && it.grind_up_down.abs() < 0.2 && g.entry == Entry::AirToGrind && g.t > 0.3 {
        g.family = GrindFamily::FiftyFifty;
        g.name = name(p, &g, &c);
        sim.events.push(Event::GrindStart { name: g.name.clone() });
    }

    // Target frame and gradual alignment (10 % a tick).
    let fwd_along = c.dir * g.along_sign;
    let (tf, tu) = match g.family {
        GrindFamily::FiftyFifty | GrindFamily::FiveO => {
            let yaw = -it.lx * 0.68 * 0.5;
            (rotate(fwd_along, c.normal, yaw), c.normal)
        }
        GrindFamily::Boardslide | GrindFamily::Darkslide => {
            let a = c.normal.cross(c.dir);
            let f = if a.dot(p.fwd) >= 0.0 { a } else { -a };
            (f, if g.family == GrindFamily::Darkslide { -c.normal } else { c.normal })
        }
        GrindFamily::Tipslide => {
            let a = c.normal.cross(c.dir);
            let f = if a.dot(p.fwd) >= 0.0 { a } else { -a };
            let sgn = if g.nose { 1.0 } else { -1.0 };
            (f, rotate(c.normal, c.dir, deg(20.0) * sgn * sign(f.dot(c.normal.cross(c.dir)))))
        }
    };
    let mut tu = tu;
    // Ledge lean (B.10.5 balance).
    let ledge = matches!(g.kind, RailKind::Ledge | RailKind::Curb) || (g.family == GrindFamily::Tipslide && g.kind == RailKind::FatRail);
    if ledge {
        let delay = match g.entry {
            Entry::RideIntoCoping => 0.1,
            Entry::RideFromBelow => 0.8,
            _ => 2.0,
        };
        let mut lean = deg(28.0) * sat((g.t - delay) / t.grind_lean_time);
        // Pushing the stick toward the ledge's top side holds the lean off.
        let toward_top = it.lx * p.right().planar().dot(g.top_side);
        lean -= toward_top * deg(10.0);
        lean = lean.max(0.0);
        g.lean = lean;
        if g.top_side.len2() > 0.0 {
            let axis = c.dir;
            let sgn = sign(c.normal.cross(axis).dot(g.top_side));
            tu = rotate(tu, axis, -lean * sgn);
        }
    }
    p.fwd = orthonormal_fwd(p.fwd.lerp(tf, 0.1), p.up);
    p.up = p.up.lerp(tu, 0.1).norm();
    p.fwd = orthonormal_fwd(p.fwd, p.up);
    p.rider_up = rotate_toward(p.rider_up, UP.lerp(c.normal, 0.5).norm(), deg(4.0), 1.0);
    p.pitch = match g.family {
        GrindFamily::FiveO => deg(10.0) * if g.nose { 1.0 } else { -1.0 },
        _ => it.grind_up_down.powi(3) * 0.2,
    };

    // The pin: keep the family's contact point on the rail.
    let base = p.pos - p.up * DECK_H;
    let contact = match g.family {
        GrindFamily::FiveO => base + p.fwd * HALF_WB * if g.nose { 1.0 } else { -1.0 },
        GrindFamily::Tipslide => base + p.fwd * 0.35 * if g.nose { 1.0 } else { -1.0 },
        GrindFamily::Darkslide => p.pos,
        _ => base,
    };
    let mut s = (contact - c.a).dot(c.dir);
    // Carry over to the next chord of the polyline.
    let mut guard = 0;
    while (s > c.len || s < 0.0) && guard < 4 {
        guard += 1;
        let step = if s > c.len { 1 } else { -1 };
        match w.next_chord(g.chord, step) {
            Some(nc) => {
                let ncv = w.chords[nc as usize];
                if angle_between(ncv.dir, c.dir) > deg(60.0) {
                    break;
                }
                g.chord = nc;
                c = ncv;
                g.along_sign = if p.fwd.dot(c.dir) >= 0.0 { 1.0 } else { -1.0 };
                s = (contact - c.a).dot(c.dir);
            }
            None => break,
        }
    }
    if s > c.len + 0.05 || s < -0.05 {
        // Ran off the end of an open rail.
        let drop_short = matches!(g.kind, RailKind::Ledge | RailKind::Curb);
        finish(sim, &g, &c, "end of the rail");
        let p = &mut sim.p;
        p.grind_block = 10;
        crate::sim::begin_air_pub(p, input::Gesture::Ollie, false, true, false, &t);
        p.air.pop = None;
        let _ = drop_short;
        predict(&sim.world, p, &t, false, true);
        return;
    }
    let rp = c.a + c.dir * s;
    let mut off = contact - rp;
    off -= c.dir * off.dot(c.dir);
    let pin = if matches!(g.family, GrindFamily::Boardslide | GrindFamily::Darkslide) { 0.2 } else { 0.35 };
    p.pos -= off * pin;
    let perp = p.vel - c.dir * p.vel.dot(c.dir);
    p.vel -= perp * 0.3;
    // Drag and gravity along the rail.
    let load = if p.crouch > 0.7 { 1.3 } else { 1.0 };
    let drag = t.grind_drag_base / 0.9 * base_drag(&g) * c.material.drag() * (1.0 + 0.35 * g.t) * load;
    let along = p.vel.dot(c.dir);
    p.vel -= c.dir * sign(along) * (drag * DT).min(along.abs());
    p.vel += c.dir * (-G * c.dir.y) * DT;
    p.pos += p.vel * DT;
    let along = p.vel.dot(c.dir);
    g.metres += along.abs() * DT;
    p.crouch = lerp(p.crouch, (p.fwd.dot(c.normal).abs() * 0.4).max(it.crouch), 0.2);
    p.com_height = lerp(1.0, 0.62, p.crouch);

    // Exits by balance.
    let mut fall: Option<(Vec3, &str)> = None;
    if ledge && g.lean > deg(28.0) - 1e-4 && g.top_side.len2() > 0.0 {
        fall = Some((-g.top_side, "leaned off"));
    }
    if ledge && g.lean > deg(15.0) {
        let probe_at = rp - g.top_side * 0.1;
        if w.raycast(probe_at, -UP, 5.0).map_or(true, |h| h.surface == Surface::Unrideable) {
            fall = Some((-g.top_side, "leaned off"));
        }
    }
    if g.family == GrindFamily::FiftyFifty && (it.grind_nudge.abs() >= 0.9 || it.grind_translate.abs() >= 0.9) {
        let side = c.normal.cross(c.dir).norm() * sign(it.grind_translate + it.grind_nudge);
        // The stick is screen-relative; the board's right is close enough to the screen's.
        let side = if p.right().dot(side) >= 0.0 { side } else { -side };
        fall = Some((side.planar().norm_or(side), "leaned off"));
    }
    // Coping support, then falling back in (B.10.5, B.11 point 5).
    // slopeSine only means riding up into it: from the air every across-velocity is steep.
    let coping = matches!(g.entry, Entry::RideIntoCoping | Entry::DropIn) || (g.entry != Entry::AirToGrind && g.slope_sine > 0.85);
    if coping {
        let relief = clamp(0.6 + 0.08 * along.abs(), 0.3, 1.2);
        if g.t > relief + 0.6 {
            fall = Some((if g.came_from.len2() > 0.0 { g.came_from } else { -g.top_side }, "stalled"));
        }
    } else if along.abs() < 0.15 {
        g.stall_t += DT;
        if g.stall_t > if ledge { 4.0 } else { 1.0 } {
            fall = Some((if g.top_side.len2() > 0.0 { -g.top_side } else { g.came_from }, "stalled"));
        }
    } else {
        g.stall_t = 0.0;
    }
    // Contact checks: the deck far from the rail means it has gone.
    if off.len() > 0.35 {
        g.lost += 1;
    } else {
        g.lost = 0;
    }
    if g.lost > 3 {
        fall = Some((g.came_from, "lost the rail"));
    }

    if let Some((dir, reason)) = fall {
        let boardslide_round = g.family == GrindFamily::Boardslide && g.kind == RailKind::ThinRail && !coping && g.lost == 0;
        finish(sim, &g, &c, reason);
        let p = &mut sim.p;
        p.vel += dir * 1.5 + UP * 1.0;
        p.grind_block = 10;
        if boardslide_round && fall.is_some() && it.grind_nudge.abs() >= 0.9 {
            p.vel += dir * 2.5;
            sim.bail("fell off the rail");
            return;
        }
        crate::sim::begin_air_pub(p, input::Gesture::Ollie, false, true, false, &t);
        p.air.pop = None;
        p.rider_up = UP.lerp(p.rider_up, 0.5).norm();
        predict(&sim.world, &mut sim.p, &t, false, true);
        return;
    }
    sim.p.grind = Some(g);
}

/// Bank the grind into the sequence and leave the grind state.
fn finish(sim: &mut Sim, g: &GrindState, _c: &RailChord, reason: &str) {
    let k = key(&g.name).to_string();
    let coping_stall = matches!(g.entry, Entry::RideIntoCoping | Entry::DropIn) && g.metres < 0.3;
    if coping_stall {
        sim.score.add_value(&format!("{} Stall", g.name), "Stall", 0.0, 0.0, 1.0);
    } else {
        sim.score.add_value(&g.name, &k, g.metres, 0.0, 1.0);
    }
    sim.events.push(Event::GrindEnd { name: g.name.clone(), metres: g.metres, reason: reason.to_string() });
    sim.p.grind = None;
}

/// Air attraction (B.10.3) for one candidate velocity `c` from `start`: the chord to land
/// on, the velocity correction, the miss distance and whether the board lands across it.
pub fn magnet(w: &World, p: &Player, t: &Tuning, start: Vec3, c: Vec3, landing: Vec3) -> Option<(u32, Vec3, f32, bool)> {
    let ids = w.rails_in_box(landing + v3(-2.0, -0.5, -2.0), landing + v3(2.0, 4.0, 2.0));
    let ids2 = w.rails_in_box(start + v3(-2.0, -0.5, -2.0), start + v3(2.0, 4.0, 2.0));
    let mut all: Vec<u32> = ids;
    for i in ids2 {
        if !all.contains(&i) {
            all.push(i);
        }
    }
    let g = v3(0.0, -G, 0.0);
    // An air going almost straight up and down (out of vert) is not steered onto a rail:
    // it comes back into the wall (B.11 point 4).
    if c.planar().len() < 1.5 {
        return None;
    }
    let mut best: Option<(f32, f32, u32, Vec3, f32, bool)> = None;
    for id in all {
        let ch = &w.chords[id as usize];
        if ch.dir.y.abs() > 0.9 {
            continue;
        }
        if ch.a.y.max(ch.b.y) < landing.y - 0.4 {
            continue;
        }
        let n = ch.normal;
        let p0 = ch.a + n * (DECK_H + 0.03);
        let qa = 0.5 * g.dot(n);
        let qb = c.dot(n);
        let qc = (start - p0).dot(n);
        let disc = qb * qb - 4.0 * qa * qc;
        if disc < 0.0 || qa.abs() < 1e-6 {
            continue;
        }
        let sq = disc.sqrt();
        let r1 = (-qb - sq) / (2.0 * qa);
        let r2 = (-qb + sq) / (2.0 * qa);
        // The descending crossing is the later root.
        let tt = r1.max(r2);
        if !(0.1..3.0).contains(&tt) {
            continue;
        }
        let x = start + c * tt + g * (0.5 * tt * tt);
        let s = (x - ch.a).dot(ch.dir);
        if s < -0.1 * ch.len || s > 1.1 * ch.len {
            continue;
        }
        let mut miss = (x - ch.a) - ch.dir * s;
        miss -= n * miss.dot(n);
        let dist = miss.len();
        let v_at = c + g * tt;
        if v_at.dot(-n) > 8.0 {
            continue;
        }
        let side_speed = v_at.on_plane(n).dot(miss.norm_or(ZERO)).abs();
        let side_limit = if ch.kind == RailKind::Ledge { 6.0 } else { 7.0 };
        if side_speed > side_limit {
            continue;
        }
        let mut approach = angle_between(c.planar(), ch.dir.planar());
        if approach > deg(90.0) {
            approach = deg(180.0) - approach;
        }
        let mut lock = t.grind_lock_dist;
        if ch.kind == RailKind::Ledge {
            // Coming at a ledge from its low side locks less. Read as dropping onto it from
            // above: applied to every ollie up onto a ledge it would contradict C.3 step 6
            // (a 0.4 m miss locks on normal, whose lock distance is 0.5).
            if start.y > ch.a.y.max(ch.b.y) + 0.3 {
                lock *= 0.6;
            }
        }
        lock = lock.max(0.1);
        if dist >= lock {
            continue;
        }
        let height = ch.a.y.max(ch.b.y);
        let better = match best {
            None => true,
            Some((ba, bh, _, _, bd, _)) => {
                if height > bh + 0.3 {
                    approach < ba + deg(20.0)
                } else if bh > height + 0.3 {
                    approach + deg(20.0) < ba
                } else if (approach - ba).abs() < deg(2.0) {
                    // The two edges of one ledge: the nearer one.
                    dist < bd
                } else {
                    approach < ba
                }
            }
        };
        if better {
            let corr = -miss / tt;
            best = Some((approach, height, id, corr, dist, approach > deg(45.0)));
        }
    }
    let _ = p;
    best.map(|(_, _, id, corr, dist, across)| (id, corr, dist, across))
}
