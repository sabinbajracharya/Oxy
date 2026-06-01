// === STRESS: traits — derive, default methods, generic impls, operator overloading ===

// --- basic trait with method ---
trait Greet {
    fn hello(self) -> String;
}

struct EnglishGuy { name: String }

impl Greet for EnglishGuy {
    fn hello(self) -> String {
        string::format("Hello, I'm {}", self.name)
    }
}

#[test]
fn test_basic_trait_impl() {
    val g = EnglishGuy { name: "Alice".to_string() };
    assert::eq(g.hello(), "Hello, I'm Alice");
}

// --- trait with default method ---
trait Named {
    fn name(self) -> String;
    fn greet(self) -> String {
        string::format("hi, {}", self.name())
    }
}

struct Cat { fluff: String }
impl Named for Cat {
    fn name(self) -> String { self.fluff.clone() }
}

#[test]
fn test_trait_default_method() {
    val c = Cat { fluff: "Mittens".to_string() };
    assert::eq(c.greet(), "hi, Mittens");
}

// --- trait with override of default ---
struct Dog { woof: String }
impl Named for Dog {
    fn name(self) -> String { self.woof.clone() }
    fn greet(self) -> String { string::format("BARK, {}!", self.name()) }
}

#[test]
fn test_trait_default_override() {
    val d = Dog { woof: "Rex".to_string() };
    assert::eq(d.greet(), "BARK, Rex!");
}

// --- two impls of same trait on different types ---
trait Area {
    fn area(self) -> Float;
}
struct Circ { r: Float }
struct Sq { s: Float }
impl Area for Circ {
    fn area(self) -> Float { 3.14 * self.r * self.r }
}
impl Area for Sq {
    fn area(self) -> Float { self.s * self.s }
}

#[test]
fn test_trait_dispatch_per_type() {
    val c = Circ { r: 2.0 };
    val s = Sq { s: 3.0 };
    assert::eq(c.area(), 12.56);
    assert::eq(s.area(), 9.0);
}

// --- generic struct + concrete impl ---
struct Pair<T> { a: T, b: T }

impl Pair<Int> {
    fn new(a: Int, b: Int) -> Pair<Int> { Pair { a, b } }
}

#[test]
fn test_generic_struct_method() {
    val p: Pair<Int> = Pair::<Int>::new(3, 4);
    assert::eq(p.a, 3);
    assert::eq(p.b, 4);
}

// --- generic impl<T> Pair<T> — impl-level type parameter ---
struct Cell<T> { v: T }

impl<T> Cell<T> {
    fn make(v: T) -> Cell<T> { Cell { v } }
}

#[test]
fn test_generic_impl_int() {
    val c = Cell::make(42);
    assert::eq(c.v, 42);
}

#[test]
fn test_generic_impl_string() {
    val c = Cell::make("hi".to_string());
    assert::eq(c.v, "hi");
}

// --- impl<T> with method that uses T as a param ---
struct Box2<T> { v: T }

impl<T> Box2<T> {
    fn get(self) -> T { self.v }
}

#[test]
fn test_generic_impl_method_uses_T_as_return() {
    val b = Box2 { v: 7 };
    assert::eq(b.get(), 7);
}

// --- impl<A, B> with two type params ---
struct TwoBox<A, B> { a: A, b: B }

impl<A, B> TwoBox<A, B> {
    fn swap(self) -> TwoBox<B, A> { TwoBox { a: self.b, b: self.a } }
}

#[test]
fn test_generic_impl_two_params() {
    val t = TwoBox { a: 1, b: "x".to_string() };
    val s = t.swap();
    assert::eq(s.a, "x");
    assert::eq(s.b, 1);
}

// --- derive(Debug) ---
#[derive(Debug)]
struct Pt { x: Int, y: Int }

#[test]
fn test_derive_debug() {
    val p = Pt { x: 1, y: 2 };
    val s = string::format("{:?}", p);
    assert::true(s.contains("1"));
    assert::true(s.contains("2"));
}

// --- derive(Clone) ---
#[derive(Clone, Debug, PartialEq)]
struct Box1 { v: Int }

#[test]
fn test_derive_clone() {
    val b = Box1 { v: 7 };
    val b2 = b.clone();
    assert::eq(b2.v, 7);
    assert::eq(b, b2);
}

// --- derive(PartialEq) on enum with data ---
#[derive(Debug, PartialEq)]
enum Op { Add, Sub, Mul }

#[test]
fn test_derive_partialeq_enum() {
    assert::eq(Op::Add, Op::Add);
    assert::true(Op::Add != Op::Sub);
}

// --- multiple traits on one type ---
trait Sound { fn noise(self) -> String; }
trait Move { fn step(self) -> Int; }

struct Walker { name: String, speed: Int }

impl Sound for Walker {
    fn noise(self) -> String { string::format("{} walks", self.name) }
}
impl Move for Walker {
    fn step(self) -> Int { self.speed }
}

#[test]
fn test_multiple_traits_on_type() {
    val w = Walker { name: "Bob".to_string(), speed: 5 };
    assert::eq(w.noise(), "Bob walks");
    val w2 = Walker { name: "Bob".to_string(), speed: 5 };
    assert::eq(w2.step(), 5);
}

// --- trait with multiple methods ---
trait Stack {
    fn pop_one(self) -> Option<Int>;
    fn peek(self) -> Option<Int>;
    fn is_empty(self) -> bool;
}

struct VecStack { data: List<Int> }

impl Stack for VecStack {
    fn pop_one(self) -> Option<Int> { self.data.pop() }
    fn peek(self) -> Option<Int> {
        if self.data.len() == 0 { None }
        else { Some(self.data[self.data.len() - 1]) }
    }
    fn is_empty(self) -> bool { self.data.len() == 0 }
}

#[test]
fn test_trait_multiple_methods() {
    val s = VecStack { data: [1, 2, 3] };
    assert::eq(s.is_empty(), false);
    assert::eq(s.peek(), Some(3));
}

// --- trait bound on generic fn ---
fn area_double<T: Area>(t: T) -> Float {
    t.area() * 2.0
}

#[test]
fn test_trait_bound_on_generic_fn() {
    val c = Circ { r: 1.0 };
    assert::eq(area_double(c), 6.28);
}
