// FIXME: remove when JIT is wired into the execution path (Phase 6)
#![allow(dead_code)]
//! Bytecode-to-Cranelift translator.
//!
//! Walks Oxy bytecode (OpCode) linearly for each function body and emits
//! Cranelift IR instructions. Control flow (jumps, branches) becomes Cranelift
//! basic blocks connected by `jump`/`brif` terminators.

use crate::vm::{Chunk, OpCode};
use cranelift_codegen::ir::{
    condcodes::IntCC, types, AbiParam, Block, FuncRef, InstBuilder, UserFuncName,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::JITModule;
use cranelift_module::{FuncId, Linkage, Module};
use std::collections::{BTreeSet, HashMap};

pub(crate) struct Translator<'a> {
    chunk: &'a Chunk,
    module: &'a mut JITModule,
    ctx: &'a mut FunctionBuilderContext,
    ffi_func_ids: HashMap<String, FuncId>,
    ip_to_func: HashMap<usize, FuncId>,
}

impl<'a> Translator<'a> {
    pub fn new(
        chunk: &'a Chunk,
        module: &'a mut JITModule,
        ctx: &'a mut FunctionBuilderContext,
    ) -> Self {
        Self {
            chunk,
            module,
            ctx,
            ffi_func_ids: HashMap::new(),
            ip_to_func: HashMap::new(),
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
        self.ffi_func_ids.insert(name.to_string(), fid);
    }

    pub fn compile_all(mut self) -> HashMap<usize, *const u8> {
        let mut entries: Vec<(usize, usize)> = self
            .chunk
            .fn_frame_sizes
            .iter()
            .map(|(&ip, &sz)| (ip, sz))
            .collect();
        let main_ip = self.chunk.entry_point;
        if !entries.iter().any(|&(ip, _)| ip == main_ip) {
            entries.push((main_ip, self.chunk.local_count));
        }
        entries.sort_by_key(|&(ip, _)| ip);

        for (entry_ip, frame_size) in &entries {
            if self.ip_to_func.contains_key(entry_ip) {
                continue;
            }
            if let Err(e) = self.compile_function(*entry_ip, *frame_size) {
                eprintln!("JIT: failed to compile function at ip={entry_ip}: {e}");
            }
        }

        self.module.finalize_definitions().unwrap();

        let mut ptrs = HashMap::new();
        for (ip, fid) in &self.ip_to_func {
            let ptr = self.module.get_finalized_function(*fid);
            ptrs.insert(*ip, ptr);
        }
        ptrs
    }

    fn compile_function(&mut self, entry_ip: usize, _frame_size: usize) -> Result<(), String> {
        let func_name = self
            .chunk
            .functions
            .iter()
            .find(|(_, &ip)| ip == entry_ip)
            .map(|(n, _)| n.as_str())
            .unwrap_or("main");

        // --- Pre-compute all metadata BEFORE creating FunctionBuilder ---
        // (avoids borrow conflicts: builder borrows self.ctx mutably)
        let fn_end_ip = self.compute_function_end(entry_ip);
        let jump_targets = self.compute_jump_targets(entry_ip, fn_end_ip);

        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));

        let fid = self
            .module
            .declare_function(func_name, Linkage::Export, &sig)
            .map_err(|e| format!("declare {func_name}: {e}"))?;
        self.ip_to_func.insert(entry_ip, fid);

        let mut fn_ctx = self.module.make_context();
        fn_ctx.func.signature = sig;
        fn_ctx.func.name = UserFuncName::testcase(func_name);

        // Pre-declare FFI FuncRefs
        let ffi_names: Vec<String> = self.ffi_func_ids.keys().cloned().collect();
        let mut ffi_refs: HashMap<String, FuncRef> = HashMap::new();
        for name in &ffi_names {
            if let Some(&ffi_fid) = self.ffi_func_ids.get(name) {
                let fref = self.module.declare_func_in_func(ffi_fid, &mut fn_ctx.func);
                ffi_refs.insert(name.clone(), fref);
            }
        }

        let mut builder = FunctionBuilder::new(&mut fn_ctx.func, self.ctx);
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        let ctx_val = builder.block_params(entry_block)[0];

        // Create Cranelift blocks for each jump target
        let mut blocks: HashMap<usize, Block> = HashMap::new();
        for &t in &jump_targets {
            if t != entry_ip {
                blocks.insert(t, builder.create_block());
            }
        }
        blocks.insert(entry_ip, entry_block);

        let mut block_stack_depths: HashMap<usize, usize> = HashMap::new();
        block_stack_depths.insert(entry_ip, 0);

        // Walk bytecode and emit CLIF
        let mut ip = entry_ip;
        let mut stack_depth: usize = 0;
        let mut current_block_terminated = false;

        loop {
            if ip >= fn_end_ip || ip >= self.chunk.code.len() {
                break;
            }

            // Switch to block if needed. If the current block isn't
            // terminated, add a fallthrough jump first.
            if let Some(&blk) = blocks.get(&ip) {
                if ip != entry_ip {
                    let cur = builder.current_block().unwrap_or(entry_block);
                    if cur != blk && !current_block_terminated {
                        builder.ins().jump(blk, &[]);
                    }
                    if cur != blk {
                        builder.switch_to_block(blk);
                    }
                    if let Some(&expected) = block_stack_depths.get(&ip) {
                        stack_depth = expected;
                    }
                    current_block_terminated = false;
                }
            }

            let op = &self.chunk.code[ip];
            let terminated = translate_op(
                op,
                &mut builder,
                ctx_val,
                &ffi_refs,
                &blocks,
                &mut block_stack_depths,
                &mut stack_depth,
            );

            if terminated {
                // After a Jump, continue processing at ip+1 in a NEW block.
                if matches!(op, OpCode::Jump(_)) {
                    current_block_terminated = true;
                    let next_ip = ip + 1;
                    if next_ip < fn_end_ip && next_ip < self.chunk.code.len() {
                        let next_block = blocks
                            .entry(next_ip)
                            .or_insert_with(|| builder.create_block());
                        builder.switch_to_block(*next_block);
                        if let Some(&expected) = block_stack_depths.get(&next_ip) {
                            stack_depth = expected;
                        } else {
                            block_stack_depths.insert(next_ip, stack_depth);
                        }
                        ip = next_ip;
                        continue;
                    }
                }
                break;
            }
            ip += 1;
        }

        builder.seal_all_blocks();
        builder.finalize();

        self.module
            .define_function(fid, &mut fn_ctx)
            .map_err(|e| format!("define {func_name}: {e}"))?;
        self.module.clear_context(&mut fn_ctx);
        Ok(())
    }

    fn compute_function_end(&self, entry_ip: usize) -> usize {
        let mut next = self.chunk.code.len();
        for &ip in self.ip_to_func.keys() {
            if ip > entry_ip && ip < next {
                next = ip;
            }
        }
        for &ip in self.chunk.fn_frame_sizes.keys() {
            if ip > entry_ip && ip < next {
                next = ip;
            }
        }
        next
    }

    fn compute_jump_targets(&self, start_ip: usize, end_ip: usize) -> Vec<usize> {
        let mut targets = BTreeSet::new();
        targets.insert(start_ip);
        let end = end_ip.min(self.chunk.code.len());
        for ip in start_ip..end {
            match &self.chunk.code[ip] {
                OpCode::Jump(t) | OpCode::JumpIfFalse(t) | OpCode::JumpIfTrue(t) => {
                    targets.insert(*t);
                }
                OpCode::Call { target, .. } => {
                    targets.insert(*target);
                    targets.insert(ip + 1);
                }
                _ => {}
            }
        }
        targets.into_iter().collect()
    }
}

// ── Free function: translates a single opcode ────────────────────────
// Separate from Translator to avoid borrow conflicts with FunctionBuilder.

#[allow(clippy::too_many_arguments)]
fn translate_op(
    op: &OpCode,
    builder: &mut FunctionBuilder,
    ctx_val: cranelift_codegen::ir::Value,
    ffi_refs: &HashMap<String, FuncRef>,
    blocks: &HashMap<usize, Block>,
    block_stack_depths: &mut HashMap<usize, usize>,
    stack_depth: &mut usize,
) -> bool {
    // Helper: get a FuncRef by name.
    fn fref(ffi_refs: &HashMap<String, FuncRef>, name: &str) -> FuncRef {
        *ffi_refs
            .get(name)
            .unwrap_or_else(|| panic!("FFI {name} not found"))
    }

    // Call void FFI (ctx only).
    let call_void =
        |builder: &mut FunctionBuilder, ffi_refs: &HashMap<String, FuncRef>, name: &str| {
            let f = fref(ffi_refs, name);
            builder.ins().call(f, &[ctx_val]);
        };
    // Call FFI with 1 extra arg.
    let call1 =
        |builder: &mut FunctionBuilder, ffi_refs: &HashMap<String, FuncRef>, name: &str, a1| {
            let f = fref(ffi_refs, name);
            builder.ins().call(f, &[ctx_val, a1]);
        };
    // Call FFI with 2 extra args.
    let call2 =
        |builder: &mut FunctionBuilder, ffi_refs: &HashMap<String, FuncRef>, name: &str, a1, a2| {
            let f = fref(ffi_refs, name);
            builder.ins().call(f, &[ctx_val, a1, a2]);
        };
    // Call FFI with 3 extra args.
    let call3 = |builder: &mut FunctionBuilder,
                 ffi_refs: &HashMap<String, FuncRef>,
                 name: &str,
                 a1,
                 a2,
                 a3| {
        let f = fref(ffi_refs, name);
        builder.ins().call(f, &[ctx_val, a1, a2, a3]);
    };
    // Call FFI with 4 extra args.
    let call4 = |builder: &mut FunctionBuilder,
                 ffi_refs: &HashMap<String, FuncRef>,
                 name: &str,
                 a1,
                 a2,
                 a3,
                 a4| {
        let f = fref(ffi_refs, name);
        builder.ins().call(f, &[ctx_val, a1, a2, a3, a4]);
    };

    match op {
        // ── Constants ──────────────────────────────────────────
        OpCode::ConstUnit => {
            call_void(builder, ffi_refs, "oxy_push_unit");
            *stack_depth += 1;
            false
        }
        OpCode::ConstBool(b) => {
            let v = builder.ins().iconst(types::I8, i64::from(*b as u8));
            call1(builder, ffi_refs, "oxy_push_bool", v);
            *stack_depth += 1;
            false
        }
        OpCode::ConstInt(n, _w) => {
            let v = builder.ins().iconst(types::I64, *n);
            call1(builder, ffi_refs, "oxy_push_int", v);
            *stack_depth += 1;
            false
        }
        OpCode::ConstFloat(n, _w) => {
            let v = builder.ins().f64const(*n);
            call1(builder, ffi_refs, "oxy_push_float", v);
            *stack_depth += 1;
            false
        }
        OpCode::ConstChar(c) => {
            let v = builder.ins().iconst(types::I32, *c as u32 as i64);
            call1(builder, ffi_refs, "oxy_push_char", v);
            *stack_depth += 1;
            false
        }
        OpCode::ConstString(s) => {
            let ptr = builder.ins().iconst(types::I64, s.as_ptr() as i64);
            let len = builder.ins().iconst(types::I64, s.len() as i64);
            call2(builder, ffi_refs, "oxy_push_string", ptr, len);
            *stack_depth += 1;
            false
        }

        // ── Stack manipulation ─────────────────────────────────
        OpCode::Pop => {
            call_void(builder, ffi_refs, "oxy_pop");
            *stack_depth = stack_depth.saturating_sub(1);
            false
        }
        OpCode::Dup => {
            call_void(builder, ffi_refs, "oxy_dup");
            *stack_depth += 1;
            false
        }

        // ── Variables ──────────────────────────────────────────
        OpCode::LoadLocal(idx) => {
            let i = builder.ins().iconst(types::I64, *idx as i64);
            call1(builder, ffi_refs, "oxy_load_local", i);
            *stack_depth += 1;
            false
        }
        OpCode::StoreLocal(idx) => {
            let i = builder.ins().iconst(types::I64, *idx as i64);
            call1(builder, ffi_refs, "oxy_store_local", i);
            *stack_depth = stack_depth.saturating_sub(1);
            false
        }
        OpCode::MakeCell(idx) => {
            let i = builder.ins().iconst(types::I64, *idx as i64);
            call1(builder, ffi_refs, "oxy_make_cell", i);
            false
        }

        // ── Output ─────────────────────────────────────────────
        OpCode::Print => {
            call_void(builder, ffi_refs, "oxy_print_val");
            *stack_depth -= 1;
            false
        }
        OpCode::PrintLn => {
            call_void(builder, ffi_refs, "oxy_println_val");
            *stack_depth -= 1;
            false
        }

        // ── Binary arithmetic ──────────────────────────────────
        OpCode::Add => {
            call_void(builder, ffi_refs, "oxy_add");
            *stack_depth -= 1;
            false
        }
        OpCode::Sub => {
            call_void(builder, ffi_refs, "oxy_sub");
            *stack_depth -= 1;
            false
        }
        OpCode::Mul => {
            call_void(builder, ffi_refs, "oxy_mul");
            *stack_depth -= 1;
            false
        }
        OpCode::Div => {
            call_void(builder, ffi_refs, "oxy_div");
            *stack_depth -= 1;
            false
        }
        OpCode::Mod => {
            call_void(builder, ffi_refs, "oxy_mod");
            *stack_depth -= 1;
            false
        }
        OpCode::Eq => {
            call_void(builder, ffi_refs, "oxy_eq");
            *stack_depth -= 1;
            false
        }
        OpCode::Neq => {
            call_void(builder, ffi_refs, "oxy_neq");
            *stack_depth -= 1;
            false
        }
        OpCode::Lt => {
            call_void(builder, ffi_refs, "oxy_lt");
            *stack_depth -= 1;
            false
        }
        OpCode::Gt => {
            call_void(builder, ffi_refs, "oxy_gt");
            *stack_depth -= 1;
            false
        }
        OpCode::Le => {
            call_void(builder, ffi_refs, "oxy_le");
            *stack_depth -= 1;
            false
        }
        OpCode::Ge => {
            call_void(builder, ffi_refs, "oxy_ge");
            *stack_depth -= 1;
            false
        }
        OpCode::And => {
            call_void(builder, ffi_refs, "oxy_and");
            *stack_depth -= 1;
            false
        }
        OpCode::Or => {
            call_void(builder, ffi_refs, "oxy_or");
            *stack_depth -= 1;
            false
        }

        // ── Bitwise ────────────────────────────────────────────
        OpCode::BitAnd => {
            call_void(builder, ffi_refs, "oxy_bitand");
            *stack_depth -= 1;
            false
        }
        OpCode::BitOr => {
            call_void(builder, ffi_refs, "oxy_bitor");
            *stack_depth -= 1;
            false
        }
        OpCode::BitXor => {
            call_void(builder, ffi_refs, "oxy_bitxor");
            *stack_depth -= 1;
            false
        }
        OpCode::Shl => {
            call_void(builder, ffi_refs, "oxy_shl");
            *stack_depth -= 1;
            false
        }
        OpCode::Shr => {
            call_void(builder, ffi_refs, "oxy_shr");
            *stack_depth -= 1;
            false
        }

        // ── Unary ──────────────────────────────────────────────
        OpCode::Neg => {
            call_void(builder, ffi_refs, "oxy_neg");
            false
        }
        OpCode::Not => {
            call_void(builder, ffi_refs, "oxy_not");
            false
        }
        OpCode::BitNot => {
            call_void(builder, ffi_refs, "oxy_bitnot");
            false
        }

        // ── Control flow ────────────────────────────────────────
        OpCode::Jump(target) => {
            let tgt = *blocks
                .get(target)
                .unwrap_or_else(|| panic!("no block for {target}"));
            block_stack_depths.insert(*target, *stack_depth);
            builder.ins().jump(tgt, &[]);
            true
        }
        OpCode::JumpIfFalse(target) => {
            let tgt = *blocks
                .get(target)
                .unwrap_or_else(|| panic!("no block for {target}"));
            let else_blk = builder.create_block();
            let f = fref(ffi_refs, "oxy_is_falsy");
            let inst = builder.ins().call(f, &[ctx_val]);
            let val = builder.inst_results(inst)[0];
            let zero = builder.ins().iconst(types::I8, 0);
            let cond = builder.ins().icmp(IntCC::NotEqual, val, zero);
            block_stack_depths.insert(*target, *stack_depth - 1);
            builder.ins().brif(cond, tgt, &[], else_blk, &[]);
            builder.switch_to_block(else_blk);
            *stack_depth -= 1;
            false
        }
        OpCode::JumpIfTrue(target) => {
            let tgt = *blocks
                .get(target)
                .unwrap_or_else(|| panic!("no block for {target}"));
            let else_blk = builder.create_block();
            let f = fref(ffi_refs, "oxy_is_truthy");
            let inst = builder.ins().call(f, &[ctx_val]);
            let val = builder.inst_results(inst)[0];
            let zero = builder.ins().iconst(types::I8, 0);
            let cond = builder.ins().icmp(IntCC::NotEqual, val, zero);
            block_stack_depths.insert(*target, *stack_depth - 1);
            builder.ins().brif(cond, tgt, &[], else_blk, &[]);
            builder.switch_to_block(else_blk);
            *stack_depth -= 1;
            false
        }

        // ── Functions ──────────────────────────────────────────
        OpCode::Call { target, arg_count } => {
            let tgt = builder.ins().iconst(types::I64, *target as i64);
            let ac = builder.ins().iconst(types::I64, *arg_count as i64);
            call2(builder, ffi_refs, "oxy_call", tgt, ac);
            *stack_depth = *stack_depth - arg_count + 1;
            false
        }
        OpCode::Return => {
            call_void(builder, ffi_refs, "oxy_return");
            *stack_depth = (*stack_depth).saturating_sub(1);
            let zero = builder.ins().iconst(types::I64, 0);
            builder.ins().return_(&[zero]);
            true
        }
        OpCode::Halt => {
            let zero = builder.ins().iconst(types::I64, 0);
            builder.ins().return_(&[zero]);
            true
        }
        OpCode::Panic => {
            call_void(builder, ffi_refs, "oxy_panic");
            *stack_depth -= 1;
            let two = builder.ins().iconst(types::I64, 2);
            builder.ins().return_(&[two]);
            true
        }

        // ── Collections ────────────────────────────────────────
        OpCode::MakeArray { count } => {
            let c = builder.ins().iconst(types::I64, *count as i64);
            call1(builder, ffi_refs, "oxy_make_array", c);
            *stack_depth = *stack_depth - count + 1;
            false
        }
        OpCode::MakeFixedArray { count } => {
            let c = builder.ins().iconst(types::I64, *count as i64);
            call1(builder, ffi_refs, "oxy_make_fixed_array", c);
            *stack_depth = *stack_depth - count + 1;
            false
        }
        OpCode::MakeTuple { count } => {
            let c = builder.ins().iconst(types::I64, *count as i64);
            call1(builder, ffi_refs, "oxy_make_tuple", c);
            *stack_depth = *stack_depth - count + 1;
            false
        }
        OpCode::MakeIter => {
            call_void(builder, ffi_refs, "oxy_make_iter");
            false
        }
        OpCode::IterLen => {
            call_void(builder, ffi_refs, "oxy_iter_len");
            false
        }
        OpCode::VecIndex => {
            call_void(builder, ffi_refs, "oxy_vec_index");
            *stack_depth -= 1;
            false
        }
        OpCode::VecIndexStore => {
            call_void(builder, ffi_refs, "oxy_vec_index_store");
            *stack_depth -= 2;
            false
        }
        OpCode::MakeRange => {
            call_void(builder, ffi_refs, "oxy_make_range");
            *stack_depth -= 1;
            false
        }

        // ── Strings ────────────────────────────────────────────
        OpCode::ToString => {
            call_void(builder, ffi_refs, "oxy_to_string");
            false
        }
        OpCode::FStringConcat { count } => {
            let c = builder.ins().iconst(types::I64, *count as i64);
            call1(builder, ffi_refs, "oxy_fstring_concat", c);
            *stack_depth = *stack_depth - count + 1;
            false
        }
        OpCode::Format { arg_count } => {
            let c = builder.ins().iconst(types::I64, *arg_count as i64);
            call1(builder, ffi_refs, "oxy_format", c);
            *stack_depth = *stack_depth - arg_count + 1;
            false
        }

        // ── Structs / fields / methods ─────────────────────────
        OpCode::FieldAccess { field_name } => {
            let p = builder.ins().iconst(types::I64, field_name.as_ptr() as i64);
            let l = builder.ins().iconst(types::I64, field_name.len() as i64);
            call2(builder, ffi_refs, "oxy_field_access", p, l);
            false
        }
        OpCode::FieldStore(field_name) => {
            let p = builder.ins().iconst(types::I64, field_name.as_ptr() as i64);
            let l = builder.ins().iconst(types::I64, field_name.len() as i64);
            call2(builder, ffi_refs, "oxy_field_store", p, l);
            *stack_depth -= 2;
            false
        }
        OpCode::MethodCall {
            method_name,
            arg_count,
        } => {
            let p = builder
                .ins()
                .iconst(types::I64, method_name.as_ptr() as i64);
            let ml = builder.ins().iconst(types::I64, method_name.len() as i64);
            let ac = builder.ins().iconst(types::I64, *arg_count as i64);
            call3(builder, ffi_refs, "oxy_method_call", p, ml, ac);
            *stack_depth -= arg_count;
            false
        }

        OpCode::EnumVariantEqual { enum_name, variant } => {
            let enp = builder.ins().iconst(types::I64, enum_name.as_ptr() as i64);
            let enl = builder.ins().iconst(types::I64, enum_name.len() as i64);
            let vp = builder.ins().iconst(types::I64, variant.as_ptr() as i64);
            let vl = builder.ins().iconst(types::I64, variant.len() as i64);
            call4(
                builder,
                ffi_refs,
                "oxy_enum_variant_equal",
                enp,
                enl,
                vp,
                vl,
            );
            // Pops scrutinee, pushes Bool — net stack change is 0
            false
        }

        OpCode::StructInit {
            name,
            field_count,
            field_names,
        } => {
            let meta_idx = crate::vm::jit::ffi::register_struct_init(
                name.clone(),
                field_names.clone(),
                *field_count,
            );
            let mi = builder.ins().iconst(types::I64, meta_idx as i64);
            call1(builder, ffi_refs, "oxy_struct_init", mi);
            // Pops field_count values, pushes one struct
            *stack_depth = *stack_depth - field_count + 1;
            false
        }

        OpCode::StructUpdate {
            name,
            field_count,
            field_names,
        } => {
            let meta_idx = crate::vm::jit::ffi::register_struct_init(
                name.clone(),
                field_names.clone(),
                *field_count,
            );
            let mi = builder.ins().iconst(types::I64, meta_idx as i64);
            call1(builder, ffi_refs, "oxy_struct_update", mi);
            // Pops base + field_count overrides, pushes one struct
            *stack_depth -= field_count;
            false
        }

        // ── Stubbed / deferred ─────────────────────────────────
        _ => {
            // For stubbed opcodes, just track stack balance based on known effects
            match op {
                OpCode::ConstEnumVariant { .. } => *stack_depth += 1,
                OpCode::MakeEnumVariant { arg_count, .. } => {
                    *stack_depth = *stack_depth - arg_count + 1
                }
                OpCode::EnumDataGet(idx) => {
                    let i = builder.ins().iconst(types::I64, *idx as i64);
                    call1(builder, ffi_refs, "oxy_enum_data_get", i);
                }
                OpCode::Closure {
                    target_ip,
                    param_count,
                    meta_idx,
                    is_async,
                } => {
                    let tgt = builder.ins().iconst(types::I64, *target_ip as i64);
                    let pc = builder.ins().iconst(types::I64, *param_count as i64);
                    let mi = builder.ins().iconst(types::I64, *meta_idx as i64);
                    let ia = builder.ins().iconst(types::I8, i64::from(*is_async as u8));
                    // oxy_push_closure(ctx, target_ip, param_count, meta_idx, is_async)
                    let f = fref(ffi_refs, "oxy_push_closure");
                    builder.ins().call(f, &[ctx_val, tgt, pc, mi, ia]);
                    *stack_depth += 1; // pushes one closure value
                }
                OpCode::AsyncBlock {
                    target_ip,
                    meta_idx,
                } => {
                    let tgt = builder.ins().iconst(types::I64, *target_ip as i64);
                    let mi = builder.ins().iconst(types::I64, *meta_idx as i64);
                    // oxy_push_async_block(ctx, target_ip, meta_idx)
                    let f = fref(ffi_refs, "oxy_push_async_block");
                    builder.ins().call(f, &[ctx_val, tgt, mi]);
                    *stack_depth += 1;
                }
                OpCode::CallClosure { arg_count } => {
                    let ac = builder.ins().iconst(types::I64, *arg_count as i64);
                    // oxy_call_closure(ctx, arg_count)
                    let f = fref(ffi_refs, "oxy_call_closure");
                    builder.ins().call(f, &[ctx_val, ac]);
                    // Pops closure + args, pushes result
                    *stack_depth -= arg_count;
                }
                OpCode::MakeFuture { arg_count, .. } => *stack_depth = *stack_depth - arg_count + 1,
                OpCode::Await => {
                    let f = fref(ffi_refs, "oxy_await_ffi");
                    let inst = builder.ins().call(f, &[ctx_val]);
                    let disc = builder.inst_results(inst)[0];
                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_done = builder.ins().icmp(IntCC::Equal, disc, zero);
                    let cont_blk = builder.create_block();
                    let ret_blk = builder.create_block();
                    builder.ins().brif(is_done, cont_blk, &[], ret_blk, &[]);
                    builder.switch_to_block(ret_blk);
                    builder.ins().return_(&[disc]);
                    builder.switch_to_block(cont_blk);
                    *stack_depth += 1;
                }
                OpCode::Spawn => {
                    call_void(builder, ffi_refs, "oxy_spawn_ffi");
                }
                OpCode::Sleep => {
                    let f = fref(ffi_refs, "oxy_sleep_ffi");
                    let inst = builder.ins().call(f, &[ctx_val]);
                    *stack_depth -= 1;
                    let disc = builder.inst_results(inst)[0];
                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_done = builder.ins().icmp(IntCC::Equal, disc, zero);
                    let cont_blk = builder.create_block();
                    let ret_blk = builder.create_block();
                    builder.ins().brif(is_done, cont_blk, &[], ret_blk, &[]);
                    builder.switch_to_block(ret_blk);
                    builder.ins().return_(&[disc]);
                    builder.switch_to_block(cont_blk);
                }
                OpCode::Select { count } => {
                    let c = builder.ins().iconst(types::I64, *count as i64);
                    let f = fref(ffi_refs, "oxy_select_ffi");
                    let inst = builder.ins().call(f, &[ctx_val, c]);
                    let disc = builder.inst_results(inst)[0];
                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_done = builder.ins().icmp(IntCC::Equal, disc, zero);
                    let cont_blk = builder.create_block();
                    let ret_blk = builder.create_block();
                    builder.ins().brif(is_done, cont_blk, &[], ret_blk, &[]);
                    builder.switch_to_block(ret_blk);
                    builder.ins().return_(&[disc]);
                    builder.switch_to_block(cont_blk);
                    *stack_depth = *stack_depth - count + 1;
                }
                OpCode::TryPop => {
                    call_void(builder, ffi_refs, "oxy_try_pop");
                }
                OpCode::CastInt(_) => {
                    call_void(builder, ffi_refs, "oxy_cast_int");
                }
                OpCode::CastFloat(_) => {
                    call_void(builder, ffi_refs, "oxy_cast_float");
                }
                OpCode::CastToChar => {
                    call_void(builder, ffi_refs, "oxy_cast_to_char");
                }
                OpCode::BindIdent(idx) => {
                    let i = builder.ins().iconst(types::I64, *idx as i64);
                    call1(builder, ffi_refs, "oxy_bind_ident", i);
                    *stack_depth -= 1;
                }
                OpCode::PathCallBuiltin {
                    segments,
                    arg_count,
                } => {
                    let path_idx = crate::vm::jit::ffi::register_builtin_path(segments.clone());
                    let pi = builder.ins().iconst(types::I64, path_idx as i64);
                    let ac = builder.ins().iconst(types::I64, *arg_count as i64);
                    call2(builder, ffi_refs, "oxy_path_call_builtin", pi, ac);
                    *stack_depth = *stack_depth - arg_count + 1;
                }
                OpCode::DisplayArg => {
                    call_void(builder, ffi_refs, "oxy_display_arg");
                }
                _ => {}
            }
            false
        }
    }
}
