// === Feature: select! — race multiple spawned tasks ===
// select(handle1, handle2, ...) returns the result of whichever
// completes first. The VM suspends the current task until at
// least one of the given JoinHandles finishes.

fn main() {}

// --- basic: select between two spawned tasks ---

#[test]
fn test_select_first_of_two() {
    let a = spawn(|| 42);
    let b = spawn(|| 99);
    let result = select(a, b);
    assert_eq!(result == 42 || result == 99, true);
}

#[test]
fn test_select_with_sleep_faster() {
    let fast = spawn(|| "fast");
    let slow = spawn(|| {
        sleep(1000);
        "slow"
    });
    let result = select(fast, slow);
    assert_eq!(result, "fast");
}

#[test]
fn test_select_slow_wins_when_fast_yields() {
    let a = spawn(|| {
        sleep(1000);
        "a"
    });
    let b = spawn(|| "b");
    let result = select(a, b);
    assert_eq!(result, "b");
}

// --- select with three handles ---

#[test]
fn test_select_three_handles() {
    let a = spawn(|| {
        sleep(500);
        "a"
    });
    let b = spawn(|| "b");
    let c = spawn(|| {
        sleep(1000);
        "c"
    });
    let result = select(a, b, c);
    assert_eq!(result, "b");
}

// --- select with sleep as timeout ---

#[test]
fn test_select_timeout_pattern() {
    let work = spawn(|| {
        sleep(100);
        "done"
    });
    let timeout = spawn(|| {
        sleep(5000);
        "timeout"
    });
    let result = select(work, timeout);
    assert_eq!(result, "done");
}

// --- select where both are ready (no sleep) ---

#[test]
fn test_select_both_ready() {
    let x = spawn(|| 1);
    let y = spawn(|| 2);
    let result = select(x, y);
    assert_eq!(result == 1 || result == 2, true);
}

// --- nested select inside spawn ---

#[test]
fn test_select_inside_spawn() {
    let outer = spawn(|| {
        let a = spawn(|| "inner_a");
        let b = spawn(|| {
            sleep(100);
            "inner_b"
        });
        select(a, b)
    });
    assert_eq!(outer.await, "inner_a");
}

// --- select with captured variables ---

#[test]
fn test_select_with_captures() {
    let x = 10;
    let y = 20;
    let a = spawn(|| x * 2);
    let b = spawn(|| y * 3);
    let result = select(a, b);
    assert_eq!(result == 20 || result == 60, true);
}

// --- compile_error: select with zero args ---

#[compile_error]
fn select_zero_args() {
    select();
}

// --- compile_error: select with one arg ---

#[compile_error]
fn select_one_arg() {
    let h = spawn(|| 42);
    select(h);
}
