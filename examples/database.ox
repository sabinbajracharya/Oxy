// Example: SQLite database in Oxy
//
// Run with: oxide run examples/database.ox

fn main() {
    // Open an in-memory database (use Db::open("myapp.db") for a file)
    val db = Db::memory();

    // Create a table
    db.execute("CREATE TABLE users (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        email TEXT,
        age INTEGER
    )");

    // Insert data with parameterized queries (prevents SQL injection)
    db.execute("INSERT INTO users (name, email, age) VALUES (?1, ?2, ?3)",
        ["Alice", "alice@example.com", 30]);
    db.execute("INSERT INTO users (name, email, age) VALUES (?1, ?2, ?3)",
        ["Bob", "bob@example.com", 25]);
    db.execute("INSERT INTO users (name, email, age) VALUES (?1, ?2, ?3)",
        ["Charlie", "charlie@example.com", 35]);

    io::println("Last insert ID: {}", db.last_insert_id());

    // Query all users
    io::println("\nAll users:");
    val users = db.query("SELECT id, name, email, age FROM users ORDER BY name");
    for user in users {
        io::println("  {} - {} ({}) age {}",
            user.get("id").unwrap(),
            user.get("name").unwrap(),
            user.get("email").unwrap(),
            user.get("age").unwrap());
    }

    // Query with parameters
    io::println("\nUsers older than 28:");
    val older = db.query("SELECT name, age FROM users WHERE age > ?1", [28]);
    for user in older {
        io::println("  {} (age {})",
            user.get("name").unwrap(),
            user.get("age").unwrap());
    }

    // Query a single row
    val bob = db.query_row("SELECT name, email FROM users WHERE name = ?1", ["Bob"]);
    match bob {
        Some(row) => io::println("\nFound Bob: {}", row.get("email").unwrap()),
        None => io::println("\nBob not found"),
    }

    // Update and check affected rows
    val updated = db.execute("UPDATE users SET age = ?1 WHERE name = ?2", [26, "Bob"]);
    io::println("\nUpdated {} row(s)", updated);

    // Delete
    val deleted = db.execute("DELETE FROM users WHERE age > ?1", [30]);
    io::println("Deleted {} row(s)", deleted);

    db.close();
    io::println("\nDone!");
}
