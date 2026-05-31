// === STRESS: no-struct-literal disambiguation in if/while/for headers ===
// `if score < MAX { ... }` must NOT treat `MAX { ... }` as a struct
// initializer — even when MAX is uppercase, like a typical constant.

const MAX_SIZE: Int = 100;
const THRESHOLD: Int = 50;

#[test]
fn test_const_in_if_condition() {
    let score = 95;
    let mut got = "above".to_string();
    if score < MAX_SIZE {
        got = "below".to_string();
    }
    assert_eq(got, "below");
}

#[test]
fn test_const_in_else_if_condition() {
    let n = 25;
    let label = if n > MAX_SIZE {
        "big".to_string()
    } else if n > THRESHOLD {
        "med".to_string()
    } else {
        "small".to_string()
    };
    assert_eq(label, "small");
}

#[test]
fn test_const_in_while_condition() {
    let mut i = 0;
    while i < MAX_SIZE {
        i = i + 1;
        if i > 5 { break; }
    }
    assert_eq(i, 6);
}

const LIMIT: Int = 5;

#[test]
fn test_uppercase_const_in_for_range() {
    let mut sum = 0;
    for i in 0..LIMIT {
        sum = sum + i;
    }
    assert_eq(sum, 10);
}

// Sanity: struct literals still work outside header positions.
struct Pt { x: Int, y: Int }
#[test]
fn test_struct_init_still_works_outside_headers() {
    let p = Pt { x: 1, y: 2 };
    assert_eq(p.x + p.y, 3);
}
