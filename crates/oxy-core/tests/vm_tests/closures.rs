//! Closures, higher-order functions, iterator chains, captures.
//!
//! Shared helpers and imports come from the parent module (`use super::*`).

use super::*;

#[test]
fn test_closure_basic() {
    let output = run_and_capture(
        r#"fn main() {
let add = |a: int, b: int| a + b;
println!("{}", add(3, 4));
}"#,
    );
    assert_eq!(output, vec!["7\n"]);
}

#[test]
fn test_closure_no_type_annotation() {
    let output = run_and_capture(
        r#"fn main() {
let double = |x| x * 2;
println!("{}", double(5));
}"#,
    );
    assert_eq!(output, vec!["10\n"]);
}

#[test]
fn test_closure_no_params() {
    let output = run_and_capture(
        r#"fn main() {
let greet = || "hello";
println!("{}", greet());
}"#,
    );
    assert_eq!(output, vec!["hello\n"]);
}

#[test]
fn test_closure_block_body() {
    let output = run_and_capture(
        r#"fn main() {
let compute = |x: int| {
    let y = x * 2;
    y + 1
};
println!("{}", compute(10));
}"#,
    );
    assert_eq!(output, vec!["21\n"]);
}

#[test]
fn test_closure_captures_variable() {
    let output = run_and_capture(
        r#"fn main() {
let factor = 3;
let multiply = |x| x * factor;
println!("{}", multiply(5));
}"#,
    );
    assert_eq!(output, vec!["15\n"]);
}

#[test]
fn test_closure_as_argument() {
    let output = run_and_capture(
        r#"fn apply(f: Fn, x: int) -> int {
    f(x)
}
fn main() {
    let result = apply(|x| x * x, 7);
    println!("{}", result);
}"#,
    );
    assert_eq!(output, vec!["49\n"]);
}

#[test]
fn test_closure_returned_from_function() {
    let output = run_and_capture(
        r#"fn make_adder(n: int) -> Fn {
    |x| x + n
}
fn main() {
    let add5 = make_adder(5);
    println!("{}", add5(10));
}"#,
    );
    assert_eq!(output, vec!["15\n"]);
}

#[test]
fn test_closure() {
    let output = run_and_capture(
        r#"fn main() {
let name = "world";
let greet = || format!("hello {}", name);
println!("{}", greet());
}"#,
    );
    assert_eq!(output, vec!["hello world\n"]);
}

#[test]
fn test_vec_map() {
    let output = run_and_capture(
        r#"fn main() {
let v = vec![1, 2, 3];
let doubled = v.map(|x| x * 2).collect();
println!("{:?}", doubled);
}"#,
    );
    assert_eq!(output, vec!["[2, 4, 6]\n"]);
}

#[test]
fn test_vec_filter() {
    let output = run_and_capture(
        r#"fn main() {
let v = vec![1, 2, 3, 4, 5];
let evens = v.filter(|x| x % 2 == 0).collect();
println!("{:?}", evens);
}"#,
    );
    assert_eq!(output, vec!["[2, 4]\n"]);
}

#[test]
fn test_vec_for_each() {
    let output = run_and_capture(
        r#"fn main() {
let v = vec![10, 20, 30];
v.for_each(|x| println!("{}", x));
}"#,
    );
    assert_eq!(output, vec!["10\n", "20\n", "30\n"]);
}

#[test]
fn test_vec_fold() {
    let output = run_and_capture(
        r#"fn main() {
let v = vec![1, 2, 3, 4];
let sum = v.fold(0, |acc, x| acc + x);
println!("{}", sum);
}"#,
    );
    assert_eq!(output, vec!["10\n"]);
}

#[test]
fn test_vec_any_all() {
    let output = run_and_capture(
        r#"fn main() {
let v = vec![1, 2, 3, 4, 5];
println!("{}", v.any(|x| x > 4));
println!("{}", v.all(|x| x > 0));
println!("{}", v.all(|x| x > 3));
}"#,
    );
    assert_eq!(output, vec!["true\n", "true\n", "false\n"]);
}

#[test]
fn test_vec_find() {
    let output = run_and_capture(
        r#"fn main() {
let v = vec![1, 2, 3, 4, 5];
let found = v.find(|x| x > 3);
println!("{:?}", found);
let not_found = v.find(|x| x > 10);
println!("{:?}", not_found);
}"#,
    );
    assert_eq!(output, vec!["Some(4)\n", "None\n"]);
}

#[test]
fn test_vec_enumerate() {
    let output = run_and_capture(
        r#"fn main() {
let v = vec!["a", "b", "c"];
let pairs = v.enumerate().collect();
println!("{:?}", pairs);
}"#,
    );
    assert_eq!(output, vec!["[(0, \"a\"), (1, \"b\"), (2, \"c\")]\n"]);
}

#[test]
fn test_vec_chain_map_filter() {
    let output = run_and_capture(
        r#"fn main() {
let v = vec![1, 2, 3, 4, 5];
let result = v.map(|x| x * 2).filter(|x| x > 4).collect();
println!("{:?}", result);
}"#,
    );
    assert_eq!(output, vec!["[6, 8, 10]\n"]);
}

#[test]
fn test_vec_flat_map() {
    let output = run_and_capture(
        r#"fn main() {
let v = vec![1, 2, 3];
let result = v.flat_map(|x| vec![x, x * 10]).collect();
println!("{:?}", result);
}"#,
    );
    assert_eq!(output, vec!["[1, 10, 2, 20, 3, 30]\n"]);
}

#[test]
fn test_vec_position() {
    let output = run_and_capture(
        r#"fn main() {
let v = vec![10, 20, 30];
println!("{:?}", v.position(|x| x == 20));
println!("{:?}", v.position(|x| x == 99));
}"#,
    );
    assert_eq!(output, vec!["Some(1)\n", "None\n"]);
}

#[test]
fn test_bitwise_op_inside_closure() {
    // Regression: BitAnd/BitOr/BitXor/Shl/Shr were missing from execute_op
    // (the dispatcher used by run_closure). Map over a Vec with a bitwise
    // closure used to error "execute_op: unhandled BitAnd".
    let out = run_and_capture(
        r#"
fn main() {
    let v = vec![0xFF, 0x0F, 0xF0];
    let masked: Vec<int> = v.iter().map(|x| x & 0x0F).collect::<Vec<_>>();
    for m in masked { println!("{}", m); }
}
"#,
    );
    assert_eq!(out, vec!["15\n", "15\n", "0\n"]);
}

#[test]
fn test_enum_match_inside_closure() {
    // Regression: EnumDataGet (and the variant-equality dance for matching
    // on Option/Result inside a closure) used to silently break — match
    // arms would all miss and print "match: no arm matched".
    let out = run_and_capture(
        r#"
fn main() {
    let opts = vec![Some(1), None, Some(3)];
    let unwrapped: Vec<int> = opts.iter().map(|o| match o {
        Some(v) => v,
        None => 0,
    }).collect::<Vec<_>>();
    for u in unwrapped { println!("{}", u); }
}
"#,
    );
    assert_eq!(out, vec!["1\n", "0\n", "3\n"]);
}

#[test]
fn test_option_map_with_closure() {
    let output = run_and_capture(
        r#"fn main() {
let val = Some(5);
let doubled = val.map(|x| x * 2);
println!("{:?}", doubled);
let none_val: Option<int> = None;
let mapped = none_val.map(|x| x * 2);
println!("{:?}", mapped);
}"#,
    );
    assert_eq!(output, vec!["Some(10)\n", "None\n"]);
}

#[test]
fn test_result_map_with_closure() {
    let output = run_and_capture(
        r#"fn main() {
let val: Result<int, String> = Ok(5);
let doubled = val.map(|x| x * 2);
println!("{:?}", doubled);
}"#,
    );
    assert_eq!(output, vec!["Ok(10)\n"]);
}

#[test]
fn test_closure_as_method_callback() {
    let output = run_and_capture(
        r#"fn main() {
let v = vec![1, 2, 3];
let sum = v.fold(0, |acc, x| acc + x);
let product = v.fold(1, |acc, x| acc * x);
println!("{} {}", sum, product);
}"#,
    );
    assert_eq!(output, vec!["6 6\n"]);
}

#[test]
fn test_iter_collect() {
    let output = run_and_capture(
        r#"fn main() {
let v = vec![1, 2, 3];
let v2 = v.iter().collect();
println!("{:?}", v2);
}"#,
    );
    assert_eq!(output, vec!["[1, 2, 3]\n"]);
}

#[test]
fn test_vec_zip() {
    let output = run_and_capture(
        r#"fn main() {
            let a = vec![1, 2, 3];
            let b = vec!["a", "b", "c"];
            let zipped = a.zip(b).collect();
            println!("{:?}", zipped);
            }"#,
    );
    assert_eq!(output, vec!["[(1, \"a\"), (2, \"b\"), (3, \"c\")]\n"]);
}

#[test]
fn test_vec_take_skip() {
    let output = run_and_capture(
        r#"fn main() {
            let v = vec![1, 2, 3, 4, 5];
            let first = v.take(3).collect();
            let rest = v.skip(2).collect();
            println!("{:?} {:?}", first, rest);
            }"#,
    );
    assert_eq!(output, vec!["[1, 2, 3] [3, 4, 5]\n"]);
}

#[test]
fn test_vec_chain() {
    let output = run_and_capture(
        r#"fn main() {
            let a = vec![1, 2];
            let b = vec![3, 4];
            let c = a.chain(b).collect();
            println!("{:?}", c);
            }"#,
    );
    assert_eq!(output, vec!["[1, 2, 3, 4]\n"]);
}

#[test]
fn test_vec_flatten() {
    let output = run_and_capture(
        r#"fn main() {
            let nested = vec![vec![1, 2], vec![3, 4]];
            let flat = nested.flatten().collect();
            println!("{:?}", flat);
            }"#,
    );
    assert_eq!(output, vec!["[1, 2, 3, 4]\n"]);
}

#[test]
fn test_vec_sum() {
    let output = run_and_capture(
        r#"fn main() {
            let v = vec![1, 2, 3, 4, 5];
            println!("{}", v.sum());
            }"#,
    );
    assert_eq!(output, vec!["15\n"]);
}

#[test]
fn test_vec_rev() {
    let output = run_and_capture(
        r#"fn main() {
            let mut v = vec![1, 2, 3];
            v.rev();
            println!("{:?}", v);
            }"#,
    );
    assert_eq!(output, vec!["[3, 2, 1]\n"]);
}

#[test]
fn test_vec_sort() {
    let output = run_and_capture(
        r#"fn main() {
            let mut v = vec![3, 1, 4, 1, 5];
            v.sort();
            println!("{:?}", v);
            }"#,
    );
    assert_eq!(output, vec!["[1, 1, 3, 4, 5]\n"]);
}

#[test]
fn test_vec_sort_by() {
    let output = run_and_capture(
        r#"fn main() {
            let mut v = vec![3, 1, 4, 1, 5];
            v.sort_by(|a, b| b - a);
            println!("{:?}", v);
            }"#,
    );
    assert_eq!(output, vec!["[5, 4, 3, 1, 1]\n"]);
}

#[test]
fn test_vec_sort_by_key() {
    let output = run_and_capture(
        r#"fn main() {
            let mut v = vec!["aa", "b", "ccc"];
            v.sort_by_key(|s| s.len());
            println!("{:?}", v);
            }"#,
    );
    assert_eq!(output, vec!["[\"b\", \"aa\", \"ccc\"]\n"]);
}

#[test]
fn test_vec_dedup() {
    let output = run_and_capture(
        r#"fn main() {
            let mut v = vec![1, 1, 2, 2, 3];
            v.dedup();
            println!("{:?}", v);
            }"#,
    );
    assert_eq!(output, vec!["[1, 2, 3]\n"]);
}

#[test]
fn test_vec_min_max() {
    let output = run_and_capture(
        r#"fn main() {
            let v = vec![3, 1, 4, 1, 5];
            println!("{:?} {:?}", v.min(), v.max());
            }"#,
    );
    assert_eq!(output, vec!["Some(1) Some(5)\n"]);
}

#[test]
fn test_vec_windows() {
    let output = run_and_capture(
        r#"fn main() {
            let v = vec![1, 2, 3, 4];
            let w = v.windows(2);
            println!("{:?}", w);
            }"#,
    );
    assert_eq!(output, vec!["[[1, 2], [2, 3], [3, 4]]\n"]);
}

#[test]
fn test_vec_chunks() {
    let output = run_and_capture(
        r#"fn main() {
            let v = vec![1, 2, 3, 4, 5];
            let c = v.chunks(2);
            println!("{:?}", c);
            }"#,
    );
    assert_eq!(output, vec!["[[1, 2], [3, 4], [5]]\n"]);
}

#[test]
fn test_iterator_chaining() {
    let output = run_and_capture(
        r#"fn main() {
            let v = vec![1, 2, 3, 4, 5, 6];
            let result = v.filter(|x| x % 2 == 0).collect().map(|x| x * 10).sum();
            println!("{}", result);
            }"#,
    );
    assert_eq!(output, vec!["120\n"]);
}

#[test]
fn test_mutable_closure_capture() {
    let output = run_and_capture(
        r#"fn main() {
                let mut count = 0;
                let inc = || { count = count + 1; };
                inc();
                inc();
                inc();
                println!("{}", count);
            }"#,
    );
    assert_eq!(output, vec!["3\n"]);
}

#[test]
fn test_closure_counter_pattern() {
    let output = run_and_capture(
        r#"
            fn make_counter() {
                let mut n = 0;
                let inc = || { n = n + 1; n };
                inc
            }
            fn main() {
                let c = make_counter();
                println!("{} {} {}", c(), c(), c());
            }
            "#,
    );
    assert_eq!(output, vec!["1 2 3\n"]);
}
