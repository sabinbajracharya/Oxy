// === STRESS: no-struct-literal disambiguation in if/while/for headers ===
// `if score < MAX { ... }` must NOT treat `MAX { ... }` as a struct
// initializer — even when MAX is uppercase, like a typical constant.

const MAX_SIZE: Int = 100;
const THRESHOLD: Int = 50;

#[test]
fn test_const_in_if_condition() {
    val score = 95;
    var got = "above".to_string();
    if score < MAX_SIZE {
        got = "below".to_string();
    }
    assert::eq(got, "below");
}

#[test]
fn test_const_in_else_if_condition() {
    val n = 25;
    val label = if n > MAX_SIZE {
        "big".to_string()
    } else if n > THRESHOLD {
        "med".to_string()
    } else {
        "small".to_string()
    };
    assert::eq(label, "small");
}

#[test]
fn test_const_in_while_condition() {
    var i = 0;
    while i < MAX_SIZE {
        i = i + 1;
        if i > 5 { break; }
    }
    assert::eq(i, 6);
}

const LIMIT: Int = 5;

#[test]
fn test_uppercase_const_in_for_range() {
    var sum = 0;
    for i in 0..LIMIT {
        sum = sum + i;
    }
    assert::eq(sum, 10);
}

// Sanity: struct literals still work outside header positions.
struct Pt { x: Int, y: Int }
#[test]
fn test_struct_init_still_works_outside_headers() {
    val p = Pt { x: 1, y: 2 };
    assert::eq(p.x + p.y, 3);
}
