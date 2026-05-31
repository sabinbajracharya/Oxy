//! Modules, use statements, visibility, field-visibility enforcement.
//!
//! Shared helpers and imports come from the parent module (`use super::*`).

use super::*;

#[test]
fn test_inline_module() {
    let output = run_and_capture(
        r#"
mod math {
    pub fn add(a: Int, b: Int) -> Int {
        a + b
    }
}
use math::add;
fn main() {
    println("{}", add(3, 4));
}"#,
    );
    assert_eq!(output, vec!["7\n"]);
}

#[test]
fn test_module_path_call() {
    let output = run_and_capture(
        r#"
mod math {
    pub fn multiply(a: Int, b: Int) -> Int {
        a * b
    }
}
fn main() {
    println("{}", math::multiply(3, 4));
}"#,
    );
    assert_eq!(output, vec!["12\n"]);
}

#[test]
fn test_use_glob_import() {
    let output = run_and_capture(
        r#"
mod utils {
    pub fn greet(name: String) -> String {
        format("Hello, {}!", name)
    }
    pub fn farewell(name: String) -> String {
        format("Goodbye, {}!", name)
    }
}
use utils::*;
fn main() {
    println("{}", greet("Alice"));
    println("{}", farewell("Bob"));
}"#,
    );
    assert_eq!(output, vec!["Hello, Alice!\n", "Goodbye, Bob!\n"]);
}

#[test]
fn test_use_group_import() {
    let output = run_and_capture(
        r#"
mod ops {
    pub fn add(a: Int, b: Int) -> Int { a + b }
    pub fn sub(a: Int, b: Int) -> Int { a - b }
    pub fn mul(a: Int, b: Int) -> Int { a * b }
}
use ops::{add, sub};
fn main() {
    println("{} {}", add(10, 3), sub(10, 3));
}"#,
    );
    assert_eq!(output, vec!["13 7\n"]);
}

#[test]
fn test_module_with_struct() {
    let output = run_and_capture(
        r#"
mod geometry {
    pub struct PoInt { x: Float, y: Float }
    impl PoInt {
        pub fn new(x: Float, y: Float) -> Self {
            PoInt { x, y }
        }
        pub fn to_string(self) -> String {
            format("({}, {})", self.x, self.y)
        }
    }
}
use geometry::PoInt;
fn main() {
    let p = PoInt::new(1.0, 2.0);
    println("{}", p.to_string());
}"#,
    );
    assert_eq!(output, vec!["(1.0, 2.0)\n"]);
}

#[test]
fn test_module_with_enum() {
    let output = run_and_capture(
        r#"
mod colors {
    pub enum Color { Red, Green, Blue }
}
use colors::Color;
fn main() {
    let c = Color::Red;
    match c {
        Color::Red => println("red"),
        Color::Green => println("green"),
        Color::Blue => println("blue"),
    }
}"#,
    );
    assert_eq!(output, vec!["red\n"]);
}

#[test]
fn test_pub_keyword_accepted() {
    let output = run_and_capture(
        r#"
pub mod math {
    pub fn add(a: Int, b: Int) -> Int { a + b }
}
use math::add;
fn main() {
    println("{}", add(1, 2));
}"#,
    );
    assert_eq!(output, vec!["3\n"]);
}

#[test]
fn test_pub_fn_accepted() {
    let output = run_and_capture(
        r#"
pub fn helper() -> Int { 42 }
fn main() {
    println("{}", helper());
}"#,
    );
    assert_eq!(output, vec!["42\n"]);
}

#[test]
fn test_multiple_modules() {
    let output = run_and_capture(
        r#"
mod a {
    pub fn foo() -> Int { 1 }
}
mod b {
    pub fn bar() -> Int { 2 }
}
use a::foo;
use b::bar;
fn main() {
    println("{}", foo() + bar());
}"#,
    );
    assert_eq!(output, vec!["3\n"]);
}

#[test]
fn test_use_inside_module() {
    let output = run_and_capture(
        r#"
mod outer {
    pub fn value() -> Int { 42 }
}
mod inner {
    use outer::value;
    pub fn call() -> Int { value() }
}
use inner::call;
fn main() {
    println("{}", call());
}"#,
    );
    assert_eq!(output, vec!["42\n"]);
}

#[test]
fn test_type_alias_inside_module() {
    let output = run_and_capture(
        r#"
mod types {
    pub type Num = Int;
    pub fn make() -> Num { 10 }
}
use types::make;
fn main() {
    println("{}", make());
}"#,
    );
    assert_eq!(output, vec!["10\n"]);
}

#[test]
fn test_visibility_filtering_glob() {
    let output = run_and_capture(
        r#"
mod lib {
    pub fn visible() -> String { "yes" }
    fn hidden() -> String { "no" }
}
use lib::*;
fn main() {
    println("{}", visible());
}"#,
    );
    assert_eq!(output, vec!["yes\n"]);
}

#[test]
fn test_glob_after_module_definition() {
    // Glob after module: still works (eager path)
    let output = run_and_capture(
        r#"
mod math {
    pub fn double(x: Int) -> Int { x * 2 }
}
use math::*;
fn main() {
    println("{}", double(21));
}"#,
    );
    assert_eq!(output, vec!["42\n"]);
}

#[test]
fn test_glob_before_module_definition() {
    // Glob BEFORE module: works via deferred resolution
    let output = run_and_capture(
        r#"
use math::*;
mod math {
    pub fn triple(x: Int) -> Int { x * 3 }
}
fn main() {
    println("{}", triple(7));
}"#,
    );
    assert_eq!(output, vec!["21\n"]);
}

#[test]
fn test_self_in_use_path() {
    // `self` in use paths resolves to the current module
    let output = run_and_capture(
        r#"
mod m {
    pub fn val() -> Int { 42 }
    pub use self::val;
}
use m::val;
fn main() {
    println("{}", val());
}"#,
    );
    assert_eq!(output, vec!["42\n"]);
}

#[test]
fn test_super_in_use_path() {
    // super resolves to parent module in nested modules
    let output = run_and_capture(
        r#"
mod a {
    pub fn val() -> Int { 99 }
    pub mod b {
        use super::val;
        pub fn call() -> Int { val() }
    }
}
use a::b::call;
fn main() {
    println("{}", call());
}"#,
    );
    assert_eq!(output, vec!["99\n"]);
}

#[test]
fn test_pub_use_re_export() {
    let output = run_and_capture(
        r#"
mod inner {
    pub fn msg() -> String { "hi".to_string() }
}
mod middle {
    pub use inner::msg;
}
use middle::msg;
fn main() {
    println("{}", msg());
}"#,
    );
    assert_eq!(output, vec!["hi\n"]);
}

#[test]
fn test_struct_init_with_use_import() {
    let output = run_and_capture(
        r#"
mod geom {
    pub struct PoInt { pub x: Float, pub y: Float }
}
use geom::PoInt;
fn main() {
    let p = PoInt { x: 1.5, y: 2.5 };
    println("({}, {})", p.x, p.y);
}"#,
    );
    assert_eq!(output, vec!["(1.5, 2.5)\n"]);
}

#[test]
fn test_use_as_rename_simple() {
    let output = run_and_capture(
        r#"
mod math {
    pub fn add(a: Int, b: Int) -> Int { a + b }
}
use math::add as sum;
fn main() {
    println("{}", sum(10, 20));
}"#,
    );
    assert_eq!(output, vec!["30\n"]);
}

#[test]
fn test_use_as_rename_group() {
    let output = run_and_capture(
        r#"
mod ops {
    pub fn add(a: Int, b: Int) -> Int { a + b }
    pub fn sub(a: Int, b: Int) -> Int { a - b }
}
use ops::{add as plus, sub as minus};
fn main() {
    println("{} {}", plus(5, 3), minus(5, 3));
}"#,
    );
    assert_eq!(output, vec!["8 2\n"]);
}

#[test]
fn test_pub_use_as_re_export() {
    let output = run_and_capture(
        r#"
mod inner {
    pub fn msg() -> String { "hello".to_string() }
}
mod middle {
    pub use inner::msg as greeting;
}
use middle::greeting;
fn main() {
    println("{}", greeting());
}"#,
    );
    assert_eq!(output, vec!["hello\n"]);
}

#[test]
fn test_sibling_module_path_call() {
    let output = run_and_capture(
        r#"
mod a {
    pub fn get_value() -> Int {
        b::helper()
    }
}
mod b {
    pub fn helper() -> Int { 42 }
}
use a::get_value;
fn main() {
    println("{}", get_value());
}"#,
    );
    assert_eq!(output, vec!["42\n"]);
}

#[test]
fn test_sibling_module_nested_path_call() {
    let output = run_and_capture(
        r#"
mod a {
    pub fn get_value() -> Int {
        b::c::deep()
    }
}
mod b {
    pub mod c {
        pub fn deep() -> Int { 77 }
    }
}
use a::get_value;
fn main() {
    println("{}", get_value());
}"#,
    );
    assert_eq!(output, vec!["77\n"]);
}

#[test]
fn test_self_qualified_path_call_in_module() {
    let output = run_and_capture(
        r#"
mod m {
    pub fn outer() -> Int {
        m::inner()
    }
    pub fn inner() -> Int { 11 }
}
use m::outer;
fn main() {
    println("{}", outer());
}"#,
    );
    assert_eq!(output, vec!["11\n"]);
}

#[test]
fn test_pub_use_glob_re_export() {
    let output = run_and_capture(
        r#"
mod inner {
    pub fn add(a: Int, b: Int) -> Int { a + b }
    pub fn sub(a: Int, b: Int) -> Int { a - b }
}
mod middle {
    pub use inner::*;
}
use middle::add;
use middle::sub;
fn main() {
    println("{} {}", add(10, 3), sub(10, 3));
}"#,
    );
    assert_eq!(output, vec!["13 7\n"]);
}

#[test]
fn test_pub_use_glob_re_export_single_import() {
    let output = run_and_capture(
        r#"
mod lib {
    pub fn version() -> Int { 1 }
    pub fn name() -> String { "oxy".to_string() }
}
mod prelude {
    pub use lib::*;
}
use prelude::*;
fn main() {
    println("{} {}", version(), name());
}"#,
    );
    assert_eq!(output, vec!["1 oxy\n"]);
}

#[test]
fn test_pub_visibility() {
    // pub works like pub — visible everywhere within the crate
    let output = run_and_capture(
        r#"
mod m {
    pub fn val() -> Int { 42 }
}
use m::val;
fn main() {
    println("{}", val());
}"#,
    );
    assert_eq!(output, vec!["42\n"]);
}

#[test]
fn test_pub_parent_visibility() {
    // pub is visible to the parent module
    let output = run_and_capture(
        r#"
mod a {
    pub fn val() -> Int { 99 }
    pub mod b {
        use super::val;
        pub fn call() -> Int { val() }
    }
}
use a::b::call;
fn main() {
    println("{}", call());
}"#,
    );
    assert_eq!(output, vec!["99\n"]);
}

#[test]
fn test_integer_type_annotation_accepts_unsuffixed_literal() {
    let output = run_and_capture(
        r#"
fn main() {
    let x: Int = 123123;
    println("{}", x);
}"#,
    );
    assert_eq!(output, vec!["123123\n"]);
}

#[test]
fn test_pub_fn() {
    run_compiled_capturing("pub fn greet() { println(\"hello\"); } fn main() { greet(); }")
        .unwrap();
}

#[test]
fn test_pub_struct() {
    run_compiled_capturing(
        "pub struct PoInt { pub x: Int, pub y: Int } fn main() { let p = PoInt { x: 1, y: 2 }; }",
    )
    .unwrap();
}

#[test]
fn test_pub_enum() {
    run_compiled_capturing("pub enum Color { Red, Blue } fn main() { let c = Color::Red; }")
        .unwrap();
}

#[test]
fn test_cannot_read_private_field_from_outside_module() {
    let result = run_compiled_capturing(
        r#"
mod database {
    pub struct Record {
        pub name: String,
        secret_key: Int,
    }
    pub fn make_record() -> Record {
        Record { name: "x".to_string(), secret_key: 42 }
    }
}
fn main() {
    let r = database::make_record();
    let k = r.secret_key;
    println("{}", k);
}"#,
    );
    assert!(result.is_err(), "should reject reading private field");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("private"),
        "error should mention 'private', got: {err}"
    );
}

#[test]
fn test_cannot_write_private_field_in_struct_init_from_outside() {
    let result = run_compiled_capturing(
        r#"
mod database {
    pub struct Record {
        pub name: String,
        secret_key: Int,
    }
}
fn main() {
    let r = database::Record { name: "x".to_string(), secret_key: 99 };
    println("{}", r.name);
}"#,
    );
    assert!(
        result.is_err(),
        "should reject struct init with private field from outside"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("private"),
        "error should mention 'private', got: {err}"
    );
}

#[test]
fn test_can_access_private_field_inside_same_module() {
    let output = run_and_capture(
        r#"
mod database {
    pub struct Record {
        pub name: String,
        secret_key: Int,
    }
    pub fn get_key(r: Record) -> Int {
        r.secret_key  // Allowed: inside the same module
    }
    pub fn make_record() -> Record {
        Record { name: "x".to_string(), secret_key: 42 }
    }
}
fn main() {
    let r = database::make_record();
    println("{}", database::get_key(r));
}"#,
    );
    assert_eq!(output, vec!["42\n"]);
}

#[test]
fn test_pub_fields_always_accessible() {
    let output = run_and_capture(
        r#"
mod shapes {
    pub struct PoInt {
        pub x: Float,
        pub y: Float,
    }
}
fn main() {
    let p = shapes::PoInt { x: 1.0, y: 2.0 };
    println("{}", p.x);
    println("{}", p.y);
}"#,
    );
    assert_eq!(output, vec!["1.0\n", "2.0\n"]);
}

// Regression tests for the slot/stack invariant class of bugs (historical
// context: docs/history/vm-locals-stack-separation.md, from the retired bytecode
// VM). The register IR + per-frame locals make these scenarios collision-free by
// construction, but the tests guard against architectural drift.

#[test]
fn test_for_loop_with_range_pattern() {
    // Range pattern inside a for-loop used to clobber the iterator slot
    // (Pattern::Range stored a scratch value in slot 0).
    let output = run_and_capture(
        r#"
fn main() {
    let mut hits = 0;
    for n in 0..20 {
        match n {
            3..=9 => { hits = hits + 1; },
            _ => {},
        }
    }
    println("{}", hits);
}"#,
    );
    assert_eq!(output, vec!["7\n"]);
}

#[test]
fn test_nested_match_in_closure() {
    // Enum match inside a closure body — EnumDataGet had to be wired up in
    // the closure dispatch path (formerly a separate execute_op).
    let output = run_and_capture(
        r#"
fn main() {
    let xs = [Some(1), None, Some(3), None, Some(5)];
    let doubled = xs.iter().map(|x| match x {
        Some(n) => n * 2,
        None => 0,
    }).collect::<List<Int>>();
    for v in doubled {
        println("{}", v);
    }
}"#,
    );
    assert_eq!(output, vec!["2\n", "0\n", "6\n", "0\n", "10\n"]);
}

#[test]
fn test_closure_mutating_captured_in_loop() {
    // Closure assigning to a captured `mut` var inside a for-loop —
    // a StoreLocal+continue bug previously corrupted the Cell-wrapped capture.
    let output = run_and_capture(
        r#"
fn main() {
    let mut total = 0;
    let add = |x: Int| { total = total + x; };
    for n in [1, 2, 3, 4, 5] {
        add(n);
    }
    println("{}", total);
}"#,
    );
    assert_eq!(output, vec!["15\n"]);
}

#[test]
fn test_deeply_nested_pattern_destructure() {
    // Tuple destructure then match on nested tuples — exercises temp-slot
    // allocation in bind_pattern_data; would surface any slot/stack collision.
    let output = run_and_capture(
        r#"
fn main() {
    let pairs = [(1, 2), (3, 4), (5, 6)];
    for (a, b) in pairs {
        match (a, b) {
            (1, y) => println("one {}", y),
            (x, 4) => println("{} four", x),
            (x, y) => println("{} {}", x, y),
        }
    }
}"#,
    );
    assert_eq!(output, vec!["one 2\n", "3 four\n", "5 6\n"]);
}

#[test]
fn test_recursive_call_inside_closure() {
    // Recursive Call inside a closure body run via run_closure — exercises
    // frame-stack discipline between the iterator builtin path and nested calls.
    let output = run_and_capture(
        r#"
fn fib(n: Int) -> Int {
    if n < 2 { return n; }
    fib(n - 1) + fib(n - 2)
}
fn main() {
    let results = [5, 6, 7].iter().map(|x| fib(x)).collect::<List<Int>>();
    for v in results {
        println("{}", v);
    }
}"#,
    );
    assert_eq!(output, vec!["5\n", "8\n", "13\n"]);
}
