//! Statement lowering for `IrGen` — part of the AST → register IR
//! lowering pass. See `mod.rs` for the `IrGen` struct and state.

use super::*;

impl IrGen {
    /// Walk a block's statements. Returns Some(reg) for the tail expression, None if no value.
    pub(super) fn gen_block_stmts(&mut self, block: &Block) -> Option<Reg> {
        let mut last: Option<Reg> = None;
        for stmt in &block.stmts {
            last = self.gen_stmt(stmt);
            // Stop if the current block was terminated (Return/Panic/Jump/Branch).
            // Remaining statements are unreachable — the type checker already
            // rejected them; this guard keeps the IR well-formed.
            if !self.current.blocks[self.current_block]
                .terminator
                .is_default()
            {
                break;
            }
        }
        last
    }

    pub(super) fn gen_stmt(&mut self, stmt: &Stmt) -> Option<Reg> {
        match stmt {
            Stmt::Let {
                name,
                mutable,
                value,
                type_ann,
                ..
            } => {
                // Evaluate the initializer BEFORE bringing the new binding into
                // scope, so a shadowing `let x = <expr using old x>` resolves `x`
                // in the initializer to the *previous* binding, not the new
                // (uninitialized) slot. Allocating the slot first made the RHS
                // read the slot it was about to define.
                let init = value.as_ref().map(|val| self.gen_expr(val));
                let slot = self.alloc_local(name);
                if *mutable {
                    // Track mutable bindings: a `let mut` captured by a closure
                    // gets promoted to a shared `Value::Cell` at closure creation
                    // so writes propagate across the capture boundary.
                    self.mut_slots.insert(slot);
                }
                if let Some(reg) = init {
                    let reg = if let Some(ta) = type_ann {
                        if let TypeAnnotation::Named { name, .. } = ta {
                            self.local_types.insert(slot, name.clone());
                        }
                        self.coerce_reg(reg, ta)
                    } else {
                        reg
                    };
                    self.emit(IrOp::StoreLocal(slot, reg));
                }
                None
            }
            Stmt::LetPattern { pattern, value, .. } => {
                let val_reg = self.gen_expr(value);
                self.gen_pattern_bind(pattern, val_reg);
                None
            }
            Stmt::Expr {
                expr,
                has_semicolon,
            } => {
                let reg = self.gen_expr(expr);
                if *has_semicolon {
                    None
                } else {
                    Some(reg)
                }
            }
            Stmt::Return { value, .. } => {
                let reg = match value {
                    Some(v) => self.gen_expr(v),
                    None => {
                        let r = self.alloc_reg();
                        self.emit(IrOp::ConstUnit(r));
                        r
                    }
                };
                let ret_ty = self.current.return_type.clone();
                let reg = self.coerce_reg_to_type_info(reg, &ret_ty);
                self.terminate(Terminator::Return(reg));
                None
            }
            Stmt::While {
                condition,
                body,
                label,
                ..
            } => self.gen_while(condition, body, label.as_deref()),
            Stmt::WhileLet {
                pattern,
                expr,
                body,
                label,
                ..
            } => self.gen_while_let(pattern, expr, body, label.as_deref()),
            Stmt::Loop { body, label, .. } => self.gen_loop(body, label.as_deref()),
            Stmt::For {
                name,
                iterable,
                body,
                label,
                ..
            } => self.gen_for_in(name, iterable, body, label.as_deref()),
            Stmt::ForDestructure {
                names,
                iterable,
                body,
                label,
                ..
            } => self.gen_for_destructure(names, iterable, body, label.as_deref()),
            Stmt::Break { label, value, .. } => {
                let reg = value.as_ref().map(|v| self.gen_expr(v)).unwrap_or_else(|| {
                    let r = self.alloc_reg();
                    self.emit(IrOp::ConstUnit(r));
                    r
                });
                if let Some(break_slot) = self.break_value_slot {
                    self.emit(IrOp::StoreLocal(break_slot, reg));
                }
                let target = if let Some(lbl) = label {
                    self.labeled_targets.get(lbl).map(|(b, _)| *b)
                } else {
                    self.break_target
                };
                if let Some(target) = target {
                    self.terminate(Terminator::Jump(target));
                }
                Some(reg)
            }
            Stmt::Continue { label, .. } => {
                let target = if let Some(lbl) = label {
                    self.labeled_targets.get(lbl).map(|(_, c)| *c)
                } else {
                    self.continue_target
                };
                if let Some(target) = target {
                    self.terminate(Terminator::Jump(target));
                }
                None
            }
            Stmt::Use(use_def) => {
                self.register_use(use_def);
                None
            }
            Stmt::Item(_) => None,
        }
    }

    // ── Expressions ────────────────────────────────────────────────────

    /// Lower an assignment `target = <val_reg>`, propagating the result back
    /// through the lvalue chain.
    ///
    /// Structs are value types (fields are cloned on copy), so `oxy_field_store`
    /// produces a *new* struct rather than mutating in place — that new struct
    /// must be written back into the binding (or the enclosing field/index) or
    /// the mutation is lost. We recurse so `a.b.c = v` rebuilds each level:
    /// `a = store(a, "b", store(a.b, "c", v))`.
    ///
    /// `Vec` (and the other collections) are `Rc<RefCell<>>`-shared, so an
    /// index store mutates the backing storage in place — its enclosing binding
    /// already observes the change, so no further write-back is needed there.
    pub(super) fn gen_store_lvalue(&mut self, target: &Expr, val_reg: Reg) {
        match target {
            Expr::Ident(name, ..) => {
                if let Some(slot) = self.lookup_local(name) {
                    self.emit(IrOp::StoreLocal(slot, val_reg));
                }
            }
            // `self` is always local slot 0 (mirrors the read path in gen_expr).
            // Required so `self.field = v` in a `mut self` method writes the
            // updated struct back, not just the discarded field-store result.
            Expr::SelfRef(..) => {
                self.emit(IrOp::StoreLocal(0, val_reg));
            }
            Expr::Grouped(inner, ..) => self.gen_store_lvalue(inner, val_reg),
            Expr::FieldAccess { object, field, .. } => {
                let obj_reg = self.gen_expr(object);
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_field_store",
                    args: vec![obj_reg, val_reg],
                    immediates: vec![],
                    strings: vec![field.clone()],
                });
                // Write the updated (value-typed) struct back into its container.
                self.gen_store_lvalue(object, r);
            }
            Expr::Index { object, index, .. } => {
                let obj_reg = self.gen_expr(object);
                let idx_reg = self.gen_expr(index);
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_vec_index_store",
                    args: vec![obj_reg, idx_reg, val_reg],
                    immediates: vec![],
                    strings: vec![],
                });
                // Collection storage is Rc-shared and mutated in place above, so
                // the enclosing binding already sees the change — no write-back.
            }
            _ => {}
        }
    }
}
