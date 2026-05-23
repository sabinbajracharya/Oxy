// === Feature: Collections — BTreeMap ===
// BTreeMap stores key-value pairs in sorted key order. Created via
// `BTreeMap::new()`. Supports insert, get, remove, bracket access,
// iteration (in key order), and clone.

// === Construction ===

#[test]
fn test_new_empty() {
    let m = BTreeMap::new();
    assert_eq!(m.len(), 0);
    assert!(m.is_empty());
}

// === insert ===

#[test]
fn test_insert() {
    let mut m = BTreeMap::new();
    m.insert("a", 1);
    m.insert("b", 2);
    m.insert("c", 3);
    assert_eq!(m.len(), 3);
    assert!(!m.is_empty());
}

#[test]
fn test_insert_overwrite() {
    let mut m = BTreeMap::new();
    m.insert("key", 1);
    m.insert("key", 99);
    assert_eq!(m.len(), 1);
}

// === get ===

#[test]
fn test_get_existing() {
    let mut m = BTreeMap::new();
    m.insert("hello", 42);
    let v = m.get("hello");
    assert!(v.is_some());
}

#[test]
fn test_get_missing() {
    let mut m = BTreeMap::new();
    m.insert("a", 1);
    let v = m.get("nonexistent");
    assert!(v.is_none());
}

// === get_or ===

#[test]
fn test_get_or_existing() {
    let mut m = BTreeMap::new();
    m.insert("x", 10);
    let v = m.get_or("x", 99);
    assert_eq!(v, 10);
}

#[test]
fn test_get_or_missing() {
    let mut m = BTreeMap::new();
    let v = m.get_or("missing", 42);
    assert_eq!(v, 42);
}

// === remove ===

#[test]
fn test_remove_existing() {
    let mut m = BTreeMap::new();
    m.insert("a", 1);
    m.insert("b", 2);
    m.remove("a");
    assert_eq!(m.len(), 1);
    assert!(m.get("a").is_none());
}

#[test]
fn test_remove_missing() {
    let mut m = BTreeMap::new();
    m.insert("a", 1);
    m.remove("nope");
    assert_eq!(m.len(), 1);
}

// === contains_key ===

#[test]
fn test_contains_key() {
    let mut m = BTreeMap::new();
    m.insert("hello", "world");
    assert!(m.contains_key("hello"));
    assert!(!m.contains_key("missing"));
}

// === keys / values ===

#[test]
fn test_keys() {
    let mut m = BTreeMap::new();
    m.insert("b", 2);
    m.insert("a", 1);
    let keys = m.keys();
    assert_eq!(keys.len(), 2);
    // BTreeMap returns keys in sorted order
    assert_eq!(keys[0], "a");
    assert_eq!(keys[1], "b");
}

#[test]
fn test_values() {
    let mut m = BTreeMap::new();
    m.insert("b", 2);
    m.insert("a", 1);
    let vals = m.values();
    assert_eq!(vals.len(), 2);
    // values follow key order
    assert_eq!(vals[0], 1);
    assert_eq!(vals[1], 2);
}

// === Bracket Access ===

#[test]
fn test_bracket_get() {
    let mut m = BTreeMap::new();
    m.insert("key", 42);
    assert_eq!(m["key"], 42);
}

// === Iteration ===

#[test]
fn test_iteration() {
    let mut m = BTreeMap::new();
    m.insert("y", 20);
    m.insert("x", 10);
    let mut count = 0;
    for pair in m {
        count = count + 1;
    }
    assert_eq!(count, 2);
}

// === clone ===

#[test]
fn test_clone() {
    let mut m = BTreeMap::new();
    m.insert("a", 1);
    let m2 = m.clone();
    assert_eq!(m2.len(), 1);
}
