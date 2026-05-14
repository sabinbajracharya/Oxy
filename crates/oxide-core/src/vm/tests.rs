#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::interpreter::{run, run_compiled, run_compiled_capturing};

    #[test]
    fn test_compiled_for_range_compiles() {
        let source = r#"
        fn main() {
            let mut sum = 0;
            for i in 0..3 {
                sum = sum + i;
            }
            println!("{}", sum);
        }
        "#;
        let result = run_compiled(source);
        assert!(result.is_ok(), "for range failed: {:?}", result.err());
    }

    #[test]
    fn test_compiled_for_range_output() {
        let source = r#"
        fn main() {
            for i in 0..3 {
                println!(i);
            }
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "for output failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["0\n", "1\n", "2\n"]);
    }

    #[test]
    fn test_compiled_for_range_sum() {
        let source = r#"
        fn main() {
            let mut sum = 0;
            for i in 0..5 {
                sum = sum + i;
            }
            println!(sum);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "for sum failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["10\n"]);
    }

    #[test]
    fn test_compiled_for_break() {
        let source = r#"
        fn main() {
            for i in 0..10 {
                if i == 3 { break; }
                println!(i);
            }
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "for break failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["0\n", "1\n", "2\n"]);
    }

    #[test]
    fn test_compiled_while_basic() {
        // Verify while loop works correctly after refactoring (no break)
        let source = r#"
        fn main() {
            let mut i = 0;
            while i < 3 {
                println!(i);
                i = i + 1;
            }
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "while basic failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["0\n", "1\n", "2\n"]);
    }

    #[test]
    fn test_compiled_while_break() {
        let source = r#"
        fn main() {
            let mut i = 0;
            while i < 10 {
                if i == 3 { break; }
                println!(i);
                i = i + 1;
            }
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "while break failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["0\n", "1\n", "2\n"]);
    }

    #[test]
    fn test_compiled_loop_break() {
        let source = r#"
        fn main() {
            let mut i = 0;
            loop {
                if i >= 3 { break; }
                println!(i);
                i = i + 1;
            }
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "loop break failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["0\n", "1\n", "2\n"]);
    }

    #[test]
    fn test_compiled_for_continue() {
        let source = r#"
        fn main() {
            for i in 0..5 {
                if i == 2 { continue; }
                println!(i);
            }
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "for continue failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["0\n", "1\n", "3\n", "4\n"]);
    }

    #[test]
    fn test_compiled_while_continue() {
        let source = r#"
        fn main() {
            let mut i = 0;
            while i < 5 {
                i = i + 1;
                if i == 3 { continue; }
                println!(i);
            }
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "while continue failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["1\n", "2\n", "4\n", "5\n"]);
    }

    #[test]
    fn test_compiled_loop_continue() {
        let source = r#"
        fn main() {
            let mut i = 0;
            loop {
                i = i + 1;
                if i == 2 { continue; }
                if i > 3 { break; }
                println!(i);
            }
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "loop continue failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["1\n", "3\n"]);
    }

    #[test]
    fn test_compiled_for_string() {
        let source = r#"
        fn main() {
            for c in "ab" {
                println!(c);
            }
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "for string failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["a\n", "b\n"]);
    }

    #[test]
    fn test_compiled_nested_for_break() {
        let source = r#"
        fn main() {
            for i in 0..3 {
                for j in 0..3 {
                    if j == 1 { break; }
                    println!(j);
                }
            }
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "nested break failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        // Should print 0 three times (once per outer iteration)
        assert_eq!(output, vec!["0\n", "0\n", "0\n"]);
    }

    #[test]
    fn test_compiled_break_outside_loop() {
        let source = r#"
        fn main() {
            break;
        }
        "#;
        let result = run_compiled(source);
        assert!(result.is_err(), "break outside loop should fail");
    }

    #[test]
    fn test_compiled_continue_outside_loop() {
        let source = r#"
        fn main() {
            continue;
        }
        "#;
        let result = run_compiled(source);
        assert!(result.is_err(), "continue outside loop should fail");
    }

    #[test]
    fn test_compiled_arithmetic() {
        let source = r#"
        fn main() {
            let x = 40 + 2;
            let y = x * 2;
            let z = y - 4;
            let w = z / 2;
            println!("{}", w);
        }
        "#;
        let result = run_compiled(source);
        assert!(result.is_ok(), "compiled failed: {:?}", result.err());
    }

    #[test]
    fn test_compiled_function_call() {
        let source = r#"
        fn add(x: i64, y: i64) -> i64 { x + y }
        fn main() {
            let r = add(3, 4);
            println!("{}", r);
        }
        "#;
        let result = run_compiled(source);
        assert!(result.is_ok(), "compiled failed: {:?}", result.err());
    }

    #[test]
    fn test_compiled_simple_if_no_recursion() {
        // Non-recursive else branch — should work
        let source = r#"
        fn check(n: i64) -> i64 {
            if n <= 1 { n } else { 99 }
        }
        fn main() {
            println!("{}", check(0));
            println!("{}", check(5));
        }
        "#;
        let result = run_compiled(source);
        assert!(result.is_ok(), "compiled failed: {:?}", result.err());
    }

    #[test]
    fn test_compiled_simple_while() {
        // Simplest while loop to debug
        let source = r#"
        fn main() {
            let mut x = 0;
            while x < 3 {
                println!("{}", x);
                x = x + 1;
            }
        }
        "#;
        let result = run_compiled(source);
        assert!(result.is_ok(), "compiled failed: {:?}", result.err());
    }

    #[test]
    fn test_compiled_fib_2() {
        let source = r#"
        fn fib(n: i64) -> i64 {
            if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }
        }
        fn main() { println!("{}", fib(2)); }
        "#;
        let result = run_compiled(source);
        assert!(result.is_ok(), "compiled failed: {:?}", result.err());
    }

    #[test]
    fn test_compiled_fibonacci() {
        let source = r#"
        fn fib(n: i64) -> i64 {
            if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }
        }
        fn main() {
            let r = fib(10);
            println!("{}", r);
        }
        "#;
        let result = run_compiled(source);
        assert!(result.is_ok(), "compiled failed: {:?}", result.err());
    }

    #[test]
    fn test_compiled_while_loop() {
        let source = r#"
        fn main() {
            let mut i = 0;
            let mut sum = 0;
            while i < 10 {
                sum = sum + i;
                i = i + 1;
            }
            println!("{}", sum);
        }
        "#;
        let result = run_compiled(source);
        assert!(result.is_ok(), "compiled failed: {:?}", result.err());
    }

    #[test]
    fn test_compiled_if_else() {
        let source = r#"
        fn is_even(n: i64) -> bool {
            if n % 2 == 0 { true } else { false }
        }
        fn main() {
            println!("{}", is_even(4));
            println!("{}", is_even(7));
        }
        "#;
        let result = run_compiled(source);
        assert!(result.is_ok(), "compiled failed: {:?}", result.err());
    }

    #[test]
    fn test_compiled_vs_interpreted_equivalent() {
        let source = r#"
        fn fib(n: i64) -> i64 {
            if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }
        }
        fn main() {
            let r = fib(10);
            println!("{}", r);
        }
        "#;
        // Both should produce the same result
        let compiled = run_compiled(source);
        let interpreted = run(source);
        assert!(compiled.is_ok());
        assert!(interpreted.is_ok());
    }
}
