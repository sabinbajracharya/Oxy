// === Feature: if/else and match branch type agreement ===

#[test]
fn test_if_else_same_type_ok() {
    let n = 5;
    let s = if n > 0 { "positive".to_string() } else { "non-positive".to_string() };
    assert_eq!(s, "positive");
}

#[test]
fn test_match_arms_same_type_ok() {
    let n = 1;
    let s = match n {
        1 => "one".to_string(),
        2 => "two".to_string(),
        _ => "other".to_string(),
    };
    assert_eq!(s, "one");
}

#[test]
fn test_if_else_with_unit_arms_ok() {
    let mut total = 0;
    if true {
        total = 1;
    } else {
        total = 2;
    }
    assert_eq!(total, 1);
}

#[test]
fn test_if_else_int_compatibility_ok() {
    // i64 and i32 are compatible at the binding level via int promotion.
    let n: i32 = 5;
    let v = if n > 0 { 10 } else { 20 };
    assert_eq!(v, 10);
}

#[compile_error]
fn test_if_else_incompatible_types_rejected() {
    let _x = if true { 42 } else { "string".to_string() };
}

#[compile_error]
fn test_match_arms_incompatible_types_rejected() {
    let n = 1;
    let _x = match n {
        1 => 100,
        _ => "string".to_string(),
    };
}

#[compile_error]
fn test_match_arms_bool_vs_string_rejected() {
    let n = 0;
    let _x = match n {
        0 => true,
        _ => "no".to_string(),
    };
}
