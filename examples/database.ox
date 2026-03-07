// Example: SQLite database in Oxide
//
// Run with: oxide run examples/database.ox

fn main() {
    // Open an in-memory database (use Db::open("myapp.db") for a file)
    let db = Db::memory();

    // Create a table
    db.execute("CREATE TABLE users (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        email TEXT,
        age INTEGER
    )");

    // Insert data with parameterized queries (prevents SQL injection)
    db.execute("INSERT INTO users (name, email, age) VALUES (?1, ?2, ?3)",
        vec!["Alice", "alice@example.com", 30]);
    db.execute("INSERT INTO users (name, email, age) VALUES (?1, ?2, ?3)",
        vec!["Bob", "bob@example.com", 25]);
    db.execute("INSERT INTO users (name, email, age) VALUES (?1, ?2, ?3)",
        vec!["Charlie", "charlie@example.com", 35]);

    println!("Last insert ID: {}", db.last_insert_id());

    // Query all users
    println!("\nAll users:");
    let users = db.query("SELECT id, name, email, age FROM users ORDER BY name");
    for user in users {
        println!("  {} - {} ({}) age {}",
            user.get("id").unwrap(),
            user.get("name").unwrap(),
            user.get("email").unwrap(),
            user.get("age").unwrap());
    }

    // Query with parameters
    println!("\nUsers older than 28:");
    let older = db.query("SELECT name, age FROM users WHERE age > ?1", vec![28]);
    for user in older {
        println!("  {} (age {})",
            user.get("name").unwrap(),
            user.get("age").unwrap());
    }

    // Query a single row
    let bob = db.query_row("SELECT name, email FROM users WHERE name = ?1", vec!["Bob"]);
    match bob {
        Some(row) => println!("\nFound Bob: {}", row.get("email").unwrap()),
        None => println!("\nBob not found"),
    }

    // Update and check affected rows
    let updated = db.execute("UPDATE users SET age = ?1 WHERE name = ?2", vec![26, "Bob"]);
    println!("\nUpdated {} row(s)", updated);

    // Delete
    let deleted = db.execute("DELETE FROM users WHERE age > ?1", vec![30]);
    println!("Deleted {} row(s)", deleted);

    db.close();
    println!("\nDone!");
}
