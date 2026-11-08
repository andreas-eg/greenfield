// scoreboard.c - the spot's best sessions while the world runs: each skater's best, the top
// five shown when it is looked at. Not saved across a reboot of the world (GF-REQ-MULL-2).
mapping best;
string shown;

void create() {
    best = ([]);
    set_name("The scoreboard");
    set_look("none");
    shown = "Nobody has skated a session here yet.";
    set_long(shown);
    // Read from the cog (MENU, "Scoreboard"), shown as a card: not a button in the world
    // (SODER-LENS-S289).
}

// From the soul when a session ends.
void post(string who, int score) {
    string *names;
    string text;
    int i;
    int j;
    string t;
    if (score <= best[who]) return;
    best[who] = score;
    names = m_indices(best);
    // Highest first: a small insertion sort, there are never many.
    for (i = 1; i < sizeof(names); i++) {
        t = names[i];
        j = i - 1;
        while (j >= 0 && best[names[j]] < best[t]) {
            names[j + 1] = names[j];
            j--;
        }
        names[j + 1] = t;
    }
    text = "Best sessions here:";
    for (i = 0; i < sizeof(names) && i < 5; i++) text += " " + (i + 1) + ". " + names[i] + " " + best[names[i]] + ".";
    shown = text;
    set_long(text);
}

string do_read() {
    return shown;
}
