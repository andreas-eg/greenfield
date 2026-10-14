// walking_castle.c — the yard inside the walking castle (greenfield since SODER-WORLD-S72).
//
// A room with a chart of its own (lens tools/citybuilder/walking_castle.py),
// entered through the castle's gate on the street and left over its
// drawbridge to wherever the castle stands by then. The yard owns the castle:
// it makes it and sends it out, as Padrone's castle rooms owned his mover.
// The yard is in a fixed frame for now; carried by the castle is the next
// step (SF-WORLD-S22, analysis/architecture/moving-places.md).
// @req SODER-WORLD-S22  A walking castle on Götgatan
void create() {
    set_title("The castle yard");
    set_description("A paved yard inside grey walls, a tower at each corner. A well stands in the middle; the keep's oak door is at the far end, and the gate at your back opens onto the drawbridge. Now and then the whole yard sways, as if the ground itself took a step.");
    set_chart("walking_castle");
}

// Once, at boot (Engine.Populator): one castle, and its gate here.
void reset() {
    object castle;
    object gate;
    castle = clone_object("vehicles/walking_castle.c");
    castle->walk_out();
    gate = clone_object("things/castle_gate.c");
    gate->belongs_to(castle);
    gate->stands(6.3, 0.0, 1.571);
}
