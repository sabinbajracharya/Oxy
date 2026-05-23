import type { Chapter } from '../types';

export const basics: Chapter = {
  id: 'basics',
  title: 'Basics',
  lessons: [
    {
      id: 'hello-world',
      title: 'Hello, World!',
      instructions: `## Hello, World!

Every Oxy program starts with a **main** function. This is the entry point that runs when you execute your program.

The \`println!\` macro prints text to stdout, followed by a newline.

**Try it:** Click Run to see the output. Then try changing the message.`,
      hints: ['Macros in Oxy end with `!` — like `println!`, `format!`, and `vec!`.'],
      initialCode: `fn main() {
    println!("Hello, Oxy!");
}
`,
    },
    {
      id: 'variables',
      title: 'Variables',
      instructions: `## Variables

Use \`let\` to declare variables. By default, variables are **immutable** — you cannot reassign them.

Add \`mut\` to make a variable **mutable** so you can change its value.

Oxy has **type inference** — types are inferred from the value, but you can also write them explicitly.

**Try it:** Uncomment the line that tries to change \`x\`. What happens? Then add \`mut\` to fix it.`,
      hints: [
        'Add `mut` before the variable name to make it reassignable.',
        'Type annotations go after a colon: `let x: int = 42;`',
      ],
      initialCode: `fn main() {
    let x = 42;
    println!("x = {}", x);

    let mut y = 10;
    y = y + 1;
    println!("y = {}", y);

    let name: String = "Oxy".to_string();
    println!("Hello, {}!", name);
}
`,
    },
    {
      id: 'type-annotations',
      title: 'Type Annotations',
      instructions: `## Type Annotations

Oxy is statically typed. The compiler infers types for you, but you can (and sometimes must) write them explicitly.

Common built-in types:
- \`int\`, \`float\` — integers and floats
- \`bool\` — \`true\` or \`false\`
- \`String\` — heap-allocated text
- \`char\` — a single Unicode character

**Try it:** Change the return type annotation to something wrong and see the error.`,
      hints: [
        'Integer literals default to `int`. Use type suffixes like `42u64` for other widths.',
        'Function return types go after `->`.',
      ],
      initialCode: `fn square(x: int) -> int {
    x * x
}

fn main() {
    let n: int = 7;
    let result: int = square(n);
    println!("{} squared is {}", n, result);

    let pi: float = 3.14159;
    let flag: bool = true;
    println!("pi = {}, flag = {}", pi, flag);
}
`,
    },
    {
      id: 'comments',
      title: 'Comments',
      instructions: `## Comments

Oxy uses \`//\` for single-line comments. There are no multi-line comment syntaxes — use multiple \`//\` lines.

Comments are ignored by the compiler. Use them to document your code or temporarily disable lines.

**Try it:** Comment out the second \`println!\` call.`,
      hints: ['Use `//` at the start of a line (or after code) to comment it out.'],
      initialCode: `// This is a comment
fn main() {
    // Print a greeting
    println!("Hello!"); // inline comment

    println!("This line runs");
    // println!("This line is commented out");
}
`,
    },
  ],
};
