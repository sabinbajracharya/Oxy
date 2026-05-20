import type { Chapter } from '../types';

export const structsEnums: Chapter = {
  id: 'structs-enums',
  title: 'Structs & Enums',
  lessons: [
    {
      id: 'struct-def',
      title: 'Struct Definition',
      instructions: `## Defining Structs

Define a struct with \`struct Name { fields }\`. Create instances with \`Name { field: value }\`.

Struct fields can have type annotations. Use \`pub\` to make fields publicly accessible.

**Try it:** Add a \`z: f64\` field to Point for a 3D point.`,
      hints: [
        'Struct field access: `point.x`.',
        'Tuple structs: `struct Pair(i64, i64);`.',
        'Unit structs: `struct Marker;`.',
      ],
      initialCode: `struct Point {
    x: f64,
    y: f64,
}

fn main() {
    let p = Point { x: 3.0, y: 4.0 };
    println!("Point({}, {})", p.x, p.y);

    // Tuple struct
    let pair = Pair(10, 20);
    println!("Pair({}, {})", pair.0, pair.1);
}

struct Pair(i64, i64);
`,
    },
    {
      id: 'impl-blocks',
      title: 'Impl Blocks',
      instructions: `## Impl Blocks

Add methods to structs with \`impl Type { ... }\`. Methods take \`self\` as the first parameter.

Use \`Self\` inside impl blocks to refer to the implementing type. \`Self\` works in return types too.

**Try it:** Add a \`scale(factor: f64)\` method that multiplies both x and y.`,
      hints: [
        '`self` gives access to fields.',
        'Associated functions (no `self`) are called with `Type::func()`.',
      ],
      initialCode: `struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    fn distance_from_origin(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

fn main() {
    let p = Point::new(3.0, 4.0);
    println!("distance: {}", p.distance_from_origin());
}
`,
    },
    {
      id: 'enum-def',
      title: 'Enum Definition',
      instructions: `## Enums

\`enum\` defines a type that can be one of several **variants**.

Variants can hold data: no data (unit variant), unnamed data (tuple variant), or named fields (struct variant).

**Try it:** Add a \`Triangle\` variant that holds three Point values.`,
      hints: [
        'Enum variants are accessed with `EnumName::Variant`.',
        'Match on enums to extract the inner data.',
      ],
      initialCode: `enum Shape {
    Circle(f64),            // radius
    Rectangle { w: f64, h: f64 },
    Nothing,                // no data
}

fn area(s: Shape) -> f64 {
    match s {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Rectangle { w, h } => w * h,
        Shape::Nothing => 0.0,
    }
}

fn main() {
    let c = Shape::Circle(5.0);
    println!("circle area: {}", area(c));

    let r = Shape::Rectangle { w: 4.0, h: 6.0 };
    println!("rect area: {}", area(r));
}
`,
    },
    {
      id: 'enum-data',
      title: 'Enum with Data',
      instructions: `## Enums Carrying Data

Each enum variant can carry different types of data. Pattern matching destructures the variant and binds the data to variables.

This is the foundation of \`Option<T>\` and \`Result<T, E>\`.

**Try it:** Add a \`Mul(Box<Expr>, Box<Expr>)\` variant for multiplication.`,
      hints: [
        'Use `Box<T>` for recursive types — it allocates on the heap.',
        'Match arms can destructure nested data.',
      ],
      initialCode: `enum Expr {
    Int(i64),
    Add(Box<Expr>, Box<Expr>),
}

fn eval(e: Expr) -> i64 {
    match e {
        Expr::Int(n) => n,
        Expr::Add(a, b) => eval(*a) + eval(*b),
    }
}

fn main() {
    let expr = Expr::Add(
        Box::new(Expr::Int(3)),
        Box::new(Expr::Int(4)),
    );
    println!("result = {}", eval(expr));
}
`,
    },
    {
      id: 'match-patterns',
      title: 'Advanced Pattern Matching',
      instructions: `## Advanced Patterns

Match supports many pattern types:
- \`Literal\` — match exact values
- \`x\` — bind to a variable
- \`_\` — wildcard (match anything, ignore)
- \`Enum::Variant(inner)\` — destructure variants
- \`Struct { field }\` — destructure structs
- \`a | b\` — OR patterns
- \`start..=end\` — range patterns
- Match guards: \`if condition\`

**Try it:** Add a match arm with a guard: numbers > 50 but only if they're even.`,
      hints: [
        '`|` in patterns means "or": `1 | 2 | 3`.',
        'Guards: `n if n % 2 == 0 => ...`.',
      ],
      initialCode: `fn describe(n: i64) -> String {
    match n {
        0 => "zero".to_string(),
        1 | 2 => "one or two".to_string(),
        3..=9 => "small digit".to_string(),
        n if n > 0 => f"positive {}" .to_string(),
        _ => "negative".to_string(),
    }
}

fn main() {
    for n in [0, 2, 5, 42, -3] {
        println!("{}: {}", n, describe(n));
    }
}
`,
    },
  ],
};
