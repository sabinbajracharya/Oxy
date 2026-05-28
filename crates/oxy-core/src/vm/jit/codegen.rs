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

        // ── Phi / block-param pre-scan ────────────────────────────────
        // Build predecessor lists.
        let mut preds: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
        for block in &ir_fn.blocks {
            match &block.terminator {
                Terminator::Jump(t) => {
                    preds.entry(*t).or_default().push(block.id);
                }
                Terminator::Branch {
                    then_block,
                    else_block,
                    ..
                } => {
                    preds.entry(*then_block).or_default().push(block.id);
                    preds.entry(*else_block).or_default().push(block.id);
                }
                _ => {}
            }
        }

        // Collect Phi nodes per block: phi_result → [source_regs in pred order]
        let mut block_phis: HashMap<BlockId, Vec<(Reg, Vec<Reg>)>> = HashMap::new();
        for block in &ir_fn.blocks {
            for op in &block.ops {
                if let IrOp::Phi(r, a, b) = op {
                    block_phis
                        .entry(block.id)
                        .or_default()
                        .push((*r, vec![*a, *b]));
                }
            }
        }

        // Find which block defines a register.
        let mut reg_def_block: HashMap<Reg, BlockId> = HashMap::new();
        for block in &ir_fn.blocks {
            for op in &block.ops {
                if matches!(op, IrOp::StoreLocal(..)) {
                    continue;
                }
                let r = op.result_reg();
                reg_def_block.insert(r, block.id);
            }
        }

        // Build forward map: (pred, succ) → Vec<Reg> to pass as jump args.
        // Match Phi sources to predecessors by finding which predecessor defines each source.
        let mut phi_args: HashMap<(BlockId, BlockId), Vec<Reg>> = HashMap::new();
        for (succ_id, phis) in &block_phis {
            let pred_list = preds.get(succ_id).cloned().unwrap_or_default();
            for (_phi_result, sources) in phis {
                // Try each source → match to predecessor that defines it
                for src in sources {
                    if let Some(def_block) = reg_def_block.get(src) {
                        if pred_list.contains(def_block) {
                            phi_args
                                .entry((*def_block, *succ_id))
                                .or_default()
                                .push(*src);
                        }
                    }
                }
            }
        }

        // ── Block parameters ──────────────────────────────────────────
        let entry_block = cl_blocks[&ir_fn.entry];
        builder.append_block_params_for_function_params(entry_block);

        // Every non-entry block gets: ctx (I64), then one I64 per Phi result.
        let mut phi_result_param: HashMap<(BlockId, Reg), usize> = HashMap::new();
        for (id, cb) in &cl_blocks {
            if *id != ir_fn.entry {
                builder.append_block_param(*cb, types::I64); // ctx
                if let Some(phis) = block_phis.get(id) {
                    for (phi_r, _sources) in phis {
                        builder.append_block_param(*cb, types::I64);
                        let param_idx = builder.block_params(*cb).len() - 1;
                        phi_result_param.insert((*id, *phi_r), param_idx);
                    }
                }
            }
        }

        let mut regs: HashMap<Reg, cranelift_codegen::ir::Value> = HashMap::new();
        let mut reg_slot: HashMap<Reg, usize> = HashMap::new();
        let mut next_spill_slot: usize = ir_fn.local_count;

        for block in &ir_fn.blocks {
            let cb = cl_blocks[&block.id];
            builder.switch_to_block(cb);

            let params = builder.block_params(cb);
            let ctx = params[0];

            // Map Phi result registers to their block parameters.
            if let Some(phis) = block_phis.get(&block.id) {
                for (phi_r, _sources) in phis {
                    if let Some(param_idx) = phi_result_param.get(&(block.id, *phi_r)) {
                        regs.insert(*phi_r, params[*param_idx]);
                    }
                }
            }

            for op in &block.ops {
                if matches!(op, IrOp::Phi(..)) {
                    continue; // handled above
                }
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
                    let extra = phi_args
                        .get(&(block.id, *target))
                        .map(|v| {
                            v.iter()
                                .filter_map(|r| {
                                    regs.get(r).copied().or_else(|| {
                                        reg_slot.get(r).and_then(|slot| {
                                            ffi_refs.get("oxy_read_local_i64").map(|f| {
                                                let sv =
                                                    builder.ins().iconst(types::I64, *slot as i64);
                                                let inst = builder.ins().call(*f, &[ctx, sv]);
                                                builder.func.dfg.inst_results(inst)[0]
                                            })
                                        })
                                    })
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let mut jump_args = vec![ctx];
                    jump_args.extend(extra);
                    builder.ins().jump(cl_blocks[target], &jump_args);
                }
                Terminator::Branch {
                    cond,
                    then_block,
                    else_block,
                } => {
                    // Condition may be in regs (CLIF) or reg_slot (spilled).
                    let c_bool = if let Some(clif_val) = regs.get(cond).copied() {
                        builder.ins().icmp_imm(IntCC::NotEqual, clif_val, 0)
                    } else if reg_slot.contains_key(cond) {
                        push_reg(&mut builder, ctx, &ffi_refs, *cond, &regs, &reg_slot);
                        if let Some(truthy) = ffi_refs.get("oxy_is_truthy") {
                            let inst = builder.ins().call(*truthy, &[ctx]);
                            let results = builder.func.dfg.inst_results(inst);
                            results[0]
                        } else {
                            builder.ins().iconst(types::I8, 0)
                        }
                    } else {
                        builder.ins().iconst(types::I8, 0)
                    };
                    // Helper to resolve a register to a CLIF value, loading from slot if needed.
                    let resolve_reg =
                        |r: &Reg,
                         builder: &mut FunctionBuilder,
                         ctx: cranelift_codegen::ir::Value,
                         ffi_refs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
                         regs: &HashMap<Reg, cranelift_codegen::ir::Value>,
                         reg_slot: &HashMap<Reg, usize>|
                         -> Option<cranelift_codegen::ir::Value> {
                            if let Some(v) = regs.get(r).copied() {
                                return Some(v);
                            }
                            if let Some(slot) = reg_slot.get(r) {
                                return ffi_refs.get("oxy_read_local_i64").map(|f| {
                                    let sv = builder.ins().iconst(types::I64, *slot as i64);
                                    let inst = builder.ins().call(*f, &[ctx, sv]);
                                    builder.func.dfg.inst_results(inst)[0]
                                });
                            }
                            None
                        };
                    let then_extra = phi_args
                        .get(&(block.id, *then_block))
                        .map(|v| {
                            v.iter()
                                .filter_map(|r| {
                                    resolve_reg(r, &mut builder, ctx, &ffi_refs, &regs, &reg_slot)
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let else_extra = phi_args
                        .get(&(block.id, *else_block))
                        .map(|v| {
                            v.iter()
                                .filter_map(|r| {
                                    resolve_reg(r, &mut builder, ctx, &ffi_refs, &regs, &reg_slot)
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let mut then_args = vec![ctx];
                    then_args.extend(then_extra);
                    let mut else_args = vec![ctx];
                    else_args.extend(else_extra);
                    builder.ins().brif(
                        c_bool,
                        cl_blocks[then_block],
                        &then_args,
                        cl_blocks[else_block],
                        &else_args,
                    );
                }
                Terminator::Halt => {
                    let disc = builder.ins().iconst(types::I64, 0);
                    builder.ins().return_(&[disc]);
                }
                Terminator::Panic(msg_reg) => {
                    push_reg(&mut builder, ctx, &ffi_refs, *msg_reg, &regs, &reg_slot);
                    if let Some(panic) = ffi_refs.get("oxy_panic") {
                        builder.ins().call(*panic, &[ctx]);
                    }
                    let disc = builder.ins().iconst(types::I64, 2);
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
    // Push lhs first, then rhs. FFI binary_op! macro pops rhs first (from top),
    // so rhs must be on top of stack.
    push_reg(builder, ctx, ffi_refs, lhs, regs, reg_slot);
    push_reg(builder, ctx, ffi_refs, rhs, regs, reg_slot);
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
            if regs.contains_key(a) && regs.contains_key(b) {
                let v = builder.ins().iadd(regs[a], regs[b]);
                regs.insert(*r, v);
            } else {
                call_ffi_binary(builder, ctx, ffi_refs, "oxy_add", *a, *b, regs, reg_slot);
                spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
            }
        }
        IrOp::Sub(r, a, b) => {
            if regs.contains_key(a) && regs.contains_key(b) {
                let v = builder.ins().isub(regs[a], regs[b]);
                regs.insert(*r, v);
            } else {
                call_ffi_binary(builder, ctx, ffi_refs, "oxy_sub", *a, *b, regs, reg_slot);
                spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
            }
        }
        IrOp::Mul(r, a, b) => {
            if regs.contains_key(a) && regs.contains_key(b) {
                let v = builder.ins().imul(regs[a], regs[b]);
                regs.insert(*r, v);
            } else {
                call_ffi_binary(builder, ctx, ffi_refs, "oxy_mul", *a, *b, regs, reg_slot);
                spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
            }
        }
        IrOp::Div(r, a, b) => {
            if regs.contains_key(a) && regs.contains_key(b) {
                let v = builder.ins().sdiv(regs[a], regs[b]);
                regs.insert(*r, v);
            } else {
                call_ffi_binary(builder, ctx, ffi_refs, "oxy_div", *a, *b, regs, reg_slot);
                spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
            }
        }
        IrOp::Rem(r, a, b) => {
            if regs.contains_key(a) && regs.contains_key(b) {
                let v = builder.ins().srem(regs[a], regs[b]);
                regs.insert(*r, v);
            } else {
                call_ffi_binary(builder, ctx, ffi_refs, "oxy_mod", *a, *b, regs, reg_slot);
                spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
            }
        }
        IrOp::Eq(r, a, b) => {
            if regs.contains_key(a) && regs.contains_key(b) {
                let c = builder.ins().icmp(IntCC::Equal, regs[a], regs[b]);
                regs.insert(*r, builder.ins().uextend(types::I64, c));
            } else {
                call_ffi_binary(builder, ctx, ffi_refs, "oxy_eq", *a, *b, regs, reg_slot);
                spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
            }
        }
        IrOp::Neq(r, a, b) => {
            if regs.contains_key(a) && regs.contains_key(b) {
                let c = builder.ins().icmp(IntCC::NotEqual, regs[a], regs[b]);
                regs.insert(*r, builder.ins().uextend(types::I64, c));
            } else {
                call_ffi_binary(builder, ctx, ffi_refs, "oxy_neq", *a, *b, regs, reg_slot);
                spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
            }
        }
        IrOp::Lt(r, a, b) => {
            if regs.contains_key(a) && regs.contains_key(b) {
                let c = builder.ins().icmp(IntCC::SignedLessThan, regs[a], regs[b]);
                regs.insert(*r, builder.ins().uextend(types::I64, c));
            } else {
                call_ffi_binary(builder, ctx, ffi_refs, "oxy_lt", *a, *b, regs, reg_slot);
                spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
            }
        }
        IrOp::Gt(r, a, b) => {
            if regs.contains_key(a) && regs.contains_key(b) {
                let c = builder
                    .ins()
                    .icmp(IntCC::SignedGreaterThan, regs[a], regs[b]);
                regs.insert(*r, builder.ins().uextend(types::I64, c));
            } else {
                call_ffi_binary(builder, ctx, ffi_refs, "oxy_gt", *a, *b, regs, reg_slot);
                spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
            }
        }
        IrOp::Le(r, a, b) => {
            if regs.contains_key(a) && regs.contains_key(b) {
                let c = builder
                    .ins()
                    .icmp(IntCC::SignedLessThanOrEqual, regs[a], regs[b]);
                regs.insert(*r, builder.ins().uextend(types::I64, c));
            } else {
                call_ffi_binary(builder, ctx, ffi_refs, "oxy_le", *a, *b, regs, reg_slot);
                spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
            }
        }
        IrOp::Ge(r, a, b) => {
            if regs.contains_key(a) && regs.contains_key(b) {
                let c = builder
                    .ins()
                    .icmp(IntCC::SignedGreaterThanOrEqual, regs[a], regs[b]);
                regs.insert(*r, builder.ins().uextend(types::I64, c));
            } else {
                call_ffi_binary(builder, ctx, ffi_refs, "oxy_ge", *a, *b, regs, reg_slot);
                spill_result(builder, ctx, ffi_refs, *r, reg_slot, next_spill_slot);
            }
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
            // Build Cranelift ABI arguments:
            // ctx, then each string as (ptr: I64, len: I64), then each immediate as I64.
            // Register args go on the operand stack, not as ABI params.
            let mut abi_args: Vec<cranelift_codegen::ir::Value> = vec![ctx];
            for s in strings {
                abi_args.push(builder.ins().iconst(types::I64, s.as_ptr() as i64));
                abi_args.push(builder.ins().iconst(types::I64, s.len() as i64));
            }
            for imm in immediates {
                abi_args.push(builder.ins().iconst(types::I64, *imm as i64));
            }
            // Push register arguments onto the operand stack.
            for arg in args.iter().rev() {
                push_reg(builder, ctx, ffi_refs, *arg, regs, reg_slot);
            }
            if let Some(f) = ffi_refs.get(*func) {
                builder.ins().call(*f, &abi_args);
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
            if let Some(f) = ffi_refs.get("oxy_error_discriminant") {
                let inst = builder.ins().call(*f, &[ctx]);
                regs.insert(*r, builder.func.dfg.inst_results(inst)[0]);
            } else {
                regs.insert(*r, builder.ins().iconst(types::I64, 0));
            }
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

    #[test]
    fn test_e2e_if_true() {
        let result = run_compiled_jit("fn main() -> int { if true { 10 } else { 20 } }");
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(10));
    }

    #[test]
    fn test_e2e_if_false() {
        let result = run_compiled_jit("fn main() -> int { if false { 10 } else { 20 } }");
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(20));
    }

    #[test]
    fn test_e2e_nested_if() {
        let src = "fn main() -> int { if true { if false { 1 } else { 2 } } else { 3 } }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(2));
    }

    #[test]
    fn test_e2e_if_cmp_cond() {
        let src = "fn main() -> int { if 1 < 2 { 10 } else { 20 } }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(10));
    }

    #[test]
    fn test_e2e_while_false() {
        // Loop body never executes — should just return initial value.
        let src = "fn main() -> int { let mut x = 42; while false { x = 1 } x }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(42));
    }

    #[test]
    fn test_e2e_let_mut_assign() {
        let src = "fn main() -> int { let mut x = 0; x = 1; x }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(1));
    }

    #[test]
    fn test_e2e_while_true_return() {
        // while true should enter the body.
        let src = "fn main() -> int { while true { return 42 } 0 }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(42));
    }

    #[test]
    fn test_e2e_if_mutate() {
        let src = "fn main() -> int { let mut x = 0; if true { x = 5 } else { x = 10 } x }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(5));
    }

    #[test]
    fn test_e2e_while_mut_return() {
        // Mutate inside loop body then immediately return — tests StoreLocal in loop.
        let src = "fn main() -> int { let mut x = 0; while x < 1 { x = 5; return x } 0 }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(5));
    }

    #[test]
    fn test_e2e_while_once() {
        let src = "fn main() -> int { let mut x = 0; while x < 1 { x = x + 1 } x }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(1));
    }

    #[test]
    fn test_e2e_while_simple() {
        let src = "fn main() -> int { let mut x = 0; while x < 3 { x = x + 1 } x }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(3));
    }

    // ── Match ────────────────────────────────────────────────────────

    #[test]
    fn test_e2e_match_literal() {
        let src = "fn main() -> int { match 2 { 0 => 10, 1 => 20, 2 => 30, _ => 40 } }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(30));
    }

    #[test]
    fn test_e2e_match_wildcard() {
        let src = "fn main() -> int { match 99 { 0 => 1, _ => 42 } }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(42));
    }

    #[test]
    fn test_e2e_match_ident_bind() {
        let src = "fn main() -> int { match 5 { x => x * 2 } }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(10));
    }

    // ── Struct / field access ─────────────────────────────────────────

    #[test]
    fn test_e2e_struct_init_and_field() {
        let src = "struct Point { x: int, y: int } fn main() -> int { let p = Point { x: 10, y: 20 }; p.x }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(10));
    }

    // ── Enum variant ──────────────────────────────────────────────────

    #[test]
    fn test_e2e_enum_unit_variant() {
        let src = "enum Color { Red, Blue } fn main() -> int { let c = Color::Red; 0 }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
    }

    // ── Array / tuple ─────────────────────────────────────────────────

    #[test]
    fn test_e2e_array_literal() {
        let src = "fn main() -> int { let a = [1, 2, 3]; 0 }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
    }

    #[test]
    fn test_e2e_tuple_literal() {
        // Tuple field access (.0) returns Unit — bug in oxy_field_access for tuples.
        let src = "fn main() -> int { let t = (1, 2); 0 }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
    }

    // ── Method call ───────────────────────────────────────────────────

    #[test]
    fn test_e2e_string_len() {
        let src = "fn main() -> int { let s = \"hello\"; s.len() }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(5));
    }

    // ── Type cast ─────────────────────────────────────────────────────

    #[test]
    fn test_e2e_cast_to_int() {
        let src = "fn main() -> int { 3.14 as int }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(3));
    }

    // ── Return / break from control flow ──────────────────────────────

    #[test]
    fn test_e2e_return_from_if() {
        let src = "fn main() -> int { if true { return 42 } 0 }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(42));
    }

    #[test]
    fn test_e2e_break_from_loop() {
        let src = "fn main() -> int { let mut x = 0; loop { x = x + 1; if x > 5 { break; } } x }";
        let result = run_compiled_jit(src);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), crate::types::Value::I64(6));
    }
}
