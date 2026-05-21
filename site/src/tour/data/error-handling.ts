import type { Chapter } from '../types';

export const errorHandling: Chapter = {
  id: 'error-handling',
  title: 'Error Handling',
  lessons: [
    {
      id: 'option',
      title: 'Option',
      instructions: `## Option — Maybe a Value

\`Option<T>\` represents a value that may or may not be present:
- \`Some(value)\` — has a value
- \`None\` — no value

Methods: \`is_some()\`, \`is_none()\`, \`unwrap()\`, \`unwrap_or(default)\`, \`map(f)\`, \`and_then(f)\`.

**Try it:** Use \`unwrap_or(0)\` instead of match to get a default value.`,
      hints: [
        '`unwrap()` panics on None — only use when sure.',
        '`map()` transforms the inner value if Some, passes through None.',
      ],
      initialCode: `fn find_even(nums: Vec<i64>) -> Option<i64> {
    for n in nums {
        if n % 2 == 0 {
            return Some(n);
        }
    }
    None
}

fn main() {
    let result = find_even([1, 3, 6, 7]);
    match result {
        Some(n) => println!("found: {}", n),
        None => println!("no even found"),
    }

    println!("is some: {}", result.is_some());
    println!("mapped: {}", result.map(|x| x * 2).unwrap());
}
`,
    },
    {
      id: 'result',
      title: 'Result',
      instructions: `## Result — Success or Error

\`Result<T, E>\` is either:
- \`Ok(value)\` — success
- \`Err(error)\` — failure

Methods: \`is_ok()\`, \`is_err()\`, \`unwrap()\`, \`unwrap_err()\`, \`map()\`, \`map_err()\`, \`and_then()\`, \`ok()\`.

**Try it:** Add another function that parses and multiplies two numbers, propagating errors.`,
      hints: [
        '`Ok` and `Err` are the constructors.',
        'Match on Result to handle both success and failure paths.',
      ],
      initialCode: `fn divide(a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        return Err("division by zero".to_string());
    }
    Ok(a / b)
}

fn main() {
    let ok = divide(10.0, 2.0);
    match ok {
        Ok(v) => println!("result: {}", v),
        Err(e) => println!("error: {}", e),
    }

    let bad = divide(1.0, 0.0);
    println!("is error: {}", bad.is_err());
    println!("error msg: {}", bad.unwrap_err());
}
`,
    },
    {
      id: 'try-operator',
      title: 'Try Operator ?',
      instructions: `## The Try Operator

The \`?\` operator propagates errors upward. If the value is \`Ok(v)\`, it unwraps to \`v\`. If it's \`Err(e)\`, it returns early with that error.

This works with both \`Option\` and \`Result\`. The enclosing function must return a compatible type.

**Try it:** Change the input to include a non-numeric string and see the error propagation.`,
      hints: [
        '`expr?` is short for: match expr { ok(v) => v, err(e) => return err(e) }.',
        '`?` works on Option too — None propagates as None.',
      ],
      initialCode: `fn parse_and_divide(a_str: String, b_str: String) -> Result<f64, String> {
    let a = a_str.parse_float().map_err(|e| f"parse error: {e}")?;
    let b = b_str.parse_float().map_err(|e| f"parse error: {e}")?;
    if b == 0.0 {
        return Err("division by zero".to_string());
    }
    Ok(a / b)
}

fn main() {
    match parse_and_divide("10.5".to_string(), "2.0".to_string()) {
        Ok(v) => println!("result: {}", v),
        Err(e) => println!("error: {}", e),
    }
}
`,
    },
    {
      id: 'combinators',
      title: 'Option / Result Combinators',
      instructions: `## Chaining with Combinators

Both Option and Result have chainable methods:
- \`map(f)\` — transform the success value
- \`and_then(f)\` — chain a fallible operation
- \`unwrap_or(default)\` — provide a fallback
- \`ok()\` / \`err()\` — convert between Option and Result

**Try it:** Chain more operations — find a number, double it, check if it's > 20.`,
      hints: [
        'Combinators avoid deeply nested match expressions.',
        '`and_then` is flat_map — the callback must return Option/Result.',
      ],
      initialCode: `fn first_even(nums: Vec<i64>) -> Option<i64> {
    nums.iter().filter(|x| x % 2 == 0).collect::<Vec<_>>().first().clone()
}

fn main() {
    let nums = [1, 3, 6, 7, 10];

    let result = first_even(nums)
        .map(|n| n * 2)
        .map(|n| f"doubled even: {n}");

    println!("{}", result.unwrap_or("no even found".to_string()));

    // Using unwrap_or
    let empty: [i64; 0] = [];
    println!("{}", first_even(empty).unwrap_or(-1));
}
`,
    },
    {
      id: 'if-let',
      title: 'If Let & While Let',
      instructions: `## If Let & While Let

\`if let\` matches a pattern and executes the body if it matches — a concise alternative to \`match\` with one arm plus a default.

\`while let\` keeps looping as long as the pattern matches — useful for iterating through Option/Result sequences.

**Try it:** Change the code to stop at the first number > 50.`,
      hints: [
        '`if let Pattern = expr { ... }` — shorter than `match` for single-arm.',
        '`while let` is great with iterators and queues.',
      ],
      initialCode: `fn main() {
    let opt = Some(42);
    if let Some(n) = opt {
        println!("got {}", n);
    }

    let mut vals = [1, 2, 3, 4, 5];
    let mut i = 0;
    while i < vals.len() {
        if let n if n % 2 == 0 = vals[i] {
            println!("even: {}", n);
        }
        i = i + 1;
    }
}
`,
    },
  ],
};
