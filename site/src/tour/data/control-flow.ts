import type { Chapter } from '../types';

export const controlFlow: Chapter = {
  id: 'control-flow',
  title: 'Control Flow',
  lessons: [
    {
      id: 'if-else',
      title: 'If / Else',
      instructions: `## If / Else Expressions

\`if\` expressions evaluate a boolean condition and execute the first branch whose condition is \`true\`.

In Oxy, \`if\` is an **expression** — it returns a value. This means you can use it on the right side of \`let\`.

**Try it:** Add an \`else if\` branch for numbers between 10 and 20.`,
      hints: [
        'No parentheses needed around conditions.',
        'All branches must return the same type when used as an expression.',
      ],
      initialCode: `fn main() {
    let x = 15;

    if x > 20 {
        println!("x is large");
    } else if x > 10 {
        println!("x is medium");
    } else {
        println!("x is small");
    }

    // if as an expression
    let label = if x % 2 == 0 { "even" } else { "odd" };
    println!("x is {}", label);
}
`,
    },
    {
      id: 'loops',
      title: 'While & Loop',
      instructions: `## While & Loop

\`while\` runs the body as long as the condition is \`true\`.

\`loop\` runs forever until you \`break\`. Loops can return a value by placing a value after \`break\`.

**Try it:** Change the while condition to count down from 10 instead of up to 5.`,
      hints: [
        'Use `break` to exit a loop early.',
        'Use `continue` to skip to the next iteration.',
        '`break value` returns a value from the loop.',
      ],
      initialCode: `fn main() {
    let mut count = 0;
    while count < 5 {
        println!("count = {}", count);
        count = count + 1;
    }

    let result = loop {
        count = count - 1;
        if count == 0 {
            break count * 10;
        }
    };
    println!("loop result: {}", result);
}
`,
    },
    {
      id: 'for-in',
      title: 'For / In',
      instructions: `## For / In Loops

\`for\` loops iterate over any iterable: ranges, arrays, vecs, strings, and more.

Ranges are written as \`start..end\` (exclusive) or \`start..=end\` (inclusive).

**Try it:** Change the range to \`1..=5\` (inclusive) and use \`..\` to iterate in steps.`,
      hints: [
        '`0..10` goes from 0 to 9 (10 items).',
        '`0..=10` goes from 0 to 10 (11 items).',
        '`for item in collection` works on Vec, Array, String, HashMap keys, etc.',
      ],
      initialCode: `fn main() {
    for i in 0..5 {
        println!("i = {}", i);
    }

    let names = ["alice", "bob", "carol"];
    for name in names {
        println!("Hello, {}!", name);
    }
}
`,
    },
    {
      id: 'match',
      title: 'Pattern Matching',
      instructions: `## Match Expressions

\`match\` compares a value against a series of **patterns**. The first matching arm executes.

Patterns can be literals, variables, wildcards (\`_\`), ranges, structs, enums, and more.

**Try it:** Add a match arm for the number 0 (print "zero").`,
      hints: [
        '`_` is the wildcard pattern — it matches anything.',
        'Match is exhaustive — all possible values must be handled.',
      ],
      initialCode: `fn main() {
    let x = 3;

    match x {
        1 => println!("one"),
        2 => println!("two"),
        3 => println!("three"),
        _ => println!("many!"),
    }

    // match as an expression
    let description = match x % 2 {
        0 => "even",
        _ => "odd",
    };
    println!("{} is {}", x, description);
}
`,
    },
    {
      id: 'break-continue',
      title: 'Break & Continue',
      instructions: `## Break & Continue

\`break\` exits a loop immediately. \`continue\` skips to the next iteration.

Both support **labels** for nested loops. Label a loop with \`'name:\` and use \`break 'name;\`.

**Try it:** Instead of skipping 3, try skipping all even numbers (hint: \`i % 2 == 0\`).`,
      hints: [
        'Labels are written with a single quote: `\'outer:`.',
        '`break \'label;` exits the labeled loop from an inner loop.',
      ],
      initialCode: `fn main() {
    for i in 0..10 {
        if i == 3 {
            continue; // skip 3
        }
        if i == 7 {
            break; // stop at 7
        }
        println!("i = {}", i);
    }

    println!("---");

    'outer: for x in 0..3 {
        for y in 0..3 {
            if x * y > 2 {
                println!("breaking outer at {},{}", x, y);
                break 'outer;
            }
            println!("  ({}, {})", x, y);
        }
    }
}
`,
    },
  ],
};
