use std::time::Instant;

use oxy_core::interpreter::{run, run_compiled};

const FIB_SOURCE: &str = r#"
fn fib(n: i64) -> i64 {
    if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }
}
fn main() {
    let r = fib(30);
    println!("{}", r);
}
"#;

#[test]
fn bench_fibonacci_30() {
    // Warmup
    run(FIB_SOURCE).unwrap();
    run_compiled(FIB_SOURCE).unwrap();

    // Interpreted
    let start = Instant::now();
    for _ in 0..5 {
        run(FIB_SOURCE).unwrap();
    }
    let interpreted = start.elapsed() / 5;

    // Compiled
    let start = Instant::now();
    for _ in 0..5 {
        run_compiled(FIB_SOURCE).unwrap();
    }
    let compiled = start.elapsed() / 5;

    let speedup = interpreted.as_secs_f64() / compiled.as_secs_f64();

    println!();
    println!("=== Fibonacci(30) Benchmark ===");
    println!("Interpreted (tree-walking): {:?} avg", interpreted);
    println!("Compiled    (bytecode VM):  {:?} avg", compiled);
    println!("Speedup: {:.1}x", speedup);
    println!();

    assert!(speedup > 1.0, "compiled should be faster than interpreted");
}
