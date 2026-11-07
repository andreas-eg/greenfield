//! The C ABI the hosts call: the lens (Rust can use the crate directly), a Swift shell on
//! iOS and macOS (staticlib), and the browser (cdylib built for wasm32-unknown-unknown).
//!
//! Lifecycle: `sk8_new(spot_json)` -> per frame `sk8_advance(dt, samples)` -> read
//! `sk8_frame` and drain `sk8_events` (JSON lines, one event per line) -> `sk8_free`.

use crate::frame::Frame;
use crate::input::VirtualController;
use crate::sim::Sim;
use crate::tuning::Difficulty;
use crate::world::World;

pub struct Handle {
    sim: Sim,
    frame: Frame,
    prev: Frame,
    events: Vec<u8>,
}

/// Build a sim from a spot (`*.skate.json`, UTF-8, not NUL-terminated). Null on a bad file.
///
/// # Safety
/// `json` must point to `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn sk8_new(json: *const u8, len: usize, difficulty: u32) -> *mut Handle {
    let bytes = std::slice::from_raw_parts(json, len);
    let Ok(text) = std::str::from_utf8(bytes) else { return std::ptr::null_mut() };
    let Ok(world) = World::from_json(text) else { return std::ptr::null_mut() };
    let sim = Sim::new(world, Difficulty::from_index(difficulty));
    let frame = Frame::of(&sim);
    Box::into_raw(Box::new(Handle { sim, frame, prev: frame, events: Vec::new() }))
}

/// # Safety
/// `h` must come from `sk8_new` and not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn sk8_free(h: *mut Handle) {
    if !h.is_null() {
        drop(Box::from_raw(h));
    }
}

/// Override tunables with `key = value` lines (the realm's tuning text).
///
/// # Safety
/// `h` valid; `text` points to `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn sk8_tune(h: *mut Handle, difficulty: u32, text: *const u8, len: usize) {
    let h = &mut *h;
    let s = std::str::from_utf8(std::slice::from_raw_parts(text, len)).unwrap_or("");
    h.sim.set_difficulty(Difficulty::from_index(difficulty), s);
}

/// # Safety
/// `h` valid.
#[no_mangle]
pub unsafe extern "C" fn sk8_set_stance(h: *mut Handle, goofy: u32) {
    (*h).sim.p.goofy = goofy != 0;
}

/// # Safety
/// `h` valid.
#[no_mangle]
pub unsafe extern "C" fn sk8_set_sample_rate(h: *mut Handle, hz: f32) {
    (*h).sim.set_sample_rate(hz);
}

/// Run real time `dt` with every controller sample since the last call. Returns the number
/// of fixed ticks taken.
///
/// # Safety
/// `h` valid; `samples` points to `n` controllers.
#[no_mangle]
pub unsafe extern "C" fn sk8_advance(h: *mut Handle, dt: f32, samples: *const VirtualController, n: usize) -> u32 {
    let h = &mut *h;
    let s = if n == 0 || samples.is_null() { &[][..] } else { std::slice::from_raw_parts(samples, n) };
    let ticks = h.sim.advance(dt, s);
    if ticks > 0 {
        h.prev = h.sim.prev_frame.unwrap_or(h.frame);
        h.frame = Frame::of(&h.sim);
        for e in h.sim.take_events() {
            if let Ok(line) = serde_json::to_string(&e) {
                h.events.extend_from_slice(line.as_bytes());
                h.events.push(b'\n');
            }
        }
    }
    ticks
}

/// The latest tick's frame.
///
/// # Safety
/// `h` valid; the pointer lives until the next `sk8_advance`.
#[no_mangle]
pub unsafe extern "C" fn sk8_frame(h: *const Handle) -> *const Frame {
    &(*h).frame
}

/// The tick before it, for interpolation by `sk8_alpha`.
///
/// # Safety
/// As `sk8_frame`.
#[no_mangle]
pub unsafe extern "C" fn sk8_prev_frame(h: *const Handle) -> *const Frame {
    &(*h).prev
}

/// How far between the previous and the latest tick the display is (0..1).
///
/// # Safety
/// `h` valid.
#[no_mangle]
pub unsafe extern "C" fn sk8_alpha(h: *const Handle) -> f32 {
    (*h).sim.accumulator / crate::sim::DT
}

/// Pending events as JSON lines; copies at most `cap` bytes into `out` and returns the
/// number of bytes pending (call with cap 0 to size the buffer). Copied bytes are consumed.
///
/// # Safety
/// `h` valid; `out` points to `cap` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn sk8_events(h: *mut Handle, out: *mut u8, cap: usize) -> usize {
    let h = &mut *h;
    let n = h.events.len();
    if cap == 0 || out.is_null() {
        return n;
    }
    let k = n.min(cap);
    std::ptr::copy_nonoverlapping(h.events.as_ptr(), out, k);
    h.events.drain(..k);
    n
}

/// Put the board at a named mark of the spot (a session start), or the spawn.
///
/// # Safety
/// `h` valid; `name` points to `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn sk8_place_at(h: *mut Handle, name: *const u8, len: usize) -> u32 {
    let h = &mut *h;
    let n = std::str::from_utf8(std::slice::from_raw_parts(name, len)).unwrap_or("");
    let mark = h.sim.world.mark(n).map(|m| (m.pos, m.heading));
    let (pos, heading) = match mark {
        Some((p, hd)) => (crate::math::v3(p[0], p[1], p[2]), hd),
        None => (h.sim.world.spawn, h.sim.world.spawn_heading),
    };
    h.sim.place(pos, crate::math::v3(heading.sin(), 0.0, heading.cos()));
    mark.is_some() as u32
}

/// Allocation helpers for hosts without their own allocator view (the browser).
#[no_mangle]
pub extern "C" fn sk8_alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// # Safety
/// `p` must come from `sk8_alloc(len)`.
#[no_mangle]
pub unsafe extern "C" fn sk8_dealloc(p: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(p, 0, len.max(1)));
}

#[no_mangle]
pub extern "C" fn sk8_frame_size() -> usize {
    std::mem::size_of::<Frame>()
}
