import type { Chapter } from '../types';

export const primitiveTypes: Chapter = {
  id: 'primitive-types',
  title: 'Primitive Types',
  lessons: [
    {
      id: 'numbers',
      title: 'Numbers',
      instructions: `## Numbers

Oxy has three numeric types:
- \`int\` — signed 64-bit integer
- \`float\` — 64-bit IEEE-754 floating point
- \`byte\` — unsigned 8-bit integer

Standard arithmetic operators work: \`+\`, \`-\`, \`*\`, \`/\`, \`%\`.

**Try it:** Implement \`square\` (returns x \* x) and \`cube\` (returns x \* x \* x) using the correct types.`,
      hints: [
        'Use \`int\` for integer parameters and \`float\` for float parameters.',
        'The return type must match: \`square\` returns \`int\`, \`cube\` returns \`float\`.',
      ],
      initialCode: `fn square(x: int) -> int {
    // TODO: return x * x
}

fn cube(x: float) -> float {
    // TODO: return x * x * x
}

fn main() {
    println!("square(5) = {}", square(5));
    println!("cube(3.0) = {}", cube(3.0));
}
`,
      testCode: `#[test] fn test_square() {
    assert_eq!(square(0), 0);
    assert_eq!(square(5), 25);
    assert_eq!(square(-3), 9);
}

#[test] fn test_cube() {
    assert!(cube(0.0) == 0.0);
    assert!(cube(3.0) == 27.0);
    assert!(cube(-2.0) == -8.0);
}
`,
    },
    {
      id: 'booleans',
      title: 'Booleans & Comparisons',
      instructions: `## Booleans & Comparisons

The \`bool\` type has two values: \`true\` and \`false\`.

Comparison operators return \`bool\`: \`==\`, \`!=\`, \`<\`, \`<=\`, \`>\`, \`>=\`.

Combine conditions with \`&&\` (and) and \`||\` (or). Negate with \`!\`.

**Try it:** Implement \`is_even\` (true when n is even) and \`in_range\` (true when lo <= n < hi).`,
      hints: [
        'Use \`% 2 == 0\` to test if a number is even.',
        'For \`in_range\`, combine two comparisons with \`&&\`.',
      ],
      initialCode: `fn is_even(n: int) -> bool {
    // TODO: return true when n is even
}

fn in_range(n: int, lo: int, hi: int) -> bool {
    // TODO: return true when lo <= n < hi
}

fn main() {
    println!("is_even(4) = {}", is_even(4));
    println!("is_even(7) = {}", is_even(7));
    println!("in_range(5, 1, 10) = {}", in_range(5, 1, 10));
}
`,
      testCode: `#[test] fn test_is_even() {
    assert!(is_even(0));
    assert!(is_even(4));
    assert!(is_even(-2));
    assert!(!is_even(1));
    assert!(!is_even(7));
}

#[test] fn test_in_range() {
    assert!(in_range(5, 1, 10));
    assert!(in_range(1, 1, 10));
    assert!(!in_range(0, 1, 10));
    assert!(!in_range(10, 1, 10));
    assert!(!in_range(20, 1, 10));
}
`,
    },
    {
      id: 'chars',
      title: 'Characters',
      instructions: `## Characters

The \`char\` type represents a single Unicode character. Use single quotes: \`'a'\`, \`'Z'\`, \`'\\n'\`.

You can compare \`char\` values with \`==\`, test ranges, and match on them.

**Try it:** Implement \`is_vowel\` that returns \`true\` for vowels (\`a\`, \`e\`, \`i\`, \`o\`, \`u\`) in any case.`,
      hints: [
        "Compare with `c == 'a' || c == 'e'` for each vowel.",
        "Check both lowercase and uppercase vowels: `'a'` and `'A'`.",
        'For a shorter solution, convert to lowercase first then check: there is no built-in, so check both cases explicitly.',
      ],
      initialCode: `fn is_vowel(c: char) -> bool {
    // TODO: return true if c is a vowel (a, e, i, o, u, in any case)
}

fn main() {
    println!("is_vowel('a') = {}", is_vowel('a'));
    println!("is_vowel('b') = {}", is_vowel('b'));
    println!("is_vowel('E') = {}", is_vowel('E'));
}
`,
      testCode: `#[test] fn test_vowels_lowercase() {
    assert!(is_vowel('a'));
    assert!(is_vowel('e'));
    assert!(is_vowel('i'));
    assert!(is_vowel('o'));
    assert!(is_vowel('u'));
}

#[test] fn test_vowels_uppercase() {
    assert!(is_vowel('A'));
    assert!(is_vowel('E'));
    assert!(is_vowel('I'));
    assert!(is_vowel('O'));
    assert!(is_vowel('U'));
}

#[test] fn test_consonants() {
    assert!(!is_vowel('b'));
    assert!(!is_vowel('z'));
    assert!(!is_vowel('B'));
    assert!(!is_vowel('1'));
}
`,
    },
    {
      id: 'strings-intro',
      title: 'Strings Intro',
      instructions: `## Strings

The \`String\` type holds heap-allocated text. Create one with \`.to_string()\` on a string literal, or \`format!\`.

Concatenate strings with \`+\` (the first operand must be a \`String\`).

**Common methods:**
- \`len()\` — returns the byte length
- \`push_str(&str)\` — but without references in Oxy, use \`+\`

**Try it:** Implement \`greet\` that returns \`"Hello, {name}!"\` where \`{name}\` is the parameter.`,
      hints: [
        'Concatenation: \`"Hello, ".to_string() + &name + "!"\` — but Oxy has no references, so just \`"Hello, ".to_string() + name + "!"\` ... actually \`name\` moves. Use \`format!\`: \`format!("Hello, {}!", name)\`.',
        'Or: \`let mut s = "Hello, ".to_string(); s = s + name; s = s + "!"; s\`',
      ],
      initialCode: `fn greet(name: String) -> String {
    // TODO: return "Hello, {name}!"
}

fn main() {
    let greeting = greet("Oxy".to_string());
    println!("{}", greeting);
}
`,
      testCode: `#[test] fn test_greet_world() {
    assert_eq!(greet("World".to_string()), "Hello, World!");
}

#[test] fn test_greet_oxy() {
    assert_eq!(greet("Oxy".to_string()), "Hello, Oxy!");
}

#[test] fn test_greet_empty() {
    assert_eq!(greet("".to_string()), "Hello, !");
}
`,
    },
    {
      id: 'type-casting',
      title: 'Type Casting',
      instructions: `## Type Casting

Convert between types using \`as\`:
- \`expr as int\` — convert to integer
- \`expr as float\` — convert to float
- \`expr as byte\` — convert to byte

Parse strings to integers with \`parse_int\`. Convert numbers to strings with \`.to_string()\`.

**Try it:** Implement \`to_celsius\` that converts Fahrenheit to Celsius: \`(f - 32.0) * 5.0 / 9.0\`. Then implement \`parse_and_double\` that parses a string to an integer and returns twice the value.`,
      hints: [
        'Use \`as float\` if you need to cast an \`int\` to \`float\` for division.',
        'Use \`parse_int(s)\` to convert a String to int. It returns \`Option<int>\`.',
        'Use \`.unwrap()\` on the Option to get the int value.',
      ],
      initialCode: `fn to_celsius(f: float) -> float {
    // TODO: convert Fahrenheit to Celsius
}

fn parse_and_double(s: String) -> int {
    // TODO: parse the string as int and return double the value
}

fn main() {
    println!("32F = {}C", to_celsius(32.0));
    println!("double of \"21\" = {}", parse_and_double("21".to_string()));
}
`,
      testCode: `#[test] fn test_to_celsius() {
    let result = to_celsius(32.0);
    assert!(result == 0.0);

    let result = to_celsius(212.0);
    assert!(result == 100.0);

    let result = to_celsius(-40.0);
    assert!(result == -40.0);
}

#[test] fn test_parse_and_double() {
    assert_eq!(parse_and_double("21".to_string()), 42);
    assert_eq!(parse_and_double("0".to_string()), 0);
    assert_eq!(parse_and_double("50".to_string()), 100);
}
`,
    },
  ],
};
