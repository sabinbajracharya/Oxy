import type { Chapter } from '../types';

export const traitsGenerics: Chapter = {
  id: 'traits-generics',
  title: 'Traits & Generics',
  lessons: [
    {
      id: 'generics',
      title: 'Generic Types',
      instructions: `## Generic Types

Use angle brackets to define and use generic types: \`struct Box<T> { ... }\`.

Generics work on structs, enums, and functions. The compiler **monomorphizes** — it generates a concrete version for each set of type arguments used.

**Try it:** Create a \`Pair<f64, f64>\` and a \`Pair<String, bool>\`.`,
      hints: [
        'Generic params: `<T>`, `<K, V>`, `<T, U>`.',
        'The compiler checks that all type arguments are known at compile time.',
      ],
      initialCode: `struct Pair<T, U> {
    first: T,
    second: U,
}

fn swap<T, U>(p: Pair<T, U>) -> Pair<U, T> {
    Pair { first: p.second, second: p.first }
}

fn main() {
    let p = Pair { first: 42, second: "hello" };
    println!("{}, {}", p.first, p.second);

    let swapped = swap(p);
    println!("{}, {}", swapped.first, swapped.second);
}
`,
    },
    {
      id: 'trait-def',
      title: 'Trait Definitions',
      instructions: `## Defining Traits

\`trait\` defines a set of method signatures that types can implement. This enables polymorphism.

A trait can include **default method implementations** that implementors can override.

**Try it:** Implement \`Greet\` for a \`Cat\` struct that says "Meow!".`,
      hints: [
        'Trait methods list signatures separated by semicolons (no body).',
        'Default methods: write the body inside the trait.',
      ],
      initialCode: `trait Greet {
    fn greet(&self) -> String;
    fn loud_greet(&self) -> String {
        self.greet().to_uppercase()
    }
}

struct Dog {
    name: String,
}

impl Greet for Dog {
    fn greet(&self) -> String {
        f"Woof! I'm {}!".to_string()
    }
}

fn main() {
    let d = Dog { name: "Rex".to_string() };
    println!("{}", d.greet());
    println!("{}", d.loud_greet());
}
`,
    },
    {
      id: 'trait-bounds',
      title: 'Trait Bounds',
      instructions: `## Trait Bounds

Restrict generic parameters with trait bounds: \`fn foo<T: Trait>(x: T)\`.

Multiple bounds: \`T: Clone + Display\`. This limits what types can be used and what methods can be called on values of that type.

**Try it:** Add a call to \`item.describe()\` inside the function.`,
      hints: [
        'Bounds go after the generic param: `<T: Bound>`.',
        '`+` combines multiple bounds.',
      ],
      initialCode: `trait Display {
    fn display(&self) -> String;
}

impl Display for i64 {
    fn display(&self) -> String {
        f"int({})".to_string()
    }
}

impl Display for String {
    fn display(&self) -> String {
        f"str({})".to_string()
    }
}

fn print_twice<T: Display>(item: T) {
    println!("1: {}", item.display());
    println!("2: {}", item.display());
}

fn main() {
    print_twice(42);
    print_twice("hello".to_string());
}
`,
    },
    {
      id: 'operator-overloading',
      title: 'Operator Overloading',
      instructions: `## Operator Overloading

Implement traits from the standard library to overload operators for your types.

Common overloadable traits: \`Add\`, \`Sub\`, \`Mul\`, \`Div\`, \`Neg\`, \`Rem\`, \`PartialEq\`, \`PartialOrd\`.

These traits are defined in the prelude — no import needed.

**Try it:** Implement \`Mul\` (multiply) for \`Point\` to scale by a \`f64\`.`,
      hints: [
        'Trait impls must implement all required methods.',
        'Operator traits are regular traits — use `impl Trait for Type { ... }`.',
      ],
      initialCode: `struct Point {
    x: f64,
    y: f64,
}

impl Add for Point {
    fn add(self, other: Point) -> Point {
        Point { x: self.x + other.x, y: self.y + other.y }
    }
}

impl Neg for Point {
    fn neg(self) -> Point {
        Point { x: -self.x, y: -self.y }
    }
}

fn main() {
    let a = Point { x: 1.0, y: 2.0 };
    let b = Point { x: 3.0, y: 4.0 };
    let c = a + b;
    println!("({}, {})", c.x, c.y);

    let neg = -c;
    println!("({}, {})", neg.x, neg.y);
}
`,
    },
  ],
};
