// bjorns.c - Björns trädgård, a Beargarden skate spot: the bowls in the park on Södermalm.
//
// Its surfaces are charts/bjorns.skate.json, written by beargarden/sk8 (cargo run --bin
// spots) from STHLM Royalty's park model (sk8/src/spots.rs): the chart charts/bjorns.json names
// it for the lens's sk8 (GF-REQ-LENS-1). The other spots are a ride away: say a spot's name.
void create() {
    set_title("Björns trädgård");
    set_description("Three concrete bowls sunk in one deck in the park on Södermalm, Medborgarhuset's brick over the trees: a big lobed bowl and two ovals, steel coping all round.");
    set_chart("bjorns");
    set_area(-30.0, -30.0, 30.0, 30.0);
    add_item("big bowl", "The big one, lobed like a clover and about a metre and a third deep: vertical at the lip, flat in the middle.");
    add_item("ovals", "Two smaller bowls to the east, a metre and a bit deep, a step apart.");
}

void reset() {
    place_object("vehicles/skateboard.c", "board_1", "world", -3.0, 6.5, 180.0);
    place_object("vehicles/skateboard.c", "board_2", "world", 0.0, 6.5, 180.0);
    place_object("vehicles/skateboard.c", "board_3", "world", 14.8, 1.5, 270.0);
    // The scooter beside the start (SODER-LENS-S291).
    place_object("vehicles/scooter.c", "scooter_1", "world", -0.1, 5.4, 270.0);
    place_object("things/session.c", "session", "world", -5.0, 7.0, 0.0);
    place_object("things/scoreboard.c", "scoreboard", "world", -6.5, 6.0, 0.0);
}
