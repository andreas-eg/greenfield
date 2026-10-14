// walking_castle.c — a castle on four legs, walking round the green field.
//
// After Padrone's Walking Castle (castles_and_quests/padrone/walker, Thomas
// Padron-McCarthy, 1990-91): a place that moves. Outside, it is a body on the
// field like a car - big, slow, walking the gravel ring corner to corner.
// Inside, it is a room of its own (castle/walking_castle.c) with a chart of
// its own. Its gate is the only way between the two: "Enter the castle" here,
// and the drawbridge in the yard, which lets you out wherever the castle
// stands now. It walked Götgatan in Söder until it moved here (SODER-WORLD-S72).
// @req SODER-WORLD-S22  A walking castle
// @req SF-WORLD-S23  A gate hands a body between a place and the inside of a thing
// @req SODER-WORLD-S72  The walking castle in a realm of its own

// The ring's corners, walked in order (geogen/model/greenfield.py: RING 30).
float *xs;
float *zs;
int leg;

void create() {
    set_name("The walking castle");
    set_long("A small grey castle on four oak legs, a tower at each corner and slate on the towers, stepping round the field at a stroll. The drawbridge hangs down at the front, level with the grass.");
    set_look("none");
    set_mount("walking_castle");
    set_size(11.0, 7.0);
    set_speed(1.0);
    add_verb("enter", "Enter the castle", "door", 8.0);
    xs = ({ -30.0, 30.0, 30.0, -30.0 });
    zs = ({ -30.0, -30.0, 30.0, 30.0 });
    set_heart_beat(1);
}

// Out onto the field, at the ring's first corner.
void walk_out() {
    leg = 1;
    place(0.0, 0.0);
    transport(this_object(), "field", xs[0], zs[0]);
}

string do_enter() {
    transport(this_player(), "walking_castle", 5.2, 0.0);
    return "You climb the drawbridge and pass under the gate into the castle yard.";
}

void heart_beat() {
    if (distance_to_point(xs[leg], zs[leg]) < 3.0) leg = (leg + 1) % 4;
    head_for(xs[leg], zs[leg]);
}
