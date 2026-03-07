// Todo App — a practical Ferrite demo using SQLite
// Usage: ferrite run todo.fe <command> [args]

fn main() {
    let args = std::env::args();
    let db = Db::open("todos.db");
    db.execute("CREATE TABLE IF NOT EXISTS todos (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, done INTEGER DEFAULT 0)");

    if args.len() < 2 {
        println!("📝 Ferrite Todo App");
        println!("");
        println!("Usage:");
        println!("  ferrite run todo.fe list           - Show all todos");
        println!("  ferrite run todo.fe add <task>      - Add a new todo");
        println!("  ferrite run todo.fe done <id>       - Mark as complete");
        println!("  ferrite run todo.fe remove <id>     - Delete a todo");
        println!("  ferrite run todo.fe clear           - Remove completed");
        db.close();
        return;
    }

    let cmd = args[1];

    if cmd == "list" {
        let rows = db.query("SELECT id, title, done FROM todos ORDER BY done, id");
        if rows.len() == 0 {
            println!("📋 No todos yet! Add one with: ferrite run todo.fe add <task>");
        } else {
            println!("");
            println!("📋 Your Todos:");
            println!("--------------------------------------------------");
            let mut done_count = 0;
            let mut total = 0;
            for row in rows {
                let id = row.get("id").unwrap();
                let title = row.get("title").unwrap();
                let done = row.get("done").unwrap();
                if done == 1 {
                    println!("  ✅ [{}] {}", id, title);
                    done_count = done_count + 1;
                } else {
                    println!("  ⬜ [{}] {}", id, title);
                }
                total = total + 1;
            }
            println
            println!("📊 {}/{} completed", done_count, total);
        }
    } else if cmd == "add" {
        if args.len() < 3 {
            println!("❌ Usage: ferrite run todo.fe add <task>");
        } else {
            let mut parts = vec![args[2]];
            let mut i = 3;
            while i < args.len() {
                parts.push(args[i]);
                i = i + 1;
            }
            let title = parts.join(" ");
            db.execute("INSERT INTO todos (title) VALUES (?)", vec![title]);
            let id = db.last_insert_id();
            println!("✅ Added todo #{}: {}", id, title);
        }
    } else if cmd == "done" {
        if args.len() < 3 {
            println!("❌ Usage: ferrite run todo.fe done <id>");
        } else {
            let id = args[2];
            db.execute("UPDATE todos SET done = 1 WHERE id = ?", vec![id]);
            let rows = db.query("SELECT title FROM todos WHERE id = ?", vec![id]);
            if rows.len() > 0 {
                println!("🎉 Completed: {}", rows[0].get("title").unwrap());
            } else {
                println!("❌ Todo #{} not found", id);
            }
        }
    } else if cmd == "remove" {
        if args.len() < 3 {
            println!("❌ Usage: ferrite run todo.fe remove <id>");
        } else {
            let id = args[2];
            let rows = db.query("SELECT title FROM todos WHERE id = ?", vec![id]);
            if rows.len() > 0 {
                db.execute("DELETE FROM todos WHERE id = ?", vec![id]);
                println!("🗑️  Removed: {}", rows[0].get("title").unwrap());
            } else {
                println!("❌ Todo #{} not found", id);
            }
        }
    } else if cmd == "clear" {
        db.execute("DELETE FROM todos WHERE done = 1");
        println!("🧹 Cleared all completed todos");
    } else {
        println!("❌ Unknown command: {}", cmd);
    }

    db.close();
}
