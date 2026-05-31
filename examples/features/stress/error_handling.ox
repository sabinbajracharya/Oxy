// === STRESS: Option / Result + `?` operator interactions ===

// --- Option construction ---
#[test]
fn test_some_constructs() { val x: Option<Int> = Some(5); assert_eq(x, Some(5)); }
#[test]
fn test_none_constructs() { val x: Option<Int> = None; assert_eq(x, None); }

// --- Result construction ---
#[test]
fn test_ok_constructs() { val r: Result<Int, String> = Ok(5); assert_eq(r, Ok(5)); }
#[test]
fn test_err_constructs() {
    val r: Result<Int, String> = Err("boom".to_string());
    assert_eq(r, Err("boom".to_string()));
}

// --- Option::is_some / is_none ---
#[test]
fn test_option_is_some() {
    val s: Option<Int> = Some(1);
    val n: Option<Int> = None;
    assert_eq(s.is_some(), true);
    assert_eq(n.is_some(), false);
}
#[test]
fn test_option_is_none() {
    val s: Option<Int> = Some(1);
    val n: Option<Int> = None;
    assert_eq(s.is_none(), false);
    assert_eq(n.is_none(), true);
}

// --- Option::unwrap_or ---
#[test]
fn test_option_unwrap_or_some() {
    val s: Option<Int> = Some(7);
    assert_eq(s.unwrap_or(99), 7);
}
#[test]
fn test_option_unwrap_or_none() {
    val n: Option<Int> = None;
    assert_eq(n.unwrap_or(99), 99);
}

// --- Option::map ---
#[test]
fn test_option_map_some() {
    val s: Option<Int> = Some(3);
    assert_eq(s.map(|x| x * 2), Some(6));
}
#[test]
fn test_option_map_none() {
    val n: Option<Int> = None;
    assert_eq(n.map(|x| x * 2), None);
}

// --- Option::ok_or ---
#[test]
fn test_option_ok_or_some() {
    val s: Option<Int> = Some(5);
    val r: Result<Int, String> = s.ok_or("nope".to_string());
    assert_eq(r, Ok(5));
}
#[test]
fn test_option_ok_or_none() {
    val n: Option<Int> = None;
    val r: Result<Int, String> = n.ok_or("nope".to_string());
    assert_eq(r, Err("nope".to_string()));
}

// --- Result::is_ok / is_err ---
#[test]
fn test_result_is_ok() {
    val r: Result<Int, String> = Ok(1);
    assert_eq(r.is_ok(), true);
    assert_eq(r.is_err(), false);
}
#[test]
fn test_result_is_err() {
    val r: Result<Int, String> = Err("x".to_string());
    assert_eq(r.is_ok(), false);
    assert_eq(r.is_err(), true);
}

// --- Result::unwrap_or ---
#[test]
fn test_result_unwrap_or_ok() {
    val r: Result<Int, String> = Ok(7);
    assert_eq(r.unwrap_or(99), 7);
}
#[test]
fn test_result_unwrap_or_err() {
    val r: Result<Int, String> = Err("nope".to_string());
    assert_eq(r.unwrap_or(99), 99);
}

// --- Result::map ---
#[test]
fn test_result_map_ok() {
    val r: Result<Int, String> = Ok(3);
    val r2: Result<Int, String> = r.map(|x| x * 2);
    assert_eq(r2, Ok(6));
}
#[test]
fn test_result_map_err_passthrough() {
    val r: Result<Int, String> = Err("x".to_string());
    val r2: Result<Int, String> = r.map(|x| x * 2);
    assert_eq(r2, Err("x".to_string()));
}

// --- Result::map_err ---
#[test]
fn test_result_map_err() {
    val r: Result<Int, String> = Err("e".to_string());
    val r2: Result<Int, String> = r.map_err(|e| format("[{}]", e));
    assert_eq(r2, Err("[e]".to_string()));
}

// --- ? in fn returning Result ---
fn parse_double(s: String) -> Result<Int, String> {
    if s == "x" { Err("bad".to_string()) } else { Ok(42) }
}

fn chained_question() -> Result<Int, String> {
    val n = parse_double("ok".to_string())?;
    Ok(n + 1)
}

#[test]
fn test_question_chain_ok() {
    assert_eq(chained_question(), Ok(43));
}

fn chained_question_err() -> Result<Int, String> {
    val n = parse_double("x".to_string())?;
    Ok(n + 1)
}

#[test]
fn test_question_chain_err() {
    assert_eq(chained_question_err(), Err("bad".to_string()));
}

// --- ? in fn returning Option ---
fn double_some(x: Option<Int>) -> Option<Int> {
    val v = x?;
    Some(v * 2)
}
#[test]
fn test_question_option_some() {
    assert_eq(double_some(Some(3)), Some(6));
}
#[test]
fn test_question_option_none() {
    assert_eq(double_some(None), None);
}

// --- ? converts between same-error types ---
fn first(s: String) -> Result<Int, String> {
    if s.len() == 0 { Err("empty".to_string()) } else { Ok(s.len() as Int) }
}

fn outer(a: String, b: String) -> Result<Int, String> {
    val la = first(a)?;
    val lb = first(b)?;
    Ok(la + lb)
}

#[test]
fn test_question_double_chain_ok() {
    assert_eq(outer("hi".to_string(), "bye".to_string()), Ok(5));
}

#[test]
fn test_question_double_chain_short_circuits() {
    assert_eq(outer("".to_string(), "bye".to_string()), Err("empty".to_string()));
    assert_eq(outer("hi".to_string(), "".to_string()), Err("empty".to_string()));
}

// --- match on Result ---
#[test]
fn test_match_on_result() {
    val r: Result<Int, String> = Ok(5);
    val n = match r {
        Ok(v) => v,
        Err(_) => -1,
    };
    assert_eq(n, 5);
}

// --- Option in List ---
#[test]
fn test_vec_of_options() {
    val v: List<Option<Int>> = [Some(1), None, Some(3)];
    assert_eq(v.len(), 3);
}

// --- Result combinators ---
#[test]
fn test_result_and_then() {
    val r: Result<Int, String> = Ok(2);
    val r2: Result<Int, String> = r.and_then(|x| if x > 0 { Ok(x * 10) } else { Err("neg".to_string()) });
    assert_eq(r2, Ok(20));
}
