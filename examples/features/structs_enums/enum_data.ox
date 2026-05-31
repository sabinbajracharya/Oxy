// === Feature: Enums — Data Variants ===
// Enum variants can carry data. Pattern matching extracts the data.
// Supports nested matching, if-let, and option-like enum patterns.

// === Option-like Enum ===

enum MyOption {
    MySome(int),
    MyNone,
}

#[test]
fn test_custom_option_some() {
    let x = MyOption::MySome(42);
    let result = match x {
        MyOption::MySome(v) => v,
        MyOption::MyNone => 0,
    };
    assert_eq(result, 42);
}

#[test]
fn test_custom_option_none() {
    let x = MyOption::MyNone;
    let result = match x {
        MyOption::MySome(v) => v,
        MyOption::MyNone => -1,
    };
    assert_eq(result, -1);
}

// === Result-like Enum ===

enum MyResult {
    MyOk(String),
    MyErr(String),
}

#[test]
fn test_custom_result_ok() {
    let r = MyResult::MyOk("success");
    let msg = match r {
        MyResult::MyOk(m) => m,
        MyResult::MyErr(e) => e,
    };
    assert_eq(msg, "success");
}

#[test]
fn test_custom_result_err() {
    let r = MyResult::MyErr("failed");
    let msg = match r {
        MyResult::MyOk(m) => m,
        MyResult::MyErr(e) => e,
    };
    assert_eq(msg, "failed");
}

// === If-Let Pattern ===

#[test]
fn test_if_let_some() {
    let x = MyOption::MySome(10);
    let mut found = 0;
    if let MyOption::MySome(v) = x {
        found = v;
    }
    assert_eq(found, 10);
}

#[test]
fn test_if_let_none_else() {
    let x = MyOption::MyNone;
    let mut result = 0;
    if let MyOption::MySome(v) = x {
        result = v;
    } else {
        result = -1;
    }
    assert_eq(result, -1);
}

// === Enum with Mixed Data ===

enum Event {
    KeyPress(char),
    Click(int, int),
    Resize { w: int, h: int },
}

#[test]
fn test_enum_mixed_variants() {
    let e = Event::Click(100, 200);
    let desc = match e {
        Event::KeyPress(c) => "key",
        Event::Click(x, y) => "click",
        Event::Resize { w, h } => "resize",
    };
    assert_eq(desc, "click");
}

// === Tuple Variants with Many Positional Fields ===
// Regression: 3+ positional fields used to bind the third (and beyond)
// to Unit because EnumVariantEqual bulk-pushed data into stack slots
// that collided with binding positions.

enum Color {
    Rgb(int, int, int),
    Rgba(int, int, int, int),
    Hsl(int, int, int),
}

#[test]
fn test_three_positional_fields_bind_correctly() {
    let c = Color::Rgb(255, 128, 64);
    match c {
        Color::Rgb(r, g, b) => {
            assert_eq(r, 255);
            assert_eq(g, 128);
            assert_eq(b, 64);
        }
        _ => assert(false),
    }
}

#[test]
fn test_four_positional_fields_bind_correctly() {
    let c = Color::Rgba(10, 20, 30, 40);
    match c {
        Color::Rgba(r, g, b, a) => {
            assert_eq(r, 10);
            assert_eq(g, 20);
            assert_eq(b, 30);
            assert_eq(a, 40);
        }
        _ => assert(false),
    }
}

#[test]
fn test_if_let_three_positional_fields() {
    let c = Color::Hsl(120, 50, 75);
    let mut total = 0;
    if let Color::Hsl(h, s, l) = c {
        total = h + s + l;
    }
    assert_eq(total, 245);
}
