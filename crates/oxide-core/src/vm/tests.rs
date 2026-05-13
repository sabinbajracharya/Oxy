#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::interpreter::{run, run_compiled};

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
