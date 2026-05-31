// Tests for String::split_whitespace()

#[test]
fn test_split_whitespace_basic() {
    val s = "foo bar baz";
    val parts = s.split_whitespace();
    assert_eq(parts.len(), 3);
    assert_eq(parts[0], "foo");
    assert_eq(parts[1], "bar");
    assert_eq(parts[2], "baz");
}

#[test]
fn test_split_whitespace_multiple_spaces() {
    val s = "  hello   world  ";
    val parts = s.split_whitespace();
    assert_eq(parts.len(), 2);
    assert_eq(parts[0], "hello");
    assert_eq(parts[1], "world");
}

#[test]
fn test_split_whitespace_tabs_and_newlines() {
    val s = "a\tb\nc";
    val parts = s.split_whitespace();
    assert_eq(parts.len(), 3);
    assert_eq(parts[0], "a");
    assert_eq(parts[1], "b");
    assert_eq(parts[2], "c");
}

#[test]
fn test_split_whitespace_empty_string() {
    val s = "";
    val parts = s.split_whitespace();
    assert_eq(parts.len(), 0);
}

#[test]
fn test_split_whitespace_only_spaces() {
    val s = "   ";
    val parts = s.split_whitespace();
    assert_eq(parts.len(), 0);
}

#[test]
fn test_split_whitespace_parse_numbers() {
    // Typical AoC pattern: space-separated numbers on one line
    val s = "10 20 30 40";
    val parts = s.split_whitespace();
    val a = parts[0].parse_int().unwrap();
    val b = parts[1].parse_int().unwrap();
    val c = parts[2].parse_int().unwrap();
    val d = parts[3].parse_int().unwrap();
    assert_eq(a + b + c + d, 100);
}
