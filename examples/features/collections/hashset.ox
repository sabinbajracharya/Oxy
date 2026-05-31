// === Feature: Collections — Set ===
// Set stores unique values. Created via `Set::new()`. Supports
// insert, contains, remove, set operations (union, intersection,
// difference), iteration, and clone.

// === Construction ===

#[test]
fn test_new_empty() {
    val s = Set::new();
    assert_eq(s.len(), 0);
    assert(s.is_empty());
}

// === insert ===

#[test]
fn test_insert() {
    var s = Set::new();
    val was_new = s.insert(1);
    assert(was_new);
    assert_eq(s.len(), 1);
}

#[test]
fn test_insert_duplicate() {
    var s = Set::new();
    s.insert(1);
    val was_new = s.insert(1);
    assert(!was_new);
    assert_eq(s.len(), 1);
}

#[test]
fn test_insert_multiple() {
    var s = Set::new();
    s.insert("a");
    s.insert("b");
    s.insert("c");
    assert_eq(s.len(), 3);
}

// === contains ===

#[test]
fn test_contains() {
    var s = Set::new();
    s.insert(42);
    assert(s.contains(42));
    assert(!s.contains(99));
}

#[test]
fn test_contains_empty() {
    val s = Set::new();
    assert(!s.contains(1));
}

// === remove ===

#[test]
fn test_remove_existing() {
    var s = Set::new();
    s.insert("hello");
    val existed = s.remove("hello");
    assert(existed);
    assert_eq(s.len(), 0);
    assert(!s.contains("hello"));
}

#[test]
fn test_remove_missing() {
    var s = Set::new();
    s.insert(1);
    val existed = s.remove(42);
    assert(!existed);
    assert_eq(s.len(), 1);
}

// === union ===

#[test]
fn test_union() {
    var a = Set::new();
    a.insert(1);
    a.insert(2);
    var b = Set::new();
    b.insert(2);
    b.insert(3);
    val u = a.union(b);
    assert_eq(u.len(), 3);
}

// === intersection ===

#[test]
fn test_intersection() {
    var a = Set::new();
    a.insert(1);
    a.insert(2);
    a.insert(3);
    var b = Set::new();
    b.insert(2);
    b.insert(3);
    b.insert(4);
    val inter = a.intersection(b);
    assert_eq(inter.len(), 2);
}

// === difference ===

#[test]
fn test_difference() {
    var a = Set::new();
    a.insert(1);
    a.insert(2);
    a.insert(3);
    var b = Set::new();
    b.insert(2);
    val diff = a.difference(b);
    assert_eq(diff.len(), 2);
}

// === to_vec ===

#[test]
fn test_to_vec() {
    var s = Set::new();
    s.insert("c");
    s.insert("a");
    s.insert("b");
    val v = s.to_vec();
    assert_eq(v.len(), 3);
}

// === Iteration ===

#[test]
fn test_iteration() {
    var s = Set::new();
    s.insert(10);
    s.insert(20);
    s.insert(30);
    var count = 0;
    for v in s {
        count = count + 1;
    }
    assert_eq(count, 3);
}

// === clone ===

#[test]
fn test_clone() {
    var s = Set::new();
    s.insert("x");
    val s2 = s.clone();
    assert_eq(s2.len(), 1);
    assert(s2.contains("x"));
}
