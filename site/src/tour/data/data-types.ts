import type { Chapter } from '../types';

export const dataTypes: Chapter = {
  id: 'data-types',
  title: 'Data Types',
  lessons: [
    {
      id: 'primitives',
      title: 'Integer & Float Types',
      instructions: `## Numbers

Oxy supports multiple integer widths: \`i8\`, \`i16\`, \`i32\`, \`i64\`, \`u8\`, \`u16\`, \`u32\`, \`u64\`, \`isize\`, \`usize\`.

Floating point: \`f32\` and \`f64\`.

Integer literals default to \`i64\`. Float literals default to \`f64\`. Use type suffixes for other widths.

**Try it:** Change some numbers to \`u8\` and \`f32\` using type suffixes.`,
      hints: [
        'Type suffixes: `42u8`, `100u64`, `3.14f32`.',
        'Arithmetic operators: `+`, `-`, `*`, `/`, `%`.',
      ],
      initialCode: `fn main() {
    let a: i64 = 42;
    let b = 10; // inferred as i64
    println!("a + b = {}", a + b);
    println!("a / b = {}", a / b);

    let x: f64 = 3.14;
    let y = 2.5;
    println!("x * y = {}", x * y);

    // Other widths
    let small: u8 = 255;
    let big: i32 = 100_000;
    println!("small = {}, big = {}", small, big);
}
`,
    },
    {
      id: 'bool-char',
      title: 'Bool & Char',
      instructions: `## Bool & Char

\`bool\` values are \`true\` or \`false\`. Logical operators: \`&&\` (and), \`||\` (or), \`!\` (not).

\`char\` represents a single Unicode character, written with single quotes: \`'a'\`, \`'铁'\`, \`'😀'\`.

Chars have methods like \`is_digit()\`, \`is_alphabetic()\`, \`to_uppercase()\`.

**Try it:** Test a non-ASCII character like \`'世'\` with \`is_alphabetic()\`.`,
      hints: [
        '`&&` and `||` use short-circuit evaluation.',
        'Char methods return `bool` — great in if conditions.',
      ],
      initialCode: `fn main() {
    let t = true;
    let f = false;
    println!("AND: {}, OR: {}, NOT: {}", t && f, t || f, !t);

    let ch = 'A';
    println!("'{}' is uppercase: {}", ch, ch.is_uppercase());
    println!("'{}' is alphabetic: {}", ch, ch.is_alphabetic());

    let lower = ch.to_lowercase();
    println!("lowercase: '{}'", lower);

    let digit = '7';
    println!("'{}' is digit: {}", digit, digit.is_digit());
}
`,
    },
    {
      id: 'strings',
      title: 'Strings',
      instructions: `## Strings

\`String\` is a growable, heap-allocated UTF-8 string. Create one with \`"text".to_string()\` or \`format!(...)\`.

Use \`f"...{expr}..."\` for string interpolation.

String methods: \`len()\`, \`is_empty()\`, \`contains()\`, \`replace()\`, \`split()\`, \`trim()\`, \`to_uppercase()\`, and more.

**Try it:** Use \`replace()\` to change "world" to "Oxy" in the greeting.`,
      hints: [
        '`format!("{} {}", a, b)` is like `println!` but returns a String.',
        'f-strings: `f"Hello {name}!"` interpolates variables directly.',
      ],
      initialCode: `fn main() {
    let s = "Hello, world!".to_string();
    println!("s = {}", s);
    println!("length = {}", s.len());
    println!("contains 'world': {}", s.contains("world"));

    let upper = s.to_uppercase();
    println!("uppercase: {}", upper);

    let trimmed = "  spaces  ".trim().to_string();
    println!("trimmed: '{}'", trimmed);

    // Interpolation
    let name = "Oxy";
    println!("{}", f"Welcome to {name}!");
}
`,
    },
    {
      id: 'casting',
      title: 'Type Casting',
      instructions: `## Type Casting

Use the \`as\` keyword to convert between numeric types.

\`as\` can also be used in turbofish syntax: \`collect::<Vec<i64>>()\` to specify generic type parameters.

**Try it:** Convert an \`i64\` to \`f64\` and compute the average of three numbers.`,
      hints: [
        'Casting from float to int truncates: `3.9 as i64` → `3`.',
        '`as` works between all integer widths and float types.',
      ],
      initialCode: `fn main() {
    let x: i64 = 42;
    let y: f64 = x as f64;
    println!("{} as f64 = {}", x, y);

    let pi: f64 = 3.14159;
    let approx: i64 = pi as i64;
    println!("{} as i64 = {}", pi, approx);

    let a: i32 = 100;
    let b: i64 = a as i64;
    println!("i32 -> i64: {}", b);
}
`,
    },
  ],
};
