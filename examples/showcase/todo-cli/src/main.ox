// todo-cli — SQLite-backed todo list manager.
//
//   tug run -- add "Buy groceries"
//   tug run -- list
//   tug run -- list --all
//   tug run -- done 3
//   tug run -- remove 2
//   tug run -- clear

use std::db::open as db_open;
use std::db::execute as db_exec;
use std::db::query as db_query;
use std::db::last_insert_id as db_last_id;
use std::db::close as db_close;

use cli_utils::{header, info, success, fail, die};

fn main() {
    let args = std::args::parse();

    if args.positionals.len() == 0 {
        die("expected a command: add, list, done, remove, clear");
    }

    let cmd = args.positionals.get(0).unwrap().to_string();

    let open_result = db_open("todos.db");
    let handle = match open_result {
        Ok(h) => h,
        Err(e) => {
            die("cannot open database: " + e);
            return;
        }
    };

    // ensure the table exists
    let _ = db_exec(handle, "CREATE TABLE IF NOT EXISTS todos (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        text TEXT NOT NULL,
        done INTEGER DEFAULT 0,
        created_at TEXT DEFAULT (datetime('now'))
    )");

    if cmd == "add" {
        if args.positionals.len() < 2 {
            let _ = db_close(handle);
            die("usage: tug run -- add <text>");
            return;
        }
        let text = args.positionals.get(1).unwrap().to_string();
        let exec_result = db_exec(handle, "INSERT INTO todos (text) VALUES (?1)", [text]);
        let id = db_last_id(handle);
        match exec_result {
            Ok(_) => success("added todo #" + id.to_string()),
            Err(e) => fail("error: " + e),
        }
    } else if cmd == "list" {
        let show_all = args.flags.contains_key("all") || args.flags.contains_key("a");
        let sql = if show_all {
            "SELECT id, text, done FROM todos ORDER BY id"
        } else {
            "SELECT id, text, done FROM todos WHERE done = 0 ORDER BY id"
        };
        let query_result = db_query(handle, sql);
        match query_result {
            Ok(rows) => {
                if rows.len() == 0 {
                    info("no todos found");
                } else {
                    header("Todos");
                    for row in rows {
                        let id = row.get("id").unwrap();
                        let text = row.get("text").unwrap();
                        let done = row.get("done").unwrap();
                        let marker = if done.to_string() == "1" { "x" } else { " " };
                        println("  [" + marker + "] [" + id.to_string() + "] " + text.to_string());
                    }
                }
            }
            Err(e) => fail("query error: " + e),
        }
    } else if cmd == "done" {
        if args.positionals.len() < 2 {
            let _ = db_close(handle);
            die("usage: tug run -- done <id>");
            return;
        }
        let id_str = args.positionals.get(1).unwrap().to_string();
        let id_result = id_str.parse_int();
        match id_result {
            Ok(id) => {
                let exec_result = db_exec(handle, "UPDATE todos SET done = 1 WHERE id = ?1", [id]);
                match exec_result {
                    Ok(rows) => {
                        if rows > 0 {
                            success("marked #" + id.to_string() + " as done");
                        } else {
                            fail("no todo with id " + id.to_string());
                        }
                    }
                    Err(e) => fail("error: " + e),
                }
            }
            Err(_) => fail("invalid id: " + id_str),
        }
    } else if cmd == "remove" || cmd == "rm" {
        if args.positionals.len() < 2 {
            let _ = db_close(handle);
            die("usage: tug run -- remove <id>");
            return;
        }
        let id_str = args.positionals.get(1).unwrap().to_string();
        let id_result = id_str.parse_int();
        match id_result {
            Ok(id) => {
                let exec_result = db_exec(handle, "DELETE FROM todos WHERE id = ?1", [id]);
                match exec_result {
                    Ok(rows) => {
                        if rows > 0 {
                            success("removed #" + id.to_string());
                        } else {
                            fail("no todo with id " + id.to_string());
                        }
                    }
                    Err(e) => fail("error: " + e),
                }
            }
            Err(_) => fail("invalid id: " + id_str),
        }
    } else if cmd == "clear" {
        let exec_result = db_exec(handle, "DELETE FROM todos WHERE done = 1");
        match exec_result {
            Ok(rows) => success("cleared " + rows.to_string() + " completed todo(s)"),
            Err(e) => fail("error: " + e),
        }
    } else {
        let _ = db_close(handle);
        die("unknown command: " + cmd);
        return;
    }

    let _ = db_close(handle);
}
