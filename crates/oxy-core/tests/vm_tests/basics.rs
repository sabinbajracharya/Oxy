//! Literals, variables, arithmetic, comparisons, logic, casts.
//!
//! Shared helpers and imports come from the parent module (`use super::*`).

use super::*;

#[test]
fn test_empty_main() {
    let val = run_and_get_value("fn main() {}");
    assert_eq!(val, Value::Unit);
}

#[test]
fn test_println_string() {
    let output = run_and_capture(r#"fn main() { println!("Hello, Oxy!"); }"#);
    assert_eq!(output, vec!["Hello, Oxy!\n"]);
}

#[test]
fn test_println_format() {
    let output = run_and_capture(r#"fn main() { let x = 42; println!("x = {}", x); }"#);
    assert_eq!(output, vec!["x = 42\n"]);
}

#[test]
fn test_println_multiple_args() {
    let output = run_and_capture(
        r#"fn main() { let a = 1; let b = 2; println!("{} + {} = {}", a, b, a + b); }"#,
    );
    assert_eq!(output, vec!["1 + 2 = 3\n"]);
}

#[test]
fn test_let_binding() {
    let output = run_and_capture(r#"fn main() { let x = 10; println!("{}", x); }"#);
    assert_eq!(output, vec!["10\n"]);
}

#[test]
fn test_let_mut_and_assign() {
    let output = run_and_capture(r#"fn main() { let mut x = 1; x = 2; println!("{}", x); }"#);
    assert_eq!(output, vec!["2\n"]);
}

#[test]
fn test_immutable_assign_error() {
    let result = run(r#"fn main() { let x = 1; x = 2; }"#);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("cannot assign to immutable"));
}

#[test]
fn test_undefined_variable_error() {
    let result = run(r#"fn main() { println!("{}", x); }"#);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("undefined variable"));
}

#[test]
fn test_shadowing() {
    let output = run_and_capture(r#"fn main() { let x = 1; let x = "hello"; println!("{}", x); }"#);
    assert_eq!(output, vec!["hello\n"]);
}

#[test]
fn test_integer_arithmetic() {
    let output = run_and_capture(r#"fn main() { println!("{}", 2 + 3 * 4); }"#);
    assert_eq!(output, vec!["14\n"]);
}

#[test]
fn test_float_arithmetic() {
    let output = run_and_capture(r#"fn main() { println!("{}", 1.5 + 2.5); }"#);
    assert_eq!(output, vec!["4.0\n"]);
}

#[test]
fn test_division_by_zero() {
    let result = run(r#"fn main() { let x = 1 / 0; }"#);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("division by zero"));
}

#[test]
fn test_string_concatenation() {
    let output =
        run_and_capture(r#"fn main() { let s = "hello" + " " + "world"; println!("{}", s); }"#);
    assert_eq!(output, vec!["hello world\n"]);
}

#[test]
fn test_negation() {
    let output = run_and_capture(r#"fn main() { let x = 5; println!("{}", -x); }"#);
    assert_eq!(output, vec!["-5\n"]);
}

#[test]
fn test_comparisons() {
    let output =
        run_and_capture(r#"fn main() { println!("{} {} {} {}", 1 < 2, 2 > 1, 1 == 1, 1 != 2); }"#);
    assert_eq!(output, vec!["true true true true\n"]);
}

#[test]
fn test_logical_and_or() {
    let output =
        run_and_capture(r#"fn main() { println!("{} {}", true && false, true || false); }"#);
    assert_eq!(output, vec!["false true\n"]);
}

#[test]
fn test_logical_not() {
    let output = run_and_capture(r#"fn main() { println!("{}", !true); }"#);
    assert_eq!(output, vec!["false\n"]);
}

#[test]
fn test_block_value() {
    let output =
        run_and_capture(r#"fn main() { let x = { let y = 10; y + 1 }; println!("{}", x); }"#);
    assert_eq!(output, vec!["11\n"]);
}

#[test]
fn test_compound_assignment() {
    let output =
        run_and_capture(r#"fn main() { let mut x = 10; x += 5; x -= 3; println!("{}", x); }"#);
    assert_eq!(output, vec!["12\n"]);
}

#[test]
fn test_no_main_error() {
    let result = run("fn foo() {}");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("no `main` function"));
}

#[test]
fn test_multiple_println() {
    let output = run_and_capture(
        r#"
fn main() {
    println!("line 1");
    println!("line 2");
    println!("line 3");
}
"#,
    );
    assert_eq!(output, vec!["line 1\n", "line 2\n", "line 3\n"]);
}

#[test]
fn test_fibonacci() {
    let output = run_and_capture(
        r#"
fn fib(n: int) -> int {
    if n <= 1 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

fn main() {
    println!("{}", fib(10));
}
"#,
    );
    assert_eq!(output, vec!["55\n"]);
}

#[test]
fn test_int_and_float_literals() {
    let output = run_and_capture(
        r#"
fn main() {
    println!("{}", 42);
    println!("{}", 1000000);
    println!("{}", 123123123123);
    println!("{}", 3.14);
    println!("{}", 2.718);
}"#,
    );
    assert_eq!(
        output,
        vec!["42\n", "1000000\n", "123123123123\n", "3.14\n", "2.718\n"]
    );
}

#[test]
fn test_type_annotation_narrowing() {
    let output = run_and_capture(
        r#"
fn main() {
    let a: int = 127;
    let b: int = 32767;
    let c: int = 100000;
    let d: byte = 255;
    let e: int = 60000;
    let f: int = 3000000000;
    println!("{} {} {} {} {} {}", a, b, c, d, e, f);
}"#,
    );
    assert_eq!(output, vec!["127 32767 100000 255 60000 3000000000\n"]);
}

// Numeric wrap tests for Oxy's two integer types: `int` and `byte`.
// (The old per-width wrapping tests were removed when `i8/i16/i32/u16/u32/u64`
// were retired from the surface language; only int/byte semantics remain.)

#[test]
fn test_byte_wraps_modulo_256() {
    // To get byte-width wrapping the result has to land back in a
    // byte-typed binding (or be cast). Intermediate arithmetic still
    // promotes to int. This is intentional — the declared type matters
    // at the binding boundary, not for every intermediate.
    let output = run_and_capture(
        r#"
fn main() {
    let a: byte = 255;
    let r1: byte = a + 1;     // 256 -> wraps to 0 on store
    let r2: byte = a + 45;    // 300 -> wraps to 44 on store
    let b: byte = 0;
    let r3: byte = b - 1;     // -1 -> wraps to 255 on store
    println!("{}", r1);
    println!("{}", r2);
    println!("{}", r3);
}"#,
    );
    assert_eq!(output, vec!["0\n", "44\n", "255\n"]);
}

#[test]
fn test_as_cast_narrowing_to_byte() {
    let output = run_and_capture(
        r#"
fn main() {
    println!("{}", 300 as byte);      // 300 mod 256 = 44
    println!("{}", (-1) as byte);     // wraps to 255
    println!("{}", 256 as byte);      // wraps to 0
}"#,
    );
    assert_eq!(output, vec!["44\n", "255\n", "0\n"]);
}

#[test]
fn test_as_cast_widening_byte_to_int() {
    let output = run_and_capture(
        r#"
fn main() {
    let b: byte = 200;
    println!("{}", b as int);
    println!("{}", (b as int) * 100);
}"#,
    );
    assert_eq!(output, vec!["200\n", "20000\n"]);
}

#[test]
fn test_literal_coercion_to_all_types() {
    // Unsuffixed literal should be assignable to any integer type
    let output = run_and_capture(
        r#"
fn main() {
    let a: int = 42;
    let b: int = 42;
    let c: int = 42;
    let d: int = 42;
    let e: byte = 42;
    let f: int = 42;
    let g: int = 42;
    let h: int = 42;
    let sum = a as int + b as int + c as int + d + e as int + f as int + g as int + h as int;
    println!("{}", sum);
}"#,
    );
    assert_eq!(output, vec!["336\n"]);
}

#[test]
fn test_as_cast_int_to_float() {
    let output = run_and_capture(r#"fn main() { let x = 42 as float; println!("{}", x); }"#);
    assert_eq!(output, vec!["42.0\n"]);
}

#[test]
fn test_as_cast_float_to_int() {
    let output = run_and_capture(r#"fn main() { let x = 3.9 as int; println!("{}", x); }"#);
    assert_eq!(output, vec!["3\n"]);
}

#[test]
fn test_as_cast_char_to_int() {
    let output = run_and_capture(r#"fn main() { let x = 'a' as int; println!("{}", x); }"#);
    assert_eq!(output, vec!["97\n"]);
}

#[test]
fn test_as_cast_int_to_char() {
    let output = run_and_capture(r#"fn main() { let x = 65 as char; println!("{}", x); }"#);
    assert_eq!(output, vec!["A\n"]);
}
