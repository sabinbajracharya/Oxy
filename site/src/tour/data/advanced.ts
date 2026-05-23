import type { Chapter } from '../types';

export const advanced: Chapter = {
  id: 'advanced',
  title: 'Advanced',
  lessons: [
    {
      id: 'macros',
      title: 'Macros',
      instructions: `## Macros

Oxy has built-in macros recognizable by the \`!\` suffix:
- \`println!(fmt, args...)\` — print with newline
- \`print!(fmt, args...)\` — print without newline
- \`format!(fmt, args...)\` — format to String
- \`vec![items...]\` — create a Vec
- \`dbg!(expr)\` — debug-print an expression
- \`eprintln!(fmt, args...)\` — print to stderr

**Try it:** Use \`format!\` to build a complex string from multiple values.`,
      hints: [
        '`dbg!(x)` prints the expression and its value, then returns the value.',
        '`vec![0; 5]` is NOT a macro — that\'s array repeat syntax.',
      ],
      initialCode: `fn main() {
    let name = "Oxy";
    let version = 0.3;

    // format! returns a String
    let msg = format!("{} v{} is running", name, version);
    println!("{}", msg);

    // dbg! prints and returns
    let x = dbg!(2 + 2);
    println!("x = {}", x);

    // vec! macro
    let v = vec![1, 2, 3, 4, 5];
    println!("vec = {}", v);
}
`,
    },
    {
      id: 'attributes',
      title: 'Attributes & Derive',
      instructions: `## Attributes & Derive

Attributes start with \`#\` and modify items:
- \`#[test]\` — marks a function as a test
- \`#[compile_error]\` — marks a test expected to fail compilation
- \`#[derive(Trait1, Trait2)]\` — auto-implements traits

Derivable traits: \`Debug\`, \`Clone\`, \`PartialEq\`, \`Eq\`, \`PartialOrd\`, \`Ord\`, \`Hash\`, \`Display\`.

**Try it:** Add \`PartialEq\` to the derive list and compare two Point values.`,
      hints: [
        '`#[derive(Debug, Clone)]` automatically generates the impls.',
        'Attributes go directly above the item they modify.',
      ],
      initialCode: `#[derive(Debug, Clone)]
struct Point {
    x: float,
    y: float,
}

fn main() {
    let p = Point { x: 3.0, y: 4.0 };
    println!("debug: {:?}", p);

    let p2 = p.clone();
    println!("clone: ({}, {})", p2.x, p2.y);
}
`,
    },
    {
      id: 'const-static',
      title: 'Const, Static, Type Aliases',
      instructions: `## Const, Static, Type Aliases

- \`const NAME: Type = expr;\` — compile-time constant, inlined at use site
- \`static NAME: Type = expr;\` — global variable with a fixed address
- \`type Name = Type;\` — create a type alias

**Try it:** Add a type alias \`Point3D = (float, float, float)\` and use it.`,
      hints: [
        '`const` values must be computable at compile time.',
        '`type` aliases don\'t create new types — they\'re transparent.',
      ],
      initialCode: `const MAX_SIZE: int = 100;
const PI: float = 3.14159265359;

type Score = int;
type Name = String;

fn main() {
    println!("MAX_SIZE = {}", MAX_SIZE);

    let score: Score = 95;
    let name: Name = "Oxy".to_string();
    println!("{} scored {}", name, score);

    if score < MAX_SIZE {
        println!("below max");
    }
}
`,
    },
  ],
};
