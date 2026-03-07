// Traits & Generics example for Oxide

trait Greet {
    fn greet(&self) -> String;
}

trait Describable {
    fn name(&self) -> String;
    // Default method — uses self.name()
    fn describe(&self) -> String {
        format!("I am {}", self.name())
    }
}

struct Person {
    name: String,
    age: i64,
}

impl Person {
    fn new(name: String, age: i64) -> Self {
        Person { name, age }
    }
}

impl Greet for Person {
    fn greet(&self) -> String {
        format!("Hello, I'm {} and I'm {} years old!", self.name, self.age)
    }
}

impl Describable for Person {
    fn name(&self) -> String {
        self.name.clone()
    }
    // describe() uses the default implementation from the trait
}

// Generic function — works with any type
fn identity<T>(x: T) -> T {
    x
}

// Generic with bounds (bounds parsed but not enforced in this phase)
fn print_value<T: Display>(val: T) {
    println!("{}", val);
}

// Operator overloading via traits
struct Vec2 {
    x: f64,
    y: f64,
}

impl Vec2 {
    fn new(x: f64, y: f64) -> Self {
        Vec2 { x, y }
    }
}

impl Add for Vec2 {
    fn add(&self, other: &Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
}

impl Mul for Vec2 {
    fn mul(&self, other: &Vec2) -> Vec2 {
        Vec2::new(self.x * other.x, self.y * other.y)
    }
}

fn main() {
    // Trait methods
    let p = Person::new(String::from("Alice"), 30);
    println!("{}", p.greet());
    println!("{}", p.describe());

    // format! macro
    let msg = format!("{}! You are {} years old.", p.name, p.age);
    println!("{}", msg);

    // Generics
    let x = identity(42);
    let s = identity("hello");
    println!("identity: {} {}", x, s);

    // Generic with bounds
    print_value(3.14);
    print_value("world");

    // Operator overloading
    let a = Vec2::new(1.0, 2.0);
    let b = Vec2::new(3.0, 4.0);
    let c = a + b;
    println!("add: ({}, {})", c.x, c.y);

    let d = Vec2::new(2.0, 3.0);
    let e = Vec2::new(4.0, 5.0);
    let f = d * e;
    println!("mul: ({}, {})", f.x, f.y);

    // String::from
    let greeting = String::from("Hello, Oxide!");
    println!("{}", greeting);
}
