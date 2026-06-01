// === Feature: Collections — BTreeSet ===
// BTreeSet stores unique values in sorted order. Created via
// `BTreeSet::new()`. Supports insert, contains, remove, set operations
// (union, intersection, difference), iteration (in order), and clone.

// === Construction ===

#[test]
fn test_new_empty() {
    val s = BTreeSet::new();
    assert::eq(s.len(), 0);
    assert::true(s.is_empty());
}

// === insert ===

#[test]
fn test_insert() {
    var s = BTreeSet::new();
    val was_new = s.insert(1);
    assert::true(was_new);
    assert::eq(s.len(), 1);
}

#[test]
fn test_insert_duplicate() {
    var s = BTreeSet::new();
    s.insert(1);
    val was_new = s.insert(1);
    assert::true(!was_new);
    assert::eq(s.len(), 1);
}

#[test]
fn test_insert_multiple() {
    var s = BTreeSet::new();
    s.insert("c");
    s.insert("a");
    s.insert("b");
    assert::eq(s.len(), 3);
}

// === contains ===

#[test]
fn test_contains() {
    var s = BTreeSet::new();
    s.insert(42);
    assert::true(s.contains(42));
    assert::true(!s.contains(99));
}

#[test]
fn test_contains_empty() {
    val s = BTreeSet::new();
    assert::true(!s.contains(1));
}

// === remove ===

#[test]
fn test_remove_existing() {
    var s = BTreeSet::new();
    s.insert("hello");
    val existed = s.remove("hello");
    assert::true(existed);
    assert::eq(s.len(), 0);
    assert::true(!s.contains("hello"));
}

#[test]
fn test_remove_missing() {
    var s = BTreeSet::new();
    s.insert(1);
    val existed = s.remove(42);
    assert::true(!existed);
    assert::eq(s.len(), 1);
}

// === union ===

#[test]
fn test_union() {
    var a = BTreeSet::new();
    a.insert(1);
    a.insert(2);
    var b = BTreeSet::new();
    b.insert(2);
    b.insert(3);
    val u = a.union(b);
    assert::eq(u.len(), 3);
}

// === intersection ===

#[test]
fn test_intersection() {
    var a = BTreeSet::new();
    a.insert(1);
    a.insert(2);
    a.insert(3);
    var b = BTreeSet::new();
    b.insert(2);
    b.insert(3);
    b.insert(4);
    val inter = a.intersection(b);
    assert::eq(inter.len(), 2);
}

// === difference ===

#[test]
fn test_difference() {
    var a = BTreeSet::new();
    a.insert(1);
    a.insert(2);
    a.insert(3);
    var b = BTreeSet::new();
    b.insert(2);
    val diff = a.difference(b);
    assert::eq(diff.len(), 2);
}

// === to_vec ===

#[test]
fn test_to_vec() {
    var s = BTreeSet::new();
    s.insert(3);
    s.insert(1);
    s.insert(2);
    val v = s.to_vec();
    assert::eq(v.len(), 3);
    // BTreeSet returns elements in sorted order
    assert::eq(v[0], 1);
    assert::eq(v[1], 2);
    assert::eq(v[2], 3);
}

// === Iteration ===

#[test]
fn test_iteration() {
    var s = BTreeSet::new();
    s.insert(30);
    s.insert(10);
    s.insert(20);
    var count = 0;
    for v in s {
        count = count + 1;
    }
    assert::eq(count, 3);
}

// === clone ===

#[test]
fn test_clone() {
    var s = BTreeSet::new();
    s.insert("x");
    val s2 = s.clone();
    assert::eq(s2.len(), 1);
    assert::true(s2.contains("x"));
}
