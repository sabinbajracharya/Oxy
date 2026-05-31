// === Feature: `?` is rejected in functions that don't return Result/Option ===
// Without this check, using `?` in `fn main()` (or any `()`-returning fn) would
// silently swallow propagated errors — exit code 0, no output, no panic.
// The fix-it points the user at the two ways out: handle with `match`, or
// change the fn signature to return `Result<_, _>` / `Option<_>`.

fn fails() -> Result<Int, String> {
    Err("boom".to_string())
}

fn never_some() -> Option<Int> {
    None
}

fn outer_result() -> Result<Int, String> {
    let v = fails()?;
    Ok(v + 1)
}

fn outer_option() -> Option<Int> {
    let v = never_some()?;
    Some(v + 1)
}

#[test]
fn test_question_mark_in_result_fn_ok() {
    match outer_result() {
        Err(_) => assert(true),
        Ok(_) => panic("should have propagated Err"),
    }
}

#[test]
fn test_question_mark_in_option_fn_ok() {
    assert_eq(outer_option(), None);
}

#[compile_error]
fn test_question_mark_in_unit_fn_rejected() {
    // main-style fn returning `()` — `?` would silently drop the Err.
    let _v = fails()?;
}

#[compile_error]
fn test_question_mark_in_int_fn_rejected() -> Int {
    let _v = fails()?;
    0
}

#[compile_error]
fn test_question_mark_on_non_result_rejected() -> Result<Int, String> {
    // `?` requires a Result/Option operand. Calling it on a plain Int
    // is meaningless.
    let n: Int = 5;
    let _v = n?;
    Ok(0)
}
