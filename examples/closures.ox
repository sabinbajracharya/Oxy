// Oxide Closures & Higher-Order Functions Example
// Demonstrates closure syntax, variable capture, and iterator methods

fn apply(f: Fn, x: i64) -> i64 {
    f(x)
}

fn make_adder(n: i64) -> Fn {
    |x| x + n
}

fn main() {
    // Basic closure
    let double = |x| x * 2;
    println!("double(5) = {}", double(5));

    // Closure with type annotations
    let add = |a: i64, b: i64| -> i64 { a + b };
    println!("add(3, 4) = {}", add(3, 4));

    // No-param closure
    let greet = || "hello!";
    println!("{}", greet());

    // Closure captures variables from enclosing scope
    let factor = 3;
    let multiply = |x| x * factor;
    println!("multiply(5) = {}", multiply(5));

    // Passing closure as argument
    let result = apply(|x| x * x, 7);
    println!("apply(|x| x*x, 7) = {}", result);

    // Returning closure from function
    let add5 = make_adder(5);
    println!("add5(10) = {}", add5(10));

    // move closure
    let name = "world";
    let greet2 = move || format!("hello {}", name);
    println!("{}", greet2());

    // Vec iterator methods
    println!("\n=== Iterator Methods ===");

    let numbers = vec![1, 2, 3, 4, 5];

    // map
    let doubled = numbers.map(|x| x * 2);
    println!("map(*2): {:?}", doubled);

    // filter
    let evens = numbers.filter(|x| x % 2 == 0);
    println!("filter(even): {:?}", evens);

    // chaining map + filter
    let result = numbers.map(|x| x * 2).filter(|x| x > 4);
    println!("map(*2).filter(>4): {:?}", result);

    // fold
    let sum = numbers.fold(0, |acc, x| acc + x);
    println!("fold(sum): {}", sum);

    // any / all
    println!("any(>4): {}", numbers.any(|x| x > 4));
    println!("all(>0): {}", numbers.all(|x| x > 0));

    // find
    let found = numbers.find(|x| x > 3);
    println!("find(>3): {:?}", found);

    // enumerate
    let words = vec!["foo", "bar", "baz"];
    let indexed = words.enumerate();
    println!("enumerate: {:?}", indexed);

    // for_each
    print!("for_each: ");
    numbers.for_each(|x| print!("{} ", x));
    println!("");

    // flat_map
    let nested = vec![1, 2, 3];
    let flat = nested.flat_map(|x| vec![x, x * 10]);
    println!("flat_map: {:?}", flat);

    // position
    println!("position(==3): {:?}", numbers.position(|x| x == 3));

    println!("\nDone!");
}
