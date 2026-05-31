// Oxy Closures & Higher-Order Functions Example
// Demonstrates closure syntax, variable capture, and iterator methods

fn apply(f: Fn, x: Int) -> Int {
    f(x)
}

fn make_adder(n: Int) -> Fn {
    |x| x + n
}

fn main() {
    // Basic closure
    val double = |x| x * 2;
    println("double(5) = {}", double(5));

    // Closure with type annotations
    val add = |a: Int, b: Int| -> Int { a + b };
    println("add(3, 4) = {}", add(3, 4));

    // No-param closure
    val greet = || "hello!";
    println("{}", greet());

    // Closure captures variables from enclosing scope
    val factor = 3;
    val multiply = |x| x * factor;
    println("multiply(5) = {}", multiply(5));

    // Passing closure as argument
    val result = apply(|x| x * x, 7);
    println("apply(|x| x*x, 7) = {}", result);

    // Returning closure from function
    val add5 = make_adder(5);
    println("add5(10) = {}", add5(10));

    // closure
    val name = "world";
    val greet2 = || format("hello {}", name);
    println("{}", greet2());

    // List iterator methods
    println("\n=== Iterator Methods ===");

    val numbers = [1, 2, 3, 4, 5];

    // map
    val doubled = numbers.map(|x| x * 2);
    println("map(*2): {:?}", doubled);

    // filter
    val evens = numbers.filter(|x| x % 2 == 0);
    println("filter(even): {:?}", evens);

    // chaining map + filter
    val result = numbers.map(|x| x * 2).filter(|x| x > 4);
    println("map(*2).filter(>4): {:?}", result);

    // fold
    val sum = numbers.fold(0, |acc, x| acc + x);
    println("fold(sum): {}", sum);

    // any / all
    println("any(>4): {}", numbers.any(|x| x > 4));
    println("all(>0): {}", numbers.all(|x| x > 0));

    // find
    val found = numbers.find(|x| x > 3);
    println("find(>3): {:?}", found);

    // enumerate
    val words = ["foo", "bar", "baz"];
    val indexed = words.enumerate();
    println("enumerate: {:?}", indexed);

    // for_each
    print("for_each: ");
    numbers.for_each(|x| print("{} ", x));
    println("");

    // flat_map
    val nested = [1, 2, 3];
    val flat = nested.flat_map(|x| [x, x * 10]);
    println("flat_map: {:?}", flat);

    // position
    println("position(==3): {:?}", numbers.position(|x| x == 3));

    println("\nDone!");
}
