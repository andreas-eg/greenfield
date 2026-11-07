// ovningsplanen.c - Övningsplanen, a Beargarden skate spot.
//
// Its surfaces are charts/ovningsplanen.skate.json, written by beargarden/sk8 (cargo run --bin
// spots) from the layout in sk8/src/spots.rs: the chart charts/ovningsplanen.json names it for the
// lens's sk8 (GF-REQ-LENS-1). The other spots are a ride away: say a spot's name (players/player.c).
void create() {
    set_title("Övningsplanen");
    set_description("A practice plaza laid on the green field: a bank, a waist-high ledge, a round rail, a quarter-pipe with steel coping, a two-and-a-half-metre bowl, a manual pad, a kerb, five steps with a handrail, and a long hill for rolling. Everything here is for learning a trick before taking it to town.");
    set_chart("ovningsplanen");
    set_area(-40.0, -40.0, 40.0, 40.0);
    add_item("bowl", "Sunk into the grass in the west: smooth concrete, steel coping all round, deep enough to air out of.");
    add_item("hill", "Sixty metres at ten degrees. Let it run and see where the speed settles.");
}

void reset() {
    place_object("vehicles/skateboard.c", "board_1", "world", 1.0, 0.5, 0.0);
    place_object("vehicles/skateboard.c", "board_2", "world", -1.5, 1.0, 30.0);
    place_object("vehicles/skateboard.c", "board_3", "world", 2.5, -1.0, 300.0);
    place_object("things/session.c", "session", "world", 2.5, 2.0, 0.0);
    place_object("things/scoreboard.c", "scoreboard", "world", 1.5, 2.0, 0.0);
}
