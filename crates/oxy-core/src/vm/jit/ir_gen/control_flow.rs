//! Control-flow lowering (if / match / loops / for) for `IrGen` — part of the AST → register IR
//! lowering pass. See `mod.rs` for the `IrGen` struct and state.

use super::*;

impl IrGen {
    pub(super) fn gen_if(
        &mut self,
        condition: &Expr,
        then_block: &Block,
        else_block: Option<&Expr>,
    ) -> Reg {
        let cond = self.gen_expr(condition);

        let then_id = self.alloc_block();

        if let Some(eb) = else_block {
            // If-else: full then / else / merge with phi.
            let else_id = self.alloc_block();
            let merge_id = self.alloc_block();

            self.terminate(Terminator::Branch {
                cond,
                then_block: then_id,
                else_block: else_id,
            });

            self.start_block(then_id);
            let then_reg = self.gen_block_stmts(then_block).unwrap_or_else(|| {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstUnit(r));
                r
            });
            if self.current.blocks[self.current_block]
                .terminator
                .is_default()
            {
                self.terminate(Terminator::Jump(merge_id));
            }

            self.start_block(else_id);
            let else_reg = self.gen_expr(eb);
            if self.current.blocks[self.current_block]
                .terminator
                .is_default()
            {
                self.terminate(Terminator::Jump(merge_id));
            }

            self.start_block(merge_id);
            let r = self.alloc_reg();
            self.emit(IrOp::Phi(r, then_reg, else_reg));
            // Store the phi result into a pseudo-local slot, then jump to a
            // continuation block. The Jump's phi_args machinery will push r
            // via push_reg, and the StoreLocal at the continuation site
            // will load it back. This keeps Phi stack ops isolated.
            let phi_temp = self.alloc_local("__phi_tmp");
            self.emit(IrOp::StoreLocal(phi_temp, r));
            let cont = self.alloc_block();
            self.terminate(Terminator::Jump(cont));
            self.start_block(cont);
            // Reload from temp slot into a new register so the caller sees
            // the value as defined in the continuation block (important for
            // nested control flow where the continuation jumps to an outer
            // merge block).
            let r2 = self.alloc_reg();
            self.emit(IrOp::LoadLocal(r2, phi_temp));
            r2
        } else {
            // If without else, used as an expression, yields the then-branch
            // value when the condition holds and unit otherwise (Oxy permits
            // `let x = if c { v };`). The else path produces unit; both merge
            // via Phi, mirroring the if-else case so the value propagates.
            let else_id = self.alloc_block();
            let merge_id = self.alloc_block();

            self.terminate(Terminator::Branch {
                cond,
                then_block: then_id,
                else_block: else_id,
            });

            self.start_block(then_id);
            let then_reg = self.gen_block_stmts(then_block).unwrap_or_else(|| {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstUnit(r));
                r
            });
            if self.current.blocks[self.current_block]
                .terminator
                .is_default()
            {
                self.terminate(Terminator::Jump(merge_id));
            }

            self.start_block(else_id);
            let else_reg = self.alloc_reg();
            self.emit(IrOp::ConstUnit(else_reg));
            self.terminate(Terminator::Jump(merge_id));

            self.start_block(merge_id);
            let r = self.alloc_reg();
            self.emit(IrOp::Phi(r, then_reg, else_reg));
            let phi_temp = self.alloc_local("__phi_tmp");
            self.emit(IrOp::StoreLocal(phi_temp, r));
            let cont = self.alloc_block();
            self.terminate(Terminator::Jump(cont));
            self.start_block(cont);
            let r2 = self.alloc_reg();
            self.emit(IrOp::LoadLocal(r2, phi_temp));
            r2
        }
    }

    pub(super) fn gen_if_let(
        &mut self,
        pattern: &Pattern,
        expr: &Expr,
        then_block: &Block,
        else_block: Option<&Expr>,
    ) -> Reg {
        let val = self.gen_expr(expr);
        let then_id = self.alloc_block();
        let else_id = self.alloc_block();
        let merge_id = self.alloc_block();

        let cond = self.gen_pattern_check(pattern, val);
        self.terminate(Terminator::Branch {
            cond,
            then_block: then_id,
            else_block: else_id,
        });

        self.start_block(then_id);
        self.gen_pattern_bind(pattern, val);
        let then_reg = self.gen_block_stmts(then_block).unwrap_or_else(|| {
            let r = self.alloc_reg();
            self.emit(IrOp::ConstUnit(r));
            r
        });
        if self.current.blocks[self.current_block]
            .terminator
            .is_default()
        {
            self.terminate(Terminator::Jump(merge_id));
        }

        self.start_block(else_id);
        let else_reg = else_block.map(|eb| self.gen_expr(eb)).unwrap_or_else(|| {
            let r = self.alloc_reg();
            self.emit(IrOp::ConstUnit(r));
            r
        });
        if self.current.blocks[self.current_block]
            .terminator
            .is_default()
        {
            self.terminate(Terminator::Jump(merge_id));
        }

        self.start_block(merge_id);
        let r = self.alloc_reg();
        self.emit(IrOp::Phi(r, then_reg, else_reg));
        let phi_temp = self.alloc_local("__phi_tmp");
        self.emit(IrOp::StoreLocal(phi_temp, r));
        let cont = self.alloc_block();
        self.terminate(Terminator::Jump(cont));
        self.start_block(cont);
        let r2 = self.alloc_reg();
        self.emit(IrOp::LoadLocal(r2, phi_temp));
        r2
    }

    pub(super) fn gen_if_let_guarded(
        &mut self,
        pattern: &Pattern,
        expr: &Expr,
        guard: &Expr,
        then_block: &Block,
        else_block: Option<&Expr>,
    ) -> Reg {
        let val = self.gen_expr(expr);
        let guard_check_id = self.alloc_block();
        let then_id = self.alloc_block();
        let else_id = self.alloc_block();
        let merge_id = self.alloc_block();

        // Entry: check pattern, branch to guard-check or else
        let matches = self.gen_pattern_check(pattern, val);
        self.terminate(Terminator::Branch {
            cond: matches,
            then_block: guard_check_id,
            else_block: else_id,
        });

        // Guard check: bind pattern (so guard can reference vars), evaluate guard
        self.start_block(guard_check_id);
        self.gen_pattern_bind(pattern, val);
        let guard_val = self.gen_expr(guard);
        self.terminate(Terminator::Branch {
            cond: guard_val,
            then_block: then_id,
            else_block: else_id,
        });

        // Then: bind pattern again (idempotent) and run body
        self.start_block(then_id);
        self.gen_pattern_bind(pattern, val);
        let then_reg = self.gen_block_stmts(then_block).unwrap_or_else(|| {
            let r = self.alloc_reg();
            self.emit(IrOp::ConstUnit(r));
            r
        });
        if self.current.blocks[self.current_block]
            .terminator
            .is_default()
        {
            self.terminate(Terminator::Jump(merge_id));
        }

        // Else
        self.start_block(else_id);
        let else_reg = else_block.map(|eb| self.gen_expr(eb)).unwrap_or_else(|| {
            let r = self.alloc_reg();
            self.emit(IrOp::ConstUnit(r));
            r
        });
        self.terminate(Terminator::Jump(merge_id));

        self.start_block(merge_id);
        let r = self.alloc_reg();
        self.emit(IrOp::Phi(r, then_reg, else_reg));
        let phi_temp = self.alloc_local("__phi_tmp");
        self.emit(IrOp::StoreLocal(phi_temp, r));
        let cont = self.alloc_block();
        self.terminate(Terminator::Jump(cont));
        self.start_block(cont);
        let r2 = self.alloc_reg();
        self.emit(IrOp::LoadLocal(r2, phi_temp));
        r2
    }

    pub(super) fn gen_match(&mut self, expr: &Expr, arms: &[MatchArm]) -> Reg {
        let val = self.gen_expr(expr);

        // Pre-allocate all check blocks and body blocks.
        // IMPORTANT: final_merge must be allocated AFTER body blocks so its block ID
        // is greater. Codegen processes blocks in ID order and the regs/reg_slot
        // maps are populated as blocks are visited. If final_merge were processed
        // before a body block, Return would see neither regs nor reg_slot.
        //
        // First check uses current block (no allocation), so we only need n-1 check blocks.
        let n = arms.len();
        let check_blocks: Vec<BlockId> = (1..n).map(|_| self.alloc_block()).collect();
        let body_blocks: Vec<BlockId> = (0..n).map(|_| self.alloc_block()).collect();
        let final_merge = self.alloc_block();
        let mut result_regs: Vec<Reg> = Vec::new();

        for (i, arm) in arms.iter().enumerate() {
            if i > 0 {
                self.start_block(check_blocks[i - 1]);
            }
            let matches = self.gen_pattern_check(&arm.pattern, val);
            let next_target = if i + 1 < n {
                check_blocks[i]
            } else {
                body_blocks[i]
            };
            if let Some(guard) = &arm.guard {
                // Pattern matches → evaluate guard before executing body.
                let guard_block = self.alloc_block();
                self.terminate(Terminator::Branch {
                    cond: matches,
                    then_block: guard_block,
                    else_block: next_target,
                });
                self.start_block(guard_block);
                self.gen_pattern_bind(&arm.pattern, val);
                let guard_val = self.gen_expr(guard);
                // Guard truthy → body; falsy → next arm.
                // Last arm's guard failure falls through to body (exhaustive catch-all).
                self.terminate(Terminator::Branch {
                    cond: guard_val,
                    then_block: body_blocks[i],
                    else_block: next_target,
                });
            } else {
                self.terminate(Terminator::Branch {
                    cond: matches,
                    then_block: body_blocks[i],
                    else_block: next_target,
                });
            }
        }

        // Generate arm body blocks. An arm body may contain control flow
        // (for/while/if/nested match), so after lowering it the "current" block
        // is the body's *exit* block — not body_blocks[i]. Record that exit
        // block: all merge wiring below must jump from there, otherwise it would
        // overwrite the branch into the loop (bypassing it) and read a result
        // register defined in an unreachable block.
        let mut body_end_blocks: Vec<BlockId> = Vec::new();
        for (i, arm) in arms.iter().enumerate() {
            self.start_block(body_blocks[i]);
            self.gen_pattern_bind(&arm.pattern, val);
            let reg = self.gen_expr(&arm.body);
            result_regs.push(reg);
            body_end_blocks.push(self.current_block);
        }

        if result_regs.is_empty() {
            self.start_block(final_merge);
            let r = self.alloc_reg();
            self.emit(IrOp::ConstUnit(r));
            return r;
        }

        if n == 1 {
            // Single arm — no Phi needed, return the result directly.
            self.current.block_mut(body_end_blocks[0]).terminator = Terminator::Jump(final_merge);
            self.start_block(final_merge);
            return result_regs[0];
        }

        if n == 2 {
            let merge_01 = self.alloc_block();
            self.current.block_mut(body_end_blocks[0]).terminator = Terminator::Jump(merge_01);
            self.current.block_mut(body_end_blocks[1]).terminator = Terminator::Jump(merge_01);
            self.start_block(merge_01);
            let r = self.alloc_reg();
            self.emit(IrOp::Phi(r, result_regs[0], result_regs[1]));
            let phi_temp = self.alloc_local("__phi_tmp");
            self.emit(IrOp::StoreLocal(phi_temp, r));
            let cont = self.alloc_block();
            self.terminate(Terminator::Jump(cont));
            self.start_block(cont);
            let r2 = self.alloc_reg();
            self.emit(IrOp::LoadLocal(r2, phi_temp));
            return r2;
        }

        // For 3+ arms: cascade intermediate merges, each combining two results.
        // body_0 → merge_01 ← body_1
        // merge_01 → merge_012 ← body_2
        // ... → final_merge ← body_{n-1}
        let mut prev_merge: Option<(BlockId, Reg)> = None;
        // Pair up results: body_0 + body_1 at merge_01, then cascade
        let mut idx = 0;
        while idx < n {
            if idx == 0 {
                // First pair: body_0, body_1 — wire from their exit blocks.
                let merge_01 = self.alloc_block();
                self.current.block_mut(body_end_blocks[0]).terminator = Terminator::Jump(merge_01);
                self.current.block_mut(body_end_blocks[1]).terminator = Terminator::Jump(merge_01);
                self.start_block(merge_01);
                let phi_r = self.alloc_reg();
                self.emit(IrOp::Phi(phi_r, result_regs[0], result_regs[1]));
                prev_merge = Some((merge_01, phi_r));
                idx = 2;
            } else {
                // Cascade: prev_merge + body_{idx}
                let (prev_block, prev_reg) = prev_merge.unwrap();
                let cascade_merge = if idx + 1 < n {
                    self.alloc_block()
                } else {
                    final_merge
                };
                // Patch prev block's terminator to jump to cascade_merge
                self.current.block_mut(prev_block).terminator = Terminator::Jump(cascade_merge);
                // Arm's exit block jumps to cascade_merge.
                self.current.block_mut(body_end_blocks[idx]).terminator =
                    Terminator::Jump(cascade_merge);
                self.start_block(cascade_merge);
                let phi_r = self.alloc_reg();
                self.emit(IrOp::Phi(phi_r, prev_reg, result_regs[idx]));
                prev_merge = Some((cascade_merge, phi_r));
                idx += 1;
            }
        }
        // prev_merge now holds the final merge block with the final phi result.
        // Store it in a temp, jump to continuation, and reload — isolates Phi
        // stack ops from user code and keeps reg_def_block correct for nesting.
        let (last_merge, phi_r) = prev_merge.unwrap();
        let phi_temp = self.alloc_local("__phi_tmp");
        self.current
            .block_mut(last_merge)
            .push(IrOp::StoreLocal(phi_temp, phi_r));
        let cont = self.alloc_block();
        self.current.block_mut(last_merge).terminator = Terminator::Jump(cont);
        self.start_block(cont);
        let r2 = self.alloc_reg();
        self.emit(IrOp::LoadLocal(r2, phi_temp));
        r2
    }

    pub(super) fn gen_while(
        &mut self,
        condition: &Expr,
        body: &Block,
        label: Option<&str>,
    ) -> Option<Reg> {
        let header_id = self.alloc_block();
        let body_id = self.alloc_block();
        let exit_id = self.alloc_block();

        // Register label if present
        if let Some(lbl) = label {
            self.labeled_targets
                .insert(lbl.to_string(), (exit_id, header_id));
        }

        // Jump to header
        self.terminate(Terminator::Jump(header_id));

        // Header: evaluate condition
        self.start_block(header_id);
        let cond = self.gen_expr(condition);
        self.terminate(Terminator::Branch {
            cond,
            then_block: body_id,
            else_block: exit_id,
        });

        // Body
        let saved_break = self.break_target;
        let saved_continue = self.continue_target;
        self.break_target = Some(exit_id);
        self.continue_target = Some(header_id);

        self.start_block(body_id);
        self.gen_block_stmts(body);
        // Don't overwrite Return/Panic/Halt from return/break inside body.
        if self.current.blocks[self.current_block]
            .terminator
            .is_default()
        {
            self.terminate(Terminator::Jump(header_id));
        }

        self.break_target = saved_break;
        self.continue_target = saved_continue;

        // Exit
        self.start_block(exit_id);
        None
    }

    pub(super) fn gen_while_let(
        &mut self,
        pattern: &Pattern,
        expr: &Expr,
        body: &Block,
        label: Option<&str>,
    ) -> Option<Reg> {
        let header_id = self.alloc_block();
        let body_id = self.alloc_block();
        let exit_id = self.alloc_block();

        if let Some(lbl) = label {
            self.labeled_targets
                .insert(lbl.to_string(), (exit_id, header_id));
        }

        self.terminate(Terminator::Jump(header_id));

        self.start_block(header_id);
        let val = self.gen_expr(expr);
        let cond = self.gen_pattern_check(pattern, val);
        self.terminate(Terminator::Branch {
            cond,
            then_block: body_id,
            else_block: exit_id,
        });

        let saved_break = self.break_target;
        let saved_continue = self.continue_target;
        self.break_target = Some(exit_id);
        self.continue_target = Some(header_id);

        self.start_block(body_id);
        self.gen_pattern_bind(pattern, val);
        self.gen_block_stmts(body);
        if self.current.blocks[self.current_block]
            .terminator
            .is_default()
        {
            self.terminate(Terminator::Jump(header_id));
        }

        self.break_target = saved_break;
        self.continue_target = saved_continue;

        self.start_block(exit_id);
        None
    }

    pub(super) fn gen_loop(&mut self, body: &Block, label: Option<&str>) -> Option<Reg> {
        let body_id = self.alloc_block();
        let exit_id = self.alloc_block();

        if let Some(lbl) = label {
            self.labeled_targets
                .insert(lbl.to_string(), (exit_id, body_id));
        }

        self.terminate(Terminator::Jump(body_id));

        let saved_break = self.break_target;
        let saved_continue = self.continue_target;
        let saved_break_slot = self.break_value_slot;

        // Allocate a temp slot to hold the break value
        let break_slot = self.alloc_local("__loop_break_val");
        self.break_value_slot = Some(break_slot);

        self.break_target = Some(exit_id);
        self.continue_target = Some(body_id);

        self.start_block(body_id);
        self.gen_block_stmts(body);
        if self.current.blocks[self.current_block]
            .terminator
            .is_default()
        {
            self.terminate(Terminator::Jump(body_id));
        }

        self.break_target = saved_break;
        self.continue_target = saved_continue;
        self.break_value_slot = saved_break_slot;

        // Exit block: load break value as the loop result
        self.start_block(exit_id);
        let r = self.alloc_reg();
        self.emit(IrOp::LoadLocal(r, break_slot));
        Some(r)
    }

    pub(super) fn gen_for_in(
        &mut self,
        name: &str,
        iterable: &Expr,
        body: &Block,
        label: Option<&str>,
    ) -> Option<Reg> {
        let iter_expr_reg = self.gen_expr(iterable);
        let var_slot = self.alloc_local(name);
        let state_slot = self.alloc_local("__iter_state");

        // Pre-allocate blocks so label can refer to them
        let header_id = self.alloc_block();
        let body_id = self.alloc_block();
        let exit_id = self.alloc_block();
        if let Some(lbl) = label {
            self.labeled_targets
                .insert(lbl.to_string(), (exit_id, header_id));
        }

        // Create iterator and store state to local slot
        let iter_tmp = self.alloc_reg();
        self.emit(IrOp::CallBuiltin {
            result: iter_tmp,
            func: "oxy_make_iter",
            args: vec![iter_expr_reg],
            immediates: vec![],
            strings: vec![],
        });
        self.emit(IrOp::StoreLocal(state_slot, iter_tmp));

        self.terminate(Terminator::Jump(header_id));

        // Header: call iter.next(). Returns i64 flag (1=has_next, 0=done)
        // and stores the raw element directly in var_slot.
        self.start_block(header_id);
        let has_next = self.alloc_reg();
        self.emit(IrOp::CallBuiltin {
            result: has_next,
            func: "oxy_iter_next",
            args: vec![],
            immediates: vec![state_slot, var_slot],
            strings: vec![],
        });
        self.terminate(Terminator::Branch {
            cond: has_next,
            then_block: body_id,
            else_block: exit_id,
        });

        let saved_break = self.break_target;
        let saved_continue = self.continue_target;
        self.break_target = Some(exit_id);
        self.continue_target = Some(header_id);

        self.start_block(body_id);
        self.gen_block_stmts(body);
        if self.current.blocks[self.current_block]
            .terminator
            .is_default()
        {
            self.terminate(Terminator::Jump(header_id));
        }

        self.break_target = saved_break;
        self.continue_target = saved_continue;

        self.start_block(exit_id);
        None
    }

    pub(super) fn gen_for_destructure(
        &mut self,
        names: &[String],
        iterable: &Expr,
        body: &Block,
        label: Option<&str>,
    ) -> Option<Reg> {
        let iter_expr_reg = self.gen_expr(iterable);
        // Allocate state slot first so oxy_iter_next_destructure writes
        // destructured fields to state_slot+1+i — matching the loop vars.
        let state_slot = self.alloc_local("__iter_state");
        for name in names {
            self.alloc_local(name);
        }

        let iter_tmp = self.alloc_reg();
        self.emit(IrOp::CallBuiltin {
            result: iter_tmp,
            func: "oxy_make_iter",
            args: vec![iter_expr_reg],
            immediates: vec![],
            strings: vec![],
        });
        self.emit(IrOp::StoreLocal(state_slot, iter_tmp));

        let header_id = self.alloc_block();
        let body_id = self.alloc_block();
        let exit_id = self.alloc_block();
        if let Some(lbl) = label {
            self.labeled_targets
                .insert(lbl.to_string(), (exit_id, header_id));
        }

        self.terminate(Terminator::Jump(header_id));
        self.start_block(header_id);
        let has_next = self.alloc_reg();
        self.emit(IrOp::CallBuiltin {
            result: has_next,
            func: "oxy_iter_next_destructure",
            args: vec![],
            immediates: vec![state_slot],
            strings: vec![],
        });
        self.terminate(Terminator::Branch {
            cond: has_next,
            then_block: body_id,
            else_block: exit_id,
        });

        let saved_break = self.break_target;
        let saved_continue = self.continue_target;
        self.break_target = Some(exit_id);
        self.continue_target = Some(header_id);

        self.start_block(body_id);
        self.gen_block_stmts(body);
        if self.current.blocks[self.current_block]
            .terminator
            .is_default()
        {
            self.terminate(Terminator::Jump(header_id));
        }

        self.break_target = saved_break;
        self.continue_target = saved_continue;

        self.start_block(exit_id);
        None
    }
}
