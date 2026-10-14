// castle_gate.c — the way out of the walking castle, over the drawbridge.
//
// It lets you out wherever the castle has walked to: it asks the castle which
// room it stands in and where, as Padrone's gate did in 1990 -
// move_player("south", call_other(CASTLE, "query_environment")) - and puts
// you on the street in front of it.
// @req SF-WORLD-S23  A gate hands a body between a place and the inside of a thing
inherit "things/thing";

mixed castle;

void setup() {
    set_name("The gate");
    set_long("The castle gate, and beyond it the drawbridge down to the street.");
    amount = 0;
    set_mount("door");
    add_verb("enter", "Go out over the drawbridge", "door", 2.5);
}

void belongs_to(mixed c) { castle = c; }

void stands(float x, float z, float yaw) {
    place(x, z);
    face(yaw);
}

string do_enter() {
    mixed room;
    float yaw;
    room = query_room(castle);
    if (room == 0) return "The drawbridge is up. The castle is not standing anywhere just now.";
    yaw = facing(castle);
    // Nine metres ahead of the castle along the way it faces: past the
    // drawbridge's end, clear of the legs.
    // Yaw 0 is east and 90 south (x east, z south): ahead is (cos, sin).
    transport(this_player(), room, query_x(castle) + 9.0 * cos(yaw), query_z(castle) + 9.0 * sin(yaw));
    return "You cross the drawbridge and step down onto the street.";
}
