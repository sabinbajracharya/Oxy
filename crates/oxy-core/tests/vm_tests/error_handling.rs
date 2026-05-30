//! Result/Option, `?`, panics, parsing failures.
//!
//! Shared helpers and imports come from the parent module (`use super::*`).

use super::*;

#[test]
fn test_option_some_none() {
    let out = run_and_capture(
        r#"
fn main() {
    let x = Some(42);
    let y = None;
    println!("{:?} {:?}", x, y);
}
"#,
    );
    assert_eq!(out, vec!["Some(42) None\n"]);
}

#[test]
fn test_option_unwrap() {
    let out = run_and_capture(
        r#"
fn main() {
    let x = Some(42);
    println!("{}", x.unwrap());
}
"#,
    );
    assert_eq!(out, vec!["42\n"]);
}

#[test]
fn test_option_is_some_is_none() {
    let out = run_and_capture(
        r#"
fn main() {
    let x = Some(42);
    let y = None;
    println!("{} {} {} {}", x.is_some(), x.is_none(), y.is_some(), y.is_none());
}
"#,
    );
    assert_eq!(out, vec!["true false false true\n"]);
}

#[test]
fn test_option_unwrap_or() {
    let out = run_and_capture(
        r#"
fn main() {
    let x = Some(42);
    let y = None;
    println!("{} {}", x.unwrap_or(0), y.unwrap_or(0));
}
"#,
    );
    assert_eq!(out, vec!["42 0\n"]);
}

#[test]
fn test_result_ok_err() {
    let out = run_and_capture(
        r#"
fn main() {
    let x = Ok(42);
    let y = Err("failed");
    println!("{:?} {:?}", x, y);
}
"#,
    );
    assert_eq!(out, vec!["Ok(42) Err(\"failed\")\n"]);
}

#[test]
fn test_result_unwrap() {
    let out = run_and_capture(
        r#"
fn main() {
    let x = Ok(42);
    println!("{}", x.unwrap());
}
"#,
    );
    assert_eq!(out, vec!["42\n"]);
}

#[test]
fn test_result_is_ok_is_err() {
    let out = run_and_capture(
        r#"
fn main() {
    let x = Ok(42);
    let y = Err("oops");
    println!("{} {} {} {}", x.is_ok(), x.is_err(), y.is_ok(), y.is_err());
}
"#,
    );
    assert_eq!(out, vec!["true false false true\n"]);
}

#[test]
fn test_result_unwrap_or() {
    let out = run_and_capture(
        r#"
fn main() {
    let x = Ok(42);
    let y = Err("oops");
    println!("{} {}", x.unwrap_or(0), y.unwrap_or(0));
}
"#,
    );
    assert_eq!(out, vec!["42 0\n"]);
}

#[test]
fn test_result_unwrap_err() {
    let out = run_and_capture(
        r#"
fn main() {
    let y = Err("oops");
    println!("{}", y.unwrap_err());
}
"#,
    );
    assert_eq!(out, vec!["oops\n"]);
}

#[test]
fn test_if_let_some() {
    let out = run_and_capture(
        r#"
fn main() {
    let x = Some(42);
    if let Some(val) = x {
        println!("got {}", val);
    } else {
        println!("nothing");
    }
}
"#,
    );
    assert_eq!(out, vec!["got 42\n"]);
}

#[test]
fn test_if_let_none() {
    let out = run_and_capture(
        r#"
fn main() {
    let x = None;
    if let Some(val) = x {
        println!("got {}", val);
    } else {
        println!("nothing");
    }
}
"#,
    );
    assert_eq!(out, vec!["nothing\n"]);
}

#[test]
fn test_while_let() {
    let out = run_and_capture(
        r#"
fn main() {
    let mut v = vec![1, 2, 3];
    while let Some(val) = v.pop() {
        println!("{}", val);
    }
}
"#,
    );
    assert_eq!(out, vec!["3\n", "2\n", "1\n"]);
}

#[test]
fn test_match_option() {
    let out = run_and_capture(
        r#"
fn main() {
    let x = Some(42);
    match x {
        Some(val) => println!("value: {}", val),
        None => println!("nothing"),
    }
}
"#,
    );
    assert_eq!(out, vec!["value: 42\n"]);
}

#[test]
fn test_match_result() {
    let out = run_and_capture(
        r#"
fn main() {
    let x = Err("problem");
    match x {
        Ok(val) => println!("ok: {}", val),
        Err(e) => println!("err: {}", e),
    }
}
"#,
    );
    assert_eq!(out, vec!["err: problem\n"]);
}

#[test]
fn test_try_operator_ok() {
    let out = run_and_capture(
        r#"
fn parse_num(s: String) -> Result {
    if s == "42" {
        Ok(42)
    } else {
        Err("parse error")
    }
}

fn do_work() -> Result {
    let n = parse_num("42")?;
    Ok(n + 1)
}

fn main() {
    let result = do_work();
    println!("{:?}", result);
}
"#,
    );
    assert_eq!(out, vec!["Ok(43)\n"]);
}

#[test]
fn test_try_operator_err() {
    let out = run_and_capture(
        r#"
fn parse_num(s: String) -> Result {
    if s == "42" {
        Ok(42)
    } else {
        Err("parse error")
    }
}

fn do_work() -> Result {
    let n = parse_num("bad")?;
    Ok(n + 1)
}

fn main() {
    let result = do_work();
    println!("{:?}", result);
}
"#,
    );
    assert_eq!(out, vec!["Err(\"parse error\")\n"]);
}

#[test]
fn test_panic_macro() {
    let src = r#"
fn main() {
    panic!("something went wrong");
}
"#;
    let result = run(src);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("something went wrong"));
}

#[test]
fn test_dbg_macro() {
    let out = run_and_capture(
        r#"
fn main() {
    let x = dbg!(42);
    println!("{}", x);
}
"#,
    );
    assert_eq!(out, vec!["42\n", "42\n"]); // dbg! prints value and returns it
}

#[test]
fn test_option_map() {
    let out = run_and_capture(
        r#"
fn double(x: int) -> int { x * 2 }

fn main() {
    let x = Some(21);
    let y = x.map(double);
    println!("{:?}", y);

    let z = None;
    let w = z.map(double);
    println!("{:?}", w);
}
"#,
    );
    assert_eq!(out, vec!["Some(42)\n", "None\n"]);
}

#[test]
fn test_result_map() {
    let out = run_and_capture(
        r#"
fn double(x: int) -> int { x * 2 }

fn main() {
    let x = Ok(21);
    let y = x.map(double);
    println!("{:?}", y);
}
"#,
    );
    assert_eq!(out, vec!["Ok(42)\n"]);
}

#[test]
fn test_result_ok_to_option() {
    let out = run_and_capture(
        r#"
fn main() {
    let x = Ok(42);
    let y = Err("bad");
    println!("{:?} {:?}", x.ok(), y.ok());
}
"#,
    );
    assert_eq!(out, vec!["Some(42) None\n"]);
}

#[test]
fn test_if_let_result() {
    let out = run_and_capture(
        r#"
fn main() {
    let x = Ok(42);
    if let Ok(val) = x {
        println!("ok: {}", val);
    } else {
        println!("err");
    }
}
"#,
    );
    assert_eq!(out, vec!["ok: 42\n"]);
}

#[test]
fn test_option_function_return() {
    let out = run_and_capture(
        r#"
fn find_item(items: Vec, target: int) -> Option {
    for i in 0..items.len() {
        if items[i] == target {
            return Some(i);
        }
    }
    None
}

fn main() {
    let items = vec![10, 20, 30, 40];
    let result = find_item(items, 30);
    match result {
        Some(idx) => println!("found at {}", idx),
        None => println!("not found"),
    }
}
"#,
    );
    assert_eq!(out, vec!["found at 2\n"]);
}

#[test]
fn test_try_operator_option() {
    let out = run_and_capture(
        r#"
fn get_first(v: Vec) -> Option {
    if v.is_empty() {
        None
    } else {
        Some(v[0])
    }
}

fn process() -> Option {
    let v = vec![10, 20, 30];
    let first = get_first(v)?;
    Some(first * 2)
}

fn main() {
    let result = process();
    println!("{:?}", result);
}
"#,
    );
    assert_eq!(out, vec!["Some(20)\n"]);
}

#[test]
fn test_option_unwrap_none_panics() {
    let src = r#"
fn main() {
    let x = None;
    x.unwrap();
}
"#;
    let result = run(src);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("called `unwrap` on a `None` value"));
}

#[test]
fn test_result_unwrap_err_panics() {
    let src = r#"
fn main() {
    let x = Err("bad");
    x.unwrap();
}
"#;
    let result = run(src);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("called `unwrap` on an `Err` value"));
}

#[test]
fn test_int_parse() {
    let output = run_and_capture(
        r#"
fn main() {
    let r = int::parse("42");
    println!("{}", r.unwrap());
}
"#,
    );
    assert_eq!(output, vec!["42\n"]);
}

#[test]
fn test_int_parse_invalid() {
    let output = run_and_capture(
        r#"
fn main() {
    let r = int::parse("abc");
    println!("{}", r.is_err());
}
"#,
    );
    assert_eq!(output, vec!["true\n"]);
}

#[test]
fn test_float_parse() {
    let output = run_and_capture(
        r#"
fn main() {
    let r = float::parse("3.14");
    println!("{}", r.unwrap());
}
"#,
    );
    assert_eq!(output, vec!["3.14\n"]);
}

#[test]
fn test_int_parse_hex() {
    let output = run_and_capture(
        r#"
fn main() {
    println!("{}", int::parse("0xFF").unwrap());
    println!("{}", int::parse("0x10").unwrap());
}
"#,
    );
    assert_eq!(output, vec!["255\n", "16\n"]);
}

#[test]
fn test_string_parse_int_method() {
    let output = run_and_capture(
        r#"
fn main() {
    let r = "42".parse_int();
    println!("{}", r.unwrap());
}
"#,
    );
    assert_eq!(output, vec!["42\n"]);
}

#[test]
fn test_string_parse_float_method() {
    let output = run_and_capture(
        r#"
fn main() {
    let r = "3.14".parse_float();
    println!("{}", r.unwrap());
}
"#,
    );
    assert_eq!(output, vec!["3.14\n"]);
}
