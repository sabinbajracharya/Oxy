import type { Chapter } from '../types';

export const functions: Chapter = {
  id: 'functions',
  title: 'Functions',
  lessons: [
    {
      id: 'definition',
      title: 'Function Definition',
      instructions: `## Defining Functions

Functions are declared with \`fn\`, followed by the name, parameters in parentheses, an optional return type, and a body in curly braces.

The last expression in a function body (without a semicolon) is the **return value**. Use \`return\` for early returns.

**Try it:** Add a third number parameter and include it in the sum.`,
      hints: [
        'Parameters are written as `name: Type`.',
        'The return type comes after `->`. If omitted, the function returns `()` (unit).',
      ],
      initialCode: `fn add(a: int, b: int) -> int {
    a + b
}

fn greet(name: String) {
    println!("Hello, {}!", name);
    // No return type = returns ()
}

fn main() {
    let sum = add(3, 4);
    println!("3 + 4 = {}", sum);
    greet("Oxy".to_string());
}
`,
    },
    {
      id: 'return-type',
      title: 'Return Types & Early Return',
      instructions: `## Return Types & Early Return

Use \`return\` to exit a function early. The \`return\` keyword can be used with or without a value.

For functions that return \`()\` (unit), a bare \`return;\` exits immediately.

**Try it:** Pass a negative number and see the guard clause in action.`,
      hints: [
        '`return expr;` exits the function with that value.',
        'A block\'s last expression without semicolon is also a return.',
      ],
      initialCode: `fn safe_divide(a: float, b: float) -> Option<float> {
    if b == 0.0 {
        return None();
    }
    Some(a / b)
}

fn main() {
    let result = safe_divide(10.0, 2.0);
    println!("10 / 2 = {}", result.unwrap());

    let bad = safe_divide(5.0, 0.0);
    println!("5 / 0 is none: {}", bad.is_none());
}
`,
    },
    {
      id: 'closures-intro',
      title: 'Closures',
      instructions: `## Closures

Closures are anonymous functions defined with \`|params| body\`. They can capture variables from their surrounding scope.

Pass closures to higher-order functions like \`map\`, \`filter\`, and \`sort_by\`.

**Try it:** Change the multiplier to 3 and see the output update.`,
      hints: [
        'Closure syntax: `|x| x * factor` or `|a, b| a + b`.',
        'Closures can capture variables from the enclosing scope.',
      ],
      initialCode: `fn main() {
    let factor = 2;
    let nums = [1, 2, 3, 4, 5];

    let doubled = nums.iter()
        .map(|x| x * factor)
        .collect::<Vec<_>>();

    println!("doubled: {}", doubled);

    let add = |a: int, b: int| -> int { a + b };
    println!("3 + 4 = {}", add(3, 4));
}
`,
    },
    {
      id: 'higher-order',
      title: 'Higher-Order Functions',
      instructions: `## Higher-Order Functions

Functions that take other functions as parameters are called **higher-order functions**. You can pass both named functions and closures.

The \`fn(T) -> R\` type annotation specifies a function parameter type.

**Try it:** Add another transformation — first subtract 1, then double.

Use the \`apply_twice\` pattern with a different function.`,
      hints: [
        'Named functions can be passed as values by referencing them without `()`.',
        'Function type syntax: `fn(int) -> int`.',
      ],
      initialCode: `fn apply(f: fn(int) -> int, x: int) -> int {
    f(x)
}

fn square(x: int) -> int {
    x * x
}

fn main() {
    let result = apply(square, 5);
    println!("square(5) = {}", result);

    let doubled = apply(|x| x * 2, 10);
    println!("double(10) = {}", doubled);
}
`,
    },
  ],
};
