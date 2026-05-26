import type { Chapter } from '../types';

export const gettingStarted: Chapter = {
  id: 'getting-started',
  title: 'Getting Started',
  lessons: [
    {
      id: 'hello-world',
      title: 'Hello, World!',
      instructions: `## Hello, World!

Every Oxy program starts with a **main** function. This is the entry point that runs when you execute your program.

The \`println!\` macro prints text to stdout, followed by a newline.

**Try it:** Click Run to see the output. Then try changing the message inside the quotes.`,
      hints: [
        'Macros in Oxy end with \`!\` — like \`println!\`, \`format!\`, and \`vec!\`.',
        'Strings are wrapped in double quotes: \`"Hello"\`.',
      ],
      initialCode: `fn main() {
    println!("Hello, Oxy!");
}
`,
      testCode: `#[test] fn test_compiles() {
    assert!(true);
}
`,
    },
    {
      id: 'variables',
      title: 'Variables & Mutability',
      instructions: `## Variables & Mutability

Use \`let\` to declare variables. By default, variables are **immutable** — you cannot reassign them.

Add \`mut\` after \`let\` to make a variable **mutable** so you can change its value.

**Try it:** The code below tries to reassign \`x\`, but \`x\` is immutable. Fix it by adding \`mut\` to the \`let\` declaration.`,
      hints: [
        'Add \`mut\` between \`let\` and the variable name: \`let mut x = 42;\`.',
        'Only make a variable mutable if you need to reassign it.',
      ],
      initialCode: `fn main() {
    let x = 42;
    x = 43;
    println!("x is now {}", x);
}
`,
      testCode: `#[test] fn test_compiles_and_runs() {
    assert!(true);
}
`,
    },
    {
      id: 'type-annotations',
      title: 'Type Annotations',
      instructions: `## Type Annotations

Oxy is statically typed. The compiler infers types for you, but you can also write them explicitly.

Function parameters **must** have type annotations. The return type goes after \`->\`.

**Try it:** Add the missing type annotations to \`add\`'s parameters so the code compiles.`,
      hints: [
        'Parameter types go after a colon: \`param: Type\`.',
        'Both parameters are integers: use \`int\`.',
      ],
      initialCode: `fn add(a, b) -> int {
    a + b
}

fn main() {
    let result = add(3, 4);
    println!("3 + 4 = {}", result);
}
`,
      testCode: `#[test] fn test_add() {
    assert_eq!(add(3, 4), 7);
}

#[test] fn test_add_negative() {
    assert_eq!(add(-2, 5), 3);
}
`,
    },
    {
      id: 'comments',
      title: 'Comments',
      instructions: `## Comments

Oxy uses \`//\` for single-line comments. Everything after \`//\` on the same line is ignored by the compiler.

Comments are great for documenting your code or temporarily disabling lines while debugging.

**Try it:** Comment out the second \`println!\` line so only the greeting is printed.`,
      hints: [
        'Place \`//\` at the start of a line to comment out the whole line.',
        'You can also put \`//\` after code for an inline comment.',
      ],
      initialCode: `fn main() {
    println!("Hello!");
    println!("Please comment me out!");
}
`,
      testCode: `#[test] fn test_compiles() {
    assert!(true);
}
`,
    },
  ],
};
