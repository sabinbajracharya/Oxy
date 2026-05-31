//! Traits, generics, where-clauses, type aliases, constants, derive.
//!
//! Shared helpers and imports come from the parent module (`use super::*`).

use super::*;

#[test]
fn test_trait_basic() {
    let out = run_and_capture(
        r#"
trait Greet {
    fn greet(self) -> String;
}

struct Person {
    name: String,
}

impl Greet for Person {
    fn greet(self) -> String {
        format!("Hello, I'm {}!", self.name)
    }
}

fn main() {
    let p = Person { name: String::from("Alice") };
    println!("{}", p.greet());
}
"#,
    );
    assert_eq!(out, vec!["Hello, I'm Alice!\n"]);
}

#[test]
fn test_trait_multiple_methods() {
    let out = run_and_capture(
        r#"
trait Shape {
    fn area(self) -> float;
    fn name(self) -> String;
}

struct Circle {
    radius: float,
}

impl Shape for Circle {
    fn area(self) -> float {
        3.14159 * self.radius * self.radius
    }

    fn name(self) -> String {
        String::from("Circle")
    }
}

fn main() {
    let c = Circle { radius: 5.0 };
    println!("{}: {}", c.name(), c.area());
}
"#,
    );
    assert_eq!(out, vec!["Circle: 78.53975\n"]);
}

#[test]
fn test_trait_default_method() {
    let out = run_and_capture(
        r#"
trait Describable {
    fn name(self) -> String;
    fn describe(self) -> String {
        format!("I am {}", self.name())
    }
}

struct Dog {
    breed: String,
}

impl Describable for Dog {
    fn name(self) -> String {
        self.breed.clone()
    }
}

fn main() {
    let d = Dog { breed: String::from("Labrador") };
    println!("{}", d.describe());
}
"#,
    );
    assert_eq!(out, vec!["I am Labrador\n"]);
}

#[test]
fn test_format_macro() {
    let out = run_and_capture(
        r#"
fn main() {
    let s = format!("Hello, {}!", "world");
    println!("{}", s);
    let n = 42;
    let msg = format!("The answer is {}", n);
    println!("{}", msg);
}
"#,
    );
    assert_eq!(out, vec!["Hello, world!\n", "The answer is 42\n"]);
}

#[test]
fn test_operator_overloading_add() {
    let out = run_and_capture(
        r#"
struct Vec2 {
    x: float,
    y: float,
}

impl Vec2 {
    fn new(x: float, y: float) -> Self {
        Vec2 { x, y }
    }
}

impl Add for Vec2 {
    fn add(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
}

fn main() {
    let a = Vec2::new(1.0, 2.0);
    let b = Vec2::new(3.0, 4.0);
    let c = a + b;
    println!("{} {}", c.x, c.y);
}
"#,
    );
    assert_eq!(out, vec!["4.0 6.0\n"]);
}

#[test]
fn test_operator_overloading_mul() {
    let out = run_and_capture(
        r#"
struct Vec2 {
    x: float,
    y: float,
}

impl Mul for Vec2 {
    fn mul(self, other: Vec2) -> Vec2 {
        Vec2 { x: self.x * other.x, y: self.y * other.y }
    }
}

fn main() {
    let a = Vec2 { x: 2.0, y: 3.0 };
    let b = Vec2 { x: 4.0, y: 5.0 };
    let c = a * b;
    println!("{} {}", c.x, c.y);
}
"#,
    );
    assert_eq!(out, vec!["8.0 15.0\n"]);
}

#[test]
fn test_generic_function() {
    let out = run_and_capture(
        r#"
fn identity<T>(x: T) -> T {
    x
}

fn main() {
    let a = identity(42);
    let b = identity("hello");
    println!("{} {}", a, b);
}
"#,
    );
    assert_eq!(out, vec!["42 hello\n"]);
}

#[test]
fn test_generic_function_with_bounds() {
    let out = run_and_capture(
        r#"
fn print_val<T: Display>(x: T) {
    println!("{}", x);
}

fn main() {
    print_val(42);
    print_val("hello");
}
"#,
    );
    assert_eq!(out, vec!["42\n", "hello\n"]);
}

#[test]
fn test_trait_with_impl_and_direct_methods() {
    let out = run_and_capture(
        r#"
trait Summary {
    fn summarize(self) -> String;
}

struct Article {
    title: String,
    content: String,
}

impl Article {
    fn new(title: String, content: String) -> Self {
        Article { title, content }
    }
}

impl Summary for Article {
    fn summarize(self) -> String {
        format!("{}: {}", self.title, self.content)
    }
}

fn main() {
    let a = Article::new(String::from("Oxy"), String::from("A Rust-like language"));
    println!("{}", a.summarize());
}
"#,
    );
    assert_eq!(out, vec!["Oxy: A Rust-like language\n"]);
}

#[test]
fn test_multiple_traits_for_type() {
    let out = run_and_capture(
        r#"
trait Greet {
    fn greet(self) -> String;
}

trait Farewell {
    fn farewell(self) -> String;
}

struct Person {
    name: String,
}

impl Greet for Person {
    fn greet(self) -> String {
        format!("Hi, I'm {}", self.name)
    }
}

impl Farewell for Person {
    fn farewell(self) -> String {
        format!("Goodbye from {}", self.name)
    }
}

fn main() {
    let p = Person { name: String::from("Bob") };
    println!("{}", p.greet());
    println!("{}", p.farewell());
}
"#,
    );
    assert_eq!(out, vec!["Hi, I'm Bob\n", "Goodbye from Bob\n"]);
}

#[test]
fn test_string_from() {
    let out = run_and_capture(
        r#"
fn main() {
    let s = String::from("hello");
    println!("{}", s);
}
"#,
    );
    assert_eq!(out, vec!["hello\n"]);
}

#[test]
fn test_trait_on_enum() {
    let out = run_and_capture(
        r#"
trait Describe {
    fn describe(self) -> String;
}

enum Color {
    Red,
    Green,
    Blue,
}

impl Describe for Color {
    fn describe(self) -> String {
        match self {
            Color::Red => String::from("red"),
            Color::Green => String::from("green"),
            Color::Blue => String::from("blue"),
        }
    }
}

fn main() {
    let c = Color::Green;
    println!("{}", c.describe());
}
"#,
    );
    assert_eq!(out, vec!["green\n"]);
}

#[test]
fn test_clone_method_on_string() {
    let out = run_and_capture(
        r#"
fn main() {
    let s = String::from("hello");
    let s2 = s.clone();
    println!("{} {}", s, s2);
}
"#,
    );
    assert_eq!(out, vec!["hello hello\n"]);
}

#[test]
fn test_type_alias() {
    let output = run_and_capture(
        r#"
type Meters = float;
fn main() {
    let d: Meters = 42.0;
    println!("{}", d);
}
"#,
    );
    assert_eq!(output, vec!["42.0\n"]);
}

#[test]
fn test_const() {
    let output = run_and_capture(
        r#"
const MAX: int = 100;
fn main() {
    println!("{}", MAX);
}
"#,
    );
    assert_eq!(output, vec!["100\n"]);
}

#[test]
fn test_const_float() {
    let output = run_and_capture(
        r#"
const PI: float = 3.14;
fn main() {
    println!("{}", PI);
}
"#,
    );
    assert_eq!(output, vec!["3.14\n"]);
}

#[test]
fn test_const_no_type_ann() {
    let output = run_and_capture(
        r#"
const GREETING = "hello";
fn main() {
    println!("{}", GREETING);
}
"#,
    );
    assert_eq!(output, vec!["hello\n"]);
}

#[test]
fn test_const_used_in_function() {
    let output = run_and_capture(
        r#"
const FACTOR: int = 10;
fn multiply(x: int) -> int {
    x * FACTOR
}
fn main() {
    println!("{}", multiply(5));
}
"#,
    );
    assert_eq!(output, vec!["50\n"]);
}

#[test]
fn test_derive_debug() {
    let out = run_and_capture(
        r#"
#[derive(Debug)]
struct Point { x: float, y: float }

fn main() {
    let p = Point { x: 1.0, y: 2.0 };
    println!("{:?}", p);
}
"#,
    );
    assert_eq!(out, vec!["Point { x: 1.0, y: 2.0 }\n"]);
}

#[test]
fn test_derive_clone() {
    let out = run_and_capture(
        r#"
#[derive(Clone)]
struct Point { x: float, y: float }

fn main() {
    let p = Point { x: 1.0, y: 2.0 };
    let p2 = p.clone();
    println!("{} {}", p2.x, p2.y);
}
"#,
    );
    assert_eq!(out, vec!["1.0 2.0\n"]);
}

#[test]
fn test_derive_partial_eq() {
    let out = run_and_capture(
        r#"
#[derive(PartialEq)]
struct Point { x: float, y: float }

fn main() {
    let a = Point { x: 1.0, y: 2.0 };
    let b = Point { x: 1.0, y: 2.0 };
    let c = Point { x: 3.0, y: 4.0 };
    println!("{}", a == b);
    println!("{}", a == c);
}
"#,
    );
    assert_eq!(out, vec!["true\n", "false\n"]);
}

#[test]
fn test_derive_multiple() {
    let out = run_and_capture(
        r#"
#[derive(Debug, Clone, PartialEq)]
struct Color { r: int, g: int, b: int }

fn main() {
    let c1 = Color { r: 255, g: 0, b: 0 };
    let c2 = c1.clone();
    println!("{:?}", c1);
    println!("{}", c1 == c2);
}
"#,
    );
    assert_eq!(out, vec!["Color { b: 0, g: 0, r: 255 }\n", "true\n"]);
}

#[test]
fn test_derive_default() {
    let out = run_and_capture(
        r#"
#[derive(Default, Debug)]
struct Config { width: int, height: int, title: String }

fn main() {
    let c = Config::default();
    println!("{:?}", c);
}
"#,
    );
    assert!(out[0].contains("width: 0"));
    assert!(out[0].contains("height: 0"));
    assert!(out[0].contains("title: \"\""));
}

#[test]
fn test_derive_enum_debug() {
    let out = run_and_capture(
        r#"
#[derive(Debug)]
enum Color { Red, Green, Blue }

fn main() {
    println!("{:?}", Color::Red);
}
"#,
    );
    assert_eq!(out, vec!["Color::Red\n"]);
}

#[test]
fn test_derive_enum_partial_eq() {
    let out = run_and_capture(
        r#"
#[derive(PartialEq)]
enum Direction { Up, Down, Left, Right }

fn main() {
    println!("{}", Direction::Up == Direction::Up);
    println!("{}", Direction::Up == Direction::Down);
}
"#,
    );
    assert_eq!(out, vec!["true\n", "false\n"]);
}

#[test]
fn test_no_derive_clone_error() {
    // In the VM, structs are always cloneable (Value implements Clone).
    // This test verifies the current behavior.
    let out = run_and_capture(
        r#"
struct Foo { x: int }

fn main() {
    let f = Foo { x: 1 };
    let f2 = f.clone();
    println!("{}", f2.x);
}
"#,
    );
    assert_eq!(out, vec!["1\n"]);
}

#[test]
fn test_attribute_ignored_unknown() {
    let out = run_and_capture(
        r#"
#[serde(rename_all)]
struct Foo { x: int }

fn main() {
    let f = Foo { x: 42 };
    println!("{}", f.x);
}
"#,
    );
    assert_eq!(out, vec!["42\n"]);
}

#[test]
fn test_derive_enum_clone() {
    let out = run_and_capture(
        r#"
#[derive(Clone, Debug)]
enum Shape { Circle(float), Square(float) }

fn main() {
    let s = Shape::Circle(5.0);
    let s2 = s.clone();
    println!("{:?}", s2);
}
"#,
    );
    assert_eq!(out, vec!["Shape::Circle(5.0)\n"]);
}

#[test]
fn test_type_alias_struct() {
    let output = run_and_capture(
        r#"
            struct Point { x: float, y: float }
            type Pos = Point;
            fn main() {
                let p = Pos { x: 1.0, y: 2.0 };
                println!("{} {}", p.x, p.y);
            }
            "#,
    );
    assert_eq!(output, vec!["1.0 2.0\n"]);
}

#[test]
fn test_type_alias_enum() {
    run_compiled_capturing(
        r#"
            enum Dir { Up, Down }
            type Direction = Dir;
            fn main() { let d = Direction::Up; }
            "#,
    )
    .unwrap();
}

#[test]
fn test_type_alias_associated_fn() {
    let output = run_and_capture(
        r#"
            struct Point { x: float, y: float }
            impl Point { fn origin() -> Point { Point { x: 0.0, y: 0.0 } } }
            type P = Point;
            fn main() {
                let p = P::origin();
                println!("{} {}", p.x, p.y);
            }
            "#,
    );
    assert_eq!(output, vec!["0.0 0.0\n"]);
}

#[test]
fn test_trait_bound_inline() {
    let output = run_and_capture(
        r#"
            trait Greet { fn greet(self) -> String; }
            struct Dog { name: String }
            impl Greet for Dog { fn greet(self) -> String { format!("Woof! I'm {}", self.name) } }
            fn say_hi<T: Greet>(item: T) {
                println!("{}", item.greet());
            }
            fn main() {
                say_hi(Dog { name: "Rex".to_string() });
            }
            "#,
    );
    assert_eq!(output, vec!["Woof! I'm Rex\n"]);
}
