// thing.c — the base every smart object inherits.
//
// "Sims light". The Sims put the intelligence in the objects, not the people:
// a fridge advertises that it relieves hunger, and a Sim only has to pick the
// best advertisement for whatever it needs most. In a MUD every object is
// already a program, so the fit is exact. A wizard writes a bench, clones it
// into a street, and every person in the realm can use it without a line of
// any person changing.
//
// A thing says four things about itself:
//   add_offer(need, amount)   what it gives, advertised to the room
//   use()                     what you actually get when you use it
//   pose()                    how a person looks while using it (a clip)
//   use_message()             what a player is told when they do
//
// People in the city and the player use things the same way: a pedestrian
// picks the best offer for their lowest need, a player presses "use" beside
// it. Neither side of that knows what a bench is.
//
// It has no body of its own: set_look("none"). A bench's mesh is already in
// the chart; this is the bench's meaning, standing where the picture is.

int amount;
// Kronor a player pays to use it; 0 is free. People in the city pay nothing:
// their money is not simulated, only the player's is.
// @req SODER-WORLD-S10  Things can cost money; a player who cannot pay is refused
int price;

void setup() {}

void create() {
    set_name("A thing");
    set_look("none");
    amount = 20;
    setup();
}

// A negative answer refuses: the driver tells the player refuse_message().
int use() {
    if (price > 0 && this_player() != 0) {
        if (query_attr(this_player(), "money") < price) return -1;
        add_attr(this_player(), "money", 0 - price);
    }
    return amount;
}
string refuse_message() {
    return "That is " + price + " kr, and you have " + query_attr(this_player(), "money") + ".";
}
string pose() { return "idle"; }
// What a player is told when they use it. Empty means the plain "You use
// the bench."; a thing worth using is worth a sentence of its own.
string use_message() { return ""; }

// Placed by whoever clones it: bench = clone_object(...); bench->put(x, z);
void put(float x, float z) { place(x, z); }

// Where a body sits on it and which way it faces there. A thing with no
// seat answers a yaw nobody could face, and a person stays on their feet
// beside it.
float seat_yaw() { return -100.0; }
float seat_x() { return 0.0; }
float seat_z() { return 0.0; }
// A thing with more than one seat says which is next (things/bench.c).
void take_seat() {}
