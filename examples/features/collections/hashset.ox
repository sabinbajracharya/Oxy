// === Feature: Collections — Set ===
// Set stores unique values. Created via `Set::new()`. Supports
// insert, contains, remove, set operations (union, intersection,
// difference), iteration, and clone.

// === Construction ===

#[test]
fn test_new_empty() {
    let s = Set::new();
    assert_eq(s.len(), 0);
    assert(s.is_empty());
}

// === insert ===

#[test]
fn test_insert() {
    let mut s = Set::new();
    let was_new = s.insert(1);
    assert(was_new);
    assert_eq(s.len(), 1);
}

#[test]
fn test_insert_duplicate() {
    let mut s = Set::new();
    s.insert(1);
    let was_new = s.insert(1);
    assert(!was_new);
    assert_eq(s.len(), 1);
}

#[test]
fn test_insert_multiple() {
    let mut s = Set::new();
    s.insert("a");
    s.insert("b");
    s.insert("c");
    assert_eq(s.len(), 3);
}

// === contains ===

#[test]
fn test_contains() {
    let mut s = Set::new();
    s.insert(42);
    assert(s.contains(42));
    assert(!s.contains(99));
}

#[test]
fn test_contains_empty() {
    let s = Set::new();
    assert(!s.contains(1));
}

// === remove ===

#[test]
fn test_remove_existing() {
    let mut s = Set::new();
    s.insert("hello");
    let existed = s.remove("hello");
    assert(existed);
    assert_eq(s.len(), 0);
    assert(!s.contains("hello"));
}

#[test]
fn test_remove_missing() {
    let mut s = Set::new();
    s.insert(1);
    let existed = s.remove(42);
    assert(!existed);
    assert_eq(s.len(), 1);
}

// === union ===

#[test]
fn test_union() {
    let mut a = Set::new();
    a.insert(1);
    a.insert(2);
    let mut b = Set::new();
    b.insert(2);
    b.insert(3);
    let u = a.union(b);
    assert_eq(u.len(), 3);
}

// === intersection ===

#[test]
fn test_intersection() {
    let mut a = Set::new();
    a.insert(1);
    a.insert(2);
    a.insert(3);
    let mut b = Set::new();
    b.insert(2);
    b.insert(3);
    b.insert(4);
    let inter = a.intersection(b);
    assert_eq(inter.len(), 2);
}

// === difference ===

#[test]
fn test_difference() {
    let mut a = Set::new();
    a.insert(1);
    a.insert(2);
    a.insert(3);
    let mut b = Set::new();
    b.insert(2);
    let diff = a.difference(b);
    assert_eq(diff.len(), 2);
}

// === to_vec ===

#[test]
fn test_to_vec() {
    let mut s = Set::new();
    s.insert("c");
    s.insert("a");
    s.insert("b");
    let v = s.to_vec();
    assert_eq(v.len(), 3);
}

// === Iteration ===

#[test]
fn test_iteration() {
    let mut s = Set::new();
    s.insert(10);
    s.insert(20);
    s.insert(30);
    let mut count = 0;
    for val in s {
        count = count + 1;
    }
    assert_eq(count, 3);
}

// === clone ===

#[test]
fn test_clone() {
    let mut s = Set::new();
    s.insert("x");
    let s2 = s.clone();
    assert_eq(s2.len(), 1);
    assert(s2.contains("x"));
}
