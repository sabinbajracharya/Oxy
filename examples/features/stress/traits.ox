// === STRESS: traits — derive, default methods, generic impls, operator overloading ===

// --- basic trait with method ---
trait Greet {
    fn hello(self) -> String;
}

struct EnglishGuy { name: String }

impl Greet for EnglishGuy {
    fn hello(self) -> String {
        format!("Hello, I'm {}", self.name)
    }
}

#[test]
fn test_basic_trait_impl() {
    let g = EnglishGuy { name: "Alice".to_string() };
    assert_eq!(g.hello(), "Hello, I'm Alice");
}

// --- trait with default method ---
trait Named {
    fn name(self) -> String;
    fn greet(self) -> String {
        format!("hi, {}", self.name())
    }
}

struct Cat { fluff: String }
impl Named for Cat {
    fn name(self) -> String { self.fluff.clone() }
}

#[test]
fn test_trait_default_method() {
    let c = Cat { fluff: "Mittens".to_string() };
    assert_eq!(c.greet(), "hi, Mittens");
}

// --- trait with override of default ---
struct Dog { woof: String }
impl Named for Dog {
    fn name(self) -> String { self.woof.clone() }
    fn greet(self) -> String { format!("BARK, {}!", self.name()) }
}

#[test]
fn test_trait_default_override() {
    let d = Dog { woof: "Rex".to_string() };
    assert_eq!(d.greet(), "BARK, Rex!");
}

// --- two impls of same trait on different types ---
trait Area {
    fn area(self) -> float;
}
struct Circ { r: float }
struct Sq { s: float }
impl Area for Circ {
    fn area(self) -> float { 3.14 * self.r * self.r }
}
impl Area for Sq {
    fn area(self) -> float { self.s * self.s }
}

#[test]
fn test_trait_dispatch_per_type() {
    let c = Circ { r: 2.0 };
    let s = Sq { s: 3.0 };
    assert_eq!(c.area(), 12.56);
    assert_eq!(s.area(), 9.0);
}

// --- generic struct + concrete impl ---
struct Pair<T> { a: T, b: T }

impl Pair<int> {
    fn new(a: int, b: int) -> Pair<int> { Pair { a, b } }
}

#[test]
fn test_generic_struct_method() {
    let p: Pair<int> = Pair::<int>::new(3, 4);
    assert_eq!(p.a, 3);
    assert_eq!(p.b, 4);
}

// --- generic impl<T> Pair<T> — impl-level type parameter ---
struct Cell<T> { v: T }

impl<T> Cell<T> {
    fn make(v: T) -> Cell<T> { Cell { v } }
}

#[test]
fn test_generic_impl_int() {
    let c = Cell::make(42);
    assert_eq!(c.v, 42);
}

#[test]
fn test_generic_impl_string() {
    let c = Cell::make("hi".to_string());
    assert_eq!(c.v, "hi");
}

// --- impl<T> with method that uses T as a param ---
struct Box2<T> { v: T }

impl<T> Box2<T> {
    fn get(self) -> T { self.v }
}

#[test]
fn test_generic_impl_method_uses_T_as_return() {
    let b = Box2 { v: 7 };
    assert_eq!(b.get(), 7);
}

// --- impl<A, B> with two type params ---
struct TwoBox<A, B> { a: A, b: B }

impl<A, B> TwoBox<A, B> {
    fn swap(self) -> TwoBox<B, A> { TwoBox { a: self.b, b: self.a } }
}

#[test]
fn test_generic_impl_two_params() {
    let t = TwoBox { a: 1, b: "x".to_string() };
    let s = t.swap();
    assert_eq!(s.a, "x");
    assert_eq!(s.b, 1);
}

// --- derive(Debug) ---
#[derive(Debug)]
struct Pt { x: int, y: int }

#[test]
fn test_derive_debug() {
    let p = Pt { x: 1, y: 2 };
    let s = format!("{:?}", p);
    assert!(s.contains("1"));
    assert!(s.contains("2"));
}

// --- derive(Clone) ---
#[derive(Clone, Debug, PartialEq)]
struct Box1 { v: int }

#[test]
fn test_derive_clone() {
    let b = Box1 { v: 7 };
    let b2 = b.clone();
    assert_eq!(b2.v, 7);
    assert_eq!(b, b2);
}

// --- derive(PartialEq) on enum with data ---
#[derive(Debug, PartialEq)]
enum Op { Add, Sub, Mul }

#[test]
fn test_derive_partialeq_enum() {
    assert_eq!(Op::Add, Op::Add);
    assert!(Op::Add != Op::Sub);
}

// --- multiple traits on one type ---
trait Sound { fn noise(self) -> String; }
trait Move { fn step(self) -> int; }

struct Walker { name: String, speed: int }

impl Sound for Walker {
    fn noise(self) -> String { format!("{} walks", self.name) }
}
impl Move for Walker {
    fn step(self) -> int { self.speed }
}

#[test]
fn test_multiple_traits_on_type() {
    let w = Walker { name: "Bob".to_string(), speed: 5 };
    assert_eq!(w.noise(), "Bob walks");
    let w2 = Walker { name: "Bob".to_string(), speed: 5 };
    assert_eq!(w2.step(), 5);
}

// --- trait with multiple methods ---
trait Stack {
    fn pop_one(self) -> Option<int>;
    fn peek(self) -> Option<int>;
    fn is_empty(self) -> bool;
}

struct VecStack { data: Vec<int> }

impl Stack for VecStack {
    fn pop_one(self) -> Option<int> { self.data.pop() }
    fn peek(self) -> Option<int> {
        if self.data.len() == 0 { None }
        else { Some(self.data[self.data.len() - 1]) }
    }
    fn is_empty(self) -> bool { self.data.len() == 0 }
}

#[test]
fn test_trait_multiple_methods() {
    let s = VecStack { data: vec![1, 2, 3] };
    assert_eq!(s.is_empty(), false);
    assert_eq!(s.peek(), Some(3));
}

// --- trait bound on generic fn ---
fn area_double<T: Area>(t: T) -> float {
    t.area() * 2.0
}

#[test]
fn test_trait_bound_on_generic_fn() {
    let c = Circ { r: 1.0 };
    assert_eq!(area_double(c), 6.28);
}
