// Tests for String::lines()

#[test]
fn test_lines_basic() {
    let s = "foo\nbar\nbaz";
    let lines = s.lines();
    assert_eq(lines.len(), 3);
    assert_eq(lines[0], "foo");
    assert_eq(lines[1], "bar");
    assert_eq(lines[2], "baz");
}

#[test]
fn test_lines_trailing_newline_ignored() {
    // Rust's lines() does not produce a trailing empty element
    let s = "foo\nbar\n";
    let lines = s.lines();
    assert_eq(lines.len(), 2);
    assert_eq(lines[0], "foo");
    assert_eq(lines[1], "bar");
}

#[test]
fn test_lines_crlf() {
    let s = "foo\r\nbar\r\nbaz";
    let lines = s.lines();
    assert_eq(lines.len(), 3);
    assert_eq(lines[0], "foo");
    assert_eq(lines[1], "bar");
    assert_eq(lines[2], "baz");
}

#[test]
fn test_lines_single_line() {
    let s = "hello";
    let lines = s.lines();
    assert_eq(lines.len(), 1);
    assert_eq(lines[0], "hello");
}

#[test]
fn test_lines_empty_string() {
    let s = "";
    let lines = s.lines();
    assert_eq(lines.len(), 0);
}

#[test]
fn test_lines_parse_numbers() {
    let s = "42\n7\n100";
    let lines = s.lines();
    let a = lines[0].parse_int().unwrap();
    let b = lines[1].parse_int().unwrap();
    let c = lines[2].parse_int().unwrap();
    assert_eq(a + b + c, 149);
}
