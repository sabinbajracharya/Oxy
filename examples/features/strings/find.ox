// String::find(pat) -> Option<int>
// Returns the character index of the first occurrence of `pat`, or None.

#[test]
fn find_basic() {
    let s = "hello world";
    let result = s.find("world");
    assert_eq!(result, Some(6));
}

#[test]
fn find_at_start() {
    let s = "hello world";
    let result = s.find("hello");
    assert_eq!(result, Some(0));
}

#[test]
fn find_at_end() {
    let s = "hello";
    let result = s.find("o");
    assert_eq!(result, Some(4));
}

#[test]
fn find_not_found() {
    let s = "hello world";
    let result = s.find("xyz");
    assert_eq!(result, None);
}

#[test]
fn find_empty_pattern() {
    let s = "hello";
    let result = s.find("");
    assert_eq!(result, Some(0));
}

#[test]
fn find_empty_string() {
    let s = "";
    let result = s.find("a");
    assert_eq!(result, None);
}

#[test]
fn find_first_occurrence() {
    let s = "ababa";
    let result = s.find("ba");
    assert_eq!(result, Some(1));
}

#[test]
fn find_single_char() {
    let s = "abcdef";
    let result = s.find("d");
    assert_eq!(result, Some(3));
}

#[test]
fn find_unicode() {
    // CJK characters — each is one codepoint, no precomposed/decomposed ambiguity
    let s = "hello 世界你好";
    let result = s.find("世界");
    assert_eq!(result, Some(6));
}

#[test]
fn find_partial_not_match() {
    let s = "hello";
    let result = s.find("helloo");
    assert_eq!(result, None);
}
