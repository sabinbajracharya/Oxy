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
            module, fn_builder_ctx, ffi_ids: HashMap::new(),
            fn_ptrs: HashMap::new(), fn_names: HashMap::new(), next_fn_idx: 0,
        }
    }

    pub fn declare_ffi(&mut self, name: &str, params: Vec<types::Type>, ret: Option<types::Type>) {
        let mut sig = self.module.make_signature();
        for p in &params { sig.params.push(AbiParam::new(*p)); }
        if let Some(r) = ret { sig.returns.push(AbiParam::new(r)); }
        let fid = self.module.declare_function(name, Linkage::Import, &sig)
            .unwrap_or_else(|e| panic!("declare FFI {name}: {e}"));
        self.ffi_ids.insert(name.to_string(), fid);
    }

    pub fn compile(&mut self, functions: Vec<IrFunction>) -> Result<(), String> {
        let mut pending: Vec<(FuncId, String)> = Vec::new();
        for func in functions {
            let (fid, name) = self.compile_fn(&func)?;
            pending.push((fid, name));
        }
        self.module.finalize_definitions().map_err(|e| format!("finalize: {e}"))?;
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

        let fid = self.module.declare_function(&ir_fn.name, Linkage::Export, &sig)
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

        for block in &ir_fn.blocks {
            let cb = cl_blocks[&block.id];
            builder.switch_to_block(cb);

            if block.id == ir_fn.entry {
                ctx_val = Some(builder.block_params(cb)[0]);
            }

            let ctx = ctx_val.unwrap();
            let mut regs: HashMap<Reg, cranelift_codegen::ir::Value> = HashMap::new();

            for op in &block.ops {
                compile_op(&mut builder, ctx, &ffi_refs, op, &mut regs);
            }

            match &block.terminator {
                Terminator::Return(r) => {
                    let val = regs.get(r).copied().unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
                    if let Some(set_result) = ffi_refs.get("oxy_set_result_i64") {
                        builder.ins().call(*set_result, &[ctx, val]);
                    }
                    let disc = builder.ins().iconst(types::I64, 0);
                    builder.ins().return_(&[disc]);
                }
                Terminator::Jump(target) => {
                    builder.ins().jump(cl_blocks[target], &[]);
                }
                Terminator::Branch { cond, then_block, else_block } => {
                    let c = regs[cond];
                    builder.ins().brif(c, cl_blocks[then_block], &[], cl_blocks[else_block], &[]);
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
        self.module.define_function(fid, &mut fn_ctx)
            .map_err(|e| format!("define {}: {e}", ir_fn.name))?;

        Ok((fid, ir_fn.name.clone()))
    }
}

fn compile_op(
    builder: &mut FunctionBuilder,
    ctx: cranelift_codegen::ir::Value,
    ffi_refs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
    op: &IrOp,
    regs: &mut HashMap<Reg, cranelift_codegen::ir::Value>,
) {
    match op {
        IrOp::ConstInt(r, n) => { regs.insert(*r, builder.ins().iconst(types::I64, *n)); }
        IrOp::ConstBool(r, b) => { regs.insert(*r, builder.ins().iconst(types::I8, *b as i64)); }
        IrOp::ConstUnit(r) => { regs.insert(*r, builder.ins().iconst(types::I64, 0)); }
        IrOp::Add(r, a, b) => { let v = builder.ins().iadd(regs[a], regs[b]); regs.insert(*r, v); }
        IrOp::Sub(r, a, b) => { let v = builder.ins().isub(regs[a], regs[b]); regs.insert(*r, v); }
        IrOp::Mul(r, a, b) => { let v = builder.ins().imul(regs[a], regs[b]); regs.insert(*r, v); }
        IrOp::Div(r, a, b) => { let v = builder.ins().sdiv(regs[a], regs[b]); regs.insert(*r, v); }
        IrOp::Rem(r, a, b) => { let v = builder.ins().srem(regs[a], regs[b]); regs.insert(*r, v); }
        IrOp::Eq(r, a, b) => { let c = builder.ins().icmp(IntCC::Equal, regs[a], regs[b]); regs.insert(*r, builder.ins().uextend(types::I64, c)); }
        IrOp::Neq(r, a, b) => { let c = builder.ins().icmp(IntCC::NotEqual, regs[a], regs[b]); regs.insert(*r, builder.ins().uextend(types::I64, c)); }
        IrOp::Lt(r, a, b) => { let c = builder.ins().icmp(IntCC::SignedLessThan, regs[a], regs[b]); regs.insert(*r, builder.ins().uextend(types::I64, c)); }
        IrOp::Gt(r, a, b) => { let c = builder.ins().icmp(IntCC::SignedGreaterThan, regs[a], regs[b]); regs.insert(*r, builder.ins().uextend(types::I64, c)); }
        IrOp::Le(r, a, b) => { let c = builder.ins().icmp(IntCC::SignedLessThanOrEqual, regs[a], regs[b]); regs.insert(*r, builder.ins().uextend(types::I64, c)); }
        IrOp::Ge(r, a, b) => { let c = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, regs[a], regs[b]); regs.insert(*r, builder.ins().uextend(types::I64, c)); }
        IrOp::Copy(r, a) => { regs.insert(*r, regs[a]); }
        IrOp::Phi(r, a, _) => { regs.insert(*r, regs[a]); }
        // All other ops: stub as zero
        _ => { regs.insert(op.result_reg(), builder.ins().iconst(types::I64, 0)); }
    }
}

// Helper to get result register from any IrOp
impl IrOp {
    fn result_reg(&self) -> Reg {
        match self {
            IrOp::ConstInt(r, _) | IrOp::ConstBool(r, _) | IrOp::ConstUnit(r)
            | IrOp::ConstFloat(r, _) | IrOp::ConstChar(r, _) | IrOp::ConstString(r, _)
            | IrOp::LoadLocal(r, _) | IrOp::Add(r, _, _) | IrOp::Sub(r, _, _)
            | IrOp::Mul(r, _, _) | IrOp::Div(r, _, _) | IrOp::Rem(r, _, _)
            | IrOp::Eq(r, _, _) | IrOp::Neq(r, _, _) | IrOp::Lt(r, _, _)
            | IrOp::Gt(r, _, _) | IrOp::Le(r, _, _) | IrOp::Ge(r, _, _)
            | IrOp::And(r, _, _) | IrOp::Or(r, _, _) | IrOp::BitAnd(r, _, _)
            | IrOp::BitOr(r, _, _) | IrOp::BitXor(r, _, _) | IrOp::Shl(r, _, _)
            | IrOp::Shr(r, _, _) | IrOp::Neg(r, _) | IrOp::Not(r, _)
            | IrOp::BitNot(r, _) | IrOp::Copy(r, _) | IrOp::Phi(r, _, _)
            | IrOp::CallBuiltin { result: r, .. } | IrOp::ReadResult(r)
            | IrOp::WriteResult(r) | IrOp::SetError(r) | IrOp::CheckError(r) => *r,
            IrOp::StoreLocal(_, _) => 0,
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
}
