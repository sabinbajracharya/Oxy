//! Closure lowering and free-variable analysis for `IrGen` — part of the AST → register IR
//! lowering pass. See `mod.rs` for the `IrGen` struct and state.

use super::*;

impl IrGen {
    /// Collect free variable names in an expression (variables not in param_names).
    pub(super) fn collect_free_vars(
        &self,
        expr: &Expr,
        param_names: &std::collections::HashSet<String>,
    ) -> Vec<String> {
        let mut vars = std::collections::HashSet::new();
        self.collect_idents(expr, param_names, &mut vars);
        vars.into_iter().collect()
    }

    pub(super) fn collect_idents(
        &self,
        expr: &Expr,
        param_names: &std::collections::HashSet<String>,
        out: &mut std::collections::HashSet<String>,
    ) {
        match expr {
            Expr::Ident(name, ..) => {
                if !param_names.contains(name) && self.locals.contains_key(name) {
                    out.insert(name.clone());
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.collect_idents(left, param_names, out);
                self.collect_idents(right, param_names, out);
            }
            Expr::UnaryOp { expr, .. } => self.collect_idents(expr, param_names, out),
            Expr::Call { callee, args, .. } => {
                self.collect_idents(callee, param_names, out);
                for a in args {
                    self.collect_idents(a, param_names, out);
                }
            }
            Expr::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.collect_idents(condition, param_names, out);
                self.collect_idents_in_block(then_block, param_names, out);
                if let Some(eb) = else_block {
                    self.collect_idents(eb, param_names, out);
                }
            }
            Expr::Block(block) => self.collect_idents_in_block(block, param_names, out),
            Expr::MethodCall { object, args, .. } => {
                self.collect_idents(object, param_names, out);
                for a in args {
                    self.collect_idents(a, param_names, out);
                }
            }
            Expr::StructInit { fields, base, .. } => {
                for (_, v) in fields {
                    self.collect_idents(v, param_names, out);
                }
                if let Some(b) = base {
                    self.collect_idents(b, param_names, out);
                }
            }
            Expr::FieldAccess { object, .. } => self.collect_idents(object, param_names, out),
            Expr::Array { elements, .. } => {
                for e in elements {
                    self.collect_idents(e, param_names, out);
                }
            }
            Expr::Index { object, index, .. } => {
                self.collect_idents(object, param_names, out);
                self.collect_idents(index, param_names, out);
            }
            Expr::Assign { target, value, .. } => {
                self.collect_idents(target, param_names, out);
                self.collect_idents(value, param_names, out);
            }
            Expr::Closure { body, .. } => {
                self.collect_idents(body, param_names, out);
            }
            Expr::Grouped(e, ..) => self.collect_idents(e, param_names, out),
            Expr::Match { expr, arms, .. } => {
                self.collect_idents(expr, param_names, out);
                for arm in arms {
                    self.collect_idents(&arm.body, param_names, out);
                    if let Some(guard) = &arm.guard {
                        self.collect_idents(guard, param_names, out);
                    }
                }
            }
            Expr::IfLet {
                expr,
                guard,
                then_block,
                else_block,
                ..
            } => {
                self.collect_idents(expr, param_names, out);
                if let Some(g) = guard {
                    self.collect_idents(g, param_names, out);
                }
                self.collect_idents_in_block(then_block, param_names, out);
                if let Some(eb) = else_block {
                    self.collect_idents(eb, param_names, out);
                }
            }
            Expr::Tuple { elements, .. } => {
                for e in elements {
                    self.collect_idents(e, param_names, out);
                }
            }
            Expr::PathCall { args, .. } => {
                for a in args {
                    self.collect_idents(a, param_names, out);
                }
            }
            Expr::Range { start, end, .. } => {
                if let Some(s) = start {
                    self.collect_idents(s, param_names, out);
                }
                if let Some(e) = end {
                    self.collect_idents(e, param_names, out);
                }
            }
            Expr::Repeat { value, count, .. } => {
                self.collect_idents(value, param_names, out);
                self.collect_idents(count, param_names, out);
            }
            Expr::FString { parts, .. } => {
                for part in parts {
                    if let crate::ast::FStringPart::Expr(e) = part {
                        self.collect_idents(e, param_names, out);
                    }
                }
            }
            Expr::As { expr, .. } => self.collect_idents(expr, param_names, out),
            Expr::Try { expr, .. } => self.collect_idents(expr, param_names, out),
            Expr::Return { value, .. } => {
                if let Some(v) = value {
                    self.collect_idents(v, param_names, out);
                }
            }
            Expr::AsyncBlock { body, .. } => self.collect_idents_in_block(body, param_names, out),
            Expr::Await { expr, .. } => self.collect_idents(expr, param_names, out),
            Expr::MacroCall { args, .. } => {
                for a in args {
                    self.collect_idents(a, param_names, out);
                }
            }
            _ => {}
        }
    }

    pub(super) fn collect_idents_in_block(
        &self,
        block: &Block,
        param_names: &std::collections::HashSet<String>,
        out: &mut std::collections::HashSet<String>,
    ) {
        for stmt in &block.stmts {
            match stmt {
                Stmt::Expr { expr, .. } => self.collect_idents(expr, param_names, out),
                Stmt::Let { value, .. } => {
                    if let Some(v) = value {
                        self.collect_idents(v, param_names, out);
                    }
                }
                Stmt::Return { value, .. } => {
                    if let Some(v) = value {
                        self.collect_idents(v, param_names, out);
                    }
                }
                Stmt::While {
                    condition, body, ..
                } => {
                    self.collect_idents(condition, param_names, out);
                    self.collect_idents_in_block(body, param_names, out);
                }
                Stmt::For { iterable, body, .. } => {
                    self.collect_idents(iterable, param_names, out);
                    self.collect_idents_in_block(body, param_names, out);
                }
                Stmt::Item(_) | Stmt::Use(_) | Stmt::Break { .. } | Stmt::Continue { .. } => {}
                _ => {}
            }
        }
    }

    pub(super) fn gen_closure(
        &mut self,
        params: &[ClosureParam],
        body: &Expr,
        is_async: bool,
        emit_as_future: bool,
    ) -> Reg {
        // Find free variables (captures) by scanning the closure body for Idents
        // that reference outer-scope locals, not params.
        let param_names: std::collections::HashSet<String> =
            params.iter().map(|p| p.name.clone()).collect();
        let free_vars = self.collect_free_vars(body, &param_names);

        let meta_idx = self.closure_meta.len();
        let closure_name = format!("closure_{}", meta_idx);
        let saved = std::mem::replace(
            &mut self.current,
            IrFunction::new(closure_name.clone(), 0, 0, usize::MAX),
        );
        let saved_locals = std::mem::take(&mut self.locals);
        // The closure body has its own mutable-slot bookkeeping; the outer sets
        // (which the capture analysis below consults) are restored afterward.
        let saved_mut_slots = std::mem::take(&mut self.mut_slots);
        let saved_celled_slots = std::mem::take(&mut self.celled_slots);
        let saved_local_count = self.local_count;
        let saved_current_block = self.current_block;
        let saved_next_reg = self.next_reg;
        let saved_next_block = self.next_block;
        self.local_count = 0;
        self.next_reg = 0;
        self.next_block = 0;

        let entry = self.alloc_block();
        self.current.entry = entry;
        self.start_block(entry);

        // Record captures (name → outer slot)
        let mut captures = Vec::new();
        for name in &free_vars {
            if let Some(slot) = saved_locals.get(name) {
                captures.push((name.clone(), *slot));
            }
        }
        self.current.captures = captures;

        // Register closure metadata for runtime capture lookup.
        let param_names_for_meta: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
        let captures_with_mut: Vec<(String, usize, bool)> = self
            .current
            .captures
            .iter()
            .map(|(name, slot)| (name.clone(), *slot, saved_mut_slots.contains(slot)))
            .collect();
        self.closure_meta
            .push((param_names_for_meta, captures_with_mut, is_async));

        // Allocate locals for captures FIRST so the closure body can access
        // them via LoadLocal. Slot indices must match what jit_closure_invoker
        // writes to the buffer: captures at 0..captures_end, then params.
        let capture_names: Vec<String> = self
            .current
            .captures
            .iter()
            .map(|(n, _)| n.clone())
            .collect();
        for name in &capture_names {
            self.alloc_local(name);
        }
        // Allocate locals for params and record explicit metadata on IrFunction.
        for p in params {
            self.alloc_local(&p.name);
        }
        self.current.params = params
            .iter()
            .map(|p| {
                let ty = p
                    .type_ann
                    .as_ref()
                    .map(Self::type_ann_to_type_info)
                    .unwrap_or(TypeInfo::Unknown);
                (p.name.clone(), ty)
            })
            .collect();

        let result_reg = self.gen_expr(body);
        if !matches!(
            self.current.blocks[self.current_block].terminator,
            Terminator::Return(_)
        ) {
            self.terminate(Terminator::Return(result_reg));
        }

        // Infer return type from the closure body so codegen uses the right
        // return mechanism (oxy_set_result_i64 for int, push+oxy_return for
        // other types). Unknown triggers push_int which works for scalars.
        self.current.return_type = match body {
            Expr::IntLiteral(..) => crate::type_checker::TypeInfo::I64,
            Expr::FloatLiteral(..) => crate::type_checker::TypeInfo::F64,
            Expr::BoolLiteral(..) => crate::type_checker::TypeInfo::Bool,
            Expr::CharLiteral(..) => crate::type_checker::TypeInfo::Char,
            Expr::StringLiteral(..) => crate::type_checker::TypeInfo::UserStruct {
                name: "String".into(),
                generic_args: vec![],
            },
            _ => crate::type_checker::TypeInfo::Unknown,
        };

        self.current.local_count = self.local_count;
        self.current.is_async = is_async;
        self.current.fn_index = self.functions.len();
        // Snapshot (name, outer_slot) captures before swapping the closure out.
        let captures_snapshot = self.current.captures.clone();
        self.functions
            .push(std::mem::replace(&mut self.current, saved));

        self.locals = saved_locals;
        self.mut_slots = saved_mut_slots;
        self.celled_slots = saved_celled_slots;
        self.local_count = saved_local_count;
        self.current_block = saved_current_block;
        self.next_reg = saved_next_reg;
        self.next_block = saved_next_block;

        // Promote any captured mutable outer binding to a shared `Value::Cell`
        // before the closure is constructed, so the closure and the outer scope
        // observe the same storage. `oxy_make_cell` is idempotent per slot via
        // `celled_slots` (multiple closures may capture the same variable).
        for (_, outer_slot) in &captures_snapshot {
            if self.mut_slots.contains(outer_slot) && self.celled_slots.insert(*outer_slot) {
                self.emit(IrOp::MakeCell(*outer_slot));
            }
        }

        // Return a register referencing the closure.
        // The closure body is compiled as a separate IrFunction in self.functions.
        // fn_index resolved later by the resolve_fn_indices post-processing pass.
        let r = self.alloc_reg();
        self.emit(IrOp::CallBuiltin {
            result: r,
            func: if emit_as_future {
                "oxy_push_async_block"
            } else {
                "oxy_push_closure"
            },
            args: vec![],
            immediates: vec![meta_idx],
            strings: vec![closure_name],
        });
        r
    }
}
