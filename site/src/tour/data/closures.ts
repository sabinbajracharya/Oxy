import type { Chapter } from '../types';

export const closures: Chapter = {
  id: 'closures',
  title: 'Closures',
  lessons: [
    {
      id: 'syntax',
      title: 'Closure Syntax',
      instructions: `## Closure Syntax

Closures are anonymous functions you can store in variables or pass around. Oxy uses Rust-style pipe syntax.

Basic forms:
- \`|x| x * 2\` — single parameter, expression body
- \`|x, y| x + y\` — multiple parameters
- \`|| 42\` — no parameters
- \`|x| { let y = x * 2; y + 1 }\` — block body

Call a closure like a regular function: \`closure(args)\`.

**Your task:** Create three closures in \`main\`: \`double\` that doubles its input, \`add\` that sums two ints, and \`always_42\` that returns 42 with no arguments. Then call each and print the results.`,
      hints: [
        'Pipe characters \`|\` surround the parameter list, not parentheses.',
        'A closure with no parameters looks like \`|| expr\`.',
        'For block bodies, wrap in \`{ }\` like a normal function body.',
        'The last expression in a block body is the return value (no semicolon).',
      ],
      initialCode: `fn main() {
    // TODO: create a closure called "double" that takes an int and returns it doubled
    let double = ___;

    // TODO: create a closure called "add" that takes two ints and returns their sum
    let add = ___;

    // TODO: create a closure called "always_42" that takes no args and returns 42
    let always_42 = ___;

    println!("double(5) = {}", double(5));
    println!("add(3, 4) = {}", add(3, 4));
    println!("always_42() = {}", always_42());
}
`,
      testCode: `#[test] fn test_single_param() {
    let double = |x| x * 2;
    assert_eq!(double(5), 10);
    assert_eq!(double(0), 0);
    assert_eq!(double(-3), -6);
}

#[test] fn test_multi_params() {
    let add = |x, y| x + y;
    assert_eq!(add(3, 4), 7);
    assert_eq!(add(0, 0), 0);
    assert_eq!(add(-5, 5), 0);
}

#[test] fn test_no_params() {
    let always_42 = || 42;
    assert_eq!(always_42(), 42);
}

#[test] fn test_block_body() {
    let compute = |x: int| -> int {
        let y = x * 2;
        y + 1
    };
    assert_eq!(compute(10), 21);
}

#[test] fn test_inline_call() {
    let result = (|x, y| x + y)(3, 4);
    assert_eq!(result, 7);
}
`,
    },
    {
      id: 'type-annotations',
      title: 'Type Annotations',
      instructions: `## Type Annotations on Closures

Closures can have explicit type annotations on parameters and return types:

\`\`\`
|x: int| -> int { x + 1 }
\`\`\`

When you add a return type, you **must** use a block body (\`{ }\`). The expression body form (\`|x| x + 1\`) works when types are inferred.

Type annotations are optional — Oxy infers types — but they make intent clear and help catch errors.

**Your task:** Write two closures with explicit type annotations. Create \`multiply\` with typed int params and a typed return type. Create \`greet\` taking a typed String.`,
      hints: [
        'Parameter annotations: \`|param: Type, param2: Type| ...\`.',
        'Return type requires a block body: \`|x: int| -> int { x * 2 }\`.',
        'Without a return type, an expression body works: \`|x: int| x * 2\`.',
      ],
      initialCode: `fn main() {
    // TODO: create a typed closure "multiply" taking two ints and returning an int
    let multiply = ___;

    // TODO: create a typed closure "greet" taking a String and returning a String
    let greet = ___;

    println!("multiply(6, 7) = {}", multiply(6, 7));
    println!("greet(\\"Oxy\\") = {}", greet("Oxy".to_string()));
}
`,
      testCode: `#[test] fn test_multiply_typed() {
    let multiply = |x: int, y: int| -> int { x * y };
    assert_eq!(multiply(6, 7), 42);
    assert_eq!(multiply(0, 5), 0);
    assert_eq!(multiply(-2, 3), -6);
}

#[test] fn test_return_type_annotation() {
    let identity = |x: int| -> int { x };
    assert_eq!(identity(99), 99);
    assert_eq!(identity(-1), -1);
}

#[test] fn test_inferred_vs_explicit() {
    let inferred = |x, y| x + y;
    let explicit = |x: int, y: int| -> int { x + y };
    assert_eq!(inferred(10, 20), explicit(10, 20));
}

#[test] fn test_typed_block_body() {
    let compute = |a: int, b: int| -> int {
        let sum = a + b;
        sum * 2
    };
    assert_eq!(compute(3, 4), 14);
}
`,
    },
    {
      id: 'captures',
      title: 'Captures',
      instructions: `## Capturing Variables

Closures can **capture** variables from their enclosing scope. The closure remembers the values and uses them when called.

\`\`\`
let multiplier = 3;
let triple = |x| x * multiplier;
triple(5); // 15
\`\`\`

The captured variable does not need to be passed as a parameter — the closure captures it automatically.

**Your task:** Write closures that capture outer variables. Create \`add_n\` that captures \`n\` and adds it to its input. Create \`above_threshold\` that captures \`threshold\` and checks if a value exceeds it.`,
      hints: [
        'Just use the outer variable inside the closure body — it\'s captured automatically.',
        'The closure can read captured variables and use them in expressions.',
        'Captured variables are available to the closure even after the original scope ends.',
      ],
      initialCode: `fn main() {
    let n = 10;

    // TODO: create a closure "add_n" that captures n and adds it to its argument
    let add_n = ___;

    let threshold = 5;
    // TODO: create a closure "above_threshold" that captures threshold and checks x > threshold
    let above_threshold = ___;

    println!("add_n(3) = {}", add_n(3));
    println!("above_threshold(10) = {}", above_threshold(10));
    println!("above_threshold(2) = {}", above_threshold(2));
}
`,
      testCode: `#[test] fn test_capture_basic() {
    let offset = 5;
    let add_offset = |x| x + offset;
    assert_eq!(add_offset(10), 15);
    assert_eq!(add_offset(0), 5);
}

#[test] fn test_capture_string() {
    let suffix = "!!!";
    let exclaim = |s: String| s + suffix;
    assert_eq!(exclaim("hello".to_string()), "hello!!!");
}

#[test] fn test_capture_multiple() {
    let a = 3;
    let b = 7;
    let add_ab = |x| x + a + b;
    assert_eq!(add_ab(10), 20);
}

#[test] fn test_capture_condition() {
    let threshold = 50;
    let is_above = |x| x > threshold;
    assert!(is_above(100));
    assert!(!is_above(25));
}

#[test] fn test_capture_in_map() {
    let factor = 3;
    let v = vec![1, 2, 3, 4];
    let result = v.map(|x| x * factor);
    assert_eq!(result.len(), 4);
    assert_eq!(result[0], 3);
    assert_eq!(result[3], 12);
}
`,
    },
    {
      id: 'move-closures',
      title: 'Move Closures',
      instructions: `## Move Closures

Use \`move\` before the parameter list to force a closure to take ownership of captured values:

\`\`\`
let x = 42;
let f = move || x * 2;
\`\`\`

The \`move\` keyword ensures the closure owns its captured data. This is important when the closure needs to outlive the scope in which the captured variables were created.

**Your task:** Write a move closure that captures a string and returns it with a suffix. Also create a move closure that captures a numeric factor and uses it with a parameter.`,
      hints: [
        'Write \`move\` right before the pipes: \`move || expression\`.',
        'With \`move\` the closure takes ownership of captured variables.',
        'Move closures can also take parameters: \`move |x| x + captured\`.',
      ],
      initialCode: `fn main() {
    let base = 100;

    // TODO: create a move closure "add_base" capturing base, taking an int, and returning base + int
    let add_base = ___;

    // TODO: create a move closure "make_greeting" capturing a name and returning a greeting string
    let name = "Oxy".to_string();
    let make_greeting = ___;

    println!("add_base(50) = {}", add_base(50));
    println!("make_greeting() = {}", make_greeting());
}
`,
      testCode: `#[test] fn test_move_basic() {
    let x = 10;
    let f = move || x * 2;
    assert_eq!(f(), 20);
}

#[test] fn test_move_with_params() {
    let offset = 5;
    let add_offset = move |n: int| n + offset;
    assert_eq!(add_offset(10), 15);
    assert_eq!(add_offset(0), 5);
}

#[test] fn test_move_string() {
    let greeting = "Hello, ".to_string();
    let greet = move |name: String| greeting + name;
    assert_eq!(greet("World".to_string()), "Hello, World");
}

#[test] fn test_move_multiple_captures() {
    let a = 3;
    let b = 7;
    let sum_captured = move |c: int| a + b + c;
    assert_eq!(sum_captured(10), 20);
    assert_eq!(sum_captured(0), 10);
}

#[test] fn test_move_closure_in_vec_map() {
    let factor = 3;
    let v = vec![1, 2, 3];
    let result = v.map(move |x| x * factor);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], 3);
    assert_eq!(result[2], 9);
}
`,
    },
    {
      id: 'higher-order',
      title: 'Higher-Order Functions',
      instructions: `## Higher-Order Functions

Higher-order functions accept other functions (or closures) as parameters. In Oxy, the function type syntax uses \`fn\`:

\`\`\`
fn apply_twice<T>(f: fn(T) -> T, x: T) -> T {
    f(f(x))
}
\`\`\`

The \`fn(T) -> T\` syntax means "a function that takes \`T\` and returns \`T\`". You can pass named functions or closures.

**Your task:** Implement \`apply_twice\` that applies a function \`f\` twice to its argument. Then use it with a closure to double a number twice and add three twice.`,
      hints: [
        'Function pointer type is \`fn(param_type) -> return_type\`.',
        'Generic type parameter: \`<T>\` after the function name.',
        'Call \`f(x)\` to apply the function once, then \`f(f(x))\` to apply twice.',
        'You can pass both named functions and closures as arguments.',
      ],
      initialCode: `// TODO: implement apply_twice<T>(f: fn(T) -> T, x: T) -> T
fn apply_twice<T>(f: fn(T) -> T, x: T) -> T {
    ___
}

fn main() {
    let double = |x| x * 2;
    let result = apply_twice(double, 5);
    println!("double twice of 5 = {}", result); // should be 20

    let add_three = |x| x + 3;
    println!("add three twice to 1 = {}", apply_twice(add_three, 1)); // should be 7
}
`,
      testCode: `#[test] fn test_apply_twice_double() {
    fn apply_twice<T>(f: fn(T) -> T, x: T) -> T {
        f(f(x))
    }
    let double = |x| x * 2;
    assert_eq!(apply_twice(double, 5), 20);
    assert_eq!(apply_twice(double, 0), 0);
    assert_eq!(apply_twice(double, -3), -12);
}

#[test] fn test_apply_twice_add() {
    fn apply_twice<T>(f: fn(T) -> T, x: T) -> T {
        f(f(x))
    }
    let add_ten = |x| x + 10;
    assert_eq!(apply_twice(add_ten, 1), 21);
    assert_eq!(apply_twice(add_ten, 0), 20);
}

#[test] fn test_apply_twice_string() {
    fn apply_twice<T>(f: fn(T) -> T, x: T) -> T {
        f(f(x))
    }
    let add_bang = |s: String| s + "!";
    let result = apply_twice(add_bang, "hello".to_string());
    assert_eq!(result, "hello!!");
}

#[test] fn test_apply_twice_generic() {
    fn apply_twice<T>(f: fn(T) -> T, x: T) -> T {
        f(f(x))
    }
    assert_eq!(apply_twice(|x| x * 3, 2), 18);  // (2*3)*3
    assert_eq!(apply_twice(|x| x - 1, 5), 3);    // (5-1)-1
}
`,
    },
  ],
};
