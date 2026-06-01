// String::find(pat) -> Option<Int>
// Returns the character index of the first occurrence of `pat`, or None.

#[test]
fn find_basic() {
    val s = "hello world";
    val result = s.find("world");
    assert::eq(result, Some(6));
}

#[test]
fn find_at_start() {
    val s = "hello world";
    val result = s.find("hello");
    assert::eq(result, Some(0));
}

#[test]
fn find_at_end() {
    val s = "hello";
    val result = s.find("o");
    assert::eq(result, Some(4));
}

#[test]
fn find_not_found() {
    val s = "hello world";
    val result = s.find("xyz");
    assert::eq(result, None);
}

#[test]
fn find_empty_pattern() {
    val s = "hello";
    val result = s.find("");
    assert::eq(result, Some(0));
}

#[test]
fn find_empty_string() {
    val s = "";
    val result = s.find("a");
    assert::eq(result, None);
}

#[test]
fn find_first_occurrence() {
    val s = "ababa";
    val result = s.find("ba");
    assert::eq(result, Some(1));
}

#[test]
fn find_single_char() {
    val s = "abcdef";
    val result = s.find("d");
    assert::eq(result, Some(3));
}

#[test]
fn find_unicode() {
    // CJK characters — each is one codepoint, no precomposed/decomposed ambiguity
    val s = "hello 世界你好";
    val result = s.find("世界");
    assert::eq(result, Some(6));
}

#[test]
fn find_partial_not_match() {
    val s = "hello";
    val result = s.find("helloo");
    assert::eq(result, None);
}
