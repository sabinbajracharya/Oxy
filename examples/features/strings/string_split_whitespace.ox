// Tests for String::split_whitespace()

#[test]
fn test_split_whitespace_basic() {
    let s = "foo bar baz";
    let parts = s.split_whitespace();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "foo");
    assert_eq!(parts[1], "bar");
    assert_eq!(parts[2], "baz");
}

#[test]
fn test_split_whitespace_multiple_spaces() {
    let s = "  hello   world  ";
    let parts = s.split_whitespace();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0], "hello");
    assert_eq!(parts[1], "world");
}

#[test]
fn test_split_whitespace_tabs_and_newlines() {
    let s = "a\tb\nc";
    let parts = s.split_whitespace();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "a");
    assert_eq!(parts[1], "b");
    assert_eq!(parts[2], "c");
}

#[test]
fn test_split_whitespace_empty_string() {
    let s = "";
    let parts = s.split_whitespace();
    assert_eq!(parts.len(), 0);
}

#[test]
fn test_split_whitespace_only_spaces() {
    let s = "   ";
    let parts = s.split_whitespace();
    assert_eq!(parts.len(), 0);
}

#[test]
fn test_split_whitespace_parse_numbers() {
    // Typical AoC pattern: space-separated numbers on one line
    let s = "10 20 30 40";
    let parts = s.split_whitespace();
    let a = parts[0].parse_int().unwrap();
    let b = parts[1].parse_int().unwrap();
    let c = parts[2].parse_int().unwrap();
    let d = parts[3].parse_int().unwrap();
    assert_eq!(a + b + c + d, 100);
}
