//! Struct & enum definition, impls, field mutation, built-in nodes.
//!
//! Shared helpers and imports come from the parent module (`use super::*`).

use super::*;

#[test]
fn test_struct_basic() {
    let out = run_and_capture(
        r#"
struct PoInt {
    x: Float,
    y: Float,
}

fn main() {
    val p = PoInt { x: 1.0, y: 2.0 };
    println("{} {}", p.x, p.y);
}
"#,
    );
    assert_eq!(out, vec!["1.0 2.0\n"]);
}

#[test]
fn test_struct_field_assignment() {
    let out = run_and_capture(
        r#"
struct PoInt {
    x: Float,
    y: Float,
}

fn main() {
    var p = PoInt { x: 1.0, y: 2.0 };
    p.x = 10.0;
    println("{} {}", p.x, p.y);
}
"#,
    );
    assert_eq!(out, vec!["10.0 2.0\n"]);
}

#[test]
fn test_struct_with_impl() {
    let out = run_and_capture(
        r#"
struct PoInt {
    x: Float,
    y: Float,
}

impl PoInt {
    fn new(x: Float, y: Float) -> Self {
        PoInt { x, y }
    }

    fn display(self) {
        println("({}, {})", self.x, self.y);
    }
}

fn main() {
    val p = PoInt::new(3.0, 4.0);
    p.display();
}
"#,
    );
    assert_eq!(out, vec!["(3.0, 4.0)\n"]);
}

#[test]
fn test_struct_method_with_args() {
    let out = run_and_capture(
        r#"
struct Rect {
    w: Float,
    h: Float,
}

impl Rect {
    fn area(self) -> Float {
        self.w * self.h
    }
}

fn main() {
    val r = Rect { w: 5.0, h: 3.0 };
    println("{}", r.area());
}
"#,
    );
    assert_eq!(out, vec!["15.0\n"]);
}

#[test]
fn test_struct_debug_format() {
    let out = run_and_capture(
        r#"
struct PoInt {
    x: Float,
    y: Float,
}

fn main() {
    val p = PoInt { x: 1.0, y: 2.0 };
    println("{:?}", p);
}
"#,
    );
    assert_eq!(out, vec!["PoInt { x: 1.0, y: 2.0 }\n"]);
}

#[test]
fn test_enum_unit_variant() {
    let out = run_and_capture(
        r#"
enum Color {
    Red,
    Green,
    Blue,
}

fn main() {
    val c = Color::Red;
    println("{}", c);
}
"#,
    );
    assert_eq!(out, vec!["Color::Red\n"]);
}

#[test]
fn test_enum_tuple_variant() {
    let out = run_and_capture(
        r#"
enum Shape {
    Circle(Float),
    Rectangle(Float, Float),
}

fn main() {
    val s = Shape::Circle(5.0);
    println("{}", s);
}
"#,
    );
    assert_eq!(out, vec!["Shape::Circle(5.0)\n"]);
}

#[test]
fn test_enum_match() {
    let out = run_and_capture(
        r#"
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

fn main() {
    val s = Shape::Circle(5.0);
    println("{}", s.area());
    val r = Shape::Rectangle(4.0, 3.0);
    println("{}", r.area());
}
"#,
    );
    assert_eq!(out, vec!["78.53975\n", "12.0\n"]);
}

#[test]
fn test_enum_match_three_field_tuple_variant() {
    // Regression: 3+ positional fields in a tuple variant used to bind the
    // third (and beyond) to Unit because EnumVariantEqual bulk-pushed data
    // into stack positions that collided with the binding slots.
    let out = run_and_capture(
        r#"
enum E { Three(Int, Int, Int), Four(Int, Int, Int, Int) }

fn main() {
    val v = E::Three(10, 20, 30);
    match v {
        E::Three(a, b, c) => println("{} {} {}", a, b, c),
        _ => println("no"),
    }
    val w = E::Four(1, 2, 3, 4);
    match w {
        E::Four(a, b, c, d) => println("{} {} {} {}", a, b, c, d),
        _ => println("no"),
    }
}
"#,
    );
    assert_eq!(out, vec!["10 20 30\n", "1 2 3 4\n"]);
}

#[test]
fn test_enum_match_unit_variant() {
    let out = run_and_capture(
        r#"
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

fn describe(d: Direction) -> String {
    match d {
        Direction::Up => "going up",
        Direction::Down => "going down",
        _ => "sideways",
    }
}

fn main() {
    println("{}", describe(Direction::Up));
    println("{}", describe(Direction::Left));
}
"#,
    );
    assert_eq!(out, vec!["going up\n", "sideways\n"]);
}

#[test]
fn test_enum_debug_format() {
    let out = run_and_capture(
        r#"
enum Shape {
    Circle(Float),
    PoInt,
}

fn main() {
    val s = Shape::Circle(2.5);
    val p = Shape::PoInt;
    println("{:?}", s);
    println("{:?}", p);
}
"#,
    );
    assert_eq!(out, vec!["Shape::Circle(2.5)\n", "Shape::PoInt\n"]);
}

#[test]
fn test_point_distance() {
    let out = run_and_capture(
        r#"
struct PoInt {
    x: Float,
    y: Float,
}

impl PoInt {
    fn new(x: Float, y: Float) -> Self {
        PoInt { x, y }
    }
}

fn main() {
    val p1 = PoInt::new(0.0, 0.0);
    val p2 = PoInt::new(3.0, 4.0);
    val dx = p1.x - p2.x;
    val dy = p1.y - p2.y;
    val dist_sq = dx * dx + dy * dy;
    println("{}", dist_sq);
}
"#,
    );
    assert_eq!(out, vec!["25.0\n"]);
}

#[test]
fn test_struct_self_type_resolution() {
    let out = run_and_capture(
        r#"
struct Counter {
    count: Int,
}

impl Counter {
    fn new() -> Self {
        Self { count: 0 }
    }

    fn value(self) -> Int {
        self.count
    }
}

fn main() {
    val c = Counter::new();
    println("{}", c.value());
}
"#,
    );
    assert_eq!(out, vec!["0\n"]);
}

#[test]
fn test_struct_shorthand_init() {
    let out = run_and_capture(
        r#"
struct PoInt {
    x: Float,
    y: Float,
}

fn main() {
    val x = 1.0;
    val y = 2.0;
    val p = PoInt { x, y };
    println("{} {}", p.x, p.y);
}
"#,
    );
    assert_eq!(out, vec!["1.0 2.0\n"]);
}

#[test]
fn test_enum_impl_methods() {
    let output = run_and_capture(
        r#"
            enum Color { Red, Blue }
            impl Color {
                fn name(self) -> String {
                    match self {
                        Color::Red => "red".to_string(),
                        Color::Blue => "blue".to_string(),
                    }
                }
            }
            fn main() { println("{}", Color::Red.name()); }
            "#,
    );
    assert_eq!(output, vec!["red\n"]);
}

#[test]
fn test_struct_field_mutation_via_method() {
    let output = run_and_capture(
        r#"
            struct Counter {
                count: Int,
            }

            impl Counter {
                fn new() -> Self {
                    Counter { count: 0 }
                }

                fn inc(self) {
                    self.count = self.count + 1;
                }
            }

            fn main() {
                var c = Counter::new();
                c.inc();
                c.inc();
                println("{}", c.count);
            }
            "#,
    );
    assert_eq!(output, vec!["2\n"]);
}

#[test]
fn test_struct_field_mutation_via_self_push() {
    let output = run_and_capture(
        r#"
            struct Stack {
                items: List,
            }

            impl Stack {
                fn new() -> Self {
                    Stack { items: [] }
                }

                fn push(self, v: Int) {
                    self.items.push(v);
                }
            }

            fn main() {
                var s = Stack::new();
                s.push(10);
                s.push(20);
                println("{}", s.items.len());
                println("{}", s.items[0]);
            }
            "#,
    );
    assert_eq!(output, vec!["2\n", "10\n"]);
}

#[test]
fn test_listnode_new() {
    let output = run_and_capture(
        r#"
            fn main() {
                val n = ListNode::new(5);
                println("{}", n.value);
                println("{}", n.next.is_none());
            }
            "#,
    );
    assert_eq!(output, vec!["5\n", "true\n"]);
}

#[test]
fn test_treenode_new() {
    let output = run_and_capture(
        r#"
            fn main() {
                val t = TreeNode::new(10);
                println("{}", t.value);
                println("{}", t.left.is_none());
                println("{}", t.right.is_none());
            }
            "#,
    );
    assert_eq!(output, vec!["10\n", "true\n", "true\n"]);
}

#[test]
fn test_listnode_linking() {
    let output = run_and_capture(
        r#"
            fn main() {
                var head = ListNode::new(1);
                val second = ListNode::new(2);
                head.next = Some(second);
                println("{}", head.value);
                println("{}", head.next.unwrap().value);
            }
            "#,
    );
    assert_eq!(output, vec!["1\n", "2\n"]);
}

#[test]
fn test_treenode_linking() {
    let output = run_and_capture(
        r#"
            fn main() {
                var root = TreeNode::new(5);
                val left = TreeNode::new(3);
                val right = TreeNode::new(7);
                root.left = Some(left);
                root.right = Some(right);
                println("{}", root.value);
                println("{}", root.left.unwrap().value);
                println("{}", root.right.unwrap().value);
            }
            "#,
    );
    assert_eq!(output, vec!["5\n", "3\n", "7\n"]);
}
