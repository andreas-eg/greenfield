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
