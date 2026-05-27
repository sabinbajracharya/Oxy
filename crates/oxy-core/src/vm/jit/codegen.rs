//! Register IR → Cranelift CLIF code generator.

use cranelift_codegen::ir::{condcodes::IntCC, types, AbiParam, InstBuilder, UserFuncName};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::JITModule;
use cranelift_module::{FuncId, Linkage, Module};
use std::collections::HashMap;

use super::ir::*;

pub(crate) struct Codegen<'a> {
    module: &'a mut JITModule,
    fn_builder_ctx: &'a mut FunctionBuilderContext,
    ffi_ids: HashMap<String, FuncId>,
    pub(crate) fn_ptrs: HashMap<usize, *const u8>,
    pub(crate) fn_names: HashMap<String, usize>,
    next_fn_idx: usize,
}

impl<'a> Codegen<'a> {
    pub fn new(module: &'a mut JITModule, fn_builder_ctx: &'a mut FunctionBuilderContext) -> Self {
        Self {
            module,
            fn_builder_ctx,
            ffi_ids: HashMap::new(),
            fn_ptrs: HashMap::new(),
            fn_names: HashMap::new(),
            next_fn_idx: 0,
        }
    }

    pub fn declare_ffi(&mut self, name: &str, params: Vec<types::Type>, ret: Option<types::Type>) {
        let mut sig = self.module.make_signature();
        for p in &params {
            sig.params.push(AbiParam::new(*p));
        }
        if let Some(r) = ret {
            sig.returns.push(AbiParam::new(r));
        }
        let fid = self
            .module
            .declare_function(name, Linkage::Import, &sig)
            .unwrap_or_else(|e| panic!("declare FFI {name}: {e}"));
        self.ffi_ids.insert(name.to_string(), fid);
    }

    pub fn compile(&mut self, functions: Vec<IrFunction>) -> Result<(), String> {
        let mut pending: Vec<(FuncId, String)> = Vec::new();
        for func in functions {
            let (fid, name) = self.compile_fn(&func)?;
            pending.push((fid, name));
        }
        self.module
            .finalize_definitions()
            .map_err(|e| format!("finalize: {e}"))?;
        for (fid, name) in pending {
            let ptr = self.module.get_finalized_function(fid);
            let idx = self.next_fn_idx;
            self.next_fn_idx += 1;
            self.fn_ptrs.insert(idx, ptr);
            self.fn_names.insert(name, idx);
        }
        Ok(())
    }

    fn compile_fn(&mut self, ir_fn: &IrFunction) -> Result<(FuncId, String), String> {
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));

        let fid = self
            .module
            .declare_function(&ir_fn.name, Linkage::Export, &sig)
            .map_err(|e| format!("declare {}: {e}", ir_fn.name))?;

        let mut fn_ctx = self.module.make_context();
        fn_ctx.func.signature = sig.clone();
        fn_ctx.func.name = UserFuncName::user(0, self.next_fn_idx as u32);

        let mut ffi_refs: HashMap<String, cranelift_codegen::ir::FuncRef> = HashMap::new();
        for (name, ffid) in &self.ffi_ids {
            let fref = self.module.declare_func_in_func(*ffid, &mut fn_ctx.func);
            ffi_refs.insert(name.clone(), fref);
        }

        let mut builder = FunctionBuilder::new(&mut fn_ctx.func, self.fn_builder_ctx);

        let mut cl_blocks: HashMap<BlockId, cranelift_codegen::ir::Block> = HashMap::new();
        for b in &ir_fn.blocks {
            cl_blocks.insert(b.id, builder.create_block());
        }

        let entry_block = cl_blocks[&ir_fn.entry];
        builder.append_block_params_for_function_params(entry_block);

        let mut ctx_val: Option<cranelift_codegen::ir::Value> = None;

        let mut regs: HashMap<Reg, cranelift_codegen::ir::Value> = HashMap::new();
        let mut reg_slot: HashMap<Reg, usize> = HashMap::new();
        let mut next_spill_slot: usize = ir_fn.local_count;

        for block in &ir_fn.blocks {
            let cb = cl_blocks[&block.id];
            builder.switch_to_block(cb);

            if block.id == ir_fn.entry {
                ctx_val = Some(builder.block_params(cb)[0]);
            }

            let ctx = ctx_val.unwrap();

            for op in &block.ops {
                compile_op(
                    &mut builder,
                    ctx,
                    &ffi_refs,
                    op,
                    &mut regs,
                    &mut reg_slot,
                    &mut next_spill_slot,
                );
            }

            match &block.terminator {
                Terminator::Return(r) => {
                    let ret_is_int = matches!(
                        &ir_fn.return_type,
                        crate::type_checker::TypeInfo::I64 | crate::type_checker::TypeInfo::U8
                    );
                    if let Some(clif_val) = regs.get(r).copied() {
                        if ret_is_int {
                            if let Some(set_result) = ffi_refs.get("oxy_set_result_i64") {
                                builder.ins().call(*set_result, &[ctx, clif_val]);
                            }
                        } else {
                            push_return_value(
                                &mut builder,
                                ctx,
                                &ffi_refs,
                                clif_val,
                                &ir_fn.return_type,
                            );
                            if let Some(ret) = ffi_refs.get("oxy_return") {
                                builder.ins().call(*ret, &[ctx]);
                            }
                        }
                    } else if let Some(slot) = reg_slot.get(r).copied() {
                        if let Some(load_local) = ffi_refs.get("oxy_load_local") {
                            let slot_val = builder.ins().iconst(types::I64, slot as i64);
                            builder.ins().call(*load_local, &[ctx, slot_val]);
                        }
                        if let Some(ret) = ffi_refs.get("oxy_return") {
                            builder.ins().call(*ret, &[ctx]);
                        }
                    } else if let Some(ret) = ffi_refs.get("oxy_return") {
                        builder.ins().call(*ret, &[ctx]);
                    }
                    let disc = builder.ins().iconst(types::I64, 0);
                    builder.ins().return_(&[disc]);
                }
                Terminator::Jump(target) => {
                    builder.ins().jump(cl_blocks[target], &[]);
                }
                Terminator::Branch {
                    cond,
                    then_block,
                    else_block,
                } => {
                    let c = regs[cond];
                    let c_bool = builder.ins().icmp_imm(IntCC::NotEqual, c, 0);
                    builder.ins().brif(
                        c_bool,
                        cl_blocks[then_block],
                        &[],
                        cl_blocks[else_block],
                        &[],
                    );
                }
                Terminator::Halt => {
                    let disc = builder.ins().iconst(types::I64, 0);
                    builder.ins().return_(&[disc]);
                }
                Terminator::Panic(_) => {
                    let disc = builder.ins().iconst(types::I64, 2);
                    builder.ins().return_(&[disc]);
                }
                Terminator::Call { .. } => {
                    let disc = builder.ins().iconst(types::I64, 0);
                    builder.ins().return_(&[disc]);
                }
            }
        }

        builder.seal_all_blocks();
        builder.finalize();
        self.module
            .define_function(fid, &mut fn_ctx)
            .map_err(|e| format!("define {}: {e}", ir_fn.name))?;

        Ok((fid, ir_fn.name.clone()))
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn push_int(
    builder: &mut FunctionBuilder,
    ctx: cranelift_codegen::ir::Value,
    ffi_refs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
    val: cranelift_codegen::ir::Value,
) {
    if let Some(push) = ffi_refs.get("oxy_push_int") {
        builder.ins().call(*push, &[ctx, val]);
    }
}

fn spill_result(
    builder: &mut FunctionBuilder,
    ctx: cranelift_codegen::ir::Value,
    ffi_refs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
    reg: Reg,
    reg_slot: &mut HashMap<Reg, usize>,
    next_spill_slot: &mut usize,
) {
    let slot = *next_spill_slot;
    *next_spill_slot += 1;
    if let Some(store) = ffi_refs.get("oxy_store_local") {
        let slot_val = builder.ins().iconst(types::I64, slot as i64);
        builder.ins().call(*store, &[ctx, slot_val]);
    }
    reg_slot.insert(reg, slot);
}

fn push_reg(
    builder: &mut FunctionBuilder,
    ctx: cranelift_codegen::ir::Value,
    ffi_refs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
    reg: Reg,
    regs: &HashMap<Reg, cranelift_codegen::ir::Value>,
    reg_slot: &HashMap<Reg, usize>,
) {
    if let Some(clif_val) = regs.get(&reg).copied() {
        push_int(builder, ctx, ffi_refs, clif_val);
    } else if let Some(slot) = reg_slot.get(&reg).copied() {
        if let Some(load) = ffi_refs.get("oxy_load_local") {
            let slot_val = builder.ins().iconst(types::I64, slot as i64);
            builder.ins().call(*load, &[ctx, slot_val]);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn call_ffi_binary(
    builder: &mut FunctionBuilder,
    ctx: cranelift_codegen::ir::Value,
    ffi_refs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
    name: &str,
    lhs: Reg,
    rhs: Reg,
    regs: &HashMap<Reg, cranelift_codegen::ir::Value>,
    reg_slot: &HashMap<Reg, usize>,
) {
    push_reg(builder, ctx, ffi_refs, rhs, regs, reg_slot);
    push_reg(builder, ctx, ffi_refs, lhs, regs, reg_slot);
    if let Some(f) = ffi_refs.get(name) {
        builder.ins().call(*f, &[ctx]);
    }
}

fn call_ffi_unary(
    builder: &mut FunctionBuilder,
    ctx: cranelift_codegen::ir::Value,
    ffi_refs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
    name: &str,
    operand: Reg,
    regs: &HashMap<Reg, cranelift_codegen::ir::Value>,
    reg_slot: &HashMap<Reg, usize>,
) {
    push_reg(builder, ctx, ffi_refs, operand, regs, reg_slot);
    if let Some(f) = ffi_refs.get(name) {
        builder.ins().call(*f, &[ctx]);
    }
}

fn push_return_value(
    builder: &mut FunctionBuilder,
    ctx: cranelift_codegen::ir::Value,
    ffi_refs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
    val: cranelift_codegen::ir::Value,
    return_type: &crate::type_checker::TypeInfo,
) {
    match return_type {
        crate::type_checker::TypeInfo::Bool => {
            if let Some(push) = ffi_refs.get("oxy_push_bool") {
                let v = builder.ins().ireduce(types::I8, val);
                builder.ins().call(*push, &[ctx, v]);
            }
        }
        crate::type_checker::TypeInfo::Char => {
            if let Some(push) = ffi_refs.get("oxy_push_char") {
                let v = builder.ins().ireduce(types::I32, val);
                builder.ins().call(*push, &[ctx, v]);
            }
        }
        crate::type_checker::TypeInfo::F64 => {
            if let Some(push) = ffi_refs.get("oxy_push_float") {
                let v =
                    builder
                        .ins()
                        .bitcast(types::F64, cranelift_codegen::ir::MemFlags::new(), val);
                builder.ins().call(*push, &[ctx, v]);
            }
        }
        crate::type_checker::TypeInfo::Unit => {
            if let Some(push) = ffi_refs.get("oxy_push_unit") {
                builder.ins().call(*push, &[ctx]);
            }
        }
        _ => {
            if let Some(push) = ffi_refs.get("oxy_push_int") {
                builder.ins().call(*push, &[ctx, val]);
            }
        }
    }
}

// ── Op compilation ──────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn compile_op(
    builder: &mut FunctionBuilder,
    ctx: cranelift_codegen::ir::Value,
    ffi_refs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
    op: &IrOp,
    regs: &mut HashMap<Reg, cranelift_codegen::ir::Value>,
    reg_slot: &mut HashMap<Reg, usize>,
    next_spill_slot: &mut usize,
) {
    match op {
        IrOp::ConstInt(r, n) => {
            regs.insert(*r, builder.ins().iconst(types::I64, *n));
        }
        IrOp::ConstBool(r, b) => {
            regs.insert(*r, builder.ins().iconst(types::I64, *b as i64));
        }
        IrOp::ConstUnit(r) => {
            regs.insert(*r, builder.ins().iconst(types::I64, 0));
        }
        IrOp::Add(r, a, b) => {
            let v = builder.ins().iadd(regs[a], regs[b]);
            regs.insert(*r, v);
        }
        IrOp::Sub(r, a, b) => {
            let v = builder.ins().isub(regs[a], regs[b]);
            regs.insert(*r, v);
        }
        IrOp::Mul(r, a, b) => {
            let v = builder.ins().imul(regs[a], regs[b]);
            regs.insert(*r, v);
        }
        IrOp::Div(r, a, b) => {
            let v = builder.ins().sdiv(regs[a], regs[b]);
            regs.insert(*r, v);
        }
        IrOp::Rem(r, a, b) => {
            let v = builder.ins().srem(regs[a], regs[b]);
            regs.insert(*r, v);
        }
        IrOp::Eq(r, a, b) => {
            let c = builder.ins().icmp(IntCC::Equal, regs[a], regs[b]);
            regs.insert(*r, builder.ins().uextend(types::I64, c));
        }
        IrOp::Neq(r, a, b) => {
            let c = builder.ins().icmp(IntCC::NotEqual, regs[a], regs[b]);
            regs.insert(*r, builder.ins().uextend(types::I64, c));
        }
        IrOp::Lt(r, a, b) => {
            let c = builder.ins().icmp(IntCC::SignedLessThan, regs[a], regs[b]);
            regs.insert(*r, builder.ins().uextend(types::I64, c));
        }
        IrOp::Gt(r, a, b) => {
            let c = builder
                .ins()
                .icmp(IntCC::SignedGreaterThan, regs[a], regs[b]);
            regs.insert(*r, builder.ins().uextend(types::I64, c));
        }
        IrOp::Le(r, a, b) => {
            let c = builder
                .ins()
                .icmp(IntCC::SignedLessThanOrEqual, regs[a], regs[b]);
            regs.insert(*r, builder.ins().uextend(types::I64, c));
        }
        IrOp::Ge(r, a, b) => {
            let c = builder
                .ins()
                .icmp(IntCC::SignedGreaterThanOrEqual, regs[a], regs[b]);
            regs.insert(*r, builder.ins().uextend(types::I64, c));
        }
        IrOp::Copy(r, a) => {
            regs.insert(*r, regs[a]);
        }
        IrOp::Phi(r, a, _) => {
            regs.insert(*r, regs[a]);
        }

        IrOp::And(r, a, b) => {
            call_ffi_binary(builder, ctx, ffi_refs, "oxy_and", *a, *b, regs, reg_slot);
            spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
        }
        IrOp::Or(r, a, b) => {
            call_ffi_binary(builder, ctx, ffi_refs, "oxy_or", *a, *b, regs, reg_slot);
            spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
        }
        IrOp::BitAnd(r, a, b) => {
            call_ffi_binary(builder, ctx, ffi_refs, "oxy_bitand", *a, *b, regs, reg_slot);
            spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
        }
        IrOp::BitOr(r, a, b) => {
            call_ffi_binary(builder, ctx, ffi_refs, "oxy_bitor", *a, *b, regs, reg_slot);
            spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
        }
        IrOp::BitXor(r, a, b) => {
            call_ffi_binary(builder, ctx, ffi_refs, "oxy_bitxor", *a, *b, regs, reg_slot);
            spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
        }
        IrOp::Shl(r, a, b) => {
            call_ffi_binary(builder, ctx, ffi_refs, "oxy_shl", *a, *b, regs, reg_slot);
            spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
        }
        IrOp::Shr(r, a, b) => {
            call_ffi_binary(builder, ctx, ffi_refs, "oxy_shr", *a, *b, regs, reg_slot);
            spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
        }

        IrOp::Neg(r, a) => {
            call_ffi_unary(builder, ctx, ffi_refs, "oxy_neg", *a, regs, reg_slot);
            spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
        }
        IrOp::Not(r, a) => {
            call_ffi_unary(builder, ctx, ffi_refs, "oxy_not", *a, regs, reg_slot);
            spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
        }
        IrOp::BitNot(r, a) => {
            call_ffi_unary(builder, ctx, ffi_refs, "oxy_bitnot", *a, regs, reg_slot);
            spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
        }

        IrOp::ConstFloat(r, n) => {
            if let Some(push) = ffi_refs.get("oxy_push_float") {
                let v = builder.ins().f64const(*n);
                builder.ins().call(*push, &[ctx, v]);
            }
            spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
        }
        IrOp::ConstChar(r, c) => {
            if let Some(push) = ffi_refs.get("oxy_push_char") {
                let v = builder.ins().iconst(types::I32, *c as i64);
                builder.ins().call(*push, &[ctx, v]);
            }
            spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
        }
        IrOp::ConstString(r, s) => {
            if let Some(push) = ffi_refs.get("oxy_push_string") {
                let ptr = builder.ins().iconst(types::I64, s.as_ptr() as i64);
                let len = builder.ins().iconst(types::I64, s.len() as i64);
                builder.ins().call(*push, &[ctx, ptr, len]);
            }
            spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
        }

        IrOp::LoadLocal(r, slot) => {
            if let Some(load) = ffi_refs.get("oxy_load_local") {
                let slot_val = builder.ins().iconst(types::I64, *slot as i64);
                builder.ins().call(*load, &[ctx, slot_val]);
            }
            spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
        }
        IrOp::StoreLocal(slot, src) => {
            push_reg(builder, ctx, ffi_refs, *src, regs, reg_slot);
            if let Some(store) = ffi_refs.get("oxy_store_local") {
                let slot_val = builder.ins().iconst(types::I64, *slot as i64);
                builder.ins().call(*store, &[ctx, slot_val]);
            }
        }

        IrOp::CallBuiltin {
            result,
            func,
            args,
            immediates,
            strings,
        } => {
            for imm in immediates {
                let v = builder.ins().iconst(types::I64, *imm as i64);
                push_int(builder, ctx, ffi_refs, v);
            }
            for s in strings {
                let ptr = builder.ins().iconst(types::I64, s.as_ptr() as i64);
                let len = builder.ins().iconst(types::I64, s.len() as i64);
                if let Some(push) = ffi_refs.get("oxy_push_string") {
                    builder.ins().call(*push, &[ctx, ptr, len]);
                }
            }
            for arg in args.iter().rev() {
                push_reg(builder, ctx, ffi_refs, *arg, regs, reg_slot);
            }
            if let Some(f) = ffi_refs.get(*func) {
                builder.ins().call(*f, &[ctx]);
            }
            spill_result(builder, ctx, ffi_refs, *result, reg_slot, next_spill_slot);
        }

        IrOp::ReadResult(r) => {
            if let Some(ret) = ffi_refs.get("oxy_return") {
                builder.ins().call(*ret, &[ctx]);
            }
            spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
        }
        IrOp::WriteResult(r) => {
            push_reg(builder, ctx, ffi_refs, *r, regs, reg_slot);
            if let Some(ret) = ffi_refs.get("oxy_return") {
                builder.ins().call(*ret, &[ctx]);
            }
        }
        IrOp::SetError(r) => {
            push_reg(builder, ctx, ffi_refs, *r, regs, reg_slot);
            if let Some(panic) = ffi_refs.get("oxy_panic") {
                builder.ins().call(*panic, &[ctx]);
            }
        }
        IrOp::CheckError(r) => {
            regs.insert(*r, builder.ins().iconst(types::I64, 0));
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::vm::api::run_compiled_jit;

    #[test]
    fn test_e2e_literal_int() {
        let result = run_compiled_jit("fn main() -> int { 42 }");
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(42));
    }

    #[test]
    fn test_e2e_add() {
        let result = run_compiled_jit("fn main() -> int { 1 + 2 }");
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(3));
    }

    #[test]
    fn test_e2e_bool_true() {
        let result = run_compiled_jit("fn main() -> bool { true }");
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::Bool(true));
    }

    #[test]
    fn test_e2e_bool_and() {
        let result = run_compiled_jit("fn main() -> bool { true && false }");
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::Bool(false));
    }

    #[test]
    fn test_e2e_bool_or() {
        let result = run_compiled_jit("fn main() -> bool { true || false }");
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::Bool(true));
    }
}
