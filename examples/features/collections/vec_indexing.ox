// === Feature: Collections — Vec Indexing & Slicing ===
// Vec supports bracket-index `v[i]` (returns element) and range-slicing
// `v[a..b]` (returns new Vec). Index assignment `v[i] = x` mutates in place.

// === Bracket Index Access ===

#[test]
fn test_index_first() {
    let v = vec![10, 20, 30];
    assert_eq!(v[0], 10);
}

#[test]
fn test_index_middle() {
    let v = vec![10, 20, 30];
    assert_eq!(v[1], 20);
}

#[test]
fn test_index_last() {
    let v = vec![10, 20, 30];
    assert_eq!(v[2], 30);
}

// === Index Assignment ===

#[test]
fn test_index_assign() {
    let mut v = vec![10, 20, 30];
    v[0] = 100;
    assert_eq!(v[0], 100);
}

#[test]
fn test_index_assign_multiple() {
    let mut v = vec![1, 2, 3];
    v[0] = 10;
    v[1] = 20;
    v[2] = 30;
    assert_eq!(v[0], 10);
    assert_eq!(v[1], 20);
    assert_eq!(v[2], 30);
}

// === Range Slicing ===

#[test]
fn test_slice_full() {
    let v = vec![10, 20, 30];
    let s = v[0..3];
    assert_eq!(s.len(), 3);
}

#[test]
fn test_slice_partial() {
    let v = vec![10, 20, 30, 40];
    let s = v[1..3];
    assert_eq!(s.len(), 2);
}

#[test]
fn test_slice_empty() {
    let v = vec![10, 20, 30];
    let s = v[1..1];
    assert_eq!(s.len(), 0);
}

#[test]
fn test_slice_from_start() {
    let v = vec![10, 20, 30];
    let s = v[..2];
    assert_eq!(s.len(), 2);
}
