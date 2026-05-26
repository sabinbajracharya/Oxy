import type { Chapter } from '../types';

export const structs: Chapter = {
  id: 'structs',
  title: 'Structs',
  lessons: [
    {
      id: 'struct-def',
      title: 'Defining Structs',
      instructions: `## Defining Structs

A **struct** groups related data into a single type. Named-field structs use curly braces:

\`\`\`
struct Rectangle {
    width: int,
    height: int,
}
\`\`\`

Create an instance by naming the fields:

\`\`\`
let r = Rectangle { width: 5, height: 10 };
\`\`\`

Access fields with dot notation: \`r.width\`, \`r.height\`.

**Your task:**

1. Define a \`struct Rectangle\` with fields \`width: int\` and \`height: int\`
2. Implement \`fn area(r: Rectangle) -> int\` that returns \`width * height\``,
      hints: [
        'Struct fields are comma-separated inside curly braces.',
        'Access fields: `r.width`, `r.height`.',
        'The area of a rectangle is width times height.',
      ],
      initialCode: `// TODO: define struct Rectangle with width and height fields

fn area(r: Rectangle) -> int {
    // TODO: return width * height
    0
}

fn main() {
    let r = Rectangle { width: 5, height: 10 };
    println!("area = {}", area(r));
}
`,
      testCode: `#[test] fn test_area_basic() {
    let r = Rectangle { width: 4, height: 7 };
    assert!(area(r) == 28);
}

#[test] fn test_area_zero_width() {
    let r = Rectangle { width: 0, height: 10 };
    assert!(area(r) == 0);
}

#[test] fn test_area_zero_height() {
    let r = Rectangle { width: 10, height: 0 };
    assert!(area(r) == 0);
}

#[test] fn test_area_square() {
    let r = Rectangle { width: 6, height: 6 };
    assert!(area(r) == 36);
}
`,
    },
    {
      id: 'field-mutation',
      title: 'Field Mutation',
      instructions: `## Mutating Struct Fields

To modify a struct's fields, the instance must be declared (or passed) as \`mut\`.

\`\`\`
let mut r = Rectangle { width: 5, height: 10 };
r.width = 20; // works because r is mut
\`\`\`

Without \`mut\`, field assignment is a compile error.

**Your task:**

Implement \`fn scale(mut r: Rectangle, factor: int) -> Rectangle\` that:
1. Multiplies both \`r.width\` and \`r.height\` by \`factor\`
2. Returns the modified \`r\`

> You need \`mut r\` on the parameter so you can assign to the fields.`,
      hints: [
        'Use `r.width = r.width * factor` to update a field.',
        'The `mut` on the parameter lets you modify fields.',
        'After updating, return `r` directly.',
      ],
      initialCode: `struct Rectangle {
    width: int,
    height: int,
}

fn scale(mut r: Rectangle, factor: int) -> Rectangle {
    // TODO: multiply both width and height by factor, then return r
    r
}

fn main() {
    let r = Rectangle { width: 5, height: 10 };
    let scaled = scale(r, 3);
    println!("scaled: {} x {}", scaled.width, scaled.height);
}
`,
      testCode: `#[test] fn test_scale_by_two() {
    let r = Rectangle { width: 3, height: 7 };
    let s = scale(r, 2);
    assert!(s.width == 6);
    assert!(s.height == 14);
}

#[test] fn test_scale_by_one() {
    let r = Rectangle { width: 10, height: 20 };
    let s = scale(r, 1);
    assert!(s.width == 10);
    assert!(s.height == 20);
}

#[test] fn test_scale_by_zero() {
    let r = Rectangle { width: 5, height: 8 };
    let s = scale(r, 0);
    assert!(s.width == 0);
    assert!(s.height == 0);
}
`,
    },
    {
      id: 'impl-blocks',
      title: 'Impl Blocks',
      instructions: `## Methods with Impl Blocks

Attach functions to a struct with an \`impl\` block:

\`\`\`
impl Rectangle {
    fn area(self) -> int {
        self.width * self.height
    }
}
\`\`\`

Methods take \`self\` as their first parameter. Call them with dot notation: \`rect.area()\`.

**Your task:**

Add an \`impl Rectangle\` block with two methods:
- \`fn area(self) -> int\` — returns \`width * height\`
- \`fn perimeter(self) -> int\` — returns \`2 * (width + height)\`

> \`self\` gives you access to the struct's fields inside the method.`,
      hints: [
        'Write `impl Rectangle {` then your methods, then `}`.',
        'Access fields with `self.field_name`.',
        'Perimeter formula: 2 * (width + height).',
      ],
      initialCode: `struct Rectangle {
    width: int,
    height: int,
}

// TODO: add impl Rectangle with area() and perimeter() methods

fn main() {
    let r = Rectangle { width: 5, height: 10 };
    println!("area = {}", r.area());
    println!("perimeter = {}", r.perimeter());
}
`,
      testCode: `#[test] fn test_area_method() {
    let r = Rectangle { width: 4, height: 7 };
    assert!(r.area() == 28);
}

#[test] fn test_perimeter_method() {
    let r = Rectangle { width: 3, height: 8 };
    assert!(r.perimeter() == 22);
}

#[test] fn test_area_zero() {
    let r = Rectangle { width: 0, height: 10 };
    assert!(r.area() == 0);
}

#[test] fn test_perimeter_square() {
    let r = Rectangle { width: 5, height: 5 };
    assert!(r.perimeter() == 20);
}
`,
    },
    {
      id: 'tuple-structs',
      title: 'Tuple Structs',
      instructions: `## Tuple Structs

A **tuple struct** looks like a struct but uses parentheses instead of named fields:

\`\`\`
struct Pair(int, int);
\`\`\`

Create an instance with parentheses: \`let p = Pair(10, 20);\`

Access fields by position: \`p.0\`, \`p.1\`.

Tuple structs are great for lightweight wrappers around a fixed number of values.

**Your task:**

1. Define a tuple struct \`struct Pair(int, int)\`
2. Implement \`fn sum_pair(p: Pair) -> int\` that returns \`p.0 + p.1\`

> Note the semicolon after the tuple struct definition — it's required!`,
      hints: [
        'Tuple struct syntax: `struct Name(Type1, Type2);` with semicolon.',
        'Access positional fields: `p.0`, `p.1`.',
        'The sum is simply `p.0 + p.1`.',
      ],
      initialCode: `// TODO: define a tuple struct Pair(int, int)

fn sum_pair(p: Pair) -> int {
    // TODO: return p.0 + p.1
    0
}

fn main() {
    let p = Pair(3, 7);
    println!("sum = {}", sum_pair(p));
}
`,
      testCode: `#[test] fn test_sum_pair_basic() {
    let p = Pair(10, 20);
    assert!(sum_pair(p) == 30);
}

#[test] fn test_sum_pair_zeros() {
    let p = Pair(0, 0);
    assert!(sum_pair(p) == 0);
}

#[test] fn test_sum_pair_negative() {
    let p = Pair(-5, 5);
    assert!(sum_pair(p) == 0);
}

#[test] fn test_sum_pair_large() {
    let p = Pair(1000, 2000);
    assert!(sum_pair(p) == 3000);
}
`,
    },
    {
      id: 'struct-update',
      title: 'Struct Update Syntax',
      instructions: `## Struct Update Syntax

When creating a struct from an existing instance, use \`..other\` to copy the remaining fields:

\`\`\`
let moved = Point { x: new_x, y: new_y, ..original };
\`\`\`

This creates a new \`Point\` with \`x\` and \`y\` set to new values and all other fields copied from \`original\`. Fields listed before \`..\` are overridden; everything after \`..\` is copied from the source.

**Your task:**

Implement \`fn move_point(p: Point, dx: int, dy: int) -> Point\` that returns a new Point with:
- \`x\` set to \`p.x + dx\`
- \`y\` set to \`p.y + dy\`
- \`z\` copied from \`p\` using the \`..p\` update syntax`,
      hints: [
        'Use `Point { x: p.x + dx, y: p.y + dy, ..p }`.',
        'Fields before `..` override; remaining fields are copied.',
        'The `..` must be the last field, with no trailing comma.',
      ],
      initialCode: `struct Point {
    x: int,
    y: int,
    z: int,
}

fn move_point(p: Point, dx: int, dy: int) -> Point {
    // TODO: return new Point with x += dx, y += dy, z copied from p
    Point { x: 0, y: 0, z: 0 }
}

fn main() {
    let p = Point { x: 10, y: 20, z: 30 };
    let moved = move_point(p, 5, 5);
    println!("moved: ({}, {}, {})", moved.x, moved.y, moved.z);
}
`,
      testCode: `#[test] fn test_move_point_basic() {
    let p = Point { x: 1, y: 2, z: 3 };
    let m = move_point(p, 10, 20);
    assert!(m.x == 11);
    assert!(m.y == 22);
    assert!(m.z == 3);
}

#[test] fn test_move_point_negative() {
    let p = Point { x: 10, y: 10, z: 99 };
    let m = move_point(p, -3, -5);
    assert!(m.x == 7);
    assert!(m.y == 5);
    assert!(m.z == 99);
}

#[test] fn test_move_point_zero() {
    let p = Point { x: 5, y: 5, z: 5 };
    let m = move_point(p, 0, 0);
    assert!(m.x == 5);
    assert!(m.y == 5);
    assert!(m.z == 5);
}
`,
    },
  ],
};
