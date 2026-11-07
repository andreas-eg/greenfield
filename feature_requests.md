# Greenfield: what it needs from the lens, mull, GeoGen and Studio

Greenfield is the Stockholm skate game, built on mull from the sk8r specification
(`sk8t/skateboard.md`). Everything in this repository was written without touching mull,
the lens, GeoGen or Studio. What follows is what those need to add for Greenfield to be the
game on iOS, macOS and the web. Each request says what exists here to build on, and how to
tell it is done.

What is here now (2026-11-07):

- `sk8/`: the whole simulation in Rust, no platform code. It covers the virtual controller,
  the Flickit recognizer, board physics, air with landing prediction, grinds, bails and
  respawn, scoring, difficulty and the camera, with a C ABI (`sk8/src/ffi.rs`). It builds
  for `wasm32-unknown-unknown` (344 KB), `aarch64-apple-ios` and the Mac. 39 tests pass,
  23 of them the spec's acceptance checks (C.3) ridden headless.
- `world/`: the realm. Four spots (Plattan, Rålis, Fryshuset, Övningsplanen), each with
  boards, a two-minute session and a scoreboard. The soul keeps score from what the lens
  reports and caps it. There is a HUD document and a tuning file. It boots on mull
  (`run-realm.exs`) and the session loop was played over telnet.
- `play/`: the client on the web, on the same wasm. It has the three input backends of C.2
  (gamepad, keyboard and mouse with pointer lock, touch with floating sticks) and a plain
  three.js view. It joins a world over mull's `/ws` bridge, rides to the spot, steps on a
  board, reports where the board is, and sends the `skate ...` lines. In headless Chrome it
  joined a local world, rode, and had its lines kept at Plattan, Rålis and Övningsplanen.
- `apple/`: the iOS and macOS apps, native: sk8 linked as a static library, SceneKit,
  GameController, touch sticks, the world over WebSocket. The iOS device build, the iOS
  simulator build and the Mac build all succeed. In the iPhone 17 Pro simulator and on the
  Mac (rendering offscreen, `-selftest`, the screen being locked) each joined the world,
  boarded at Plattan, Rålis or Fryshuset, rode, and was drawn.

These clients are Greenfield until the lens hosts sk8. They draw a crude rider and the
spots' plain geometry. The lens's renderer, characters and the surveyed Stockholm are what
the requests below bring. The apps are not signed for devices or TestFlight from here
(GF-REQ-STUDIO-2).

---

## Lens

**Status, 2026-11-07:** the lens branch `feat/greenfield-sk8` (worktree
`lens.worktrees/lens-sk8`, built with `--features lens-app/sk8` and
`LENS_REALM=<greenfield>/world`) does LENS-1, LENS-2, LENS-3 (as meshes, below), LENS-5,
and LENS-4 on the Mac (keys, mouse drag, gamepad, the glass's PUSH, FOOT BRAKE and OLLIE).
It was played on the Mac against this realm on mull: board, push, carve, ollie, kickflip,
grabs, a 50-50, banking, a session, a bail and respawn, travel to Rålis, and all six
skaters. What is still open:
- **LENS-4:** haptics, and the web lens (`crates/web` does not host sk8). On glass the
  right thumb is now a floating stick (60 pt) for the flick. It was tested with scripted
  swipes on the Mac, not yet on a phone.
- **LENS-6:** others see the rider's skate clip (the lens says `skate clip <name>`, the
  soul plays it with `play_clip`), but not the board's flips.
- **The dependency:** the branch depends on `greenfield/sk8` by path, so it builds only
  beside this checkout. Merging it to the lens's main needs a git dependency or sk8
  vendored into the lens.

How it is played, for a person at the keys:
- W or Space pushes, S or C brakes, A and D lean.
- The arrow keys are the right stick. Down then up is an ollie; down, then round to the
  left and up is a kickflip.
- Z and G are the triggers: crouch, and grab in the air. H holds a manual (Y a nose
  manual), and S with A or D powerslides.
- T brings the board back after a run-out, as walking to it and pushing does.
- Dragging the mouse with the button held also flicks.
- F steps off.

LENS-3 is done in the realm, not the lens. `sk8/src/bin/spots.rs` writes each spot as two
meshes: `spot_<name>_floor` (Plattan's triangle paving, concrete slabs or asphalt, tiled
by the lens's `uv_from_position`) and `spot_<name>` (the rest, coloured by surface). The
charts name both.

### GF-REQ-LENS-1: ride a board whose drive says `"sim": "sk8"` with the sk8 simulation

The board (`world/vehicles/skateboard.c`) declares
`set_drive((["kind": "kick", ..., "sim": "sk8", "sim_tuning": "skate/tuning.txt", "sim_stance_attr": "skate_stance", "sim_difficulty_attr": "skate_difficulty", "sim_events": "skate"]))`.
Today's lens reads `kind: kick` and rides it as Söder's board, so Greenfield is rideable
now, just not with the sk8 feel.

A lens that does this request:

1. Depends on `greenfield/sk8` (a path or git dependency; it needs only serde and
   serde_json). It uses `sk8::Sim` directly, not the C ABI.
2. On boarding, builds `sk8::World` from the chart's `"skate"` file
   (`charts/<spot>.json` has `"skate": "<spot>.skate.json"`, beside it in the realm). It
   places the board where the rider stands (`Sim::place`) and sets the stance and
   difficulty from the player's `skate_stance` / `skate_difficulty` attributes. It applies
   `sim_tuning` (`Sim::set_difficulty(d, text)`), whose format is in
   `sk8/src/tuning.rs`.
3. Every frame, fills `VirtualController`s from the input (GF-REQ-LENS-4) and calls
   `Sim::advance(dt, samples)`. It draws from `sk8::Frame`: the board's transform with
   `flip_angle` / `shuv_angle` / `pitch`, the rider at `rider_pos` / `rider_up` playing
   `frame::clip_name(clip)`, and the camera from `cam_pos` / `cam_target` / `cam_fov`. It
   interpolates between `prev_frame` and the latest by `accumulator / DT` (B.1). Nothing
   in sk8 runs at the display rate.
4. Reports the rider's position to the world as it does for any ride (`walk` / `move`), so
   other players and the world see them where sk8 has them.

Coordinates: sk8 is right-handed, Y up, metres. A spot's x and z are the chart's x and z.
The heading is radians from +z toward +x. `place_object(..., degrees)` in the spot rooms
uses the same x and z; the lens's yaw convention (0 = east) needs a conversion at the
boundary.

Done when: on the Mac, a gamepad ollie at Övningsplanen pops 0.85 to 1.05 m from a full
snap and under 0.3 m from a 150 ms push, the same numbers `sk8/tests/acceptance.rs` checks.

### GF-REQ-LENS-2: send what happened as `skate ...` lines

The world keeps score only from what it is told (`world/support/skate_rules.c`). From
`Sim::take_events()`, send:

| sk8 event | line |
|---|---|
| `Banked { names, points, multiplier }` | `skate banked <points as int> <multiplier> <name>+<name>+...` |
| `LineEnded { points }` | `skate line <points as int>` |
| `Bail { reason }` | `skate bail <reason>` |
| `Land { quality }` | `skate land <clean\|adjust\|sketchy\|spin>` |
| `GrindStart { name }` | `skate grind <name>` |

The multiplier goes as text (`1.5`). The world takes it apart with `%d %s %s`, because
mull's `sscanf` has no `%f` (GF-REQ-MULL-1).

Done when: a landed kickflip at Plattan shows on the HUD's line plate and in
`skate_score`.

### GF-REQ-LENS-3: draw a spot from its skate file until it has meshes

The spot charts have an empty `instances` list, because there are no meshes for them yet
(GF-REQ-GEOGEN-1). Until there are, the lens should draw the triangles of the chart's
`"skate"` file with a material per surface category (smooth, rough, slow, slippery,
veryslow, stairs, unrideable), and the rails as thin cylinders. `play/main.js` `buildSpot`
does exactly this in about forty lines.

### GF-REQ-LENS-4: Flickit input on every platform (spec C.2)

The recognizer counts samples, so input fidelity is the feature (A.8). sk8 takes every
sample since the last frame (`advance(dt, samples)`) and scales its windows with
`set_sample_rate(hz)`.

- **Gamepad**, iOS and macOS: `GCExtendedGamepad` polled from `CADisplayLink` at up to
  120 Hz, every sample passed. The standard mapping is in `play/main.js` `readPad`.
- **Touch**, iOS: two floating sticks (left and right halves, 60 pt radius), fed
  `UIEvent.coalescedTouches(for:)`. A tap on the left pushes. A second finger on the left
  brakes. Corner buttons hold LT and RT.
- **Mouse as the right stick**, macOS: pointer locked
  (`CGAssociateMouseAndMouseCursorPosition(false)` or `GCMouse`). Deltas integrate into a
  stick (120 px is full deflection) that springs back after 30 ms at rest. WASD is the
  left stick on a 0.12 s / 0.08 s ramp.
- **Web**: the lens's web build does as `play/main.js` does, with pointer lock,
  `getCoalescedEvents`, the Gamepad API and `touch-action: none`.
- **Haptics**: a transient on pop and landing, light and continuous while grinding.

Done when: C.3 step 10 holds, with identical ollie heights for identical flick timing on
pad, touch and mouse.

### GF-REQ-LENS-5: the rider's clips (spec B.17) - done on the lens branch

The skaters carry 13 skate clips, authored in ActionForge and retargeted onto each body
(`tools/README.md`). The lens's `skate.rs` (`body_clip`) maps sk8's clip names onto them.
The original request:

`sk8::frame::CLIPS` names the minimum set: ride, push, push_mongo, brake, manual,
nose_manual, powerslide, kick_turn, ollie, air, grab, land, land_sketchy, the grind
families, revert, bail, run_out, walk and crouch. The band bodies have none of these. Until
CharGen makes them, alias them to what the bodies have, as Söder's catalogue aliases
`knocked_down` (SODER-BUG-606). Crouch and lean are numbers in the frame (`crouch`,
`lean`) for a blend or a bone offset.

### GF-REQ-LENS-6: other skaters' boards

Another player's board should be drawn with its flips and grinds, not just moved. The
smallest version sends `flip_angle`, `shuv_angle` and `pitch` with the ride's position
updates, and the lens draws other boards with them.

---

## mull

### GF-REQ-MULL-1: `%f` in `sscanf`

`sscanf` takes `%s` and `%d` only. Greenfield works round it with `%s` and `to_float`
(`skate_rules.c skate_bank`). A `%f` would make the protocol plain.

### GF-REQ-MULL-2: storage that outlives a reboot, shared by a realm

A spot's scoreboard (`world/things/scoreboard.c`) keeps every skater's best session in a
mapping that is lost when the world restarts. A player's own bests survive, because they
are attributes. Needed: an efun pair to save and restore a mapping under a realm key, for
example `save_value(key, value)` and `restore_value(key)`, kept beside the player store.

### GF-REQ-MULL-3: proving a score, not capping it

Today the world trusts the lens up to a cap: a named trick's best value plus 1200, times
1.5, times a clean landing, times the line multiplier (at most x3), and at most 40 000
points in any 20 s. sk8 is deterministic for a given input stream and tuning, so the
world could replay a sequence and know. Needed: a way for the realm to run sk8 (a port
or a NIF behind an efun such as `verify_ride(spot, tuning, inputs)`) on the input log
the lens sends with a banked sequence. This is the physics.md rule ("the client
predicts, the server decides") applied to scores.

### GF-REQ-MULL-4: a structured event from the lens

`skate ...` lines go through the soul's `command()`, which works. A typed message
(`event <name> <json>`, reaching a hook such as `lens_event(name, mapping)`) would save
parsing in LPC, and the lens could send several events in one frame.

### GF-REQ-MULL-5: exits with names

`add_exit("ralis", "ralis")` is accepted, but `go ralis` is refused: `move` looks the word
up in a fixed table of compass directions. Greenfield's spots are linked by the soul
instead (a spot's name `transport`s you there, board and all). Needed: an exit whose
direction is any word, so a realm can say `add_exit("tunnelbana", "ralis")`.

---

## GeoGen

### GF-REQ-GEOGEN-1: the real spots

`sk8/src/spots.rs` lays the spots out from what is known of them (the paving, the stairs
up to the upper level and the benches at Sergels torg; the bowl, mini ramp, pyramid and
hubba at Rålambshovsparken; the vert ramp at Fryshuset). It is schematic, not surveyed.
Needed: each spot built from survey data and photographs as a chart with meshes. Each
must also have a `.skate.json` (the format is `sk8::world::SpotFile`: triangles by
surface category, rail polylines with kind and material, named marks). The rails can be
extracted from edges as B.3.2 describes, then pruned by hand. Bowls need closed coping
loops that follow the rim exactly.

Done when: `cargo test` still passes the spots test with GeoGen's files in place of the
generated ones, and Plattan's benches are where the photographs have them.

### GF-REQ-GEOGEN-2: meshes for Övningsplanen

The practice plaza is a test level by design (D.2 Phase 0), so its geometry can stay
generated. It needs drawable meshes (concrete, steel coping, painted kerbs) from its
skate file, unless the lens draws skate files (GF-REQ-LENS-3).

---

## Studio and releases

### GF-REQ-STUDIO-1: track Greenfield

The README says Greenfield is the GREENFIELD project in Studio. Nothing was filed from
here. Stories should be filed for the requests above, with one for the work done here,
whose evidence is `cargo test` in `sk8/` and the telnet session in this file's history.

### GF-REQ-STUDIO-2: ship it

- **iOS and macOS**: the lens app shipping Greenfield as a world beside Söder (TestFlight
  from the mini, as Söder's builds are), once GF-REQ-LENS-1 to 4 land.
- **Web**: a hosted world on play.eldergrounds.com. Meanwhile `play/` can be published
  as a static page (`play/build.sh`, then the folder as it is). It needs HTTPS for the
  gamepad on other devices.
