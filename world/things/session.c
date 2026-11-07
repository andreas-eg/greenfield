// session.c - where a session starts at a spot: two minutes, scored, with a card and the
// spot's best (support/skate_rules.c). Offered from anywhere near the spot's start.
inherit "support/skate_rules";

void create() {
    set_name("A session");
    set_long("Two minutes on the clock. Every line you land counts; a bail only loses what was in the air.");
    set_look("none");
    add_verb("session", "Start a two-minute session", "skateboard", 40.0);
}

string do_session() {
    return skate_session_start(this_player());
}
