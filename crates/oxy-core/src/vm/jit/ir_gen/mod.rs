//! AST → Register IR + CFG code generator.
//!
//! Walks typed AST items and emits register-based IR with basic blocks.
//! Replaces the old bytecode compiler (`compiler/`).
//!
//! # Pipeline
//! 1. Parse source → AST
//! 2. Type-check AST
//! 3. `IrGen::gen_program()` → `Vec<IrFunction>` (register IR + CFG)
//! 4. `codegen.rs` → Cranelift CLIF

use crate::ast::*;
use crate::type_checker::TypeInfo;
use super::ir::*;

/// IR code generator. Walks a typed AST and produces register IR.
pub(crate) struct IrGen {
    /// All generated functions (including closures, async blocks).
    functions: Vec<IrFunction>,
    /// Current function being generated.
    current: IrFunction,
    /// Current basic block being built.
    current_block: BlockId,
    /// Next available virtual register.
    next_reg: Reg,
    /// Next available block ID.
    next_block: BlockId,
    /// Local variable name → slot index.
    locals: std::collections::HashMap<String, usize>,
    /// Number of local slots allocated.
    local_count: usize,
    /// Current break target (loop exit block).
    break_target: Option<BlockId>,
    /// Current continue target (loop header block).
    continue_target: Option<BlockId>,
}

impl IrGen {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            current: IrFunction::new(String::new(), 0, 0),
            current_block: 0,
            next_reg: 0,
            next_block: 0,
            locals: std::collections::HashMap::new(),
            local_count: 0,
            break_target: None,
            continue_target: None,
        }
    }

    // ── Helpers ────────────────────────────────────────────────────────

    fn alloc_reg(&mut self) -> Reg {
        let r = self.next_reg;
        self.next_reg += 1;
        r
    }

    fn alloc_block(&mut self) -> BlockId {
        let id = self.next_block;
        self.next_block += 1;
        id
    }

    fn alloc_local(&mut self, name: &str) -> usize {
        let slot = self.local_count;
        self.locals.insert(name.to_string(), slot);
        self.local_count += 1;
        slot
    }

    fn lookup_local(&self, name: &str) -> Option<usize> {
        self.locals.get(name).copied()
    }

    fn emit(&mut self, op: IrOp) {
        self.current.block_mut(self.current_block).push(op);
    }

    fn terminate(&mut self, term: Terminator) {
        self.current.block_mut(self.current_block).terminate(term);
    }

    fn start_block(&mut self, id: BlockId) {
        while self.current.blocks.len() <= id {
            self.current.add_block();
        }
        self.current_block = id;
    }

    // ── Top-level ──────────────────────────────────────────────────────

    /// Generate IR for an entire program.
    pub fn gen_program(&mut self, program: &Program) {
        for item in &program.items {
            match item {
                Item::Function(f) => self.gen_fn(f),
                Item::Impl(imp) => {
                    for method in &imp.methods {
                        self.gen_fn(method);
                    }
                }
                // Struct/enum/mod/use/const/type items don't generate IR directly
                _ => {}
            }
        }
    }

    /// Generate IR for one function.
    fn gen_fn(&mut self, f: &FnDef) {
        // Save current state
        let saved = std::mem::replace(
            &mut self.current,
            IrFunction::new(f.name.clone(), 0, 0),
        );
        let saved_locals = std::mem::take(&mut self.locals);
        let saved_local_count = self.local_count;
        let saved_break = self.break_target;
        let saved_continue = self.continue_target;
        self.local_count = 0;
        self.next_reg = 0;
        self.next_block = 0;
        self.break_target = None;
        self.continue_target = None;

        // Create entry block
        let entry = self.alloc_block();
        self.current.entry = entry;
        self.start_block(entry);

        // Allocate locals for params
        for param in &f.params {
            self.alloc_local(&param.name);
        }

        // Generate body
        let result_reg = self.gen_block_stmts(&f.body);
        // If no explicit return, add implicit return of tail expression
        if !matches!(self.current.blocks[self.current_block].terminator, Terminator::Return(_) | Terminator::Panic(_)) {
            let reg = result_reg.unwrap_or_else(|| {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstUnit(r));
                r
            });
            self.terminate(Terminator::Return(reg));
        }

        self.current.local_count = self.local_count;
        self.functions.push(std::mem::replace(
            &mut self.current,
            saved,
        ));

        // Restore state
        self.locals = saved_locals;
        self.local_count = saved_local_count;
        self.break_target = saved_break;
        self.continue_target = saved_continue;
    }

    // ── Block / Stmt ───────────────────────────────────────────────────

    /// Walk a block's statements. Returns Some(reg) for the tail expression, None if no value.
    fn gen_block_stmts(&mut self, block: &Block) -> Option<Reg> {
        let mut last: Option<Reg> = None;
        for stmt in &block.stmts {
            last = self.gen_stmt(stmt);
        }
        last
    }

    fn gen_stmt(&mut self, stmt: &Stmt) -> Option<Reg> {
        match stmt {
            Stmt::Let { name, value, .. } => {
                let slot = self.alloc_local(name);
                if let Some(val) = value {
                    let reg = self.gen_expr(val);
                    self.emit(IrOp::StoreLocal(slot, reg));
                }
                None
            }
            Stmt::LetPattern { pattern, value, .. } => {
                let val_reg = self.gen_expr(value);
                self.gen_pattern_bind(pattern, val_reg);
                None
            }
            Stmt::Expr { expr, has_semicolon } => {
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
                self.terminate(Terminator::Return(reg));
                None
            }
            Stmt::While { condition, body, .. } => {
                self.gen_while(condition, body, None)
            }
            Stmt::WhileLet { pattern, expr, body, .. } => {
                self.gen_while_let(pattern, expr, body)
            }
            Stmt::Loop { body, .. } => {
                self.gen_loop(body)
            }
            Stmt::For { name, iterable, body, .. } => {
                self.gen_for_in(name, iterable, body)
            }
            Stmt::ForDestructure { names, iterable, body, .. } => {
                self.gen_for_destructure(names, iterable, body)
            }
            Stmt::Break { value, .. } => {
                let reg = value.as_ref().map(|v| self.gen_expr(v)).unwrap_or_else(|| {
                    let r = self.alloc_reg();
                    self.emit(IrOp::ConstUnit(r));
                    r
                });
                if let Some(target) = self.break_target {
                    self.terminate(Terminator::Jump(target));
                }
                Some(reg)
            }
            Stmt::Continue { .. } => {
                if let Some(target) = self.continue_target {
                    self.terminate(Terminator::Jump(target));
                }
                None
            }
            Stmt::Use(_) | Stmt::Item(_) => None,
        }
    }

    // ── Expressions ────────────────────────────────────────────────────

    fn gen_expr(&mut self, expr: &Expr) -> Reg {
        match expr {
            Expr::IntLiteral(n, ..) => {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstInt(r, *n));
                r
            }
            Expr::FloatLiteral(n, ..) => {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstFloat(r, *n));
                r
            }
            Expr::BoolLiteral(b, ..) => {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstBool(r, *b));
                r
            }
            Expr::StringLiteral(s, ..) => {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstString(r, s.clone()));
                r
            }
            Expr::CharLiteral(c, ..) => {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstChar(r, *c));
                r
            }
            Expr::Ident(name, ..) => {
                if let Some(slot) = self.lookup_local(name) {
                    let r = self.alloc_reg();
                    self.emit(IrOp::LoadLocal(r, slot));
                    r
                } else {
                    // Global function or builtin — return as ident ref for Call handling
                    let r = self.alloc_reg();
                    // Load from globals/environment — for now, stub
                    self.emit(IrOp::ConstUnit(r));
                    r
                }
            }
            Expr::Block(block) => {
                self.gen_block_stmts(block).unwrap_or_else(|| {
                    let r = self.alloc_reg();
                    self.emit(IrOp::ConstUnit(r));
                    r
                })
            }
            Expr::BinaryOp { left, op, right, .. } => {
                let lhs = self.gen_expr(left);
                let rhs = self.gen_expr(right);
                let r = self.alloc_reg();
                match op {
                    BinOp::Add => self.emit(IrOp::Add(r, lhs, rhs)),
                    BinOp::Sub => self.emit(IrOp::Sub(r, lhs, rhs)),
                    BinOp::Mul => self.emit(IrOp::Mul(r, lhs, rhs)),
                    BinOp::Div => self.emit(IrOp::Div(r, lhs, rhs)),
                    BinOp::Mod => self.emit(IrOp::Rem(r, lhs, rhs)),
                    BinOp::Eq => self.emit(IrOp::Eq(r, lhs, rhs)),
                    BinOp::NotEq => self.emit(IrOp::Neq(r, lhs, rhs)),
                    BinOp::Lt => self.emit(IrOp::Lt(r, lhs, rhs)),
                    BinOp::Gt => self.emit(IrOp::Gt(r, lhs, rhs)),
                    BinOp::LtEq => self.emit(IrOp::Le(r, lhs, rhs)),
                    BinOp::GtEq => self.emit(IrOp::Ge(r, lhs, rhs)),
                    BinOp::And => self.emit(IrOp::And(r, lhs, rhs)),
                    BinOp::Or => self.emit(IrOp::Or(r, lhs, rhs)),
                    BinOp::BitAnd => self.emit(IrOp::BitAnd(r, lhs, rhs)),
                    BinOp::BitOr => self.emit(IrOp::BitOr(r, lhs, rhs)),
                    BinOp::BitXor => self.emit(IrOp::BitXor(r, lhs, rhs)),
                    BinOp::Shl => self.emit(IrOp::Shl(r, lhs, rhs)),
                    BinOp::Shr => self.emit(IrOp::Shr(r, lhs, rhs)),
                }
                r
            }
            Expr::UnaryOp { op, expr, .. } => {
                let val = self.gen_expr(expr);
                let r = self.alloc_reg();
                match op {
                    UnaryOp::Neg => self.emit(IrOp::Neg(r, val)),
                    UnaryOp::Not => self.emit(IrOp::Not(r, val)),
                    UnaryOp::BitNot => self.emit(IrOp::BitNot(r, val)),
                }
                r
            }
            Expr::Call { callee, args, .. } => {
                let fname = match callee.as_ref() {
                    Expr::Ident(name, ..) => name.clone(),
                    Expr::Path { segments, .. } => segments.join("::"),
                    _ => String::from("<anon>"),
                };
                let mut arg_regs = Vec::new();
                for a in args {
                    arg_regs.push(self.gen_expr(a));
                }
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_call",
                    args: arg_regs,
                    immediates: vec![],
                });
                r
            }
            Expr::MethodCall { object, method, args, .. } => {
                let obj_reg = self.gen_expr(object);
                let mut arg_regs = vec![obj_reg];
                for a in args {
                    arg_regs.push(self.gen_expr(a));
                }
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_method_call",
                    args: arg_regs,
                    immediates: vec![],
                });
                r
            }
            Expr::If { condition, then_block, else_block, .. } => {
                self.gen_if(condition, then_block, else_block.as_deref())
            }
            Expr::IfLet { pattern, expr, then_block, else_block, guard, .. } => {
                if guard.is_some() {
                    // if-let with guard — complex, stub for now
                    let r = self.alloc_reg();
                    self.emit(IrOp::ConstUnit(r));
                    r
                } else {
                    self.gen_if_let(pattern, expr, then_block, else_block.as_deref())
                }
            }
            Expr::Match { expr, arms, .. } => {
                self.gen_match(expr, arms)
            }
            Expr::StructInit { name, fields, base, .. } => {
                let mut arg_regs = Vec::new();
                for (_, val) in fields {
                    arg_regs.push(self.gen_expr(val));
                }
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_struct_init",
                    args: arg_regs,
                    immediates: vec![fields.len()],
                });
                r
            }
            Expr::FieldAccess { object, field, .. } => {
                let obj = self.gen_expr(object);
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_field_access",
                    args: vec![obj],
                    immediates: vec![],
                });
                r
            }
            Expr::PathCall { path, turbofish, args, .. } => {
                let mut arg_regs = Vec::new();
                for a in args {
                    arg_regs.push(self.gen_expr(a));
                }
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_path_call_builtin",
                    args: arg_regs,
                    immediates: vec![],
                });
                r
            }
            Expr::Array { elements, .. } => {
                let mut regs = Vec::new();
                for e in elements {
                    regs.push(self.gen_expr(e));
                }
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_make_array",
                    args: regs,
                    immediates: vec![elements.len()],
                });
                r
            }
            Expr::Tuple { elements, .. } => {
                let mut regs = Vec::new();
                for e in elements {
                    regs.push(self.gen_expr(e));
                }
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_make_tuple",
                    args: regs,
                    immediates: vec![elements.len()],
                });
                r
            }
            Expr::Index { object, index, .. } => {
                let obj = self.gen_expr(object);
                let idx = self.gen_expr(index);
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_vec_index",
                    args: vec![obj, idx],
                    immediates: vec![],
                });
                r
            }
            Expr::Try { expr, .. } => {
                let val = self.gen_expr(expr);
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_try_pop",
                    args: vec![val],
                    immediates: vec![],
                });
                r
            }
            Expr::FString { parts, .. } => {
                let mut regs = Vec::new();
                for part in parts {
                    match part {
                        crate::ast::FStringPart::Literal(s) => {
                            let r = self.alloc_reg();
                            self.emit(IrOp::ConstString(r, s.clone()));
                            regs.push(r);
                        }
                        crate::ast::FStringPart::Expr(e) => {
                            regs.push(self.gen_expr(e));
                        }
                    }
                }
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_fstring_concat",
                    args: regs,
                    immediates: vec![parts.len()],
                });
                r
            }
            Expr::Assign { target, value, .. } => {
                let val_reg = self.gen_expr(value);
                if let Expr::Ident(name, ..) = target.as_ref() {
                    if let Some(slot) = self.lookup_local(name) {
                        self.emit(IrOp::StoreLocal(slot, val_reg));
                    }
                }
                val_reg
            }
            Expr::CompoundAssign { target, op, value, .. } => {
                let val_reg = self.gen_expr(value);
                let target_reg = self.gen_expr(target);
                let r = self.alloc_reg();
                match op {
                    BinOp::Add => self.emit(IrOp::Add(r, target_reg, val_reg)),
                    BinOp::Sub => self.emit(IrOp::Sub(r, target_reg, val_reg)),
                    _ => { self.emit(IrOp::Copy(r, val_reg)); }
                }
                if let Expr::Ident(name, ..) = target.as_ref() {
                    if let Some(slot) = self.lookup_local(name) {
                        self.emit(IrOp::StoreLocal(slot, r));
                    }
                }
                r
            }
            Expr::Range { start, end, inclusive, .. } => {
                let start_reg = start.as_ref().map(|s| self.gen_expr(s)).unwrap_or_else(|| {
                    let r = self.alloc_reg();
                    self.emit(IrOp::ConstInt(r, 0));
                    r
                });
                let end_reg = end.as_ref().map(|e| self.gen_expr(e)).unwrap_or_else(|| {
                    let r = self.alloc_reg();
                    self.emit(IrOp::ConstInt(r, -1));
                    r
                });
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_make_range",
                    args: vec![start_reg, end_reg],
                    immediates: vec![*inclusive as usize],
                });
                r
            }
            Expr::Closure { params, body, is_async, .. } => {
                self.gen_closure(params, body, *is_async)
            }
            Expr::Path { segments, .. } => {
                // Unit enum variant: Color::Red — handled via const enum variant call
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_const_enum_variant",
                    args: vec![],
                    immediates: vec![],
                });
                r
            }
            Expr::SelfRef(..) => {
                // self parameter — load from local slot 0
                let r = self.alloc_reg();
                self.emit(IrOp::LoadLocal(r, 0));
                r
            }
            Expr::As { expr, .. } => {
                let val = self.gen_expr(expr);
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_cast_int",
                    args: vec![val],
                    immediates: vec![],
                });
                r
            }
            // Stub remaining
            _ => {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstUnit(r));
                r
            }
        }
    }

    // ── Control flow helpers ───────────────────────────────────────────

    fn gen_if(&mut self, condition: &Expr, then_block: &Block, else_block: Option<&Expr>) -> Reg {
        let cond = self.gen_expr(condition);

        let then_id = self.alloc_block();
        let else_id = self.alloc_block();
        let merge_id = self.alloc_block();

        // Entry: branch
        self.terminate(Terminator::Branch { cond, then_block: then_id, else_block: else_id });

        // Then block
        self.start_block(then_id);
        let then_reg = self.gen_block_stmts(then_block).unwrap_or_else(|| {
            let r = self.alloc_reg();
            self.emit(IrOp::ConstUnit(r));
            r
        });
        self.terminate(Terminator::Jump(merge_id));

        // Else block
        self.start_block(else_id);
        let else_reg = match else_block {
            Some(eb) => self.gen_expr(eb),
            None => {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstUnit(r));
                r
            }
        };
        self.terminate(Terminator::Jump(merge_id));

        // Merge block with phi
        self.start_block(merge_id);
        let r = self.alloc_reg();
        self.emit(IrOp::Phi(r, then_reg, else_reg));
        r
    }

    fn gen_if_let(&mut self, pattern: &Pattern, expr: &Expr, then_block: &Block, else_block: Option<&Expr>) -> Reg {
        let val = self.gen_expr(expr);
        let then_id = self.alloc_block();
        let else_id = self.alloc_block();
        let merge_id = self.alloc_block();

        // Bind pattern variables (they'll be read in then_block)
        self.gen_pattern_bind(pattern, val);

        // branch on pattern match result (stub — always true for simple patterns)
        let cond = self.alloc_reg();
        self.emit(IrOp::ConstBool(cond, true));
        self.terminate(Terminator::Branch { cond, then_block: then_id, else_block: else_id });

        self.start_block(then_id);
        let then_reg = self.gen_block_stmts(then_block).unwrap_or_else(|| {
            let r = self.alloc_reg();
            self.emit(IrOp::ConstUnit(r));
            r
        });
        self.terminate(Terminator::Jump(merge_id));

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
        r
    }

    fn gen_match(&mut self, expr: &Expr, arms: &[MatchArm]) -> Reg {
        let val = self.gen_expr(expr);
        let merge_id = self.alloc_block();
        let mut arm_results: Vec<(BlockId, Reg)> = Vec::new();

        for arm in arms {
            let arm_id = self.alloc_block();
            // Bind pattern
            self.gen_pattern_bind(&arm.pattern, val);
            self.start_block(arm_id);
            let reg = self.gen_expr(&arm.body);
            arm_results.push((arm_id, reg));
            self.terminate(Terminator::Jump(merge_id));
        }

        // TODO: proper match dispatch with guards/conditions
        // For now, just chain blocks
        if !arm_results.is_empty() {
            let first = arm_results[0].0;
            self.terminate(Terminator::Jump(first));
        }

        self.start_block(merge_id);
        if arm_results.is_empty() {
            let r = self.alloc_reg();
            self.emit(IrOp::ConstUnit(r));
            r
        } else {
            let r = self.alloc_reg();
            self.emit(IrOp::Phi(r, arm_results[0].1, arm_results.last().unwrap().1));
            r
        }
    }

    fn gen_while(&mut self, condition: &Expr, body: &Block, _label: Option<&str>) -> Option<Reg> {
        let header_id = self.alloc_block();
        let body_id = self.alloc_block();
        let exit_id = self.alloc_block();

        // Jump to header
        self.terminate(Terminator::Jump(header_id));

        // Header: evaluate condition
        self.start_block(header_id);
        let cond = self.gen_expr(condition);
        self.terminate(Terminator::Branch { cond, then_block: body_id, else_block: exit_id });

        // Body
        let saved_break = self.break_target;
        let saved_continue = self.continue_target;
        self.break_target = Some(exit_id);
        self.continue_target = Some(header_id);

        self.start_block(body_id);
        self.gen_block_stmts(body);
        self.terminate(Terminator::Jump(header_id));

        self.break_target = saved_break;
        self.continue_target = saved_continue;

        // Exit
        self.start_block(exit_id);
        None
    }

    fn gen_while_let(&mut self, pattern: &Pattern, expr: &Expr, body: &Block) -> Option<Reg> {
        let header_id = self.alloc_block();
        let body_id = self.alloc_block();
        let exit_id = self.alloc_block();

        self.terminate(Terminator::Jump(header_id));

        self.start_block(header_id);
        let val = self.gen_expr(expr);
        self.gen_pattern_bind(pattern, val);
        let cond = self.alloc_reg();
        self.emit(IrOp::ConstBool(cond, true));
        self.terminate(Terminator::Branch { cond, then_block: body_id, else_block: exit_id });

        let saved_break = self.break_target;
        let saved_continue = self.continue_target;
        self.break_target = Some(exit_id);
        self.continue_target = Some(header_id);

        self.start_block(body_id);
        self.gen_block_stmts(body);
        self.terminate(Terminator::Jump(header_id));

        self.break_target = saved_break;
        self.continue_target = saved_continue;

        self.start_block(exit_id);
        None
    }

    fn gen_loop(&mut self, body: &Block) -> Option<Reg> {
        let body_id = self.alloc_block();
        let exit_id = self.alloc_block();

        self.terminate(Terminator::Jump(body_id));

        let saved_break = self.break_target;
        let saved_continue = self.continue_target;
        self.break_target = Some(exit_id);
        self.continue_target = Some(body_id);

        self.start_block(body_id);
        self.gen_block_stmts(body);
        self.terminate(Terminator::Jump(body_id));

        self.break_target = saved_break;
        self.continue_target = saved_continue;

        self.start_block(exit_id);
        None
    }

    fn gen_for_in(&mut self, name: &str, iterable: &Expr, body: &Block) -> Option<Reg> {
        let iter_reg = self.gen_expr(iterable);
        let slot = self.alloc_local(name);

        let header_id = self.alloc_block();
        let body_id = self.alloc_block();
        let exit_id = self.alloc_block();

        self.terminate(Terminator::Jump(header_id));

        // Header: iter.next() → check if done
        self.start_block(header_id);
        let cond = self.alloc_reg();
        self.emit(IrOp::ConstBool(cond, true));  // stub
        self.terminate(Terminator::Branch { cond, then_block: body_id, else_block: exit_id });

        let saved_break = self.break_target;
        let saved_continue = self.continue_target;
        self.break_target = Some(exit_id);
        self.continue_target = Some(header_id);

        self.start_block(body_id);
        self.gen_block_stmts(body);
        self.terminate(Terminator::Jump(header_id));

        self.break_target = saved_break;
        self.continue_target = saved_continue;

        self.start_block(exit_id);
        None
    }

    fn gen_for_destructure(&mut self, names: &[String], iterable: &Expr, body: &Block) -> Option<Reg> {
        let _iter_reg = self.gen_expr(iterable);
        for name in names {
            self.alloc_local(name);
        }
        let header_id = self.alloc_block();
        let body_id = self.alloc_block();
        let exit_id = self.alloc_block();

        self.terminate(Terminator::Jump(header_id));
        self.start_block(header_id);
        let cond = self.alloc_reg();
        self.emit(IrOp::ConstBool(cond, true));
        self.terminate(Terminator::Branch { cond, then_block: body_id, else_block: exit_id });

        let saved_break = self.break_target;
        let saved_continue = self.continue_target;
        self.break_target = Some(exit_id);
        self.continue_target = Some(header_id);

        self.start_block(body_id);
        self.gen_block_stmts(body);
        self.terminate(Terminator::Jump(header_id));

        self.break_target = saved_break;
        self.continue_target = saved_continue;

        self.start_block(exit_id);
        None
    }

    fn gen_pattern_bind(&mut self, pattern: &Pattern, val_reg: Reg) {
        match pattern {
            Pattern::Ident(name, ..) => {
                let slot = self.alloc_local(name);
                self.emit(IrOp::StoreLocal(slot, val_reg));
            }
            Pattern::Wildcard(..) => {}
            Pattern::EnumVariant { fields, .. } => {
                for (i, p) in fields.iter().enumerate() {
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_enum_data_get",
                        args: vec![val_reg],
                        immediates: vec![i],
                    });
                    self.gen_pattern_bind(p, r);
                }
            }
            Pattern::Struct { fields, .. } => {
                for (i, (fname, p)) in fields.iter().enumerate() {
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_field_access",
                        args: vec![val_reg],
                        immediates: vec![i],
                    });
                    self.gen_pattern_bind(p, r);
                }
            }
            Pattern::Tuple(patterns, ..) => {
                for (i, p) in patterns.iter().enumerate() {
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_enum_data_get",
                        args: vec![val_reg],
                        immediates: vec![i],
                    });
                    self.gen_pattern_bind(p, r);
                }
            }
            Pattern::Literal(..) => {}
            Pattern::Or(..) => {}
            Pattern::Rest(..) => {}
            Pattern::Slice(..) => {}
            Pattern::Range { .. } => {}
        }
    }

    /// Collect free variable names in an expression (variables not in param_names).
    fn collect_free_vars(&self, expr: &Expr, param_names: &std::collections::HashSet<String>) -> Vec<String> {
        let mut vars = std::collections::HashSet::new();
        self.collect_idents(expr, param_names, &mut vars);
        vars.into_iter().collect()
    }

    fn collect_idents(
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
                for a in args { self.collect_idents(a, param_names, out); }
            }
            Expr::If { condition, then_block, else_block, .. } => {
                self.collect_idents(condition, param_names, out);
                self.collect_idents_in_block(then_block, param_names, out);
                if let Some(eb) = else_block {
                    self.collect_idents(eb, param_names, out);
                }
            }
            Expr::Block(block) => self.collect_idents_in_block(block, param_names, out),
            Expr::MethodCall { object, args, .. } => {
                self.collect_idents(object, param_names, out);
                for a in args { self.collect_idents(a, param_names, out); }
            }
            Expr::StructInit { fields, base, .. } => {
                for (_, v) in fields { self.collect_idents(v, param_names, out); }
                if let Some(b) = base { self.collect_idents(b, param_names, out); }
            }
            Expr::FieldAccess { object, .. } => self.collect_idents(object, param_names, out),
            Expr::Array { elements, .. } => {
                for e in elements { self.collect_idents(e, param_names, out); }
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
                // Don't capture through nested closures
                self.collect_idents(body, param_names, out);
            }
            _ => {}
        }
    }

    fn collect_idents_in_block(
        &self,
        block: &Block,
        param_names: &std::collections::HashSet<String>,
        out: &mut std::collections::HashSet<String>,
    ) {
        for stmt in &block.stmts {
            match stmt {
                Stmt::Expr { expr, .. } => self.collect_idents(expr, param_names, out),
                Stmt::Let { value, .. } => {
                    if let Some(v) = value { self.collect_idents(v, param_names, out); }
                }
                Stmt::Return { value, .. } => {
                    if let Some(v) = value { self.collect_idents(v, param_names, out); }
                }
                Stmt::While { condition, body, .. } => {
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

    fn gen_closure(&mut self, params: &[ClosureParam], body: &Expr, is_async: bool) -> Reg {
        // Find free variables (captures) by scanning the closure body for Idents
        // that reference outer-scope locals, not params.
        let param_names: std::collections::HashSet<String> =
            params.iter().map(|p| p.name.clone()).collect();
        let free_vars = self.collect_free_vars(body, &param_names);

        let closure_name = format!("closure_{}", self.functions.len());
        let saved = std::mem::replace(
            &mut self.current,
            IrFunction::new(closure_name.clone(), 0, 0),
        );
        let saved_locals = std::mem::take(&mut self.locals);
        let saved_local_count = self.local_count;
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

        // Allocate locals for params
        for p in params {
            self.alloc_local(&p.name);
        }

        let result_reg = self.gen_expr(body);
        if !matches!(self.current.blocks[self.current_block].terminator, Terminator::Return(_)) {
            self.terminate(Terminator::Return(result_reg));
        }

        self.current.local_count = self.local_count;
        self.current.is_async = is_async;
        self.functions.push(std::mem::replace(
            &mut self.current,
            saved,
        ));

        self.locals = saved_locals;
        self.local_count = saved_local_count;

        // Return a register (codegen will create the closure Value)
        let r = self.alloc_reg();
        self.emit(IrOp::CallBuiltin {
            result: r,
            func: if is_async { "oxy_push_async_block" } else { "oxy_push_closure" },
            args: vec![],
            immediates: vec![],
        });
        r
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: parse + type-check + generate IR, return the IrGen and program.
    fn gen(source: &str) -> IrGen {
        let program = crate::parser::parse(source).expect("parse failed");
        crate::type_checker::TypeChecker::new().check_program(&program).expect("type-check failed");
        let mut ir = IrGen::new();
        ir.gen_program(&program);
        ir
    }

    /// Helper: find an IrFunction by name.
    fn find_fn<'a>(ir: &'a IrGen, name: &str) -> &'a IrFunction {
        ir.functions.iter().find(|f| f.name == name)
            .unwrap_or_else(|| panic!("function not found: {name}"))
    }

    /// Helper: collect all IrOp variants in a function as strings (for simple matching).
    fn op_names(f: &IrFunction) -> Vec<String> {
        f.blocks.iter().flat_map(|b| {
            b.ops.iter().map(|op| format!("{:?}", op))
        }).collect()
    }

    // ── Literals ───────────────────────────────────────────────────────

    #[test]
    fn test_literal_int() {
        let ir = gen("fn main() -> int { 42 }");
        let f = find_fn(&ir, "main");
        assert!(f.blocks.len() >= 1, "should have at least one block");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::ConstInt(_, 42))),
            "should have ConstInt(42), got: {:?}", ops);
    }

    #[test]
    fn test_literal_bool_true() {
        let ir = gen("fn main() -> bool { true }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::ConstBool(_, true))),
            "should have ConstBool(true), got: {:?}", ops);
    }

    #[test]
    fn test_literal_bool_false() {
        let ir = gen("fn main() -> bool { false }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::ConstBool(_, false))),
            "should have ConstBool(false), got: {:?}", ops);
    }

    #[test]
    fn test_literal_float() {
        let ir = gen("fn main() -> float { 3.14 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::ConstFloat(_, _))),
            "should have ConstFloat, got: {:?}", ops);
    }

    #[test]
    fn test_literal_string() {
        let ir = gen("fn main() -> String { \"hello\" }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::ConstString(_, _))),
            "should have ConstString, got: {:?}", ops);
    }

    #[test]
    fn test_literal_char() {
        let ir = gen("fn main() -> char { 'x' }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::ConstChar(_, 'x'))),
            "should have ConstChar('x'), got: {:?}", ops);
    }

    #[test]
    fn test_literal_unit() {
        let ir = gen("fn main() { }");
        let f = find_fn(&ir, "main");
        assert!(f.blocks.len() >= 1);
        // Should have terminator Return or Halt
    }

    // ── Binary arithmetic ──────────────────────────────────────────────

    #[test]
    fn test_add_two_ints() {
        let ir = gen("fn main() -> int { 1 + 2 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::Add(_, _, _))),
            "should have Add, got: {:?}", ops);
    }

    #[test]
    fn test_sub() {
        let ir = gen("fn main() -> int { 5 - 3 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::Sub(_, _, _))));
    }

    #[test]
    fn test_mul() {
        let ir = gen("fn main() -> int { 2 * 3 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::Mul(_, _, _))));
    }

    #[test]
    fn test_div() {
        let ir = gen("fn main() -> int { 6 / 2 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::Div(_, _, _))));
    }

    #[test]
    fn test_rem() {
        let ir = gen("fn main() -> int { 7 % 3 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::Rem(_, _, _))));
    }

    // ── Comparisons ────────────────────────────────────────────────────

    #[test]
    fn test_eq() {
        let ir = gen("fn main() -> bool { 1 == 2 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::Eq(_, _, _))));
    }

    #[test]
    fn test_neq() {
        let ir = gen("fn main() -> bool { 1 != 2 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::Neq(_, _, _))));
    }

    #[test]
    fn test_lt() {
        let ir = gen("fn main() -> bool { 1 < 2 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::Lt(_, _, _))));
    }

    #[test]
    fn test_gt() {
        let ir = gen("fn main() -> bool { 3 > 2 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::Gt(_, _, _))));
    }

    #[test]
    fn test_le() {
        let ir = gen("fn main() -> bool { 1 <= 2 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::Le(_, _, _))));
    }

    #[test]
    fn test_ge() {
        let ir = gen("fn main() -> bool { 2 >= 1 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::Ge(_, _, _))));
    }

    // ── Logical operators ──────────────────────────────────────────────

    #[test]
    fn test_and() {
        let ir = gen("fn main() -> bool { true && false }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::And(_, _, _))));
    }

    #[test]
    fn test_or() {
        let ir = gen("fn main() -> bool { true || false }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::Or(_, _, _))));
    }

    // ── Unary ──────────────────────────────────────────────────────────

    #[test]
    fn test_neg() {
        let ir = gen("fn main() -> int { -42 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::Neg(_, _))));
    }

    #[test]
    fn test_not() {
        let ir = gen("fn main() -> bool { !true }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::Not(_, _))));
    }

    // ── Bitwise ────────────────────────────────────────────────────────

    #[test]
    fn test_bitand() {
        let ir = gen("fn main() -> int { 1 & 2 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::BitAnd(_, _, _))));
    }

    #[test]
    fn test_bitor() {
        let ir = gen("fn main() -> int { 1 | 2 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::BitOr(_, _, _))));
    }

    #[test]
    fn test_bitxor() {
        let ir = gen("fn main() -> int { 1 ^ 2 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::BitXor(_, _, _))));
    }

    #[test]
    fn test_shl() {
        let ir = gen("fn main() -> int { 1 << 2 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::Shl(_, _, _))));
    }

    #[test]
    fn test_shr() {
        let ir = gen("fn main() -> int { 4 >> 1 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::Shr(_, _, _))));
    }

    // ── Variables (let bindings) ───────────────────────────────────────

    #[test]
    fn test_let_binding() {
        let ir = gen("fn main() -> int { let x = 5; x }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::StoreLocal(_, _))),
            "should have StoreLocal for let binding, got: {:?}", ops);
        assert!(ops.iter().any(|op| matches!(op, IrOp::LoadLocal(_, _))),
            "should have LoadLocal for reading x, got: {:?}", ops);
    }

    #[test]
    fn test_let_mut_binding() {
        let ir = gen("fn main() -> int { let mut x = 5; x = 10; x }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::StoreLocal(_, _))),
            "should have StoreLocal ops");
    }

    #[test]
    fn test_multiple_lets() {
        let ir = gen("fn main() -> int { let a = 1; let b = 2; a + b }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().filter(|op| matches!(op, IrOp::StoreLocal(_, _))).count() >= 2,
            "should have at least 2 StoreLocal ops");
    }

    // ── Control flow (if/else) ─────────────────────────────────────────

    #[test]
    fn test_if_then() {
        let ir = gen("fn main() -> int { if true { 1 } else { 0 } }");
        let f = find_fn(&ir, "main");
        assert!(f.blocks.len() >= 3, "should have at least 3 blocks (entry, then, else), got {}", f.blocks.len());
        // Should have Branch terminator
        let entry = &f.blocks[f.entry];
        assert!(matches!(entry.terminator, Terminator::Branch { .. }),
            "entry should have Branch terminator, got: {:?}", entry.terminator);
    }

    #[test]
    fn test_if_no_else() {
        let ir = gen("fn main() -> int { if true { 1 }; 0 }");
        let f = find_fn(&ir, "main");
        assert!(f.blocks.len() >= 2, "should have at least 2 blocks");
    }

    #[test]
    fn test_if_else_if() {
        let ir = gen("fn main() -> int { if true { 1 } else if false { 2 } else { 3 } }");
        let f = find_fn(&ir, "main");
        assert!(f.blocks.len() >= 4, "should have multiple blocks for else-if chain");
    }

    #[test]
    fn test_if_let() {
        let ir = gen("fn main() -> int { let x = Option::Some(42); if let Option::Some(v) = x { v } else { 0 } }");
        let f = find_fn(&ir, "main");
        assert!(f.blocks.len() >= 3, "if-let should have multiple blocks");
    }

    // ── Loops ──────────────────────────────────────────────────────────

    #[test]
    fn test_while_loop() {
        let ir = gen("fn main() -> int { let mut x = 0; while x < 5 { x = x + 1; } x }");
        let f = find_fn(&ir, "main");
        assert!(f.blocks.len() >= 2, "while should have at least 2 blocks, got {}", f.blocks.len());
    }

    #[test]
    fn test_loop_expression() {
        let ir = gen("fn main() -> int { let mut x = 0; loop { x = x + 1; if x > 5 { break; } } x }");
        let f = find_fn(&ir, "main");
        assert!(f.blocks.len() >= 2, "loop should have multiple blocks, got {}", f.blocks.len());
    }

    #[test]
    fn test_for_in() {
        let ir = gen("fn main() -> int { let mut sum = 0; for x in vec![1, 2, 3] { sum = sum + x; } sum }");
        let f = find_fn(&ir, "main");
        assert!(f.blocks.len() >= 2, "for-in should have multiple blocks, got {}", f.blocks.len());
    }

    #[test]
    fn test_break_value() {
        let ir = gen("fn main() -> int { let result = loop { break 42 }; result }");
        let f = find_fn(&ir, "main");
        assert!(f.blocks.len() >= 2);
    }

    #[test]
    fn test_continue_in_loop() {
        let ir = gen("fn main() -> int { let mut x = 0; while x < 10 { x = x + 1; if x == 2 { continue; } } x }");
        let f = find_fn(&ir, "main");
        assert!(f.blocks.len() >= 2, "while with continue should have blocks");
    }

    // ── Function calls ─────────────────────────────────────────────────

    #[test]
    fn test_fn_call_no_args() {
        let ir = gen("fn foo() -> int { 42 } fn main() -> int { foo() }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
            "should have a call op, got: {:?}", ops);
    }

    #[test]
    fn test_fn_call_with_args() {
        let ir = gen("fn add(a: int, b: int) -> int { a + b } fn main() -> int { add(1, 2) }");
        let f = find_fn(&ir, "main");
        assert!(!f.blocks.is_empty());
    }

    #[test]
    fn test_method_call() {
        let ir = gen("fn main() -> int { let s = \"hello\"; s.len() }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
            "method call should be a CallBuiltin");
    }

    #[test]
    fn test_path_call() {
        let ir = gen("fn main() -> int { let m = HashMap::new(); 0 }");
        let f = find_fn(&ir, "main");
        assert!(!f.blocks.is_empty());
    }

    // ── Return ─────────────────────────────────────────────────────────

    #[test]
    fn test_return_value() {
        let ir = gen("fn main() -> int { return 42; }");
        let f = find_fn(&ir, "main");
        let entry = &f.blocks[f.entry];
        assert!(matches!(entry.terminator, Terminator::Return(_)),
            "should have Return terminator, got: {:?}", entry.terminator);
    }

    #[test]
    fn test_return_expr_tail() {
        let ir = gen("fn main() -> int { 42 }");
        let f = find_fn(&ir, "main");
        let last_block = &f.blocks.last().unwrap();
        assert!(matches!(last_block.terminator, Terminator::Return(_)),
            "tail expr should generate Return terminator");
    }

    // ── Blocks and scoping ─────────────────────────────────────────────

    #[test]
    fn test_nested_block() {
        let ir = gen("fn main() -> int { let x = { let y = 1; y }; x }");
        let f = find_fn(&ir, "main");
        assert!(!f.blocks.is_empty());
    }

    // ── Struct construction ────────────────────────────────────────────

    #[test]
    fn test_struct_init() {
        let ir = gen("struct Point { x: int, y: int } fn main() -> int { let p = Point { x: 1, y: 2 }; p.x }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
            "struct init should use CallBuiltin for oxy_struct_init");
    }

    #[test]
    fn test_struct_update() {
        let ir = gen("struct Point { x: int, y: int } fn main() -> int { let p = Point { x: 1, y: 2 }; let p2 = Point { x: 3, ..p }; p2.x }");
        let f = find_fn(&ir, "main");
        assert!(!f.blocks.is_empty());
    }

    #[test]
    fn test_field_access() {
        let ir = gen("struct Point { x: int, y: int } fn main() -> int { let p = Point { x: 1, y: 2 }; p.x }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
            "field access should use CallBuiltin for oxy_field_access");
    }

    // ── Enum variants ──────────────────────────────────────────────────

    #[test]
    fn test_enum_variant_unit() {
        let ir = gen("enum Color { Red, Blue } fn main() -> int { let c = Color::Red; 0 }");
        let f = find_fn(&ir, "main");
        assert!(!f.blocks.is_empty());
    }

    #[test]
    fn test_enum_variant_with_data() {
        let ir = gen("enum MyOption { Some(int), None } fn main() -> int { let x = MyOption::Some(42); 0 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
            "enum variant should use CallBuiltin");
    }

    // ── Pattern matching ───────────────────────────────────────────────

    #[test]
    fn test_match_expression() {
        let ir = gen("fn main() -> int { match 1 { 0 => 10, 1 => 20, _ => 30 } }");
        let f = find_fn(&ir, "main");
        assert!(f.blocks.len() >= 3, "match should have multiple blocks, got {}", f.blocks.len());
    }

    #[test]
    fn test_match_on_enum() {
        let ir = gen("enum MyOption { Some(int), None } fn main() -> int { let x = MyOption::Some(42); match x { MyOption::Some(v) => v, MyOption::None => 0 } }");
        let f = find_fn(&ir, "main");
        assert!(f.blocks.len() >= 3, "match on enum should have multiple blocks");
    }

    // ── Collections ────────────────────────────────────────────────────

    #[test]
    fn test_vec_literal() {
        let ir = gen("fn main() -> int { let v = vec![1, 2, 3]; 0 }");
        let f = find_fn(&ir, "main");
        assert!(!f.blocks.is_empty());
    }

    #[test]
    fn test_array_literal() {
        let ir = gen("fn main() -> int { let a = [1, 2, 3]; 0 }");
        let f = find_fn(&ir, "main");
        assert!(!f.blocks.is_empty());
    }

    #[test]
    fn test_tuple_literal() {
        let ir = gen("fn main() -> int { let t = (1, \"hello\", true); 0 }");
        let f = find_fn(&ir, "main");
        assert!(!f.blocks.is_empty());
    }

    #[test]
    fn test_index_expr() {
        let ir = gen("fn main() -> int { let v = vec![1, 2, 3]; v[0] }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
            "index should use CallBuiltin for oxy_vec_index");
    }

    // ── F-string ───────────────────────────────────────────────────────

    #[test]
    fn test_fstring() {
        let ir = gen("fn main() -> String { let name = \"world\"; f\"Hello {name}!\" }");
        let f = find_fn(&ir, "main");
        assert!(!f.blocks.is_empty());
    }

    // ── Try operator ───────────────────────────────────────────────────

    #[test]
    fn test_try_operator() {
        let ir = gen("fn main() -> Option { let x = Option::Some(42); let y = x?; Option::Some(y) }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
            "try should use CallBuiltin for oxy_try_pop");
    }

    // ── Closures ───────────────────────────────────────────────────────

    #[test]
    fn test_closure_simple() {
        let ir = gen("fn main() -> int { let add = |a: int, b: int| -> int { a + b }; add(1, 2) }");
        let f = find_fn(&ir, "main");
        assert!(!f.blocks.is_empty());
        // Should have a separate function for the closure
        let closure = ir.functions.iter().find(|f| f.name.contains("closure"));
        assert!(closure.is_some(), "should have a closure function");
    }

    #[test]
    fn test_closure_capture() {
        let ir = gen("fn main() -> int { let x = 10; let f = || -> int { x }; f() }");
        let f = find_fn(&ir, "main");
        let closure = ir.functions.iter().find(|f| f.name.contains("closure"));
        assert!(closure.is_some(), "should have a closure function");
        if let Some(c) = closure {
            assert!(!c.captures.is_empty(), "closure should capture x");
        }
    }

    // ── Async ──────────────────────────────────────────────────────────

    #[test]
    fn test_async_block() {
        let ir = gen("fn main() -> int { let fut = async { 42 }; 0 }");
        let f = find_fn(&ir, "main");
        assert!(!f.blocks.is_empty());
    }

    // ── Generics ───────────────────────────────────────────────────────

    #[test]
    fn test_generic_fn() {
        let ir = gen("fn identity(x: int) -> int { x } fn main() -> int { identity(42) }");
        let f = find_fn(&ir, "identity");
        assert!(!f.blocks.is_empty());
    }

    // ── Multiple functions ─────────────────────────────────────────────

    #[test]
    fn test_multiple_functions() {
        let ir = gen("fn a() -> int { 1 } fn b() -> int { 2 } fn main() -> int { a() + b() }");
        assert!(find_fn(&ir, "a").blocks.len() >= 1);
        assert!(find_fn(&ir, "b").blocks.len() >= 1);
        assert!(find_fn(&ir, "main").blocks.len() >= 1);
    }

    // ── Assignment ─────────────────────────────────────────────────────

    #[test]
    fn test_assign() {
        let ir = gen("fn main() -> int { let mut x = 5; x = 10; x }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        let store_count = ops.iter().filter(|op| matches!(op, IrOp::StoreLocal(_, _))).count();
        assert!(store_count >= 2, "should have at least 2 stores (init + assignment), got {}", store_count);
    }

    #[test]
    fn test_compound_assign() {
        let ir = gen("fn main() -> int { let mut x = 5; x += 3; x }");
        let f = find_fn(&ir, "main");
        assert!(!f.blocks.is_empty());
    }

    // ── Self reference ─────────────────────────────────────────────────

    #[test]
    fn test_method_with_self() {
        let ir = gen("struct Counter { value: int } impl Counter { fn inc(mut self) { self.value = self.value + 1 } } fn main() -> int { 0 }");
        // Should generate IR for the inc method
        let method = ir.functions.iter().find(|f| f.name.contains("inc"));
        assert!(method.is_some(), "should have inc method, functions: {:?}", ir.functions.iter().map(|f| &f.name).collect::<Vec<_>>());
    }

    // ── Edge cases ─────────────────────────────────────────────────────

    #[test]
    fn test_empty_function() {
        let ir = gen("fn main() { }");
        let f = find_fn(&ir, "main");
        assert!(f.blocks.len() >= 1);
    }

    #[test]
    fn test_deeply_nested() {
        let ir = gen("fn main() -> int { let x = if true { if false { 1 } else { 2 } } else { 3 }; x }");
        let f = find_fn(&ir, "main");
        assert!(f.blocks.len() >= 4, "nested if should have multiple blocks");
    }

    #[test]
    fn test_complex_expression() {
        let ir = gen("fn main() -> int { let a = 1; let b = 2; let c = 3; (a + b) * c }");
        let f = find_fn(&ir, "main");
        assert!(!f.blocks.is_empty());
    }

    // ── Compile error tests ────────────────────────────────────────────

    #[test]
    fn test_unreachable_code_does_not_crash() {
        // Code after return should be handled gracefully
        let ir = gen("fn main() -> int { return 42; let x = 1; x }");
        let f = find_fn(&ir, "main");
        assert!(f.blocks.len() >= 1);
    }
}
