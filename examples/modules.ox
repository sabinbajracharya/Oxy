// Example: Modules & Use Statements in Oxy

mod math {
    fn add(a: Int, b: Int) -> Int {
        a + b
    }

    fn multiply(a: Int, b: Int) -> Int {
        a * b
    }

    fn factorial(n: Int) -> Int {
        if n <= 1 {
            1
        } else {
            n * factorial(n - 1)
        }
    }
}

mod geometry {
    struct Point {
        x: Float,
        y: Float,
    }

    impl Point {
        fn new(x: Float, y: Float) -> Self {
            Point { x, y }
        }

        fn distance(self, other: Point) -> Float {
            val dx = self.x - other.x;
            val dy = self.y - other.y;
            (dx * dx + dy * dy).sqrt()
        }
    }

    enum Shape {
        Circle(Float),
        Rectangle(Float, Float),
    }

    impl Shape {
        fn area(self) -> Float {
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
    println("3 + 4 = {}", add(3, 4));
    println("3 * 4 = {}", multiply(3, 4));

    // Using module path directly
    println("5! = {}", math::factorial(5));

    // Using imported struct
    val p1 = Point::new(0.0, 0.0);
    val p2 = Point::new(3.0, 4.0);
    println("Distance: {}", p1.distance(p2));

    // Using imported enum
    val circle = Shape::Circle(5.0);
    val rect = Shape::Rectangle(3.0, 4.0);
    println("Circle area: {}", circle.area());
    println("Rectangle area: {}", rect.area());
}
