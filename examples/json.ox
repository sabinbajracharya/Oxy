// examples/json.ox — JSON serialization & deserialization in Oxide

struct Person {
    name: String,
    age: i64,
}

enum Status {
    Active,
    Inactive,
}

fn main() {
    // --- Serialize primitives ---
    println!("=== Primitives ===");
    println!("{}", json::serialize(42).unwrap());
    println!("{}", json::serialize(3.14).unwrap());
    println!("{}", json::serialize(true).unwrap());
    println!("{}", json::serialize("hello world").unwrap());
    println!("{}", json::serialize(()).unwrap());

    // --- Serialize a Vec ---
    println!("\n=== Vec ===");
    let numbers = vec![1, 2, 3, 4, 5];
    println!("{}", json::serialize(numbers).unwrap());

    // --- Serialize a HashMap ---
    println!("\n=== HashMap ===");
    let mut config = HashMap::new();
    config.insert("host", "localhost");
    config.insert("port", "8080");
    println!("{}", json::serialize(config).unwrap());

    // --- Serialize a Struct ---
    println!("\n=== Struct ===");
    let alice = Person { name: "Alice".to_string(), age: 30 };
    println!("{}", json::serialize(alice).unwrap());

    // --- Serialize Enums ---
    println!("\n=== Enum ===");
    println!("{}", json::serialize(Status::Active).unwrap());

    // --- Pretty printing ---
    println!("\n=== Pretty Print ===");
    let data = vec![1, 2, 3];
    println!("{}", json::to_string_pretty(data).unwrap());

    // --- Deserialize JSON ---
    println!("\n=== Deserialize ===");
    let parsed = json::deserialize("{\"name\": \"Bob\", \"age\": 25}").unwrap();
    let name = parsed.get("name").unwrap();
    let age = parsed.get("age").unwrap();
    println!("Name: {}, Age: {:?}", name, age);

    let arr = json::parse("[10, 20, 30]").unwrap();
    println!("Array: {:?}", arr);

    // --- Round-trip ---
    println!("\n=== Round-trip ===");
    let original = vec![100, 200, 300];
    let json_str = json::serialize(original).unwrap();
    println!("JSON:   {}", json_str);
    let restored = json::deserialize(json_str).unwrap();
    println!("Parsed: {:?}", restored);

    // --- .to_json() method ---
    println!("\n=== .to_json() method ===");
    let v = vec![1, 2, 3];
    println!("{}", v.to_json().unwrap());

    // --- Typed deserialization ---
    println!("\n=== Typed Deserialization ===");
    let json_str = "{\"name\": \"Charlie\", \"age\": 40}";
    let person = json::from_struct(json_str, "Person").unwrap();
    println!("{:?}", person);

    println!("\nDone!");
}
