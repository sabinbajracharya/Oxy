//! Block, statement, and expression compilation.
//!
//! ```text
//! expr.rs  ── impl Compiler { compile_block, compile_stmt, compile_expr, ... }
//!   uses: mod.rs (Compiler struct), helpers.rs (free functions),
//!         visibility.rs (via self.is_visible()), sym_table.rs (SymTable)
//! ```

use std::collections::HashSet;

use crate::ast::*;
use crate::errors::FerriError;
use crate::lexer::IntegerSuffix;
use crate::types::{FloatWidth, IntegerWidth};
use crate::vm::OpCode;

use super::helpers::{
    check_literal_fits_type, emit_narrowing_cast, find_free_vars, resolve_use_path,
};
use super::{Compiler, LoopContext, SymTable};

impl Compiler {
    pub(crate) fn compile_block(&mut self, block: &Block) -> Result<(), FerriError> {
        for (i, stmt) in block.stmts.iter().enumerate() {
            let is_last = i == block.stmts.len() - 1;
            self.compile_stmt(stmt, is_last)?;
        }
        Ok(())
    }

    /// Walk up the loop_stack to find the loop matching `label`.
    /// - `None` (unlabeled) → innermost loop
    /// - `Some(name)` → first loop with that label (searching from innermost outward)
    pub(crate) fn resolve_label(&mut self, label: &Option<String>) -> Option<&mut LoopContext> {
        match label {
            None => self.loop_stack.last_mut(),
            Some(name) => self
                .loop_stack
                .iter_mut()
                .rev()
                .find(|ctx| ctx.label.as_deref() == Some(name)),
        }
    }

    pub(crate) fn compile_stmt(&mut self, stmt: &Stmt, is_last: bool) -> Result<(), FerriError> {
        match stmt {
            Stmt::Let {
                name,
                mutable,
                type_ann,
                value,
                ..
            } => {
                // `let _ = expr;` — evaluate the expression and discard the result
                if name == "_" {
                    if let Some(expr) = value {
                        self.compile_expr(expr)?;
                        self.emit(OpCode::Pop);
                    }
                    return Ok(());
                }
                if let Some(expr) = value {
                    // Check literal out-of-range before compilation
                    if let Some(TypeAnnotation::Named { name, span, .. }) = type_ann {
                        check_literal_fits_type(expr, name, *span)?;
                    }
                    self.compile_expr(expr)?;
                    // Narrow to the annotated type if it specifies a width
                    if let Some(TypeAnnotation::Named { name, .. }) = type_ann {
                        emit_narrowing_cast(self, name);
                    }
                } else {
                    self.emit(OpCode::ConstUnit);
                }
                let slot = if *mutable {
                    self.sym.define_mut(name)
                } else {
                    self.sym.define(name)
                };
                if let Some(ann) = type_ann {
                    if let Some(w) = super::integer_width_of(ann) {
                        self.sym.set_width(name, w);
                    }
                }
                self.emit(OpCode::StoreLocal(slot));
                if *mutable && self.captured_mutable.contains(name) {
                    self.emit(OpCode::MakeCell(slot));
                }
                Ok(())
            }

            Stmt::Expr {
                expr,
                has_semicolon,
            } => {
                self.compile_expr(expr)?;
                if *has_semicolon {
                    // Expression value not used, pop it
                    self.emit(OpCode::Pop);
                } else if is_last {
                    // Tail expression: leave on stack as return value
                    // Remove the implicit Return's ConstUnit if present
                }
                Ok(())
            }

            Stmt::Return { value, .. } => {
                if let Some(expr) = value {
                    self.compile_expr(expr)?;
                } else {
                    self.emit(OpCode::ConstUnit);
                }
                self.emit_return();
                Ok(())
            }

            Stmt::While {
                label,
                condition,
                body,
                ..
            } => {
                let loop_start = self.code.len();
                self.loop_stack.push(LoopContext {
                    label: label.clone(),
                    continue_target: loop_start,
                    break_patches: vec![],
                    continue_patches: vec![],
                });
                self.compile_expr(condition)?;
                let jump_out = self.emit(OpCode::JumpIfFalse(0));
                self.compile_block(body)?;
                self.emit(OpCode::Jump(loop_start));
                let loop_end = self.code.len();
                self.patch(jump_out, OpCode::JumpIfFalse(loop_end));
                let ctx = self.loop_stack.pop().unwrap();
                for idx in &ctx.break_patches {
                    self.patch(*idx, OpCode::Jump(loop_end));
                }
                for idx in &ctx.continue_patches {
                    self.patch(*idx, OpCode::Jump(loop_start));
                }
                Ok(())
            }

            Stmt::Loop { label, body, .. } => {
                let loop_start = self.code.len();
                self.loop_stack.push(LoopContext {
                    label: label.clone(),
                    continue_target: loop_start,
                    break_patches: vec![],
                    continue_patches: vec![],
                });
                self.compile_block(body)?;
                self.emit(OpCode::Jump(loop_start));
                let loop_end = self.code.len();
                let ctx = self.loop_stack.pop().unwrap();
                for idx in &ctx.break_patches {
                    self.patch(*idx, OpCode::Jump(loop_end));
                }
                for idx in &ctx.continue_patches {
                    self.patch(*idx, OpCode::Jump(ctx.continue_target));
                }
                Ok(())
            }

            Stmt::For {
                label,
                name,
                iterable,
                body,
                ..
            } => {
                let saved_sym = self.sym.clone();
                let vec_slot = self.sym.define("__for_vec");
                let idx_slot = self.sym.define("__for_idx");
                let var_slot = self.sym.define(name);

                // Preamble: evaluate iterable, materialize as Vec
                self.compile_expr(iterable)?;
                self.emit(OpCode::MakeIter);
                self.emit(OpCode::StoreLocal(vec_slot));
                self.emit(OpCode::ConstInt(0, IntegerWidth::I64));
                self.emit(OpCode::StoreLocal(idx_slot));

                // Jump to condition check on first iteration
                let jump_to_check = self.emit(OpCode::Jump(0));

                // --- Body: load current element ---
                let body_start = self.code.len();
                self.emit(OpCode::LoadLocal(vec_slot));
                self.emit(OpCode::LoadLocal(idx_slot));
                self.emit(OpCode::VecIndex);
                self.emit(OpCode::StoreLocal(var_slot));

                // Push loop context (continue_target is placeholder, set after body)
                self.loop_stack.push(LoopContext {
                    label: label.clone(),
                    continue_target: 0,
                    break_patches: vec![],
                    continue_patches: vec![],
                });

                self.compile_block(body)?;

                let ctx = self.loop_stack.pop().unwrap();

                // --- Advance: increment index (continue jumps here) ---
                let advance_start = self.code.len();
                self.emit(OpCode::LoadLocal(idx_slot));
                self.emit(OpCode::ConstInt(1, IntegerWidth::I64));
                self.emit(OpCode::Add);
                self.emit(OpCode::StoreLocal(idx_slot));

                // --- Condition check ---
                let check_start = self.code.len();
                self.emit(OpCode::LoadLocal(idx_slot));
                self.emit(OpCode::LoadLocal(vec_slot));
                self.emit(OpCode::IterLen);
                self.emit(OpCode::Lt);
                self.emit(OpCode::JumpIfTrue(body_start));

                // --- Exit ---
                let loop_end = self.code.len();
                self.patch(jump_to_check, OpCode::Jump(check_start));
                for idx in &ctx.break_patches {
                    self.patch(*idx, OpCode::Jump(loop_end));
                }
                for idx in &ctx.continue_patches {
                    self.patch(*idx, OpCode::Jump(advance_start));
                }

                self.sym = saved_sym;
                Ok(())
            }

            Stmt::Break { label, value, span } => {
                if self.loop_stack.is_empty() {
                    return Err(FerriError::Runtime {
                        message: "break outside of loop".into(),
                        line: span.line,
                        column: span.column,
                    });
                }
                if let Some(expr) = value {
                    self.compile_expr(expr)?;
                }
                let patch = self.emit(OpCode::Jump(0));
                // Walk up loop_stack to find matching label
                let target = self.resolve_label(label);
                match target {
                    Some(ctx) => ctx.break_patches.push(patch),
                    None => {
                        return Err(FerriError::Runtime {
                            message: format!(
                                "use of undeclared label `{}`",
                                label.as_deref().unwrap_or("")
                            ),
                            line: span.line,
                            column: span.column,
                        });
                    }
                }
                Ok(())
            }

            Stmt::Continue { label, span } => {
                if self.loop_stack.is_empty() {
                    return Err(FerriError::Runtime {
                        message: "continue outside of loop".into(),
                        line: span.line,
                        column: span.column,
                    });
                }
                let patch = self.emit(OpCode::Jump(0));
                let target = self.resolve_label(label);
                match target {
                    Some(ctx) => ctx.continue_patches.push(patch),
                    None => {
                        return Err(FerriError::Runtime {
                            message: format!(
                                "use of undeclared label `{}`",
                                label.as_deref().unwrap_or("")
                            ),
                            line: span.line,
                            column: span.column,
                        });
                    }
                }
                Ok(())
            }

            Stmt::ForDestructure {
                label,
                names,
                iterable,
                body,
                ..
            } => {
                let saved_sym = self.sym.clone();
                let vec_slot = self.sym.define("__for_vec");
                let idx_slot = self.sym.define("__for_idx");
                let tmp_slot = self.sym.define("__for_tmp");
                let name_slots: Vec<usize> = names.iter().map(|n| self.sym.define(n)).collect();

                // Preamble
                self.compile_expr(iterable)?;
                self.emit(OpCode::MakeIter);
                self.emit(OpCode::StoreLocal(vec_slot));
                self.emit(OpCode::ConstInt(0, IntegerWidth::I64));
                self.emit(OpCode::StoreLocal(idx_slot));
                let jump_to_check = self.emit(OpCode::Jump(0));

                // Body: load current tuple, destructure by index
                let body_start = self.code.len();
                self.emit(OpCode::LoadLocal(vec_slot));
                self.emit(OpCode::LoadLocal(idx_slot));
                self.emit(OpCode::VecIndex);
                self.emit(OpCode::StoreLocal(tmp_slot));
                for (i, &slot) in name_slots.iter().enumerate() {
                    self.emit(OpCode::LoadLocal(tmp_slot));
                    self.emit(OpCode::ConstInt(i as i64, IntegerWidth::I64));
                    self.emit(OpCode::VecIndex);
                    self.emit(OpCode::StoreLocal(slot));
                }

                self.loop_stack.push(LoopContext {
                    label: label.clone(),
                    continue_target: 0,
                    break_patches: vec![],
                    continue_patches: vec![],
                });
                self.compile_block(body)?;
                let ctx = self.loop_stack.pop().unwrap();

                let advance_start = self.code.len();
                self.emit(OpCode::LoadLocal(idx_slot));
                self.emit(OpCode::ConstInt(1, IntegerWidth::I64));
                self.emit(OpCode::Add);
                self.emit(OpCode::StoreLocal(idx_slot));

                let check_start = self.code.len();
                self.emit(OpCode::LoadLocal(idx_slot));
                self.emit(OpCode::LoadLocal(vec_slot));
                self.emit(OpCode::IterLen);
                self.emit(OpCode::Lt);
                self.emit(OpCode::JumpIfTrue(body_start));

                let loop_end = self.code.len();
                self.patch(jump_to_check, OpCode::Jump(check_start));
                for idx in &ctx.break_patches {
                    self.patch(*idx, OpCode::Jump(loop_end));
                }
                for idx in &ctx.continue_patches {
                    self.patch(*idx, OpCode::Jump(advance_start));
                }

                self.sym = saved_sym;
                Ok(())
            }

            // Statements without native bytecode — fall back to interpreter
            Stmt::WhileLet {
                pattern,
                expr,
                body,
                label,
                span: _,
            } => {
                let loop_start = self.code.len();
                // Evaluate expression, store in temp.
                self.compile_expr(expr)?;
                let scrut_slot = self.sym.define("__whilelet_scrutinee");
                let current_slot = self.sym.next_slot;
                self.emit(OpCode::StoreLocal(scrut_slot));
                // Pattern check (uniform contract: scrutinee in → [bool] out).
                self.emit(OpCode::LoadLocal(scrut_slot));
                self.compile_pattern(pattern, &mut vec![], true)?;
                let jump_to_end = self.emit(OpCode::JumpIfFalse(0));
                // Matched: reload scrutinee for bind, then body.
                self.emit(OpCode::LoadLocal(scrut_slot));
                self.bind_pattern_data(pattern)?;
                // Loop context for break/continue
                self.loop_stack.push(LoopContext {
                    label: label.clone(),
                    continue_target: loop_start,
                    break_patches: vec![],
                    continue_patches: vec![],
                });
                self.compile_block(body)?;
                let ctx = self.loop_stack.pop().unwrap();
                // Jump back to loop start
                self.emit(OpCode::Jump(loop_start));
                // End: patch exit jump
                let loop_end = self.code.len();
                self.patch(jump_to_end, OpCode::JumpIfFalse(loop_end));
                // Patch break/continue
                for idx in &ctx.break_patches {
                    self.patch(*idx, OpCode::Jump(loop_end));
                }
                for idx in &ctx.continue_patches {
                    self.patch(*idx, OpCode::Jump(loop_start));
                }
                self.sym.next_slot = current_slot;
                Ok(())
            }
            Stmt::LetPattern {
                pattern,
                value,
                span,
                mutable,
            } => {
                // Try native tuple destructuring: let (a, b, ...) = expr;
                if let Pattern::Tuple(patterns, _) = pattern.as_ref() {
                    return self.compile_destructure(value, patterns, *span);
                }
                // Try native slice destructuring: let [a, b, ...] = expr;
                if let Pattern::Slice(patterns, _) = pattern.as_ref() {
                    return self.compile_destructure(value, patterns, *span);
                }
                // For other patterns, not yet supported natively
                self.compile_letpattern_unsupported(pattern, value, *span, *mutable)
            }
            Stmt::Use(use_def) => self.compile_use(use_def),
            // Nested items are hoisted to top-level by the parser; the
            // hoisted copy is compiled via the normal `compile_item` walk,
            // so a Stmt::Item that survives into compile time is a no-op.
            Stmt::Item(_) => Ok(()),
        }
    }

    pub(crate) fn compile_expr(&mut self, expr: &Expr) -> Result<(), FerriError> {
        match expr {
            Expr::IntLiteral(n, _suffix, _span) => {
                // Literals always start life as `int` (i64). A surrounding
                // typed binding or `as` cast may narrow to byte later.
                self.emit(OpCode::ConstInt(*n, IntegerWidth::I64));
                Ok(())
            }
            Expr::FloatLiteral(n, _suffix, _) => {
                self.emit(OpCode::ConstFloat(*n, FloatWidth::F64));
                Ok(())
            }
            Expr::BoolLiteral(b, _) => {
                self.emit(OpCode::ConstBool(*b));
                Ok(())
            }
            Expr::StringLiteral(s, _) => {
                self.emit(OpCode::ConstString(s.clone()));
                Ok(())
            }
            Expr::CharLiteral(c, _) => {
                self.emit(OpCode::ConstChar(*c));
                Ok(())
            }

            Expr::Ident(name, span) => {
                // Handle bare enum variant constructors without parens
                match name.as_str() {
                    "None" => {
                        self.emit(OpCode::MakeEnumVariant {
                            enum_name: "Option".to_string(),
                            variant: "None".to_string(),
                            arg_count: 0,
                        });
                        return Ok(());
                    }
                    _ => {}
                }
                // Check const values first (compile-time inlined)
                if let Some(val) = self.const_values.get(name) {
                    match val {
                        crate::types::Value::I64(n) => {
                            self.emit(OpCode::ConstInt(*n, IntegerWidth::I64));
                        }
                        crate::types::Value::F64(n) => {
                            self.emit(OpCode::ConstFloat(*n, FloatWidth::F64));
                        }
                        crate::types::Value::Bool(b) => {
                            self.emit(OpCode::ConstBool(*b));
                        }
                        crate::types::Value::String(s) => {
                            self.emit(OpCode::ConstString(s.clone()));
                        }
                        crate::types::Value::Char(c) => {
                            self.emit(OpCode::ConstChar(*c));
                        }
                        crate::types::Value::Unit | _ => {
                            self.emit(OpCode::ConstUnit);
                        }
                    }
                    return Ok(());
                }
                if let Some(slot) = self.sym.get(name) {
                    self.emit(OpCode::LoadLocal(slot));
                    Ok(())
                } else {
                    let resolved = self
                        .use_aliases
                        .get(name)
                        .cloned()
                        .unwrap_or_else(|| name.clone());
                    if let Some(target) = self.functions.get(&resolved).copied() {
                        // Emit a function reference as a Value::Function pointing to the
                        // existing compiled function body at `target`.
                        let (params, body_expr, _return_type) =
                            self.fn_meta.get(&resolved).cloned().unwrap_or_else(|| {
                                (
                                    vec![],
                                    Box::new(crate::ast::Expr::IntLiteral(
                                        0,
                                        IntegerSuffix::None,
                                        *span,
                                    )),
                                    None,
                                )
                            });
                        let meta_idx = self.closure_meta.len();
                        let param_names: Vec<String> =
                            params.iter().map(|p| p.name.clone()).collect();
                        self.closure_meta.push((param_names, *body_expr, vec![]));
                        let is_async = self.async_fn_names.contains(&resolved);
                        self.emit(OpCode::Closure {
                            target_ip: target,
                            param_count: params.len(),
                            meta_idx,
                            is_async,
                        });
                        Ok(())
                    } else if let Some(sdef) = self.struct_defs.get(&resolved) {
                        if matches!(sdef.kind, crate::ast::StructKind::Unit) {
                            self.emit(OpCode::StructInit {
                                name: resolved,
                                field_count: 0,
                                field_names: vec![],
                            });
                            Ok(())
                        } else {
                            Err(FerriError::Runtime {
                                message: format!(
                                    "expected arguments or named fields for struct '{resolved}'"
                                ),
                                line: span.line,
                                column: span.column,
                            })
                        }
                    } else {
                        // Suggest similar variable names
                        let suggestion = self
                            .sym
                            .build_slot_names()
                            .into_iter()
                            .filter(|n| !n.is_empty())
                            .map(|n| (crate::errors::edit_distance(name, &n), n))
                            .filter(|(d, _)| *d <= 2)
                            .min_by_key(|(d, _)| *d);
                        let msg = if let Some((_, suggestion)) = suggestion {
                            format!("undefined variable '{name}'; did you mean '{suggestion}'?")
                        } else {
                            format!("undefined variable '{name}'")
                        };
                        Err(FerriError::Runtime {
                            message: msg,
                            line: span.line,
                            column: span.column,
                        })
                    }
                }
            }

            Expr::BinaryOp {
                left,
                op,
                right,
                span: _,
            } => {
                // Short-circuit && and ||
                if *op == BinOp::And {
                    self.compile_expr(left)?;
                    self.emit(OpCode::Dup); // preserve left for false case
                    let jump = self.emit(OpCode::JumpIfFalse(0));
                    self.emit(OpCode::Pop); // discard dup; left is false, keep it
                    self.compile_expr(right)?;
                    self.patch(jump, OpCode::JumpIfFalse(self.code.len()));
                    return Ok(());
                }
                if *op == BinOp::Or {
                    self.compile_expr(left)?;
                    self.emit(OpCode::Dup); // preserve left for true case
                    let jump = self.emit(OpCode::JumpIfTrue(0));
                    self.emit(OpCode::Pop); // discard left; evaluate right
                    self.compile_expr(right)?;
                    self.patch(jump, OpCode::JumpIfTrue(self.code.len()));
                    return Ok(());
                }
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                let opcode = match op {
                    BinOp::Add => OpCode::Add,
                    BinOp::Sub => OpCode::Sub,
                    BinOp::Mul => OpCode::Mul,
                    BinOp::Div => OpCode::Div,
                    BinOp::Mod => OpCode::Mod,
                    BinOp::Eq => OpCode::Eq,
                    BinOp::NotEq => OpCode::Neq,
                    BinOp::Lt => OpCode::Lt,
                    BinOp::Gt => OpCode::Gt,
                    BinOp::LtEq => OpCode::Le,
                    BinOp::GtEq => OpCode::Ge,
                    BinOp::BitAnd => OpCode::BitAnd,
                    BinOp::BitOr => OpCode::BitOr,
                    BinOp::BitXor => OpCode::BitXor,
                    BinOp::Shl => OpCode::Shl,
                    BinOp::Shr => OpCode::Shr,
                    BinOp::And | BinOp::Or => unreachable!(),
                };
                self.emit(opcode);
                Ok(())
            }

            Expr::UnaryOp {
                op,
                expr: inner,
                span,
            } => {
                self.compile_expr(inner)?;
                match op {
                    UnaryOp::Neg => self.emit(OpCode::Neg),
                    UnaryOp::Not => self.emit(OpCode::Not),
                    UnaryOp::BitNot => self.emit(OpCode::BitNot),
                    #[allow(unreachable_patterns)]
                    _ => {
                        return Err(FerriError::Runtime {
                            message: format!("unsupported unary op in compiler: {:?}", op),
                            line: span.line,
                            column: span.column,
                        })
                    }
                };
                Ok(())
            }

            Expr::Call {
                callee,
                turbofish,
                args,
                span,
            } => {
                // Handle bare enum constructors: Some(val), None, Ok(val), Err(val)
                if let Expr::Ident(name, _) = callee.as_ref() {
                    let enum_info: Option<(&str, &str)> = match name.as_str() {
                        "Some" => Some(("Option", "Some")),
                        "None" => Some(("Option", "None")),
                        "Ok" => Some(("Result", "Ok")),
                        "Err" => Some(("Result", "Err")),
                        _ => None,
                    };
                    if let Some((enum_name, variant)) = enum_info {
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit(OpCode::MakeEnumVariant {
                            enum_name: enum_name.to_string(),
                            variant: variant.to_string(),
                            arg_count: args.len(),
                        });
                        return Ok(());
                    }
                }

                // Determine if this is a direct function call (compile-time resolved)
                let direct_target: Option<usize> = if let Expr::Ident(name, _) = callee.as_ref() {
                    if name == "println!" || name == "print!" {
                        // Compile args first, then emit print
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        if name == "println!" {
                            self.emit(OpCode::PrintLn);
                        } else {
                            self.emit(OpCode::Print);
                        }
                        // Macro call is an expression — leave a Unit on the
                        // stack so the surrounding statement's Pop has a
                        // value to discard (otherwise Pop dips into the
                        // caller's frame).
                        self.emit(OpCode::ConstUnit);
                        return Ok(());
                    }
                    // spawn(closure) → run closure eagerly, wrap in JoinHandle
                    if name == "spawn" {
                        if args.len() != 1 {
                            return Err(FerriError::Runtime {
                                message: "spawn expects exactly 1 argument (a closure)".to_string(),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        if !matches!(args[0], crate::ast::Expr::Closure { .. }) {
                            return Err(FerriError::Runtime {
                                message: "spawn expects a closure argument".to_string(),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        self.compile_expr(&args[0])?;
                        self.emit(OpCode::Spawn);
                        return Ok(());
                    }
                    // sleep(ms) → pause for ms milliseconds
                    if name == "sleep" {
                        if args.len() != 1 {
                            return Err(FerriError::Runtime {
                                message: "sleep expects exactly 1 argument (milliseconds)"
                                    .to_string(),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        self.compile_expr(&args[0])?;
                        self.emit(OpCode::Sleep);
                        return Ok(());
                    }
                    // select(h1, h2, ...) → race multiple JoinHandles, return first result
                    if name == "select" {
                        if args.len() < 2 {
                            return Err(FerriError::Runtime {
                                message: "select expects at least 2 arguments (JoinHandles)"
                                    .to_string(),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit(OpCode::Select { count: args.len() });
                        return Ok(());
                    }
                    // Follow use_aliases chain (handles pub use re-exports)
                    let mut resolved = name.clone();
                    let mut seen: HashSet<&str> = HashSet::new();
                    while let Some(alias_target) = self.use_aliases.get(&resolved) {
                        if !seen.insert(alias_target) {
                            break; // cycle guard
                        }
                        resolved = alias_target.clone();
                    }
                    let result = self
                        .functions
                        .get(&resolved)
                        .copied()
                        .or_else(|| self.functions.get(name).copied());
                    // If not found directly, try module-qualified name
                    result.or_else(|| {
                        let module_prefix = self.module_stack.join("::");
                        if module_prefix.is_empty() {
                            return None;
                        }
                        let qualified = format!("{}::{}", module_prefix, name);
                        self.functions.get(&qualified).copied()
                    })
                } else {
                    None
                };

                if let Some(target) = direct_target {
                    // Visibility check: reject calls to private functions resolved through use aliases
                    if let Expr::Ident(name, _) = callee.as_ref() {
                        let mut resolved = name.clone();
                        let mut seen: HashSet<&str> = HashSet::new();
                        while let Some(alias_target) = self.use_aliases.get(&resolved) {
                            if !seen.insert(alias_target) {
                                break; // cycle guard
                            }
                            resolved = alias_target.clone();
                        }
                        if resolved != *name
                            && self.functions.contains_key(&resolved)
                            && !self.is_visible(&resolved)
                        {
                            return Err(FerriError::Runtime {
                                message: format!("`{}` is private", resolved),
                                line: span.line,
                                column: span.column,
                            });
                        }
                    }
                    // Check if we should monomorphize: turbofish present with all concrete types
                    let concrete_types: Option<Vec<String>> = turbofish.as_ref().map(|tf| {
                        tf.iter()
                            .map(|ta| ta.name().to_string())
                            .filter(|n| n != "_")
                            .collect()
                    });
                    let should_monomorphize = concrete_types
                        .as_ref()
                        .map(|ct| !ct.is_empty())
                        .unwrap_or(false);

                    if should_monomorphize {
                        // Monomorphize: compile a copy with concrete types substituted
                        if let Expr::Ident(name, _) = callee.as_ref() {
                            let resolved = self
                                .use_aliases
                                .get(name)
                                .cloned()
                                .unwrap_or_else(|| name.clone());
                            let type_args = concrete_types.unwrap();
                            let mono_target = self.monomorphize_call(
                                &resolved,
                                &type_args,
                                span.line,
                                span.column,
                            )?;
                            // Compile args and emit call to monomorphized version
                            for arg in args {
                                self.compile_expr(arg)?;
                            }
                            self.emit(OpCode::Call {
                                target: mono_target,
                                arg_count: args.len(),
                            });
                            return Ok(());
                        }
                    }

                    // Check argument count against function definition
                    if let Expr::Ident(name, _) = callee.as_ref() {
                        let resolved = self
                            .use_aliases
                            .get(name)
                            .cloned()
                            .unwrap_or_else(|| name.clone());
                        if let Some((params, _, _)) = self
                            .fn_meta
                            .get(&resolved)
                            .or_else(|| self.fn_meta.get(name))
                        {
                            if args.len() != params.len() {
                                return Err(FerriError::Runtime {
                                    message: format!(
                                        "function '{}' expects {} argument{}, but {} {} provided",
                                        resolved,
                                        params.len(),
                                        if params.len() == 1 { "" } else { "s" },
                                        args.len(),
                                        if args.len() == 1 { "was" } else { "were" },
                                    ),
                                    line: span.line,
                                    column: span.column,
                                });
                            }
                        }
                    }
                    // Check if calling an async function → emit MakeFuture instead of Call
                    if let Expr::Ident(name, _) = callee.as_ref() {
                        let resolved = self
                            .use_aliases
                            .get(name)
                            .cloned()
                            .unwrap_or_else(|| name.clone());
                        if self.async_fn_names.contains(&resolved)
                            || self.async_fn_names.contains(name)
                        {
                            for arg in args {
                                self.compile_expr(arg)?;
                            }
                            self.emit(OpCode::MakeFuture {
                                target_ip: target,
                                arg_count: args.len(),
                            });
                            return Ok(());
                        }
                    }
                    // Direct call: compile args first, emit Call
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    let call_idx = self.emit(OpCode::Call {
                        target,
                        arg_count: args.len(),
                    });
                    // Forward reference: record for patching after all functions compiled
                    if target == usize::MAX {
                        if let Expr::Ident(name, _) = callee.as_ref() {
                            let resolved = self
                                .use_aliases
                                .get(name)
                                .cloned()
                                .unwrap_or_else(|| name.clone());
                            self.forward_calls.push((call_idx, resolved));
                        }
                    }
                } else if let Expr::Ident(name, _) = callee.as_ref() {
                    // Not a known function — check if it's a tuple struct constructor.
                    let resolved = self
                        .type_aliases
                        .get(name)
                        .cloned()
                        .or_else(|| self.use_aliases.get(name).cloned())
                        .unwrap_or_else(|| name.clone());
                    // Also try Self -> current_impl_type
                    let resolved = if resolved == "Self" {
                        self.current_impl_type.clone().unwrap_or(resolved)
                    } else {
                        resolved
                    };
                    // Clone struct kind before mutable borrows below
                    let struct_kind = self.struct_defs.get(&resolved).map(|sd| sd.kind.clone());
                    if let Some(kind) = struct_kind {
                        match &kind {
                            crate::ast::StructKind::Tuple(type_anns) => {
                                if args.len() != type_anns.len() {
                                    return Err(FerriError::Runtime {
                                        message: format!(
                                            "tuple struct '{}' expects {} field{}, but {} {} provided",
                                            resolved,
                                            type_anns.len(),
                                            if type_anns.len() == 1 { "" } else { "s" },
                                            args.len(),
                                            if args.len() == 1 { "was" } else { "were" },
                                        ),
                                        line: callee.span().line,
                                        column: callee.span().column,
                                    });
                                }
                                for arg in args {
                                    self.compile_expr(arg)?;
                                }
                                let field_names: Vec<String> =
                                    (0..type_anns.len()).map(|i| i.to_string()).collect();
                                self.emit(OpCode::StructInit {
                                    name: resolved,
                                    field_count: type_anns.len(),
                                    field_names,
                                });
                                return Ok(());
                            }
                            crate::ast::StructKind::Unit => {
                                if !args.is_empty() {
                                    return Err(FerriError::Runtime {
                                        message: format!(
                                            "unit struct '{}' does not take arguments",
                                            resolved
                                        ),
                                        line: callee.span().line,
                                        column: callee.span().column,
                                    });
                                }
                                self.emit(OpCode::StructInit {
                                    name: resolved,
                                    field_count: 0,
                                    field_names: vec![],
                                });
                                return Ok(());
                            }
                            _ => {
                                return Err(FerriError::Runtime {
                                    message: format!(
                                        "struct '{}' has named fields; use {} {{ field: value }} syntax",
                                        resolved, resolved
                                    ),
                                    line: callee.span().line,
                                    column: callee.span().column,
                                });
                            }
                        }
                    }
                    // Also check enum variant constructors: Type::Variant(args)
                    if resolved.contains("::") {
                        let parts: Vec<&str> = resolved.split("::").collect();
                        if parts.len() == 2 {
                            let enum_name = parts[0].to_string();
                            let variant = parts[1].to_string();
                            if self.enum_defs.contains_key(&enum_name) {
                                for arg in args {
                                    self.compile_expr(arg)?;
                                }
                                self.emit(OpCode::MakeEnumVariant {
                                    enum_name,
                                    variant,
                                    arg_count: args.len(),
                                });
                                return Ok(());
                            }
                        }
                    }
                    // Check if the resolved name (through use aliases) is a
                    // built-in path like `std::db::open`. This lets
                    // `use std::db::open as db_open; db_open(...)` work.
                    if resolved.contains("::") {
                        let segments: Vec<String> =
                            resolved.split("::").map(|s| s.to_string()).collect();
                        if super::helpers::is_builtin_path(&segments) {
                            for arg in args {
                                self.compile_expr(arg)?;
                            }
                            self.emit(OpCode::PathCallBuiltin {
                                segments,
                                arg_count: args.len(),
                            });
                            return Ok(());
                        }
                    }
                    // Fall through to dynamic CallClosure
                    self.compile_expr(callee)?;
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    self.emit(OpCode::CallClosure {
                        arg_count: args.len(),
                    });
                    return Ok(());
                } else {
                    // Unknown at compile time — emit dynamic call via CallClosure.
                    // This handles closures from variables (|x|), array indexing (arr[0]),
                    // field access (obj.func), and other dynamic expressions.
                    self.compile_expr(callee)?;
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    self.emit(OpCode::CallClosure {
                        arg_count: args.len(),
                    });
                    return Ok(());
                }
                Ok(())
            }

            Expr::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.compile_expr(condition)?;
                let jump_else = self.emit(OpCode::JumpIfFalse(0)); // placeholder
                self.compile_block(then_block)?;
                let jump_end = if else_block.is_some() {
                    Some(self.emit(OpCode::Jump(0))) // placeholder
                } else {
                    None
                };
                // Patch jump_else to point here (after then_block)
                let after_then = self.code.len();
                self.patch(jump_else, OpCode::JumpIfFalse(after_then));
                if let Some(else_expr) = else_block {
                    self.compile_expr(else_expr)?;
                    let after_else = self.code.len();
                    self.patch(jump_end.unwrap(), OpCode::Jump(after_else));
                }
                Ok(())
            }

            Expr::Block(block) => self.compile_block(block),

            Expr::Grouped(inner, _) => self.compile_expr(inner),

            Expr::Assign {
                target,
                value,
                span,
            } => {
                if let Expr::Ident(name, _) = target.as_ref() {
                    // Check immutability: variable already defined but not mutable
                    if self.sym.get(name).is_some() && !self.sym.is_mutable(name) {
                        return Err(FerriError::Runtime {
                            message: format!("cannot assign to immutable variable `{name}`"),
                            line: span.line,
                            column: span.column,
                        });
                    }
                    self.compile_expr(value)?;
                    if let Some(w) = self.sym.width_of(name) {
                        self.emit(OpCode::CastInt(w));
                    }
                    if let Some(slot) = self.sym.get(name) {
                        self.emit(OpCode::Dup);
                        self.emit(OpCode::StoreLocal(slot));
                    } else {
                        let slot = self.sym.define(name);
                        self.emit(OpCode::Dup);
                        self.emit(OpCode::StoreLocal(slot));
                    }
                    Ok(())
                } else if let Expr::FieldAccess { object, field, .. } = target.as_ref() {
                    // Field assignment: mutating a field is mutation of the binding
                    // itself in Oxy's value semantics (no aliasing), so the binding
                    // must be `mut`. Reject `let x = ...; x.field = ...` and
                    // `fn method(self) { self.field = ... }`; permit `let mut x` and
                    // `fn method(mut self) { ... }`.
                    match object.as_ref() {
                        Expr::Ident(name, _) => {
                            if self.sym.get(name).is_some() && !self.sym.is_mutable(name) {
                                return Err(FerriError::Runtime {
                                    message: format!(
                                        "cannot assign to field of immutable variable `{name}` — declare it `let mut {name}` or `mut {name}: T`"
                                    ),
                                    line: span.line,
                                    column: span.column,
                                });
                            }
                        }
                        Expr::SelfRef(_) => {
                            if !self.sym.is_mutable("self") {
                                return Err(FerriError::Runtime {
                                    message:
                                        "cannot assign to field of immutable `self` — declare the method receiver as `mut self`"
                                            .into(),
                                    line: span.line,
                                    column: span.column,
                                });
                            }
                        }
                        _ => {}
                    }
                    self.compile_expr(object)?;
                    self.compile_expr(value)?;
                    self.emit(OpCode::FieldStore(field.clone()));
                    // Store the updated struct back to the original binding.
                    match object.as_ref() {
                        Expr::Ident(name, _) => {
                            if let Some(slot) = self.sym.get(name) {
                                self.emit(OpCode::Dup);
                                self.emit(OpCode::StoreLocal(slot));
                            }
                        }
                        Expr::SelfRef(_) => {
                            // self is always at slot 0 in methods
                            self.emit(OpCode::Dup);
                            self.emit(OpCode::StoreLocal(0));
                        }
                        _ => {}
                    }
                    Ok(())
                } else if let Expr::Index { object, index, .. } = target.as_ref() {
                    self.compile_expr(object)?;
                    self.compile_expr(index)?;
                    self.compile_expr(value)?;
                    self.emit(OpCode::VecIndexStore);
                    Ok(())
                } else {
                    Err(FerriError::Runtime {
                        message: "compiled: only simple variable assignment supported".into(),
                        line: span.line,
                        column: span.column,
                    })
                }
            }

            Expr::CompoundAssign {
                target,
                op,
                value,
                span,
            } => {
                if let Expr::Ident(name, _) = target.as_ref() {
                    if let Some(slot) = self.sym.get(name) {
                        if !self.sym.is_mutable(name) {
                            return Err(FerriError::Runtime {
                                message: format!("cannot assign to immutable variable `{name}`"),
                                line: span.line,
                                column: span.column,
                            });
                        }
                        self.emit(OpCode::LoadLocal(slot));
                        self.compile_expr(value)?;
                        let opcode = match op {
                            BinOp::Add => OpCode::Add,
                            BinOp::Sub => OpCode::Sub,
                            BinOp::Mul => OpCode::Mul,
                            BinOp::Div => OpCode::Div,
                            BinOp::Mod => OpCode::Mod,
                            _ => {
                                return Err(FerriError::Runtime {
                                    message: format!(
                                        "unsupported compound op in compiler: {:?}",
                                        op
                                    ),
                                    line: span.line,
                                    column: span.column,
                                })
                            }
                        };
                        self.emit(opcode);
                        if let Some(w) = self.sym.width_of(name) {
                            self.emit(OpCode::CastInt(w));
                        }
                        self.emit(OpCode::StoreLocal(slot));
                        Ok(())
                    } else {
                        Err(FerriError::Runtime {
                            message: format!("compiled: undefined variable '{}'", name),
                            line: span.line,
                            column: span.column,
                        })
                    }
                } else {
                    Err(self.not_yet_supported("Compound assign on field/index", expr.span()))
                }
            }

            Expr::Try { expr: inner, .. } => {
                self.compile_expr(inner)?;
                self.emit(OpCode::TryPop);
                Ok(())
            }

            Expr::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                if let Some(s) = start {
                    self.compile_expr(s)?;
                } else {
                    self.emit(OpCode::ConstInt(i64::MIN, IntegerWidth::I64));
                }
                if let Some(e) = end {
                    self.compile_expr(e)?;
                } else {
                    self.emit(OpCode::ConstInt(i64::MAX, IntegerWidth::I64));
                }
                if *inclusive {
                    self.emit(OpCode::ConstInt(1, IntegerWidth::I64));
                    self.emit(OpCode::Add);
                }
                self.emit(OpCode::MakeRange);
                Ok(())
            }

            Expr::Repeat {
                value, count, span, ..
            } => {
                let n = match crate::compiler::helpers::try_eval_const(count) {
                    Some(crate::types::Value::I64(n)) if n >= 0 => n as usize,
                    _ => {
                        return Err(crate::errors::FerriError::Runtime {
                            message: "array repeat count must be a non-negative integer constant"
                                .into(),
                            line: span.line,
                            column: span.column,
                        });
                    }
                };
                self.compile_expr(value)?;
                for _ in 1..n {
                    self.emit(OpCode::Dup);
                }
                self.emit(OpCode::MakeFixedArray { count: n });
                Ok(())
            }
            Expr::Array { elements, .. } => {
                let count = elements.len();
                for elem in elements {
                    self.compile_expr(elem)?;
                }
                self.emit(OpCode::MakeArray { count });
                Ok(())
            }

            Expr::Tuple { elements, .. } => {
                let count = elements.len();
                for elem in elements {
                    self.compile_expr(elem)?;
                }
                self.emit(OpCode::MakeTuple { count });
                Ok(())
            }

            Expr::Index { object, index, .. } => {
                self.compile_expr(object)?;
                self.compile_expr(index)?;
                self.emit(OpCode::VecIndex);
                Ok(())
            }

            Expr::FieldAccess { object, field, .. } => {
                self.compile_expr(object)?;
                if let Ok(idx) = field.parse::<i64>() {
                    self.emit(OpCode::ConstInt(idx, IntegerWidth::I64));
                    self.emit(OpCode::VecIndex);
                } else {
                    self.emit(OpCode::FieldAccess {
                        field_name: field.clone(),
                    });
                }
                Ok(())
            }

            Expr::FString { parts, .. } => {
                let mut count = 0usize;
                for part in parts {
                    match part {
                        FStringPart::Literal(s) => {
                            self.emit(OpCode::ConstString(s.clone()));
                            count += 1;
                        }
                        FStringPart::Expr(expr) => {
                            self.compile_expr(expr)?;
                            self.emit(OpCode::ToString);
                            count += 1;
                        }
                    }
                }
                self.emit(OpCode::FStringConcat { count });
                Ok(())
            }

            Expr::SelfRef(_) => {
                // `self` is always the first parameter → slot 0.
                self.emit(OpCode::LoadLocal(0));
                Ok(())
            }

            Expr::StructInit {
                name, fields, base, ..
            } => {
                // Resolve `Self` to the current impl type name, then type aliases, then use aliases
                let mut resolved_name = if name == "Self" {
                    self.current_impl_type
                        .clone()
                        .unwrap_or_else(|| name.clone())
                } else {
                    self.type_aliases
                        .get(name)
                        .cloned()
                        .or_else(|| self.use_aliases.get(name).cloned())
                        .unwrap_or_else(|| name.clone())
                };
                // Try module-qualified name for unqualified structs in the current module
                if !resolved_name.contains("::") {
                    let module_prefix = self.module_stack.join("::");
                    if !module_prefix.is_empty() {
                        let qualified = format!("{}::{}", module_prefix, resolved_name);
                        if self.struct_defs.contains_key(&qualified) {
                            resolved_name = qualified;
                        }
                    }
                }
                // Check if this is an enum variant constructor (e.g. Message::Move { x, y })
                if resolved_name.contains("::") {
                    let parts: Vec<&str> = resolved_name.split("::").collect();
                    if parts.len() == 2 {
                        let enum_name = parts[0].to_string();
                        let variant = parts[1].to_string();
                        if self.enum_defs.contains_key(&enum_name) {
                            // Compile field values in order
                            for (_, expr) in fields {
                                self.compile_expr(expr)?;
                            }
                            self.emit(OpCode::MakeEnumVariant {
                                enum_name,
                                variant,
                                arg_count: fields.len(),
                            });
                            return Ok(());
                        }
                    }
                }
                // Check struct visibility — private structs can't be constructed from outside
                if !self.is_visible(&resolved_name) {
                    return Err(FerriError::Runtime {
                        message: format!("`{}` is private", resolved_name),
                        line: expr.span().line,
                        column: expr.span().column,
                    });
                }
                let field_names: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();
                for (field_name, field_expr) in fields {
                    self.check_field_visibility(&resolved_name, field_name, field_expr.span())?;
                    self.compile_expr(field_expr)?;
                }
                if let Some(base_expr) = base {
                    // Push base on top of explicit fields; VM merges them.
                    self.compile_expr(base_expr)?;
                    self.emit(OpCode::StructUpdate {
                        name: resolved_name,
                        field_count: fields.len(),
                        field_names,
                    });
                } else {
                    self.emit(OpCode::StructInit {
                        name: resolved_name,
                        field_count: fields.len(),
                        field_names,
                    });
                }
                Ok(())
            }

            Expr::MethodCall {
                object,
                method,
                args,
                ..
            } => {
                // If the receiver is a local variable, check if this is an
                // &mut self method so we can write the result back.
                let receiver_slot = if let Expr::Ident(name, _) = object.as_ref() {
                    self.sym.get(name).filter(|_| {
                        // Only write back for &mut self methods (return_type is None
                        // and first param is "self").
                        self.fn_meta.get(method).map_or(false, |(params, _, ret)| {
                            ret.is_none() && params.first().map_or(false, |p| p.name == "self")
                        })
                    })
                } else {
                    None
                };
                self.compile_expr(object)?;
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit(OpCode::MethodCall {
                    method_name: method.clone(),
                    arg_count: args.len(),
                });
                if let Some(slot) = receiver_slot {
                    self.emit(OpCode::Dup);
                    self.emit(OpCode::StoreLocal(slot));
                }
                Ok(())
            }

            Expr::Path { segments, .. } => {
                if segments.len() == 2 {
                    let enum_name = &segments[0];
                    let variant = &segments[1];
                    // Resolve via type aliases and use aliases
                    let resolved_enum = self
                        .type_aliases
                        .get(enum_name)
                        .cloned()
                        .or_else(|| self.use_aliases.get(enum_name).cloned())
                        .unwrap_or_else(|| enum_name.clone());
                    let enum_key = self
                        .enum_defs
                        .get(enum_name)
                        .or_else(|| self.enum_defs.get(&resolved_enum));
                    if let Some(ed) = enum_key {
                        for v in &ed.variants {
                            if &v.name == variant {
                                self.emit(OpCode::ConstEnumVariant {
                                    enum_name: resolved_enum.clone(),
                                    variant: variant.clone(),
                                    data: vec![],
                                });
                                return Ok(());
                            }
                        }
                    }
                    if enum_name == "math" {
                        match variant.as_str() {
                            "PI" => {
                                self.emit(OpCode::ConstFloat(
                                    std::f64::consts::PI,
                                    FloatWidth::F64,
                                ));
                                return Ok(());
                            }
                            "E" => {
                                self.emit(OpCode::ConstFloat(std::f64::consts::E, FloatWidth::F64));
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                }
                // 3+ segments: try `prefix::Enum::Variant` where the leading
                // segments form a (possibly nested) module path and the last
                // two are the enum + variant. e.g. `shapes::Color::Green`
                // resolves against the enum registered as `shapes::Color`.
                if segments.len() >= 3 {
                    let variant = segments.last().unwrap().clone();
                    let qualified_enum = segments[..segments.len() - 1].join("::");
                    if let Some(ed) = self.enum_defs.get(&qualified_enum).cloned() {
                        if ed.variants.iter().any(|v| v.name == variant) {
                            self.emit(OpCode::ConstEnumVariant {
                                enum_name: qualified_enum,
                                variant,
                                data: vec![],
                            });
                            return Ok(());
                        }
                    }
                }
                Err(self.not_yet_supported("Unknown path", expr.span()))
            }

            Expr::PathCall {
                path,
                turbofish,
                args,
                ..
            } => {
                use super::path_resolution::PathResolution;

                for arg in args {
                    self.compile_expr(arg)?;
                }
                // Resolve self/super/crate in path prefix
                let resolved_path = resolve_use_path(path, &self.module_stack);
                let path = if resolved_path != *path {
                    &resolved_path
                } else {
                    path
                };

                match self.resolve_path_call(path, turbofish) {
                    PathResolution::EnumVariant { enum_name, variant } => {
                        self.emit(OpCode::MakeEnumVariant {
                            enum_name,
                            variant,
                            arg_count: args.len(),
                        });
                        Ok(())
                    }
                    PathResolution::Function {
                        qualified,
                        target,
                        is_direct,
                    } => {
                        if is_direct {
                            self.check_path_visible_with_leaf(path, &qualified, expr.span())?;
                        } else if !self.is_visible(&qualified) {
                            return Err(FerriError::Runtime {
                                message: format!("`{}` is private", qualified),
                                line: expr.span().line,
                                column: expr.span().column,
                            });
                        }
                        let call_idx = self.emit(OpCode::Call {
                            target,
                            arg_count: args.len(),
                        });
                        if target == usize::MAX {
                            self.forward_calls.push((call_idx, qualified));
                        }
                        Ok(())
                    }
                    PathResolution::Builtin => {
                        self.emit(OpCode::PathCallBuiltin {
                            segments: path.clone(),
                            arg_count: args.len(),
                        });
                        Ok(())
                    }
                    PathResolution::GenericPlaceholder {
                        type_param,
                        method_name,
                    } => {
                        // Generic body never runs directly — emit a panic
                        // pointing the user at turbofish monomorphization.
                        self.emit(OpCode::ConstString(format!(
                            "generic function called without monomorphization: \
                             cannot resolve `{}::{}()` without a concrete type; \
                             use turbofish: `func::<Type>(args)`",
                            type_param, method_name,
                        )));
                        self.emit(OpCode::Panic);
                        // Stack balance: Return expects one value.
                        self.emit(OpCode::ConstUnit);
                        Ok(())
                    }
                    PathResolution::Unknown => {
                        Err(self.not_yet_supported("Unknown path call", expr.span()))
                    }
                }
            }

            Expr::Closure {
                params,
                body,
                is_async,
                ..
            } => {
                // Emit a jump to skip over the closure body in the instruction stream
                let skip_jump_idx = self.emit(OpCode::Jump(0));
                let target_ip = self.code.len();
                // Swap in a fresh sym table. Captures get dense slots 0..N first,
                // then params at N..N+param_count, then body locals after — so the
                // closure's frame size is independent of the parent's local count.
                let saved_sym = std::mem::replace(&mut self.sym, SymTable::new(0));
                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                let captured_free = find_free_vars(body, &param_names);
                // Build the captured list. `outer_slot` is the parent's slot for
                // fetching the value at OpCode::Closure time; the child places it
                // densely at the index it appears in this vec.
                let captured: Vec<(String, usize, bool)> = captured_free
                    .iter()
                    .filter_map(|name| {
                        saved_sym.get(name).map(|outer_slot| {
                            let is_mut = saved_sym.is_mutable(name);
                            // Define each capture at the next dense slot. Order
                            // here matches the dense placement done by the VM.
                            self.sym.define(name);
                            if is_mut {
                                self.sym.mutable.insert(name.clone());
                            }
                            (name.clone(), outer_slot, is_mut)
                        })
                    })
                    .collect();
                // Params get the next dense slots after captures.
                // Closures don't have surface `mut` syntax on params yet.
                for param in params {
                    self.sym.define(&param.name);
                }
                self.compile_expr(body)?;
                self.emit(OpCode::Return);
                self.fn_frame_sizes.insert(target_ip, self.sym.next_slot);
                self.sym = saved_sym;
                // Patch the skip jump to land after the Return
                self.patch(skip_jump_idx, OpCode::Jump(self.code.len()));
                let meta_idx = self.closure_meta.len();
                self.closure_meta
                    .push((param_names, *body.clone(), captured));
                self.emit(OpCode::Closure {
                    target_ip,
                    param_count: params.len(),
                    meta_idx,
                    is_async: *is_async,
                });
                Ok(())
            }

            Expr::AsyncBlock { body, .. } => {
                let skip_jump_idx = self.emit(OpCode::Jump(0));
                let target_ip = self.code.len();
                let saved_sym = std::mem::replace(&mut self.sym, SymTable::new(0));
                let param_names: Vec<String> = vec![];
                let captured_free = find_free_vars(&Expr::Block(body.clone()), &param_names);
                let captured: Vec<(String, usize, bool)> = captured_free
                    .iter()
                    .filter_map(|name| {
                        saved_sym.get(name).map(|outer_slot| {
                            let is_mut = saved_sym.is_mutable(name);
                            self.sym.define(name);
                            if is_mut {
                                self.sym.mutable.insert(name.clone());
                            }
                            (name.clone(), outer_slot, is_mut)
                        })
                    })
                    .collect();
                let stmt_count = body.stmts.len();
                for (i, stmt) in body.stmts.iter().enumerate() {
                    self.compile_stmt(stmt, i == stmt_count - 1)?;
                }
                let has_tail = body.stmts.last().map_or(false, |s| {
                    matches!(
                        s,
                        Stmt::Expr {
                            has_semicolon: false,
                            ..
                        }
                    )
                });
                if !has_tail {
                    self.emit(OpCode::ConstUnit);
                }
                self.emit(OpCode::Return);
                self.fn_frame_sizes.insert(target_ip, self.sym.next_slot);
                self.sym = saved_sym;
                self.patch(skip_jump_idx, OpCode::Jump(self.code.len()));
                let meta_idx = self.closure_meta.len();
                self.closure_meta
                    .push((param_names, Expr::Block(body.clone()), captured));
                self.emit(OpCode::AsyncBlock {
                    target_ip,
                    meta_idx,
                });
                Ok(())
            }

            Expr::Match {
                expr: scrutinee,
                arms,
                ..
            } => {
                // Exhaustiveness check: require wildcard for integer literal matches.
                // Enum variants, bool, and ident patterns are fine without catch-all.
                let has_catch_all = arms
                    .iter()
                    .any(|a| matches!(a.pattern, Pattern::Wildcard(_) | Pattern::Ident(..)));
                let has_enum = arms
                    .iter()
                    .any(|a| matches!(a.pattern, Pattern::EnumVariant { .. }));
                let has_int_literal = arms
                    .iter()
                    .any(|a| matches!(a.pattern, Pattern::Literal(Expr::IntLiteral(..))));
                if !has_catch_all && !has_enum && has_int_literal {
                    return Err(FerriError::Runtime {
                        message: "non-exhaustive patterns: missing wildcard `_` arm".into(),
                        line: expr.span().line,
                        column: expr.span().column,
                    });
                }
                // Evaluate scrutinee once, store in temp slot.
                self.compile_expr(scrutinee)?;
                let scrutinee_slot = self.sym.define("__match_scrutinee");
                let current_slot = self.sym.next_slot;
                self.emit(OpCode::StoreLocal(scrutinee_slot));

                let mut arm_jumps: Vec<usize> = vec![];

                // Stack contract per arm (under the uniform pattern protocol):
                //   pre-pattern : []           — previous arm's failed branch
                //                                always leaves the stack clean
                //   load scrut  : [scrut]
                //   compile_pat : [bool]       — scrutinee always consumed
                //   JumpIfFalse : []           — on fail-jump, stack is []
                //   load scrut  : [scrut]      — reload for binding
                //   bind        : []           — bind_pattern_data consumes
                //   guard?      : [bool] → []  — JumpIfFalse consumes
                //   body        : [result]
                //   Jump to end
                for arm in arms.iter() {
                    self.emit(OpCode::LoadLocal(scrutinee_slot));
                    self.compile_pattern(&arm.pattern, &mut vec![], false)?;
                    let jump_to_next = self.emit(OpCode::JumpIfFalse(0));

                    self.emit(OpCode::LoadLocal(scrutinee_slot));
                    self.bind_pattern_data(&arm.pattern)?;

                    let guard_fail_jump = if let Some(guard) = &arm.guard {
                        self.compile_expr(guard)?;
                        Some(self.emit(OpCode::JumpIfFalse(0)))
                    } else {
                        None
                    };

                    self.compile_expr(&arm.body)?;
                    arm_jumps.push(self.emit(OpCode::Jump(0)));

                    // Pattern-fail and guard-fail both land at the next arm
                    // (or the no-match epilogue for the last arm).
                    let next_arm = self.code.len();
                    self.patch(jump_to_next, OpCode::JumpIfFalse(next_arm));
                    if let Some(gj) = guard_fail_jump {
                        self.patch(gj, OpCode::JumpIfFalse(next_arm));
                    }

                    // Clear arm-local bindings.
                    self.sym.next_slot = current_slot;
                }

                // Unreachable in exhaustive matches (checker enforces a
                // wildcard or full coverage), but emit a sentinel for safety.
                self.emit(OpCode::ConstString("match: no arm matched".into()));
                self.emit(OpCode::PrintLn);
                self.emit(OpCode::ConstUnit);

                let end = self.code.len();
                for j in &arm_jumps {
                    self.patch(*j, OpCode::Jump(end));
                }

                Ok(())
            }

            Expr::IfLet {
                pattern,
                expr: scrutinee,
                guard,
                then_block,
                else_block,
                ..
            } => {
                // Evaluate scrutinee, store in temp.
                self.compile_expr(scrutinee)?;
                let scrut_slot = self.sym.define("__iflet_scrutinee");
                let current_slot = self.sym.next_slot;
                self.emit(OpCode::StoreLocal(scrut_slot));

                // Pattern check (uniform contract: scrutinee in → [bool] out).
                self.emit(OpCode::LoadLocal(scrut_slot));
                self.compile_pattern(pattern, &mut vec![], true)?;
                let jump_pattern_fail = self.emit(OpCode::JumpIfFalse(0));

                // Matched: bind pattern variables (guard and body can use them).
                self.emit(OpCode::LoadLocal(scrut_slot));
                self.bind_pattern_data(pattern)?;

                // Optional `&&` guard — evaluated after binding so bound vars are live.
                let jump_guard_fail = if let Some(guard_expr) = guard {
                    self.compile_expr(guard_expr)?;
                    Some(self.emit(OpCode::JumpIfFalse(0)))
                } else {
                    None
                };

                self.compile_block(then_block)?;

                // Jump over else block
                let jump_to_end = self.emit(OpCode::Jump(0));

                // Else block — both pattern-miss and guard-fail land here.
                let else_start = self.code.len();
                self.patch(jump_pattern_fail, OpCode::JumpIfFalse(else_start));
                if let Some(jgf) = jump_guard_fail {
                    self.patch(jgf, OpCode::JumpIfFalse(else_start));
                }
                self.sym.next_slot = current_slot;
                if let Some(else_expr) = else_block {
                    self.compile_expr(else_expr)?;
                } else {
                    self.emit(OpCode::ConstUnit);
                }

                // End
                self.patch(jump_to_end, OpCode::Jump(self.code.len()));
                Ok(())
            }

            Expr::Await { expr: inner, .. } => {
                self.compile_expr(inner)?;
                self.emit(OpCode::Await);
                Ok(())
            }

            Expr::MacroCall { name, args, .. } => {
                // For println!/print!/format! with simple {} format strings,
                // emit native DisplayArg for each arg to enable Display::fmt dispatch.
                let is_println = name == "println" || name == "print";
                let is_format = name == "format";
                if (is_println || is_format) && args.len() > 1 {
                    // Parse format string: split on "{}" and emit parts + DisplayArg
                    let fmt = match &args[0] {
                        Expr::StringLiteral(s, _) => s.clone(),
                        Expr::FString { .. } => String::new(), // f-strings handled elsewhere
                        _ => String::new(),
                    };
                    let parts: Vec<&str> = fmt.split("{}").collect();
                    // If there are {:?} placeholders, fall back to Format opcode
                    if fmt.contains("{:?}") {
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit(OpCode::Format {
                            arg_count: args.len(),
                        });
                    } else if parts.len() == args.len() {
                        // Interleave format parts and args: part0, arg1, part1, arg2, part2, ...
                        let mut concat_count = 0usize;
                        for i in 0..parts.len() {
                            // Emit the literal part
                            if !parts[i].is_empty() {
                                self.emit(OpCode::ConstString(parts[i].to_string()));
                                concat_count += 1;
                            }
                            // Emit the arg (except for the last part)
                            if i < args.len() - 1 {
                                self.compile_expr(&args[i + 1])?;
                                self.emit(OpCode::DisplayArg);
                                concat_count += 1;
                            }
                        }
                        if concat_count > 1 {
                            self.emit(OpCode::FStringConcat {
                                count: concat_count,
                            });
                        }
                    } else {
                        // Mismatched {} count — fall back to Format
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit(OpCode::Format {
                            arg_count: args.len(),
                        });
                    }
                    if is_println {
                        if name == "println" {
                            self.emit(OpCode::PrintLn);
                        } else {
                            self.emit(OpCode::Print);
                        }
                        // See note above: leave Unit on the stack.
                        self.emit(OpCode::ConstUnit);
                    }
                } else if (is_println || is_format) && args.len() == 1 {
                    // No format args — just print/format the literal
                    self.compile_expr(&args[0])?;
                    if name == "println" {
                        self.emit(OpCode::PrintLn);
                        self.emit(OpCode::ConstUnit);
                    } else if name == "print" {
                        self.emit(OpCode::Print);
                        self.emit(OpCode::ConstUnit);
                    }
                    // format! with no args just returns the string
                } else if name == "vec" {
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    self.emit(OpCode::MakeArray { count: args.len() });
                } else if name == "format" {
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    self.emit(OpCode::Format {
                        arg_count: args.len(),
                    });
                } else if name == "panic" {
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    self.emit(OpCode::Panic);
                    // Unreachable, but the surrounding expression context
                    // expects a value to be left on the stack.
                    self.emit(OpCode::ConstUnit);
                } else if name == "assert" {
                    // assert!(cond) or assert!(cond, "message")
                    self.compile_expr(&args[0])?; // compile condition
                    let skip = self.emit(OpCode::JumpIfTrue(0));
                    if args.len() > 1 {
                        self.compile_expr(&args[1])?; // custom message
                    } else {
                        self.emit(OpCode::ConstString("assertion failed".to_string()));
                    }
                    self.emit(OpCode::Panic);
                    self.patch(skip, OpCode::JumpIfTrue(self.code.len()));
                    self.emit(OpCode::ConstUnit);
                } else if name == "assert_eq" {
                    // assert_eq!(left, right) or assert_eq!(left, right, "message")
                    self.compile_expr(&args[0])?;
                    self.compile_expr(&args[1])?;
                    self.emit(OpCode::Eq);
                    let skip = self.emit(OpCode::JumpIfTrue(0));
                    if args.len() > 2 {
                        self.compile_expr(&args[2])?;
                    } else {
                        self.emit(OpCode::ConstString(
                            "assertion failed: left != right".to_string(),
                        ));
                    }
                    self.emit(OpCode::Panic);
                    self.patch(skip, OpCode::JumpIfTrue(self.code.len()));
                    self.emit(OpCode::ConstUnit);
                } else if name == "assert_ne" {
                    // assert_ne!(left, right) or assert_ne!(left, right, "message")
                    self.compile_expr(&args[0])?;
                    self.compile_expr(&args[1])?;
                    self.emit(OpCode::Neq);
                    let skip = self.emit(OpCode::JumpIfTrue(0));
                    if args.len() > 2 {
                        self.compile_expr(&args[2])?;
                    } else {
                        self.emit(OpCode::ConstString(
                            "assertion failed: left == right".to_string(),
                        ));
                    }
                    self.emit(OpCode::Panic);
                    self.patch(skip, OpCode::JumpIfTrue(self.code.len()));
                    self.emit(OpCode::ConstUnit);
                } else if name == "dbg" {
                    // dbg!(expr) — print debug representation and return the value
                    self.compile_expr(&args[0])?;
                    self.emit(OpCode::Dup);
                    self.emit(OpCode::PrintLn);
                } else {
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    return Err(self.not_yet_supported("Unknown macro", expr.span()));
                }
                Ok(())
            }
            Expr::As {
                expr: inner,
                type_name,
                ..
            } => {
                self.compile_expr(inner)?;
                match type_name.as_str() {
                    "int" => self.emit(OpCode::CastInt(IntegerWidth::I64)),
                    "byte" => self.emit(OpCode::CastInt(IntegerWidth::U8)),
                    "float" => self.emit(OpCode::CastFloat(FloatWidth::F64)),
                    "char" => self.emit(OpCode::CastToChar),
                    _ => return Ok(()),
                };
                Ok(())
            }
            Expr::Return { value, .. } => {
                if let Some(expr) = value {
                    self.compile_expr(expr)?;
                } else {
                    self.emit(OpCode::ConstUnit);
                }
                self.emit_return();
                Ok(())
            }
        }
    }
}
