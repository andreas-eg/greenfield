// session.c - where a session starts at a spot: two minutes, scored, with a card and the
// spot's best (support/skate_rules.c). Started from the round SESSION button the HUD shows at a
// spot (hud/skate.json, the soul's `session`), not offered as a verb (SODER-LENS-S289).
inherit "support/skate_rules";

void create() {
    set_name("A session");
    set_long("Two minutes on the clock. Every line you land counts; a bail only loses what was in the air.");
    set_look("none");
}

string do_session() {
    return skate_session_start(this_player());
}
