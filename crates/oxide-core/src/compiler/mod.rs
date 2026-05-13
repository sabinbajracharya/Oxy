//! Compiler: walks the Oxide AST and emits stack-based bytecode for the VM.
//!
//! The compiler is single-pass. It resolves local variable names to stack
//! slot indices and emits [`OpCode`]s into a [`Chunk`]. Forward jumps
//! (for `if`, `while`, `loop`) are backpatched after the target is known.

use std::collections::HashMap;

use crate::ast::*;
use crate::errors::FerriError;
use crate::vm::{Chunk, OpCode};

/// Symbol table tracking local variables in the current scope.
#[derive(Clone)]
struct SymTable {
    /// Variable name → stack slot index.
    locals: HashMap<String, usize>,
    /// Next available slot index.
    next_slot: usize,
}

impl SymTable {
    fn new(start_slot: usize) -> Self {
        Self {
            locals: HashMap::new(),
            next_slot: start_slot,
        }
    }

    fn define(&mut self, name: &str) -> usize {
        let slot = self.next_slot;
        self.locals.insert(name.to_string(), slot);
        self.next_slot += 1;
        slot
    }

    fn get(&self, name: &str) -> Option<usize> {
        self.locals.get(name).copied()
    }

    fn slot_count(&self) -> usize {
        self.next_slot
    }
}

/// The Oxide bytecode compiler.
pub struct Compiler {
    /// The output code buffer.
    code: Vec<OpCode>,
    /// Current scope's symbol table.
    sym: SymTable,
    /// Function entry points: name → instruction index.
    functions: HashMap<String, usize>,
}

impl Compiler {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self {
            code: Vec::new(),
            sym: SymTable::new(0),
            functions: HashMap::new(),
        }
    }
}

impl Compiler {
    /// Compile a full program. Returns a [`Chunk`] ready for the VM.
    pub fn compile(mut self, program: &Program) -> Result<Chunk, FerriError> {
        // Emit entry preamble first: Call(main), Pop, Halt
        // main's ip is backpatched after main is compiled.
        let entry_point = 0;
        let call_site = self.emit(OpCode::Call {
            target: 0, // placeholder
            arg_count: 0,
        });
        self.emit(OpCode::Pop);
        self.emit(OpCode::Halt);

        // Compile function bodies
        for item in &program.items {
            self.compile_item(item)?;
        }

        // Backpatch the Call to main
        if let Some(&main_ip) = self.functions.get("main") {
            self.patch(
                call_site,
                OpCode::Call {
                    target: main_ip,
                    arg_count: 0,
                },
            );
        }

        Ok(Chunk {
            code: self.code,
            local_count: self.sym.slot_count(),
            entry_point,
            functions: self.functions,
        })
    }

    fn emit(&mut self, op: OpCode) -> usize {
        let idx = self.code.len();
        self.code.push(op);
        idx
    }

    /// Patch a previously emitted instruction at `idx` with a new opcode.
    fn patch(&mut self, idx: usize, op: OpCode) {
        self.code[idx] = op;
    }

    fn compile_item(&mut self, item: &Item) -> Result<(), FerriError> {
        match item {
            Item::Function(f) => {
                // Register the entry point
                let ip = self.code.len();
                self.functions.insert(f.name.clone(), ip);

                // Allocate slots for parameters
                let saved_sym = self.sym.clone();
                for param in &f.params {
                    self.sym.define(&param.name);
                }

                // Compile the body — the last expression (tail) is the return value.
                // compile_block leaves the tail value on the stack.
                self.compile_block(&f.body)?;
                self.emit(OpCode::Return);

                self.sym = saved_sym;
                Ok(())
            }
            Item::Const {
                name, value, span, ..
            } => {
                self.compile_expr(value)?;
                let slot = self.sym.define(name);
                self.emit(OpCode::StoreLocal(slot));
                let _ = span;
                Ok(())
            }
            // Skip other items — they don't produce executable code
            _ => Ok(()),
        }
    }

    fn compile_block(&mut self, block: &Block) -> Result<(), FerriError> {
        for (i, stmt) in block.stmts.iter().enumerate() {
            let is_last = i == block.stmts.len() - 1;
            self.compile_stmt(stmt, is_last)?;
        }
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &Stmt, is_last: bool) -> Result<(), FerriError> {
        match stmt {
            Stmt::Let {
                name,
                mutable: _,
                value,
                ..
            } => {
                if let Some(expr) = value {
                    self.compile_expr(expr)?;
                } else {
                    self.emit(OpCode::ConstUnit);
                }
                let slot = self.sym.define(name);
                self.emit(OpCode::StoreLocal(slot));
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
                self.emit(OpCode::Return);
                Ok(())
            }

            Stmt::While {
                condition, body, ..
            } => {
                let loop_start = self.code.len();
                self.compile_expr(condition)?;
                let jump_out = self.emit(OpCode::JumpIfFalse(0)); // placeholder
                self.compile_block(body)?;
                self.emit(OpCode::Jump(loop_start));
                let loop_end = self.code.len();
                self.patch(jump_out, OpCode::JumpIfFalse(loop_end));
                Ok(())
            }

            Stmt::Loop { body, .. } => {
                let loop_start = self.code.len();
                self.compile_block(body)?;
                self.emit(OpCode::Jump(loop_start));
                Ok(())
            }

            Stmt::For { span, .. } => Err(FerriError::Runtime {
                message: "compiled for loops not yet supported".into(),
                line: span.line,
                column: span.column,
            }),

            Stmt::Break { span, .. } => Err(FerriError::Runtime {
                message: "compiled break not yet supported".into(),
                line: span.line,
                column: span.column,
            }),

            Stmt::Continue { span } => Err(FerriError::Runtime {
                message: "compiled continue not yet supported".into(),
                line: span.line,
                column: span.column,
            }),

            // For simplicity, skip other statements
            _ => Ok(()),
        }
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), FerriError> {
        match expr {
            Expr::IntLiteral(n, _) => {
                self.emit(OpCode::ConstInt(*n));
                Ok(())
            }
            Expr::FloatLiteral(n, _) => {
                self.emit(OpCode::ConstFloat(*n));
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
                self.emit(OpCode::ConstString(c.to_string()));
                Ok(())
            }

            Expr::Ident(name, span) => {
                if let Some(slot) = self.sym.get(name) {
                    self.emit(OpCode::LoadLocal(slot));
                    Ok(())
                } else if self.functions.contains_key(name) {
                    self.emit(OpCode::ConstUnit); // placeholder for function ref
                    Ok(())
                } else {
                    Err(FerriError::Runtime {
                        message: format!("undefined variable '{name}'"),
                        line: span.line,
                        column: span.column,
                    })
                }
            }

            Expr::BinaryOp {
                left,
                op,
                right,
                span,
            } => {
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
                    BinOp::And => OpCode::And,
                    BinOp::Or => OpCode::Or,
                    _ => {
                        return Err(FerriError::Runtime {
                            message: format!("unsupported binary op in compiler: {:?}", op),
                            line: span.line,
                            column: span.column,
                        })
                    }
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
                callee, args, span, ..
            } => {
                // Compile arguments (left to right, so first arg is deepest on stack)
                for arg in args {
                    self.compile_expr(arg)?;
                }

                if let Expr::Ident(name, _) = callee.as_ref() {
                    // Check for built-in macros that we handle inline
                    if name == "println!" || name == "print!" {
                        let is_println = name == "println!";
                        // For now, just print the first formatted arg
                        // (full format string support would need interpreter interop)
                        if is_println {
                            self.emit(OpCode::PrintLn);
                        } else {
                            self.emit(OpCode::Print);
                        }
                        return Ok(());
                    }

                    if let Some(&target) = self.functions.get(name) {
                        self.emit(OpCode::Call {
                            target,
                            arg_count: args.len(),
                        });
                        return Ok(());
                    }
                }

                Err(FerriError::Runtime {
                    message: "compiled: only direct function calls supported".into(),
                    line: span.line,
                    column: span.column,
                })
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
                self.compile_expr(value)?;
                if let Expr::Ident(name, _) = target.as_ref() {
                    if let Some(slot) = self.sym.get(name) {
                        self.emit(OpCode::Dup);
                        self.emit(OpCode::StoreLocal(slot));
                        Ok(())
                    } else {
                        let slot = self.sym.define(name);
                        self.emit(OpCode::Dup);
                        self.emit(OpCode::StoreLocal(slot));
                        Ok(())
                    }
                } else {
                    Err(FerriError::Runtime {
                        message: "compiled: only simple variable assignment supported".into(),
                        line: span.line,
                        column: span.column,
                    })
                }
            }

            Expr::Try { expr: inner, .. } => {
                self.compile_expr(inner)?;
                // ? operator: leave value on stack, runtime handles unwrapping
                Ok(())
            }

            // Fallback for expressions not yet compiled
            Expr::Match { span, .. }
            | Expr::Range { span, .. }
            | Expr::Array { span, .. }
            | Expr::Tuple { span, .. }
            | Expr::StructInit { span, .. }
            | Expr::MethodCall { span, .. }
            | Expr::FieldAccess { span, .. }
            | Expr::Index { span, .. }
            | Expr::PathCall { span, .. }
            | Expr::Closure { span, .. }
            | Expr::Await { span, .. }
            | Expr::FString { span, .. } => Err(FerriError::Runtime {
                message: "this expression type is not yet supported in compiled mode".into(),
                line: span.line,
                column: span.column,
            }),

            _ => Ok(()),
        }
    }
}
