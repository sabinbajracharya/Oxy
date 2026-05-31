// === Feature: Collections — List Operations ===
// Sorting, dedup, reverse, join, min, max, chunks, windows, clone,
// extend, shared mutation via Rc<RefCell<>>.

// === sort ===

#[test]
fn test_sort() {
    let mut v = list(3, 1, 4, 1, 5);
    v.sort();
    assert_eq(v[0], 1);
    assert_eq(v[4], 5);
}

#[test]
fn test_sort_empty() {
    let mut v = list();
    v.sort();
    assert_eq(v.len(), 0);
}

#[test]
fn test_sort_single() {
    let mut v = list(42);
    v.sort();
    assert_eq(v.len(), 1);
}

// === sort_by ===

#[test]
fn test_sort_by_descending() {
    let mut v = list(1, 3, 2);
    v.sort_by(|a, b| b - a);
    assert_eq(v[0], 3);
    assert_eq(v[2], 1);
}

// === sort_by_key ===

#[test]
fn test_sort_by_key() {
    let mut v = list("bb", "a", "ccc");
    v.sort_by_key(|s| s.len());
    assert_eq(v[0], "a");
}

// === reverse / rev ===

#[test]
fn test_reverse() {
    let mut v = list(1, 2, 3);
    v.reverse();
    assert_eq(v[0], 3);
    assert_eq(v[2], 1);
}

// === dedup ===

#[test]
fn test_dedup() {
    let mut v = list(1, 1, 2, 2, 3);
    v.dedup();
    assert(v.len() <= 3);
}

// === join ===

#[test]
fn test_join() {
    let v = list("a", "b", "c");
    let s = v.join(", ");
    assert(s.contains(","));
}

#[test]
fn test_join_single() {
    let v = list("hello");
    let s = v.join(", ");
    assert_eq(s, "hello");
}

#[test]
fn test_join_empty() {
    let v = list();
    let s = v.join(",");
    assert_eq(s, "");
}

// === min / max ===

#[test]
fn test_min() {
    let v = list(5, 3, 8, 1, 4);
    let m = v.min();
    assert(m.is_some());
}

#[test]
fn test_min_empty() {
    let v = list();
    let m = v.min();
    assert(m.is_none());
}

#[test]
fn test_max() {
    let v = list(5, 3, 8, 1, 4);
    let m = v.max();
    assert(m.is_some());
}

// === chunks ===

#[test]
fn test_chunks() {
    let v = list(1, 2, 3, 4, 5);
    let c = v.chunks(2);
    assert(c.len() > 1);
}

// === windows ===

#[test]
fn test_windows() {
    let v = list(1, 2, 3, 4);
    let w = v.windows(2);
    assert_eq(w.len(), 3);
}

// === clone ===

#[test]
fn test_clone() {
    let v = list(1, 2, 3);
    let v2 = v.clone();
    assert_eq(v2.len(), 3);
}

// === extend ===

#[test]
fn test_extend() {
    let mut v = list(1, 2);
    v.extend(list(3, 4));
    assert_eq(v.len(), 4);
}

// === Shared Mutation (Rc<RefCell<>>) ===

#[test]
fn test_shared_mutation() {
    let a = list(1, 2, 3);
    let b = a;
    b.push(4);
    // a and b share the same data
    assert_eq(a.len(), 4);
}

#[test]
fn test_clone_deep_copy() {
    let a = list(1, 2, 3);
    let b = a.clone();
    b.push(4);
    // clone creates independent copy
    assert_eq(a.len(), 3);
    assert_eq(b.len(), 4);
}
