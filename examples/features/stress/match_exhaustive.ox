// === STRESS: match patterns — every kind, every nesting, every guard ===
// Covers literal/wildcard/ident/enum-variant (unit, tuple, struct) /
// struct / tuple / or / slice / rest / range patterns, with and without
// guards, in nested positions, and as the tail expression of a fn.

enum Color { Red, Green, Blue }

enum Shape {
    Circle(float),
    Rect { w: float, h: float },
    Triangle(float, float, float),
    Nothing,
}

struct Point { x: int, y: int }

// --- 1. literal patterns ---
#[test]
fn test_match_int_literal() {
    let n = 3;
    let s = match n {
        0 => "zero",
        1 => "one",
        2 => "two",
        3 => "three",
        _ => "many",
    };
    assert_eq(s, "three");
}

#[test]
fn test_match_string_literal() {
    let s = "hello";
    let r = match s {
        "hi" => 1,
        "hello" => 2,
        "bye" => 3,
        _ => 0,
    };
    assert_eq(r, 2);
}

#[test]
fn test_match_bool_literal() {
    let b = true;
    let r = match b {
        true => 1,
        false => 0,
    };
    assert_eq(r, 1);
}

#[test]
fn test_match_char_literal() {
    let c = 'b';
    let r = match c {
        'a' => 1,
        'b' => 2,
        'c' => 3,
        _ => 0,
    };
    assert_eq(r, 2);
}

// --- 2. wildcard catches anything ---
#[test]
fn test_match_wildcard_only() {
    let n = 999;
    let r = match n {
        _ => 42,
    };
    assert_eq(r, 42);
}

// --- 3. ident pattern binds ---
#[test]
fn test_match_ident_binds() {
    let n = 7;
    let r = match n {
        x => x * 10,
    };
    assert_eq(r, 70);
}

// --- 4. enum unit variant ---
#[test]
fn test_match_enum_unit_variant() {
    let c = Color::Green;
    let s = match c {
        Color::Red => "r",
        Color::Green => "g",
        Color::Blue => "b",
    };
    assert_eq(s, "g");
}

// --- 5. enum tuple variant ---
#[test]
fn test_match_enum_tuple_variant() {
    let s = Shape::Triangle(3.0, 4.0, 5.0);
    let p = match s {
        Shape::Circle(r) => r,
        Shape::Triangle(a, b, c) => a + b + c,
        Shape::Rect { w, h } => w + h,
        Shape::Nothing => 0.0,
    };
    assert_eq(p, 12.0);
}

// --- 6. enum struct variant ---
#[test]
fn test_match_enum_struct_variant() {
    let s = Shape::Rect { w: 4.0, h: 6.0 };
    let area = match s {
        Shape::Circle(r) => 3.14 * r * r,
        Shape::Triangle(a, b, c) => a + b + c,
        Shape::Rect { w, h } => w * h,
        Shape::Nothing => 0.0,
    };
    assert_eq(area, 24.0);
}

// --- 7. struct pattern (top-level struct, not enum variant) ---
#[test]
fn test_match_struct_pattern() {
    let p = Point { x: 3, y: 4 };
    let dist = match p {
        Point { x, y } => x * x + y * y,
    };
    assert_eq(dist, 25);
}

// --- 8. tuple pattern ---
#[test]
fn test_match_tuple_pattern() {
    let t = (1, 2, 3);
    let sum = match t {
        (a, b, c) => a + b + c,
    };
    assert_eq(sum, 6);
}

// --- 9. or-pattern ---
#[test]
fn test_match_or_pattern() {
    let n = 5;
    let category = match n {
        1 | 3 | 5 | 7 | 9 => "odd",
        2 | 4 | 6 | 8 => "even",
        _ => "other",
    };
    assert_eq(category, "odd");
}

// --- 10. range pattern (exclusive) ---
#[test]
fn test_match_range_exclusive() {
    let n = 5;
    let band = match n {
        0..10 => "small",
        10..100 => "med",
        _ => "large",
    };
    assert_eq(band, "small");
}

// --- 11. range pattern (inclusive) ---
#[test]
fn test_match_range_inclusive() {
    let n = 10;
    let band = match n {
        0..=9 => "small",
        10..=99 => "med",
        _ => "large",
    };
    assert_eq(band, "med");
}

// --- 12. guarded arms ---
#[test]
fn test_match_with_guard() {
    let n = 8;
    let s = match n {
        x if x < 0 => "neg",
        0 => "zero",
        x if x < 10 => "small",
        x if x < 100 => "med",
        _ => "large",
    };
    assert_eq(s, "small");
}

// --- 13. guard with enum variant bindings ---
#[test]
fn test_match_guard_with_enum_bindings() {
    let s = Shape::Circle(10.0);
    let kind = match s {
        Shape::Circle(r) if r < 5.0 => "small circle",
        Shape::Circle(r) if r < 20.0 => "med circle",
        Shape::Circle(_) => "large circle",
        _ => "other",
    };
    assert_eq(kind, "med circle");
}

// --- 14. nested match ---
#[test]
fn test_match_nested() {
    let s = Shape::Rect { w: 5.0, h: 5.0 };
    let kind = match s {
        Shape::Rect { w, h } => match w == h {
            true => "square",
            false => "rect",
        },
        _ => "other",
    };
    assert_eq(kind, "square");
}

// --- 15. match as a statement (no value used) ---
#[test]
fn test_match_as_statement() {
    let mut counter = 0;
    let c = Color::Blue;
    match c {
        Color::Red => counter = 1,
        Color::Green => counter = 2,
        Color::Blue => counter = 3,
    }
    assert_eq(counter, 3);
}

// --- 16. match returning struct ---
#[test]
fn test_match_returning_struct() {
    let n = 1;
    let p = match n {
        0 => Point { x: 0, y: 0 },
        1 => Point { x: 1, y: 1 },
        _ => Point { x: 99, y: 99 },
    };
    assert_eq(p.x + p.y, 2);
}

// --- 17. match with println in each arm (stack discipline) ---
#[test]
fn test_match_with_side_effects_each_arm() {
    let mut total = 0;
    let shapes = [
        Shape::Circle(1.0),
        Shape::Rect { w: 2.0, h: 3.0 },
        Shape::Triangle(1.0, 1.0, 1.0),
        Shape::Nothing,
    ];
    for s in shapes {
        match s {
            Shape::Circle(_r) => { total = total + 1; }
            Shape::Rect { w: _, h: _ } => { total = total + 10; }
            Shape::Triangle(_a, _b, _c) => { total = total + 100; }
            Shape::Nothing => { total = total + 1000; }
        }
    }
    assert_eq(total, 1111);
}

// --- 18. match inside a fn returning value, called repeatedly ---
fn describe(s: Shape) -> String {
    match s {
        Shape::Circle(r) => format("circle r={}", r),
        Shape::Rect { w, h } => format("rect {}x{}", w, h),
        Shape::Triangle(_, _, _) => "tri".to_string(),
        Shape::Nothing => "nothing".to_string(),
    }
}

#[test]
fn test_match_fn_called_many_times() {
    assert_eq(describe(Shape::Circle(2.0)), "circle r=2.0");
    assert_eq(describe(Shape::Rect { w: 3.0, h: 4.0 }), "rect 3.0x4.0");
    assert_eq(describe(Shape::Triangle(1.0, 1.0, 1.0)), "tri");
    assert_eq(describe(Shape::Nothing), "nothing");
}

// --- 19. match in loop body ---
#[test]
fn test_match_in_loop() {
    let mut acc = 0;
    let mut i = 0;
    while i < 5 {
        acc = acc + match i {
            0 => 0,
            1 => 10,
            2 => 100,
            3 => 1000,
            _ => 10000,
        };
        i = i + 1;
    }
    assert_eq(acc, 11110);
}

// --- 20. Option match ---
#[test]
fn test_match_option() {
    let some_v: Option<int> = Some(42);
    let none_v: Option<int> = None;
    let a = match some_v {
        Some(x) => x,
        None => -1,
    };
    let b = match none_v {
        Some(x) => x,
        None => -1,
    };
    assert_eq(a, 42);
    assert_eq(b, -1);
}

// --- 21. Result match ---
#[test]
fn test_match_result() {
    let ok: Result<int, String> = Ok(7);
    let err: Result<int, String> = Err("nope".to_string());
    let a = match ok {
        Ok(x) => x,
        Err(_) => 0,
    };
    let b = match err {
        Ok(x) => x,
        Err(_) => 99,
    };
    assert_eq(a, 7);
    assert_eq(b, 99);
}
