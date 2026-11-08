// scooter.c - Söder's electric Voi in Beargarden (SODER-LENS-S291): one rider, standing, hands
// on the bars. "It's basically just a skateboard with a holding stick" (Andreas, 2026-11-08).
//
// Its drive says two things, as the board's does (vehicles/skateboard.c). To a lens that hosts
// sk8: "sim": "sk8_scooter", ride it with the simulation as a scooter: the stick is the throttle
// (as far as it is out, about 11 m/s at full, SODER-LENS-S290), FOOT BRAKE brakes, the trick
// button is a bunny hop, and the board's tricks have a scooter rider's names (tailwhips,
// barspins). To any other lens: Söder's throttle scooter (soder/world/vehicles/scooter.c).
//
// It stays where it is got off, like the board: "use" gets on, it goes where you go.
// @req SODER-LENS-S291

string rider;

void create() {
    set_name("A scooter");
    set_long("A coral Voi with a scuffed deck and a bell nobody uses, unlocked. Electric: open the throttle and go.");
    set_look("none");
    rider = "";
    set_mount("scooter");
    add_offer("ride", 1);
    add_verb("use", "Get on", "scooter", 1.8);
    add_verb("exit", "Get off", "scooter_off", 0.0);
    // What a lens without sk8 binds; with sk8 the controls are the sim's. The third word is the
    // icon the glass draws (SODER-LENS-S276).
    add_control("gas", "Throttle", "throttle");
    add_control("brake", "Brake", "brake");
    add_control("steer", "Steer", "bars");
    add_control("trick", "Hop", "ollie");
    set_drive(([
        "kind": "throttle", "cruise": 11.1, "top": 12.5, "power": 6.0, "brake": 9.0,
        "turn": 1.9, "lean": 0.3, "chassis": "lean",
        "chase": (["back": 4.0, "height": 2.3, "aim": 1.4]),
        // The sk8 simulation as a scooter, tuned as the board is.
        "sim": "sk8_scooter",
        "sim_tuning": "skate/tuning.txt",
        "sim_stance_attr": "skate_stance",
        "sim_difficulty_attr": "skate_difficulty",
        "sim_events": "skate"
    ]));
}

void put(float x, float z) { place(x, z); }

// Free when nobody rides it, or when its rider has gone (left the world, or got on something
// else).
int is_free() {
    if (rider != "" && query_attr(rider, "riding") != "scooter") {
        rider = "";
        follow("");
        set_mount("scooter");
    }
    return rider == "";
}

string board(string who) {
    if (rider == who) return "scooter";
    if (!is_free()) return "";
    rider = who;
    set_mount("");
    follow(who);
    set_attr(who, "riding", "scooter");
    return "scooter";
}

void unboard(string who) {
    float yaw;
    if (rider != who) return;
    rider = "";
    follow("");
    set_mount("scooter");
    set_attr(who, "riding", 0);
    yaw = query_spot_yaw();
    set_down(who, query_spot_x() + 0.7 * sin(yaw), query_spot_z() - 0.7 * cos(yaw));
}

string seat_of(string who) { return rider == who ? "rider" : ""; }
string exit_message() { return "You get off the scooter."; }
string refuse_message() { return "Somebody is already on that scooter."; }
