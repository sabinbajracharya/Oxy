// Example: Structs, Enums, and Impl Blocks in Oxide

struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }

    fn display(&self) {
        println!("Point({}, {})", self.x, self.y);
    }
}

enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
}

impl Shape {
    fn area(&self) -> f64 {
        match self {
            Shape::Circle(r) => 3.14159 * r * r,
            Shape::Rectangle(w, h) => w * h,
        }
    }

    fn describe(&self) {
        match self {
            Shape::Circle(r) => println!("Circle with radius {}", r),
            Shape::Rectangle(w, h) => println!("Rectangle {}x{}", w, h),
        }
    }
}

fn main() {
    let p1 = Point::new(0.0, 0.0);
    let p2 = Point::new(3.0, 4.0);
    p1.display();
    p2.display();

    // Field access
    let dx = p1.x - p2.x;
    let dy = p1.y - p2.y;
    let dist_sq = dx * dx + dy * dy;
    println!("Distance squared: {}", dist_sq);

    // Enum variants
    let circle = Shape::Circle(5.0);
    let rect = Shape::Rectangle(4.0, 3.0);

    circle.describe();
    println!("Area: {}", circle.area());

    rect.describe();
    println!("Area: {}", rect.area());

    // Debug format
    println!("{:?}", circle);
    println!("{:?}", rect);
}
