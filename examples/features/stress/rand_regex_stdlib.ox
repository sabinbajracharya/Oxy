// === STRESS: std::rand and std::regex stdlib ===

// --- rand::rand_int (inclusive range) ---
#[test]
fn test_rand_int_in_range() {
    var all_in_range = true;
    var i = 0;
    while i < 50 {
        val r = std::rand::rand_int(1, 6);
        if r < 1 || r > 6 { all_in_range = false; }
        i = i + 1;
    }
    assert::true(all_in_range);
}

#[test]
fn test_rand_int_single_value() {
    val r = std::rand::rand_int(7, 7);
    assert::eq(r, 7);
}

// --- rand::range (half-open) ---
#[test]
fn test_rand_range_in_bounds() {
    var all_in = true;
    var i = 0;
    while i < 50 {
        val r = std::rand::range(0, 10);
        if r < 0 || r >= 10 { all_in = false; }
        i = i + 1;
    }
    assert::true(all_in);
}

// --- rand::random (Float in [0, 1)) ---
#[test]
fn test_rand_random_float() {
    val r = std::rand::random();
    assert::true(r >= 0.0 && r < 1.0);
}

// --- rand::bool ---
#[test]
fn test_rand_bool_returns_bool() {
    val b = std::rand::bool();
    // tautology — just verify the call doesn't error and returns a bool
    assert::true(b == true || b == false);
}

// --- regex: function-form API ---
#[test]
fn test_regex_is_match_true() {
    val r = std::regex::is_match("\\d+", "hello 42 world");
    assert::eq(r, true);
}

#[test]
fn test_regex_is_match_false() {
    val r = std::regex::is_match("\\d+", "hello world");
    assert::eq(r, false);
}

#[test]
fn test_regex_find_returns_some() {
    val m = std::regex::find("\\d+", "abc 42 def");
    assert::eq(m.is_some(), true);
}

#[test]
fn test_regex_find_returns_none() {
    val m = std::regex::find("\\d+", "no digits here");
    assert::eq(m.is_none(), true);
}

#[test]
fn test_regex_find_all_count() {
    val v: List<Map<String, String>> = std::regex::find_all("\\d+", "1 22 333");
    assert::eq(v.len(), 3);
}
