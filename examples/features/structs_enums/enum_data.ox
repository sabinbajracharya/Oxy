// === Feature: Enums — Data Variants ===
// Enum variants can carry data. Pattern matching extracts the data.
// Supports nested matching, if-val, and option-like enum patterns.

// === Option-like Enum ===

enum MyOption {
    MySome(Int),
    MyNone,
}

#[test]
fn test_custom_option_some() {
    val x = MyOption::MySome(42);
    val result = match x {
        MyOption::MySome(v) => v,
        MyOption::MyNone => 0,
    };
    assert::eq(result, 42);
}

#[test]
fn test_custom_option_none() {
    val x = MyOption::MyNone;
    val result = match x {
        MyOption::MySome(v) => v,
        MyOption::MyNone => -1,
    };
    assert::eq(result, -1);
}

// === Result-like Enum ===

enum MyResult {
    MyOk(String),
    MyErr(String),
}

#[test]
fn test_custom_result_ok() {
    val r = MyResult::MyOk("success");
    val msg = match r {
        MyResult::MyOk(m) => m,
        MyResult::MyErr(e) => e,
    };
    assert::eq(msg, "success");
}

#[test]
fn test_custom_result_err() {
    val r = MyResult::MyErr("failed");
    val msg = match r {
        MyResult::MyOk(m) => m,
        MyResult::MyErr(e) => e,
    };
    assert::eq(msg, "failed");
}

// === If-Let Pattern ===

#[test]
fn test_if_let_some() {
    val x = MyOption::MySome(10);
    var found = 0;
    if val MyOption::MySome(v) = x {
        found = v;
    }
    assert::eq(found, 10);
}

#[test]
fn test_if_let_none_else() {
    val x = MyOption::MyNone;
    var result = 0;
    if val MyOption::MySome(v) = x {
        result = v;
    } else {
        result = -1;
    }
    assert::eq(result, -1);
}

// === Enum with Mixed Data ===

enum Event {
    KeyPress(char),
    Click(Int, Int),
    Resize { w: Int, h: Int },
}

#[test]
fn test_enum_mixed_variants() {
    val e = Event::Click(100, 200);
    val desc = match e {
        Event::KeyPress(c) => "key",
        Event::Click(x, y) => "click",
        Event::Resize { w, h } => "resize",
    };
    assert::eq(desc, "click");
}

// === Tuple Variants with Many Positional Fields ===
// Regression: 3+ positional fields used to bind the third (and beyond)
// to Unit because EnumVariantEqual bulk-pushed data into stack slots
// that collided with binding positions.

enum Color {
    Rgb(Int, Int, Int),
    Rgba(Int, Int, Int, Int),
    Hsl(Int, Int, Int),
}

#[test]
fn test_three_positional_fields_bind_correctly() {
    val c = Color::Rgb(255, 128, 64);
    match c {
        Color::Rgb(r, g, b) => {
            assert::eq(r, 255);
            assert::eq(g, 128);
            assert::eq(b, 64);
        }
        _ => assert::true(false),
    }
}

#[test]
fn test_four_positional_fields_bind_correctly() {
    val c = Color::Rgba(10, 20, 30, 40);
    match c {
        Color::Rgba(r, g, b, a) => {
            assert::eq(r, 10);
            assert::eq(g, 20);
            assert::eq(b, 30);
            assert::eq(a, 40);
        }
        _ => assert::true(false),
    }
}

#[test]
fn test_if_let_three_positional_fields() {
    val c = Color::Hsl(120, 50, 75);
    var total = 0;
    if val Color::Hsl(h, s, l) = c {
        total = h + s + l;
    }
    assert::eq(total, 245);
}
