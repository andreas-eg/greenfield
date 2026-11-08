// player.c - the soul every Beargarden player lives by (SF-WORLD-S16): what they skate
// with, what the world keeps of what they do, and what the lens draws of it.
//
// The ride itself is the lens's: on a Beargarden board it runs the sk8 simulation (the
// board's drive says so, vehicles/skateboard.c) and says what happened in `skate ...`
// lines. This soul hears them, keeps score by support/skate_rules.c, runs sessions, and
// keeps a player's stance and difficulty, which the lens reads from their attributes.
inherit "support/skate_rules";

void login() {
    float *start;
    if (query_attr(this_player(), "skate_known") == 0) {
        set_attr(this_player(), "skate_known", 1);
        set_attr(this_player(), "skate_stance", "regular");
        set_attr(this_player(), "skate_difficulty", "normal");
    }
    // A skater: the band's and CharGen bodies with the skate clips (look/catalogue.json).
    // Players from before the skaters came stood on the board in the band's walking body.
    if (query_attr(this_player(), "skate_known") < 2) {
        set_attr(this_player(), "skate_known", 2);
        set_attr(this_player(), "skater", "hugo");
        set_body(this_player(), "skater_hugo");
    }
    // Arcade or realistic (the lens reads `skate_style` and tunes sk8 by world/skate/tuning.txt's
    // [arcade] or [realistic]): everyone starts arcade.
    if (query_attr(this_player(), "skate_known") < 3) {
        set_attr(this_player(), "skate_known", 3);
        set_attr(this_player(), "skate_style", "arcade");
    }
    // A reconnect never resumes a session or a line; the bests stay.
    set_attr(this_player(), "skate_session", 0);
    set_attr(this_player(), "skate_line", 0);
    set_attr(this_player(), "skate_mult", 10);
    set_attr(this_player(), "riding", 0);
    foreach (string a : ({ "skate_style", "skate_stance", "skate_difficulty", "skate_score", "skate_line", "skate_mult",
                    "skate_trick", "skate_flash", "skate_bail", "skate_session", "skate_session_score",
                    "skate_session_left", "skate_result", "skate_record", "skate_best", "skate_spot",
                    "skate_landing", "riding", "skate_board" }))
        show_attr(a);
    // Every visit starts at the spot's start, facing its first line: a new player arrived
    // where the room's middle is, which at Plattan faced the buildings.
    if (member(skate_spots(), query_room(this_player())) >= 0) {
        start = skate_start(query_room(this_player()));
        transport(this_player(), query_room(this_player()), start[0], start[1]);
    }
    set_hud(this_player(), "hud/skate.json");
    set_heart_beat(1);
}

void heart_beat() {
    string said;
    string room;
    add_attr(this_player(), "skate_beat", 1);
    // Which spot they are at: the rooms under spots/.
    room = query_room(this_player());
    if (member(skate_spots(), room) >= 0) set_attr(this_player(), "skate_spot", room);
    else set_attr(this_player(), "skate_spot", 0);
    said = skate_session_tick(this_player());
    if (said != "") write(said);
}

int command(string verb, string args) {
    string what;
    int kept;
    float *start;
    string room;
    object b;
    int n;
    string words;
    if (verb == "feel") {
        // After a session (skate_feel), how did that feel: 1 to 5 and a line if you like. Kept on
        // the player (skate_feelings, saved with them) and said back, so a phone's diagnostics,
        // which the app uploads, carry it as well (the sk8r prototype's feedback).
        words = "";
        if (!args || (sscanf(args, "%d %s", n, words) != 2 && sscanf(args, "%d", n) != 1)) n = 0;
        if (n < 1 || n > 5) {
            write("How did that feel? feel 1 to 5, and a line if you like.");
            return 1;
        }
        what = query_attr(this_player(), "skate_feelings");
        if (!what) what = "";
        set_attr(this_player(), "skate_feelings", what + query_attr(this_player(), "skate_session_spot") + " " + query_attr(this_player(), "skate_result") + " " + n + " " + words + "|");
        set_attr(this_player(), "skate_feel", 0);
        write("Feedback kept: " + n + " of 5" + (words != "" ? ", " + words : "") + ". Thanks!");
        return 1;
    }
    if (verb == "skate") {
        if (!args || args == "") return 0;
        if (sscanf(args, "banked %s", what) == 1) {
            kept = skate_bank(this_player(), what);
            return 1;
        }
        if (sscanf(args, "line %s", what) == 1) {
            skate_line_end(this_player());
            return 1;
        }
        if (sscanf(args, "bail %s", what) == 1) {
            skate_bail(this_player(), implode(explode(what, "_"), " "));
            return 1;
        }
        // The rider's clip, from the lens that rides sk8: everyone else sees it.
        if (sscanf(args, "clip %s", what) == 1) {
            if (query_attr(this_player(), "riding") == "skateboard" && (sscanf(what, "skate_%s", room) == 1 || what == "walk"))
                play_clip(this_player(), what, 3.0);
            return 1;
        }
        // The board's flip, from the lens that rides sk8 ("<turns>_<shuvit deg>_<seconds>"):
        // shown on the rider with a count in front, so the same flip twice is still news,
        // and every other lens turns this rider's board through it.
        if (sscanf(args, "board %s", what) == 1) {
            if (query_attr(this_player(), "riding") == "skateboard") {
                add_attr(this_player(), "skate_board_n", 1);
                set_attr(this_player(), "skate_board", query_attr(this_player(), "skate_board_n") + " " + what);
            }
            return 1;
        }
        if (sscanf(args, "land %s", what) == 1) {
            set_attr(this_player(), "skate_landing", what);
            return 1;
        }
        if (sscanf(args, "grind %s", what) == 1) {
            set_attr(this_player(), "skate_trick", implode(explode(what, "_"), " "));
            set_attr(this_player(), "skate_flash", query_attr(this_player(), "skate_flash") == 1 ? 2 : 1);
            return 1;
        }
        return 0;
    }
    // The lens's character chooser (CHARACTERS, and NEW GAME's): one of the skater bodies.
    if (verb == "character") {
        if (member(({ "skater_hugo", "skater_erik", "skater_zikai", "skater_npc06", "skater_npc09", "skater_npc20" }), args) < 0) {
            write("That skater is not here.");
            return 1;
        }
        set_body(this_player(), args);
        set_attr(this_player(), "skater", args);
        write("Character selected.");
        return 1;
    }
    // Outfits have no art in Beargarden yet: taken and kept, nothing else.
    if (verb == "outfit") {
        set_attr(this_player(), "outfit", args);
        return 1;
    }
    // Who skates: the band (Hugo, Erik, Zikai) or a CharGen skater (Sanna, Mo, Lova).
    if (verb == "skater") {
        string *who = ({ "hugo", "erik", "zikai", "sanna", "mo", "lova" });
        string *body = ({ "hugo", "erik", "zikai", "npc06", "npc09", "npc20" });
        n = member(who, args);
        if (n < 0) {
            write("Skate as hugo, erik, zikai, sanna, mo or lova.");
            return 1;
        }
        set_attr(this_player(), "skater", args);
        set_body(this_player(), "skater_" + body[n]);
        write("You skate as " + args + ".");
        return 1;
    }
    // The rider's settings, read by the lens's sk8 (B.15 difficulty, A.2 stance).
    if (verb == "stance") {
        if (args != "regular" && args != "goofy") {
            write("Stance is regular (left foot forward) or goofy (right foot forward).");
            return 1;
        }
        set_attr(this_player(), "skate_stance", args);
        write("You skate " + args + ".");
        return 1;
    }
    // On their own (MENU's rows): the next one round.
    if ((verb == "style" || verb == "stance" || verb == "difficulty") && (!args || args == "")) {
        if (verb == "style") args = query_attr(this_player(), "skate_style") == "realistic" ? "arcade" : "realistic";
        if (verb == "stance") args = query_attr(this_player(), "skate_stance") == "goofy" ? "regular" : "goofy";
        if (verb == "difficulty") {
            what = query_attr(this_player(), "skate_difficulty");
            args = what == "easy" ? "normal" : what == "normal" ? "hardcore" : "easy";
        }
    }
    if (verb == "style") {
        if (args != "arcade" && args != "realistic") {
            write("Style is arcade (big airs, easy grinds, long lines) or realistic (as a board really rides).");
            return 1;
        }
        set_attr(this_player(), "skate_style", args);
        write("You skate " + args + ".");
        return 1;
    }
    if (verb == "difficulty") {
        if (args != "easy" && args != "normal" && args != "hardcore") {
            write("Difficulty is easy, normal or hardcore.");
            return 1;
        }
        set_attr(this_player(), "skate_difficulty", args);
        write("Difficulty: " + args + ".");
        return 1;
    }
    if (verb == "session") {
        write(skate_session_start(this_player()));
        return 1;
    }
    if (verb == "score") {
        write("Score " + query_attr(this_player(), "skate_score") + ", best line " + query_attr(this_player(), "skate_best_line") + ".");
        return 1;
    }
    // Onto a free board at this spot, wherever it stands (the clients say this on arriving).
    if (verb == "board") {
        if (query_attr(this_player(), "riding") == "skateboard") return 1;
        write(board_here() ? "You step onto a board." : "Every board here is taken.");
        return 1;
    }
    // A spot's name on its own is the way there: mull's exits are compass directions only,
    // so this is a ride, not a door (GF-REQ-MULL-5). A skater steps off this spot's board,
    // goes, and steps onto one of the next spot's: the boards belong to their spots.
    if ((!args || args == "") && member(skate_spots(), verb) >= 0) {
        start = skate_start(verb);
        if (query_attr(this_player(), "skate_session") == 1 || query_attr(this_player(), "skate_session") == 2) set_attr(this_player(), "skate_session", 0);
        kept = query_attr(this_player(), "riding") == "skateboard";
        if (kept) do_command("exit");
        transport(this_player(), verb, start[0], start[1]);
        if (kept) board_here();
        return 1;
    }
    // A skater's words for getting on and off.
    if (verb == "push") return do_command("use skateboard");
    if (verb == "dismount" || verb == "step" && args == "off") return do_command("exit");
    return 0;
}
