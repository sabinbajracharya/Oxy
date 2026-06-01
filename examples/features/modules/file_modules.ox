// === Feature: Modules — File-Based Modules ===
// Tests `mod name;` loading from name.ox and name/mod.ox.

// === Standalone file module: mymath.ox ===
mod mymath;

use mymath::add;
use mymath::fib;

#[test]
fn test_file_module_function() {
    assert::eq(add(2, 3), 5);
}

#[test]
fn test_file_module_qualified_call() {
    assert::eq(mymath::sub(10, 3), 7);
}

#[test]
fn test_file_module_public_function() {
    assert::eq(fib(10), 55);
}

// === Subdirectory module: subpkg/mod.ox ===
mod subpkg;

use subpkg::helper;

#[test]
fn test_subdir_module() {
    assert::eq(helper::greet(), "hello from subpkg");
}

// === Struct from file module ===
use mymath::Point;

#[test]
fn test_struct_from_file_module() {
    val p = Point { x: 1.0, y: 2.0 };
    assert::eq(p.x, 1.0);
    assert::eq(p.y, 2.0);
}

// === Enum from file module ===
use mymath::Operation;

#[test]
fn test_enum_from_file_module() {
    val add_op = Operation::Add(3, 4);
    assert::eq(mymath::execute(add_op), 7);
    val mul_op = Operation::Mul(3, 4);
    assert::eq(mymath::execute(mul_op), 12);
}
