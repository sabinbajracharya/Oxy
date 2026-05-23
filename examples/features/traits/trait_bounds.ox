// === Feature: Traits — Trait Bounds on Generics ===
// Generic functions with trait bounds restrict type parameters to types
// that implement specific traits. Where clauses provide alternative syntax.

// === Basic Trait Bound ===

trait AsText {
    fn as_text(self) -> String;
}

impl AsText for int {
    fn as_text(self) -> String {
        self.to_string()
    }
}

fn print_val<T: AsText>(x: T) -> String {
    x.as_text()
}

#[test]
fn test_basic_trait_bound() {
    let s = print_val(42);
    assert_eq!(s, "42");
}

// === Multiple Trait Bounds ===

trait Doublable {
    fn double(self) -> Self;
}

impl Doublable for int {
    fn double(self) -> int {
        self * 2
    }
}

fn double_and_print<T: AsText + Doublable>(x: T) -> String {
    x.double().as_text()
}

#[test]
fn test_multiple_bounds() {
    let s = double_and_print(10);
    assert_eq!(s, "20");
}

// === Where Clause ===

fn add_str<T>(x: T) -> String
where
    T: AsText,
{
    x.as_text()
}

#[test]
fn test_where_clause() {
    let s = add_str(99);
    assert_eq!(s, "99");
}

// === Where Clause with Multiple Bounds ===

fn transform<T>(x: T) -> String
where
    T: AsText + Doublable,
{
    x.double().as_text()
}

#[test]
fn test_where_clause_multi() {
    let s = transform(5);
    assert_eq!(s, "10");
}

// === Trait Bound on Struct Generic ===

struct Wrapper<T: AsText> {
    inner: T,
}

fn show_wrapper<T: AsText>(w: Wrapper<T>) -> String {
    w.inner.as_text()
}

#[test]
fn test_trait_bound_on_struct() {
    let w = Wrapper { inner: 7 };
    let s = show_wrapper(w);
    assert_eq!(s, "7");
}
