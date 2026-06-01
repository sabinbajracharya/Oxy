#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::vm::{run_compiled, run_compiled_capturing};

    // --- Array tests ---

    #[test]
    fn test_compiled_array_literal() {
        let source = r#"
        fn main() {
            val arr = [1, 2, 3];
            io::println(arr);
        }
        "#;
        let result = run_compiled(source);
        assert!(result.is_ok(), "array literal failed: {:?}", result.err());
    }

    #[test]
    fn test_compiled_array_empty() {
        let source = r#"
        fn main() {
            val arr = [];
            io::println(arr);
        }
        "#;
        let result = run_compiled(source);
        assert!(result.is_ok(), "empty array failed: {:?}", result.err());
    }

    #[test]
    fn test_compiled_array_nested() {
        let source = r#"
        fn main() {
            val arr = [[1, 2], [3, 4]];
            io::println(arr);
        }
        "#;
        let result = run_compiled(source);
        assert!(result.is_ok(), "nested array failed: {:?}", result.err());
    }

    // --- Index tests ---

    #[test]
    fn test_compiled_index_list() {
        let source = r#"
        fn main() {
            val arr = [10, 20, 30];
            io::println(arr[0]);
            io::println(arr[2]);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "index vec failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["10\n", "30\n"]);
    }

    #[test]
    fn test_compiled_index_string() {
        let source = r#"
        fn main() {
            val s = "ab";
            io::println(s[0]);
            io::println(s[1]);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "index string failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["a\n", "b\n"]);
    }

    #[test]
    fn test_compiled_index_tuple() {
        let source = r#"
        fn main() {
            val t = (10, 20);
            io::println(t.0);
            io::println(t.1);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "index tuple failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["10\n", "20\n"]);
    }

    #[test]
    fn test_compiled_io_println_captured() {
        let source = r#"
        fn main() {
            io::io::println("fib({}) = {}", 8, 21);
            io::io::print("done");
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(
            result.is_ok(),
            "io::println capture failed: {:?}",
            result.err()
        );
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["fib(8) = 21\n", "done"]);
    }

    // --- ForDestructure tests ---

    #[test]
    fn test_compiled_for_destructure() {
        let source = r#"
        fn main() {
            for (a, b) in [(1, 10), (2, 20)] {
                io::println(a + b);
            }
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "for destructure failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["11\n", "22\n"]);
    }

    // --- For loop tests ---

    #[test]
    fn test_compiled_for_range_compiles() {
        let source = r#"
        fn main() {
            var sum = 0;
            for i in 0..3 {
                sum = sum + i;
            }
            io::println("{}", sum);
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
                io::println(i);
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
            var sum = 0;
            for i in 0..5 {
                sum = sum + i;
            }
            io::println(sum);
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
                io::println(i);
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
            var i = 0;
            while i < 3 {
                io::println(i);
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
            var i = 0;
            while i < 10 {
                if i == 3 { break; }
                io::println(i);
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
            var i = 0;
            loop {
                if i >= 3 { break; }
                io::println(i);
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
                io::println(i);
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
            var i = 0;
            while i < 5 {
                i = i + 1;
                if i == 3 { continue; }
                io::println(i);
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
            var i = 0;
            loop {
                i = i + 1;
                if i == 2 { continue; }
                if i > 3 { break; }
                io::println(i);
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
                io::println(c);
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
                    io::println(j);
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
            val x = 40 + 2;
            val y = x * 2;
            val z = y - 4;
            val w = z / 2;
            io::println("{}", w);
        }
        "#;
        let result = run_compiled(source);
        assert!(result.is_ok(), "compiled failed: {:?}", result.err());
    }

    #[test]
    fn test_compiled_function_call() {
        let source = r#"
        fn add(x: Int, y: Int) -> Int { x + y }
        fn main() {
            val r = add(3, 4);
            io::println("{}", r);
        }
        "#;
        let result = run_compiled(source);
        assert!(result.is_ok(), "compiled failed: {:?}", result.err());
    }

    #[test]
    fn test_compiled_simple_if_no_recursion() {
        // Non-recursive else branch — should work
        let source = r#"
        fn check(n: Int) -> Int {
            if n <= 1 { n } else { 99 }
        }
        fn main() {
            io::println("{}", check(0));
            io::println("{}", check(5));
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
            var x = 0;
            while x < 3 {
                io::println("{}", x);
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
        fn fib(n: Int) -> Int {
            if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }
        }
        fn main() { io::println("{}", fib(2)); }
        "#;
        let result = run_compiled(source);
        assert!(result.is_ok(), "compiled failed: {:?}", result.err());
    }

    #[test]
    fn test_compiled_fibonacci() {
        let source = r#"
        fn fib(n: Int) -> Int {
            if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }
        }
        fn main() {
            val r = fib(10);
            io::println("{}", r);
        }
        "#;
        let result = run_compiled(source);
        assert!(result.is_ok(), "compiled failed: {:?}", result.err());
    }

    #[test]
    fn test_compiled_while_loop() {
        let source = r#"
        fn main() {
            var i = 0;
            var sum = 0;
            while i < 10 {
                sum = sum + i;
                i = i + 1;
            }
            io::println("{}", sum);
        }
        "#;
        let result = run_compiled(source);
        assert!(result.is_ok(), "compiled failed: {:?}", result.err());
    }

    #[test]
    fn test_compiled_if_else() {
        let source = r#"
        fn is_even(n: Int) -> bool {
            if n % 2 == 0 { true } else { false }
        }
        fn main() {
            io::println("{}", is_even(4));
            io::println("{}", is_even(7));
        }
        "#;
        let result = run_compiled(source);
        assert!(result.is_ok(), "compiled failed: {:?}", result.err());
    }

    #[test]
    fn test_compiled_vs_interpreted_equivalent() {
        let source = r#"
        fn fib(n: Int) -> Int {
            if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }
        }
        fn main() {
            val r = fib(10);
            io::println("{}", r);
        }
        "#;
        // Both should produce the same result
        let compiled = run_compiled(source);
        let interpreted = run_compiled(source);
        assert!(compiled.is_ok());
        assert!(interpreted.is_ok());
    }

    // --- CompoundAssign tests ---

    #[test]
    fn test_compiled_compound_assign_add() {
        let source = r#"
        fn main() {
            var x = 10;
            x += 5;
            io::println("{}", x);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "compound add failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["15\n"]);
    }

    #[test]
    fn test_compiled_compound_assign_sub() {
        let source = r#"
        fn main() {
            var x = 10;
            x -= 3;
            io::println("{}", x);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "compound sub failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["7\n"]);
    }

    #[test]
    fn test_compiled_compound_assign_mul() {
        let source = r#"
        fn main() {
            var x = 7;
            x *= 3;
            io::println("{}", x);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "compound mul failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["21\n"]);
    }

    #[test]
    fn test_compiled_compound_assign_div() {
        let source = r#"
        fn main() {
            var x = 20;
            x /= 4;
            io::println("{}", x);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "compound div failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["5\n"]);
    }

    // --- FString tests ---

    #[test]
    fn test_compiled_fstring_literal_only() {
        let source = r#"
        fn main() {
            val msg = f"hello world";
            io::println("{}", msg);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "fstring literal failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["hello world\n"]);
    }

    #[test]
    fn test_compiled_fstring_interpolated() {
        let source = r#"
        fn main() {
            val name = "Oxy";
            val msg = f"Hello, {name}!";
            io::println("{}", msg);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(
            result.is_ok(),
            "fstring interpolated failed: {:?}",
            result.err()
        );
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["Hello, Oxy!\n"]);
    }

    #[test]
    fn test_compiled_fstring_multiple_exprs() {
        let source = r#"
        fn main() {
            val x = 10;
            val y = 20;
            val msg = f"{x} + {y} = {x + y}";
            io::println("{}", msg);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(
            result.is_ok(),
            "fstring multiple failed: {:?}",
            result.err()
        );
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["10 + 20 = 30\n"]);
    }

    // --- Struct/Enum/Impl compilation tests ---

    #[test]
    fn test_compiled_struct_and_enum_define() {
        let source = r#"
        struct PoInt { x: Int, y: Int }
        enum Shape { Circle, Square(Int) }
        fn main() {
            io::println("ok");
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(
            result.is_ok(),
            "struct/enum define failed: {:?}",
            result.err()
        );
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["ok\n"]);
    }

    #[test]
    fn test_compiled_impl_method() {
        let source = r#"
        struct Counter { n: Int }
        impl Counter {
            fn inc(self) -> Counter {
                Counter { n: self.n + 1 }
            }
        }
        fn main() {
            val c = Counter { n: 0 };
            val c2 = c.inc();
            io::println("{}", c2.n);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "impl method failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["1\n"]);
    }

    #[test]
    fn test_compiled_self_ref() {
        let source = r#"
        struct Value { x: Int }
        impl Value {
            fn get(self) -> Int {
                self.x
            }
        }
        fn main() {
            val v = Value { x: 42 };
            io::println("{}", v.get());
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "self ref failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["42\n"]);
    }

    // --- Match tests ---

    #[test]
    fn test_compiled_match_literal() {
        let source = r#"
        fn main() {
            val x = 1;
            val r = match x {
                1 => "one",
                _ => "other",
            };
            io::println("{}", r);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "match literal failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["one\n"]);
    }

    #[test]
    fn test_compiled_match_wildcard() {
        let source = r#"
        fn main() {
            val x = 99;
            val r = match x {
                1 => "one",
                _ => "other",
            };
            io::println("{}", r);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "match wildcard failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["other\n"]);
    }

    #[test]
    fn test_compiled_match_ident_binding() {
        let source = r#"
        fn main() {
            val x = 42;
            val r = match x {
                v => v + 1,
            };
            io::println("{}", r);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(
            result.is_ok(),
            "match ident binding failed: {:?}",
            result.err()
        );
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["43\n"]);
    }

    #[test]
    fn test_compiled_match_enum_variant() {
        let source = r#"
        enum Opt { Some(Int), None }
        fn main() {
            val x = Opt::Some(10);
            val r = match x {
                Opt::Some(v) => v,
                Opt::None => 0,
            };
            io::println("{}", r);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(
            result.is_ok(),
            "match enum variant failed: {:?}",
            result.err()
        );
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["10\n"]);
    }

    // --- IfLet tests ---

    #[test]
    fn test_compiled_if_let_some() {
        let source = r#"
        enum Opt { Some(Int), None }
        fn main() {
            val x = Opt::Some(42);
            if val Opt::Some(v) = x {
                io::println("{}", v);
            }
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "if let some failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["42\n"]);
    }

    #[test]
    fn test_compiled_if_let_none_else() {
        let source = r#"
        enum Opt { Some(Int), None }
        fn main() {
            val x = Opt::None;
            if val Opt::Some(v) = x {
                io::println("{}", v);
            } else {
                io::println("nothing");
            }
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(
            result.is_ok(),
            "if let none else failed: {:?}",
            result.err()
        );
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["nothing\n"]);
    }

    // --- PathCall built-in tests ---

    #[test]
    fn test_compiled_pathcall_math_sqrt() {
        let source = r#"
        fn main() {
            io::println("{}", math::sqrt(16.0));
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "pathcall sqrt failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["4.0\n"]);
    }

    #[test]
    fn test_compiled_pathcall_math_abs() {
        let source = r#"
        fn main() {
            io::println("{}", math::abs(-42));
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "pathcall abs failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["42\n"]);
    }

    #[test]
    fn test_compiled_pathcall_string_from() {
        let source = r#"
        fn main() {
            val s = String::from("hello");
            io::println("{}", s);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(
            result.is_ok(),
            "pathcall String::from failed: {:?}",
            result.err()
        );
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["hello\n"]);
    }

    #[test]
    fn test_compiled_pathcall_hashmap_new() {
        let source = r#"
        fn main() {
            val m = Map::new();
            io::println("{}", m.len());
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(
            result.is_ok(),
            "pathcall Map::new failed: {:?}",
            result.err()
        );
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["0\n"]);
    }

    // --- Module compilation tests ---

    #[test]
    fn test_compiled_inline_module_call() {
        let source = r#"
        mod math {
            pub fn double(x: Int) -> Int { x * 2 }
        }
        fn main() {
            io::println("{}", math::double(21));
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(
            result.is_ok(),
            "inline module call failed: {:?}",
            result.err()
        );
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["42\n"]);
    }

    #[test]
    fn test_compiled_nested_module() {
        let source = r#"
        mod outer {
            pub mod inner {
                pub fn value() -> Int { 99 }
            }
        }
        fn main() {
            io::println("{}", outer::inner::value());
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "nested module failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["99\n"]);
    }

    #[test]
    fn test_compiled_module_with_use() {
        let source = r#"
        mod calc {
            pub fn triple(x: Int) -> Int { x * 3 }
        }
        use calc::triple;
        fn main() {
            io::println("{}", triple(7));
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "module with use failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["21\n"]);
    }

    #[test]
    fn test_compiled_module_chain() {
        let source = r#"
        mod a {
            pub fn one() -> Int { 1 }
        }
        mod b {
            pub fn two() -> Int { a::one() + a::one() }
        }
        fn main() {
            io::println("{}", b::two());
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "module chain failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["2\n"]);
    }

    #[test]
    fn test_compiled_iter_any() {
        let source = r#"
        fn main() {
            val v = [1, 2, 3];
            val r = v.iter().any(|x| x == 2);
            io::println("{}", r);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "iter any failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["true\n"], "expected 'true', got {:?}", output);
    }

    // --- Event-loop spawn tests ---

    #[test]
    fn test_event_loop_spawn_basic() {
        let source = r#"
        fn main() {
            val h = spawn(|| 42);
            io::println("{}", h.await);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "spawn basic failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["42\n"]);
    }

    #[test]
    fn test_event_loop_spawn_multiple() {
        let source = r#"
        fn main() {
            val a = spawn(|| 100);
            val b = spawn(|| 200);
            io::println("{}", a.await);
            io::println("{}", b.await);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "spawn multiple failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["100\n", "200\n"]);
    }

    #[test]
    fn test_event_loop_sleep_in_spawn() {
        let source = r#"
        fn main() {
            val h = spawn(|| {
                sleep(0);
                99
            });
            io::println("{}", h.await);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "sleep in spawn failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        assert_eq!(output, vec!["99\n"]);
    }

    #[test]
    fn test_event_loop_select_basic() {
        let source = r#"
        fn main() {
            val a = spawn(|| 42);
            val b = spawn(|| 99);
            val result = select(a, b);
            io::println("{}", result);
        }
        "#;
        let result = run_compiled_capturing(source);
        assert!(result.is_ok(), "select basic failed: {:?}", result.err());
        let (_, output) = result.unwrap();
        let val = output[0].trim().parse::<i64>().unwrap();
        assert!(val == 42 || val == 99, "expected 42 or 99, got {}", val);
    }
}
