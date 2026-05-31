// === STRESS: std::rand and std::regex stdlib ===

// --- rand::rand_int (inclusive range) ---
#[test]
fn test_rand_int_in_range() {
    let mut all_in_range = true;
    let mut i = 0;
    while i < 50 {
        let r = std::rand::rand_int(1, 6);
        if r < 1 || r > 6 { all_in_range = false; }
        i = i + 1;
    }
    assert(all_in_range);
}

#[test]
fn test_rand_int_single_value() {
    let r = std::rand::rand_int(7, 7);
    assert_eq(r, 7);
}

// --- rand::range (half-open) ---
#[test]
fn test_rand_range_in_bounds() {
    let mut all_in = true;
    let mut i = 0;
    while i < 50 {
        let r = std::rand::range(0, 10);
        if r < 0 || r >= 10 { all_in = false; }
        i = i + 1;
    }
    assert(all_in);
}

// --- rand::random (Float in [0, 1)) ---
#[test]
fn test_rand_random_float() {
    let r = std::rand::random();
    assert(r >= 0.0 && r < 1.0);
}

// --- rand::bool ---
#[test]
fn test_rand_bool_returns_bool() {
    let b = std::rand::bool();
    // tautology — just verify the call doesn't error and returns a bool
    assert(b == true || b == false);
}

// --- regex: function-form API ---
#[test]
fn test_regex_is_match_true() {
    let r = std::regex::is_match("\\d+", "hello 42 world");
    assert_eq(r, true);
}

#[test]
fn test_regex_is_match_false() {
    let r = std::regex::is_match("\\d+", "hello world");
    assert_eq(r, false);
}

#[test]
fn test_regex_find_returns_some() {
    let m = std::regex::find("\\d+", "abc 42 def");
    assert_eq(m.is_some(), true);
}

#[test]
fn test_regex_find_returns_none() {
    let m = std::regex::find("\\d+", "no digits here");
    assert_eq(m.is_none(), true);
}

#[test]
fn test_regex_find_all_count() {
    let v: List<Map<String, String>> = std::regex::find_all("\\d+", "1 22 333");
    assert_eq(v.len(), 3);
}
