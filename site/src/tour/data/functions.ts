import type { Chapter } from '../types';

export const functions: Chapter = {
  id: 'functions',
  title: 'Functions',
  lessons: [
    {
      id: 'definition',
      title: 'Function Definition',
      instructions: `## Function Definition

Functions are declared with \`fn\`, followed by the name, parameters in parentheses, an optional return type, and a body in curly braces.

The last expression in a function body (without a semicolon) is the **return value**. Use \`return\` for early returns.

**Try it:** Implement \`get_meaning\` that returns \`42\` as an \`int\`. No parameters needed.`,
      hints: [
        'A function with no parameters still needs empty parentheses: \`fn name() -> Type\`.',
        'The return type comes after \`->\`. If omitted, the function returns \`()\` (unit).',
        'Either write \`return 42;\` or just \`42\` as the last expression.',
      ],
      initialCode: `// TODO: define a function called get_meaning that returns 42

fn main() {
    let answer = get_meaning();
    println!("The answer is {}", answer);
}
`,
      testCode: `#[test] fn test_get_meaning() {
    assert_eq!(get_meaning(), 42);
}
`,
    },
    {
      id: 'parameters',
      title: 'Parameters',
      instructions: `## Parameters

Functions can accept multiple parameters. Each parameter needs a **name** and a **type annotation** separated by \`:\`.

Separate multiple parameters with commas: \`fn name(a: Type, b: Type) -> ReturnType\`.

**Try it:** Implement \`multiply\` (multiplies two ints) and \`concat\` (concatenates two strings).`,
      hints: [
        'For \`multiply\`, use \`*\` to multiply the two ints.',
        'For \`concat\`, use \`+\` or \`format!\` to join the strings: \`a + b\`.',
      ],
      initialCode: `// TODO: implement multiply(a: int, b: int) -> int
// TODO: implement concat(a: String, b: String) -> String

fn main() {
    println!("3 * 4 = {}", multiply(3, 4));
    let msg = concat("Hello".to_string(), " World".to_string());
    println!("{}", msg);
}
`,
      testCode: `#[test] fn test_multiply() {
    assert_eq!(multiply(3, 4), 12);
    assert_eq!(multiply(0, 99), 0);
    assert_eq!(multiply(-2, 5), -10);
}

#[test] fn test_concat() {
    assert_eq!(concat("Hello".to_string(), " World".to_string()), "Hello World");
    assert_eq!(concat("foo".to_string(), "bar".to_string()), "foobar");
    assert_eq!(concat("".to_string(), "".to_string()), "");
}
`,
    },
    {
      id: 'return-values',
      title: 'Return Values',
      instructions: `## Return Values

In Oxy, the last expression in a function body is automatically returned — no semicolon needed! This is called an **expression-based return**.

If you put a semicolon, it becomes a **statement** (returning \`()\`), not a value.

**Try it:** Implement \`max\` that returns the larger of two ints. Use an \`if\` expression (no semicolons in the branches).`,
      hints: [
        'An \`if\` expression returns a value: \`if a > b { a } else { b }\`.',
        'No semicolons after the values in each branch, or they become unit \`()\`.',
      ],
      initialCode: `fn max(a: int, b: int) -> int {
    // TODO: return the larger of a and b using an if expression
}

fn main() {
    println!("max(3, 7) = {}", max(3, 7));
    println!("max(10, 5) = {}", max(10, 5));
}
`,
      testCode: `#[test] fn test_max() {
    assert_eq!(max(3, 7), 7);
    assert_eq!(max(10, 5), 10);
    assert_eq!(max(42, 42), 42);
    assert_eq!(max(-5, -1), -1);
}
`,
    },
    {
      id: 'mut-params',
      title: 'Mutable Parameters',
      instructions: `## Mutable Parameters

By default, function parameters are immutable inside the function body. To modify a parameter, add \`mut\` before the parameter name.

This is different from Rust's \`&mut\` — Oxy has no references. The parameter is simply a mutable local binding.

**Try it:** Implement \`increment\` that takes a \`mut n: int\`, adds 1, and returns the new value.`,
      hints: [
        'Write the parameter as \`mut n: int\`.',
        'Inside the body: \`n = n + 1; n\` (modify then return).',
      ],
      initialCode: `fn increment(mut n: int) -> int {
    // TODO: add 1 to n and return the result
}

fn main() {
    println!("increment(5) = {}", increment(5));
    println!("increment(0) = {}", increment(0));
}
`,
      testCode: `#[test] fn test_increment() {
    assert_eq!(increment(0), 1);
    assert_eq!(increment(5), 6);
    assert_eq!(increment(-1), 0);
}
`,
    },
    {
      id: 'void-functions',
      title: 'Functions Without Return',
      instructions: `## Functions Without Return

If a function has no \`-> ReturnType\`, it returns \`()\` (unit, the empty tuple). These are often called **void functions** or procedures.

Use them for side effects like printing, or for actions that don't produce a value.

**Try it:** Implement \`print_greeting\` that takes a \`name: String\` and prints \`"Hello, {name}!"\` using \`println!\`.`,
      hints: [
        'No return type annotation means the function returns \`()\`.',
        'Use \`println!("Hello, {}!", name);\` inside the body.',
        'No need for a return statement — just the \`println!\` call is enough.',
      ],
      initialCode: `// TODO: implement print_greeting(name: String) that prints "Hello, {name}!"

fn main() {
    print_greeting("Alice".to_string());
    print_greeting("Bob".to_string());
}
`,
      testCode: `#[test] fn test_void_function_compiles() {
    print_greeting("test".to_string());
    assert!(true);
}
`,
    },
  ],
};
