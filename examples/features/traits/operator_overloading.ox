// === Feature: Traits — Operator Overloading ===
// Implement operator traits (Add, Sub, Mul, Div, Rem, Neg) on custom types
// to enable +, -, *, /, %, and unary - operators.

// === Add Operator ===

struct Vec2 {
    x: f64,
    y: f64,
}

impl Add for Vec2 {
    fn add(self, other: Vec2) -> Vec2 {
        Vec2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

#[test]
fn test_add_operator() {
    let a = Vec2 { x: 1.5, y: 2.5 };
    let b = Vec2 { x: 3.0, y: 1.0 };
    let c = a + b;
    assert_eq!(c.x, 4.5);
    assert_eq!(c.y, 3.5);
}

// === Sub Operator ===

impl Sub for Vec2 {
    fn sub(self, other: Vec2) -> Vec2 {
        Vec2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

#[test]
fn test_sub_operator() {
    let a = Vec2 { x: 5.0, y: 3.0 };
    let b = Vec2 { x: 2.0, y: 1.0 };
    let c = a - b;
    assert_eq!(c.x, 3.0);
    assert_eq!(c.y, 2.0);
}

// === Mul Operator ===

impl Mul for Vec2 {
    fn mul(self, other: Vec2) -> Vec2 {
        Vec2 {
            x: self.x * other.x,
            y: self.y * other.y,
        }
    }
}

#[test]
fn test_mul_operator() {
    let a = Vec2 { x: 2.0, y: 3.0 };
    let b = Vec2 { x: 4.0, y: 5.0 };
    let c = a * b;
    assert_eq!(c.x, 8.0);
    assert_eq!(c.y, 15.0);
}

// === Neg Operator (unary -) ===

impl Neg for Vec2 {
    fn neg(self) -> Vec2 {
        Vec2 {
            x: -self.x,
            y: -self.y,
        }
    }
}

#[test]
fn test_neg_operator() {
    let v = Vec2 { x: 3.0, y: -4.0 };
    let n = -v;
    assert_eq!(n.x, -3.0);
    assert_eq!(n.y, 4.0);
}

// === Rem (modulo) Operator ===

struct WrappedInt(i64);

impl Rem for WrappedInt {
    fn rem(self, other: WrappedInt) -> WrappedInt {
        WrappedInt(self.0 % other.0)
    }
}

#[test]
fn test_rem_operator() {
    let a = WrappedInt(17);
    let b = WrappedInt(5);
    let c = a % b;
    assert_eq!(c.0, 2);
}

// === Div Operator ===

impl Div for WrappedInt {
    fn div(self, other: WrappedInt) -> WrappedInt {
        WrappedInt(self.0 / other.0)
    }
}

#[test]
fn test_div_operator() {
    let a = WrappedInt(20);
    let b = WrappedInt(4);
    let c = a / b;
    assert_eq!(c.0, 5);
}

// === Operator on Enum ===

struct BoxedInt {
    val: i64,
}

impl Add for BoxedInt {
    fn add(self, other: BoxedInt) -> BoxedInt {
        BoxedInt { val: self.val + other.val }
    }
}

#[test]
fn test_operator_on_enum() {
    let a = BoxedInt { val: 10 };
    let b = BoxedInt { val: 20 };
    let c = a + b;
    assert_eq!(c.val, 30);
}

// === Method Overrides Operator ===
// When both operator overloading and a direct method exist, the operator
// still dispatches through the trait impl.

#[test]
fn test_operator_chaining() {
    let a = Vec2 { x: 2.0, y: 1.0 };
    let b = Vec2 { x: 3.0, y: 4.0 };
    let c = Vec2 { x: 1.0, y: 1.0 };
    let result = a + b - c;
    assert_eq!(result.x, 4.0);
    assert_eq!(result.y, 4.0);
}
