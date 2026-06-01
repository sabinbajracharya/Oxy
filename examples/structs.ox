// Example: Structs, Enums, and Impl Blocks in Oxy

struct Point {
    x: Float,
    y: Float,
}

impl Point {
    fn new(x: Float, y: Float) -> Self {
        Point { x, y }
    }

    fn display(self) {
        io::println("Point({}, {})", self.x, self.y);
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

    fn describe(self) {
        match self {
            Shape::Circle(r) => io::println("Circle with radius {}", r),
            Shape::Rectangle(w, h) => io::println("Rectangle {}x{}", w, h),
        }
    }
}

fn main() {
    val p1 = Point::new(0.0, 0.0);
    val p2 = Point::new(3.0, 4.0);
    p1.display();
    p2.display();

    // Field access
    val dx = p1.x - p2.x;
    val dy = p1.y - p2.y;
    val dist_sq = dx * dx + dy * dy;
    io::println("Distance squared: {}", dist_sq);

    // Enum variants
    val circle = Shape::Circle(5.0);
    val rect = Shape::Rectangle(4.0, 3.0);

    circle.describe();
    io::println("Area: {}", circle.area());

    rect.describe();
    io::println("Area: {}", rect.area());

    // Debug format
    io::println("{:?}", circle);
    io::println("{:?}", rect);
}
