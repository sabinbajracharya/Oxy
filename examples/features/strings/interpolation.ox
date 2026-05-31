// === Feature: Strings — F-String Interpolation ===
// F-strings (f"...") embed expressions inside string literals using {expr}
// syntax. The expression is evaluated at runtime and converted to string
// via its Display/to_string representation.
//
// === Declaration Styles ===
//   f"Hello {name}"        (f-string with interpolation)
//   f"No interpolation"    (f-string with just literal text)

// === Basic Interpolation ===

#[test]
fn test_fstring_single_variable() {
    val name = "Alice";
    val s = f"Hello {name}";
    assert_eq(s, "Hello Alice");
}

#[test]
fn test_fstring_int_interpolation() {
    val x = 42;
    val s = f"value: {x}";
    assert_eq(s, "value: 42");
}

#[test]
fn test_fstring_float_interpolation() {
    val pi = 3.14;
    val s = f"pi is {pi}";
    assert(s.contains("3.14"));
}

// === Multiple Interpolations ===

#[test]
fn test_fstring_multiple() {
    val x = 10;
    val y = 20;
    val s = f"x={x}, y={y}";
    assert(s.contains("x=10"));
    assert(s.contains("y=20"));
}

#[test]
fn test_fstring_three_exprs() {
    val a = "a";
    val b = "b";
    val c = "c";
    val s = f"{a}{b}{c}";
    assert_eq(s, "abc");
}

// === F-String Literal Text ===

#[test]
fn test_fstring_no_interpolation() {
    val s = f"hello world";
    assert_eq(s, "hello world");
}

#[test]
fn test_fstring_empty_interpolation() {
    val s = f"";
    assert_eq(s, "");
}

// === Expressions in F-Strings ===

#[test]
fn test_fstring_arithmetic_expression() {
    val s = f"1 + 2 = {1 + 2}";
    assert_eq(s, "1 + 2 = 3");
}

#[test]
fn test_fstring_method_call() {
    val s = f"uppercase: {"hello".to_uppercase()}";
    assert_eq(s, "uppercase: HELLO");
}

#[test]
fn test_fstring_function_call() {
    val s = f"len: {"abc".len()}";
    assert_eq(s, "len: 3");
}

// === F-String with String Variable ===

#[test]
fn test_fstring_string_var() {
    val greeting = "Hey";
    val name = "Bob";
    val s = f"{greeting} {name}!";
    assert_eq(s, "Hey Bob!");
}

// === F-String with Chars ===

#[test]
fn test_fstring_char() {
    val c = 'X';
    val s = f"char: {c}";
    assert_eq(s, "char: X");
}
