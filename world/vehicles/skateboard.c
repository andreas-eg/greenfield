// skateboard.c - a Beargarden board. One seat, ridden standing, nothing to hold.
//
// Its drive says two things. To a lens that hosts sk8 (GF-REQ-LENS-1): "sim": "sk8", ride
// it with the simulation on the spot's surfaces (the chart's "skate" file), tuned by the
// realm's skate/tuning.txt, with the rider's stance and difficulty from their attributes.
// To any other lens: a kick board, as Söder's (SODER-LENS-S97), so it is rideable today.
//
// It stays where it is stepped off, like Söder's: "use" steps on, it goes where you go.

string rider;

void create() {
    set_name("A skateboard");
    set_long("An 8.25 deck on 53 mm wheels, the grip tape still black, the tail barely chipped. Beargarden's: step on and skate.");
    set_look("none");
    rider = "";
    set_mount("skateboard");
    add_offer("ride", 1);
    add_verb("use", "Step on", "skateboard", 1.8);
    add_verb("exit", "Step off", "skateboard_off", 0.0);
    // What a lens without sk8 binds; with sk8 the controls are the sim's (C.2).
    // The third word is the icon the glass draws in the circle (SODER-LENS-S276).
    add_control("kick", "Push", "push");
    add_control("brake", "Foot brake", "brake");
    add_control("steer", "Lean", "weight");
    add_control("trick", "Ollie", "ollie");
    set_drive(([
        "kind": "kick", "cruise": 4.0, "top": 8.5, "power": 2.0, "brake": 5.0,
        "turn": 2.4, "kick_fade": 0.65, "lean": 0.3, "chassis": "lean",
        "chase": (["back": 2.6, "height": 1.4, "aim": 0.5]),
        // GF-REQ-LENS-1: the sk8 simulation and what it reads.
        "sim": "sk8",
        "sim_tuning": "skate/tuning.txt",
        "sim_stance_attr": "skate_stance",
        "sim_difficulty_attr": "skate_difficulty",
        "sim_events": "skate"
    ]));
}

void put(float x, float z) { place(x, z); }

// Free when nobody rides it, or when its rider has gone (left the world, or stepped onto
// something else): a board is never held by somebody who is not on it.
int is_free() {
    if (rider != "" && query_attr(rider, "riding") != "skateboard") {
        rider = "";
        follow("");
        set_mount("skateboard");
    }
    return rider == "";
}

string board(string who) {
    if (rider == who) return "skateboard";
    if (!is_free()) return "";
    rider = who;
    set_mount("");
    follow(who);
    set_attr(who, "riding", "skateboard");
    return "skateboard";
}

void unboard(string who) {
    float yaw;
    if (rider != who) return;
    rider = "";
    follow("");
    set_mount("skateboard");
    set_attr(who, "riding", 0);
    yaw = query_spot_yaw();
    set_down(who, query_spot_x() + 0.6 * sin(yaw), query_spot_z() - 0.6 * cos(yaw));
}

string seat_of(string who) { return rider == who ? "deck" : ""; }
string exit_message() { return "You step off the board."; }
string refuse_message() { return "Somebody is already on that board."; }
