// Advanced Oxy features demo

type Distance = Float;
type Name = String;

const MAX_ITEMS: Int = 10;
const VERSION: Int = 1;

fn main() {
    // Constants
    io::println("Max items: {}", MAX_ITEMS);
    io::println("Version: {}", VERSION);

    // Map
    var scores = Map::new();
    scores.insert("alice", 95);
    scores.insert("bob", 87);
    scores.insert("carol", 92);

    io::println("Scores: {:?}", scores);
    io::println("Alice's score: {}", scores.get("alice").unwrap());
    io::println("Has bob? {}", scores.contains_key("bob"));
    io::println("Keys: {:?}", scores.keys());
    io::println("Count: {}", scores.len());

    // Map iteration with destructuring
    for (name, score) in scores {
        io::println("  {} scored {}", name, score);
    }

    // Tuple destructuring in for loops
    val pairs = [(1, "one"), (2, "two"), (3, "three")];
    for (num, word) in pairs {
        io::println("{} = {}", num, word);
    }

    // CLI args
    val args = std::env::args();
    io::println("Program args: {:?}", args);
}
