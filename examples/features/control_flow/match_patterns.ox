// === Feature: Control Flow — Match ===
// `match` is an expression that tests a value against patterns. Supports
// literal patterns, wildcards, variable bindings, guards (`if` clause),
// enum variant patterns (Option, Result), range patterns, and or-patterns.

// === Match on Integer Literals ===

#[test]
fn test_match_integer_literal() {
    let result = match 2 {
        1 => "one",
        2 => "two",
        3 => "three",
        _ => "other",
    };
    assert_eq!(result, "two");
}

#[test]
fn test_match_wildcard_fallback() {
    let result = match 99 {
        1 => "one",
        _ => "other",
    };
    assert_eq!(result, "other");
}

// === Match as Expression ===

#[test]
fn test_match_returns_value() {
    let n = 10;
    let category = match n {
        0 => "zero",
        1..=9 => "small",
        _ => "large",
    };
    assert_eq!(category, "large");
}

// === Match with Variable Binding ===

#[test]
fn test_match_binding() {
    let x = 42;
    let y = match x {
        v => v,
    };
    assert_eq!(y, 42);
}

// === Match on Strings ===

#[test]
fn test_match_string() {
    let result = match "hello" {
        "hi" => 1,
        "hello" => 2,
        _ => 3,
    };
    assert_eq!(result, 2);
}

// === Match on Booleans ===

#[test]
fn test_match_bool() {
    let result = match true {
        true => "yes",
        false => "no",
    };
    assert_eq!(result, "yes");
}

// === Match with Guard (if clause) ===

#[test]
fn test_match_guard() {
    let n = 15;
    let result = match n {
        x if x < 10 => "small",
        x if x < 20 => "medium",
        _ => "large",
    };
    assert_eq!(result, "medium");
}

#[test]
fn test_match_guard_false_falls_through() {
    let n = 5;
    let result = match n {
        x if x > 10 => "big",
        x if x < 10 => "small",
        _ => "other",
    };
    assert_eq!(result, "small");
}

// === Match on Enum Variants (Option) ===

#[test]
fn test_match_option_some() {
    let opt = Some(42);
    let result = match opt {
        Some(v) => v,
        None => 0,
    };
    assert_eq!(result, 42);
}

#[test]
fn test_match_option_none() {
    let opt = None;
    let result = match opt {
        Some(v) => v,
        None => -1,
    };
    assert_eq!(result, -1);
}

// === Match on Enum Variants (Result) ===

#[test]
fn test_match_result_ok() {
    let r: Result = Ok(100);
    let result = match r {
        Ok(v) => v,
        Err(_) => -1,
    };
    assert_eq!(result, 100);
}

#[test]
fn test_match_result_err() {
    let r: Result = Err("oops");
    let result = match r {
        Ok(v) => v.to_string(),
        Err(e) => e,
    };
    assert_eq!(result, "oops");
}

// === Match with Range Patterns ===

#[test]
fn test_match_range() {
    let n = 5;
    let result = match n {
        1..=3 => "low",
        4..=6 => "mid",
        _ => "high",
    };
    assert_eq!(result, "mid");
}

#[test]
fn test_match_range_exclusive() {
    let n = 3;
    let result = match n {
        1..3 => "low",
        3..5 => "mid",
        _ => "other",
    };
    assert_eq!(result, "mid");
}

// === Match with Multiple Patterns (same arm via multiple match arms) ===

#[test]
fn test_match_multiple_patterns() {
    let c = 'x';
    let result = match c {
        'a' => "vowel-a",
        'e' => "vowel-e",
        _ => "consonant",
    };
    assert_eq!(result, "consonant");
}

// === Or Patterns ===

#[test]
fn test_or_pattern_vowels() {
    let c = 'e';
    let kind = match c {
        'a' | 'e' | 'i' | 'o' | 'u' => "vowel",
        _ => "consonant",
    };
    assert_eq!(kind, "vowel");
}

#[test]
fn test_or_pattern_consonant_falls_through() {
    let c = 'b';
    let kind = match c {
        'a' | 'e' | 'i' | 'o' | 'u' => "vowel",
        _ => "consonant",
    };
    assert_eq!(kind, "consonant");
}

#[test]
fn test_or_pattern_int_literals() {
    let mut hits = 0;
    for n in [1, 2, 3, 4] {
        match n {
            1 | 2 => hits = hits + 10,
            _ => hits = hits + 1,
        }
    }
    // 10 + 10 + 1 + 1 = 22
    assert_eq!(hits, 22);
}

#[test]
fn test_or_pattern_combined_with_range_and_guard_in_for_loop() {
    // Regression: combining OR + Range + guard inside a for loop used to
    // corrupt the iterator slot (Range pattern's slot-0 scratch hack).
    let mut buf = "".to_string();
    for n in [0, 2, 5, 42, -3] {
        let label = match n {
            0 => "zero",
            1 | 2 => "or",
            3..=9 => "range",
            x if x > 0 => "guard",
            _ => "neg",
        };
        buf = f"{buf}{label};";
    }
    assert_eq!(buf, "zero;or;range;guard;neg;");
}

// === Match with Option Enum Variant ===

#[test]
fn test_match_option_enum() {
    let opt = Some(42);
    let result = match opt {
        Some(v) => v,
        None => -1,
    };
    assert_eq!(result, 42);
}

// === Match with underscore in enum ===

#[test]
fn test_match_some_wildcard() {
    let opt = Some("hello");
    let result = match opt {
        Some(_) => 1,
        None => 0,
    };
    assert_eq!(result, 1);
}
