// Advanced Oxy features demo

type Distance = Float;
type Name = String;

const MAX_ITEMS: Int = 10;
const VERSION: Int = 1;

fn main() {
    // Constants
    println("Max items: {}", MAX_ITEMS);
    println("Version: {}", VERSION);

    // Map
    var scores = Map::new();
    scores.insert("alice", 95);
    scores.insert("bob", 87);
    scores.insert("carol", 92);

    println("Scores: {:?}", scores);
    println("Alice's score: {}", scores.get("alice").unwrap());
    println("Has bob? {}", scores.contains_key("bob"));
    println("Keys: {:?}", scores.keys());
    println("Count: {}", scores.len());

    // Map iteration with destructuring
    for (name, score) in scores {
        println("  {} scored {}", name, score);
    }

    // Tuple destructuring in for loops
    val pairs = [(1, "one"), (2, "two"), (3, "three")];
    for (num, word) in pairs {
        println("{} = {}", num, word);
    }

    // CLI args
    val args = std::env::args();
    println("Program args: {:?}", args);
}
