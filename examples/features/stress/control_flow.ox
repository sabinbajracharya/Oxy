// === STRESS: control flow — if / while / for / loop / break / continue ===

// --- if/else as expression ---
#[test]
fn test_if_else_expr() {
    val n = if true { 1 } else { 2 };
    assert::eq(n, 1);
}
#[test]
fn test_if_else_expr_false() {
    val n = if false { 1 } else { 2 };
    assert::eq(n, 2);
}
#[test]
fn test_if_no_else_unit() {
    var x = 0;
    if true { x = 5; }
    assert::eq(x, 5);
}
#[test]
fn test_if_else_if_chain() {
    val n = 5;
    val s = if n < 0 { "neg" }
        else if n == 0 { "zero" }
        else if n < 10 { "small" }
        else { "big" };
    assert::eq(s, "small");
}
#[test]
fn test_nested_if_in_branch() {
    val r = if true {
        if false { 1 } else { 2 }
    } else {
        if true { 3 } else { 4 }
    };
    assert::eq(r, 2);
}

// --- while loops ---
#[test]
fn test_while_counter() {
    var i = 0;
    var sum = 0;
    while i < 5 {
        sum = sum + i;
        i = i + 1;
    }
    assert::eq(sum, 10);
}
#[test]
fn test_while_break() {
    var i = 0;
    while i < 100 {
        if i == 5 { break; }
        i = i + 1;
    }
    assert::eq(i, 5);
}
#[test]
fn test_while_continue() {
    var acc = 0;
    var i = 0;
    while i < 10 {
        i = i + 1;
        if i % 2 == 0 { continue; }
        acc = acc + i;
    }
    assert::eq(acc, 25);  // 1+3+5+7+9
}
#[test]
fn test_while_false_runs_zero_times() {
    var x = 0;
    while false { x = 99; }
    assert::eq(x, 0);
}

// --- for-in ---
#[test]
fn test_for_in_range() {
    var sum = 0;
    for i in 0..5 {
        sum = sum + i;
    }
    assert::eq(sum, 10);
}
#[test]
fn test_for_in_inclusive_range() {
    var sum = 0;
    for i in 1..=4 {
        sum = sum + i;
    }
    assert::eq(sum, 10);
}
#[test]
fn test_for_in_list() {
    val v = [10, 20, 30, 40];
    var sum = 0;
    for x in v {
        sum = sum + x;
    }
    assert::eq(sum, 100);
}
#[test]
fn test_for_in_array_literal() {
    var sum = 0;
    for x in [1, 2, 3] {
        sum = sum + x;
    }
    assert::eq(sum, 6);
}
#[test]
fn test_for_in_string_chars() {
    var count = 0;
    for _c in "hello".chars() {
        count = count + 1;
    }
    assert::eq(count, 5);
}
#[test]
fn test_for_in_break() {
    var hit = 0;
    for i in 0..10 {
        if i == 3 { break; }
        hit = hit + 1;
    }
    assert::eq(hit, 3);
}
#[test]
fn test_for_in_continue() {
    var count = 0;
    for i in 0..10 {
        if i % 3 == 0 { continue; }
        count = count + 1;
    }
    assert::eq(count, 6);  // 1,2,4,5,7,8
}

// --- loop with break-value ---
#[test]
fn test_loop_break_value() {
    var i = 0;
    val r = loop {
        if i == 7 { break i * 10; }
        i = i + 1;
    };
    assert::eq(r, 70);
}
#[test]
fn test_loop_no_value_break() {
    var i = 0;
    loop {
        if i == 3 { break; }
        i = i + 1;
    }
    assert::eq(i, 3);
}

// --- nested loops ---
#[test]
fn test_nested_loops_double_count() {
    var total = 0;
    for _i in 0..3 {
        for _j in 0..4 {
            total = total + 1;
        }
    }
    assert::eq(total, 12);
}
#[test]
fn test_break_only_breaks_inner() {
    var total = 0;
    for _i in 0..3 {
        for j in 0..10 {
            if j == 2 { break; }
            total = total + 1;
        }
    }
    assert::eq(total, 6);  // 3 outer × 2 inner
}

// --- labeled break ---
#[test]
fn test_labeled_break_outer() {
    var total = 0;
    'outer: for i in 0..5 {
        for j in 0..5 {
            if i == 2 && j == 2 { break 'outer; }
            total = total + 1;
        }
    }
    assert::eq(total, 12);  // 5 + 5 + 2
}
#[test]
fn test_labeled_continue_outer() {
    var total = 0;
    'outer: for i in 0..3 {
        for j in 0..3 {
            if j == 1 { continue 'outer; }
            total = total + i + j;
        }
    }
    assert::eq(total, 3);  // i=0,j=0 → 0; i=1,j=0 → 1; i=2,j=0 → 2
}

// --- if-val ---
#[test]
fn test_if_let_some() {
    val x: Option<Int> = Some(7);
    var got = 0;
    if val Some(v) = x {
        got = v;
    }
    assert::eq(got, 7);
}
#[test]
fn test_if_let_none_skips() {
    val x: Option<Int> = None;
    var got = -1;
    if val Some(v) = x {
        got = v;
    }
    assert::eq(got, -1);
}
#[test]
fn test_if_let_else() {
    val x: Option<Int> = None;
    val n = if val Some(v) = x { v } else { 99 };
    assert::eq(n, 99);
}

// --- while-val with List.pop ---
#[test]
fn test_while_let_pops_list() {
    var v = [1, 2, 3, 4];
    var sum = 0;
    while val Some(x) = v.pop() {
        sum = sum + x;
    }
    assert::eq(sum, 10);
    assert::eq(v.len(), 0);
}

// --- short-circuit && and || ---
#[test]
fn test_and_short_circuits() {
    // Nested fn declared inside a test body — Oxy hoists nested items to
    // top-level with a mangled name and aliases them locally.
    fn always_false(_c: Int) -> bool { false }
    val r = always_false(99) && (true || true);
    assert::eq(r, false);
}
#[test]
fn test_or_short_circuits() {
    val r = true || false;
    assert::eq(r, true);
}

// --- return-as-expression in nested positions ---
fn early_return_in_if(b: bool) -> Int {
    if b { return 42; }
    -1
}
#[test]
fn test_early_return_taken() { assert::eq(early_return_in_if(true), 42); }
#[test]
fn test_early_return_not_taken() { assert::eq(early_return_in_if(false), -1); }

fn early_return_in_loop() -> Int {
    for i in 0..100 {
        if i == 5 { return i * 2; }
    }
    -1
}
#[test]
fn test_return_from_for() { assert::eq(early_return_in_loop(), 10); }

fn return_from_while() -> Int {
    var i = 0;
    while i < 100 {
        if i == 7 { return i; }
        i = i + 1;
    }
    -1
}
#[test]
fn test_return_from_while() { assert::eq(return_from_while(), 7); }

// --- block as expression ---
#[test]
fn test_block_as_expr() {
    val n = {
        val a = 5;
        val b = 10;
        a + b
    };
    assert::eq(n, 15);
}
#[test]
fn test_nested_block_expr() {
    val n = {
        val x = { val y = 2; y * y };
        x + 1
    };
    assert::eq(n, 5);
}
