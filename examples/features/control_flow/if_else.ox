// === Feature: Control Flow — If / Else ===
// `if` is an expression in Oxy (returns a value). Supports `else if` chains,
// nested conditions, and all comparison operators in conditions.

// === Basic If ===

#[test]
fn test_if_true_branch() {
    var x = 0;
    if true {
        x = 1;
    }
    assert_eq(x, 1);
}

#[test]
fn test_if_false_skips() {
    var x = 0;
    if false {
        x = 1;
    }
    assert_eq(x, 0);
}

// === If / Else ===

#[test]
fn test_if_else_true() {
    var x = 0;
    if true {
        x = 1;
    } else {
        x = 2;
    }
    assert_eq(x, 1);
}

#[test]
fn test_if_else_false() {
    var x = 0;
    if false {
        x = 1;
    } else {
        x = 2;
    }
    assert_eq(x, 2);
}

// === If / Else If / Else ===

#[test]
fn test_else_if_chain() {
    var result = 0;
    val n = 20;
    if n < 10 {
        result = 1;
    } else if n < 30 {
        result = 2;
    } else {
        result = 3;
    }
    assert_eq(result, 2);
}

#[test]
fn test_else_if_all_false() {
    var result = 0;
    val n = 100;
    if n < 10 {
        result = 1;
    } else if n < 30 {
        result = 2;
    } else {
        result = 3;
    }
    assert_eq(result, 3);
}

// === If as Expression ===

#[test]
fn test_if_expression() {
    val x = if true { 10 } else { 20 };
    assert_eq(x, 10);

    val y = if false { 10 } else { 20 };
    assert_eq(y, 20);
}

#[test]
fn test_if_expression_no_else() {
    val x = if true { 42 };
    // No else: returns unit-like value when false
    assert_eq(x, 42);
}

// === Nested If ===

#[test]
fn test_nested_if() {
    var x = 0;
    if true {
        if true {
            x = 42;
        }
    }
    assert_eq(x, 42);
}

#[test]
fn test_nested_if_else() {
    var x = 0;
    if true {
        if false {
            x = 1;
        } else {
            x = 2;
        }
    }
    assert_eq(x, 2);
}

// === Conditions with Comparisons ===

#[test]
fn test_if_with_less_than() {
    var x = 0;
    if 5 < 10 {
        x = 1;
    }
    assert_eq(x, 1);
}

#[test]
fn test_if_with_equality() {
    var x = 0;
    if 42 == 42 {
        x = 1;
    }
    assert_eq(x, 1);
}

#[test]
fn test_if_with_not() {
    var x = 0;
    if !false {
        x = 1;
    }
    assert_eq(x, 1);
}

#[test]
fn test_if_with_compound_condition() {
    var x = 0;
    if 5 < 10 && 20 > 15 {
        x = 1;
    }
    assert_eq(x, 1);
}

// === If with Block Statements ===

#[test]
fn test_if_multiple_stmts() {
    var a = 0;
    var b = 0;
    if true {
        a = 1;
        b = 2;
    }
    assert_eq(a, 1);
    assert_eq(b, 2);
}

// === Dangling Else (binds to nearest if) ===

#[test]
fn test_dangling_else() {
    var x = 0;
    if true {
        if false {
            x = 1;
        } else {
            x = 2;
        }
    }
    assert_eq(x, 2);
}
