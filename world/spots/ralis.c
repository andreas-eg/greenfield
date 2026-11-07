// ralis.c - Rålis, Rålambshovsparken, a Beargarden skate spot.
//
// Its surfaces are charts/ralis.skate.json, written by beargarden/sk8 (cargo run --bin
// spots) from the layout in sk8/src/spots.rs: the chart charts/ralis.json names it for the
// lens's sk8 (GF-REQ-LENS-1). The other spots are a ride away: say a spot's name (players/player.c).
void create() {
    set_title("Rålis, Rålambshovsparken");
    set_description("The concrete park in Rålambshovsparken, by the water at the end of Norr Mälarstrand: a deep bowl, a mini ramp, a pyramid with a rail down its face, a hubba and a run of ledges, with the park's grass all round.");
    set_chart("ralis");
    set_area(-40.0, -30.0, 40.0, 30.0);
    add_item("bowl", "Two and a half metres of floor, a deep wall all round and pool coping on top.");
    add_item("pyramid", "Four banks round a flat top, a round rail running down the west face.");
}

void reset() {
    place_object("vehicles/skateboard.c", "board_1", "world", 1.0, 1.0, 0.0);
    place_object("vehicles/skateboard.c", "board_2", "world", -2.0, 2.5, 45.0);
    place_object("vehicles/skateboard.c", "board_3", "world", 3.0, -1.0, 270.0);
    place_object("things/session.c", "session", "world", 2.5, 2.0, 0.0);
    place_object("things/scoreboard.c", "scoreboard", "world", 1.5, 2.0, 0.0);
}
