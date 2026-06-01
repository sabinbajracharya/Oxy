// === Feature: Control Flow — For / In ===
// `for` loops iterate over ranges, strings, and collections. The iteration
// variable is rebound on each iteration. Supports break and continue.

// === For over Range ===

#[test]
fn test_for_range() {
    var sum = 0;
    for i in 0..5 {
        sum = sum + i;
    }
    assert::eq(sum, 10);
}

#[test]
fn test_for_range_empty() {
    var count = 0;
    for i in 0..0 {
        count = count + 1;
    }
    assert::eq(count, 0);
}

#[test]
fn test_for_range_single() {
    var sum = 0;
    for i in 5..6 {
        sum = sum + i;
    }
    assert::eq(sum, 5);
}

#[test]
fn test_for_range_large() {
    var sum = 0;
    for i in 0..100 {
        sum = sum + i;
    }
    assert::eq(sum, 4950);
}

// === For over String (iterates chars) ===

#[test]
fn test_for_string() {
    var chars = "";
    for c in "abc" {
        chars = chars + c;
    }
    // For-in appends each char — string concatenation
    assert::true(chars.len() > 0);
}

// === For over List ===

#[test]
fn test_for_list() {
    val items = "a,b,c".split(",");
    var count = 0;
    for item in items {
        count = count + 1;
    }
    assert::eq(count, 3);
}

// === For with Break ===

#[test]
fn test_for_break() {
    var sum = 0;
    for i in 0..10 {
        if i == 5 {
            break;
        }
        sum = sum + i;
    }
    assert::eq(sum, 10);
}

// === For with Continue ===

#[test]
fn test_for_continue() {
    var sum = 0;
    for i in 0..10 {
        if i % 2 == 0 {
            continue;
        }
        sum = sum + i;
    }
    assert::eq(sum, 1 + 3 + 5 + 7 + 9);
}

// === For with Labeled Break ===

#[test]
fn test_for_labeled_break() {
    var count = 0;
    'outer: for i in 0..5 {
        for j in 0..5 {
            count = count + 1;
            if j == 2 {
                break 'outer;
            }
        }
    }
    assert::eq(count, 3);
}

// === Nested For Loops ===

#[test]
fn test_nested_for() {
    var pairs = 0;
    for i in 0..3 {
        for j in 0..3 {
            pairs = pairs + 1;
        }
    }
    assert::eq(pairs, 9);
}

// === For with Reversed Range (empty) ===

#[test]
fn test_for_reversed_range() {
    var count = 0;
    for i in 5..0 {
        count = count + 1;
    }
    assert::eq(count, 0);
}
