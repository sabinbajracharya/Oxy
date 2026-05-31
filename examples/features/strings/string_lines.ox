// Tests for String::lines()

#[test]
fn test_lines_basic() {
    val s = "foo\nbar\nbaz";
    val lines = s.lines();
    assert_eq(lines.len(), 3);
    assert_eq(lines[0], "foo");
    assert_eq(lines[1], "bar");
    assert_eq(lines[2], "baz");
}

#[test]
fn test_lines_trailing_newline_ignored() {
    // Rust's lines() does not produce a trailing empty element
    val s = "foo\nbar\n";
    val lines = s.lines();
    assert_eq(lines.len(), 2);
    assert_eq(lines[0], "foo");
    assert_eq(lines[1], "bar");
}

#[test]
fn test_lines_crlf() {
    val s = "foo\r\nbar\r\nbaz";
    val lines = s.lines();
    assert_eq(lines.len(), 3);
    assert_eq(lines[0], "foo");
    assert_eq(lines[1], "bar");
    assert_eq(lines[2], "baz");
}

#[test]
fn test_lines_single_line() {
    val s = "hello";
    val lines = s.lines();
    assert_eq(lines.len(), 1);
    assert_eq(lines[0], "hello");
}

#[test]
fn test_lines_empty_string() {
    val s = "";
    val lines = s.lines();
    assert_eq(lines.len(), 0);
}

#[test]
fn test_lines_parse_numbers() {
    val s = "42\n7\n100";
    val lines = s.lines();
    val a = lines[0].parse_int().unwrap();
    val b = lines[1].parse_int().unwrap();
    val c = lines[2].parse_int().unwrap();
    assert_eq(a + b + c, 149);
}
