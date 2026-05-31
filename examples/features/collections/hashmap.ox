// === Feature: Collections — Map ===
// Map stores key-value pairs. Created via `Map::new()`. Supports
// insert, get, remove, bracket access, iteration, and shared mutation.

// === Construction ===

#[test]
fn test_new_empty() {
    val m = Map::new();
    assert_eq(m.len(), 0);
    assert(m.is_empty());
}

// === insert ===

#[test]
fn test_insert() {
    var m = Map::new();
    m.insert("a", 1);
    m.insert("b", 2);
    m.insert("c", 3);
    assert_eq(m.len(), 3);
    assert(!m.is_empty());
}

#[test]
fn test_insert_overwrite() {
    var m = Map::new();
    m.insert("key", 1);
    m.insert("key", 99);
    assert_eq(m.len(), 1);
}

// === get ===

#[test]
fn test_get_existing() {
    var m = Map::new();
    m.insert("hello", 42);
    val v = m.get("hello");
    assert(v.is_some());
}

#[test]
fn test_get_missing() {
    var m = Map::new();
    m.insert("a", 1);
    val v = m.get("nonexistent");
    assert(v.is_none());
}

// === get_or ===

#[test]
fn test_get_or_existing() {
    var m = Map::new();
    m.insert("x", 10);
    val v = m.get_or("x", 99);
    assert_eq(v, 10);
}

#[test]
fn test_get_or_missing() {
    var m = Map::new();
    val v = m.get_or("missing", 42);
    assert_eq(v, 42);
}

// === remove ===

#[test]
fn test_remove_existing() {
    var m = Map::new();
    m.insert("a", 1);
    m.insert("b", 2);
    m.remove("a");
    assert_eq(m.len(), 1);
    assert(m.get("a").is_none());
}

#[test]
fn test_remove_missing() {
    var m = Map::new();
    m.insert("a", 1);
    m.remove("nope");
    assert_eq(m.len(), 1);
}

// === contains_key ===

#[test]
fn test_contains_key() {
    var m = Map::new();
    m.insert("hello", "world");
    assert(m.contains_key("hello"));
    assert(!m.contains_key("missing"));
}

// === keys / values ===

#[test]
fn test_keys() {
    var m = Map::new();
    m.insert("a", 1);
    m.insert("b", 2);
    val keys = m.keys();
    assert_eq(keys.len(), 2);
}

#[test]
fn test_values() {
    var m = Map::new();
    m.insert("a", 1);
    m.insert("b", 2);
    val vals = m.values();
    assert_eq(vals.len(), 2);
}

// === Bracket Access ===

#[test]
fn test_bracket_get() {
    var m = Map::new();
    m.insert("key", 42);
    assert_eq(m["key"], 42);
}

// === Iteration ===

#[test]
fn test_iteration() {
    var m = Map::new();
    m.insert("x", 10);
    m.insert("y", 20);
    var count = 0;
    for pair in m {
        count = count + 1;
    }
    assert_eq(count, 2);
}

// === clone ===

#[test]
fn test_clone() {
    var m = Map::new();
    m.insert("a", 1);
    val m2 = m.clone();
    assert_eq(m2.len(), 1);
}
