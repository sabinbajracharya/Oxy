// === Feature: select! — race multiple spawned tasks ===
// select(handle1, handle2, ...) returns the result of whichever
// completes first. The VM suspends the current task until at
// least one of the given JoinHandles finishes.

fn main() {}

// --- basic: select between two spawned tasks ---

#[test]
fn test_select_first_of_two() {
    val a = spawn(|| 42);
    val b = spawn(|| 99);
    val result = select(a, b);
    assert::eq(result == 42 || result == 99, true);
}

#[test]
fn test_select_with_sleep_faster() {
    val fast = spawn(|| "fast");
    val slow = spawn(|| {
        sleep(1000);
        "slow"
    });
    val result = select(fast, slow);
    assert::eq(result, "fast");
}

#[test]
fn test_select_slow_wins_when_fast_yields() {
    val a = spawn(|| {
        sleep(1000);
        "a"
    });
    val b = spawn(|| "b");
    val result = select(a, b);
    assert::eq(result, "b");
}

// --- select with three handles ---

#[test]
fn test_select_three_handles() {
    val a = spawn(|| {
        sleep(500);
        "a"
    });
    val b = spawn(|| "b");
    val c = spawn(|| {
        sleep(1000);
        "c"
    });
    val result = select(a, b, c);
    assert::eq(result, "b");
}

// --- select with sleep as timeout ---

#[test]
fn test_select_timeout_pattern() {
    val work = spawn(|| {
        sleep(100);
        "done"
    });
    val timeout = spawn(|| {
        sleep(5000);
        "timeout"
    });
    val result = select(work, timeout);
    assert::eq(result, "done");
}

// --- select where both are ready (no sleep) ---

#[test]
fn test_select_both_ready() {
    val x = spawn(|| 1);
    val y = spawn(|| 2);
    val result = select(x, y);
    assert::eq(result == 1 || result == 2, true);
}

// --- nested select inside spawn ---

#[test]
fn test_select_inside_spawn() {
    val outer = spawn(|| {
        val a = spawn(|| "inner_a");
        val b = spawn(|| {
            sleep(100);
            "inner_b"
        });
        select(a, b)
    });
    assert::eq(outer.await, "inner_a");
}

// --- select with captured variables ---

#[test]
fn test_select_with_captures() {
    val x = 10;
    val y = 20;
    val a = spawn(|| x * 2);
    val b = spawn(|| y * 3);
    val result = select(a, b);
    assert::eq(result == 20 || result == 60, true);
}

// --- compile_error: select with zero args ---

#[compile_error]
fn select_zero_args() {
    select();
}

// --- compile_error: select with one arg ---

#[compile_error]
fn select_one_arg() {
    val h = spawn(|| 42);
    select(h);
}
