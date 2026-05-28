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

use super::ir::*;
use crate::ast::*;
use crate::type_checker::TypeInfo;

/// IR code generator. Walks a typed AST and produces register IR.
pub(crate) struct IrGen {
    /// All generated functions (including closures, async blocks).
    pub(crate) functions: Vec<IrFunction>,
    /// Closure metadata indexed by meta_idx (param_names, captures, is_async).
    pub(crate) closure_meta: Vec<(Vec<String>, Vec<(String, usize, bool)>, bool)>,
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
    /// Slot to store break value for `loop { break expr; }` result propagation.
    break_value_slot: Option<usize>,
    /// Labeled loop targets: label → (break_block, continue_block).
    labeled_targets: std::collections::HashMap<String, (BlockId, BlockId)>,
    /// Global const values: name → value expression (from `const NAME = expr;`).
    global_consts: std::collections::HashMap<String, crate::ast::Expr>,
}

impl IrGen {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            closure_meta: Vec::new(),
            current: IrFunction::new(String::new(), 0, 0),
            current_block: 0,
            next_reg: 0,
            next_block: 0,
            locals: std::collections::HashMap::new(),
            local_count: 0,
            break_target: None,
            continue_target: None,
            break_value_slot: None,
            labeled_targets: std::collections::HashMap::new(),
            global_consts: std::collections::HashMap::new(),
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
                Item::Function(f) => self.gen_fn(f, None),
                Item::Impl(imp) => {
                    let prefix = imp.type_name.clone();
                    for method in &imp.methods {
                        self.gen_fn(method, Some(&prefix));
                    }
                }
                Item::ImplTrait(imp) => {
                    let prefix = imp.type_name.clone();
                    for method in &imp.methods {
                        self.gen_fn(method, Some(&prefix));
                    }
                }
                Item::Const { name, value, .. } => {
                    self.global_consts.insert(name.clone(), value.clone());
                }
                // Struct/enum/mod/use/type items don't generate IR directly
                _ => {}
            }
        }
    }

    /// Convert a type annotation to TypeInfo (simple types only; complex types map to Unknown).
    fn type_ann_to_type_info(ann: &TypeAnnotation) -> TypeInfo {
        match ann {
            TypeAnnotation::Named {
                name, generic_args, ..
            } => {
                if generic_args.is_empty() {
                    TypeInfo::from_name(name)
                } else {
                    // Parameterized types — map generics and construct
                    let args: Vec<TypeInfo> = generic_args
                        .iter()
                        .map(Self::type_ann_to_type_info)
                        .collect();
                    match name.as_str() {
                        "Vec" => TypeInfo::Vec(Box::new(
                            args.first().cloned().unwrap_or(TypeInfo::Unknown),
                        )),
                        "HashMap" => TypeInfo::HashMap(
                            Box::new(args.first().cloned().unwrap_or(TypeInfo::Unknown)),
                            Box::new(args.get(1).cloned().unwrap_or(TypeInfo::Unknown)),
                        ),
                        "Option" => TypeInfo::Option(Box::new(
                            args.first().cloned().unwrap_or(TypeInfo::Unknown),
                        )),
                        "Result" => TypeInfo::Result(
                            Box::new(args.first().cloned().unwrap_or(TypeInfo::Unknown)),
                            Box::new(args.get(1).cloned().unwrap_or(TypeInfo::Unknown)),
                        ),
                        _ => TypeInfo::UserStruct {
                            name: name.clone(),
                            generic_args: args,
                        },
                    }
                }
            }
            TypeAnnotation::Array { inner, size, .. } => {
                TypeInfo::Array(Box::new(Self::type_ann_to_type_info(inner)), *size)
            }
        }
    }

    /// Generate IR for one function.
    fn gen_fn(&mut self, f: &FnDef, struct_prefix: Option<&str>) {
        let ret_ty = f
            .return_type
            .as_ref()
            .map(Self::type_ann_to_type_info)
            .unwrap_or(TypeInfo::Unit);
        let name = match struct_prefix {
            Some(prefix) => format!("{prefix}::{}", f.name),
            None => f.name.clone(),
        };
        // Save current state
        let saved = std::mem::replace(&mut self.current, IrFunction::new(name, 0, 0));
        self.current.return_type = ret_ty;
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
        if !matches!(
            self.current.blocks[self.current_block].terminator,
            Terminator::Return(_) | Terminator::Panic(_)
        ) {
            let reg = result_reg.unwrap_or_else(|| {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstUnit(r));
                r
            });
            self.terminate(Terminator::Return(reg));
        }

        self.current.local_count = self.local_count;
        self.functions
            .push(std::mem::replace(&mut self.current, saved));

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
                } else if let Some(const_val) = self.global_consts.get(name).cloned() {
                    // Inline const value at use site
                    self.gen_expr(&const_val)
                } else {
                    // Global function or builtin — reference for Call handling
                    let r = self.alloc_reg();
                    self.emit(IrOp::ConstUnit(r));
                    r
                }
            }
            Expr::Block(block) => self.gen_block_stmts(block).unwrap_or_else(|| {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstUnit(r));
                r
            }),
            Expr::BinaryOp {
                left, op, right, ..
            } => {
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
                    strings: vec![fname],
                });
                r
            }
            Expr::MethodCall {
                object,
                method,
                args,
                ..
            } => {
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
                    immediates: vec![args.len()],
                    strings: vec![method.clone()],
                });
                r
            }
            Expr::If {
                condition,
                then_block,
                else_block,
                ..
            } => self.gen_if(condition, then_block, else_block.as_deref()),
            Expr::IfLet {
                pattern,
                expr,
                then_block,
                else_block,
                guard,
                ..
            } => {
                if let Some(guard_expr) = guard {
                    self.gen_if_let_guarded(
                        pattern,
                        expr,
                        guard_expr,
                        then_block,
                        else_block.as_deref(),
                    )
                } else {
                    self.gen_if_let(pattern, expr, then_block, else_block.as_deref())
                }
            }
            Expr::Match { expr, arms, .. } => self.gen_match(expr, arms),
            Expr::StructInit {
                name, fields, base, ..
            } => {
                let mut arg_regs = Vec::new();
                let mut field_names = Vec::new();
                for (fname, val) in fields {
                    arg_regs.push(self.gen_expr(val));
                    field_names.push(fname.clone());
                }
                // Join field names with \0 for the FFI to parse.
                let names_joined = field_names.join("\0");
                if let Some(base_expr) = base {
                    // Struct update: Point { x: 1, ..base }
                    let base_reg = self.gen_expr(base_expr);
                    arg_regs.push(base_reg);
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_struct_update",
                        args: arg_regs,
                        immediates: vec![fields.len()],
                        strings: vec![names_joined],
                    });
                    r
                } else {
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_struct_init",
                        args: arg_regs,
                        immediates: vec![fields.len()],
                        strings: vec![name.clone(), names_joined],
                    });
                    r
                }
            }
            Expr::FieldAccess { object, field, .. } => {
                let obj = self.gen_expr(object);
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_field_access",
                    args: vec![obj],
                    immediates: vec![],
                    strings: vec![field.clone()],
                });
                r
            }
            Expr::PathCall {
                path,
                turbofish: _,
                args,
                ..
            } => {
                let mut arg_regs = Vec::new();
                for a in args {
                    arg_regs.push(self.gen_expr(a));
                }
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_path_call_builtin",
                    args: arg_regs,
                    immediates: vec![args.len()],
                    strings: vec![path.join("\0")],
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
                    strings: vec![],
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
                    strings: vec![],
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
                    strings: vec![],
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
                    strings: vec![],
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
                    strings: vec![],
                });
                r
            }
            Expr::Assign { target, value, .. } => {
                let val_reg = self.gen_expr(value);
                match target.as_ref() {
                    Expr::Ident(name, ..) => {
                        if let Some(slot) = self.lookup_local(name) {
                            self.emit(IrOp::StoreLocal(slot, val_reg));
                        }
                    }
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
                    }
                    _ => {}
                }
                val_reg
            }
            Expr::CompoundAssign {
                target, op, value, ..
            } => {
                let val_reg = self.gen_expr(value);
                let target_reg = self.gen_expr(target);
                let r = self.alloc_reg();
                match op {
                    BinOp::Add => self.emit(IrOp::Add(r, target_reg, val_reg)),
                    BinOp::Sub => self.emit(IrOp::Sub(r, target_reg, val_reg)),
                    BinOp::Mul => self.emit(IrOp::Mul(r, target_reg, val_reg)),
                    BinOp::Div => self.emit(IrOp::Div(r, target_reg, val_reg)),
                    BinOp::Mod => self.emit(IrOp::Rem(r, target_reg, val_reg)),
                    BinOp::BitAnd => self.emit(IrOp::BitAnd(r, target_reg, val_reg)),
                    BinOp::BitOr => self.emit(IrOp::BitOr(r, target_reg, val_reg)),
                    BinOp::BitXor => self.emit(IrOp::BitXor(r, target_reg, val_reg)),
                    BinOp::Shl => self.emit(IrOp::Shl(r, target_reg, val_reg)),
                    BinOp::Shr => self.emit(IrOp::Shr(r, target_reg, val_reg)),
                    _ => {
                        self.emit(IrOp::Copy(r, val_reg));
                    }
                }
                if let Expr::Ident(name, ..) = target.as_ref() {
                    if let Some(slot) = self.lookup_local(name) {
                        self.emit(IrOp::StoreLocal(slot, r));
                    }
                }
                r
            }
            Expr::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                let start_reg = start.as_ref().map(|s| self.gen_expr(s)).unwrap_or_else(|| {
                    let r = self.alloc_reg();
                    self.emit(IrOp::ConstInt(r, 0));
                    r
                });
                let end_reg = end.as_ref().map(|e| self.gen_expr(e)).unwrap_or_else(|| {
                    let r = self.alloc_reg();
                    // i64::MAX sentinel for unbounded — avoids conflicting with
                    // legitimate -1 as a range endpoint.
                    self.emit(IrOp::ConstInt(r, i64::MAX));
                    r
                });
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_make_range",
                    args: vec![start_reg, end_reg],
                    immediates: vec![*inclusive as usize],
                    strings: vec![],
                });
                r
            }
            Expr::Closure {
                params,
                body,
                is_async,
                ..
            } => self.gen_closure(params, body, *is_async),
            Expr::Path { segments, .. } => {
                // Unit enum variant: Color::Red
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_const_enum_variant",
                    args: vec![],
                    immediates: vec![],
                    strings: segments.clone(),
                });
                r
            }
            Expr::SelfRef(..) => {
                // self parameter — load from local slot 0
                let r = self.alloc_reg();
                self.emit(IrOp::LoadLocal(r, 0));
                r
            }
            Expr::Grouped(inner, _) => self.gen_expr(inner),
            Expr::MacroCall { name, args, .. } => {
                let mut arg_regs = Vec::new();
                for a in args {
                    arg_regs.push(self.gen_expr(a));
                }
                let r = self.alloc_reg();
                let (func, strings, extra_immediates) = match name.as_str() {
                    "println" => ("oxy_println_val", vec![], vec![]),
                    "print" => ("oxy_print_val", vec![], vec![]),
                    "format" => ("oxy_format", vec![], vec![args.len()]),
                    _ => (
                        "oxy_path_call_builtin",
                        vec![name.clone()],
                        vec![args.len()],
                    ),
                };
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func,
                    args: arg_regs,
                    immediates: extra_immediates,
                    strings,
                });
                r
            }
            Expr::Repeat { value, count, .. } => {
                let val_reg = self.gen_expr(value);
                let count_reg = self.gen_expr(count);
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_make_repeat",
                    args: vec![val_reg, count_reg],
                    immediates: vec![],
                    strings: vec![],
                });
                r
            }
            Expr::AsyncBlock { body, .. } => {
                // Generate as a closure-like async function
                let params: Vec<ClosureParam> = Vec::new();
                let body_expr = Expr::Block(body.clone());
                self.gen_closure(&params, &body_expr, true)
            }
            Expr::Await { expr, .. } => {
                let val = self.gen_expr(expr);
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_await_ffi",
                    args: vec![val],
                    immediates: vec![],
                    strings: vec![],
                });
                r
            }
            Expr::Return { value, .. } => {
                let reg = value.as_ref().map(|v| self.gen_expr(v)).unwrap_or_else(|| {
                    let r = self.alloc_reg();
                    self.emit(IrOp::ConstUnit(r));
                    r
                });
                self.terminate(Terminator::Return(reg));
                reg
            }
            Expr::As {
                expr, type_name, ..
            } => {
                let val = self.gen_expr(expr);
                let r = self.alloc_reg();
                let func = match type_name.as_str() {
                    "int" => "oxy_cast_int",
                    "float" => "oxy_cast_float",
                    "char" => "oxy_cast_to_char",
                    _ => "oxy_cast_int",
                };
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func,
                    args: vec![val],
                    immediates: vec![],
                    strings: vec![],
                });
                r
            } // Unreachable: all Expr variants are handled above.
        }
    }

    // ── Control flow helpers ───────────────────────────────────────────

    fn gen_if(&mut self, condition: &Expr, then_block: &Block, else_block: Option<&Expr>) -> Reg {
        let cond = self.gen_expr(condition);

        let then_id = self.alloc_block();
        let else_id = self.alloc_block();
        let merge_id = self.alloc_block();

        // Entry: branch
        self.terminate(Terminator::Branch {
            cond,
            then_block: then_id,
            else_block: else_id,
        });

        // Then block
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
        if self.current.blocks[self.current_block]
            .terminator
            .is_default()
        {
            self.terminate(Terminator::Jump(merge_id));
        }

        // Merge block with phi
        self.start_block(merge_id);
        let r = self.alloc_reg();
        self.emit(IrOp::Phi(r, then_reg, else_reg));
        r
    }

    fn gen_if_let(
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
        r
    }

    fn gen_if_let_guarded(
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
        r
    }

    fn gen_match(&mut self, expr: &Expr, arms: &[MatchArm]) -> Reg {
        let val = self.gen_expr(expr);

        // Pre-allocate all check blocks and body blocks.
        // IMPORTANT: final_merge must be allocated AFTER body blocks so its block ID
        // is greater. Codegen processes blocks in ID order and the regs/reg_slot
        // maps are populated as blocks are visited. If final_merge were processed
        // before a body block, Return would see neither regs nor reg_slot.
        let n = arms.len();
        let check_blocks: Vec<BlockId> = (0..n).map(|_| self.alloc_block()).collect();
        let body_blocks: Vec<BlockId> = (0..n).map(|_| self.alloc_block()).collect();
        let final_merge = self.alloc_block();
        let mut result_regs: Vec<Reg> = Vec::new();

        // First check uses current block, subsequent checks use check_blocks[i]
        for (i, arm) in arms.iter().enumerate() {
            if i > 0 {
                self.start_block(check_blocks[i]);
            }
            let matches = self.gen_pattern_check(&arm.pattern, val);
            let else_target = if i + 1 < n {
                check_blocks[i + 1]
            } else {
                body_blocks[i]
            };
            self.terminate(Terminator::Branch {
                cond: matches,
                then_block: body_blocks[i],
                else_block: else_target,
            });
        }

        // Generate arm body blocks
        for (i, arm) in arms.iter().enumerate() {
            self.start_block(body_blocks[i]);
            self.gen_pattern_bind(&arm.pattern, val);
            let reg = self.gen_expr(&arm.body);
            result_regs.push(reg);
            if n <= 2 {
                self.terminate(Terminator::Jump(final_merge));
            }
            // For 3+ arms, jump targets set below
        }

        if result_regs.is_empty() {
            self.start_block(final_merge);
            let r = self.alloc_reg();
            self.emit(IrOp::ConstUnit(r));
            return r;
        }

        if n == 1 {
            // Single arm — no Phi needed, return the result directly.
            self.start_block(final_merge);
            return result_regs[0];
        }

        if n == 2 {
            self.start_block(final_merge);
            let r = self.alloc_reg();
            self.emit(IrOp::Phi(r, result_regs[0], result_regs[1]));
            return r;
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
                // First pair: body_0, body_1
                self.start_block(body_blocks[0]);
                if n > 1 {
                    let merge_01 = self.alloc_block();
                    self.terminate(Terminator::Jump(merge_01));
                    self.start_block(body_blocks[1]);
                    self.terminate(Terminator::Jump(merge_01));
                    self.start_block(merge_01);
                    let phi_r = self.alloc_reg();
                    self.emit(IrOp::Phi(phi_r, result_regs[0], result_regs[1]));
                    prev_merge = Some((merge_01, phi_r));
                    idx = 2;
                } else {
                    // Only one arm — jump directly to final_merge
                    self.terminate(Terminator::Jump(final_merge));
                    self.start_block(final_merge);
                    return result_regs[0];
                }
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
                // Body block jumps to cascade_merge
                self.current.block_mut(body_blocks[idx]).terminator =
                    Terminator::Jump(cascade_merge);
                self.start_block(cascade_merge);
                let phi_r = self.alloc_reg();
                self.emit(IrOp::Phi(phi_r, prev_reg, result_regs[idx]));
                prev_merge = Some((cascade_merge, phi_r));
                idx += 1;
            }
        }
        // prev_merge now holds the final merge block with the final phi result
        prev_merge.unwrap().1
    }

    /// Emit a pattern-match check: returns a register that is truthy if pattern matches.
    fn gen_pattern_check(&mut self, pattern: &Pattern, val_reg: Reg) -> Reg {
        let mut r = self.alloc_reg();
        match pattern {
            Pattern::Literal(lit_expr) => {
                let lit_reg = self.gen_expr(lit_expr);
                self.emit(IrOp::Eq(r, val_reg, lit_reg));
            }
            Pattern::Wildcard(..) | Pattern::Ident(..) | Pattern::Rest(..) => {
                self.emit(IrOp::ConstBool(r, true));
            }
            Pattern::EnumVariant {
                enum_name,
                variant,
                fields,
                ..
            } => {
                // Check variant discriminant, then recursively check inner field patterns
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_enum_variant_equal",
                    args: vec![val_reg],
                    immediates: vec![],
                    strings: vec![enum_name.clone(), variant.clone()],
                });
                // If there are inner patterns, also check them
                for (i, inner) in fields.iter().enumerate() {
                    let inner_val = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: inner_val,
                        func: "oxy_enum_data_get",
                        args: vec![val_reg],
                        immediates: vec![i],
                        strings: vec![],
                    });
                    let inner_match = self.gen_pattern_check(inner, inner_val);
                    // AND the results: r = r && inner_match
                    let new_r = self.alloc_reg();
                    self.emit(IrOp::And(new_r, r, inner_match));
                    r = new_r;
                }
            }
            Pattern::Tuple(patterns, ..) => {
                // Check each element recursively, AND all results
                self.emit(IrOp::ConstBool(r, true));
                for (i, p) in patterns.iter().enumerate() {
                    let elem_val = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: elem_val,
                        func: "oxy_enum_data_get",
                        args: vec![val_reg],
                        immediates: vec![i],
                        strings: vec![],
                    });
                    let elem_match = self.gen_pattern_check(p, elem_val);
                    let new_r = self.alloc_reg();
                    self.emit(IrOp::And(new_r, r, elem_match));
                    r = new_r;
                }
            }
            Pattern::Struct { fields, .. } => {
                // Check each named field recursively
                self.emit(IrOp::ConstBool(r, true));
                for (i, (_fname, p)) in fields.iter().enumerate() {
                    let field_val = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: field_val,
                        func: "oxy_field_access",
                        args: vec![val_reg],
                        immediates: vec![i],
                        strings: vec![_fname.clone()],
                    });
                    let field_match = self.gen_pattern_check(p, field_val);
                    let new_r = self.alloc_reg();
                    self.emit(IrOp::And(new_r, r, field_match));
                    r = new_r;
                }
            }
            Pattern::Or(patterns, ..) => {
                // Match if ANY sub-pattern matches
                self.emit(IrOp::ConstBool(r, false));
                for p in patterns {
                    let sub_match = self.gen_pattern_check(p, val_reg);
                    let new_r = self.alloc_reg();
                    self.emit(IrOp::Or(new_r, r, sub_match));
                    r = new_r;
                }
            }
            Pattern::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                // Check if val_reg is within [start, end] or [start, end)
                self.emit(IrOp::ConstBool(r, true));
                if let Some(lo) = start {
                    let lo_reg = self.alloc_reg();
                    self.emit(IrOp::ConstInt(lo_reg, *lo));
                    let ge_check = self.alloc_reg();
                    self.emit(IrOp::Ge(ge_check, val_reg, lo_reg));
                    let new_r = self.alloc_reg();
                    self.emit(IrOp::And(new_r, r, ge_check));
                    r = new_r;
                }
                if let Some(hi) = end {
                    let hi_reg = self.alloc_reg();
                    self.emit(IrOp::ConstInt(hi_reg, *hi));
                    let cmp = self.alloc_reg();
                    if *inclusive {
                        self.emit(IrOp::Le(cmp, val_reg, hi_reg));
                    } else {
                        self.emit(IrOp::Lt(cmp, val_reg, hi_reg));
                    }
                    let new_r = self.alloc_reg();
                    self.emit(IrOp::And(new_r, r, cmp));
                    r = new_r;
                }
            }
            Pattern::Slice(patterns, ..) => {
                // Check each element against sub-patterns, skipping Rest (..).
                // No length check — we don't have a collection-len FFI function yet.
                self.emit(IrOp::ConstBool(r, true));
                let mut elem_idx = 0usize;
                for p in patterns {
                    if matches!(p, Pattern::Rest(..)) {
                        continue;
                    }
                    let elem_val = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: elem_val,
                        func: "oxy_enum_data_get",
                        args: vec![val_reg],
                        immediates: vec![elem_idx],
                        strings: vec![],
                    });
                    let sub = self.gen_pattern_check(p, elem_val);
                    let new_r = self.alloc_reg();
                    self.emit(IrOp::And(new_r, r, sub));
                    r = new_r;
                    elem_idx += 1;
                }
            }
        }
        r
    }

    fn gen_while(&mut self, condition: &Expr, body: &Block, label: Option<&str>) -> Option<Reg> {
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

    fn gen_while_let(
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

    fn gen_loop(&mut self, body: &Block, label: Option<&str>) -> Option<Reg> {
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

    fn gen_for_in(
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

        // Header: call iter.next(), store element, check if done
        self.start_block(header_id);
        let elem_r = self.alloc_reg();
        self.emit(IrOp::CallBuiltin {
            result: elem_r,
            func: "oxy_iter_next",
            args: vec![],
            immediates: vec![state_slot],
            strings: vec![],
        });
        self.emit(IrOp::StoreLocal(var_slot, elem_r));
        let cond = self.alloc_reg();
        self.emit(IrOp::CallBuiltin {
            result: cond,
            func: "oxy_is_truthy",
            args: vec![elem_r],
            immediates: vec![],
            strings: vec![],
        });
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

    fn gen_for_destructure(
        &mut self,
        names: &[String],
        iterable: &Expr,
        body: &Block,
        label: Option<&str>,
    ) -> Option<Reg> {
        let iter_expr_reg = self.gen_expr(iterable);
        for name in names {
            self.alloc_local(name);
        }
        let state_slot = self.alloc_local("__iter_state");

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
        let next_r = self.alloc_reg();
        self.emit(IrOp::CallBuiltin {
            result: next_r,
            func: "oxy_iter_next_destructure",
            args: vec![],
            immediates: vec![state_slot],
            strings: vec![],
        });
        let cond = self.alloc_reg();
        self.emit(IrOp::CallBuiltin {
            result: cond,
            func: "oxy_is_truthy",
            args: vec![next_r],
            immediates: vec![],
            strings: vec![],
        });
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
                        strings: vec![],
                    });
                    self.gen_pattern_bind(p, r);
                }
            }
            Pattern::Struct { fields, .. } => {
                for (i, (_fname, p)) in fields.iter().enumerate() {
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_field_access",
                        args: vec![val_reg],
                        immediates: vec![i],
                        strings: vec![_fname.clone()],
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
                        strings: vec![],
                    });
                    self.gen_pattern_bind(p, r);
                }
            }
            Pattern::Literal(..) => {}
            Pattern::Or(patterns, ..) => {
                // Bind from the first sub-pattern. The type checker ensures all
                // arms bind the same variables with the same types, so any arm
                // produces equivalent bindings.
                if let Some(first) = patterns.first() {
                    self.gen_pattern_bind(first, val_reg);
                }
            }
            Pattern::Rest(..) => {}
            Pattern::Slice(patterns, ..) => {
                let mut elem_idx = 0usize;
                for p in patterns {
                    if matches!(p, Pattern::Rest(..)) {
                        continue;
                    }
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_enum_data_get",
                        args: vec![val_reg],
                        immediates: vec![elem_idx],
                        strings: vec![],
                    });
                    self.gen_pattern_bind(p, r);
                    elem_idx += 1;
                }
            }
            Pattern::Range { .. } => {}
        }
    }

    /// Collect free variable names in an expression (variables not in param_names).
    fn collect_free_vars(
        &self,
        expr: &Expr,
        param_names: &std::collections::HashSet<String>,
    ) -> Vec<String> {
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

    fn gen_closure(&mut self, params: &[ClosureParam], body: &Expr, is_async: bool) -> Reg {
        // Find free variables (captures) by scanning the closure body for Idents
        // that reference outer-scope locals, not params.
        let param_names: std::collections::HashSet<String> =
            params.iter().map(|p| p.name.clone()).collect();
        let free_vars = self.collect_free_vars(body, &param_names);

        let meta_idx = self.closure_meta.len();
        let closure_name = format!("closure_{}", meta_idx);
        let saved = std::mem::replace(
            &mut self.current,
            IrFunction::new(closure_name.clone(), 0, 0),
        );
        let saved_locals = std::mem::take(&mut self.locals);
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
            .map(|(name, slot)| (name.clone(), *slot, false))
            .collect();
        self.closure_meta
            .push((param_names_for_meta, captures_with_mut, is_async));

        // Allocate locals for params
        for p in params {
            self.alloc_local(&p.name);
        }

        let result_reg = self.gen_expr(body);
        if !matches!(
            self.current.blocks[self.current_block].terminator,
            Terminator::Return(_)
        ) {
            self.terminate(Terminator::Return(result_reg));
        }

        self.current.local_count = self.local_count;
        self.current.is_async = is_async;
        self.functions
            .push(std::mem::replace(&mut self.current, saved));

        self.locals = saved_locals;
        self.local_count = saved_local_count;
        self.current_block = saved_current_block;
        self.next_reg = saved_next_reg;
        self.next_block = saved_next_block;

        // Return a register referencing the closure.
        // The closure body is compiled as a separate IrFunction in self.functions.
        // meta_idx allows the FFI layer to look up the captures in the closure meta table.
        let r = self.alloc_reg();
        self.emit(IrOp::CallBuiltin {
            result: r,
            func: if is_async {
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

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: parse + type-check + generate IR, return the IrGen and program.
    fn gen(source: &str) -> IrGen {
        let program = crate::parser::parse(source).expect("parse failed");
        crate::type_checker::TypeChecker::new()
            .check_program(&program)
            .expect("type-check failed");
        let mut ir = IrGen::new();
        ir.gen_program(&program);
        ir
    }

    /// Helper: find an IrFunction by name.
    fn find_fn<'a>(ir: &'a IrGen, name: &str) -> &'a IrFunction {
        ir.functions
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("function not found: {name}"))
    }

    /// Helper: collect all IrOp variants in a function as strings (for simple matching).
    fn op_names(f: &IrFunction) -> Vec<String> {
        f.blocks
            .iter()
            .flat_map(|b| b.ops.iter().map(|op| format!("{:?}", op)))
            .collect()
    }

    // ── Literals ───────────────────────────────────────────────────────

    #[test]
    fn test_literal_int() {
        let ir = gen("fn main() -> int { 42 }");
        let f = find_fn(&ir, "main");
        assert!(!f.blocks.is_empty(), "should have at least one block");
        let ops = &f.blocks[f.entry].ops;
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::ConstInt(_, 42))),
            "should have ConstInt(42), got: {:?}",
            ops
        );
    }

    #[test]
    fn test_literal_bool_true() {
        let ir = gen("fn main() -> bool { true }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::ConstBool(_, true))),
            "should have ConstBool(true), got: {:?}",
            ops
        );
    }

    #[test]
    fn test_literal_bool_false() {
        let ir = gen("fn main() -> bool { false }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::ConstBool(_, false))),
            "should have ConstBool(false), got: {:?}",
            ops
        );
    }

    #[test]
    fn test_literal_float() {
        let ir = gen("fn main() -> float { 3.14 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::ConstFloat(_, _))),
            "should have ConstFloat, got: {:?}",
            ops
        );
    }

    #[test]
    fn test_literal_string() {
        let ir = gen("fn main() -> String { \"hello\" }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::ConstString(_, _))),
            "should have ConstString, got: {:?}",
            ops
        );
    }

    #[test]
    fn test_literal_char() {
        let ir = gen("fn main() -> char { 'x' }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::ConstChar(_, 'x'))),
            "should have ConstChar('x'), got: {:?}",
            ops
        );
    }

    #[test]
    fn test_literal_unit() {
        let ir = gen("fn main() { }");
        let f = find_fn(&ir, "main");
        assert!(!f.blocks.is_empty());
        // Should have terminator Return or Halt
    }

    // ── Binary arithmetic ──────────────────────────────────────────────

    #[test]
    fn test_add_two_ints() {
        let ir = gen("fn main() -> int { 1 + 2 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::Add(_, _, _))),
            "should have Add, got: {:?}",
            ops
        );
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
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::StoreLocal(_, _))),
            "should have StoreLocal for let binding, got: {:?}",
            ops
        );
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::LoadLocal(_, _))),
            "should have LoadLocal for reading x, got: {:?}",
            ops
        );
    }

    #[test]
    fn test_let_mut_binding() {
        let ir = gen("fn main() -> int { let mut x = 5; x = 10; x }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::StoreLocal(_, _))),
            "should have StoreLocal ops"
        );
    }

    #[test]
    fn test_multiple_lets() {
        let ir = gen("fn main() -> int { let a = 1; let b = 2; a + b }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(
            ops.iter()
                .filter(|op| matches!(op, IrOp::StoreLocal(_, _)))
                .count()
                >= 2,
            "should have at least 2 StoreLocal ops"
        );
    }

    // ── Control flow (if/else) ─────────────────────────────────────────

    #[test]
    fn test_if_then() {
        let ir = gen("fn main() -> int { if true { 1 } else { 0 } }");
        let f = find_fn(&ir, "main");
        assert!(
            f.blocks.len() >= 3,
            "should have at least 3 blocks (entry, then, else), got {}",
            f.blocks.len()
        );
        // Should have Branch terminator
        let entry = &f.blocks[f.entry];
        assert!(
            matches!(entry.terminator, Terminator::Branch { .. }),
            "entry should have Branch terminator, got: {:?}",
            entry.terminator
        );
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
        assert!(
            f.blocks.len() >= 4,
            "should have multiple blocks for else-if chain"
        );
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
        assert!(
            f.blocks.len() >= 2,
            "while should have at least 2 blocks, got {}",
            f.blocks.len()
        );
    }

    #[test]
    fn test_loop_expression() {
        let ir =
            gen("fn main() -> int { let mut x = 0; loop { x = x + 1; if x > 5 { break; } } x }");
        let f = find_fn(&ir, "main");
        assert!(
            f.blocks.len() >= 2,
            "loop should have multiple blocks, got {}",
            f.blocks.len()
        );
    }

    #[test]
    fn test_for_in() {
        let ir = gen(
            "fn main() -> int { let mut sum = 0; for x in vec![1, 2, 3] { sum = sum + x; } sum }",
        );
        let f = find_fn(&ir, "main");
        assert!(
            f.blocks.len() >= 2,
            "for-in should have multiple blocks, got {}",
            f.blocks.len()
        );
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
        assert!(
            f.blocks.len() >= 2,
            "while with continue should have blocks"
        );
    }

    // ── Function calls ─────────────────────────────────────────────────

    #[test]
    fn test_fn_call_no_args() {
        let ir = gen("fn foo() -> int { 42 } fn main() -> int { foo() }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
            "should have a call op, got: {:?}",
            ops
        );
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
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
            "method call should be a CallBuiltin"
        );
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
        assert!(
            matches!(entry.terminator, Terminator::Return(_)),
            "should have Return terminator, got: {:?}",
            entry.terminator
        );
    }

    #[test]
    fn test_return_expr_tail() {
        let ir = gen("fn main() -> int { 42 }");
        let f = find_fn(&ir, "main");
        let last_block = &f.blocks.last().unwrap();
        assert!(
            matches!(last_block.terminator, Terminator::Return(_)),
            "tail expr should generate Return terminator"
        );
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
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
            "struct init should use CallBuiltin for oxy_struct_init"
        );
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
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
            "field access should use CallBuiltin for oxy_field_access"
        );
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
        let ir = gen(
            "enum MyOption { Some(int), None } fn main() -> int { let x = MyOption::Some(42); 0 }",
        );
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
            "enum variant should use CallBuiltin"
        );
    }

    // ── Pattern matching ───────────────────────────────────────────────

    #[test]
    fn test_match_expression() {
        let ir = gen("fn main() -> int { match 1 { 0 => 10, 1 => 20, _ => 30 } }");
        let f = find_fn(&ir, "main");
        assert!(
            f.blocks.len() >= 3,
            "match should have multiple blocks, got {}",
            f.blocks.len()
        );
    }

    #[test]
    fn test_match_on_enum() {
        let ir = gen("enum MyOption { Some(int), None } fn main() -> int { let x = MyOption::Some(42); match x { MyOption::Some(v) => v, MyOption::None => 0 } }");
        let f = find_fn(&ir, "main");
        assert!(
            f.blocks.len() >= 3,
            "match on enum should have multiple blocks"
        );
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
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
            "index should use CallBuiltin for oxy_vec_index"
        );
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
        let ir =
            gen("fn main() -> Option { let x = Option::Some(42); let y = x?; Option::Some(y) }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
            "try should use CallBuiltin for oxy_try_pop"
        );
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
        let _f = find_fn(&ir, "main");
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
        assert!(!find_fn(&ir, "a").blocks.is_empty());
        assert!(!find_fn(&ir, "b").blocks.is_empty());
        assert!(!find_fn(&ir, "main").blocks.is_empty());
    }

    // ── Assignment ─────────────────────────────────────────────────────

    #[test]
    fn test_assign() {
        let ir = gen("fn main() -> int { let mut x = 5; x = 10; x }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        let store_count = ops
            .iter()
            .filter(|op| matches!(op, IrOp::StoreLocal(_, _)))
            .count();
        assert!(
            store_count >= 2,
            "should have at least 2 stores (init + assignment), got {}",
            store_count
        );
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
        assert!(
            method.is_some(),
            "should have inc method, functions: {:?}",
            ir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
    }

    // ── Edge cases ─────────────────────────────────────────────────────

    #[test]
    fn test_empty_function() {
        let ir = gen("fn main() { }");
        let f = find_fn(&ir, "main");
        assert!(!f.blocks.is_empty());
    }

    #[test]
    fn test_deeply_nested() {
        let ir =
            gen("fn main() -> int { let x = if true { if false { 1 } else { 2 } } else { 3 }; x }");
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
        assert!(!f.blocks.is_empty());
    }

    // ── Gaps from audit: MacroCall, Grouped, Repeat, AsyncBlock, Await ──

    #[test]
    fn test_grouped_expression() {
        let ir = gen("fn main() -> int { (42) }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::ConstInt(_, 42))),
            "grouped should unwrap inner literal, got: {:?}",
            ops
        );
    }

    #[test]
    fn test_macro_call_println() {
        let ir = gen("fn main() { println!(\"hello\") }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
            "println should emit CallBuiltin"
        );
    }

    #[test]
    fn test_repeat_expression() {
        let ir = gen("fn main() -> int { let a = [0; 5]; 0 }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
            "repeat should emit CallBuiltin"
        );
    }

    #[test]
    fn test_async_block_expr() {
        let ir = gen("fn main() -> int { let fut = async { 42 }; 0 }");
        let f = find_fn(&ir, "main");
        assert!(!f.blocks.is_empty());
    }

    #[test]
    fn test_await_expr() {
        let ir = gen("fn main() -> int { let fut = async { 42 }; fut.await }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(
            ops.iter().any(|op| matches!(op, IrOp::CallBuiltin { .. })),
            "await should emit CallBuiltin"
        );
    }

    // ── Gaps from audit: WhileLet, ForDestructure, LetPattern ──────────

    #[test]
    fn test_while_let() {
        let ir = gen("fn main() -> int { let x = Option::Some(1); while let Option::Some(v) = x { break; } 0 }");
        let f = find_fn(&ir, "main");
        assert!(
            f.blocks.len() >= 3,
            "while-let should have multiple blocks, got {}",
            f.blocks.len()
        );
    }

    #[test]
    fn test_for_destructure() {
        let ir =
            gen("fn main() -> int { for (a, b) in vec![(1, 2), (3, 4)] { let _x = a + b; } 0 }");
        let f = find_fn(&ir, "main");
        assert!(
            f.blocks.len() >= 3,
            "for-destructure should have multiple blocks, got {}",
            f.blocks.len()
        );
    }

    #[test]
    fn test_let_pattern() {
        let ir = gen("fn main() -> int { let (x, y) = (1, 2); x + y }");
        let f = find_fn(&ir, "main");
        let ops = &f.blocks[f.entry].ops;
        assert!(
            ops.iter()
                .filter(|op| matches!(op, IrOp::StoreLocal(_, _)))
                .count()
                >= 2,
            "let-pattern should bind both vars"
        );
    }

    // ── Gaps from audit: nested closures, labeled break ────────────────

    #[test]
    fn test_closure_inside_match() {
        let ir = gen("fn main() -> int { let x = 10; let f = match 1 { 1 => || -> int { x }, _ => || -> int { 0 } }; f() }");
        let _f = find_fn(&ir, "main");
        let closures: Vec<_> = ir
            .functions
            .iter()
            .filter(|f| f.name.contains("closure"))
            .collect();
        assert!(!closures.is_empty(), "should have closure inside match");
    }

    #[test]
    fn test_cast_to_float() {
        let ir = gen("fn main() -> float { let x: float = 3; x }");
        let f = find_fn(&ir, "main");
        assert!(!f.blocks.is_empty());
    }

    #[test]
    fn test_method_with_self_param() {
        let ir = gen("struct Counter { value: int } impl Counter { fn inc(mut self) { self.value = self.value + 1 } } fn main() -> int { 0 }");
        let method = ir.functions.iter().find(|f| f.name.contains("inc"));
        assert!(method.is_some(), "should have inc method");
    }
}
