// control_flow.ox — Demonstrates Phase 5 control flow features

fn classify(n: Int) -> Int {
    match n {
        0 => 0,
        1 => 1,
        _ => n * 2,
    }
}

fn main() {
    // While loop
    var i = 1;
    var factorial = 1;
    while i <= 5 {
        factorial *= i;
        i += 1;
    }
    println("5! = {}", factorial);

    // For loop with range
    var sum = 0;
    for i in 1..=100 {
        sum += i;
    }
    println("Sum 1..=100 = {}", sum);

    // Loop with break value
    var x = 1;
    val result = loop {
        x *= 2;
        if x > 100 {
            break x;
        }
    };
    println("First power of 2 > 100: {}", result);

    // FizzBuzz
    println("FizzBuzz 1..=15:");
    for i in 1..=15 {
        if i % 15 == 0 {
            println("  FizzBuzz");
        } else if i % 3 == 0 {
            println("  Fizz");
        } else if i % 5 == 0 {
            println("  Buzz");
        } else {
            println("  {}", i);
        }
    }

    // Match expression
    for i in 0..4 {
        println("classify({}) = {}", i, classify(i));
    }
}
