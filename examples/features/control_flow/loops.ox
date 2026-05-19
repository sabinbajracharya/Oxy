// === Feature: Control Flow — While, Loop, Break, Continue ===
// `while` loops test a condition before each iteration. `loop` runs forever
// until `break`. Both support `break` (with optional value) and `continue`
// (skip to next iteration). Labeled break/continue targets outer loops.

// === While Loop ===

#[test]
fn test_while_basic() {
    let mut sum = 0;
    let mut i = 0;
    while i < 5 {
        sum = sum + i;
        i = i + 1;
    }
    assert_eq!(sum, 10);
    assert_eq!(i, 5);
}

#[test]
fn test_while_zero_iterations() {
    let mut count = 0;
    while false {
        count = count + 1;
    }
    assert_eq!(count, 0);
}

#[test]
fn test_while_single_iteration() {
    let mut count = 0;
    let mut done = false;
    while !done {
        count = count + 1;
        done = true;
    }
    assert_eq!(count, 1);
}

// === Break in While ===

#[test]
fn test_while_break() {
    let mut i = 0;
    while i < 100 {
        if i == 5 {
            break;
        }
        i = i + 1;
    }
    assert_eq!(i, 5);
}

// === Continue in While ===

#[test]
fn test_while_continue() {
    let mut sum = 0;
    let mut i = 0;
    while i < 5 {
        i = i + 1;
        if i == 3 {
            continue;
        }
        sum = sum + i;
    }
    assert_eq!(sum, 1 + 2 + 4 + 5);
}

// === Loop with Break ===

#[test]
fn test_loop_break() {
    let mut i = 0;
    loop {
        i = i + 1;
        if i >= 10 {
            break;
        }
    }
    assert_eq!(i, 10);
}

#[test]
fn test_loop_break_with_value() {
    let result = loop {
        break 42;
    };
    assert_eq!(result, 42);
}

// === Continue in Loop ===

#[test]
fn test_loop_continue() {
    let mut sum = 0;
    let mut i = 0;
    loop {
        i = i + 1;
        if i > 10 {
            break;
        }
        if i % 2 == 0 {
            continue;
        }
        sum = sum + i;
    }
    assert_eq!(sum, 1 + 3 + 5 + 7 + 9);
}

// === Nested Loops ===

#[test]
fn test_nested_loops() {
    let mut total = 0;
    let mut i = 0;
    while i < 3 {
        let mut j = 0;
        while j < 3 {
            total = total + 1;
            j = j + 1;
        }
        i = i + 1;
    }
    assert_eq!(total, 9);
}

// === Break from Nested Loop (innermost) ===

#[test]
fn test_break_innermost() {
    let mut outer = 0;
    let mut inner = 0;
    while outer < 5 {
        outer = outer + 1;
        inner = 0;
        while inner < 5 {
            inner = inner + 1;
            if inner == 2 {
                break;
            }
        }
    }
    assert_eq!(outer, 5);
    assert_eq!(inner, 2);
}

// === Labeled Break ===

#[test]
fn test_labeled_break() {
    let mut x = 0;
    'outer: while x < 10 {
        let mut y = 0;
        while y < 10 {
            y = y + 1;
            if y == 3 {
                break 'outer;
            }
        }
        x = x + 1;
    }
    assert_eq!(x, 0);
}

// === Labeled Continue ===

#[test]
fn test_labeled_continue() {
    let mut sum = 0;
    let mut i = 0;
    'outer: while i < 5 {
        i = i + 1;
        let mut j = 0;
        while j < 5 {
            j = j + 1;
            if j == i {
                continue 'outer;
            }
            sum = sum + j;
        }
    }
    // Only inner loops where j != i contribute
    assert!(sum > 0);
}

// === Break all nested loops ===

#[test]
fn test_break_outer_from_deep() {
    let mut count = 0;
    'a: loop {
        count = count + 1;
        'b: loop {
            count = count + 1;
            'c: loop {
                count = count + 1;
                break 'a;
            }
        }
    }
    assert_eq!(count, 3);
}
