// === Feature: if/else and match branch type agreement ===

#[test]
fn test_if_else_same_type_ok() {
    val n = 5;
    val s = if n > 0 { "positive".to_string() } else { "non-positive".to_string() };
    assert::eq(s, "positive");
}

#[test]
fn test_match_arms_same_type_ok() {
    val n = 1;
    val s = match n {
        1 => "one".to_string(),
        2 => "two".to_string(),
        _ => "other".to_string(),
    };
    assert::eq(s, "one");
}

#[test]
fn test_if_else_with_unit_arms_ok() {
    var total = 0;
    if true {
        total = 1;
    } else {
        total = 2;
    }
    assert::eq(total, 1);
}

#[test]
fn test_if_else_int_compatibility_ok() {
    // Int and Int are compatible at the binding level via Int promotion.
    val n: Int = 5;
    val v = if n > 0 { 10 } else { 20 };
    assert::eq(v, 10);
}

#[compile_error]
fn test_if_else_incompatible_types_rejected() {
    val _x = if true { 42 } else { "string".to_string() };
}

#[compile_error]
fn test_match_arms_incompatible_types_rejected() {
    val n = 1;
    val _x = match n {
        1 => 100,
        _ => "string".to_string(),
    };
}

#[compile_error]
fn test_match_arms_bool_vs_string_rejected() {
    val n = 0;
    val _x = match n {
        0 => true,
        _ => "no".to_string(),
    };
}
