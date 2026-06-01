// === Feature: Strings — Literals ===
// String literal forms in Oxy: regular strings with escape sequences,
// raw strings (no escapes processed), and character literals.
//
// === Declaration Styles ===
//   val s = "hello";          (bare string, inferred as String)
//   val s: String = "hello";  (type annotation)

// === Regular String Literals ===

#[test]
fn test_empty_string() {
    val s = "";
    assert::eq(s, "");
    assert::eq(s.len(), 0);
    assert::true(s.is_empty());
}

#[test]
fn test_simple_string() {
    val s = "hello";
    assert::eq(s, "hello");
    assert::eq(s.len(), 5);
    assert::true(!s.is_empty());
}

#[test]
fn test_string_with_spaces() {
    val s = "hello world";
    assert::eq(s.len(), 11);
}

// === Escape Sequences ===

#[test]
fn test_escape_newline() {
    val s = "line1\nline2";
    assert::true(s.contains("\n"));
}

#[test]
fn test_escape_tab() {
    val s = "col1\tcol2";
    assert::true(s.contains("\t"));
}

#[test]
fn test_escape_quote() {
    val s = "he said \"hello\"";
    assert::true(s.contains("\""));
}

#[test]
fn test_escape_backslash() {
    val s = "path\\to\\file";
    assert::true(s.contains("\\"));
}

#[test]
fn test_escape_null() {
    val s = "a\0b";
    assert::eq(s.len(), 3);
}

// === Raw String Literals ===

#[test]
fn test_raw_string_no_escapes() {
    val s = r"hello\nworld";
    // Raw string: backslash-n is literal, not newline
    assert::true(s.contains("\\n"));
    assert::true(!s.contains("\n"));
}

#[test]
fn test_raw_string_with_quotes() {
    val s = r#"he said "hello""#;
    assert::true(s.contains("\""));
}

#[test]
fn test_raw_string_backslash() {
    val s = r"path\to\file";
    assert::eq(s, "path\\to\\file");
}

// === Char Literals ===

#[test]
fn test_char_literal() {
    val c = 'a';
    assert::eq(c, 'a');
}

#[test]
fn test_char_newline() {
    val c = '\n';
    assert::eq(c, '\n');
}

#[test]
fn test_char_unicode() {
    val c = '字';
    assert::eq(c, '字');
}

// === Unicode Strings ===

#[test]
fn test_unicode_string() {
    val s = "héllo 世界";
    assert::eq(s.len(), 8);
}

#[test]
fn test_emoji_string() {
    val s = "hello 👋 world 🌍";
    assert::true(s.len() > 10);
}

// === String with Type Annotation ===

#[test]
fn test_type_annotation_string() {
    val s: String = "hello";
    assert::eq(s, "hello");
}

// === Multi-line Strings ===

#[test]
fn test_multiline_string() {
    val s = "line one
line two";
    assert::true(s.contains("\n"));
}
