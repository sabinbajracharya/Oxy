import type { Chapter } from '../types';

export const enumsMatching: Chapter = {
  id: 'enums-matching',
  title: 'Enums & Pattern Matching',
  lessons: [
    {
      id: 'enum-def',
      title: 'Defining Enums',
      instructions: `## Defining Enums

An \`enum\` defines a type that can be one of several **variants**:

\`\`\`
enum Color {
    Red,
    Green,
    Blue,
}
\`\`\`

Create a variant with the \`::\` syntax: \`let c = Color::Red;\`

Check which variant you have using \`match\`:

\`\`\`
match c {
    Color::Red => true,
    _ => false,
}
\`\`\`

**Your task:**

1. Define \`enum Color { Red, Green, Blue }\`
2. Implement \`fn is_red(c: Color) -> bool\` that returns \`true\` if \`c\` is \`Color::Red\`, \`false\` otherwise`,
      hints: [
        'Use `match c { Color::Red => true, _ => false }`.',
        'The `_` wildcard matches any remaining variant.',
        'Enum variants are namespaced: `Color::Red`.',
      ],
      initialCode: `// TODO: define enum Color { Red, Green, Blue }

fn is_red(c: Color) -> bool {
    // TODO: match on c and return true for Red, false otherwise
    false
}

fn main() {
    let c = Color::Red;
    println!("is red? {}", is_red(c));
    println!("is green? {}", is_red(Color::Green));
}
`,
      testCode: `#[test] fn test_is_red_returns_true_for_red() {
    assert!(is_red(Color::Red));
}

#[test] fn test_is_red_returns_false_for_green() {
    assert!(!is_red(Color::Green));
}

#[test] fn test_is_red_returns_false_for_blue() {
    assert!(!is_red(Color::Blue));
}
`,
    },
    {
      id: 'enum-data',
      title: 'Enums with Data',
      instructions: `## Enums Carrying Data

Enum variants can hold values. Each variant can carry different types:

\`\`\`
enum Shape {
    Circle(float),           // tuple variant — holds a radius
    Rectangle(int, int),     // tuple variant — holds width and height
}
\`\`\`

Use \`match\` to extract the data:

\`\`\`
match s {
    Shape::Circle(r) => 3.14159 * r * r,
    Shape::Rectangle(w, h) => (w as float) * (h as float),
}
\`\`\`

**Your task:**

1. Define \`enum Shape { Circle(float), Rectangle(float, float) }\`
2. Implement \`fn area(s: Shape) -> float\`:
   - \`Circle(r)\`: return \`3.14159 * r * r\`
   - \`Rectangle(w, h)\`: return \`w * h\``,
      hints: [
        'Match on `s` and destructure each variant: `Shape::Circle(r) => ...`.',
        'For Circle, use `3.14159 * r * r`.',
        'For Rectangle, use `w * h`.',
      ],
      initialCode: `// TODO: define enum Shape { Circle(float), Rectangle(float, float) }

fn area(s: Shape) -> float {
    // TODO: match and calculate area for each variant
    0.0
}

fn main() {
    let c = Shape::Circle(5.0);
    println!("circle area = {}", area(c));

    let r = Shape::Rectangle(3.0, 4.0);
    println!("rect area = {}", area(r));
}
`,
      testCode: `#[test] fn test_circle_area() {
    let c = Shape::Circle(5.0);
    let a = area(c);
    assert!((a - 78.53975).abs() < 0.001);
}

#[test] fn test_rectangle_area() {
    let r = Shape::Rectangle(3.0, 4.0);
    assert!((area(r) - 12.0).abs() < 0.001);
}

#[test] fn test_circle_zero() {
    let c = Shape::Circle(0.0);
    assert!(area(c) == 0.0);
}

#[test] fn test_rectangle_zero_width() {
    let r = Shape::Rectangle(0.0, 5.0);
    assert!(area(r) == 0.0);
}
`,
    },
    {
      id: 'match-basics',
      title: 'Match Basics',
      instructions: `## Match Expressions

\`match\` compares a value against a series of **patterns**. The first matching arm executes:

\`\`\`
match value {
    0 => "zero",
    1 => "one",
    _ => "something else",
}
\`\`\`

Key rules:
- Match arms are written as \`pattern => expression\`
- The last expression in each arm is the result (no semicolon)
- Match must be **exhaustive** — all possibilities must be covered
- Use \`_\` as the wildcard arm to catch everything else

\`match\` is an expression — it returns a value.

**Your task:**

Implement \`fn describe_number(n: int) -> String\` using \`match\`:
- \`0\` → \`"zero"\`
- \`1\` → \`"one"\`
- \`2\` → \`"two"\`
- Everything else → \`"other"\``,
      hints: [
        'Use `match n { 0 => ..., 1 => ..., 2 => ..., _ => ... }`.',
        'Return the string: `"zero".to_string()`.',
        'The wildcard `_` catches everything not listed above.',
      ],
      initialCode: `fn describe_number(n: int) -> String {
    // TODO: match on n:
    //   0 -> "zero"
    //   1 -> "one"
    //   2 -> "two"
    //   _ -> "other"
    "".to_string()
}

fn main() {
    for n in [0, 1, 2, 42] {
        println!("{}: {}", n, describe_number(n));
    }
}
`,
      testCode: `#[test] fn test_zero() {
    assert!(describe_number(0) == "zero".to_string());
}

#[test] fn test_one() {
    assert!(describe_number(1) == "one".to_string());
}

#[test] fn test_two() {
    assert!(describe_number(2) == "two".to_string());
}

#[test] fn test_other_positive() {
    assert!(describe_number(42) == "other".to_string());
}

#[test] fn test_other_negative() {
    assert!(describe_number(-5) == "other".to_string());
}
`,
    },
    {
      id: 'match-guards',
      title: 'Match Guards',
      instructions: `## Match Guards

Add an \`if\` condition to a match arm with a **guard**:

\`\`\`
match n {
    n if n < 0 => "negative".to_string(),
    n if n == 0 => "zero".to_string(),
    _ => "positive".to_string(),
}
\`\`\`

The guard evaluates after the pattern matches. If the guard returns \`false\`, match continues to the next arm.

Guards let you express conditions that range patterns alone cannot capture.

**Your task:**

Implement \`fn classify_range(n: int) -> String\` using match with guards:

| Condition | Result |
|-----------|--------|
| \`n < 0\` | \`"negative"\` |
| \`n == 0\` | \`"zero"\` |
| \`n <= 10\` | \`"low"\` |
| \`n <= 100\` | \`"medium"\` |
| everything else | \`"high"\` |

Write guards like \`n if n < 0 => "negative".to_string()\`
`,
      hints: [
        'Guard syntax: `n if condition => expression` — note no comma before `if`.',
        'The first arm with a matching pattern AND a true guard wins.',
        'Order matters — put more specific conditions first.',
      ],
      initialCode: `fn classify_range(n: int) -> String {
    // TODO: use match with guards for each range
    "".to_string()
}

fn main() {
    for n in [-5, 0, 5, 50, 200] {
        println!("{}: {}", n, classify_range(n));
    }
}
`,
      testCode: `#[test] fn test_negative() {
    assert!(classify_range(-5) == "negative".to_string());
}

#[test] fn test_zero() {
    assert!(classify_range(0) == "zero".to_string());
}

#[test] fn test_low() {
    assert!(classify_range(7) == "low".to_string());
}

#[test] fn test_low_boundary() {
    assert!(classify_range(10) == "low".to_string());
}

#[test] fn test_medium() {
    assert!(classify_range(50) == "medium".to_string());
}

#[test] fn test_medium_boundary() {
    assert!(classify_range(100) == "medium".to_string());
}

#[test] fn test_high() {
    assert!(classify_range(200) == "high".to_string());
}

#[test] fn test_high_very_large() {
    assert!(classify_range(9999) == "high".to_string());
}
`,
    },
    {
      id: 'or-patterns',
      title: 'Or Patterns',
      instructions: `## Or Patterns with \`|\`

Use \`|\` (pipe) in a match arm to match one of several patterns:

\`\`\`
match c {
    'a' | 'e' | 'i' | 'o' | 'u' => "vowel",
    _ => "consonant",
}
\`\`\`

The pipe reads as "or" — the arm matches if any of the patterns match.

Or patterns keep your match blocks compact when multiple values should produce the same result.

**Your task:**

Implement \`fn is_vowel(c: char) -> bool\` using match with or patterns:
- Vowels: \`'a'\`, \`'e'\`, \`'i'\`, \`'o'\`, \`'u'\` → return \`true\`
- Everything else → return \`false\``,
      hints: [
        "Use `'a' | 'e' | 'i' | 'o' | 'u' => true`.",
        'The wildcard `_ => false` catches consonants.',
        "Use single quotes for char literals: `'a'`.",
      ],
      initialCode: `fn is_vowel(c: char) -> bool {
    // TODO: use match with or patterns to check for vowels
    false
}

fn main() {
    println!("a: {}", is_vowel('a'));
    println!("b: {}", is_vowel('b'));
    println!("z: {}", is_vowel('z'));
}
`,
      testCode: `#[test] fn test_all_vowels() {
    assert!(is_vowel('a'));
    assert!(is_vowel('e'));
    assert!(is_vowel('i'));
    assert!(is_vowel('o'));
    assert!(is_vowel('u'));
}

#[test] fn test_consonants() {
    assert!(!is_vowel('b'));
    assert!(!is_vowel('z'));
    assert!(!is_vowel('m'));
    assert!(!is_vowel('k'));
    assert!(!is_vowel('t'));
}
`,
    },
    {
      id: 'if-let',
      title: 'If Let',
      instructions: `## If Let

\`if let\` is a concise way to match a single pattern and ignore the rest.

Instead of:

\`\`\`
match opt {
    Some(val) => do_something(val),
    _ => {},
}
\`\`\`

Write:

\`\`\`
if let Some(val) = opt {
    do_something(val);
}
\`\`\`

Add an \`else\` branch to handle the non-matching case. This pairs naturally with \`Option<T>\`.

**Your task:**

Implement \`fn print_if_some(opt: Option<int>) -> String\`:
- If \`opt\` is \`Some(val)\`, return \`format!("Got: {}", val)\`
- If \`opt\` is \`None\`, return \`"Nothing".to_string()\`

Use \`if let Some(val) = opt { ... } else { ... }\`.`,
      hints: [
        'Use `if let Some(val) = opt { ... } else { ... }`.',
        '`format!("Got: {}", val)` converts the value to a string.',
        'Return `"Nothing".to_string()` in the else branch.',
      ],
      initialCode: `fn print_if_some(opt: Option<int>) -> String {
    // TODO: use if let to extract the value or return "Nothing"
    "".to_string()
}

fn main() {
    println!("{}", print_if_some(Some(42)));
    println!("{}", print_if_some(None()));
}
`,
      testCode: `#[test] fn test_some_value() {
    let r = print_if_some(Some(42));
    assert!(r == "Got: 42".to_string());
}

#[test] fn test_none() {
    let r = print_if_some(None());
    assert!(r == "Nothing".to_string());
}

#[test] fn test_some_zero() {
    let r = print_if_some(Some(0));
    assert!(r == "Got: 0".to_string());
}

#[test] fn test_some_negative() {
    let r = print_if_some(Some(-7));
    assert!(r == "Got: -7".to_string());
}
`,
    },
  ],
};
