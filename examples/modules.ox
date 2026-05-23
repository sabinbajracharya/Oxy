// Example: Modules & Use Statements in Oxy

mod math {
    fn add(a: int, b: int) -> int {
        a + b
    }

    fn multiply(a: int, b: int) -> int {
        a * b
    }

    fn factorial(n: int) -> int {
        if n <= 1 {
            1
        } else {
            n * factorial(n - 1)
        }
    }
}

mod geometry {
    struct Point {
        x: float,
        y: float,
    }

    impl Point {
        fn new(x: float, y: float) -> Self {
            Point { x, y }
        }

        fn distance(self, other: Point) -> float {
            let dx = self.x - other.x;
            let dy = self.y - other.y;
            (dx * dx + dy * dy).sqrt()
        }
    }

    enum Shape {
        Circle(float),
        Rectangle(float, float),
    }

    impl Shape {
        fn area(self) -> float {
            match self {
                Shape::Circle(r) => 3.14159 * r * r,
                Shape::Rectangle(w, h) => w * h,
            }
        }
    }
}

// Import specific items
use math::{add, multiply};
// Import everything from geometry
use geometry::*;

fn main() {
    // Using imported functions
    println!("3 + 4 = {}", add(3, 4));
    println!("3 * 4 = {}", multiply(3, 4));

    // Using module path directly
    println!("5! = {}", math::factorial(5));

    // Using imported struct
    let p1 = Point::new(0.0, 0.0);
    let p2 = Point::new(3.0, 4.0);
    println!("Distance: {}", p1.distance(p2));

    // Using imported enum
    let circle = Shape::Circle(5.0);
    let rect = Shape::Rectangle(3.0, 4.0);
    println!("Circle area: {}", circle.area());
    println!("Rectangle area: {}", rect.area());
}
