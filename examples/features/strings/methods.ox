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
    assert_eq("".len(), 0);
    assert_eq("hello".len(), 5);
    assert_eq("abc".len(), 3);
}

#[test]
fn test_is_empty() {
    assert("".is_empty());
    assert(!"hello".is_empty());
    assert(!" ".is_empty());
}

// === to_uppercase / to_lowercase ===

#[test]
fn test_to_uppercase() {
    assert_eq("hello".to_uppercase(), "HELLO");
    assert_eq("Hello".to_uppercase(), "HELLO");
    assert_eq("HELLO".to_uppercase(), "HELLO");
    assert_eq("".to_uppercase(), "");
}

#[test]
fn test_to_lowercase() {
    assert_eq("HELLO".to_lowercase(), "hello");
    assert_eq("Hello".to_lowercase(), "hello");
    assert_eq("hello".to_lowercase(), "hello");
    assert_eq("".to_lowercase(), "");
}

#[test]
fn test_case_with_numbers() {
    assert_eq("abc123".to_uppercase(), "ABC123");
    assert_eq("XYZ789".to_lowercase(), "xyz789");
}

// === trim ===

#[test]
fn test_trim_basic() {
    assert_eq("  hello  ".trim(), "hello");
    assert_eq("hello".trim(), "hello");
    assert_eq("   ".trim(), "");
    assert_eq("".trim(), "");
}

#[test]
fn test_trim_edges() {
    assert_eq("\n\thello\t\n".trim(), "hello");
    assert_eq("  hello world  ".trim(), "hello world");
}

// === contains ===

#[test]
fn test_contains() {
    assert("hello".contains("ell"));
    assert("hello".contains("h"));
    assert("hello".contains("o"));
    assert(!"hello".contains("x"));
    assert("hello".contains(""));
}

#[test]
fn test_contains_empty_pat() {
    assert("".contains(""));
    assert("abc".contains(""));
}

// === starts_with / ends_with ===

#[test]
fn test_starts_with() {
    assert("hello".starts_with("h"));
    assert("hello".starts_with("hel"));
    assert("hello".starts_with("hello"));
    assert(!"hello".starts_with("ello"));
    assert(!"hello".starts_with("x"));
}

#[test]
fn test_ends_with() {
    assert("hello".ends_with("o"));
    assert("hello".ends_with("llo"));
    assert("hello".ends_with("hello"));
    assert(!"hello".ends_with("hell"));
    assert(!"hello".ends_with("x"));
}

#[test]
fn test_starts_ends_empty() {
    assert("abc".starts_with(""));
    assert("abc".ends_with(""));
    assert("".starts_with(""));
    assert("".ends_with(""));
}

// === replace ===

#[test]
fn test_replace_basic() {
    assert_eq("hello".replace("l", "x"), "hexxo");
    assert_eq("aaa".replace("a", "b"), "bbb");
    assert_eq("hello".replace("x", "y"), "hello");
}

#[test]
fn test_replace_empty() {
    assert_eq("".replace("a", "b"), "");
    assert_eq("abc".replace("", "x"), "xaxbxcx");
}

// === split ===

#[test]
fn test_split_basic() {
    let parts = "a,b,c".split(",");
    assert_eq(parts.len(), 3);
    assert_eq(parts[0], "a");
    assert_eq(parts[1], "b");
    assert_eq(parts[2], "c");
}

#[test]
fn test_split_no_match() {
    let parts = "hello".split(",");
    assert_eq(parts.len(), 1);
    assert_eq(parts[0], "hello");
}

#[test]
fn test_split_empty_string() {
    let parts = "".split(",");
    assert_eq(parts.len(), 1);
    assert_eq(parts[0], "");
}

#[test]
fn test_split_by_whitespace() {
    let parts = "one two three".split(" ");
    assert_eq(parts.len(), 3);
}

// === chars ===

#[test]
fn test_chars_basic() {
    let chars = "abc".chars();
    assert_eq(chars.len(), 3);
    assert_eq(chars[0], 'a');
    assert_eq(chars[1], 'b');
    assert_eq(chars[2], 'c');
}

#[test]
fn test_chars_empty() {
    let chars = "".chars();
    assert_eq(chars.len(), 0);
}

#[test]
fn test_chars_unicode() {
    let chars = "héllo".chars();
    assert_eq(chars.len(), 5);
    assert_eq(chars[0], 'h');
    assert_eq(chars[1], 'é');
}

// === repeat ===

#[test]
fn test_repeat_basic() {
    assert_eq("abc".repeat(3), "abcabcabc");
    assert_eq("x".repeat(5), "xxxxx");
}

#[test]
fn test_repeat_zero() {
    assert_eq("hello".repeat(0), "");
}

#[test]
fn test_repeat_one() {
    assert_eq("hi".repeat(1), "hi");
}

#[test]
fn test_repeat_empty_string() {
    assert_eq("".repeat(10), "");
}

// === char_at ===

#[test]
fn test_char_at_basic() {
    assert_eq("hello".char_at(0), 'h');
    assert_eq("hello".char_at(1), 'e');
    assert_eq("hello".char_at(4), 'o');
}

// === substring ===

#[test]
fn test_substring_basic() {
    assert_eq("hello".substring(0, 2), "he");
    assert_eq("hello".substring(1, 4), "ell");
    assert_eq("hello".substring(0, 5), "hello");
}

#[test]
fn test_substring_empty() {
    assert_eq("hello".substring(2, 2), "");
}

#[test]
fn test_substring_full() {
    assert_eq("abc".substring(0, 3), "abc");
}

// === parse_int / parse_float ===

#[test]
fn test_parse_int() {
    let r = "42".parse_int();
    assert(r.is_ok());
}

#[test]
fn test_parse_int_invalid() {
    let r = "notanumber".parse_int();
    assert(r.is_err());
}

#[test]
fn test_parse_float() {
    let r = "3.14".parse_float();
    assert(r.is_ok());
}

// === clone / to_string ===

#[test]
fn test_clone() {
    let s = "hello";
    let s2 = s.clone();
    assert_eq(s, s2);
}

#[test]
fn test_to_string() {
    let s = "hello";
    assert_eq(s.to_string(), "hello");
    assert_eq("".to_string(), "");
}

// === Method Chaining ===

#[test]
fn test_method_chain() {
    assert_eq("  hello  ".trim().to_uppercase(), "HELLO");
    assert_eq("hello".replace("l", "x").to_uppercase(), "HEXXO");
}

#[test]
fn test_chain_after_len() {
    // len() returns int, not a string — but other methods chain on strings
    let s = "  hi  ".trim();
    assert_eq(s.len(), 2);
}
