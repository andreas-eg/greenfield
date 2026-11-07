# sk8

Beargarden's skate simulation: the sk8r specification (`~/src/eg/sk8t/skateboard.md`) as a
fixed-step (1/60 s) library with no platform code.

| Module | Spec |
|---|---|
| `input.rs` | virtual controller, dead zone, the Flickit pattern set and matcher, flick strength, stance mirroring, intents (B.2.3, B.4) |
| `world.rs` | triangles with surface categories, rail chords, the spot file format, raycast, sphere and rail queries (B.2.1, B.3) |
| `sim.rs` | probes and ground normal, rider up, the target-speed model, push, steer and grip, brake, kick turn, powerslide, revert, manual, crouch and pump, pop, trajectory prediction, air spin and auto heading, flips and grabs, touchdown, landing quality, bails and run-out, wipeout, respawn and checkpoints, session marker, off board, camera (B.5 to B.9, B.11 to B.13, B.16) |
| `grind.rs` | contact features and families, air magnetism, entry, pin, drag, ledge lean, exits, the grind name (B.10) |
| `sim.rs` (also) | wall ride, kerb ride-over (B.6.6), handplant (B.11), Hall of Meat (B.13.1) |
| `scoring.rs` | trick values, sequences and carriers, lines, multiplier, repetition (B.14) |
| `tuning.rs` | every number, the three difficulty sets, `key = value` overrides (C.1, B.15) |
| `frame.rs` | what a renderer reads each tick, a `repr(C)` struct |
| `ffi.rs` | the C ABI for Swift and the browser |
| `build.rs`, `spots.rs` | the spot builder and Beargarden's spots; `src/bin/spots.rs` writes them into the realm |

`cargo test` runs the unit tests and `tests/acceptance.rs`, the spec's C.3 checks ridden with
scripted controllers: coasting a 10° hill, the carve radius, a bank and back, a bowl wall to
vertical, pushes, braking, the powerslide, the kick turn, ollie height from flick speed, a bank
drop landing clean, a quarter-pipe air keeping its speed, sketchy and bailed landings, a
kickflip by its shape, a 180 landing fakie, a 50-50 locked from 0.4 m off line, a boardslide,
leaning off, the ledge lean-off at about 4.5 s, a grind pop clearing 0.8 m, pumping, a run-out,
respawn, and a banked kickflip.

Builds: `cargo build --release --target wasm32-unknown-unknown --lib` for the web
(`../play/build.sh`), `--target aarch64-apple-ios` for iOS, the host for macOS.

## Where it departs from the spec, and why

The spec marks most numbers `[default]` (a guess to tune). In a few places its own numbers
cannot all hold at once. Each change is commented where it is made.

- **Recognizer aging (B.4.3).** The spec never ages a pattern while the stick is within its
  first point, and kills it after three misses. With both rules, a lazy 150 ms push scores
  0.9 strength, or no ollie at all, where C.3 step 3 wants "a hop under 0.3 m". Here,
  holding the first point *still* costs nothing (the long crouch stays free). Moving
  through it slowly ages the pattern, and a sample still closing on the next point is not
  a miss.
- **Rolling drag.** The linear `0.35 + 0.055 v` friction alone settles a 10° hill near
  24 m/s; C.3 says about 9. A small quadratic term `rolling_drag = 0.0035` does it.
- **Powerslide scrub.** `2.2 + 3.5 sin(slip)` takes a 90° slide from 8 m/s to under 3 in
  a second; the spec's check says about 4. Base 1.2 does it. On exit the board swings back
  onto the velocity while grip ramps, as the spec's "grip ramps back over 0.3 s"
  describes, without scrubbing the speed.
- **Pushes to top speed.** C.3 says four pushes to 8 m/s. The push numbers from the code
  fixture (2.0 falling to 1.0 m/s) cannot add up to that in four, so the test allows seven.
- **Ledge lock from the low side (B.10.3).** Reduced only when dropping onto a ledge from
  above. Applied to every ollie up onto a ledge, it would fail C.3 step 6's 0.4 m lock on
  normal.
- **Magnetism and vert.** An air moving almost straight up and down is not steered onto a
  rail, and a board falling past the coping, side-on to it, does not stall on the lip. It
  comes back into the wall (B.11 point 4).
- **Pump smoothing.** Rising speed is smoothed at 0.12 a tick, not 0.3, so a stand-up
  spreads over the wall instead of hitting the max-accel clamp for three ticks.
- **Fakie.** Rolling tail first is fakie however the board got turned round, not only
  after a landing or a slide.

`tests/optional.rs` covers the optional systems of D.1: a 6 cm kerb ridden over, a wall
ride (met at a shallow angle, gravity cancelled at first and less so over 0.8 s, then off the
wall), a handplant on a quarter-pipe's coping (RB going up the vert, 0.9 s over the apex, back
in fakie), and the Hall of Meat wipeout (the right stick picks the pose, the left stick spins
and drifts, a full trigger pushes off).

- **Wall ride entry.** The spec asks for the board already turned to the wall
  (dot(up, normal) > 0.71); here a wall met at a shallow angle with more speed along it than
  into it turns the board. It must be a wall (standing above the waist), not a ledge's side,
  and an air the prediction aimed at a rail goes to the rail.

- **The camera's numbers** (B.16: 2 to 2.6 m back, 12 deg up, 70 deg) are tunables
  (`cam_*`), and the realm's `skate/tuning.txt` sets the lens's view a little wider. Pulled
  in close by a wall behind, the camera rises over the rider rather than sitting inside
  them.

`tests/spots.rs` rides every spot flat out in 16 directions from its start and marks, and
fails if any ride ends "off the spot". It found Plattan's gap at the foot of the terrace,
and Övningsplanen and Rålis having no fence.

Not built: a remapping screen (the bindings are tables in `play/main.js` and
`apple/Sources/Input.swift`).
