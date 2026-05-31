// === Feature: Collections — List Operations ===
// Sorting, dedup, reverse, join, min, max, chunks, windows, clone,
// extend, shared mutation via Rc<RefCell<>>.

// === sort ===

#[test]
fn test_sort() {
    var v = [3, 1, 4, 1, 5];
    v.sort();
    assert_eq(v[0], 1);
    assert_eq(v[4], 5);
}

#[test]
fn test_sort_empty() {
    var v = [];
    v.sort();
    assert_eq(v.len(), 0);
}

#[test]
fn test_sort_single() {
    var v = [42];
    v.sort();
    assert_eq(v.len(), 1);
}

// === sort_by ===

#[test]
fn test_sort_by_descending() {
    var v = [1, 3, 2];
    v.sort_by(|a, b| b - a);
    assert_eq(v[0], 3);
    assert_eq(v[2], 1);
}

// === sort_by_key ===

#[test]
fn test_sort_by_key() {
    var v = ["bb", "a", "ccc"];
    v.sort_by_key(|s| s.len());
    assert_eq(v[0], "a");
}

// === reverse / rev ===

#[test]
fn test_reverse() {
    var v = [1, 2, 3];
    v.reverse();
    assert_eq(v[0], 3);
    assert_eq(v[2], 1);
}

// === dedup ===

#[test]
fn test_dedup() {
    var v = [1, 1, 2, 2, 3];
    v.dedup();
    assert(v.len() <= 3);
}

// === join ===

#[test]
fn test_join() {
    val v = ["a", "b", "c"];
    val s = v.join(", ");
    assert(s.contains(","));
}

#[test]
fn test_join_single() {
    val v = ["hello"];
    val s = v.join(", ");
    assert_eq(s, "hello");
}

#[test]
fn test_join_empty() {
    val v = [];
    val s = v.join(",");
    assert_eq(s, "");
}

// === min / max ===

#[test]
fn test_min() {
    val v = [5, 3, 8, 1, 4];
    val m = v.min();
    assert(m.is_some());
}

#[test]
fn test_min_empty() {
    val v = [];
    val m = v.min();
    assert(m.is_none());
}

#[test]
fn test_max() {
    val v = [5, 3, 8, 1, 4];
    val m = v.max();
    assert(m.is_some());
}

// === chunks ===

#[test]
fn test_chunks() {
    val v = [1, 2, 3, 4, 5];
    val c = v.chunks(2);
    assert(c.len() > 1);
}

// === windows ===

#[test]
fn test_windows() {
    val v = [1, 2, 3, 4];
    val w = v.windows(2);
    assert_eq(w.len(), 3);
}

// === clone ===

#[test]
fn test_clone() {
    val v = [1, 2, 3];
    val v2 = v.clone();
    assert_eq(v2.len(), 3);
}

// === extend ===

#[test]
fn test_extend() {
    var v = [1, 2];
    v.extend([3, 4]);
    assert_eq(v.len(), 4);
}

// === Shared Mutation (Rc<RefCell<>>) ===

#[test]
fn test_shared_mutation() {
    val a = [1, 2, 3];
    val b = a;
    b.push(4);
    // a and b share the same data
    assert_eq(a.len(), 4);
}

#[test]
fn test_clone_deep_copy() {
    val a = [1, 2, 3];
    val b = a.clone();
    b.push(4);
    // clone creates independent copy
    assert_eq(a.len(), 3);
    assert_eq(b.len(), 4);
}
