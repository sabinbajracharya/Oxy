// === Feature: Collections — List Indexing & Slicing ===
// List supports bracket-index `v[i]` (returns element) and range-slicing
// `v[a..b]` (returns new List). Index assignment `v[i] = x` mutates in place.

// === Bracket Index Access ===

#[test]
fn test_index_first() {
    val v = [10, 20, 30];
    assert::eq(v[0], 10);
}

#[test]
fn test_index_middle() {
    val v = [10, 20, 30];
    assert::eq(v[1], 20);
}

#[test]
fn test_index_last() {
    val v = [10, 20, 30];
    assert::eq(v[2], 30);
}

// === Index Assignment ===

#[test]
fn test_index_assign() {
    var v = [10, 20, 30];
    v[0] = 100;
    assert::eq(v[0], 100);
}

#[test]
fn test_index_assign_multiple() {
    var v = [1, 2, 3];
    v[0] = 10;
    v[1] = 20;
    v[2] = 30;
    assert::eq(v[0], 10);
    assert::eq(v[1], 20);
    assert::eq(v[2], 30);
}

// === Range Slicing ===

#[test]
fn test_slice_full() {
    val v = [10, 20, 30];
    val s = v[0..3];
    assert::eq(s.len(), 3);
}

#[test]
fn test_slice_partial() {
    val v = [10, 20, 30, 40];
    val s = v[1..3];
    assert::eq(s.len(), 2);
}

#[test]
fn test_slice_empty() {
    val v = [10, 20, 30];
    val s = v[1..1];
    assert::eq(s.len(), 0);
}

#[test]
fn test_slice_from_start() {
    val v = [10, 20, 30];
    val s = v[..2];
    assert::eq(s.len(), 2);
}
