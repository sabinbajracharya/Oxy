// === Feature: Control Flow — Match ===
// `match` is an expression that tests a value against patterns. Supports
// literal patterns, wildcards, variable bindings, guards (`if` clause),
// enum variant patterns (Option, Result), range patterns, and or-patterns.

// === Match on Integer Literals ===

#[test]
fn test_match_integer_literal() {
    val result = match 2 {
        1 => "one",
        2 => "two",
        3 => "three",
        _ => "other",
    };
    assert::eq(result, "two");
}

#[test]
fn test_match_wildcard_fallback() {
    val result = match 99 {
        1 => "one",
        _ => "other",
    };
    assert::eq(result, "other");
}

// === Match as Expression ===

#[test]
fn test_match_returns_value() {
    val n = 10;
    val category = match n {
        0 => "zero",
        1..=9 => "small",
        _ => "large",
    };
    assert::eq(category, "large");
}

// === Match with Variable Binding ===

#[test]
fn test_match_binding() {
    val x = 42;
    val y = match x {
        v => v,
    };
    assert::eq(y, 42);
}

// === Match on Strings ===

#[test]
fn test_match_string() {
    val result = match "hello" {
        "hi" => 1,
        "hello" => 2,
        _ => 3,
    };
    assert::eq(result, 2);
}

// === Match on Booleans ===

#[test]
fn test_match_bool() {
    val result = match true {
        true => "yes",
        false => "no",
    };
    assert::eq(result, "yes");
}

// === Match with Guard (if clause) ===

#[test]
fn test_match_guard() {
    val n = 15;
    val result = match n {
        x if x < 10 => "small",
        x if x < 20 => "medium",
        _ => "large",
    };
    assert::eq(result, "medium");
}

#[test]
fn test_match_guard_false_falls_through() {
    val n = 5;
    val result = match n {
        x if x > 10 => "big",
        x if x < 10 => "small",
        _ => "other",
    };
    assert::eq(result, "small");
}

// === Match on Enum Variants (Option) ===

#[test]
fn test_match_option_some() {
    val opt = Some(42);
    val result = match opt {
        Some(v) => v,
        None => 0,
    };
    assert::eq(result, 42);
}

#[test]
fn test_match_option_none() {
    val opt = None;
    val result = match opt {
        Some(v) => v,
        None => -1,
    };
    assert::eq(result, -1);
}

// === Match on Enum Variants (Result) ===

#[test]
fn test_match_result_ok() {
    val r: Result = Ok(100);
    val result = match r {
        Ok(v) => v,
        Err(_) => -1,
    };
    assert::eq(result, 100);
}

#[test]
fn test_match_result_err() {
    val r: Result = Err("oops");
    val result = match r {
        Ok(v) => v.to_string(),
        Err(e) => e,
    };
    assert::eq(result, "oops");
}

// === Match with Range Patterns ===

#[test]
fn test_match_range() {
    val n = 5;
    val result = match n {
        1..=3 => "low",
        4..=6 => "mid",
        _ => "high",
    };
    assert::eq(result, "mid");
}

#[test]
fn test_match_range_exclusive() {
    val n = 3;
    val result = match n {
        1..3 => "low",
        3..5 => "mid",
        _ => "other",
    };
    assert::eq(result, "mid");
}

// === Match with Multiple Patterns (same arm via multiple match arms) ===

#[test]
fn test_match_multiple_patterns() {
    val c = 'x';
    val result = match c {
        'a' => "vowel-a",
        'e' => "vowel-e",
        _ => "consonant",
    };
    assert::eq(result, "consonant");
}

// === Or Patterns ===

#[test]
fn test_or_pattern_vowels() {
    val c = 'e';
    val kind = match c {
        'a' | 'e' | 'i' | 'o' | 'u' => "vowel",
        _ => "consonant",
    };
    assert::eq(kind, "vowel");
}

#[test]
fn test_or_pattern_consonant_falls_through() {
    val c = 'b';
    val kind = match c {
        'a' | 'e' | 'i' | 'o' | 'u' => "vowel",
        _ => "consonant",
    };
    assert::eq(kind, "consonant");
}

#[test]
fn test_or_pattern_int_literals() {
    var hits = 0;
    for n in [1, 2, 3, 4] {
        match n {
            1 | 2 => hits = hits + 10,
            _ => hits = hits + 1,
        }
    }
    // 10 + 10 + 1 + 1 = 22
    assert::eq(hits, 22);
}

#[test]
fn test_or_pattern_combined_with_range_and_guard_in_for_loop() {
    // Regression: combining OR + Range + guard inside a for loop used to
    // corrupt the iterator slot (Range pattern's slot-0 scratch hack).
    var buf = "".to_string();
    for n in [0, 2, 5, 42, -3] {
        val label = match n {
            0 => "zero",
            1 | 2 => "or",
            3..=9 => "range",
            x if x > 0 => "guard",
            _ => "neg",
        };
        buf = f"{buf}{label};";
    }
    assert::eq(buf, "zero;or;range;guard;neg;");
}

// === Match with Option Enum Variant ===

#[test]
fn test_match_option_enum() {
    val opt = Some(42);
    val result = match opt {
        Some(v) => v,
        None => -1,
    };
    assert::eq(result, 42);
}

// === Match with underscore in enum ===

#[test]
fn test_match_some_wildcard() {
    val opt = Some("hello");
    val result = match opt {
        Some(_) => 1,
        None => 0,
    };
    assert::eq(result, 1);
}

// === Tuple patterns in match arms ===

#[test]
fn test_match_tuple_all_bindings() {
    val pair = (3, 4);
    val label = match pair {
        (a, b) => f"{a}+{b}",
    };
    assert::eq(label, "3+4");
}

#[test]
fn test_match_tuple_literal_then_binding() {
    val pair = (1, 99);
    val label = match pair {
        (1, y) => f"one-{y}",
        (x, y) => f"{x}-{y}",
    };
    assert::eq(label, "one-99");
}

#[test]
fn test_match_tuple_binding_then_literal() {
    val pair = (5, 0);
    val label = match pair {
        (x, 0) => f"{x}-zero",
        (x, y) => f"{x}-{y}",
    };
    assert::eq(label, "5-zero");
}

#[test]
fn test_match_tuple_all_literals() {
    val pair = (2, 3);
    val label = match pair {
        (1, 1) => "ones",
        (2, 3) => "two-three",
        (_, _) => "other",
    };
    assert::eq(label, "two-three");
}

#[test]
fn test_match_tuple_with_wildcards() {
    val pair = (7, 8);
    val label = match pair {
        (_, 0) => "second-zero",
        (_, y) => f"second-{y}",
    };
    assert::eq(label, "second-8");
}

#[test]
fn test_match_three_tuple() {
    val triple = (1, 2, 3);
    val label = match triple {
        (1, b, c) => f"one-{b}-{c}",
        (a, b, c) => f"{a}-{b}-{c}",
    };
    assert::eq(label, "one-2-3");
}

#[test]
fn test_match_tuple_inside_for_loop() {
    // Regression: pattern-binding slots cannot collide with iterator slots.
    val pairs = [(1, 2), (3, 4), (5, 6)];
    var buf = "".to_string();
    for (a, b) in pairs {
        val label = match (a, b) {
            (1, y) => f"one-{y}",
            (x, 4) => f"{x}-four",
            (x, y) => f"{x}+{y}",
        };
        buf = f"{buf}{label};";
    }
    assert::eq(buf, "one-2;3-four;5+6;");
}

#[test]
fn test_if_let_tuple_pattern() {
    val pair = (10, 20);
    if val (x, y) = pair {
        assert::eq(x, 10);
        assert::eq(y, 20);
    }
}
