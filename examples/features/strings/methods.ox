// === Feature: Strings — Methods ===
// All built-in String methods in Oxy. Strings are immutable — methods
// that transform return new strings.
//
// === Method Categories ===
//   Query:      len, is_empty, contains, starts_with, ends_with
//   Transform:  to_uppercase, to_lowercase, trim, replace, repeat
//   Access:     char_at, chars, substring, split
//   Parse:      parse_int, parse_float
//   Utility:    clone, to_string

// === len / is_empty ===

#[test]
fn test_len() {
    assert::eq("".len(), 0);
    assert::eq("hello".len(), 5);
    assert::eq("abc".len(), 3);
}

#[test]
fn test_is_empty() {
    assert::true("".is_empty());
    assert::true(!"hello".is_empty());
    assert::true(!" ".is_empty());
}

// === to_uppercase / to_lowercase ===

#[test]
fn test_to_uppercase() {
    assert::eq("hello".to_uppercase(), "HELLO");
    assert::eq("Hello".to_uppercase(), "HELLO");
    assert::eq("HELLO".to_uppercase(), "HELLO");
    assert::eq("".to_uppercase(), "");
}

#[test]
fn test_to_lowercase() {
    assert::eq("HELLO".to_lowercase(), "hello");
    assert::eq("Hello".to_lowercase(), "hello");
    assert::eq("hello".to_lowercase(), "hello");
    assert::eq("".to_lowercase(), "");
}

#[test]
fn test_case_with_numbers() {
    assert::eq("abc123".to_uppercase(), "ABC123");
    assert::eq("XYZ789".to_lowercase(), "xyz789");
}

// === trim ===

#[test]
fn test_trim_basic() {
    assert::eq("  hello  ".trim(), "hello");
    assert::eq("hello".trim(), "hello");
    assert::eq("   ".trim(), "");
    assert::eq("".trim(), "");
}

#[test]
fn test_trim_edges() {
    assert::eq("\n\thello\t\n".trim(), "hello");
    assert::eq("  hello world  ".trim(), "hello world");
}

// === contains ===

#[test]
fn test_contains() {
    assert::true("hello".contains("ell"));
    assert::true("hello".contains("h"));
    assert::true("hello".contains("o"));
    assert::true(!"hello".contains("x"));
    assert::true("hello".contains(""));
}

#[test]
fn test_contains_empty_pat() {
    assert::true("".contains(""));
    assert::true("abc".contains(""));
}

// === starts_with / ends_with ===

#[test]
fn test_starts_with() {
    assert::true("hello".starts_with("h"));
    assert::true("hello".starts_with("hel"));
    assert::true("hello".starts_with("hello"));
    assert::true(!"hello".starts_with("ello"));
    assert::true(!"hello".starts_with("x"));
}

#[test]
fn test_ends_with() {
    assert::true("hello".ends_with("o"));
    assert::true("hello".ends_with("llo"));
    assert::true("hello".ends_with("hello"));
    assert::true(!"hello".ends_with("hell"));
    assert::true(!"hello".ends_with("x"));
}

#[test]
fn test_starts_ends_empty() {
    assert::true("abc".starts_with(""));
    assert::true("abc".ends_with(""));
    assert::true("".starts_with(""));
    assert::true("".ends_with(""));
}

// === replace ===

#[test]
fn test_replace_basic() {
    assert::eq("hello".replace("l", "x"), "hexxo");
    assert::eq("aaa".replace("a", "b"), "bbb");
    assert::eq("hello".replace("x", "y"), "hello");
}

#[test]
fn test_replace_empty() {
    assert::eq("".replace("a", "b"), "");
    assert::eq("abc".replace("", "x"), "xaxbxcx");
}

// === split ===

#[test]
fn test_split_basic() {
    val parts = "a,b,c".split(",");
    assert::eq(parts.len(), 3);
    assert::eq(parts[0], "a");
    assert::eq(parts[1], "b");
    assert::eq(parts[2], "c");
}

#[test]
fn test_split_no_match() {
    val parts = "hello".split(",");
    assert::eq(parts.len(), 1);
    assert::eq(parts[0], "hello");
}

#[test]
fn test_split_empty_string() {
    val parts = "".split(",");
    assert::eq(parts.len(), 1);
    assert::eq(parts[0], "");
}

#[test]
fn test_split_by_whitespace() {
    val parts = "one two three".split(" ");
    assert::eq(parts.len(), 3);
}

// === chars ===

#[test]
fn test_chars_basic() {
    val chars = "abc".chars();
    assert::eq(chars.len(), 3);
    assert::eq(chars[0], 'a');
    assert::eq(chars[1], 'b');
    assert::eq(chars[2], 'c');
}

#[test]
fn test_chars_empty() {
    val chars = "".chars();
    assert::eq(chars.len(), 0);
}

#[test]
fn test_chars_unicode() {
    val chars = "héllo".chars();
    assert::eq(chars.len(), 5);
    assert::eq(chars[0], 'h');
    assert::eq(chars[1], 'é');
}

// === repeat ===

#[test]
fn test_repeat_basic() {
    assert::eq("abc".repeat(3), "abcabcabc");
    assert::eq("x".repeat(5), "xxxxx");
}

#[test]
fn test_repeat_zero() {
    assert::eq("hello".repeat(0), "");
}

#[test]
fn test_repeat_one() {
    assert::eq("hi".repeat(1), "hi");
}

#[test]
fn test_repeat_empty_string() {
    assert::eq("".repeat(10), "");
}

// === char_at ===

#[test]
fn test_char_at_basic() {
    assert::eq("hello".char_at(0), 'h');
    assert::eq("hello".char_at(1), 'e');
    assert::eq("hello".char_at(4), 'o');
}

// === substring ===

#[test]
fn test_substring_basic() {
    assert::eq("hello".substring(0, 2), "he");
    assert::eq("hello".substring(1, 4), "ell");
    assert::eq("hello".substring(0, 5), "hello");
}

#[test]
fn test_substring_empty() {
    assert::eq("hello".substring(2, 2), "");
}

#[test]
fn test_substring_full() {
    assert::eq("abc".substring(0, 3), "abc");
}

// === parse_int / parse_float ===

#[test]
fn test_parse_int() {
    val r = "42".parse_int();
    assert::true(r.is_ok());
}

#[test]
fn test_parse_int_invalid() {
    val r = "notanumber".parse_int();
    assert::true(r.is_err());
}

#[test]
fn test_parse_float() {
    val r = "3.14".parse_float();
    assert::true(r.is_ok());
}

// === clone / to_string ===

#[test]
fn test_clone() {
    val s = "hello";
    val s2 = s.clone();
    assert::eq(s, s2);
}

#[test]
fn test_to_string() {
    val s = "hello";
    assert::eq(s.to_string(), "hello");
    assert::eq("".to_string(), "");
}

// === Method Chaining ===

#[test]
fn test_method_chain() {
    assert::eq("  hello  ".trim().to_uppercase(), "HELLO");
    assert::eq("hello".replace("l", "x").to_uppercase(), "HEXXO");
}

#[test]
fn test_chain_after_len() {
    // len() returns Int, not a string — but other methods chain on strings
    val s = "  hi  ".trim();
    assert::eq(s.len(), 2);
}
