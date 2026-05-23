import type { Chapter } from '../types';

export const dataTypes: Chapter = {
  id: 'data-types',
  title: 'Data Types',
  lessons: [
    {
      id: 'primitives',
      title: 'Integer & Float Types',
      instructions: `## Numbers

Oxy has just two integer types — \`int\` (signed, 64-bit wrapping) and \`byte\` (unsigned, 8-bit wrapping) — plus one float type, \`float\` (64-bit).

That's it. No \`i8 / i16 / i32 / u16 / u32 / u64\`, no \`f32\`. The Rust-style width zoo was deliberately retired: pick \`int\` for numbers, \`byte\` for binary data, \`float\` for fractions.

**Try it:** Add a few more values, mix \`int\` and \`float\` in an expression, and see what happens.`,
      hints: [
        'Integer literals default to `int`. Float literals default to `float`.',
        'A byte literal needs an explicit annotation or cast: `let b: byte = 200;` or `200 as byte`.',
        'Use `as int`, `as byte`, `as float` to convert between numeric types.',
      ],
      initialCode: `fn main() {
    let a: int = 42;
    let b = 10; // inferred as int
    println!("a + b = {}", a + b);
    println!("a / b = {}", a / b);

    let x: float = 3.14;
    let y = 2.5;
    println!("x * y = {}", x * y);

    // byte wraps modulo 256
    let small: byte = 255;
    let next: byte = small + 1;
    println!("small + 1 = {}", next);
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

\`as\` can also be used in turbofish syntax: \`collect::<Vec<int>>()\` to specify generic type parameters.

**Try it:** Convert an \`int\` to \`float\` and compute the average of three numbers.`,
      hints: [
        'Casting from float to int truncates: `3.9 as int` → `3`.',
        '`as byte` wraps modulo 256: `300 as byte` → `44`.',
      ],
      initialCode: `fn main() {
    let x: int = 42;
    let y: float = x as float;
    println!("{} as float = {}", x, y);

    let pi: float = 3.14159;
    let approx: int = pi as int;
    println!("{} as int = {}", pi, approx);

    let big: int = 300;
    let wrapped: byte = big as byte;
    println!("{} as byte = {}", big, wrapped);
}
`,
    },
  ],
};
