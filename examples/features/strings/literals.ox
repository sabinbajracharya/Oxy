// === Feature: Strings — Literals ===
// String literal forms in Oxy: regular strings with escape sequences,
// raw strings (no escapes processed), and character literals.
//
// === Declaration Styles ===
//   let s = "hello";          (bare string, inferred as String)
//   let s: String = "hello";  (type annotation)

// === Regular String Literals ===

#[test]
fn test_empty_string() {
    let s = "";
    assert_eq!(s, "");
    assert_eq!(s.len(), 0);
    assert!(s.is_empty());
}

#[test]
fn test_simple_string() {
    let s = "hello";
    assert_eq!(s, "hello");
    assert_eq!(s.len(), 5);
    assert!(!s.is_empty());
}

#[test]
fn test_string_with_spaces() {
    let s = "hello world";
    assert_eq!(s.len(), 11);
}

// === Escape Sequences ===

#[test]
fn test_escape_newline() {
    let s = "line1\nline2";
    assert!(s.contains("\n"));
}

#[test]
fn test_escape_tab() {
    let s = "col1\tcol2";
    assert!(s.contains("\t"));
}

#[test]
fn test_escape_quote() {
    let s = "he said \"hello\"";
    assert!(s.contains("\""));
}

#[test]
fn test_escape_backslash() {
    let s = "path\\to\\file";
    assert!(s.contains("\\"));
}

#[test]
fn test_escape_null() {
    let s = "a\0b";
    assert_eq!(s.len(), 3);
}

// === Raw String Literals ===

#[test]
fn test_raw_string_no_escapes() {
    let s = r"hello\nworld";
    // Raw string: backslash-n is literal, not newline
    assert!(s.contains("\\n"));
    assert!(!s.contains("\n"));
}

#[test]
fn test_raw_string_with_quotes() {
    let s = r#"he said "hello""#;
    assert!(s.contains("\""));
}

#[test]
fn test_raw_string_backslash() {
    let s = r"path\to\file";
    assert_eq!(s, "path\\to\\file");
}

// === Char Literals ===

#[test]
fn test_char_literal() {
    let c = 'a';
    assert_eq!(c, 'a');
}

#[test]
fn test_char_newline() {
    let c = '\n';
    assert_eq!(c, '\n');
}

#[test]
fn test_char_unicode() {
    let c = '字';
    assert_eq!(c, '字');
}

// === Unicode Strings ===

#[test]
fn test_unicode_string() {
    let s = "héllo 世界";
    assert_eq!(s.len(), 8);
}

#[test]
fn test_emoji_string() {
    let s = "hello 👋 world 🌍";
    assert!(s.len() > 10);
}

// === String with Type Annotation ===

#[test]
fn test_type_annotation_string() {
    let s: String = "hello";
    assert_eq!(s, "hello");
}

// === Multi-line Strings ===

#[test]
fn test_multiline_string() {
    let s = "line one
line two";
    assert!(s.contains("\n"));
}
