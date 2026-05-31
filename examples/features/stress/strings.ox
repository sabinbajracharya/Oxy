// === STRESS: strings — methods, escapes, indexing, interpolation ===

// --- empty string ---
#[test]
fn test_empty_string() {
    val s = "".to_string();
    assert_eq(s.len(), 0);
    assert_eq(s.is_empty(), true);
}

// --- non-empty length ---
#[test]
fn test_string_len_ascii() {
    val s = "hello".to_string();
    assert_eq(s.len(), 5);
}

// --- concatenation via + ---
#[test]
fn test_string_concat() {
    val a = "foo".to_string();
    val b = "bar".to_string();
    assert_eq(a + b, "foobar");
}

// --- format ---
#[test]
fn test_format_basic() {
    val s = format("{}-{}", 1, 2);
    assert_eq(s, "1-2");
}
#[test]
fn test_format_three_args() {
    val s = format("{} {} {}", "a", "b", "c");
    assert_eq(s, "a b c");
}
#[test]
fn test_format_string_arg() {
    val name = "world".to_string();
    val s = format("hello, {}", name);
    assert_eq(s, "hello, world");
}

// --- f-string interpolation ---
#[test]
fn test_fstring_basic() {
    val x = 5;
    val s = f"value is {x}";
    assert_eq(s, "value is 5");
}
#[test]
fn test_fstring_multiple() {
    val a = 1;
    val b = 2;
    val s = f"{a} + {b} = {a + b}";
    assert_eq(s, "1 + 2 = 3");
}

// --- to_string on primitives ---
#[test]
fn test_int_to_string() {
    assert_eq(42.to_string(), "42");
    assert_eq((-5).to_string(), "-5");
}
#[test]
fn test_bool_to_string() {
    assert_eq(true.to_string(), "true");
    assert_eq(false.to_string(), "false");
}
#[test]
fn test_float_to_string() {
    val f: Float = 3.5;
    assert_eq(f.to_string(), "3.5");
}

// --- String methods ---
#[test]
fn test_string_contains() {
    val s = "hello world".to_string();
    assert_eq(s.contains("world"), true);
    assert_eq(s.contains("xyz"), false);
}

#[test]
fn test_string_starts_with() {
    val s = "hello".to_string();
    assert_eq(s.starts_with("he"), true);
    assert_eq(s.starts_with("lo"), false);
}

#[test]
fn test_string_ends_with() {
    val s = "hello".to_string();
    assert_eq(s.ends_with("lo"), true);
    assert_eq(s.ends_with("he"), false);
}

#[test]
fn test_string_to_uppercase() {
    val s = "hello".to_string();
    assert_eq(s.to_uppercase(), "HELLO");
}

#[test]
fn test_string_to_lowercase() {
    val s = "HELLO".to_string();
    assert_eq(s.to_lowercase(), "hello");
}

#[test]
fn test_string_trim() {
    val s = "   hello   ".to_string();
    assert_eq(s.trim(), "hello");
}

#[test]
fn test_string_split() {
    val s = "a,b,c".to_string();
    val parts: List<String> = s.split(",").collect();
    assert_eq(parts.len(), 3);
    assert_eq(parts[0], "a");
    assert_eq(parts[1], "b");
    assert_eq(parts[2], "c");
}

#[test]
fn test_string_split_empty_segments() {
    val s = "a,,b".to_string();
    val parts: List<String> = s.split(",").collect();
    assert_eq(parts.len(), 3);
    assert_eq(parts[1], "");
}

#[test]
fn test_string_replace() {
    val s = "hello world".to_string();
    assert_eq(s.replace("world", "rust"), "hello rust");
}

#[test]
fn test_string_repeat() {
    val s = "ab".to_string();
    assert_eq(s.repeat(3), "ababab");
}

// --- chars iteration ---
#[test]
fn test_chars_count() {
    val s = "hello".to_string();
    val n = s.chars().count();
    assert_eq(n, 5);
}

#[test]
fn test_chars_collect_list() {
    val s = "abc".to_string();
    val v: List<char> = s.chars().collect();
    assert_eq(v.len(), 3);
    assert_eq(v[0], 'a');
    assert_eq(v[2], 'c');
}

// --- escape sequences ---
#[test]
fn test_string_with_newline_escape() {
    val s = "a\nb".to_string();
    assert_eq(s.len(), 3);
}

#[test]
fn test_string_with_tab() {
    val s = "a\tb".to_string();
    assert_eq(s.len(), 3);
}

#[test]
fn test_string_with_backslash() {
    val s = "a\\b".to_string();
    assert_eq(s.len(), 3);
}

#[test]
fn test_string_with_quote() {
    val s = "a\"b".to_string();
    assert_eq(s.len(), 3);
}

// --- char operations ---
#[test]
fn test_char_eq() {
    val c: char = 'a';
    assert_eq(c == 'a', true);
    assert_eq(c == 'b', false);
}

#[test]
fn test_char_to_string() {
    val c: char = 'x';
    assert_eq(c.to_string(), "x");
}

// --- comparison ---
#[test]
fn test_string_eq() {
    val a = "hello".to_string();
    val b = "hello".to_string();
    assert_eq(a, b);
}

#[test]
fn test_string_neq() {
    val a = "hello".to_string();
    val b = "world".to_string();
    assert(a != b);
}

#[test]
fn test_string_lt() {
    val a = "apple".to_string();
    val b = "banana".to_string();
    assert(a < b);
}

// --- nested f-string + format ---
#[test]
fn test_fstring_inside_format() {
    val n = 7;
    val inner = f"#{n}";
    val outer = format("[{}]", inner);
    assert_eq(outer, "[#7]");
}
