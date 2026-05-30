//! Portable register-IR interpreter.
//!
//! Executes the exact same `IrFunction`s that the Cranelift backend compiles,
//! by walking each block's `IrOp`s and delegating runtime semantics to the
//! shared `oxy_*` FFI functions (the same ones the JIT calls). This is the
//! execution backend on `wasm32`, where Cranelift is unavailable, and the
//! reference engine the `jit_interp_parity` test diffs the JIT against.
//!
//! # Divergence guard
//!
//! This module is compiled on **all** targets (not just wasm) precisely so the
//! exhaustive `match` over `IrOp` / `Terminator` below is type-checked by every
//! native build. Adding or removing an IR op makes this file fail to compile
//! until the interpreter is updated — divergence becomes a build error, not a
//! silent wasm-only breakage. Features reached through `CallBuiltin` (the common
//! case) ride the shared FFI automatically; the `symbol_consistency` test guards
//! that string-keyed surface.

// FIXME: remove once the interpreter is wired into the wasm execution path
// (api.rs target dispatch). Until then it is exercised only by tests on native.
#![allow(dead_code)]

use std::collections::HashMap;

use crate::types::Value;

use super::jit::ffi::{self, FfiRet};
use super::jit::ir::{BlockId, IrFunction, IrOp, Reg, Terminator};
use super::jit::ir_gen::IrGen;
use super::jit::{ClosureRuntimeMeta, JitContext, JitTables};
use super::VmResult;

/// A program lowered to register IR, ready for interpretation. The interpreter
/// analogue of `JitEngine` — holds the IR functions plus the metadata tables an
/// executing context reads, but no native code.
pub(crate) struct InterpEngine {
    functions: Vec<IrFunction>,
    name_to_index: HashMap<String, usize>,
    tables: JitTables,
    /// FFI function name → (raw pointer, ABI return kind).
    ffi_table: HashMap<&'static str, (*const u8, FfiRet)>,
}

impl InterpEngine {
    /// Lower a type-checked program to register IR and build the engine.
    pub(crate) fn compile(program: &crate::ast::Program) -> Result<Self, String> {
        ffi::reset_runtime_state();

        let mut ir = IrGen::new();
        ir.gen_program(program);
        let functions: Vec<IrFunction> = std::mem::take(&mut ir.functions);
        let closure_meta: Vec<ClosureRuntimeMeta> = ir
            .closure_meta
            .drain(..)
            .map(|(param_names, captured, is_async)| ClosureRuntimeMeta {
                param_names,
                captured,
                is_async,
            })
            .collect();

        let mut name_to_index = HashMap::new();
        let mut fn_local_counts = HashMap::new();
        for f in &functions {
            name_to_index.insert(f.name.clone(), f.fn_index);
            fn_local_counts.insert(f.fn_index, f.local_count);
        }

        let tables = JitTables {
            // No native function pointers exist on the interpreter path; user
            // calls are resolved by name and interpreted recursively instead.
            fn_table: HashMap::new(),
            fn_local_counts,
            name_to_index: name_to_index.clone(),
            closure_meta,
        };

        let ffi_table = ffi::ffi_symbols()
            .into_iter()
            .map(|(name, ptr, ret)| (name, (ptr, ret)))
            .collect();

        Ok(Self {
            functions,
            name_to_index,
            tables,
            ffi_table,
        })
    }

    fn function(&self, name: &str) -> Option<&IrFunction> {
        self.name_to_index.get(name).map(|&i| &self.functions[i])
    }
}

/// Outcome discriminant returned by interpreting one function body, mirroring
/// the JIT's native-return convention: 0 = normal completion, 2 = error / `?`
/// propagation (message — possibly empty — sits in `ctx.error_*`).
type Disc = u64;

/// An executing interpreter bound to one engine. Holds the per-program tables
/// and an optional captured-output buffer (for tests / the wasm playground).
pub(crate) struct Interpreter<'e> {
    engine: &'e InterpEngine,
    output: Option<std::rc::Rc<std::cell::RefCell<Vec<String>>>>,
}

impl<'e> Interpreter<'e> {
    pub(crate) fn new(engine: &'e InterpEngine) -> Self {
        Self {
            engine,
            output: None,
        }
    }

    pub(crate) fn with_captured_output(&mut self) {
        self.output = Some(std::rc::Rc::new(std::cell::RefCell::new(Vec::new())));
    }

    pub(crate) fn captured_output(&self) -> Vec<String> {
        self.output
            .as_ref()
            .map(|rc| rc.borrow().clone())
            .unwrap_or_default()
    }

    /// Run `main`.
    pub(crate) fn run(&mut self) -> VmResult {
        self.run_function("main")
    }

    /// Run a named function with no arguments (entry point / `#[test]`).
    pub(crate) fn run_function(&mut self, name: &str) -> VmResult {
        let func = match self.engine.function(name) {
            Some(f) => f,
            None => return VmResult::Error(format!("function not found: {name}")),
        };
        let mut ctx = self.fresh_ctx(func.local_count);
        let disc = self.interpret(&mut ctx, func);
        self.finish(ctx, disc)
    }

    /// Build a context wired to this program's tables and capture buffer.
    fn fresh_ctx(&self, local_count: usize) -> JitContext {
        let mut ctx = JitContext::new(local_count);
        ctx.result = Value::Unit;
        ctx.tables = &self.engine.tables as *const JitTables;
        if let Some(ref output_rc) = self.output {
            ctx.output = output_rc as *const _;
        }
        ctx
    }

    /// Translate a final discriminant + context into a `VmResult` (mirrors the
    /// JIT's `JitVm::call_fn` tail).
    fn finish(&self, ctx: JitContext, disc: Disc) -> VmResult {
        match disc {
            0 => VmResult::Value(ctx.result.clone()),
            2 => {
                if ctx.error_len == 1 && ctx.error_msg[0] == 0 {
                    // `?` propagation with an empty marker: the propagated value
                    // lives in ctx.result.
                    VmResult::Value(ctx.result.clone())
                } else {
                    let msg = String::from_utf8_lossy(&ctx.error_msg[..ctx.error_len.min(1024)])
                        .into_owned();
                    VmResult::Error(msg)
                }
            }
            other => VmResult::Error(format!("unexpected discriminant {other}")),
        }
    }

    // ── Core interpretation loop ───────────────────────────────────────────

    /// Interpret one function body in `ctx`, returning its discriminant.
    /// `ctx`'s locals 0..n must already hold the arguments.
    fn interpret(&mut self, ctx: &mut JitContext, func: &IrFunction) -> Disc {
        let reg_def_block = reg_def_blocks(func);
        let mut regs: HashMap<Reg, Value> = HashMap::new();
        let mut block_id: BlockId = func.entry;
        let mut prev_block: Option<BlockId> = None;

        loop {
            let block = &func.blocks[block_id];
            for op in &block.ops {
                self.exec_op(ctx, op, &mut regs, prev_block, &reg_def_block);
            }
            match &block.terminator {
                Terminator::Return(r) => {
                    if let Some(v) = regs.get(r) {
                        ctx.result = v.clone();
                    }
                    return discriminant(ctx);
                }
                Terminator::Jump(target) => {
                    prev_block = Some(block_id);
                    block_id = *target;
                }
                Terminator::Branch {
                    cond,
                    then_block,
                    else_block,
                } => {
                    let truthy = self.truthy(ctx, regs.get(cond));
                    prev_block = Some(block_id);
                    block_id = if truthy { *then_block } else { *else_block };
                }
                Terminator::Halt => return discriminant(ctx),
                Terminator::Panic(msg_reg) => {
                    let v = Self::reg_val(&regs, *msg_reg);
                    self.call_named(ctx, "oxy_panic", &[v], &[], &[]);
                    return 2;
                }
            }
        }
    }

    /// Execute a single non-terminator op, updating the register file.
    fn exec_op(
        &mut self,
        ctx: &mut JitContext,
        op: &IrOp,
        regs: &mut HashMap<Reg, Value>,
        prev_block: Option<BlockId>,
        reg_def_block: &HashMap<Reg, BlockId>,
    ) {
        match op {
            // ── Constants: materialized directly, no FFI round-trip ──────────
            IrOp::ConstInt(r, n) => {
                regs.insert(*r, Value::I64(*n));
            }
            IrOp::ConstFloat(r, n) => {
                regs.insert(*r, Value::F64(*n));
            }
            IrOp::ConstBool(r, b) => {
                regs.insert(*r, Value::Bool(*b));
            }
            IrOp::ConstChar(r, c) => {
                regs.insert(*r, Value::Char(*c));
            }
            IrOp::ConstUnit(r) => {
                regs.insert(*r, Value::Unit);
            }
            IrOp::ConstString(r, s) => {
                regs.insert(*r, Value::String(s.clone()));
            }

            // ── Locals: routed through the shared FFI for Cell semantics ─────
            IrOp::LoadLocal(r, slot) => {
                let v = self.call_collect(ctx, "oxy_load_local", &[], &[], &[*slot]);
                regs.insert(*r, v);
            }
            IrOp::LoadLocalRaw(r, slot) => {
                let v = self.call_collect(ctx, "oxy_load_local_raw", &[], &[], &[*slot]);
                regs.insert(*r, v);
            }
            IrOp::StoreLocal(slot, src) => {
                let v = Self::reg_val(regs, *src);
                self.call_named(ctx, "oxy_store_local", &[v], &[], &[*slot]);
            }
            IrOp::MakeCell(slot) => {
                self.call_named(ctx, "oxy_make_cell", &[], &[], &[*slot]);
            }

            // ── Arithmetic / comparison / bitwise: shared FFI semantics ──────
            IrOp::Add(r, a, b) => self.binary(ctx, regs, "oxy_add", *r, *a, *b),
            IrOp::Sub(r, a, b) => self.binary(ctx, regs, "oxy_sub", *r, *a, *b),
            IrOp::Mul(r, a, b) => self.binary(ctx, regs, "oxy_mul", *r, *a, *b),
            IrOp::Div(r, a, b) => self.binary(ctx, regs, "oxy_div", *r, *a, *b),
            IrOp::Rem(r, a, b) => self.binary(ctx, regs, "oxy_mod", *r, *a, *b),
            IrOp::Eq(r, a, b) => self.binary(ctx, regs, "oxy_eq", *r, *a, *b),
            IrOp::Neq(r, a, b) => self.binary(ctx, regs, "oxy_neq", *r, *a, *b),
            IrOp::Lt(r, a, b) => self.binary(ctx, regs, "oxy_lt", *r, *a, *b),
            IrOp::Gt(r, a, b) => self.binary(ctx, regs, "oxy_gt", *r, *a, *b),
            IrOp::Le(r, a, b) => self.binary(ctx, regs, "oxy_le", *r, *a, *b),
            IrOp::Ge(r, a, b) => self.binary(ctx, regs, "oxy_ge", *r, *a, *b),
            IrOp::And(r, a, b) => self.binary(ctx, regs, "oxy_and", *r, *a, *b),
            IrOp::Or(r, a, b) => self.binary(ctx, regs, "oxy_or", *r, *a, *b),
            IrOp::BitAnd(r, a, b) => self.binary(ctx, regs, "oxy_bitand", *r, *a, *b),
            IrOp::BitOr(r, a, b) => self.binary(ctx, regs, "oxy_bitor", *r, *a, *b),
            IrOp::BitXor(r, a, b) => self.binary(ctx, regs, "oxy_bitxor", *r, *a, *b),
            IrOp::Shl(r, a, b) => self.binary(ctx, regs, "oxy_shl", *r, *a, *b),
            IrOp::Shr(r, a, b) => self.binary(ctx, regs, "oxy_shr", *r, *a, *b),
            IrOp::Neg(r, a) => self.unary(ctx, regs, "oxy_neg", *r, *a),
            IrOp::Not(r, a) => self.unary(ctx, regs, "oxy_not", *r, *a),
            IrOp::BitNot(r, a) => self.unary(ctx, regs, "oxy_bitnot", *r, *a),

            // ── Register moves ───────────────────────────────────────────────
            IrOp::Copy(r, a) => {
                if let Some(v) = regs.get(a).cloned() {
                    regs.insert(*r, v);
                }
            }
            IrOp::Phi(r, a, b) => {
                // SSA phi: only the source defined in the predecessor we arrived
                // from has been computed. Pick it; fall back to whichever is set.
                let pick = phi_source(*a, *b, prev_block, reg_def_block, regs);
                if let Some(v) = pick {
                    regs.insert(*r, v);
                }
            }

            // ── Result / error plumbing ──────────────────────────────────────
            IrOp::ReadResult(r) => {
                // Mirror codegen: oxy_return moves the operand-stack top (if any)
                // into ctx.result, then we read it back into the register.
                self.call_named(ctx, "oxy_return", &[], &[], &[]);
                regs.insert(*r, ctx.result.clone());
            }
            IrOp::WriteResult(src) => {
                let v = Self::reg_val(regs, *src);
                self.call_named(ctx, "oxy_return", &[v], &[], &[]);
            }
            IrOp::SetError(src) => {
                let v = Self::reg_val(regs, *src);
                self.call_named(ctx, "oxy_panic", &[v], &[], &[]);
            }
            IrOp::CheckError(r) => {
                regs.insert(*r, Value::I64(discriminant(ctx) as i64));
            }

            // ── Dynamic builtin / user-call dispatch ─────────────────────────
            IrOp::CallBuiltin {
                result,
                func,
                args,
                immediates,
                strings,
            } => {
                let v = self.call_builtin(ctx, func, args, regs, immediates, strings);
                regs.insert(*result, v);
            }
        }
    }

    // ── Operand-stack helpers (shared push/pop preserve move safety) ─────────

    fn push(&self, ctx: &mut JitContext, v: Value) {
        unsafe { ffi::push(ctx, v) }
    }

    fn pop(&self, ctx: &mut JitContext) -> Value {
        unsafe { ffi::pop(ctx) }
    }

    fn reg_val(regs: &HashMap<Reg, Value>, reg: Reg) -> Value {
        regs.get(&reg).cloned().unwrap_or(Value::Unit)
    }

    // ── FFI dispatch ─────────────────────────────────────────────────────────

    /// Push lhs, push rhs, call the binary op, store the popped result.
    fn binary(
        &mut self,
        ctx: &mut JitContext,
        regs: &mut HashMap<Reg, Value>,
        name: &'static str,
        dst: Reg,
        lhs: Reg,
        rhs: Reg,
    ) {
        let args = [Self::reg_val(regs, lhs), Self::reg_val(regs, rhs)];
        let v = self.call_collect(ctx, name, &args, &[], &[]);
        regs.insert(dst, v);
    }

    fn unary(
        &mut self,
        ctx: &mut JitContext,
        regs: &mut HashMap<Reg, Value>,
        name: &'static str,
        dst: Reg,
        operand: Reg,
    ) {
        let args = [Self::reg_val(regs, operand)];
        let v = self.call_collect(ctx, name, &args, &[], &[]);
        regs.insert(dst, v);
    }

    /// Truthiness of a branch condition, via the runtime's own `oxy_is_truthy`.
    fn truthy(&mut self, ctx: &mut JitContext, cond: Option<&Value>) -> bool {
        if let Some(v) = cond.cloned() {
            self.push(ctx, v);
        } else {
            self.push(ctx, Value::Unit);
        }
        let (ptr, _) = self.engine.ffi_table["oxy_is_truthy"];
        let raw = unsafe { call_raw(ptr, ctx, &[], FfiRet::I8) };
        raw.unwrap_or(0) != 0
    }

    /// A `CallBuiltin`. Returns the value that lands in the result register.
    fn call_builtin(
        &mut self,
        ctx: &mut JitContext,
        func: &str,
        args: &[Reg],
        regs: &HashMap<Reg, Value>,
        immediates: &[usize],
        strings: &[String],
    ) -> Value {
        // User-function calls have no native pointer to jump to on this path;
        // they are interpreted recursively. Implemented in a later phase.
        if matches!(func, "oxy_call" | "oxy_call_closure") {
            ffi::set_error(
                ctx,
                format!("interpreter: user function calls not yet supported ({func})"),
            );
            return Value::Unit;
        }
        let arg_vals: Vec<Value> = args.iter().map(|&a| Self::reg_val(regs, a)).collect();
        self.call_collect(ctx, func, &arg_vals, strings, immediates)
    }

    /// Call `name` with the given register args + string/immediate ABI metadata,
    /// discarding the result.
    fn call_named(
        &mut self,
        ctx: &mut JitContext,
        name: &str,
        arg_vals: &[Value],
        strings: &[String],
        immediates: &[usize],
    ) {
        let _ = self.call_collect(ctx, name, arg_vals, strings, immediates);
    }

    /// Call `name`, pushing `arg_vals` onto the operand stack as register args,
    /// then the string/immediate ABI args. Returns the value it produces: the
    /// pushed result, the captured scalar return, or Unit.
    ///
    /// `sp_before` is captured *before* pushing args, so the post-call depth
    /// (the function consumes its args and may push one result) tells us whether
    /// a result was produced — independent of the arg count.
    fn call_collect(
        &mut self,
        ctx: &mut JitContext,
        name: &str,
        arg_vals: &[Value],
        strings: &[String],
        immediates: &[usize],
    ) -> Value {
        let (ptr, ret) = match self.engine.ffi_table.get(name) {
            Some(&entry) => entry,
            None => {
                ffi::set_error(ctx, format!("interpreter: unknown FFI function {name}"));
                return Value::Unit;
            }
        };

        let sp_before = ctx.sp;
        for v in arg_vals {
            self.push(ctx, v.clone());
        }

        // ABI: ctx, then each string as (ptr, len), then each immediate. All are
        // passed as i64 (matching codegen's iconst lowering). The string buffers
        // are borrowed from the live IR op, so the pointers stay valid.
        let mut raw: Vec<i64> = Vec::with_capacity(strings.len() * 2 + immediates.len());
        for s in strings {
            raw.push(s.as_ptr() as i64);
            raw.push(s.len() as i64);
        }
        for imm in immediates {
            raw.push(*imm as i64);
        }

        let scalar = unsafe { call_raw(ptr, ctx, &raw, ret) };

        // The function consumed its args. If it also pushed a result, sp now
        // exceeds the pre-arg baseline; pop it. A scalar return (loop
        // discriminants) is wrapped. Otherwise (e.g. println) Unit.
        if ctx.sp > sp_before {
            self.pop(ctx)
        } else if let Some(n) = scalar {
            match ret {
                FfiRet::I8 => Value::Bool(n != 0),
                _ => Value::I64(n),
            }
        } else {
            Value::Unit
        }
    }
}

// ── Free helpers ────────────────────────────────────────────────────────────

/// The JIT discriminant for a context: 2 if an error is set, else 0.
fn discriminant(ctx: &JitContext) -> Disc {
    if ctx.error_len > 0 {
        2
    } else {
        0
    }
}

/// Map each register to the block that defines it (skipping ops that define no
/// register), used to resolve SSA phi sources by predecessor.
fn reg_def_blocks(func: &IrFunction) -> HashMap<Reg, BlockId> {
    let mut map = HashMap::new();
    for block in &func.blocks {
        for op in &block.ops {
            if matches!(op, IrOp::StoreLocal(..) | IrOp::MakeCell(..)) {
                continue;
            }
            map.insert(op.result_reg(), block.id);
        }
    }
    map
}

/// Resolve a phi's value: prefer the source defined in the predecessor we came
/// from; otherwise whichever source currently holds a value.
fn phi_source(
    a: Reg,
    b: Reg,
    prev_block: Option<BlockId>,
    reg_def_block: &HashMap<Reg, BlockId>,
    regs: &HashMap<Reg, Value>,
) -> Option<Value> {
    if let Some(pred) = prev_block {
        if reg_def_block.get(&a) == Some(&pred) {
            if let Some(v) = regs.get(&a) {
                return Some(v.clone());
            }
        }
        if reg_def_block.get(&b) == Some(&pred) {
            if let Some(v) = regs.get(&b) {
                return Some(v.clone());
            }
        }
    }
    regs.get(&a).or_else(|| regs.get(&b)).cloned()
}

/// Call an `oxy_*` FFI pointer with `ctx` followed by `args` (all i64), matching
/// the codegen ABI. Returns the scalar result for `I64`/`I8` return kinds.
///
/// # Safety
/// `ptr` must point to an `extern "C"` function whose signature is
/// `(*mut JitContext, i64 × args.len()) -> {(), i64, i8}` consistent with `ret`.
unsafe fn call_raw(ptr: *const u8, ctx: &mut JitContext, args: &[i64], ret: FfiRet) -> Option<i64> {
    let c = ctx as *mut JitContext;
    // Maps each repetition element to the literal type `i64`, so the function
    // signature's parameter list repeats in lockstep with the argument indices.
    macro_rules! as_i64 {
        ($_idx:tt) => {
            i64
        };
    }
    macro_rules! dispatch {
        ($($n:literal => ($($idx:tt),*)),* $(,)?) => {
            match ret {
                FfiRet::Void => {
                    match args.len() {
                        $( $n => {
                            let f: extern "C" fn(*mut JitContext $(, as_i64!($idx))*) =
                                unsafe { std::mem::transmute(ptr) };
                            f(c $(, args[$idx])*);
                        } )*
                        n => panic!("interpreter: unsupported FFI arity {n}"),
                    }
                    None
                }
                FfiRet::I64 => {
                    match args.len() {
                        $( $n => {
                            let f: extern "C" fn(*mut JitContext $(, as_i64!($idx))*) -> i64 =
                                unsafe { std::mem::transmute(ptr) };
                            Some(f(c $(, args[$idx])*))
                        } )*
                        n => panic!("interpreter: unsupported FFI arity {n}"),
                    }
                }
                FfiRet::I8 => {
                    match args.len() {
                        $( $n => {
                            let f: extern "C" fn(*mut JitContext $(, as_i64!($idx))*) -> i8 =
                                unsafe { std::mem::transmute(ptr) };
                            Some(f(c $(, args[$idx])*) as i64)
                        } )*
                        n => panic!("interpreter: unsupported FFI arity {n}"),
                    }
                }
            }
        };
    }
    dispatch! {
        0 => (),
        1 => (0),
        2 => (0, 1),
        3 => (0, 1, 2),
        4 => (0, 1, 2, 3),
        5 => (0, 1, 2, 3, 4),
        6 => (0, 1, 2, 3, 4, 5),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile + type-check `src`, then run `main` through the interpreter and
    /// return its captured output.
    fn interp_capture(src: &str) -> Vec<String> {
        let mut program = crate::parser::parse(src).expect("parse");
        crate::vm::jit::expand_derives(&mut program);
        crate::type_checker::TypeChecker::new()
            .check_program(&program)
            .expect("type check");
        let engine = InterpEngine::compile(&program).expect("ir lowering");
        let mut interp = Interpreter::new(&engine);
        interp.with_captured_output();
        let _ = interp.run();
        interp.captured_output()
    }

    /// The same source run through the Cranelift JIT, for parity assertions.
    fn jit_capture(src: &str) -> Vec<String> {
        crate::vm::run_compiled_capturing(src).expect("jit run").1
    }

    /// Assert the interpreter and the JIT produce identical output.
    fn assert_parity(src: &str) {
        assert_eq!(interp_capture(src), jit_capture(src), "src:\n{src}");
    }

    #[test]
    fn interp_arithmetic_and_println() {
        assert_parity("fn main() { println!(\"{}\", 1 + 2 * 3 - 4); }");
    }

    #[test]
    fn interp_let_bindings_and_mutation() {
        assert_parity(
            "fn main() { let mut x = 10; x = x + 5; let y = x / 3; println!(\"{} {}\", x, y); }",
        );
    }

    #[test]
    fn interp_if_else() {
        assert_parity(
            "fn main() { let n = 7; if n % 2 == 0 { println!(\"even\"); } else { println!(\"odd\"); } }",
        );
    }

    #[test]
    fn interp_while_loop() {
        assert_parity(
            "fn main() { let mut i = 0; let mut sum = 0; while i < 5 { sum = sum + i; i = i + 1; } println!(\"{}\", sum); }",
        );
    }

    #[test]
    fn interp_vec_and_index() {
        assert_parity("fn main() { let v = vec![10, 20, 30]; println!(\"{} {}\", v[0], v[2]); }");
    }

    #[test]
    fn interp_string_and_bool() {
        assert_parity(
            "fn main() { let s = \"hi\"; let b = s == \"hi\"; println!(\"{} {}\", s, b); }",
        );
    }

    #[test]
    fn interp_float_arithmetic() {
        assert_parity("fn main() { let x = 3.5; let y = 2.0; println!(\"{}\", x * y); }");
    }
}
