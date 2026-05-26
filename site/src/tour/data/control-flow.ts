import type { Chapter } from '../types';

export const controlFlow: Chapter = {
  id: 'control-flow',
  title: 'Control Flow',
  lessons: [
    {
      id: 'if-else',
      title: 'If / Else',
      instructions: `## If / Else

\`if\` expressions evaluate a boolean condition and execute the first branch whose condition is \`true\`.

An optional \`else\` branch runs when the condition is \`false\`.

No parentheses are needed around the condition — just write \`if condition { ... }\`.

**Try it:** Implement \`classify\` that returns \`"positive"\`, \`"negative"\`, or \`"zero"\` based on the input number.`,
      hints: [
        'No parentheses around the condition: \`if n > 0 { ... }\` not \`if (n > 0)\`.',
        'Braces are required around each branch body.',
      ],
      initialCode: `fn classify(n: int) -> String {
    // TODO: return "positive", "negative", or "zero"
}

fn main() {
    println!("5 is {}", classify(5));
    println!("-3 is {}", classify(-3));
    println!("0 is {}", classify(0));
}
`,
      testCode: `#[test] fn test_classify() {
    assert_eq!(classify(5), "positive");
    assert_eq!(classify(1), "positive");
    assert_eq!(classify(-3), "negative");
    assert_eq!(classify(-1), "negative");
    assert_eq!(classify(0), "zero");
}
`,
    },
    {
      id: 'else-if',
      title: 'Else If Chains',
      instructions: `## Else If Chains

Chain multiple conditions with \`else if\`. The first matching branch executes and the rest are skipped.

**Try it:** Implement \`grade\` that returns a letter grade based on score:
- 90+ → \`"A"\`
- 80-89 → \`"B"\`
- 70-79 → \`"C"\`
- 60-69 → \`"D"\`
- below 60 → \`"F"\``,
      hints: [
        'Check the highest range first, then chain down with \`else if\`.',
        'Use \`&&\` for ranges like \`score >= 80 && score < 90\`, or rely on the fact that earlier conditions have already failed.',
      ],
      initialCode: `fn grade(score: int) -> String {
    // TODO: return "A", "B", "C", "D", or "F" based on score
}

fn main() {
    println!("95 -> {}", grade(95));
    println!("83 -> {}", grade(83));
    println!("72 -> {}", grade(72));
    println!("65 -> {}", grade(65));
    println!("42 -> {}", grade(42));
}
`,
      testCode: `#[test] fn test_grade_a() {
    assert_eq!(grade(95), "A");
    assert_eq!(grade(100), "A");
    assert_eq!(grade(90), "A");
}

#[test] fn test_grade_b() {
    assert_eq!(grade(89), "B");
    assert_eq!(grade(80), "B");
}

#[test] fn test_grade_c() {
    assert_eq!(grade(79), "C");
    assert_eq!(grade(70), "C");
}

#[test] fn test_grade_d() {
    assert_eq!(grade(69), "D");
    assert_eq!(grade(60), "D");
}

#[test] fn test_grade_f() {
    assert_eq!(grade(59), "F");
    assert_eq!(grade(0), "F");
}
`,
    },
    {
      id: 'if-expressions',
      title: 'If Expressions',
      instructions: `## If Expressions

In Oxy, \`if\` is an **expression** that returns a value. You can use it on the right side of \`let\`.

Each branch must return the same type. The last expression in each branch (without a semicolon) becomes the value.

**Try it:** Implement \`abs\` that returns the absolute value using an \`if\` expression. No \`return\` keyword needed.`,
      hints: [
        'The \`if\` expression goes on the right side of \`=\`: \`let result = if cond { val1 } else { val2 };\`.',
        'No semicolons after the values inside each branch.',
        'Absolute value: if n < 0 return -n, otherwise return n.',
      ],
      initialCode: `fn abs(n: int) -> int {
    // TODO: return the absolute value using an if expression
}

fn main() {
    println!("abs(5) = {}", abs(5));
    println!("abs(-3) = {}", abs(-3));
    println!("abs(0) = {}", abs(0));
}
`,
      testCode: `#[test] fn test_abs() {
    assert_eq!(abs(5), 5);
    assert_eq!(abs(-3), 3);
    assert_eq!(abs(0), 0);
    assert_eq!(abs(-100), 100);
}
`,
    },
    {
      id: 'while-loops',
      title: 'While Loops',
      instructions: `## While Loops

\`while\` runs the loop body as long as the condition evaluates to \`true\`. The condition is checked before each iteration.

Use a \`mut\` counter variable to track progress.

**Try it:** Implement \`sum_to\` that sums all integers from 1 to \`n\` (inclusive) using a \`while\` loop. If \`n\` is 0 or less, return 0.`,
      hints: [
        'Initialize a \`let mut i = 1\` counter and a \`let mut sum = 0\` accumulator.',
        'Loop \`while i <= n { ... }\`, adding \`i\` to \`sum\` and incrementing \`i\`.',
        'Return \`sum\` after the loop ends.',
      ],
      initialCode: `fn sum_to(n: int) -> int {
    // TODO: sum integers from 1 to n using a while loop
}

fn main() {
    println!("sum_to(5) = {}", sum_to(5));
    println!("sum_to(0) = {}", sum_to(0));
}
`,
      testCode: `#[test] fn test_sum_to() {
    assert_eq!(sum_to(5), 15);
    assert_eq!(sum_to(1), 1);
    assert_eq!(sum_to(10), 55);
}

#[test] fn test_sum_to_zero() {
    assert_eq!(sum_to(0), 0);
    assert_eq!(sum_to(-3), 0);
}
`,
    },
    {
      id: 'for-in',
      title: 'For / In Loops',
      instructions: `## For / In Loops

\`for\` loops iterate over elements of a collection. The syntax is \`for element in collection { ... }\`.

You can iterate over \`Vec\`, arrays, ranges, and more.

**Try it:** Implement \`sum_list\` that returns the sum of all integers in a \`Vec<int>\`. Use a \`for\` loop.`,
      hints: [
        'Iterate with \`for item in items { ... }\`.',
        'Use a \`let mut sum = 0\` before the loop, then add each item.',
        'For an empty vec, the loop body never runs, so the result should be 0.',
      ],
      initialCode: `fn sum_list(items: Vec<int>) -> int {
    // TODO: sum all integers in the vec using a for loop
}

fn main() {
    let nums = vec![1, 2, 3, 4, 5];
    println!("sum = {}", sum_list(nums));

    let empty: Vec<int> = vec![];
    println!("sum of empty = {}", sum_list(empty));
}
`,
      testCode: `#[test] fn test_sum_list() {
    assert_eq!(sum_list(vec![1, 2, 3, 4, 5]), 15);
    assert_eq!(sum_list(vec![10, 20]), 30);
}

#[test] fn test_sum_list_empty() {
    let empty: Vec<int> = vec![];
    assert_eq!(sum_list(empty), 0);
}

#[test] fn test_sum_list_single() {
    assert_eq!(sum_list(vec![42]), 42);
}
`,
    },
    {
      id: 'break-continue',
      title: 'Break & Continue',
      instructions: `## Break & Continue

\`break\` exits a loop immediately. \`continue\` skips to the next iteration, skipping the rest of the current body.

Use \`loop {}\` for an infinite loop that you exit with \`break\`.

**Try it:** Implement \`find_first_even\` that returns the first even number in a \`Vec<int>\` as \`Some(value)\`, or \`None()\` if no even number is found. Use a \`for\` loop with \`break\`.`,
      hints: [
        'Iterate with \`for item in items { ... }\`.',
        'Use \`if item % 2 == 0 { break Some(item); }\` inside the loop.',
        'After the loop, return \`None()\` (nothing was found).',
        'Return type is \`Option<int>\`.',
      ],
      initialCode: `fn find_first_even(items: Vec<int>) -> Option<int> {
    // TODO: return the first even number, or None()
}

fn main() {
    let nums = vec![1, 3, 5, 2, 7];
    let result = find_first_even(nums);
    println!("first even: {}", result.unwrap_or(-1));

    let no_evens = vec![1, 3, 5, 7];
    println!("no evens: {}", find_first_even(no_evens).is_none());
}
`,
      testCode: `#[test] fn test_find_first_even_found() {
    let nums = vec![1, 3, 5, 2, 7];
    assert_eq!(find_first_even(nums), Some(2));
}

#[test] fn test_find_first_even_first() {
    let nums = vec![2, 4, 6];
    assert_eq!(find_first_even(nums), Some(2));
}

#[test] fn test_find_first_even_not_found() {
    let nums = vec![1, 3, 5, 7];
    assert_eq!(find_first_even(nums), None());
}

#[test] fn test_find_first_even_empty() {
    let empty: Vec<int> = vec![];
    assert_eq!(find_first_even(empty), None());
}
`,
    },
  ],
};
