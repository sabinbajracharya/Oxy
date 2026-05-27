// FIXME: remove when JIT is wired into the execution path (Phase 6)
#![allow(dead_code)]
//! Bytecode-to-Cranelift translator.
//!
//! Walks Oxy bytecode (OpCode) linearly for each function body and emits
//! Cranelift IR instructions. Control flow (jumps, branches) becomes Cranelift
//! basic blocks connected by `jump`/`brif` terminators.

use crate::vm::{Chunk, OpCode};
use cranelift_codegen::ir::{
    condcodes::IntCC, types, AbiParam, Block, FuncRef, InstBuilder, MemFlags, UserFuncName,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::JITModule;
use cranelift_module::{FuncId, Linkage, Module};
use std::collections::{BTreeSet, HashMap};

// ── CFG types ──────────────────────────────────────────────────────

struct BasicBlock {
    /// First bytecode IP in this block (inclusive).
    start_ip: usize,
    /// One past the last bytecode IP in this block (exclusive).
    end_ip: usize,
    /// CFG successors (block indices, not IPs).
    successors: Vec<usize>,
    /// Cranelift block assigned during translation.
    clif_block: Option<Block>,
    /// Expected operand-stack depth on entry.
    stack_in: usize,
}

struct Cfg {
    blocks: Vec<BasicBlock>,
    /// RPO traversal order (block indices).
    rpo: Vec<usize>,
    /// Map from bytecode IP → block index.
    ip_to_block: HashMap<usize, usize>,
}

// ── Translator ─────────────────────────────────────────────────────

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

        // Also add closure/async-block target IPs from the bytecode.
        for op in &self.chunk.code {
            match op {
                OpCode::Closure {
                    target_ip,
                    param_count,
                    ..
                } => {
                    if !entries.iter().any(|&(eip, _)| eip == *target_ip) {
                        entries.push((*target_ip, *param_count));
                    }
                }
                OpCode::AsyncBlock { target_ip, .. } => {
                    if !entries.iter().any(|&(eip, _)| eip == *target_ip) {
                        entries.push((*target_ip, 0));
                    }
                }
                _ => {}
            }
        }
        // Also add nested entries detected via the forward-Jump pattern.
        for ip in 0..self.chunk.code.len() {
            if self.is_nested_entry(ip) && !entries.iter().any(|&(eip, _)| eip == ip) {
                entries.push((ip, 0));
            }
        }

        entries.sort_by_key(|&(ip, _)| ip);

        entries.sort_by_key(|&(ip, _)| ip);

        for (entry_ip, frame_size) in &entries {
            if self.ip_to_func.contains_key(entry_ip) {
                continue;
            }
            if let Err(e) = self.compile_function(*entry_ip, *frame_size) {
                eprintln!("JIT: failed to compile function at ip={entry_ip}: {e}");
            }
        }

        self.module.finalize_definitions().unwrap_or_else(|e| {
            eprintln!(
                "JIT: finalize_definitions warning: {e} ({} functions compiled)",
                self.ip_to_func.len()
            );
        });

        let mut ptrs = HashMap::new();
        for (ip, fid) in &self.ip_to_func {
            let ptr = self.module.get_finalized_function(*fid);
            ptrs.insert(*ip, ptr);
        }
        ptrs
    }

    fn compile_function(&mut self, entry_ip: usize, _frame_size: usize) -> Result<(), String> {
        let func_name: String = self
            .chunk
            .functions
            .iter()
            .find(|(_, &ip)| ip == entry_ip)
            .map(|(n, _)| n.clone())
            .unwrap_or_else(|| format!("closure_{entry_ip}"));
        let func_name: &str = func_name.as_str();

        // --- Pre-compute all metadata BEFORE creating FunctionBuilder ---
        // (avoids borrow conflicts: builder borrows self.ctx mutably)
        let fn_end_ip = self.compute_function_end(entry_ip);
        let nested_entry_ips: std::collections::HashSet<usize> = (0..self.chunk.code.len())
            .filter(|&ip| self.is_nested_entry(ip))
            .collect();

        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));

        let mut fn_ctx = self.module.make_context();
        let sig_for_decl = sig.clone();
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

        // Build the CFG BEFORE creating the FunctionBuilder (to avoid
        // borrow conflicts: builder borrows self.ctx mutably).
        let mut cfg = self.analyze_cfg(entry_ip, fn_end_ip, &nested_entry_ips);

        // Find all yield-point IPs (Await/Sleep/Select) for resume dispatch.
        let end_scan = fn_end_ip.min(self.chunk.code.len());
        let mut yield_ips: Vec<usize> = Vec::new();
        for ip in entry_ip..end_scan {
            if nested_entry_ips.contains(&ip) && ip != entry_ip {
                continue;
            }
            match &self.chunk.code[ip] {
                OpCode::Await | OpCode::Sleep | OpCode::Select { .. } => {
                    yield_ips.push(ip);
                }
                _ => {}
            }
        }

        let mut builder = FunctionBuilder::new(&mut fn_ctx.func, self.ctx);
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        let ctx_val = builder.block_params(entry_block)[0];

        // Assign Cranelift blocks to each CFG basic block.
        // The entry_block is used for the resume dispatch; the first CFG block
        // gets its own block (so we can jump to it from the dispatch).
        for idx in 0..cfg.blocks.len() {
            cfg.blocks[idx].clif_block = Some(builder.create_block());
        }

        // The first CFG block in RPO order — all dispatch paths lead here eventually.
        let first_cfg_block = cfg.blocks[cfg.rpo[0]]
            .clif_block
            .expect("first CFG block has no clif_block");

        // Emit resume dispatch in entry_block.
        if !yield_ips.is_empty() {
            // Load resume_ip from JitContext (offset 32 = after buffer + local_count + sp + capacity).
            let resume_ip_addr = builder.ins().iadd_imm(ctx_val, 32);
            let resume_ip = builder
                .ins()
                .load(types::I64, MemFlags::new(), resume_ip_addr, 0);
            let zero = builder.ins().iconst(types::I64, 0);
            let is_zero = builder.ins().icmp(IntCC::Equal, resume_ip, zero);

            let dispatch_block = builder.create_block();
            builder
                .ins()
                .brif(is_zero, first_cfg_block, &[], dispatch_block, &[]);

            // In dispatch_block: chain of comparisons for each yield IP.
            builder.switch_to_block(dispatch_block);
            for &yield_ip in &yield_ips {
                let target = cfg
                    .blocks
                    .iter()
                    .find(|b| b.start_ip == yield_ip)
                    .and_then(|b| b.clif_block);
                if let Some(target_block) = target {
                    let yield_val = builder.ins().iconst(types::I64, yield_ip as i64);
                    let is_match = builder.ins().icmp(IntCC::Equal, resume_ip, yield_val);
                    let next_check = builder.create_block();
                    builder
                        .ins()
                        .brif(is_match, target_block, &[], next_check, &[]);
                    builder.switch_to_block(next_check);
                }
            }
            // No match — fall through to the first CFG block.
            builder.ins().jump(first_cfg_block, &[]);
        } else {
            // No yield points — entry block is dead code, jump to first CFG block.
            builder.ins().jump(first_cfg_block, &[]);
        }

        // Switch to the first CFG block so the RPO loop starts translating there.
        builder.switch_to_block(first_cfg_block);

        // Build a HashMap for compatibility with translate_op (IP→Block lookups)
        let mut blocks: HashMap<usize, Block> = HashMap::new();
        for block in &cfg.blocks {
            if let Some(clif_blk) = block.clif_block {
                blocks.insert(block.start_ip, clif_blk);
            }
        }

        // Build the block_stack_depths map for JumpIfTrue/False targets
        let mut block_stack_depths: HashMap<usize, usize> = HashMap::new();

        // Yield-point blocks are entered via the resume dispatch with values
        // on the stack (the FFI functions push back their arguments before
        // yielding). Set initial stack depths for these blocks.
        for &yield_ip in &yield_ips {
            match self.chunk.code.get(yield_ip) {
                Some(OpCode::Select { count }) => {
                    block_stack_depths.insert(yield_ip, *count);
                }
                _ => {
                    // Await, Sleep: 1 value on stack (JoinHandle/ms)
                    block_stack_depths.insert(yield_ip, 1);
                }
            }
        }

        // Walk blocks in RPO order, emitting each block's bytecodes
        let rpo = cfg.rpo.clone();
        let block_data = std::mem::take(&mut cfg.blocks);
        let mut is_first = true;
        for &blk_idx in &rpo {
            let block = &block_data[blk_idx];
            let clif_blk = block.clif_block.expect("block has no clif_block");

            if !is_first {
                // If the current block isn't terminated, add a fallthrough jump
                let cur = builder.current_block().unwrap_or(entry_block);
                if cur != clif_blk {
                    // Don't add jump if the current block is already terminated
                    builder.switch_to_block(clif_blk);
                }
            }
            is_first = false;

            let mut stack_depth = block_stack_depths
                .get(&block.start_ip)
                .copied()
                .unwrap_or(0);

            // Translate each opcode in this block
            let mut block_terminated = false;
            for ip in block.start_ip..block.end_ip {
                let op = &self.chunk.code[ip];
                let terminated = translate_op(
                    op,
                    &mut builder,
                    ctx_val,
                    &ffi_refs,
                    &blocks,
                    &mut block_stack_depths,
                    &mut stack_depth,
                    ip,
                );

                if terminated {
                    block_terminated = true;
                    break;
                }
            }

            // If this block didn't have a terminator, it falls through
            // to its unique successor. Add an explicit jump.
            if !block_terminated {
                if let Some(&succ_idx) = block.successors.first() {
                    let succ_block = block_data[succ_idx]
                        .clif_block
                        .expect("successor has no clif_block");
                    builder.ins().jump(succ_block, &[]);
                }
            }
        }

        builder.seal_all_blocks();
        builder.finalize();

        let fid = self
            .module
            .declare_function(func_name, Linkage::Export, &sig_for_decl)
            .map_err(|e| format!("declare {func_name}: {e}"))?;

        self.module.define_function(fid, &mut fn_ctx).map_err(|e| {
            let bytecode = self.dump_function_bytecode(entry_ip);
            format!("define {func_name}: {e}\nBytecode:\n{bytecode}")
        })?;
        self.module.clear_context(&mut fn_ctx);
        self.ip_to_func.insert(entry_ip, fid);
        Ok(())
    }

    // Helper to dump bytecode for context when verifier fails
    fn dump_function_bytecode(&self, entry_ip: usize) -> String {
        let fn_end_ip = self.compute_function_end(entry_ip);
        let mut s = String::new();
        let mut ip = entry_ip;
        while ip < fn_end_ip && ip < self.chunk.code.len() {
            let op = &self.chunk.code[ip];
            use std::fmt::Write;
            let _ = writeln!(s, "  {ip:4}: {op:?}");
            ip += 1;
        }
        s
    }

    /// Returns true if an entry at `ip` is a nested closure/async block.
    /// Closures are preceded by a forward Jump that skips over the closure body;
    /// the jump target contains a Closure, AsyncBlock, or similar opcode.
    fn is_nested_entry(&self, ip: usize) -> bool {
        if ip == 0 {
            return false;
        }
        match self.chunk.code.get(ip - 1) {
            Some(OpCode::Jump(target)) if *target > ip => {
                // Verify the jump target is a closure-related opcode
                matches!(
                    self.chunk.code.get(*target),
                    Some(OpCode::Closure { .. } | OpCode::AsyncBlock { .. })
                )
            }
            _ => false,
        }
    }

    fn compute_function_end(&self, entry_ip: usize) -> usize {
        // If this entry is a closure/async block (preceded by a forward Jump),
        // find its end from the jump target that skips over it.
        if self.is_nested_entry(entry_ip) {
            if let Some(OpCode::Jump(skip_target)) = self.chunk.code.get(entry_ip - 1) {
                return *skip_target;
            }
        }

        let mut next = self.chunk.code.len();
        for &ip in self.ip_to_func.keys() {
            if ip > entry_ip && ip < next && !self.is_nested_entry(ip) {
                next = ip;
            }
        }
        for &ip in self.chunk.fn_frame_sizes.keys() {
            if ip > entry_ip && ip < next && !self.is_nested_entry(ip) {
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
                _ => {}
            }
        }
        targets.into_iter().collect()
    }

    // ── CFG analysis ────────────────────────────────────────────────

    /// Build the CFG for a function body ranging from `entry_ip` to `fn_end_ip`.
    fn analyze_cfg(
        &self,
        entry_ip: usize,
        fn_end_ip: usize,
        nested_entry_ips: &std::collections::HashSet<usize>,
    ) -> Cfg {
        let end = fn_end_ip.min(self.chunk.code.len());

        // Pre-compute ranges of nested entries (closure/async bodies) to skip.
        // Don't include the current function's own entry IP (closures need to
        // compile their own body).
        let mut nested_ranges: Vec<(usize, usize)> = Vec::new();
        for &nip in nested_entry_ips {
            if nip > entry_ip && nip < end && nip > 0 {
                if let Some(OpCode::Jump(skip_target)) = self.chunk.code.get(nip - 1) {
                    nested_ranges.push((nip, *skip_target));
                }
            }
        }
        nested_ranges.sort_by_key(|&(s, _)| s);

        // Helper: check if an IP is inside a nested (closure) range
        let is_in_nested = |ip: usize| -> bool {
            nested_ranges
                .iter()
                .any(|&(start, end)| ip >= start && ip < end)
        };

        let mut leaders = BTreeSet::new();
        leaders.insert(entry_ip);

        // First pass: find block leaders (instructions that start a new block)
        for ip in entry_ip..end {
            // Skip bytecodes that belong to nested entries (closures),
            // but don't skip the function's own entry IP.
            if is_in_nested(ip) && ip != entry_ip {
                continue;
            }
            match &self.chunk.code[ip] {
                OpCode::Jump(target) => {
                    leaders.insert(*target);
                    let next_ip = ip + 1;
                    if next_ip < end && !nested_entry_ips.contains(&next_ip) {
                        leaders.insert(next_ip);
                    }
                }
                OpCode::JumpIfTrue(target) | OpCode::JumpIfFalse(target) => {
                    leaders.insert(*target);
                    leaders.insert(ip + 1);
                }
                OpCode::Return | OpCode::Halt | OpCode::Panic => {
                    if ip + 1 < end {
                        leaders.insert(ip + 1);
                    }
                }
                // Yield points (await, sleep, select) must be block leaders
                // so the resume dispatch can jump directly to them.
                OpCode::Await | OpCode::Sleep | OpCode::Select { .. } => {
                    leaders.insert(ip);
                }
                _ => {}
            }
        }

        // Build blocks from leaders, trimming at nested range boundaries
        let sorted_leaders: Vec<usize> = leaders.into_iter().collect();
        let mut ip_to_block: HashMap<usize, usize> = HashMap::new();
        let mut blocks: Vec<BasicBlock> = Vec::new();

        // Merge all "cut points": leaders + nested range starts + function end
        let mut cut_points: BTreeSet<usize> = BTreeSet::new();
        for &l in &sorted_leaders {
            cut_points.insert(l);
        }
        for &(start, _end) in &nested_ranges {
            cut_points.insert(start);
        }
        cut_points.insert(fn_end_ip);

        let sorted_cuts: Vec<usize> = cut_points.into_iter().collect();
        for i in 0..sorted_cuts.len() - 1 {
            let start = sorted_cuts[i];
            let block_end = sorted_cuts[i + 1];

            // Skip blocks that are entirely inside nested ranges,
            // but don't skip the function's own entry point (closures
            // have their entry inside their own nested range).
            if is_in_nested(start) && start != entry_ip {
                continue;
            }

            ip_to_block.insert(start, blocks.len());
            blocks.push(BasicBlock {
                start_ip: start,
                end_ip: block_end,
                successors: Vec::new(),
                clif_block: None,
                stack_in: 0,
            });
        }

        // Second pass: compute successors by examining each block's last instruction
        for idx in 0..blocks.len() {
            if blocks[idx].end_ip <= blocks[idx].start_ip {
                continue;
            }
            let last_ip = blocks[idx].end_ip - 1;
            let last_op = &self.chunk.code[last_ip];
            let succs = match last_op {
                OpCode::Jump(target) => {
                    let mut s = Vec::new();
                    if let Some(&si) = ip_to_block.get(target) {
                        s.push(si);
                    }
                    s
                }
                OpCode::JumpIfTrue(target) | OpCode::JumpIfFalse(target) => {
                    let mut s = Vec::new();
                    if let Some(&si) = ip_to_block.get(target) {
                        s.push(si);
                    }
                    // Fallthrough
                    if let Some(&si) = ip_to_block.get(&(last_ip + 1)) {
                        s.push(si);
                    }
                    s
                }
                OpCode::Return | OpCode::Halt | OpCode::Panic => Vec::new(),
                _ => {
                    // Non-terminator: falls through to next block
                    let mut s = Vec::new();
                    if idx + 1 < blocks.len() {
                        s.push(idx + 1);
                    }
                    s
                }
            };
            blocks[idx].successors = succs;
        }

        // Compute RPO via iterative post-order DFS
        let n = blocks.len();
        let mut rpo = Vec::with_capacity(n);
        let mut visited = vec![false; n];
        let mut in_stack = vec![false; n];
        let mut stack: Vec<(usize, usize)> = Vec::new();
        stack.push((0, 0));
        in_stack[0] = true;

        while let Some(&(idx, succ_idx)) = stack.last() {
            let succs = &blocks[idx].successors;
            if succ_idx < succs.len() {
                let next = succs[succ_idx];
                stack.last_mut().unwrap().1 = succ_idx + 1;
                if !visited[next] && !in_stack[next] {
                    stack.push((next, 0));
                    in_stack[next] = true;
                }
            } else {
                stack.pop();
                in_stack[idx] = false;
                visited[idx] = true;
                rpo.push(idx);
            }
        }
        rpo.reverse();

        Cfg {
            blocks,
            rpo,
            ip_to_block,
        }
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
    current_ip: usize,
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
    // Call FFI with 5 extra args.
    let call5 = |builder: &mut FunctionBuilder,
                 ffi_refs: &HashMap<String, FuncRef>,
                 name: &str,
                 a1,
                 a2,
                 a3,
                 a4,
                 a5| {
        let f = fref(ffi_refs, name);
        builder.ins().call(f, &[ctx_val, a1, a2, a3, a4, a5]);
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
            // Use the CFG-created block for the false branch (ip+1)
            let else_blk = *blocks
                .get(&(current_ip + 1))
                .unwrap_or_else(|| panic!("no block for false branch at {}", current_ip + 1));
            let f = fref(ffi_refs, "oxy_is_falsy");
            let inst = builder.ins().call(f, &[ctx_val]);
            let val = builder.inst_results(inst)[0];
            let zero = builder.ins().iconst(types::I8, 0);
            let cond = builder.ins().icmp(IntCC::NotEqual, val, zero);
            block_stack_depths.insert(*target, *stack_depth - 1);
            builder.ins().brif(cond, tgt, &[], else_blk, &[]);
            *stack_depth -= 1;
            true
        }
        OpCode::JumpIfTrue(target) => {
            let tgt = *blocks
                .get(target)
                .unwrap_or_else(|| panic!("no block for {target}"));
            let else_blk = *blocks
                .get(&(current_ip + 1))
                .unwrap_or_else(|| panic!("no block for false branch at {}", current_ip + 1));
            let f = fref(ffi_refs, "oxy_is_truthy");
            let inst = builder.ins().call(f, &[ctx_val]);
            let val = builder.inst_results(inst)[0];
            let zero = builder.ins().iconst(types::I8, 0);
            let cond = builder.ins().icmp(IntCC::NotEqual, val, zero);
            block_stack_depths.insert(*target, *stack_depth - 1);
            builder.ins().brif(cond, tgt, &[], else_blk, &[]);
            *stack_depth -= 1;
            true
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
            // Return 2 if any FFI function set an error, otherwise 0
            let f = fref(ffi_refs, "oxy_error_discriminant");
            let disc = builder.ins().call(f, &[ctx_val]);
            let disc_val = builder.inst_results(disc)[0];
            builder.ins().return_(&[disc_val]);
            true
        }
        OpCode::Halt => {
            let f = fref(ffi_refs, "oxy_error_discriminant");
            let disc = builder.ins().call(f, &[ctx_val]);
            let disc_val = builder.inst_results(disc)[0];
            builder.ins().return_(&[disc_val]);
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

        OpCode::MakeEnumVariant {
            enum_name,
            variant,
            arg_count,
        } => {
            let enp = builder.ins().iconst(types::I64, enum_name.as_ptr() as i64);
            let enl = builder.ins().iconst(types::I64, enum_name.len() as i64);
            let vp = builder.ins().iconst(types::I64, variant.as_ptr() as i64);
            let vl = builder.ins().iconst(types::I64, variant.len() as i64);
            let ac = builder.ins().iconst(types::I64, *arg_count as i64);
            call5(
                builder,
                ffi_refs,
                "oxy_make_enum_variant",
                enp,
                enl,
                vp,
                vl,
                ac,
            );
            // Pops arg_count values, pushes one enum variant
            *stack_depth = *stack_depth - arg_count + 1;
            false
        }

        OpCode::ConstEnumVariant {
            enum_name,
            variant,
            data,
        } => {
            let meta_idx = crate::vm::jit::ffi::register_const_enum_variant(
                enum_name.clone(),
                variant.clone(),
                data.clone(),
            );
            let mi = builder.ins().iconst(types::I64, meta_idx as i64);
            call1(builder, ffi_refs, "oxy_const_enum_variant", mi);
            *stack_depth += 1;
            false
        }

        OpCode::EnumDataGet(idx) => {
            let i = builder.ins().iconst(types::I64, *idx as i64);
            call1(builder, ffi_refs, "oxy_enum_data_get", i);
            // Pops one enum variant, pushes one data element — net stack change is 0
            false
        }

        OpCode::TryPop => {
            call_void(builder, ffi_refs, "oxy_try_pop");
            // Pops one, pushes one (or triggers early return) — net 0
            false
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
            let f = fref(ffi_refs, "oxy_push_closure");
            builder.ins().call(f, &[ctx_val, tgt, pc, mi, ia]);
            *stack_depth += 1;
            false
        }

        OpCode::AsyncBlock {
            target_ip,
            meta_idx,
        } => {
            let tgt = builder.ins().iconst(types::I64, *target_ip as i64);
            let mi = builder.ins().iconst(types::I64, *meta_idx as i64);
            let f = fref(ffi_refs, "oxy_push_async_block");
            builder.ins().call(f, &[ctx_val, tgt, mi]);
            *stack_depth += 1;
            false
        }

        OpCode::CallClosure { arg_count } => {
            let ac = builder.ins().iconst(types::I64, *arg_count as i64);
            let f = fref(ffi_refs, "oxy_call_closure");
            builder.ins().call(f, &[ctx_val, ac]);
            *stack_depth -= arg_count;
            false
        }

        OpCode::Await => {
            // Store resume IP so we can resume at this exact instruction.
            // resume_ip is at offset 32 in JitContext.
            let resume_ip_addr = builder.ins().iadd_imm(ctx_val, 32);
            let ip_val = builder.ins().iconst(types::I64, current_ip as i64);
            builder
                .ins()
                .store(MemFlags::new(), ip_val, resume_ip_addr, 0);
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
            false
        }

        OpCode::Spawn => {
            call_void(builder, ffi_refs, "oxy_spawn_ffi");
            false
        }

        OpCode::Sleep => {
            // Store resume IP so we can resume at this exact instruction.
            let resume_ip_addr = builder.ins().iadd_imm(ctx_val, 32);
            let ip_val = builder.ins().iconst(types::I64, current_ip as i64);
            builder
                .ins()
                .store(MemFlags::new(), ip_val, resume_ip_addr, 0);
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
            false
        }

        OpCode::Select { count } => {
            // Store resume IP so we can resume at this exact instruction.
            let resume_ip_addr = builder.ins().iadd_imm(ctx_val, 32);
            let ip_val = builder.ins().iconst(types::I64, current_ip as i64);
            builder
                .ins()
                .store(MemFlags::new(), ip_val, resume_ip_addr, 0);
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
            false
        }

        OpCode::CastInt(_) => {
            call_void(builder, ffi_refs, "oxy_cast_int");
            false
        }

        OpCode::CastFloat(_) => {
            call_void(builder, ffi_refs, "oxy_cast_float");
            false
        }

        OpCode::CastToChar => {
            call_void(builder, ffi_refs, "oxy_cast_to_char");
            false
        }

        OpCode::BindIdent(idx) => {
            let i = builder.ins().iconst(types::I64, *idx as i64);
            call1(builder, ffi_refs, "oxy_bind_ident", i);
            *stack_depth -= 1;
            false
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
            false
        }

        OpCode::DisplayArg => {
            call_void(builder, ffi_refs, "oxy_display_arg");
            false
        }

        OpCode::MakeFuture {
            target_ip,
            arg_count,
        } => {
            let tgt = builder.ins().iconst(types::I64, *target_ip as i64);
            let ac = builder.ins().iconst(types::I64, *arg_count as i64);
            call2(builder, ffi_refs, "oxy_make_future", tgt, ac);
            *stack_depth = *stack_depth - arg_count + 1;
            false
        }
    }
}
