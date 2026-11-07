# greenfield

Greenfield is the Stockholm skate game, on mull (the small-forest driver), for iOS, macOS
and the web. Its movement follows the sk8r specification (`~/src/eg/sk8t/skateboard.md`):
the right stick is the board (Flickit), the left stick is the body, and there are no
balance meters. Grinds are pulled on by prediction, not by buttons.

It began as a realm of its own beside Söder for what does not belong in Söder (the walking
castle, SODER-WORLD-S72). The field and the castle are still here.

## Layout

- `sk8/`: the simulation, in Rust with no platform code. It covers input and the Flickit
  recognizer (B.4), board physics, air and landing (B.5 to B.9), grinds (B.10), wipeout
  and respawn (B.13), scoring (B.14), difficulty (B.15) and the camera (B.16). It has a C
  ABI for Swift and the browser. `cargo test` rides the spec's acceptance checks (C.3)
  headless. See `sk8/README.md`.
- `world/`: the realm.
  - `spots/`: the skate spots. `plattan.c` (Sergels torg, the start room), `ralis.c`
    (Rålambshovsparken), `fryshuset.c` (the hall at Hammarby) and `ovningsplanen.c` (the
    practice plaza, on the field). Each has three boards, a session start and a scoreboard.
  - `charts/<spot>.json` and `<spot>.skate.json`: the chart the room names, and the
    surfaces and rails sk8 rides. Both are written by `cargo run --bin spots -- ../world/charts`
    from `sk8/src/spots.rs`.
  - `vehicles/skateboard.c`: the board. Its drive asks a lens for sk8
    (`"sim": "sk8"`) and falls back to Söder's kick board on today's lens.
  - `players/player.c`: the soul. It holds stance and difficulty, hears `skate ...` lines
    from the lens and runs sessions.
  - `support/skate_rules.c`: what the world keeps. It has the trick values, caps a
    sequence at what its tricks could be worth, rate-limits, and runs two-minute sessions
    with bests.
  - `things/session.c`, `things/scoreboard.c`, `hud/skate.json`, `skate/tuning.txt`.
  - `field.c`, `castle/`, `vehicles/walking_castle.c`, `things/castle_gate.c`: the green
    field and the walking castle, as before.
- `play/`: the game's client on the web. It runs the same simulation (wasm), the three input
  backends of spec C.2 (gamepad, keyboard and mouse, touch), and a plain three.js view. Given
  a world (`?world=ws://host:4062/ws`, or the menu's field) it plays on mull: it logs in, rides
  to the spot, steps on a board, reports the board's position, and sends the `skate ...` lines
  the world scores. Without a world, nothing is kept.
- `apple/`: the iOS and macOS apps, native. sk8 is linked in as a static library
  (`build-sk8.sh` builds it for the platform being built), stepped from SceneKit's
  display-linked loop, with GameController pads, keyboards and mice, floating touch sticks
  on iOS, a SwiftUI menu and HUD, and the world over its WebSocket bridge (spec C.2.5).
  `xcodegen generate`, then the `Greenfield-iOS` or `Greenfield-macOS` scheme. Launch
  arguments `-world ws://host:4062/ws -spot ralis` join a world at once;
  `-selftest 12 -snapshot shot.png -demo YES` rides, renders offscreen and prints a summary.
- `tools/`, `art/actionforge/`: the skaters. Their 13 skate clips (ride, push, ollie, air,
  grab, grind, manual, powerslide, land, bail and more) were authored in ActionForge and
  are retargeted onto the band's bodies (Hugo, Erik, Zikai) and three CharGen ones (Sanna,
  Mo, Lova) into `world/scenes/skaters/` (not in git). The catalogue names them
  `skater_*`. See `tools/README.md`.
- `conditioned/greenfield/`: the spots' meshes for the lens (the floor's paving tiled,
  the rest coloured by surface), written with the charts.
- `feature_requests.md`: what the lens, mull, GeoGen and Studio need to add (sk8 in the
  lens, input backends, clips, storage, score proofs, surveyed spots).

## Run it

The realm on a driver checkout (nothing in the driver changes):

    cd ~/src/eg/repos/small-forest/repos/engine
    PORT=4061 WEB_PORT=4062 elixir --no-halt -S mix run --no-start ~/src/eg/greenfield/run-realm.exs

Then `nc localhost 4061`, or connect a lens. A telnet player moves with `jump <x> <z>`.
Commands: `board` (onto a free board here), `use a skateboard`, `session`, `stance goofy`,
`difficulty hardcore`, `score`, and a spot's name to ride there (`ralis`). mull's exits are
compass directions only, so the spots are linked by the soul, not by exits.

The simulation in a browser:

    cd play && ./build.sh && python3 -m http.server 8088    # then http://localhost:8088

Add `?world=ws://127.0.0.1:4062/ws` to play on the world above (`&spot=ralis` picks the
spot, `&demo=1` rides a scripted line). Click to lock the mouse, which is the right stick:
pull back, then push forward fast to ollie. W/A/S/D steer and push; Space pushes. On a
phone, the left half is the body and the right half is the board. A gamepad works too.

In the lens (its branch `feat/greenfield-sk8` hosts sk8 for this realm's boards):

    tools/make_skaters.sh                                    # once: the skaters
    cd ~/src/eg/lens.worktrees/lens-sk8
    LENS_REALM=$HOME/src/eg/greenfield/world cargo build --release -p lens-app --features lens-app/sk8
    # run it from a folder whose conditioned/ holds the lens's conditioned/* and
    # conditioned/greenfield -> greenfield/conditioned/greenfield
    LENS_LOOK=folder LENS_ADDR=127.0.0.1:4061 LENS_WEB=127.0.0.1:4062 <target>/release/lens-app

The start menu is Greenfield's (`hud/menu.json`, `scenes/menu-greenfield.json`).
SKATERS picks a skater. On a board:
- W or Space pushes, S brakes, A and D lean.
- The arrows are the right stick: down then up is an ollie.
- Z and G grab, and F steps off.
- A pad works as in the spec.
- `LENS_SK8_TIMESCALE=0.25` slows the board down to look at a trick.
- The world also takes `skater <name>`, `stance goofy` and a spot's name.

The apps:

    cd apple && xcodegen generate && open Greenfield.xcodeproj

The simulation's tests:

    cd sk8 && cargo test

Written in LPC for mull; the efun reference is `/efuns` on a running world
(`priv/efuns.md` in the engine). Tracked in Studio as the GREENFIELD project.
