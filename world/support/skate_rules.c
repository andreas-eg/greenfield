// skate_rules.c - Beargarden's scoring, as the world keeps it. A content library inherited
// by the soul (players/player.c) and the session things, not a room.
//
// The lens rides the board with the sk8 simulation (beargarden/sk8) and scores as it goes
// for instant feedback; it tells the world what happened in `skate ...` lines. The world
// decides what is kept: it takes a sequence only from a rider on a Beargarden board in a
// spot, never more than its own cap for the tricks named, and never faster than a skater
// can land them. Trick values are the spec's (sk8t/skateboard.md B.14.1), the same table
// as beargarden/sk8/src/scoring.rs.
//
// Lines the lens sends (GF-REQ-LENS-2):
//   skate banked <points> <multiplier> <name>+<name>+...   a sequence banked into the line
//   skate line <points>                                     the line ended, its total is score
//   skate bail <reason>                                     a bail: the open sequence is lost
//   skate land <clean|adjust|sketchy|spin>                  a landing
//   skate grind <name>                                      a grind started (for the HUD)

// The rooms that are skate spots (spots/*.c), by room name.
string *skate_spots() { return ({ "ovningsplanen", "plattan", "ralis", "fryshuset", "bjorns" }); }

// Where a skater arrives at each spot: its start mark (sk8/src/spots.rs), x then z.
float *skate_start(string spot) {
    if (spot == "plattan") return ({ 7.6, 13.0 });
    if (spot == "fryshuset") return ({ -6.0, 0.0 });
    if (spot == "bjorns") return ({ -1.5, 5.4 });
    return ({ 0.0, 0.0 });
}

// Base points by scoring key (B.14.1). A name holds its key: "Switch FS 180 Kickflip".
mapping skate_values() {
    return ([
        "Ollie": 50, "Nollie": 70, "Pop Shuvit": 150, "FS Pop Shuvit": 150,
        "Kickflip": 250, "Heelflip": 250, "Varial Kickflip": 400, "Varial Heelflip": 400,
        "Hardflip": 500, "Inward Heelflip": 500, "360 Pop Shuvit": 400, "360 Flip": 800,
        "Laser Flip": 800, "Late Kickflip": 450, "Late Heelflip": 450, "Late Shuvit": 350,
        "Front Flip": 1500, "Back Flip": 1500, "180": 100, "360": 350, "540": 800, "720": 1500,
        "Indy": 150, "Melon": 150, "Mute": 150, "Stalefish": 150, "Nosegrab": 150, "Tailgrab": 150,
        "Manual": 100, "Nose Manual": 100, "50-50": 150, "5-0": 250, "Nosegrind": 250,
        "Boardslide": 200, "Lipslide": 200, "Noseslide": 300, "Tailslide": 300,
        "Crooked": 450, "Overcrook": 450, "Smith": 450, "Feeble": 450, "Salad": 450, "Willy": 450,
        "Bluntslide": 600, "Noseblunt": 600, "Darkslide": 700, "Powerslide": 50, "Revert": 50,
        "Stall": 100, "Handplant": 500, "Air": 0
    ]);
}

// What one named trick can be worth at most: the best key it contains, plus what holding it
// (a grind's metres, a manual's seconds, a grab's seconds) or the air under it can add.
int skate_cap_of(string name) {
    mapping v;
    string *keys;
    int best;
    int i;
    v = skate_values();
    keys = m_indices(v);
    best = 0;
    for (i = 0; i < sizeof(keys); i++) {
        // The key as whole words in the name.
        if (sizeof(explode(" " + name + " ", " " + keys[i] + " ")) > 1 && v[keys[i]] > best) best = v[keys[i]];
    }
    // A flip with a spin and a grab in one name: their values add.
    if (sizeof(explode(name, " 180 ")) > 1 || sizeof(explode(name, " 360 ")) > 1) best += 350;
    if (sizeof(explode(name, " 540 ")) > 1 || sizeof(explode(name, " 720 ")) > 1) best += 1500;
    // Air height and distance, grind metres, manual and grab seconds: a long one is generous.
    return best + 1200;
}

// The most a sequence of these names may bank: their caps, the switch bonus, the line's
// multiplier (at most x3, B.14.3), and a clean landing.
int skate_cap(string *names, float multiplier) {
    int total;
    int i;
    total = 0;
    for (i = 0; i < sizeof(names); i++) total += skate_cap_of(names[i]);
    if (multiplier > 3.0) multiplier = 3.0;
    if (multiplier < 1.0) multiplier = 1.0;
    return to_int(to_float(total) * 1.5 * 1.1 * multiplier);
}

// Is `who` somewhere Beargarden scores: on one of its boards, in one of its spots?
int skate_scoring(object who) {
    if (query_attr(who, "riding") != "skateboard") return 0;
    return query_attr(who, "skate_spot") != 0;
}

// A banked sequence from the lens. Answers what the world keeps of it (0: nothing).
int skate_bank(object who, string args) {
    int points;
    float mult;
    string mults;
    string names;
    string *list;
    int cap;
    int now;
    int spent;
    // sscanf has no %f in this driver: the multiplier comes as text.
    if (sscanf(args, "%d %s %s", points, mults, names) != 3) return 0;
    mult = to_float(mults);
    // A trick's name comes with its spaces as underscores (a line is split on spaces).
    names = implode(explode(names, "_"), " ");
    if (!skate_scoring(who) || points <= 0) return 0;
    list = explode(names, "+");
    cap = skate_cap(list, mult);
    if (points > cap) points = cap;
    // No faster than a skater lands: at most 40 000 points in any 20 s of the world's ticks.
    now = query_attr(who, "skate_beat");
    if (now - query_attr(who, "skate_window_start") > 10) {
        set_attr(who, "skate_window_start", now);
        set_attr(who, "skate_window_points", 0);
    }
    spent = query_attr(who, "skate_window_points");
    if (spent + points > 40000) points = 40000 - spent;
    if (points <= 0) return 0;
    set_attr(who, "skate_window_points", spent + points);
    add_attr(who, "skate_line", points);
    set_attr(who, "skate_mult", to_int(mult * 10.0));
    set_attr(who, "skate_trick", implode(list, " + "));
    set_attr(who, "skate_flash", query_attr(who, "skate_flash") == 1 ? 2 : 1);
    if (query_attr(who, "skate_session") == 2) add_attr(who, "skate_session_score", points);
    return points;
}

// The line ended: what it held is score, for the spot's best and the session.
void skate_line_end(object who) {
    int line;
    line = query_attr(who, "skate_line");
    if (line <= 0) return;
    add_attr(who, "skate_score", line);
    set_attr(who, "skate_line", 0);
    set_attr(who, "skate_mult", 10);
    if (line > query_attr(who, "skate_best_line")) set_attr(who, "skate_best_line", line);
}

// A bail loses the open sequence; what the line had banked still counts (B.14.2).
void skate_bail(object who, string reason) {
    skate_line_end(who);
    set_attr(who, "skate_bail", reason);
    set_attr(who, "skate_bails", query_attr(who, "skate_bails") + 1);
}

// ---- sessions: two minutes at a spot, scored, with a card and a best ----------------------

int skate_session_beats() { return 60; }   // 60 world ticks of 2 s: two minutes

// Onto the first free board of the spot the player is at; 1 when they are on one.
// (Here so the session post's offer and the soul's `session` verb both board a skater.)
int board_here() {
    string room;
    object b;
    int n;
    room = query_room(this_player());
    for (n = 1; n <= 3; n++) {
        b = find_placed(room, "board_" + n);
        if (b && b->is_free()) {
            // The board comes to the skater, not the skater to wherever it was left.
            b->put(query_x(this_player()), query_z(this_player()));
            board_player(this_player(), b, 1);
            return 1;
        }
    }
    return 0;
}

string skate_session_start(object who) {
    string spot;
    spot = query_attr(who, "skate_spot");
    if (!spot) return "Sessions are skated at a spot: Plattan, Rålis, Fryshuset, Övningsplanen or Björns trädgård.";
    if (query_attr(who, "skate_session") == 1 || query_attr(who, "skate_session") == 2) return "A session is running. Skate it out.";
    // Started on foot (the post's offer is there to anyone near it): onto a board first. The
    // board's own hook says riding a moment later; the session needs it now. Both ways in, the
    // post's offer and the soul's `session`, come here (the offer did not board, 2026-11-08).
    if (query_attr(who, "riding") != "skateboard" && board_here()) set_attr(who, "riding", "skateboard");
    if (query_attr(who, "riding") != "skateboard") return "Step on a board first.";
    skate_line_end(who);
    set_attr(who, "skate_session", 1);
    set_attr(who, "skate_session_score", 0);
    set_attr(who, "skate_session_left", skate_session_beats());
    set_attr(who, "skate_session_spot", spot);
    return "Two minutes at " + spot + ". Three, two, one...";
}

// Every world tick from the soul's heart_beat.
string skate_session_tick(object who) {
    int state;
    int left;
    string spot;
    int best;
    object board;
    state = query_attr(who, "skate_session");
    if (state == 1) {
        set_attr(who, "skate_session", 2);
        return "";
    }
    if (state == 3) {
        if (add_attr(who, "skate_card_left", -1) <= 0) set_attr(who, "skate_session", 0);
        return "";
    }
    if (state != 2) return "";
    if (query_attr(who, "skate_spot") != query_attr(who, "skate_session_spot")) {
        set_attr(who, "skate_session", 0);
        return "You left the spot: the session is off.";
    }
    left = add_attr(who, "skate_session_left", -1);
    if (left > 0) return "";
    skate_line_end(who);
    spot = query_attr(who, "skate_session_spot");
    best = query_attr(who, "skate_best_" + spot);
    set_attr(who, "skate_session", 3);
    set_attr(who, "skate_card_left", 8);
    set_attr(who, "skate_result", query_attr(who, "skate_session_score"));
    if (query_attr(who, "skate_session_score") > best) {
        set_attr(who, "skate_best_" + spot, query_attr(who, "skate_session_score"));
        set_attr(who, "skate_record", 1);
    } else {
        set_attr(who, "skate_record", 0);
    }
    set_attr(who, "skate_best", query_attr(who, "skate_best_" + spot));
    board = find_placed(spot, "scoreboard");
    if (board) board->post(query_name(who), query_attr(who, "skate_session_score"));
    return "Time! " + query_attr(who, "skate_session_score") + " points at " + spot + ".";
}
