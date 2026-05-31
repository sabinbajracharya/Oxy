// === STRESS: operator overloading + Self traits + comparisons ===

// --- Add for custom struct ---
#[derive(Clone, Debug, PartialEq)]
struct V2 { x: Int, y: Int }

impl Add for V2 {
    fn add(self, other: V2) -> V2 {
        V2 { x: self.x + other.x, y: self.y + other.y }
    }
}

#[test]
fn test_add_for_struct() {
    val a = V2 { x: 1, y: 2 };
    val b = V2 { x: 10, y: 20 };
    val c = a + b;
    assert_eq(c.x, 11);
    assert_eq(c.y, 22);
}

// --- Sub for custom struct ---
impl Sub for V2 {
    fn sub(self, other: V2) -> V2 {
        V2 { x: self.x - other.x, y: self.y - other.y }
    }
}

#[test]
fn test_sub_for_struct() {
    val a = V2 { x: 10, y: 20 };
    val b = V2 { x: 1, y: 2 };
    val c = a - b;
    assert_eq(c.x, 9);
    assert_eq(c.y, 18);
}

// --- Mul for custom struct (scalar) ---
struct Scaled { s: Int }
impl Mul for V2 {
    fn mul(self, other: V2) -> V2 {
        V2 { x: self.x * other.x, y: self.y * other.y }
    }
}

#[test]
fn test_mul_for_struct() {
    val a = V2 { x: 2, y: 3 };
    val b = V2 { x: 4, y: 5 };
    val c = a * b;
    assert_eq(c.x, 8);
    assert_eq(c.y, 15);
}

// --- PartialEq from derive ---
#[test]
fn test_partial_eq_derived() {
    val a = V2 { x: 1, y: 2 };
    val b = V2 { x: 1, y: 2 };
    assert_eq(a, b);
}

#[test]
fn test_partial_eq_inequality() {
    val a = V2 { x: 1, y: 2 };
    val b = V2 { x: 1, y: 3 };
    assert(a != b);
}

// --- Display via to_string ---
#[derive(Debug)]
struct Named { name: String }

impl Named {
    fn fmt(self) -> String {
        format("Named({})", self.name)
    }
}

#[test]
fn test_custom_display() {
    val n = Named { name: "test".to_string() };
    assert_eq(n.fmt(), "Named(test)");
}

// --- Chained operator with custom struct ---
#[test]
fn test_chained_add() {
    val a = V2 { x: 1, y: 1 };
    val b = V2 { x: 2, y: 2 };
    val c = V2 { x: 3, y: 3 };
    val r = a + b + c;
    assert_eq(r.x, 6);
    assert_eq(r.y, 6);
}

// --- Self in impl ---
struct Counter1 { val: Int }

impl Counter1 {
    fn new() -> Self { Counter1 { val: 0 } }
    fn bumped(self) -> Self { Counter1 { val: self.val + 1 } }
}

#[test]
fn test_self_in_impl() {
    val c = Counter1::new();
    val c2 = c.bumped();
    val c3 = c2.bumped();
    assert_eq(c3.val, 2);
}

// --- self vs mut self ---
struct Bag { items: List<Int> }

impl Bag {
    fn new() -> Bag { Bag { items: [] } }
    fn add(self, x: Int) -> Bag {
        self.items.push(x);
        self
    }
    fn count(self) -> Int { self.items.len() }
}

#[test]
fn test_mut_self_in_method() {
    val b = Bag::new();
    val b = b.add(1);
    val b = b.add(2);
    val b = b.add(3);
    assert_eq(b.count(), 3);
}

// --- Static method (no self) ---
struct Util;

impl Util {
    fn double(n: Int) -> Int { n * 2 }
    fn triple(n: Int) -> Int { n * 3 }
}

#[test]
fn test_static_methods() {
    assert_eq(Util::double(5), 10);
    assert_eq(Util::triple(5), 15);
}

// --- Builder pattern ---
struct Config { name: String, retries: Int }

impl Config {
    fn new() -> Config { Config { name: "".to_string(), retries: 0 } }
    fn name(self, n: String) -> Config { self.name = n; self }
    fn retries(self, r: Int) -> Config { self.retries = r; self }
}

#[test]
fn test_builder_pattern() {
    val c = Config::new().name("foo".to_string()).retries(3);
    assert_eq(c.name, "foo");
    assert_eq(c.retries, 3);
}

// --- ordering — comparisons on i64 ---
#[test]
fn test_int_ordering_in_data() {
    var v = [3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5];
    v.sort();
    assert_eq(v[0], 1);
    assert_eq(v[v.len() - 1], 9);
}

// --- ordering on String ---
#[test]
fn test_string_ordering() {
    var v = ["banana".to_string(), "apple".to_string(), "cherry".to_string()];
    v.sort();
    assert_eq(v[0], "apple");
    assert_eq(v[1], "banana");
    assert_eq(v[2], "cherry");
}
