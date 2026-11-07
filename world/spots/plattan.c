// plattan.c - Plattan, Sergels torg, a Beargarden skate spot.
//
// Its surfaces are charts/plattan.skate.json, written by beargarden/sk8 (cargo run --bin
// spots) from the layout in sk8/src/spots.rs: the chart charts/plattan.json names it for the
// lens's sk8 (GF-REQ-LENS-1). The other spots are a ride away: say a spot's name (players/player.c).
void create() {
    set_title("Plattan, Sergels torg");
    set_description("The sunken plaza under Sergels torg, black and white triangles to the edges, Kulturhuset's glass along one side and the upper level's wide stairs on the other. Skaters have owned it since the eighties; the granite benches are waxed smooth.");
    set_chart("plattan");
    set_area(-32.0, -30.0, 32.0, 20.0);
    add_item("stairs", "Twelve steps up to the upper level, three flights side by side, a handrail down each.");
    add_item("benches", "Low granite benches along the plaza's edge, their corners rounded by thirty years of grinding.");
}

void reset() {
    place_object("vehicles/skateboard.c", "board_1", "world", 1.0, 8.5, 180.0);
    place_object("vehicles/skateboard.c", "board_2", "world", -3.0, 10.0, 200.0);
    place_object("vehicles/skateboard.c", "board_3", "world", 6.0, 6.0, 90.0);
    place_object("things/session.c", "session", "world", 3.0, 9.5, 0.0);
    place_object("things/scoreboard.c", "scoreboard", "world", 1.5, 8.0, 0.0);
}
