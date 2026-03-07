// Advanced Oxide features demo

type Distance = f64;
type Name = String;

const MAX_ITEMS: i64 = 10;
static VERSION: i64 = 1;

fn main() {
    // Constants
    println!("Max items: {}", MAX_ITEMS);
    println!("Version: {}", VERSION);

    // HashMap
    let mut scores = HashMap::new();
    scores.insert("alice", 95);
    scores.insert("bob", 87);
    scores.insert("carol", 92);

    println!("Scores: {:?}", scores);
    println!("Alice's score: {}", scores.get("alice").unwrap());
    println!("Has bob? {}", scores.contains_key("bob"));
    println!("Keys: {:?}", scores.keys());
    println!("Count: {}", scores.len());

    // HashMap iteration with destructuring
    for (name, score) in scores {
        println!("  {} scored {}", name, score);
    }

    // Tuple destructuring in for loops
    let pairs = vec![(1, "one"), (2, "two"), (3, "three")];
    for (num, word) in pairs {
        println!("{} = {}", num, word);
    }

    // CLI args
    let args = std::env::args();
    println!("Program args: {:?}", args);
}
