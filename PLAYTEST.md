# Play tests

What playing Beargarden in the lens showed, and what was done about it. Lens-side findings
(the lens as a client for several games) are in `LENS_IMPROVEMENTS.md`.

## 2026-11-07, lens on the Mac, played live

The lens (`feat/beargarden-sk8`) run against the local realm, ridden by an agent through
`LENS_PLAY_LIVE`: one step at a time, each chosen from the last screenshot and the rider's
trail. Erik's body, regular, normal difficulty.

| Tried | What happened | Done |
|---|---|---|
| Push, ollie on the flat | 0.9 m pop, clean landing, "Ollie + Air" 150 | - |
| The score after a small trick | LINE stayed up and SCORE never moved: a trick under 500 with no line running was scored inside sk8 but never said, so the realm lost every small trick | Fixed in sk8 `scoring.rs`, with a test |
| Riding back to the spot's start | The lens kept riding from where it was and dragged the world back 44 m | Fixed in the lens: a jump of over 3 m from the world puts the rider down afresh |
| Coasting | 3 m/s rolled to a stop in about 6 s; a line was mostly pushing | `world/skate/tuning.txt`: rolling friction about halved (0.18 + 0.03 v) |
| Finding something to skate from the Plattan start | It faced open plaza, with every ledge behind or to the side, and keys steer about 10 deg in 0.3 s | Start moved onto the east bench's line, 0.4 m off it, facing it (`sk8/src/spots.rs`, `support/skate_rules.c`) |
| FS 50-50 on the east bench | Locked, 3.4 m, leaned off at the end, then re-locked the same ledge every half second, stuck on top | Fixed in sk8 `grind.rs`: not onto the rail just fallen off until a pop or the ground away from it; test added. Now: 50-50, lean off, clean landing, 384 |
| A slow 50-50 (3.5 m/s in) | Drag stopped it 3.4 m along and the spec's 4 s ledge stall held the rider frozen on the bench | `grind_ledge_stall` tunable (spec 4 s, so C.3 step 6 holds), Beargarden 1 s: off, clean, 386 |
| The offers while riding | The scoreboard, session and STEP ON covered the ledge being ground | The lens shows only STEP OFF while the board rolls |
| Kickflip, heelflip (arrows down, then down-left or down-right to up) | Both landed clean, about 370 each | - |
| Grab (Z in the air) | Mute, clean, 308 | - |
| Manual (H), powerslide (S+A) | Manual 1.5 s for 383, powerslide 76 | - |
| A two-minute session at Rålis and Plattan | Countdown, then the clock was a small grey number by the buttons | The clock is a plate in the top right, under SCORE and SESSION (`hud/skate.json`) |
| Rålis | The start faces a flat bar; rides over it | - |
| Fryshuset vert | Up the wall and back fakie, the rider leaning into it, the camera following | - |

Still open from this session:
- The flat ollie pops 0.9 m on a full flick, high for flat ground, and it scores as "Ollie + Air".
- Keyboard steering is all or nothing: a tap barely turns, and a 0.9 s hold turns 90 deg.
- The keys hint is a permanent panel.
- START A TWO-MINUTE SESSION is still offered during a session (hidden while rolling).
- Not yet played: Övningsplanen, bails and respawn, other skaters, goofy stance, and the end of a session.

## 2026-11-07, later: styles and others' boards

- **Arcade and realistic.** A player setting (MENU: Arcade / realistic; `style`), arcade by
  default, each a section of `world/skate/tuning.txt`. Measured in the lens: a full flick on
  the flat pops 1.04 m in arcade and 0.56 m in realistic. `sk8/tests/styles.rs` holds them to
  it: the pop, coasting, grind lock from 0.8 m off (arcade) against on the line only
  (realistic), and a spin let go early, finished in arcade, bailed in realistic.
- **Other players see the board.** A second lens watched a kickflip: the rider stood on the
  deck in the skate pose and the board turned over under them. Before, other players saw the
  rider stood beside nothing.
- **Björns trädgård.** The three bowls from STHLM Royalty's park model (`BT_Skate_Basins_01`
  v005: rims from the 2023 orthophoto, ±1 m; depths from photos, ±0.4 m) in one deck in the
  grass. Ridden in the lens: rolling off the start at 2.8 m/s the board caught the coping for
  a tail stall and dropped in, landed clean on the floor and rode across. `tests/spots.rs`
  drops in at 4 m/s, down to the floor and up the far wall without a bail.
- **A style changed mid-ride** first rebuilt the ride where the camera was, on a bowl's wall,
  and stood the rider sideways on it. Now it retunes the same ride where it is.

## 2026-11-07, evening: the iPhone simulator

The lens built for iOS (iPhone 17 simulator, iOS 26.5) with Beargarden's realm and its own
driver in the app, played by script through the phone's controls.

| Tried | What happened | Done |
|---|---|---|
| Start | The driver put every new player in Söder's gotgatan, which Beargarden has not: never connected | `driver.m` starts in the realm's `start_room` |
| A new player | Arrived in the room's middle, facing the buildings at Plattan, and rode into the corner | Every visit starts at the spot's start (`player.c` login) |
| The HUD | The keyboard hint showed on the phone | Keys on a desk only (`lens_glass`); on glass a thumb hint |
| PUSH, FOOT BRAKE, OLLIE | Work | - |
| Right-thumb flick (0.15 s) | An ollie at half strength, 0.66 m: a thumb is slower than a stick | Arcade's flick window widened (3.0 to 6.5): full strength, 1.04 m; test added |
| Grab | No triggers on a phone: no grabs at all | OLLIE pressed again in the air grabs: a Mute, clean |

Still open on the phone:
- Lining up a bench is fiddly: the board ran 0.7 m beside the bench's line. A real thumb has to say whether that is the steering or the script.
- The Dynamic Island covers the left edge of the menu rows (CHOOSE HUGO) in landscape.
- The app is still called Söder, with Söder's icon and bundle id: a Beargarden target before TestFlight.
- Its own meshes (skaters, spots) are packed from local folders; for TestFlight they go into Studio and the realm's lock.

## 2026-11-07, evening: Andreas on the simulator

- **START A TWO-MINUTE SESSION did nothing.** On foot, the world answered "Step on a board
  first." and the phone shows no world lines. The session now puts the skater on a board
  first. The two minutes run their full time (heard at 8.8 s and 130.3 s).
- **Sunk into the stairs, and through the world after a fall.** The spots' walkmasks were a
  flat floor at 0, so a body off the board stood at 0 inside the stairs or the upper level.
  The spot builder now writes the surfaces as walkmask decks, leaves out any surface with
  another over it, and puts the highest first. Measured: the stairs 1.8 / 0.8 / 0.2 m, the
  upper level 2.4, the benches 0.42, the kerb 0.18.
- **Instrumentation.** The lens writes the ride to `sk8.jsonl` in the session's Diagnostics
  folder: every sk8 event, every change of state, on and off the board, and a sample twice a
  second with the ground under the rider. Under the spot's lowest floor it writes
  `fell_through`, also to events.jsonl (and Firebase on a device), and puts the rider back at
  the start.

Still open: world lines (the reply to a tap) do not show on the phone.

## 2026-11-08, the city spike: Björns trädgård to Medborgarplatsen from GeoGen's island

A box of the real island (x 165..315, z 160..310, island 67) as a spot, `medborgarplatsen`, made by
`tools/city_spot.py` and ridden with sk8 on the desk (SODER-LENS-S272). Regenerate with:

    python3 tools/city_spot.py ~/.soder/islands/soder-67 ~/src/eg/soder/world sk8/spots/medborgarplatsen.json
    cd sk8 && cargo run --release --bin spots -- ../world/charts ../conditioned/beargarden

| Tried | What happened | Done |
|---|---|---|
| The island's ground, clipped to the box | 75 k triangles; the lens drew a slab of grass in the air and nothing else | The lens merges a mesh's primitives under one u16 index base (SODER-BUG-628): the spot builder writes a big spot as several meshes |
| Riding north from the Medborgarplatsen side | Fell through the ground in front of the terrace wall | Walls and structures GeoGen draws as their own meshes leave no ground under them: the edges' walls go in as walls, and every 1 m cell without ground as a solid block (3 817 cells, most of them under houses) |
| The start on GeoGen's own skate spot | A push did not get going: the spot is on the lawn | The start is the first smooth ground ahead of it, on the deck |
| The surfaces | Nearly everything outside the park read as lawn | GeoGen's `yard` is made ground (courtyards, an untagged square such as Medborgarplatsen), now rough, not slow |
| The bowls | Rideable: dropped in 2.3 m and rode 15.6 m across, no bail (headless); live, in and out, and the coping locks into BS and FS bluntslides | GeoGen's bowls are deeper than the hand-built spot's: 2.15, 1.46 and 1.45 m against 1.35, 1.10 and 1.25 (guessed lips and floors) |
| A ride from the bowls to the square | About three minutes live across the bowls, the coping, the lawns and the houses with no fall through the world (sk8.jsonl, three runs) and no ride off the box (16 headings from the start and from the square, tests/spots.rs); but no line on the board from the park to the square | A band of lawn 16 to 20 m wide and the terrace's stairs lie between them, and GeoGen has the park's paths only as lawn: the last stretch is a walk |

What it says for the whole island (streamed tiles): the ground holds a board and the bowls ride,
but it is drawn for looking. Before tiles: GeoGen paths through parks (surveyed, not lawn), the
structures' footprints as ground or solids in the ground mesh itself, kerbs as edges with a height
(GF-REQ-GEOGEN-1), and bowl depths from a survey rather than a guess.
