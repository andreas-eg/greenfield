// field.c — the green field, greenfield's one place outdoors.
//
// Grass to the horizon and a gravel ring sixty metres across, which the
// walking castle walks round (vehicles/walking_castle.c). The chart is
// geogen/model/greenfield.py's. Players arrive here (SPAWN=field).
// @req SODER-WORLD-S72  The walking castle in a realm of its own
void create() {
    set_title("The green field");
    set_description("Open grass on every side, and a ring of gravel trodden into it by something with very large feet. Now and then the ground shakes.");
    set_chart("greenfield");
    set_area(-80.0, -80.0, 80.0, 80.0);
}
