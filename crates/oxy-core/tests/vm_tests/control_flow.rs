//! if/else, match, loops, labeled break/continue, range patterns.
//!
//! Shared helpers and imports come from the parent module (`use super::*`).

use super::*;

#[test]
fn test_if_true() {
    let output = run_and_capture(r#"fn main() { if true { println("yes"); } }"#);
    assert_eq!(output, vec!["yes\n"]);
}

#[test]
fn test_if_false() {
    let output = run_and_capture(r#"fn main() { if false { println("yes"); } }"#);
    assert!(output.is_empty());
}

#[test]
fn test_if_else() {
    let output =
        run_and_capture(r#"fn main() { let x = if true { 1 } else { 2 }; println("{}", x); }"#);
    assert_eq!(output, vec!["1\n"]);
}

#[test]
fn test_if_else_if() {
    let output = run_and_capture(
        r#"
fn classify(x: int) -> int {
    if x > 0 {
        1
    } else if x < 0 {
        -1
    } else {
        0
    }
}

fn main() {
    println("{} {} {}", classify(5), classify(-3), classify(0));
}
"#,
    );
    assert_eq!(output, vec!["1 -1 0\n"]);
}

#[test]
fn test_while_loop() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut i = 0;
    let mut sum = 0;
    while i < 5 {
        sum += i;
        i += 1;
    }
    println("{}", sum);
}
"#,
    );
    assert_eq!(output, vec!["10\n"]);
}

#[test]
fn test_while_false() {
    let output =
        run_and_capture(r#"fn main() { while false { println("never"); } println("done"); }"#);
    assert_eq!(output, vec!["done\n"]);
}

#[test]
fn test_loop_with_break() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut i = 0;
    loop {
        if i >= 3 {
            break;
        }
        println("{}", i);
        i += 1;
    }
}
"#,
    );
    assert_eq!(output, vec!["0\n", "1\n", "2\n"]);
}

#[test]
fn test_loop_break_value() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut i = 0;
    let result = loop {
        i += 1;
        if i == 5 {
            break i * 10;
        }
    };
    println("{}", result);
}
"#,
    );
    assert_eq!(output, vec!["50\n"]);
}

#[test]
fn test_continue_in_while() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut i = 0;
    while i < 5 {
        i += 1;
        if i == 3 {
            continue;
        }
        println("{}", i);
    }
}
"#,
    );
    assert_eq!(output, vec!["1\n", "2\n", "4\n", "5\n"]);
}

#[test]
fn test_for_range() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut sum = 0;
    for i in 0..5 {
        sum += i;
    }
    println("{}", sum);
}
"#,
    );
    assert_eq!(output, vec!["10\n"]);
}

#[test]
fn test_for_range_inclusive() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut sum = 0;
    for i in 0..=5 {
        sum += i;
    }
    println("{}", sum);
}
"#,
    );
    assert_eq!(output, vec!["15\n"]);
}

#[test]
fn test_for_with_break() {
    let output = run_and_capture(
        r#"
fn main() {
    for i in 0..10 {
        if i == 3 {
            break;
        }
        println("{}", i);
    }
}
"#,
    );
    assert_eq!(output, vec!["0\n", "1\n", "2\n"]);
}

#[test]
fn test_for_with_continue() {
    let output = run_and_capture(
        r#"
fn main() {
    for i in 0..5 {
        if i % 2 == 0 {
            continue;
        }
        println("{}", i);
    }
}
"#,
    );
    assert_eq!(output, vec!["1\n", "3\n"]);
}

#[test]
fn test_match_literals() {
    let output = run_and_capture(
        r#"
fn main() {
    let x = 2;
    let result = match x {
        1 => "one",
        2 => "two",
        3 => "three",
        _ => "other",
    };
    println("{}", result);
}
"#,
    );
    assert_eq!(output, vec!["two\n"]);
}

#[test]
fn test_match_wildcard() {
    let output = run_and_capture(
        r#"
fn main() {
    let x = 99;
    let result = match x {
        1 => "one",
        _ => "other",
    };
    println("{}", result);
}
"#,
    );
    assert_eq!(output, vec!["other\n"]);
}

#[test]
fn test_match_with_blocks() {
    let output = run_and_capture(
        r#"
fn main() {
    let x = 1;
    match x {
        1 => {
            println("it's one!");
        }
        _ => {
            println("something else");
        }
    }
}
"#,
    );
    assert_eq!(output, vec!["it's one!\n"]);
}

#[test]
fn test_match_string() {
    let output = run_and_capture(
        r#"
fn main() {
    let cmd = "hello";
    let result = match cmd {
        "hello" => "greeting",
        "bye" => "farewell",
        _ => "unknown",
    };
    println("{}", result);
}
"#,
    );
    assert_eq!(output, vec!["greeting\n"]);
}

#[test]
fn test_match_bool() {
    let output = run_and_capture(
        r#"
fn main() {
    let x = true;
    let s = match x {
        true => "yes",
        false => "no",
    };
    println("{}", s);
}
"#,
    );
    assert_eq!(output, vec!["yes\n"]);
}

#[test]
fn test_match_variable_binding() {
    let output = run_and_capture(
        r#"
fn main() {
    let x = 42;
    let result = match x {
        n => n + 1,
    };
    println("{}", result);
}
"#,
    );
    assert_eq!(output, vec!["43\n"]);
}

#[test]
fn test_match_non_exhaustive_error() {
    let result = run_compiled(
        r#"
fn main() {
    let x = 5;
    match x {
        1 => "one",
        2 => "two",
    };
}
"#,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("non-exhaustive"));
}

#[test]
fn test_nested_loops() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut count = 0;
    for i in 0..3 {
        for j in 0..3 {
            count += 1;
        }
    }
    println("{}", count);
}
"#,
    );
    assert_eq!(output, vec!["9\n"]);
}

#[test]
fn test_loop_in_function() {
    let output = run_and_capture(
        r#"
fn find_first_multiple(n: int, target: int) -> int {
    let mut i = 1;
    loop {
        if i * n >= target {
            return i * n;
        }
        i += 1;
    }
}

fn main() {
    println("{}", find_first_multiple(7, 50));
}
"#,
    );
    assert_eq!(output, vec!["56\n"]);
}

#[test]
fn test_fizzbuzz() {
    let output = run_and_capture(
        r#"
fn main() {
    for i in 1..=15 {
        if i % 15 == 0 {
            println("FizzBuzz");
        } else if i % 3 == 0 {
            println("Fizz");
        } else if i % 5 == 0 {
            println("Buzz");
        } else {
            println("{}", i);
        }
    }
}
"#,
    );
    assert_eq!(
        output,
        vec![
            "1\n",
            "2\n",
            "Fizz\n",
            "4\n",
            "Buzz\n",
            "Fizz\n",
            "7\n",
            "8\n",
            "Fizz\n",
            "Buzz\n",
            "11\n",
            "Fizz\n",
            "13\n",
            "14\n",
            "FizzBuzz\n"
        ]
    );
}

#[test]
fn test_labeled_break_outer() {
    let output = run_and_capture(
        r#"
            fn main() {
                let mut i = 0;
                'outer: loop {
                    i = i + 1;
                    if i > 10 {
                        break 'outer;
                    }
                }
                println("{}", i);
            }
            "#,
    );
    assert_eq!(output, vec!["11\n"]);
}

#[test]
fn test_labeled_break_nested() {
    let output = run_and_capture(
        r#"
            fn main() {
                let mut count = 0;
                'outer: for x in 0..5 {
                    for y in 0..5 {
                        if x == 2 && y == 2 {
                            break 'outer;
                        }
                        count = count + 1;
                    }
                }
                println("{}", count);
            }
            "#,
    );
    assert_eq!(output, vec!["12\n"]);
}

#[test]
fn test_labeled_continue_outer() {
    let output = run_and_capture(
        r#"
            fn main() {
                let mut result = 0;
                'outer: for x in 0..5 {
                    for y in 0..5 {
                        if y == 2 {
                            continue 'outer;
                        }
                        result = result + 1;
                    }
                }
                println("{}", result);
            }
            "#,
    );
    // Each outer iteration skips inner loop after y=2 check,
    // so only y=0,1 contribute per outer iteration: 5 * 2 = 10
    assert_eq!(output, vec!["10\n"]);
}

#[test]
fn test_match_range_inclusive() {
    let output = run_and_capture(
        r#"
            fn main() {
                let x = 5;
                let result = match x {
                    1..=3 => "low",
                    4..=7 => "mid",
                    _ => "other",
                };
                println("{}", result);
            }
            "#,
    );
    assert_eq!(output, vec!["mid\n"]);
}

#[test]
fn test_match_range_exclusive() {
    let output = run_and_capture(
        r#"
            fn main() {
                let x = 3;
                let result = match x {
                    1..5 => "yes",
                    _ => "no",
                };
                println("{}", result);
            }
            "#,
    );
    assert_eq!(output, vec!["yes\n"]);
}
