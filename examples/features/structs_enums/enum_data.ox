// === Feature: Enums — Data Variants ===
// Enum variants can carry data. Pattern matching extracts the data.
// Supports nested matching, if-let, and option-like enum patterns.

// === Option-like Enum ===

enum MyOption {
    MySome(i64),
    MyNone,
}

#[test]
fn test_custom_option_some() {
    let x = MyOption::MySome(42);
    let result = match x {
        MyOption::MySome(v) => v,
        MyOption::MyNone => 0,
    };
    assert_eq!(result, 42);
}

#[test]
fn test_custom_option_none() {
    let x = MyOption::MyNone;
    let result = match x {
        MyOption::MySome(v) => v,
        MyOption::MyNone => -1,
    };
    assert_eq!(result, -1);
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
    assert_eq!(msg, "success");
}

#[test]
fn test_custom_result_err() {
    let r = MyResult::MyErr("failed");
    let msg = match r {
        MyResult::MyOk(m) => m,
        MyResult::MyErr(e) => e,
    };
    assert_eq!(msg, "failed");
}

// === If-Let Pattern ===

#[test]
fn test_if_let_some() {
    let x = MyOption::MySome(10);
    let mut found = 0;
    if let MyOption::MySome(v) = x {
        found = v;
    }
    assert_eq!(found, 10);
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
    assert_eq!(result, -1);
}

// === Enum with Mixed Data ===

enum Event {
    KeyPress(char),
    Click(i64, i64),
    Resize { w: i64, h: i64 },
}

#[test]
fn test_enum_mixed_variants() {
    let e = Event::Click(100, 200);
    let desc = match e {
        Event::KeyPress(c) => "key",
        Event::Click(x, y) => "click",
        Event::Resize { w, h } => "resize",
    };
    assert_eq!(desc, "click");
}
