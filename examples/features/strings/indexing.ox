// === Feature: Strings — Indexing & Slicing ===
// Strings support bracket-index `s[i]` returning a Char, and range-slicing
// `s[a..b]` returning a String. Both use character-based (not Byte-based)
// positions, matching Rust's `.chars().nth()` semantics.

// === Single Character Index: s[i] ===

#[test]
fn test_index_first() {
    let s = "hello";
    assert_eq(s[0], 'h');
}

#[test]
fn test_index_middle() {
    let s = "hello";
    assert_eq(s[1], 'e');
    assert_eq(s[3], 'l');
}

#[test]
fn test_index_last() {
    let s = "hello";
    assert_eq(s[4], 'o');
}

#[test]
fn test_index_zero_len_one() {
    let s = "x";
    assert_eq(s[0], 'x');
}

// === Range Slice: s[a..b] ===

#[test]
fn test_slice_full_range() {
    let s = "hello";
    assert_eq(s[0..5], "hello");
}

#[test]
fn test_slice_partial_front() {
    let s = "hello";
    assert_eq(s[0..2], "he");
}

#[test]
fn test_slice_partial_middle() {
    let s = "hello";
    assert_eq(s[1..4], "ell");
}

#[test]
fn test_slice_partial_end() {
    let s = "hello";
    assert_eq(s[2..5], "llo");
}

#[test]
fn test_slice_empty_result() {
    let s = "hello";
    assert_eq(s[2..2], "");
    assert_eq(s[0..0], "");
    assert_eq(s[5..5], "");
}

#[test]
fn test_slice_single_char() {
    let s = "hello";
    assert_eq(s[0..1], "h");
    assert_eq(s[4..5], "o");
}

// === Slice From Start: s[..end] ===

#[test]
fn test_slice_from_start() {
    let s = "hello";
    assert_eq(s[..3], "hel");
    assert_eq(s[..1], "h");
    assert_eq(s[..5], "hello");
    assert_eq(s[..0], "");
}

// === Slice To End: s[start..] ===

#[test]
fn test_slice_to_end() {
    let s = "hello";
    assert_eq(s[2..], "llo");
    assert_eq(s[0..], "hello");
    assert_eq(s[4..], "o");
    assert_eq(s[5..], "");
}

// === Slice Full: s[..] ===

#[test]
fn test_slice_full_shorthand() {
    let s = "hello";
    assert_eq(s[..], "hello");
}

// === Indexing on Empty String ===

#[test]
fn test_slice_empty_string() {
    let s = "";
    assert_eq(s[..], "");
    assert_eq(s[0..0], "");
    assert_eq(s[..0], "");
    assert_eq(s[0..], "");
}

// === Unicode Indexing ===

#[test]
fn test_index_unicode() {
    let s = "héllo";
    assert_eq(s[0], 'h');
    assert_eq(s[1], 'é');
}

#[test]
fn test_slice_unicode() {
    let s = "héllo世界";
    let slice = s[1..6];
    assert_eq(slice.len(), 5);
    assert_eq(slice.chars()[0], 'é');
}
