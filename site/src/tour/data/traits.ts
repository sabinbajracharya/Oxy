import type { Chapter } from '../types';

export const traits: Chapter = {
  id: 'traits',
  title: 'Traits',
  lessons: [
    {
      id: 'trait-def',
      title: 'Defining Traits',
      instructions: `## Defining and Implementing Traits

A **trait** defines a set of method signatures that types can implement:

\`\`\`oxy
trait Speak {
    fn speak(self) -> String;
}
\`\`\`

Any type can implement a trait:

\`\`\`oxy
struct Dog { name: String }
impl Speak for Dog {
    fn speak(self) -> String {
        "Woof! I'm " + self.name
    }
}
\`\`\`

Once implemented, trait methods are called with dot syntax: \`dog.speak()\`.

**Your task:** Read the \`Speak\` trait and the \`Dog\` implementation below, then add an \`impl Speak for Cat\` block.`,
      hints: [
        'Use the same `impl TraitName for TypeName { ... }` syntax.',
        'The `Cat` struct already has a `name` field — use it in the return string.',
        'String concatenation works with `+`: `"hello " + self.name`.',
      ],
      initialCode: `trait Speak {
    fn speak(self) -> String;
}

struct Dog {
    name: String,
}

impl Speak for Dog {
    fn speak(self) -> String {
        "Woof! I'm " + self.name
    }
}

// TODO: implement Speak for Cat — make it say "Meow! I'm <name>"
struct Cat {
    name: String,
}

fn main() {
    let dog = Dog { name: "Rex".to_string() };
    let cat = Cat { name: "Whiskers".to_string() };
    println!("{}", dog.speak());
    println!("{}", cat.speak());
}
`,
      testCode: `#[test]
fn test_dog_speaks() {
    let d = Dog { name: "Rex".to_string() };
    assert_eq!(d.speak(), "Woof! I'm Rex");
}

#[test]
fn test_dog_different_name() {
    let d = Dog { name: "Buddy".to_string() };
    let s = d.speak();
    assert!(s.contains("Buddy"));
    assert!(s.contains("Woof"));
}

#[test]
fn test_cat_speaks() {
    let c = Cat { name: "Whiskers".to_string() };
    assert_eq!(c.speak(), "Meow! I'm Whiskers");
}

#[test]
fn test_cat_different_name() {
    let c = Cat { name: "Mittens".to_string() };
    let s = c.speak();
    assert!(s.contains("Mittens"));
    assert!(s.contains("Meow"));
}
`,
    },
    {
      id: 'trait-default',
      title: 'Default Methods',
      instructions: `## Default Method Implementations

Traits can provide **default implementations** that implementors inherit:

\`\`\`oxy
trait Greet {
    fn greet(self) -> String {
        "Hello!".to_string()  // default
    }
}

impl Greet for Person {}  // uses default — no override needed
\`\`\`

Implementors can override defaults by providing their own version. Default methods can also call other trait methods, including ones without defaults.

**Your task:** Add an \`impl Descriptor for Person\` that overrides \`describe\`. The \`label\` method already has a default that calls \`describe\`.`,
      hints: [
        'Override `describe` by writing `impl Descriptor for Person { fn describe(self) -> String { ... } }`.',
        'The `name` field is on `Person` — use `self.name` in the response.',
        'When you override `describe`, the default `label` method automatically uses your version.',
      ],
      initialCode: `trait Descriptor {
    fn describe(self) -> String {
        "an unknown thing".to_string()
    }
    fn label(self) -> String {
        "Item: ".to_string() + self.describe()
    }
}

struct Person {
    name: String,
}

// TODO: implement Descriptor for Person
// Override describe to return "a person named " + self.name

fn main() {
    let p = Person { name: "Alice".to_string() };
    println!("{}", p.describe());
    println!("{}", p.label());  // uses default, calls overridden describe
}
`,
      testCode: `#[test]
fn test_person_describe() {
    let p = Person { name: "Alice".to_string() };
    assert_eq!(p.describe(), "a person named Alice");
}

#[test]
fn test_person_describe_other_name() {
    let p = Person { name: "Bob".to_string() };
    assert_eq!(p.describe(), "a person named Bob");
}

#[test]
fn test_person_label() {
    let p = Person { name: "Charlie".to_string() };
    assert_eq!(p.label(), "Item: a person named Charlie");
}

#[test]
fn test_default_describe_for_empty_impl() {
    struct Widget;
    impl Descriptor for Widget {}
    let w = Widget;
    assert_eq!(w.describe(), "an unknown thing");
    assert_eq!(w.label(), "Item: an unknown thing");
}
`,
    },
    {
      id: 'trait-bounds',
      title: 'Trait Bounds',
      instructions: `## Trait Bounds on Generic Functions

A **trait bound** restricts a generic parameter to types that implement a specific trait:

\`\`\`oxy
fn say_twice<T: Speak>(x: T) -> String {
    x.speak() + " " + x.speak()
}
\`\`\`

The bound \`T: Speak\` means: "T can be any type that implements Speak." Inside the function, you can call \`x.speak()\` because the bound guarantees it exists.

Multiple bounds use \`+\`: \`T: Speak + Clone\`.

**Your task:** Complete \`say_twice\` so it calls \`x.speak()\` twice and joins the results with a space.`,
      hints: [
        'The bound `<T: Speak>` lets you call `x.speak()` inside the function.',
        'Concatenate: `x.speak() + " " + x.speak()`.',
        'You can implement Speak on any type, including int — see the test.',
      ],
      initialCode: `trait Speak {
    fn speak(self) -> String;
}

struct Dog {
    name: String,
}

impl Speak for Dog {
    fn speak(self) -> String {
        "Woof! I'm " + self.name
    }
}

impl Speak for int {
    fn speak(self) -> String {
        f"Number {self}"
    }
}

fn say_twice<T: Speak>(x: T) -> String {
    // TODO: call x.speak() twice, joining with a space
    ___  // example: x.speak() + " " + x.speak()
}

fn main() {
    let dog = Dog { name: "Rex".to_string() };
    println!("dog says: {}", say_twice(dog));
    println!("42 says: {}", say_twice(42));
}
`,
      testCode: `#[test]
fn test_say_twice_dog() {
    let d = Dog { name: "Rex".to_string() };
    let result = say_twice(d);
    assert_eq!(result, "Woof! I'm Rex Woof! I'm Rex");
}

#[test]
fn test_say_twice_dog_other_name() {
    let d = Dog { name: "Buddy".to_string() };
    let result = say_twice(d);
    assert_eq!(result, "Woof! I'm Buddy Woof! I'm Buddy");
}

#[test]
fn test_say_twice_int() {
    let result = say_twice(42);
    assert_eq!(result, "Number 42 Number 42");
}

#[test]
fn test_say_twice_int_zero() {
    let result = say_twice(0);
    assert_eq!(result, "Number 0 Number 0");
}

#[test]
fn test_say_twice_int_negative() {
    let result = say_twice(-5);
    assert_eq!(result, "Number -5 Number -5");
}
`,
    },
    {
      id: 'derive',
      title: 'Derive Macros',
      instructions: `## Derive Macros

The \`#[derive(...)]\` attribute auto-generates trait implementations:

\`\`\`oxy
#[derive(Default)]
struct Config {
    host: String,
    port: int,
}
\`\`\`

With \`#[derive(Default)]\`, you get \`Config::default()\` which fills every field with its zero value (empty string, 0, false, etc.).

Common derivable traits: \`Default\`, \`Debug\`, \`Clone\`.

**Your task:** Add \`#[derive(Default)]\` to the \`Settings\` struct, then call \`Settings::default()\` to create an instance.`,
      hints: [
        'Add `#[derive(Default)]` on the line right before `struct Settings {`.',
        'Call `Settings::default()` to create an instance with all zero values.',
        'A derived Default gives each field its zero value: "" for String, 0 for int.',
      ],
      initialCode: `// TODO: add #[derive(Default)] before this struct
struct Settings {
    host: String,
    port: int,
    debug: bool,
    timeout: int,
}

fn main() {
    // TODO: create a default Settings instance
    let cfg = ___;  // replace with Settings::default()
    println!("host: '{}'", cfg.host);
    println!("port: {}", cfg.port);
    println!("debug: {}", cfg.debug);
    println!("timeout: {}", cfg.timeout);
}
`,
      testCode: `#[test]
fn test_derive_default_basic() {
    let c = Settings::default();
    assert_eq!(c.host, "");
    assert_eq!(c.port, 0);
    assert_eq!(c.debug, false);
    assert_eq!(c.timeout, 0);
}

#[test]
fn test_derive_default_host_via_annotation() {
    let c = Settings::default();
    assert_eq!(c.host, "");
}

#[test]
fn test_default_is_zero_values() {
    let c = Settings::default();
    assert_eq!(c.port, 0);
    assert_eq!(c.debug, false);
    assert_eq!(c.timeout, 0);
}

#[test]
fn test_struct_still_works_with_explicit_init() {
    let c = Settings { host: "localhost".to_string(), port: 8080, debug: true, timeout: 30 };
    assert_eq!(c.host, "localhost");
    assert_eq!(c.port, 8080);
    assert!(c.debug);
    assert_eq!(c.timeout, 30);
}
`,
    },
    {
      id: 'operator-overloading',
      title: 'Operator Overloading',
      instructions: `## Overloading Operators with Traits

Implement standard library operator traits to use \`+\`, \`-\`, \`*\`, etc. with your custom types:

\`\`\`oxy
impl Add for Vec2 {
    fn add(self, other: Vec2) -> Vec2 {
        Vec2 { x: self.x + other.x, y: self.y + other.y }
    }
}

let result = a + b;  // calls Vec2::add(a, b)
\`\`\`

Common operator traits: \`Add\`, \`Sub\`, \`Mul\`, \`Div\`, \`Neg\`, \`Rem\`, \`PartialEq\`. They're defined in the prelude — no import needed.

**Your task:** Implement \`Add\` for the \`Point\` struct so that \`a + b\` adds corresponding coordinates.`,
      hints: [
        'The `Add` trait has method `fn add(self, other: Point) -> Point`.',
        'Add `self.x + other.x` and `self.y + other.y` for the result.',
        'Operator traits come from the standard prelude — they\'re always available.',
      ],
      initialCode: `struct Point {
    x: int,
    y: int,
}

// TODO: implement Add for Point so that a + b works
impl Add for Point {
    fn add(self, other: Point) -> Point {
        // TODO: return a Point with x = self.x + other.x, y = self.y + other.y
        Point { x: self.x + other.x, y: self.y + other.y }  // already correct!
    }
}

fn main() {
    let a = Point { x: 1, y: 2 };
    let b = Point { x: 3, y: 4 };
    let c = a + b;
    println!("({}, {})", c.x, c.y);
}
`,
      testCode: `#[test]
fn test_add_basic() {
    let a = Point { x: 1, y: 2 };
    let b = Point { x: 3, y: 4 };
    let c = a + b;
    assert_eq!(c.x, 4);
    assert_eq!(c.y, 6);
}

#[test]
fn test_add_with_negative() {
    let a = Point { x: 10, y: -5 };
    let b = Point { x: -3, y: 8 };
    let c = a + b;
    assert_eq!(c.x, 7);
    assert_eq!(c.y, 3);
}

#[test]
fn test_add_zero() {
    let a = Point { x: 5, y: 10 };
    let b = Point { x: 0, y: 0 };
    let c = a + b;
    assert_eq!(c.x, 5);
    assert_eq!(c.y, 10);
}

#[test]
fn test_add_chaining() {
    let a = Point { x: 1, y: 1 };
    let b = Point { x: 2, y: 2 };
    let c = Point { x: 3, y: 3 };
    let d = a + b + c;
    assert_eq!(d.x, 6);
    assert_eq!(d.y, 6);
}

#[test]
fn test_add_commutative() {
    let a = Point { x: 7, y: 8 };
    let b = Point { x: 3, y: 2 };
    let ab = a + b;
    let ba = b + a;  // Add is commutative
    assert_eq!(ab.x, ba.x);
    assert_eq!(ab.y, ba.y);
}
`,
    },
  ],
};
