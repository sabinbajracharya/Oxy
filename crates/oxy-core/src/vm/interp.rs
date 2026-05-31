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

// This module is the execution backend on `wasm32` (wired through api.rs target
// dispatch) and the reference engine for the `jit_interp_parity` test on native.
// On a native *non-test* build (e.g. the CLI, which always uses the JIT) nothing
// here is reachable, so allow dead code rather than litter cfg attributes.
#![allow(dead_code)]

use std::collections::HashMap;

use crate::types::Value;

use super::jit::ffi::{self, FfiRet};
use super::jit::ir::{BlockId, IrFunction, IrOp, Reg, Terminator};
use super::jit::ir_gen::IrGen;
use super::jit::{ClosureRuntimeMeta, JitContext, JitTables};
use super::VmResult;

/// Explicitly mark a runtime feature as **not implemented by the IR interpreter**
/// (the wasm/browser execution backend). Sets a clear interpreter error in `ctx`
/// and evaluates to `Value::Unit`, so the next `Return`/`Halt` surfaces it as a
/// normal runtime error instead of the feature misbehaving silently.
///
/// This is the deliberate, greppable opt-out the project's two-backend policy
/// calls for: when a feature is reachable through the shared FFI but genuinely
/// cannot run without native code (e.g. the async scheduler driving JIT'd
/// tasks), route it here rather than letting it fall through to an FFI that does
/// the wrong thing on an empty `fn_table`. To *support* such a feature on wasm,
/// implement it in this module and remove the marker. See CLAUDE.md
/// "Two execution backends".
///
/// Currently nothing routes here: higher-order built-ins and async eager-runs
/// both call back into the interpreter via the FFI closure-invoker hook (see
/// `install_invoker`). The macro is kept as the ready, greppable opt-out for any
/// future feature that genuinely cannot run without native code.
#[allow(unused_macros)]
macro_rules! unsupported_on_wasm {
    ($ctx:expr, $feature:expr) => {{
        ffi::set_error(
            $ctx,
            format!("{}: {}", $feature, ffi::INTERP_UNSUPPORTED_MARKER),
        );
        Value::Unit
    }};
}

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
    pub(crate) fn run(&self) -> VmResult {
        self.run_function("main")
    }

    /// Run a named function with no arguments (entry point / `#[test]`).
    pub(crate) fn run_function(&self, name: &str) -> VmResult {
        let func = match self.engine.function(name) {
            Some(f) => f,
            None => return VmResult::Error(format!("function not found: {name}")),
        };
        // Install a fresh per-execution scheduler (no global state shared
        // across parallel tests) and the closure-invoker hook. Both are
        // restored via RAII guards on return.
        let _sched = ffi::SchedulerGuard::new();
        let _invoker = self.install_invoker();
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
    fn interpret(&self, ctx: &mut JitContext, func: &IrFunction) -> Disc {
        let reg_def_block = reg_def_blocks(func);
        let mut regs: HashMap<Reg, Value> = HashMap::new();
        let mut block_id: BlockId = func.entry;
        let mut prev_block: Option<BlockId> = None;

        loop {
            let block = &func.blocks[block_id];
            for op in &block.ops {
                self.exec_op(ctx, op, &mut regs, prev_block, &reg_def_block);
                // A genuine runtime error (panic, failed builtin, unsupported
                // feature) aborts the function immediately, like the JIT's
                // panic path — otherwise a later op (e.g. an `assert_eq`) would
                // overwrite `ctx.error_*` and mask the real cause. The empty `?`
                // marker is NOT a real error: it is consumed by a following
                // `CheckError`, so it must not short-circuit here.
                if has_real_error(ctx) {
                    return discriminant(ctx);
                }
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
                Terminator::Panic(_msg_reg) => {
                    // Error is already set by the preceding SetError op or oxy_try_pop.
                    // Just exit with the error discriminant; the register is informational.
                    return 2;
                }
            }
        }
    }

    /// Execute a single non-terminator op, updating the register file.
    fn exec_op(
        &self,
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
            //
            // Arithmetic and bitwise ops are operator-overloadable on user
            // structs/enums; `binary`/`unary` recognize that from the FFI op
            // name (see `overload_method`) and dispatch to the `Type::<method>`
            // trait impl before the numeric fallback, mirroring the JIT macros.
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
        // Safety: ctx is always a valid, non-aliased JitContext allocated by fresh_ctx.
        // The operand stack has sufficient capacity.
        unsafe { ffi::push(ctx, v) }
    }

    fn pop(&self, ctx: &mut JitContext) -> Value {
        // Safety: ctx is valid and the operand stack has the correct depth per the IR.
        unsafe { ffi::pop(ctx) }
    }

    fn reg_val(regs: &HashMap<Reg, Value>, reg: Reg) -> Value {
        regs.get(&reg).cloned().unwrap_or(Value::Unit)
    }

    // ── FFI dispatch ─────────────────────────────────────────────────────────

    /// Push lhs, push rhs, call the binary op, store the popped result. If the
    /// op is operator-overloadable (see `overload_method`) and `lhs` is a user
    /// struct/enum with a matching `Type::<method>` impl, dispatch to that
    /// method instead (mirroring the JIT's `binary_op!` trait dispatch).
    fn binary(
        &self,
        ctx: &mut JitContext,
        regs: &mut HashMap<Reg, Value>,
        name: &'static str,
        dst: Reg,
        lhs: Reg,
        rhs: Reg,
    ) {
        let l = Self::reg_val(regs, lhs);
        let r = Self::reg_val(regs, rhs);
        if let Some(method) = overload_method(name) {
            if let Some(v) = self.try_op_overload(ctx, &l, std::slice::from_ref(&r), method) {
                regs.insert(dst, v);
                return;
            }
        }
        let v = self.call_collect(ctx, name, &[l, r], &[], &[]);
        regs.insert(dst, v);
    }

    fn unary(
        &self,
        ctx: &mut JitContext,
        regs: &mut HashMap<Reg, Value>,
        name: &'static str,
        dst: Reg,
        operand: Reg,
    ) {
        let v = Self::reg_val(regs, operand);
        if let Some(method) = overload_method(name) {
            if let Some(out) = self.try_op_overload(ctx, &v, &[], method) {
                regs.insert(dst, out);
                return;
            }
        }
        let out = self.call_collect(ctx, name, &[v], &[], &[]);
        regs.insert(dst, out);
    }

    /// Operator-overload dispatch: if `receiver` is a user struct/enum whose
    /// type defines `<Type>::<method>`, interpret that method with frame
    /// `[receiver, args..]` and return its result. `None` means no overload (a
    /// primitive operand, or no such impl) — the caller falls back to the FFI.
    fn try_op_overload(
        &self,
        ctx: &mut JitContext,
        receiver: &Value,
        args: &[Value],
        method: &str,
    ) -> Option<Value> {
        let lookup = match receiver {
            Value::Struct { name, .. } => name.clone(),
            Value::EnumVariant { enum_name, .. } => enum_name.clone(),
            _ => return None,
        };
        let qualified = format!("{lookup}::{method}");
        let &idx = self.engine.name_to_index.get(&qualified)?;
        let mut frame = vec![receiver.clone()];
        frame.extend_from_slice(args);
        let callee_fn = &self.engine.functions[idx];
        Some(self.invoke(ctx, callee_fn, frame))
    }

    /// Truthiness of a branch condition, via the runtime's own `oxy_is_truthy`.
    fn truthy(&self, ctx: &mut JitContext, cond: Option<&Value>) -> bool {
        if let Some(v) = cond.cloned() {
            self.push(ctx, v);
        } else {
            self.push(ctx, Value::Unit);
        }
        let (ptr, _) = self.engine.ffi_table["oxy_is_truthy"];
        // Safety: ptr is a valid oxy_* entry point from ffi_symbols; ctx is valid.
        let raw = unsafe { call_raw(ptr, ctx, &[], FfiRet::I8) };
        raw.unwrap_or(0) != 0
    }

    /// A `CallBuiltin`. Returns the value that lands in the result register.
    fn call_builtin(
        &self,
        ctx: &mut JitContext,
        func: &str,
        args: &[Reg],
        regs: &HashMap<Reg, Value>,
        immediates: &[usize],
        strings: &[String],
    ) -> Value {
        // User function/method calls jump to native code in the JIT; on this
        // path there is none, so we intercept them and interpret the callee
        // recursively. Everything else (and the async / built-in-method cases,
        // which never invoke native code) falls through to the shared FFI.
        match func {
            "oxy_call_closure" => {
                if let Some(v) = self.try_call_closure(ctx, args, regs) {
                    return v;
                }
            }
            "oxy_method_call" => {
                if let Some(v) = self.try_call_method(ctx, args, regs, strings) {
                    return v;
                }
            }
            // Module-qualified and associated-function calls (`math::add(..)`,
            // `Counter::new(..)`). On the JIT these resolve through the function
            // table; here the table is empty, so the shared FFI would fall
            // through to its stdlib/enum-variant handling and mis-handle a user
            // function (e.g. construct a bogus `EnumVariant`). Resolve by name first.
            "oxy_path_call_builtin" => {
                if let Some(v) = self.try_call_path(ctx, args, regs, strings) {
                    return v;
                }
            }
            // Async (`spawn`/`await`/`sleep`/`select`) flows to the shared FFI.
            // The scheduler runs each task eagerly; where the JIT would invoke a
            // task/future body through a native fn pointer, `oxy_spawn_ffi` /
            // `oxy_await_ffi` fall back to the installed interpreter hook (see
            // `install_invoker`) and interpret it instead. `sleep`/`select` touch
            // only the scheduler's virtual clock, so they need no native code.
            _ => {}
        }
        let arg_vals: Vec<Value> = args.iter().map(|&a| Self::reg_val(regs, a)).collect();
        self.call_collect(ctx, func, &arg_vals, strings, immediates)
    }

    /// Intercept a synchronous user-function/closure call. Returns `Some(result)`
    /// when handled here; `None` to fall through to the FFI (async closures build
    /// a `Future` and non-callables raise the proper error there).
    fn try_call_closure(
        &self,
        ctx: &mut JitContext,
        args: &[Reg],
        regs: &HashMap<Reg, Value>,
    ) -> Option<Value> {
        let callee = Self::reg_val(regs, *args.first()?);
        let Value::Function(f) = &callee else {
            return None;
        };
        if f.is_async {
            return None;
        }
        let target_ip = match f.target_ip {
            Some(ip) if ip != usize::MAX => ip,
            _ => return None,
        };

        // Frame layout matches oxy_call_closure: captures first, then args.
        let mut frame: Vec<Value> = f
            .captured_names
            .iter()
            .map(|name| f.closure_env.borrow().get(name).ok().unwrap_or(Value::Unit))
            .collect();
        frame.extend(args[1..].iter().map(|&a| Self::reg_val(regs, a)));

        let eng = self.engine;
        let callee_fn = &eng.functions[target_ip];
        Some(self.invoke(ctx, callee_fn, frame))
    }

    /// Intercept a call to a user-defined struct/enum method (`Type::method`).
    /// Returns `None` for built-in receivers (Vec/String/…), letting the FFI's
    /// built-in method dispatch handle them.
    fn try_call_method(
        &self,
        ctx: &mut JitContext,
        args: &[Reg],
        regs: &HashMap<Reg, Value>,
        strings: &[String],
    ) -> Option<Value> {
        let method = strings.first()?;
        let receiver = Self::reg_val(regs, *args.first()?);
        // A `mut self` receiver arrives Cell-wrapped; dispatch on the inner type
        // but hand the cell itself to the method (preserving write-back).
        let dispatch_value = match &receiver {
            Value::Cell(rc) => rc.borrow().clone(),
            other => other.clone(),
        };
        let lookup = match &dispatch_value {
            Value::Struct { name, .. } => name.clone(),
            Value::EnumVariant { enum_name, .. } => enum_name.clone(),
            other => other.type_name().to_string(),
        };
        let qualified = format!("{lookup}::{method}");
        let &idx = self.engine.name_to_index.get(&qualified)?;

        // Frame layout matches invoke_compiled_method: receiver, then args.
        let mut frame = vec![receiver];
        frame.extend(args[1..].iter().map(|&a| Self::reg_val(regs, a)));

        let eng = self.engine;
        let callee_fn = &eng.functions[idx];
        Some(self.invoke(ctx, callee_fn, frame))
    }

    /// Intercept a module-qualified / associated-function path call
    /// (`oxy_path_call_builtin`). The path arrives as a single NUL-separated
    /// string (the FFI splits on `\0`); joined with `::` it is the qualified
    /// function name. If it names a user function, interpret it with the call
    /// arguments as its frame. Returns `None` for stdlib/builtin paths (not in
    /// `name_to_index`), letting the shared FFI's registry dispatch handle them.
    fn try_call_path(
        &self,
        ctx: &mut JitContext,
        args: &[Reg],
        regs: &HashMap<Reg, Value>,
        strings: &[String],
    ) -> Option<Value> {
        let path = strings.first()?;
        let fn_name = path.split('\0').collect::<Vec<_>>().join("::");
        let &idx = self.engine.name_to_index.get(&fn_name)?;
        let frame: Vec<Value> = args.iter().map(|&a| Self::reg_val(regs, a)).collect();
        let callee_fn = &self.engine.functions[idx];
        Some(self.invoke(ctx, callee_fn, frame))
    }

    /// Interpret `callee_fn` in a fresh frame whose locals 0.. are `frame_locals`
    /// (captures/receiver then args), then propagate its error state back to the
    /// caller and return its result value — mirroring the JIT's callee-frame
    /// teardown (`CalleeFrame::execute` / `invoke_compiled_method`).
    fn invoke(
        &self,
        caller_ctx: &mut JitContext,
        callee_fn: &'e IrFunction,
        frame_locals: Vec<Value>,
    ) -> Value {
        let local_count = callee_fn.local_count.max(frame_locals.len());
        let mut callee_ctx = self.fresh_ctx(local_count);
        for (i, v) in frame_locals.into_iter().enumerate() {
            // Safety: i < local_count (guaranteed by callee_ctx construction);
            // local_slot asserts bounds. The slot holds a valid, initialized Value.
            unsafe { callee_ctx.local_slot(i).write(v) };
        }
        let _disc = self.interpret(&mut callee_ctx, callee_fn);
        let result = std::mem::replace(&mut callee_ctx.result, Value::Unit);
        propagate_error(&callee_ctx, caller_ctx);
        result
    }

    /// Interpret the function at `target_ip` with `frame_locals` as its initial
    /// locals (captures/receiver first, then args), returning its result or an
    /// error message. This is the body behind the FFI closure-invoker hook: the
    /// shared `oxy_*` runtime calls back here whenever it would otherwise invoke
    /// a compiled function through the (empty, on this backend) `fn_table` —
    /// higher-order built-ins, async eager-runs, user `Display::fmt`.
    ///
    /// The result mapping mirrors the JIT's `jit_closure_invoker`: discriminant 0
    /// is success; anything else is an error carrying `ctx.error_*`.
    fn invoke_target(&self, target_ip: usize, frame_locals: Vec<Value>) -> Result<Value, String> {
        if target_ip == usize::MAX || target_ip >= self.engine.functions.len() {
            return Err(format!(
                "interpreter: invalid closure target_ip {target_ip}"
            ));
        }
        let callee_fn = &self.engine.functions[target_ip];
        let local_count = callee_fn.local_count.max(frame_locals.len());
        let mut ctx = self.fresh_ctx(local_count);
        for (i, v) in frame_locals.into_iter().enumerate() {
            // Safety: i < local_count; local_slot asserts bounds.
            unsafe { ctx.local_slot(i).write(v) };
        }
        let disc = self.interpret(&mut ctx, callee_fn);
        let result = std::mem::replace(&mut ctx.result, Value::Unit);
        if disc == 0 {
            Ok(result)
        } else {
            Err(String::from_utf8_lossy(&ctx.error_msg[..ctx.error_len.min(1024)]).into_owned())
        }
    }

    /// Install this interpreter as the thread-local FFI closure-invoker hook for
    /// the duration of the returned guard. The guard restores the previous hook
    /// on drop, so nested/reentrant runs compose and the raw `self` pointer never
    /// outlives the borrow it was taken from.
    fn install_invoker(&self) -> InvokerGuard {
        let data = self as *const Self as *const ();
        let prev = ffi::set_interp_invoke(Some((interp_invoke_trampoline, data)));
        InvokerGuard { prev }
    }

    /// Call `name` with the given register args + string/immediate ABI metadata,
    /// discarding the result.
    fn call_named(
        &self,
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
        &self,
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

        // ABI: ctx, then each string as (ptr, len), then each immediate. Every
        // trailing scalar is passed **pointer-width** (`usize`), because that is
        // what the `oxy_*` functions the interpreter reaches actually declare —
        // slot indices, counts, `*const u8` string pointers and their lengths
        // are all pointer-width. On 64-bit native `usize == i64` (so this matches
        // the JIT's i64 iconst lowering), but on `wasm32` `usize == i32`; passing
        // these as a fixed `i64` made the transmuted call signature disagree with
        // the function's real wasm type and trapped with "indirect call signature
        // mismatch". The string buffers are borrowed from the live IR op, so the
        // pointers stay valid for the duration of the call.
        let mut raw: Vec<usize> = Vec::with_capacity(strings.len() * 2 + immediates.len());
        for s in strings {
            raw.push(s.as_ptr() as usize);
            raw.push(s.len());
        }
        for imm in immediates {
            raw.push(*imm);
        }

        // Safety: ptr is a valid oxy_* FFI entry point from ffi_symbols; ctx is
        // valid; raw args match the function's expected parameter count and types.
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

// ── Closure-invoker hook plumbing ─────────────────────────────────────────────

/// Trampoline matching `ffi::InterpInvokeFn`. Reconstructs the `Interpreter`
/// from the opaque data pointer installed by `install_invoker` and interprets
/// the requested target. Sound because `InvokerGuard` clears the hook before the
/// referenced `Interpreter` (and its `engine` borrow) is dropped, so `data`
/// never dangles while the hook is live.
fn interp_invoke_trampoline(
    data: *const (),
    target_ip: usize,
    frame: Vec<Value>,
) -> Result<Value, String> {
    // Safety: `data` is the opaque pointer installed by install_invoker(),
    // which derives from `&self` on the Interpreter. InvokerGuard clears the
    // hook before the Interpreter (and its `engine` borrow) is dropped, so
    // the pointer never dangles while the hook is live.
    let interp = unsafe { &*(data as *const Interpreter<'_>) };
    interp.invoke_target(target_ip, frame)
}

/// RAII guard returned by `Interpreter::install_invoker`. Restores the
/// previously-installed hook (usually `None`) when dropped.
struct InvokerGuard {
    prev: Option<(ffi::InterpInvokeFn, *const ())>,
}

impl Drop for InvokerGuard {
    fn drop(&mut self) {
        ffi::set_interp_invoke(self.prev.take());
    }
}

// ── Free helpers ────────────────────────────────────────────────────────────

/// Map an `oxy_*` binary/unary op FFI name to the trait method an operator
/// overload would define on a user type, or `None` for ops the JIT does not
/// overload (comparisons, `&&`/`||`). Single source for the same op→method
/// mapping the JIT encodes in its `binary_op!`/unary macros.
fn overload_method(ffi_name: &str) -> Option<&'static str> {
    match ffi_name {
        "oxy_add" => Some("add"),
        "oxy_sub" => Some("sub"),
        "oxy_mul" => Some("mul"),
        "oxy_div" => Some("div"),
        "oxy_mod" => Some("rem"),
        "oxy_bitand" => Some("bitand"),
        "oxy_bitor" => Some("bitor"),
        "oxy_bitxor" => Some("bitxor"),
        "oxy_shl" => Some("shl"),
        "oxy_shr" => Some("shr"),
        "oxy_neg" => Some("neg"),
        "oxy_not" => Some("not"),
        "oxy_bitnot" => Some("bitnot"),
        _ => None,
    }
}

/// The JIT discriminant for a context: 2 if an error is set, else 0.
fn discriminant(ctx: &JitContext) -> Disc {
    if ctx.error_len > 0 {
        2
    } else {
        0
    }
}

/// Whether a *real* runtime error is set, as opposed to the empty-message `?`
/// propagation marker (`error_len == 1 && error_msg[0] == 0`). Only real errors
/// abort the block loop; the `?` marker is left for `CheckError` to consume.
fn has_real_error(ctx: &JitContext) -> bool {
    ctx.error_len > 0 && !(ctx.error_len == 1 && ctx.error_msg[0] == 0)
}

/// Propagate a callee's error state to its caller, mirroring `CalleeFrame::execute`.
/// A real runtime error (non-empty message) bubbles up so the caller's next
/// `Return`/`Halt` reports it. The empty-message marker is the `?` short-circuit
/// signal; it has done its job inside the callee (the Err/None is now the return
/// value), so it is dropped rather than propagated — otherwise the caller's next
/// `CheckError` would fire spuriously.
fn propagate_error(callee_ctx: &JitContext, caller_ctx: &mut JitContext) {
    let is_empty_marker = callee_ctx.error_len == 1 && callee_ctx.error_msg[0] == 0;
    if callee_ctx.error_len > 0 && !is_empty_marker {
        let len = callee_ctx.error_len.min(1024);
        caller_ctx.error_msg[..len].copy_from_slice(&callee_ctx.error_msg[..len]);
        caller_ctx.error_len = callee_ctx.error_len;
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

/// Call an `oxy_*` FFI pointer with `ctx` followed by `args` (all pointer-width),
/// matching the runtime ABI. Returns the scalar result for `I64`/`I8` return
/// kinds.
///
/// Trailing args are `usize`, not `i64`: the functions the interpreter reaches
/// declare pointer-width params (slot indices, counts, `*const u8` pointers and
/// `usize` lengths), and on `wasm32` `usize == i32`. A `*const u8` and a `usize`
/// share the same wasm type (`i32`), so a uniform `usize` transmute target lines
/// up with both. Functions taking a genuinely 64-bit param (e.g. the const-push
/// helpers' `i64`/`f64`, `oxy_set_result_i64`) are codegen-only and must **not**
/// be routed here — every interpreter-reached `oxy_*` takes pointer-width args.
///
/// # Safety
/// `ptr` must point to an `extern "C"` function whose signature is
/// `(*mut JitContext, usize × args.len()) -> {(), i64, i8}` consistent with `ret`.
unsafe fn call_raw(
    ptr: *const u8,
    ctx: &mut JitContext,
    args: &[usize],
    ret: FfiRet,
) -> Option<i64> {
    let c = ctx as *mut JitContext;
    // Maps each repetition element to the literal type `usize`, so the function
    // signature's parameter list repeats in lockstep with the argument indices.
    macro_rules! as_usize {
        ($_idx:tt) => {
            usize
        };
    }
    macro_rules! dispatch {
        ($($n:literal => ($($idx:tt),*)),* $(,)?) => {
            match ret {
                FfiRet::Void => {
                    match args.len() {
                        $( $n => {
                            let f: extern "C" fn(*mut JitContext $(, as_usize!($idx))*) =
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
                            let f: extern "C" fn(*mut JitContext $(, as_usize!($idx))*) -> i64 =
                                unsafe { std::mem::transmute(ptr) };
                            Some(f(c $(, args[$idx])*))
                        } )*
                        n => panic!("interpreter: unsupported FFI arity {n}"),
                    }
                }
                FfiRet::I8 => {
                    match args.len() {
                        $( $n => {
                            let f: extern "C" fn(*mut JitContext $(, as_usize!($idx))*) -> i8 =
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

    /// Run `src`'s `main` through the interpreter and return the final result.
    fn interp_run(src: &str) -> VmResult {
        let mut program = crate::parser::parse(src).expect("parse");
        crate::vm::jit::expand_derives(&mut program);
        crate::type_checker::TypeChecker::new()
            .check_program(&program)
            .expect("type check");
        let engine = InterpEngine::compile(&program).expect("ir lowering");
        let interp = Interpreter::new(&engine);
        interp.run()
    }

    /// Async (`spawn`/`await`) runs eagerly on the interpreter: where the JIT
    /// would invoke the task body through a native fn pointer, `oxy_spawn_ffi`
    /// falls back to the installed closure-invoker hook and interprets it. The
    /// result must match the JIT's.
    #[test]
    fn interp_async_spawn_await() {
        assert_parity("fn main() { val h = spawn(|| 42); println(\"{}\", h.await); }");
    }

    /// A higher-order built-in (`map`/`filter`/`sum`) drives its closure through
    /// the same hook. This is the central Phase-4b capability.
    #[test]
    fn interp_higher_order_builtin() {
        assert_parity(
            "fn main() { val v = [1, 2, 3, 4]; \
             val s: Int = v.iter().map(|x| x * 2).filter(|x| x > 4).sum(); \
             println(\"{}\", s); }",
        );
    }

    /// Assert the interpreter and the JIT produce identical output.
    fn assert_parity(src: &str) {
        assert_eq!(interp_capture(src), jit_capture(src), "src:\n{src}");
    }

    #[test]
    fn interp_arithmetic_and_println() {
        assert_parity("fn main() { println(\"{}\", 1 + 2 * 3 - 4); }");
    }

    #[test]
    fn interp_let_bindings_and_mutation() {
        assert_parity(
            "fn main() { var x = 10; x = x + 5; val y = x / 3; println(\"{} {}\", x, y); }",
        );
    }

    #[test]
    fn interp_if_else() {
        assert_parity(
            "fn main() { val n = 7; if n % 2 == 0 { println(\"even\"); } else { println(\"odd\"); } }",
        );
    }

    #[test]
    fn interp_while_loop() {
        assert_parity(
            "fn main() { var i = 0; var sum = 0; while i < 5 { sum = sum + i; i = i + 1; } println(\"{}\", sum); }",
        );
    }

    #[test]
    fn interp_vec_and_index() {
        assert_parity("fn main() { val v = [10, 20, 30]; println(\"{} {}\", v[0], v[2]); }");
    }

    #[test]
    fn interp_string_and_bool() {
        assert_parity(
            "fn main() { val s = \"hi\"; val b = s == \"hi\"; println(\"{} {}\", s, b); }",
        );
    }

    #[test]
    fn interp_float_arithmetic() {
        assert_parity("fn main() { val x = 3.5; val y = 2.0; println(\"{}\", x * y); }");
    }

    #[test]
    fn interp_function_call() {
        assert_parity(
            "fn add(a: Int, b: Int) -> Int { a + b }\nfn main() { println(\"{}\", add(2, 3)); }",
        );
    }

    #[test]
    fn interp_recursion() {
        assert_parity(
            "fn fib(n: Int) -> Int { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }\nfn main() { println(\"{}\", fib(10)); }",
        );
    }

    #[test]
    fn interp_nested_calls() {
        assert_parity(
            "fn dbl(x: Int) -> Int { x * 2 }\nfn inc(x: Int) -> Int { x + 1 }\nfn main() { println(\"{}\", dbl(inc(dbl(5)))); }",
        );
    }

    #[test]
    fn interp_struct_method() {
        assert_parity(
            "struct Counter { n: Int }\nimpl Counter { fn get(self) -> Int { self.n } fn bump(self) { self.n = self.n + 1; } }\nfn main() { var c = Counter { n: 5 }; c.bump(); println(\"{}\", c.get()); }",
        );
    }

    #[test]
    fn interp_result_ok() {
        assert_parity(
            "fn half(n: Int) -> Result<Int, String> { if n % 2 == 0 { Ok(n / 2) } else { Err(\"odd\") } }\nfn main() { match half(10) { Ok(v) => println(\"ok {}\", v), Err(e) => println(\"err {}\", e) } }",
        );
    }

    #[test]
    fn interp_closure_direct_call() {
        assert_parity("fn main() { val f = |x| x + 1; println(\"{}\", f(5)); }");
    }

    #[test]
    fn interp_closure_capture() {
        assert_parity(
            "fn main() { val base = 100; val add = |x| x + base; println(\"{}\", add(7)); }",
        );
    }

    #[test]
    fn interp_operator_overload() {
        assert_parity(
            "struct V2 { x: Int, y: Int }\n\
             impl V2 { fn add(self, o: V2) -> V2 { V2 { x: self.x + o.x, y: self.y + o.y } } }\n\
             fn main() { val a = V2 { x: 1, y: 2 }; val b = V2 { x: 3, y: 4 }; val c = a + b; println(\"{} {}\", c.x, c.y); }",
        );
    }

    #[test]
    fn interp_associated_function() {
        assert_parity(
            "struct Counter { n: Int }\n\
             impl Counter { fn new() -> Counter { Counter { n: 42 } } }\n\
             fn main() { val c = Counter::new(); println(\"{}\", c.n); }",
        );
    }

    #[test]
    fn interp_module_function_call() {
        assert_parity(
            "mod math { pub fn square(x: Int) -> Int { x * x } }\n\
             fn main() { println(\"{}\", math::square(7)); }",
        );
    }

    #[test]
    fn interp_question_propagation() {
        assert_parity(
            "fn parse(n: Int) -> Result<Int, String> { if n < 0 { Err(\"neg\") } else { Ok(n) } }\nfn run(n: Int) -> Result<Int, String> { val x = parse(n)?; Ok(x + 1) }\nfn main() { match run(5) { Ok(v) => println(\"{}\", v), Err(e) => println(\"{}\", e) } match run(-1) { Ok(v) => println(\"{}\", v), Err(e) => println(\"{}\", e) } }",
        );
    }
}
