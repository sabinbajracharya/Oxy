// todo-cli — SQLite-backed todo list manager.
//
//   tug run -- add "Buy groceries"
//   tug run -- list
//   tug run -- list --all
//   tug run -- done 3
//   tug run -- remove 2
//   tug run -- clear

use cli_utils;

fn main() {
    let args = std::args::parse();

    if args.positionals.len() == 0 {
        cli_utils::die("expected a command: add, list, done, remove, clear");
    }

    let cmd = args.positionals.get(0).unwrap().to_string();

    let open_result = std::db::open("todos.db");
    let handle = match open_result {
        Ok(h) => h,
        Err(e) => {
            cli_utils::die("cannot open database: " + e);
            return;
        }
    };

    // ensure the table exists
    let _ = std::db::execute(handle, "CREATE TABLE IF NOT EXISTS todos (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        text TEXT NOT NULL,
        done INTEGER DEFAULT 0,
        created_at TEXT DEFAULT (datetime('now'))
    )");

    if cmd == "add" {
        if args.positionals.len() < 2 {
            let _ = std::db::close(handle);
            cli_utils::die("usage: tug run -- add <text>");
            return;
        }
        let text = args.positionals.get(1).unwrap().to_string();
        let exec_result = std::db::execute(handle, "INSERT INTO todos (text) VALUES (?1)", vec![text]);
        let id = std::db::last_insert_id(handle);
        match exec_result {
            Ok(_) => cli_utils::success("added todo #" + id.to_string()),
            Err(e) => cli_utils::fail("error: " + e),
        }
    } else if cmd == "list" {
        let show_all = args.flags.contains_key("all") || args.flags.contains_key("a");
        let sql = if show_all {
            "SELECT id, text, done FROM todos ORDER BY id"
        } else {
            "SELECT id, text, done FROM todos WHERE done = 0 ORDER BY id"
        };
        let query_result = std::db::query(handle, sql);
        match query_result {
            Ok(rows) => {
                if rows.len() == 0 {
                    cli_utils::info("no todos found");
                } else {
                    cli_utils::header("Todos");
                    for row in rows {
                        let id = row.get("id").unwrap();
                        let text = row.get("text").unwrap();
                        let done = row.get("done").unwrap();
                        let marker = if done.to_string() == "1" { "x" } else { " " };
                        println!("  [" + marker + "] [" + id.to_string() + "] " + text.to_string());
                    }
                }
            }
            Err(e) => cli_utils::fail("query error: " + e),
        }
    } else if cmd == "done" {
        if args.positionals.len() < 2 {
            let _ = std::db::close(handle);
            cli_utils::die("usage: tug run -- done <id>");
            return;
        }
        let id_str = args.positionals.get(1).unwrap().to_string();
        let id_result = id_str.parse_int();
        match id_result {
            Ok(id) => {
                let exec_result = std::db::execute(handle, "UPDATE todos SET done = 1 WHERE id = ?1", vec![id]);
                match exec_result {
                    Ok(rows) => {
                        if rows > 0 {
                            cli_utils::success("marked #" + id.to_string() + " as done");
                        } else {
                            cli_utils::fail("no todo with id " + id.to_string());
                        }
                    }
                    Err(e) => cli_utils::fail("error: " + e),
                }
            }
            Err(_) => cli_utils::fail("invalid id: " + id_str),
        }
    } else if cmd == "remove" || cmd == "rm" {
        if args.positionals.len() < 2 {
            let _ = std::db::close(handle);
            cli_utils::die("usage: tug run -- remove <id>");
            return;
        }
        let id_str = args.positionals.get(1).unwrap().to_string();
        let id_result = id_str.parse_int();
        match id_result {
            Ok(id) => {
                let exec_result = std::db::execute(handle, "DELETE FROM todos WHERE id = ?1", vec![id]);
                match exec_result {
                    Ok(rows) => {
                        if rows > 0 {
                            cli_utils::success("removed #" + id.to_string());
                        } else {
                            cli_utils::fail("no todo with id " + id.to_string());
                        }
                    }
                    Err(e) => cli_utils::fail("error: " + e),
                }
            }
            Err(_) => cli_utils::fail("invalid id: " + id_str),
        }
    } else if cmd == "clear" {
        let exec_result = std::db::execute(handle, "DELETE FROM todos WHERE done = 1");
        match exec_result {
            Ok(rows) => cli_utils::success("cleared " + rows.to_string() + " completed todo(s)"),
            Err(e) => cli_utils::fail("error: " + e),
        }
    } else {
        let _ = std::db::close(handle);
        cli_utils::die("unknown command: " + cmd);
        return;
    }

    let _ = std::db::close(handle);
}
