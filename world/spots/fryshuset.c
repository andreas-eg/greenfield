// fryshuset.c - Fryshuset, a Greenfield skate spot.
//
// Its surfaces are charts/fryshuset.skate.json, written by greenfield/sk8 (cargo run --bin
// spots) from the layout in sk8/src/spots.rs: the chart charts/fryshuset.json names it for the
// lens's sk8 (GF-REQ-LENS-1). The other spots are a ride away: say a spot's name (players/player.c).
void create() {
    set_title("Fryshuset");
    set_description("The hall at Fryshuset in Hammarby: a vert ramp ten metres wide under the lights, a mini ramp beside it, a box and a rail at the far end, plywood everywhere and the music too loud.");
    set_chart("fryshuset");
    set_area(-21.0, -25.0, 21.0, 25.0);
    add_item("vert", "Three-metre transitions and almost a metre of vert, steel coping, a deck to stand on and drop in from.");
    add_item("mini ramp", "A mini ramp for warming up, no vert, wide enough for two.");
}

void reset() {
    place_object("vehicles/skateboard.c", "board_1", "world", 1.0, 12.0, 180.0);
    place_object("vehicles/skateboard.c", "board_2", "world", -1.0, 13.0, 160.0);
    place_object("vehicles/skateboard.c", "board_3", "world", 2.5, 11.0, 200.0);
    place_object("things/session.c", "session", "world", 1.0, 0.0, 0.0);
    place_object("things/scoreboard.c", "scoreboard", "world", 1.5, 13.0, 0.0);
}
