// examples/json.ox — JSON serialization & deserialization in Oxy

struct Person {
    name: String,
    age: Int,
}

enum Status {
    Active,
    Inactive,
}

fn main() {
    // --- Serialize primitives ---
    io::println("=== Primitives ===");
    io::println("{}", json::serialize(42).unwrap());
    io::println("{}", json::serialize(3.14).unwrap());
    io::println("{}", json::serialize(true).unwrap());
    io::println("{}", json::serialize("hello world").unwrap());
    io::println("{}", json::serialize(()).unwrap());

    // --- Serialize a List ---
    io::println("\n=== List ===");
    val numbers = [1, 2, 3, 4, 5];
    io::println("{}", json::serialize(numbers).unwrap());

    // --- Serialize a Map ---
    io::println("\n=== Map ===");
    var config = Map::new();
    config.insert("host", "localhost");
    config.insert("port", "8080");
    io::println("{}", json::serialize(config).unwrap());

    // --- Serialize a Struct ---
    io::println("\n=== Struct ===");
    val alice = Person { name: "Alice".to_string(), age: 30 };
    io::println("{}", json::serialize(alice).unwrap());

    // --- Serialize Enums ---
    io::println("\n=== Enum ===");
    io::println("{}", json::serialize(Status::Active).unwrap());

    // --- Pretty printing ---
    io::println("\n=== Pretty Print ===");
    val data = [1, 2, 3];
    io::println("{}", json::to_string_pretty(data).unwrap());

    // --- Deserialize JSON ---
    io::println("\n=== Deserialize ===");
    val parsed = json::deserialize("{\"name\": \"Bob\", \"age\": 25}").unwrap();
    val name = parsed.get("name").unwrap();
    val age = parsed.get("age").unwrap();
    io::println("Name: {}, Age: {:?}", name, age);

    val arr = json::parse("[10, 20, 30]").unwrap();
    io::println("Array: {:?}", arr);

    // --- Round-trip ---
    io::println("\n=== Round-trip ===");
    val original = [100, 200, 300];
    val json_str = json::serialize(original).unwrap();
    io::println("JSON:   {}", json_str);
    val restored = json::deserialize(json_str).unwrap();
    io::println("Parsed: {:?}", restored);

    // --- .to_json() method ---
    io::println("\n=== .to_json() method ===");
    val v = [1, 2, 3];
    io::println("{}", v.to_json().unwrap());

    // --- Typed deserialization ---
    io::println("\n=== Typed Deserialization ===");
    val json_str = "{\"name\": \"Charlie\", \"age\": 40}";
    val person = json::from_struct(json_str, "Person").unwrap();
    io::println("{:?}", person);

    io::println("\nDone!");
}
