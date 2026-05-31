// === STRESS: std::db SQLite bindings ===

#[test]
fn test_open_in_memory_returns_handle() {
    let h = std::db::open_in_memory().unwrap();
    assert(h > 0);
    std::db::close(h);
}

#[test]
fn test_create_insert_query() {
    let h = std::db::open_in_memory().unwrap();
    let _ = std::db::execute(h, "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)").unwrap();
    let inserted = std::db::execute(h, "INSERT INTO t (name) VALUES (?1)", "Alice").unwrap();
    assert_eq(inserted, 1);
    let rows = std::db::query(h, "SELECT name FROM t").unwrap();
    assert_eq(rows.len(), 1);
    assert_eq(rows[0].get("name").unwrap(), "Alice");
    std::db::close(h);
}

#[test]
fn test_parameterized_query() {
    let h = std::db::open_in_memory().unwrap();
    std::db::execute(h, "CREATE TABLE t (id INTEGER PRIMARY KEY, age INTEGER)").unwrap();
    std::db::execute(h, "INSERT INTO t (age) VALUES (?1)", 25).unwrap();
    std::db::execute(h, "INSERT INTO t (age) VALUES (?1)", 40).unwrap();
    let rows = std::db::query(h, "SELECT age FROM t WHERE age > ?1", 30).unwrap();
    assert_eq(rows.len(), 1);
    assert_eq(rows[0].get("age").unwrap(), 40);
    std::db::close(h);
}

#[test]
fn test_last_insert_id() {
    let h = std::db::open_in_memory().unwrap();
    std::db::execute(h, "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)").unwrap();
    std::db::execute(h, "INSERT INTO t (v) VALUES (?1)", "first").unwrap();
    std::db::execute(h, "INSERT INTO t (v) VALUES (?1)", "second").unwrap();
    assert_eq(std::db::last_insert_id(h), 2);
    std::db::close(h);
}

#[test]
fn test_query_returns_multiple_rows_ordered() {
    let h = std::db::open_in_memory().unwrap();
    std::db::execute(h, "CREATE TABLE n (v INTEGER)").unwrap();
    std::db::execute(h, "INSERT INTO n VALUES (?1)", 3).unwrap();
    std::db::execute(h, "INSERT INTO n VALUES (?1)", 1).unwrap();
    std::db::execute(h, "INSERT INTO n VALUES (?1)", 2).unwrap();
    let rows = std::db::query(h, "SELECT v FROM n ORDER BY v").unwrap();
    assert_eq(rows.len(), 3);
    assert_eq(rows[0].get("v").unwrap(), 1);
    assert_eq(rows[1].get("v").unwrap(), 2);
    assert_eq(rows[2].get("v").unwrap(), 3);
    std::db::close(h);
}

#[test]
fn test_close_returns_true_then_false() {
    let h = std::db::open_in_memory().unwrap();
    assert(std::db::close(h));
    assert(!std::db::close(h));
}

#[test]
fn test_invalid_handle_errors() {
    let r = std::db::query(99999, "SELECT 1");
    assert(r.is_err());
}
