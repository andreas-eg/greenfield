# Lens improvements

What Beargarden ran into in the lens: places where it is still Söder's client rather than a
window onto several kinds of game. Each entry says what happened, and what a more generic lens
might do instead. Found while building and playing the skate game (lens branch
`feat/beargarden-sk8`). Nothing here has been decided.

## How Beargarden uses the lens today

- sk8 is an optional cargo feature (`lens-app/sk8`) with a path dependency on `beargarden/sk8`.
  A board whose drive says `"sim":"sk8"` hands the frame to `skate.rs`, which runs the camera,
  board, body clips and input, in place of walking and the vehicle update.
- Everything else is Söder's: the start menu (CONTINUE, NEW GAME with naming and save slots,
  CHARACTERS and outfits), the menu backdrop as a director scene, walking and the walkmask, the
  HUD system, the realm connection and scripted play.
- Beargarden fills in that shape: `menu_scene`, `hud/*.json`, a catalogue. Some verbs (`outfit`)
  exist only because the menu sends them.

## Findings

### Game shape

1. **Söder defaults in the code.** `menu_scene` used to be hard-coded to `menu-soder` (it is now
   read from `realm.json`). `chart_root` still falls back to `../soder/world`. The realm should
   name everything the lens needs, with no game as the fallback.
2. **The start flow is a story game's.** NEW GAME asks for a name, CONTINUE asks for a save
   slot, and there is an outfit picker. A skate game wants: pick a skater, pick a spot, ride. The
   realm could describe its start flow (its screens and the verbs they send) rather than the lens
   building Söder's.
3. **A simulation hosted inside a vehicle.** sk8 gets in through Söder's vehicle model: board
   as a vehicle, `drive.sim`, `vehicle_name`, `exit`. It works, but it is indirect, and the
   session post at the start was once mistaken for the vehicle. A lens-level "game mode" or
   "controller" interface (input in, camera and poses out), with walking, vehicles and sk8 as
   implementations, would make the next game type a plug-in instead of a patch to `main.rs`.
4. **Gameplay compiled in.** sk8 is a cargo feature with a path dependency, so the lens branch
   cannot merge until it becomes a git dependency or is vendored. A generic lens needs a story
   for game code: features per game, a plug-in ABI (sk8 already has a C ABI and a wasm build),
   or the web/wasm route.
13. **A teleport while riding a simulation the lens owns.** A ride to a spot's start is off
    the board, transport, on a board in one world tick, so the lens never sees "off". sk8 rode
    on from where it was and pulled the world's position back, 44 m. Patched: a correction of
    over 3 m while on sk8 puts the rider down afresh where the world says. A generic lens wants
    an explicit "placed" message from the world for any client-owned simulation.

### Input

5. **Menu input depends on the backdrop (from reading the code, not yet seen).** The start
   menu takes arrows, Enter and Space only when `state.camera` is `None`; with a camera they go
   to walking. Beargarden's menu scene is a director scene; whether it has a camera here is not
   checked. (The menu did ignore keys on 2026-11-07, but so did everything: the OS was dropping
   the synthetic events, item 11.) Menu input should not depend on what the backdrop is.
6. **Esc quits from the menu scene** (the director scene's "Esc quits"), where a player would
   expect BACK.
7. **Input bindings are spread across files.** The keys are in `skate.rs`, the pad in `pad.rs`,
   touch in `touch.rs`, and walking and director keys in `main.rs`. There is no remapping
   screen. A per-game action map (actions to keys, buttons and touch controls), declared by the
   realm or the game module, would serve every game.

### Screen

8. **Söder's offers stay on screen while skating.** "Start a two-minute session", "Read the
   scoreboard" and STEP ON covered the ledge being ground. Patched on the branch: while a sk8
   board rolls (over 0.7 m/s) only STEP OFF shows, and the rest return when it stops. A game
   mode should be able to say which world offers show during play, and where.
9. **Stale HUD values.** LINE 172 stayed on screen and SCORE did not move. It was sk8, not the
   lens: a small trick with no line running was scored inside sk8 but never said as a line's
   end, so the realm never counted it. Fixed in sk8 (`scoring.rs`).
10. **The keys hint is a permanent HUD panel.** A generic "controls" overlay (shown at first,
    then on demand) would serve every game.

### Testing

11. **No way to play by hand without OS input permission.** Synthetic key events are dropped
    unless the terminal has Accessibility permission. `LENS_PLAY_LIVE=<file>` (added on the
    branch) makes scripted play take new steps from a file as they are appended, so an agent can
    play in a loop: look at a frame, decide, act. It is a test hook, but a general one.
12. **Scripted play lays out the phone's controls on a desk.** Useful for one script on both
    platforms, but the desk view while testing is not the desk view a player gets.
14. **The lens takes focus when it starts, and a script's keys mix with the desk's.** With
    the user at the same Mac, their typing went into a live-played game. Patched for
    `LENS_PLAY_LIVE`: desk keys and mouse are ignored (a script's fingers come in as touches),
    and the launcher hands focus back. A test mode should never read a person's input.
15. **Other riders on a client-run simulation were drawn as Söder's seated riders.** A
    skateboard has no seat, so another player on a sk8 board stood beside it and vanished when
    riding. Patched: a rider on `sth_skateboard` in a `skate_*` clip stands on the deck (and
    turned round when their shown `skate_stance` is goofy), and their board turns through the
    flips their realm shows (`skate_board`). A generic lens wants a way for a game to say how
    its riders and their mounts are posed for everyone, not only for the one riding.
16. **The intro credits a game.** "A GAME BY STHLM ROYALTY AND ELDER GROUNDS" plays before
    Beargarden sk8r's menu too: the credits should be the realm's.
17. **In the background, the window shows the intro.** With another app in front, the lens
    draws "Elder Grounds presents" over the game while the game runs on behind it. Seen while
    two lenses played side by side.
