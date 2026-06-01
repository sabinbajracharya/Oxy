// Traits & Generics example for Oxy

trait Greet {
    fn greet(self) -> String;
}

trait Describable {
    fn name(self) -> String;
    // Default method — uses self.name()
    fn describe(self) -> String {
        string::format("I am {}", self.name())
    }
}

struct Person {
    name: String,
    age: Int,
}

impl Person {
    fn new(name: String, age: Int) -> Self {
        Person { name, age }
    }
}

impl Greet for Person {
    fn greet(self) -> String {
        string::format("Hello, I'm {} and I'm {} years old!", self.name, self.age)
    }
}

impl Describable for Person {
    fn name(self) -> String {
        self.name.clone()
    }
    // describe() uses the default implementation from the trait
}

// Generic function — works with any type
fn identity<T>(x: T) -> T {
    x
}

// Generic with bounds (bounds parsed but not enforced in this phase)
fn print_value<T: Display>(value: T) {
    io::println("{}", value);
}

// Operator overloading via traits
struct Vec2 {
    x: Float,
    y: Float,
}

impl Vec2 {
    fn new(x: Float, y: Float) -> Self {
        Vec2 { x, y }
    }
}

impl Add for Vec2 {
    fn add(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
}

impl Mul for Vec2 {
    fn mul(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x * other.x, self.y * other.y)
    }
}

fn main() {
    // Trait methods
    val p = Person::new(String::from("Alice"), 30);
    io::println("{}", p.greet());
    io::println("{}", p.describe());

    // format function
    val msg = string::format("{}! You are {} years old.", p.name, p.age);
    io::println("{}", msg);

    // Generics
    val x = identity(42);
    val s = identity("hello");
    io::println("identity: {} {}", x, s);

    // Generic with bounds
    print_value(3.14);
    print_value("world");

    // Operator overloading
    val a = Vec2::new(1.0, 2.0);
    val b = Vec2::new(3.0, 4.0);
    val c = a + b;
    io::println("add: ({}, {})", c.x, c.y);

    val d = Vec2::new(2.0, 3.0);
    val e = Vec2::new(4.0, 5.0);
    val f = d * e;
    io::println("mul: ({}, {})", f.x, f.y);

    // String::from
    val greeting = String::from("Hello, Oxy!");
    io::println("{}", greeting);
}
