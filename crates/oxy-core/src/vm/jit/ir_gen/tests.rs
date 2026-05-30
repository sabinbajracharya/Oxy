use super::*;

/// Helper: parse + type-check + generate IR, return the IrGen and program.
fn gen(source: &str) -> IrGen {
    let program = crate::parser::parse(source).expect("parse failed");
    crate::type_checker::TypeChecker::new()
        .check_program(&program)
        .expect("type-check failed");
    let mut ir = IrGen::new();
    ir.gen_program(&program);
    ir
}

/// Helper: find an IrFunction by name.
fn find_fn<'a>(ir: &'a IrGen, name: &str) -> &'a IrFunction {
    ir.functions
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("function not found: {name}"))
}

/// Helper: collect all IrOp variants in a function as strings (for simple matching).
fn op_names(f: &IrFunction) -> Vec<String> {
    f.blocks
        .iter()
        .flat_map(|b| b.ops.iter().map(|op| format!("{:?}", op)))
        .collect()
}

// ── Literals ───────────────────────────────────────────────────────

#[test]
fn test_literal_int() {
    let ir = gen("fn main() -> int { 42 }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty(), "should have at least one block");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::ConstInt(_, 42))),
        "should have ConstInt(42), got: {:?}",
        ops
    );
}

#[test]
fn test_literal_bool_true() {
    let ir = gen("fn main() -> bool { true }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::ConstBool(_, true))),
        "should have ConstBool(true), got: {:?}",
        ops
    );
}

#[test]
fn test_literal_bool_false() {
    let ir = gen("fn main() -> bool { false }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::ConstBool(_, false))),
        "should have ConstBool(false), got: {:?}",
        ops
    );
}

#[test]
fn test_literal_float() {
    let ir = gen("fn main() -> float { 3.14 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::ConstFloat(_, _))),
        "should have ConstFloat, got: {:?}",
        ops
    );
}

#[test]
fn test_literal_string() {
    let ir = gen("fn main() -> String { \"hello\" }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::ConstString(_, _))),
        "should have ConstString, got: {:?}",
        ops
    );
}

#[test]
fn test_literal_char() {
    let ir = gen("fn main() -> char { 'x' }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::ConstChar(_, 'x'))),
        "should have ConstChar('x'), got: {:?}",
        ops
    );
}

#[test]
fn test_literal_unit() {
    let ir = gen("fn main() { }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty());
    // Should have terminator Return or Halt
}

// ── Binary arithmetic ──────────────────────────────────────────────

#[test]
fn test_add_two_ints() {
    let ir = gen("fn main() -> int { 1 + 2 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::Add(_, _, _))),
        "should have Add, got: {:?}",
        ops
    );
}

#[test]
fn test_sub() {
    let ir = gen("fn main() -> int { 5 - 3 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(ops.iter().any(|op| matches!(op, IrOp::Sub(_, _, _))));
}

#[test]
fn test_mul() {
    let ir = gen("fn main() -> int { 2 * 3 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(ops.iter().any(|op| matches!(op, IrOp::Mul(_, _, _))));
}

#[test]
fn test_div() {
    let ir = gen("fn main() -> int { 6 / 2 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(ops.iter().any(|op| matches!(op, IrOp::Div(_, _, _))));
}

#[test]
fn test_rem() {
    let ir = gen("fn main() -> int { 7 % 3 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(ops.iter().any(|op| matches!(op, IrOp::Rem(_, _, _))));
}

// ── Comparisons ────────────────────────────────────────────────────

#[test]
fn test_eq() {
    let ir = gen("fn main() -> bool { 1 == 2 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(ops.iter().any(|op| matches!(op, IrOp::Eq(_, _, _))));
}

#[test]
fn test_neq() {
    let ir = gen("fn main() -> bool { 1 != 2 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(ops.iter().any(|op| matches!(op, IrOp::Neq(_, _, _))));
}

#[test]
fn test_lt() {
    let ir = gen("fn main() -> bool { 1 < 2 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(ops.iter().any(|op| matches!(op, IrOp::Lt(_, _, _))));
}

#[test]
fn test_gt() {
    let ir = gen("fn main() -> bool { 3 > 2 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(ops.iter().any(|op| matches!(op, IrOp::Gt(_, _, _))));
}

#[test]
fn test_le() {
    let ir = gen("fn main() -> bool { 1 <= 2 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(ops.iter().any(|op| matches!(op, IrOp::Le(_, _, _))));
}

#[test]
fn test_ge() {
    let ir = gen("fn main() -> bool { 2 >= 1 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(ops.iter().any(|op| matches!(op, IrOp::Ge(_, _, _))));
}

// ── Logical operators ──────────────────────────────────────────────

#[test]
fn test_and() {
    // `&&` lowers to short-circuiting control flow (a Branch on the left
    // operand merging via Phi), not an eager `And` op that would evaluate
    // the right operand unconditionally.
    let ir = gen("fn main() -> bool { true && false }");
    let f = find_fn(&ir, "main");
    let has_branch = f
        .blocks
        .iter()
        .any(|b| matches!(b.terminator, Terminator::Branch { .. }));
    let has_phi = f
        .blocks
        .iter()
        .flat_map(|b| &b.ops)
        .any(|op| matches!(op, IrOp::Phi(_, _, _)));
    let no_eager_and = !f
        .blocks
        .iter()
        .flat_map(|b| &b.ops)
        .any(|op| matches!(op, IrOp::And(_, _, _)));
    assert!(has_branch && has_phi && no_eager_and);
}

#[test]
fn test_or() {
    // `||` short-circuits the same way `&&` does.
    let ir = gen("fn main() -> bool { true || false }");
    let f = find_fn(&ir, "main");
    let has_branch = f
        .blocks
        .iter()
        .any(|b| matches!(b.terminator, Terminator::Branch { .. }));
    let has_phi = f
        .blocks
        .iter()
        .flat_map(|b| &b.ops)
        .any(|op| matches!(op, IrOp::Phi(_, _, _)));
    let no_eager_or = !f
        .blocks
        .iter()
        .flat_map(|b| &b.ops)
        .any(|op| matches!(op, IrOp::Or(_, _, _)));
    assert!(has_branch && has_phi && no_eager_or);
}

// ── Unary ──────────────────────────────────────────────────────────

#[test]
fn test_neg() {
    let ir = gen("fn main() -> int { -42 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(ops.iter().any(|op| matches!(op, IrOp::Neg(_, _))));
}

#[test]
fn test_not() {
    let ir = gen("fn main() -> bool { !true }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(ops.iter().any(|op| matches!(op, IrOp::Not(_, _))));
}

// ── Bitwise ────────────────────────────────────────────────────────

#[test]
fn test_bitand() {
    let ir = gen("fn main() -> int { 1 & 2 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(ops.iter().any(|op| matches!(op, IrOp::BitAnd(_, _, _))));
}

#[test]
fn test_bitor() {
    let ir = gen("fn main() -> int { 1 | 2 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(ops.iter().any(|op| matches!(op, IrOp::BitOr(_, _, _))));
}

#[test]
fn test_bitxor() {
    let ir = gen("fn main() -> int { 1 ^ 2 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(ops.iter().any(|op| matches!(op, IrOp::BitXor(_, _, _))));
}

#[test]
fn test_shl() {
    let ir = gen("fn main() -> int { 1 << 2 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(ops.iter().any(|op| matches!(op, IrOp::Shl(_, _, _))));
}

#[test]
fn test_shr() {
    let ir = gen("fn main() -> int { 4 >> 1 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(ops.iter().any(|op| matches!(op, IrOp::Shr(_, _, _))));
}

// ── Variables (let bindings) ───────────────────────────────────────

#[test]
fn test_let_binding() {
    let ir = gen("fn main() -> int { let x = 5; x }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::StoreLocal(_, _))),
        "should have StoreLocal for let binding, got: {:?}",
        ops
    );
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::LoadLocal(_, _))),
        "should have LoadLocal for reading x, got: {:?}",
        ops
    );
}

#[test]
fn test_let_mut_binding() {
    let ir = gen("fn main() -> int { let mut x = 5; x = 10; x }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::StoreLocal(_, _))),
        "should have StoreLocal ops"
    );
}

#[test]
fn test_multiple_lets() {
    let ir = gen("fn main() -> int { let a = 1; let b = 2; a + b }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter()
            .filter(|op| matches!(op, IrOp::StoreLocal(_, _)))
            .count()
            >= 2,
        "should have at least 2 StoreLocal ops"
    );
}

// ── Control flow (if/else) ─────────────────────────────────────────

#[test]
fn test_if_then() {
    let ir = gen("fn main() -> int { if true { 1 } else { 0 } }");
    let f = find_fn(&ir, "main");
    assert!(
        f.blocks.len() >= 3,
        "should have at least 3 blocks (entry, then, else), got {}",
        f.blocks.len()
    );
    // Should have Branch terminator
    let entry = &f.blocks[f.entry];
    assert!(
        matches!(entry.terminator, Terminator::Branch { .. }),
        "entry should have Branch terminator, got: {:?}",
        entry.terminator
    );
}

#[test]
fn test_if_no_else() {
    let ir = gen("fn main() -> int { if true { 1 }; 0 }");
    let f = find_fn(&ir, "main");
    assert!(f.blocks.len() >= 2, "should have at least 2 blocks");
}

#[test]
fn test_if_else_if() {
    let ir = gen("fn main() -> int { if true { 1 } else if false { 2 } else { 3 } }");
    let f = find_fn(&ir, "main");
    assert!(
        f.blocks.len() >= 4,
        "should have multiple blocks for else-if chain"
    );
}

#[test]
fn test_if_let() {
    let ir = gen("fn main() -> int { let x = Option::Some(42); if let Option::Some(v) = x { v } else { 0 } }");
    let f = find_fn(&ir, "main");
    assert!(f.blocks.len() >= 3, "if-let should have multiple blocks");
}

// ── Loops ──────────────────────────────────────────────────────────

#[test]
fn test_while_loop() {
    let ir = gen("fn main() -> int { let mut x = 0; while x < 5 { x = x + 1; } x }");
    let f = find_fn(&ir, "main");
    assert!(
        f.blocks.len() >= 2,
        "while should have at least 2 blocks, got {}",
        f.blocks.len()
    );
}

#[test]
fn test_loop_expression() {
    let ir = gen("fn main() -> int { let mut x = 0; loop { x = x + 1; if x > 5 { break; } } x }");
    let f = find_fn(&ir, "main");
    assert!(
        f.blocks.len() >= 2,
        "loop should have multiple blocks, got {}",
        f.blocks.len()
    );
}

#[test]
fn test_for_in() {
    let ir =
        gen("fn main() -> int { let mut sum = 0; for x in vec![1, 2, 3] { sum = sum + x; } sum }");
    let f = find_fn(&ir, "main");
    assert!(
        f.blocks.len() >= 2,
        "for-in should have multiple blocks, got {}",
        f.blocks.len()
    );
}

#[test]
fn test_break_value() {
    let ir = gen("fn main() -> int { let result = loop { break 42 }; result }");
    let f = find_fn(&ir, "main");
    assert!(f.blocks.len() >= 2);
}

#[test]
fn test_continue_in_loop() {
    let ir = gen(
        "fn main() -> int { let mut x = 0; while x < 10 { x = x + 1; if x == 2 { continue; } } x }",
    );
    let f = find_fn(&ir, "main");
    assert!(
        f.blocks.len() >= 2,
        "while with continue should have blocks"
    );
}

// ── Function calls ─────────────────────────────────────────────────

#[test]
fn test_fn_call_no_args() {
    let ir = gen("fn foo() -> int { 42 } fn main() -> int { foo() }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
        "should have a call op, got: {:?}",
        ops
    );
}

#[test]
fn test_fn_call_with_args() {
    let ir = gen("fn add(a: int, b: int) -> int { a + b } fn main() -> int { add(1, 2) }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty());
}

#[test]
fn test_method_call() {
    let ir = gen("fn main() -> int { let s = \"hello\"; s.len() }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
        "method call should be a CallBuiltin"
    );
}

#[test]
fn test_path_call() {
    let ir = gen("fn main() -> int { let m = HashMap::new(); 0 }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty());
}

// ── Return ─────────────────────────────────────────────────────────

#[test]
fn test_return_value() {
    let ir = gen("fn main() -> int { return 42; }");
    let f = find_fn(&ir, "main");
    let entry = &f.blocks[f.entry];
    assert!(
        matches!(entry.terminator, Terminator::Return(_)),
        "should have Return terminator, got: {:?}",
        entry.terminator
    );
}

#[test]
fn test_return_expr_tail() {
    let ir = gen("fn main() -> int { 42 }");
    let f = find_fn(&ir, "main");
    let last_block = &f.blocks.last().unwrap();
    assert!(
        matches!(last_block.terminator, Terminator::Return(_)),
        "tail expr should generate Return terminator"
    );
}

// ── Blocks and scoping ─────────────────────────────────────────────

#[test]
fn test_nested_block() {
    let ir = gen("fn main() -> int { let x = { let y = 1; y }; x }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty());
}

// ── Struct construction ────────────────────────────────────────────

#[test]
fn test_struct_init() {
    let ir = gen(
        "struct Point { x: int, y: int } fn main() -> int { let p = Point { x: 1, y: 2 }; p.x }",
    );
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
        "struct init should use CallBuiltin for oxy_struct_init"
    );
}

#[test]
fn test_struct_update() {
    let ir = gen("struct Point { x: int, y: int } fn main() -> int { let p = Point { x: 1, y: 2 }; let p2 = Point { x: 3, ..p }; p2.x }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty());
}

#[test]
fn test_field_access() {
    let ir = gen(
        "struct Point { x: int, y: int } fn main() -> int { let p = Point { x: 1, y: 2 }; p.x }",
    );
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
        "field access should use CallBuiltin for oxy_field_access"
    );
}

// ── Enum variants ──────────────────────────────────────────────────

#[test]
fn test_enum_variant_unit() {
    let ir = gen("enum Color { Red, Blue } fn main() -> int { let c = Color::Red; 0 }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty());
}

#[test]
fn test_enum_variant_with_data() {
    let ir =
        gen("enum MyOption { Some(int), None } fn main() -> int { let x = MyOption::Some(42); 0 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
        "enum variant should use CallBuiltin"
    );
}

// ── Pattern matching ───────────────────────────────────────────────

#[test]
fn test_match_expression() {
    let ir = gen("fn main() -> int { match 1 { 0 => 10, 1 => 20, _ => 30 } }");
    let f = find_fn(&ir, "main");
    assert!(
        f.blocks.len() >= 3,
        "match should have multiple blocks, got {}",
        f.blocks.len()
    );
}

#[test]
fn test_match_on_enum() {
    let ir = gen("enum MyOption { Some(int), None } fn main() -> int { let x = MyOption::Some(42); match x { MyOption::Some(v) => v, MyOption::None => 0 } }");
    let f = find_fn(&ir, "main");
    assert!(
        f.blocks.len() >= 3,
        "match on enum should have multiple blocks"
    );
}

// ── Collections ────────────────────────────────────────────────────

#[test]
fn test_vec_literal() {
    let ir = gen("fn main() -> int { let v = vec![1, 2, 3]; 0 }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty());
}

#[test]
fn test_array_literal() {
    let ir = gen("fn main() -> int { let a = [1, 2, 3]; 0 }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty());
}

#[test]
fn test_tuple_literal() {
    let ir = gen("fn main() -> int { let t = (1, \"hello\", true); 0 }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty());
}

#[test]
fn test_index_expr() {
    let ir = gen("fn main() -> int { let v = vec![1, 2, 3]; v[0] }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
        "index should use CallBuiltin for oxy_vec_index"
    );
}

// ── F-string ───────────────────────────────────────────────────────

#[test]
fn test_fstring() {
    let ir = gen("fn main() -> String { let name = \"world\"; f\"Hello {name}!\" }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty());
}

// ── Try operator ───────────────────────────────────────────────────

#[test]
fn test_try_operator() {
    let ir = gen("fn main() -> Option { let x = Option::Some(42); let y = x?; Option::Some(y) }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
        "try should use CallBuiltin for oxy_try_pop"
    );
}

// ── Closures ───────────────────────────────────────────────────────

#[test]
fn test_closure_simple() {
    let ir = gen("fn main() -> int { let add = |a: int, b: int| -> int { a + b }; add(1, 2) }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty());
    // Should have a separate function for the closure
    let closure = ir.functions.iter().find(|f| f.name.contains("closure"));
    assert!(closure.is_some(), "should have a closure function");
}

#[test]
fn test_closure_capture() {
    let ir = gen("fn main() -> int { let x = 10; let f = || -> int { x }; f() }");
    let _f = find_fn(&ir, "main");
    let closure = ir.functions.iter().find(|f| f.name.contains("closure"));
    assert!(closure.is_some(), "should have a closure function");
    if let Some(c) = closure {
        assert!(!c.captures.is_empty(), "closure should capture x");
    }
}

// ── Async ──────────────────────────────────────────────────────────

#[test]
fn test_async_block() {
    let ir = gen("fn main() -> int { let fut = async { 42 }; 0 }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty());
}

// ── Generics ───────────────────────────────────────────────────────

#[test]
fn test_generic_fn() {
    let ir = gen("fn identity(x: int) -> int { x } fn main() -> int { identity(42) }");
    let f = find_fn(&ir, "identity");
    assert!(!f.blocks.is_empty());
}

// ── Multiple functions ─────────────────────────────────────────────

#[test]
fn test_multiple_functions() {
    let ir = gen("fn a() -> int { 1 } fn b() -> int { 2 } fn main() -> int { a() + b() }");
    assert!(!find_fn(&ir, "a").blocks.is_empty());
    assert!(!find_fn(&ir, "b").blocks.is_empty());
    assert!(!find_fn(&ir, "main").blocks.is_empty());
}

// ── Assignment ─────────────────────────────────────────────────────

#[test]
fn test_assign() {
    let ir = gen("fn main() -> int { let mut x = 5; x = 10; x }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    let store_count = ops
        .iter()
        .filter(|op| matches!(op, IrOp::StoreLocal(_, _)))
        .count();
    assert!(
        store_count >= 2,
        "should have at least 2 stores (init + assignment), got {}",
        store_count
    );
}

#[test]
fn test_compound_assign() {
    let ir = gen("fn main() -> int { let mut x = 5; x += 3; x }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty());
}

// ── Self reference ─────────────────────────────────────────────────

#[test]
fn test_method_with_self() {
    let ir = gen("struct Counter { value: int } impl Counter { fn inc(mut self) { self.value = self.value + 1 } } fn main() -> int { 0 }");
    // Should generate IR for the inc method
    let method = ir.functions.iter().find(|f| f.name.contains("inc"));
    assert!(
        method.is_some(),
        "should have inc method, functions: {:?}",
        ir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
}

// ── Edge cases ─────────────────────────────────────────────────────

#[test]
fn test_empty_function() {
    let ir = gen("fn main() { }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty());
}

#[test]
fn test_deeply_nested() {
    let ir =
        gen("fn main() -> int { let x = if true { if false { 1 } else { 2 } } else { 3 }; x }");
    let f = find_fn(&ir, "main");
    assert!(f.blocks.len() >= 4, "nested if should have multiple blocks");
}

#[test]
fn test_complex_expression() {
    let ir = gen("fn main() -> int { let a = 1; let b = 2; let c = 3; (a + b) * c }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty());
}

// ── Compile error tests ────────────────────────────────────────────

#[test]
fn test_unreachable_code_does_not_crash() {
    // Code after return should be handled gracefully
    let ir = gen("fn main() -> int { return 42; let x = 1; x }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty());
}

// ── Gaps from audit: MacroCall, Grouped, Repeat, AsyncBlock, Await ──

#[test]
fn test_grouped_expression() {
    let ir = gen("fn main() -> int { (42) }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::ConstInt(_, 42))),
        "grouped should unwrap inner literal, got: {:?}",
        ops
    );
}

#[test]
fn test_macro_call_println() {
    let ir = gen("fn main() { println!(\"hello\") }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
        "println should emit CallBuiltin"
    );
}

#[test]
fn test_repeat_expression() {
    let ir = gen("fn main() -> int { let a = [0; 5]; 0 }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
        "repeat should emit CallBuiltin"
    );
}

#[test]
fn test_async_block_expr() {
    let ir = gen("fn main() -> int { let fut = async { 42 }; 0 }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty());
}

#[test]
fn test_await_expr() {
    let ir = gen("fn main() -> int { let fut = async { 42 }; fut.await }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
        "await should emit CallBuiltin"
    );
}

// ── Gaps from audit: WhileLet, ForDestructure, LetPattern ──────────

#[test]
fn test_while_let() {
    let ir = gen(
        "fn main() -> int { let x = Option::Some(1); while let Option::Some(v) = x { break; } 0 }",
    );
    let f = find_fn(&ir, "main");
    assert!(
        f.blocks.len() >= 3,
        "while-let should have multiple blocks, got {}",
        f.blocks.len()
    );
}

#[test]
fn test_for_destructure() {
    let ir = gen("fn main() -> int { for (a, b) in vec![(1, 2), (3, 4)] { let _x = a + b; } 0 }");
    let f = find_fn(&ir, "main");
    assert!(
        f.blocks.len() >= 3,
        "for-destructure should have multiple blocks, got {}",
        f.blocks.len()
    );
}

#[test]
fn test_let_pattern() {
    let ir = gen("fn main() -> int { let (x, y) = (1, 2); x + y }");
    let f = find_fn(&ir, "main");
    let ops = &f.blocks[f.entry].ops;
    assert!(
        ops.iter()
            .filter(|op| matches!(op, IrOp::StoreLocal(_, _)))
            .count()
            >= 2,
        "let-pattern should bind both vars"
    );
}

// ── Gaps from audit: nested closures, labeled break ────────────────

#[test]
fn test_closure_inside_match() {
    let ir = gen("fn main() -> int { let x = 10; let f = match 1 { 1 => || -> int { x }, _ => || -> int { 0 } }; f() }");
    let _f = find_fn(&ir, "main");
    let closures: Vec<_> = ir
        .functions
        .iter()
        .filter(|f| f.name.contains("closure"))
        .collect();
    assert!(!closures.is_empty(), "should have closure inside match");
}

#[test]
fn test_cast_to_float() {
    let ir = gen("fn main() -> float { let x: float = 3; x }");
    let f = find_fn(&ir, "main");
    assert!(!f.blocks.is_empty());
}

#[test]
fn test_method_with_self_param() {
    let ir = gen("struct Counter { value: int } impl Counter { fn inc(mut self) { self.value = self.value + 1 } } fn main() -> int { 0 }");
    let method = ir.functions.iter().find(|f| f.name.contains("inc"));
    assert!(method.is_some(), "should have inc method");
}
