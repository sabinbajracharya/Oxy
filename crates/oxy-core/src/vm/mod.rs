//! Stack-based virtual machine for executing compiled Oxy bytecode.
//!
//! The VM executes a flat sequence of [`OpCode`]s produced by the compiler.
//! It uses a value stack and a call stack. Each call frame tracks its own
//! local variable slots and return address.

use std::cell::RefCell;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::rc::Rc;

use crate::interpreter::Interpreter;
use crate::types::Value;

/// Bytecode instructions for the Oxy VM.
#[derive(Debug, Clone)]
pub enum OpCode {
    // --- Constants ---
    ConstInt(i64),
    ConstFloat(f64),
    ConstBool(bool),
    ConstString(String),
    ConstChar(char),
    ConstUnit,

    // --- Variables ---
    /// Load local at slot index, push onto stack.
    LoadLocal(usize),
    /// Pop stack, store into local at slot index.
    StoreLocal(usize),

    // --- Binary operations (pop two, push result) ---
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,

    // --- Bitwise operations ---
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,

    // --- Unary operations ---
    Neg,
    Not,

    // --- Control flow ---
    /// Unconditional jump to instruction index.
    Jump(usize),
    /// Pop stack; if value is falsy, jump to instruction index.
    JumpIfFalse(usize),
    /// Pop stack; if value is truthy, jump to instruction index.
    JumpIfTrue(usize),

    // --- Functions ---
    /// Call the function at instruction index `target`. The arguments are already
    /// on the stack (last arg on top). `arg_count` args are consumed.
    Call {
        target: usize,
        arg_count: usize,
    },
    /// Return from the current function, leaving the top-of-stack as the result.
    Return,
    /// Pop a string message and halt with a runtime error.
    Panic,
    /// Stop execution.
    Halt,

    // --- Output ---
    /// Pop and print the value (no newline).
    Print,
    /// Pop and print with newline.
    PrintLn,

    // --- Stack manipulation ---
    /// Duplicate the top of stack.
    Dup,
    /// Pop and discard the top of stack.
    Pop,

    // --- Iteration ---
    /// Pop a Value, convert to Vec<Value> for iteration, push Vec.
    MakeIter,
    /// Pop a Vec, push its length as Integer.
    IterLen,
    /// Pop index (Integer), pop Vec, push element at Vec[index].
    VecIndex,
    /// Pop end (Value), pop start (Value), push Range(start, end).
    MakeRange,

    // --- Collections ---
    /// Pop `count` elements, push them as Value::Vec.
    MakeArray {
        count: usize,
    },
    /// Pop `count` elements, push them as Value::Tuple.
    MakeTuple {
        count: usize,
    },

    // --- String operations ---
    /// Pop a Value, convert to its string representation, push String.
    ToString,
    /// Pop `count` values, convert each to string, concatenate, push result.
    FStringConcat {
        count: usize,
    },
    /// Pop `arg_count` values, use first as format string, substitute subsequent
    /// values for `{}` placeholders, push the formatted result.
    Format {
        arg_count: usize,
    },
    /// Pop `field_count` values, build Value::Struct.
    /// Field values are on the stack in field_names order.
    StructInit {
        name: String,
        field_count: usize,
        field_names: Vec<String>,
    },
    /// Pop `arg_count` args + receiver, dispatch method by name on the receiver.
    MethodCall {
        method_name: String,
        arg_count: usize,
    },
    /// Pop object, push the value of its named field.
    FieldAccess {
        field_name: String,
    },
    /// Push an enum variant value (for `Type::Variant` paths).
    ConstEnumVariant {
        enum_name: String,
        variant: String,
        data: Vec<Value>,
    },
    /// Pop `arg_count` values, push an enum variant wrapping them.
    MakeEnumVariant {
        enum_name: String,
        variant: String,
        arg_count: usize,
    },
    /// Push a closure value: body starts at `target_ip`, takes `param_count` args.
    /// `meta_idx` indexes Chunk::closure_meta for param names + AST body.
    Closure {
        target_ip: usize,
        param_count: usize,
        meta_idx: usize,
    },
    /// Pop a Value::Function, extract its target IP, call with `arg_count` args.
    CallClosure {
        arg_count: usize,
    },
    /// Await a future: pop Value, if Future run its body, if JoinHandle unwrap,
    /// otherwise pass through.
    Await,
    /// Try operator `?`: pop value; if Err(e) or None, return early with that value;
    /// otherwise push unwrapped inner value.
    TryPop,
    /// Type cast: pop value, push converted value. Target: 0=int→float, 1=float→int,
    /// 2=int→char, 3=char→int.
    Cast(u8),

    // --- Pattern matching ---
    /// Pop a value and store it in the given local slot (for pattern binding).
    BindIdent(usize),
    /// Peek the top of stack; if it's EnumVariant{enum_name, variant}, push true
    /// and push each data field; otherwise push false.
    EnumVariantEqual {
        enum_name: String,
        variant: String,
    },
    /// Pop an EnumVariant, push data[index] (index 0 is first tuple field).
    EnumDataGet(usize),
    /// Pop `arg_count` values, dispatch to native built-in by path segments.
    PathCallBuiltin {
        segments: Vec<String>,
        arg_count: usize,
    },

    // --- Interpreter fallback ---
    /// Delegate evaluation of an AST expression to the tree-walking interpreter.
    /// The index references `Chunk::ast_nodes`.
    /// Pop value, pop struct, set field. For `obj.field = val`.
    FieldStore(String),
    Eval(usize),
}

/// A compiled Oxy program: a flat sequence of opcodes.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<OpCode>,
    /// Number of local variable slots needed for the top-level scope.
    pub local_count: usize,
    /// Instruction index where execution starts.
    pub entry_point: usize,
    /// Entry points: function name → instruction index.
    pub functions: std::collections::HashMap<String, usize>,
    /// AST expression nodes for interpreter fallback (indexed by Eval opcode arg).
    pub ast_nodes: Vec<crate::ast::Expr>,
    /// Closure metadata: (param_names, body_expr, captured_vars_with_slots).
    pub closure_meta: Vec<(Vec<String>, crate::ast::Expr, Vec<(String, usize)>)>,
    /// Local variable names: slot_index → name (for Eval env reconstruction).
    pub local_names: Vec<String>,
    /// Registered struct definitions (for StructInit and method dispatch).
    pub struct_defs: std::collections::HashMap<String, crate::ast::StructDef>,
    /// Registered enum definitions (for Path enum variant lookup).
    pub enum_defs: std::collections::HashMap<String, crate::ast::EnumDef>,
    /// Impl methods: type_name → method definitions.
    pub impl_methods: std::collections::HashMap<String, Vec<crate::ast::FnDef>>,
    /// Compiled method entry points: (type_name, method_name) → instruction index.
    pub method_ips: std::collections::HashMap<(String, String), usize>,
}

/// The stack-based VM executor.
pub struct Vm {
    /// The compiled chunk being executed.
    chunk: Chunk,
    /// Value stack (shared across frames for simplicity).
    stack: Vec<Value>,
    /// Instruction pointer.
    ip: usize,
    /// Call stack: (return_address, stack_base_before_call, local_count).
    call_stack: Vec<Frame>,
    /// Captured output (for testing).
    output: Option<Vec<String>>,
    /// Tree-walking interpreter for Eval opcode fallback.
    interpreter: Interpreter,
}

struct Frame {
    return_ip: usize,
    base: usize,
    /// Maximum slot index accessed + 1 (protects locals from Pop).
    max_slot: usize,
}

/// Result of VM execution.
pub enum VmResult {
    Value(Value),
    Error(String),
}

impl Vm {
    pub fn new(chunk: Chunk) -> Self {
        Self {
            chunk,
            stack: Vec::new(),
            ip: 0,
            call_stack: Vec::new(),
            output: None,
            interpreter: Interpreter::new(),
        }
    }

    /// Create a VM that captures printed output (for testing).
    pub fn with_captured_output(chunk: Chunk) -> Self {
        let mut vm = Self::new(chunk);
        vm.output = Some(Vec::new());
        vm
    }

    /// Get captured output lines.
    pub fn captured_output(&self) -> &[String] {
        self.output.as_deref().unwrap_or(&[])
    }

    /// Execute the chunk, starting at the entry point.
    pub fn run(&mut self) -> VmResult {
        self.ip = self.chunk.entry_point;

        // Push a synthetic top-level frame to protect locals from Pop
        self.call_stack.push(Frame {
            return_ip: 0,
            base: 0,
            max_slot: 0,
        });

        loop {
            let op = match self.chunk.code.get(self.ip) {
                Some(op) => op.clone(),
                None => return VmResult::Error("unexpected end of code".into()),
            };

            match op {
                OpCode::ConstInt(n) => self.stack.push(Value::Integer(n)),
                OpCode::ConstFloat(n) => self.stack.push(Value::Float(n)),
                OpCode::ConstBool(b) => self.stack.push(Value::Bool(b)),
                OpCode::ConstString(s) => self.stack.push(Value::String(s.clone())),
                OpCode::ConstChar(c) => self.stack.push(Value::Char(c)),
                OpCode::ConstUnit => self.stack.push(Value::Unit),

                OpCode::LoadLocal(slot) => {
                    let base = self.frame_base();
                    let idx = base + slot;
                    let val = self.stack.get(idx).cloned().unwrap_or(Value::Unit);
                    self.stack.push(val);
                }

                OpCode::StoreLocal(slot) => {
                    let val = self.stack.pop().unwrap_or(Value::Unit);
                    let base = self.frame_base();
                    let idx = base + slot;
                    while idx >= self.stack.len() {
                        self.stack.push(Value::Unit);
                    }
                    self.stack[idx] = val;
                    // Update frame's max_slot to protect this local
                    if let Some(frame) = self.call_stack.last_mut() {
                        if slot + 1 > frame.max_slot {
                            frame.max_slot = slot + 1;
                        }
                    }
                }

                OpCode::Add => self.binary_op(vm_add),
                OpCode::Sub => self.binary_op(vm_sub),
                OpCode::Mul => self.binary_op(vm_mul),
                OpCode::Div => self.binary_op(vm_div),
                OpCode::Mod => self.binary_op(vm_rem),
                OpCode::Eq => {
                    let (a, b) = self.pop_two();
                    self.stack.push(Value::Bool(a == b));
                }
                OpCode::Neq => {
                    let (a, b) = self.pop_two();
                    self.stack.push(Value::Bool(a != b));
                }
                OpCode::Lt => {
                    let (a, b) = self.pop_two();
                    self.stack.push(Value::Bool(a < b));
                }
                OpCode::Gt => {
                    let (a, b) = self.pop_two();
                    self.stack.push(Value::Bool(a > b));
                }
                OpCode::Le => {
                    let (a, b) = self.pop_two();
                    self.stack.push(Value::Bool(a <= b));
                }
                OpCode::Ge => {
                    let (a, b) = self.pop_two();
                    self.stack.push(Value::Bool(a >= b));
                }
                OpCode::And => {
                    let (a, b) = self.pop_two();
                    self.stack.push(Value::Bool(a.is_truthy() && b.is_truthy()));
                }
                OpCode::Or => {
                    let (a, b) = self.pop_two();
                    self.stack.push(Value::Bool(a.is_truthy() || b.is_truthy()));
                }

                OpCode::BitAnd => self.binary_op(vm_bitand),
                OpCode::BitOr => self.binary_op(vm_bitor),
                OpCode::BitXor => self.binary_op(vm_bitxor),
                OpCode::Shl => self.binary_op(vm_shl),
                OpCode::Shr => self.binary_op(vm_shr),

                OpCode::Neg => {
                    let v = self.stack.pop().unwrap_or(Value::Unit);
                    self.stack.push(vm_neg(v));
                }

                OpCode::Not => {
                    let v = self.stack.pop().unwrap_or(Value::Unit);
                    self.stack.push(Value::Bool(!v.is_truthy()));
                }

                OpCode::Jump(target) => {
                    self.ip = target;
                    continue;
                }

                OpCode::JumpIfFalse(target) => {
                    let cond = self.stack.pop().unwrap_or(Value::Unit);
                    if !cond.is_truthy() {
                        self.ip = target;
                        continue;
                    }
                }

                OpCode::JumpIfTrue(target) => {
                    let cond = self.stack.pop().unwrap_or(Value::Unit);
                    if cond.is_truthy() {
                        self.ip = target;
                        continue;
                    }
                }

                OpCode::Call { target, arg_count } => {
                    if self.call_stack.len() >= 1024 {
                        return VmResult::Error("recursion limit exceeded (max depth 1024)".into());
                    }
                    let args_start = self.stack.len() - arg_count;
                    self.call_stack.push(Frame {
                        return_ip: self.ip + 1,
                        base: args_start,
                        max_slot: arg_count, // args occupy slots 0..arg_count-1
                    });
                    self.ip = target;
                    continue;
                }

                OpCode::Return => {
                    let result = self.stack.pop().unwrap_or(Value::Unit);
                    let frame = self.call_stack.pop().unwrap();
                    if self.call_stack.is_empty() {
                        // Top-level return (only synthetic frame remains → popped it)
                        return VmResult::Value(result);
                    }
                    // Return to caller: truncate to frame base, push result
                    self.stack.truncate(frame.base);
                    self.stack.push(result);
                    self.ip = frame.return_ip;
                    continue;
                }

                OpCode::Panic => {
                    let msg = self.stack.pop().map(|v| v.to_string()).unwrap_or_default();
                    return VmResult::Error(msg);
                }
                OpCode::Halt => {
                    return VmResult::Value(Value::Unit);
                }

                OpCode::Print => {
                    let v = self.stack.pop().unwrap_or(Value::Unit);
                    let s = v.to_string();
                    self.write_output(&s);
                }

                OpCode::PrintLn => {
                    let v = self.stack.pop().unwrap_or(Value::Unit);
                    let s = format!("{}\n", v);
                    self.write_output(&s);
                }

                OpCode::Dup => {
                    let v = self.stack.last().cloned().unwrap_or(Value::Unit);
                    self.stack.push(v);
                }

                OpCode::Pop => {
                    let protected = self.frame_protected();
                    if self.stack.len() > protected {
                        self.stack.pop();
                    }
                }

                OpCode::MakeIter => {
                    let value = self.stack.pop().unwrap_or(Value::Unit);
                    match value.into_iterable() {
                        Ok(vec) => self.stack.push(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(vec)))),
                        Err(e) => return VmResult::Error(e),
                    }
                }

                OpCode::IterLen => {
                    let v = self.stack.pop().unwrap_or(Value::Unit);
                    match v {
                        Value::Vec(rc) => self.stack.push(Value::Integer(rc.borrow().len() as i64)),
                        other => {
                            return VmResult::Error(format!(
                                "cannot get length of {}",
                                other.type_name()
                            ))
                        }
                    }
                }

                OpCode::VecIndex => {
                    let idx = match self.stack.pop().unwrap_or(Value::Unit) {
                        Value::Integer(i) => i as usize,
                        other => {
                            return VmResult::Error(format!(
                                "index must be integer, got {}",
                                other.type_name()
                            ))
                        }
                    };
                    match self.stack.pop().unwrap_or(Value::Unit) {
                        Value::Vec(rc) => {
                            let vec = rc.borrow();
                            if idx < vec.len() {
                                self.stack.push(vec[idx].clone());
                            } else {
                                return VmResult::Error(format!(
                                    "index {} out of bounds for len {}",
                                    idx,
                                    vec.len()
                                ));
                            }
                        }
                        Value::String(s) => {
                            if let Some(c) = s.chars().nth(idx) {
                                self.stack.push(Value::Char(c));
                            } else {
                                return VmResult::Error(format!(
                                    "index {} out of bounds for len {}",
                                    idx,
                                    s.chars().count()
                                ));
                            }
                        }
                        Value::Tuple(t) => {
                            if idx < t.len() {
                                self.stack.push(t[idx].clone());
                            } else {
                                return VmResult::Error(format!(
                                    "index {} out of bounds for len {}",
                                    idx,
                                    t.len()
                                ));
                            }
                        }
                        other => {
                            return VmResult::Error(format!("cannot index {}", other.type_name()))
                        }
                    }
                }

                OpCode::MakeRange => {
                    let end = self.stack.pop().unwrap_or(Value::Unit);
                    let start = self.stack.pop().unwrap_or(Value::Unit);
                    match (start, end) {
                        (Value::Integer(s), Value::Integer(e)) => {
                            self.stack.push(Value::Range(s, e));
                        }
                        (s, e) => {
                            return VmResult::Error(format!(
                                "range bounds must be integers, got {} and {}",
                                s.type_name(),
                                e.type_name()
                            ));
                        }
                    }
                }

                OpCode::MakeArray { count } => {
                    let start = self.stack.len().saturating_sub(count);
                    let elements: Vec<Value> = self.stack.drain(start..).collect();
                    self.stack.push(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(elements))));
                }

                OpCode::MakeTuple { count } => {
                    let start = self.stack.len().saturating_sub(count);
                    let elements: Vec<Value> = self.stack.drain(start..).collect();
                    self.stack.push(Value::Tuple(elements));
                }

                OpCode::StructInit {
                    name,
                    field_count,
                    field_names,
                } => {
                    let start = self.stack.len().saturating_sub(field_count);
                    let values: Vec<Value> = self.stack.drain(start..).collect();
                    let fields: HashMap<String, Value> =
                        field_names.into_iter().zip(values).collect();
                    // Validate required fields against struct definition
                    if let Some(struct_def) = self.chunk.struct_defs.get(&name) {
                        if let crate::ast::StructKind::Named(named_fields) = &struct_def.kind {
                            for required in named_fields {
                                if !fields.contains_key(&required.name) {
                                    return VmResult::Error(format!(
                                        "struct '{}' missing required field '{}'",
                                        name, required.name
                                    ));
                                }
                            }
                        }
                    }
                    self.stack.push(Value::Struct { name, fields });
                }

                OpCode::ConstEnumVariant {
                    enum_name,
                    variant,
                    data,
                } => {
                    self.stack.push(Value::EnumVariant {
                        enum_name,
                        variant,
                        data,
                    });
                }

                OpCode::MakeEnumVariant {
                    enum_name,
                    variant,
                    arg_count,
                } => {
                    let start = self.stack.len().saturating_sub(arg_count);
                    let data: Vec<Value> = self.stack.drain(start..).collect();
                    self.stack.push(Value::EnumVariant {
                        enum_name,
                        variant,
                        data,
                    });
                }

                OpCode::Closure {
                    target_ip,
                    param_count,
                    meta_idx,
                } => {
                    let blank_span = crate::lexer::Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        column: 0,
                    };
                    let (param_names, body_expr, captured_vars) =
                        self.chunk.closure_meta.get(meta_idx).cloned().unwrap_or_else(
                            || {
                                (
                                    (0..param_count).map(|i| format!("_{i}")).collect(),
                                    crate::ast::Expr::IntLiteral(0, blank_span),
                                    Vec::new(),
                                )
                            },
                        );
                    let params: Vec<crate::ast::Param> = param_names
                        .into_iter()
                        .map(|name| crate::ast::Param {
                            name,
                            type_ann: crate::ast::TypeAnnotation {
                                name: "_".into(),
                                span: blank_span,
                            },
                            span: blank_span,
                        })
                        .collect();
                    let body_block = crate::ast::Block {
                        stmts: vec![crate::ast::Stmt::Expr {
                            expr: body_expr,
                            has_semicolon: false,
                        }],
                        span: blank_span,
                    };
                    // Build closure environment with captured outer variables
                    let mut closure_env = crate::env::Environment::new();
                    if !captured_vars.is_empty() {
                        let base = self.frame_base();
                        for (name, slot) in &captured_vars {
                            let idx = base + slot;
                            let val = self.stack.get(idx).cloned().unwrap_or(Value::Unit);
                            closure_env.borrow_mut().define(name.clone(), val, false);
                        }
                    }
                    self.stack
                        .push(Value::Function(Box::new(crate::types::FunctionData {
                            name: "<closure>".into(),
                            params,
                            return_type: None,
                            body: body_block,
                            closure_env,
                            target_ip: Some(target_ip),
                        })));
                }

                OpCode::CallClosure { arg_count } => {
                    let fn_val = self
                        .stack
                        .get(self.stack.len().saturating_sub(arg_count + 1))
                        .cloned();
                    if let Some(Value::Function(f)) = fn_val {
                        if let Some(target) = f.target_ip {
                            if self.call_stack.len() >= 1024 {
                                return VmResult::Error(
                                    "recursion limit exceeded (max depth 1024)".into(),
                                );
                            }
                            let args_start = self.stack.len() - arg_count;
                            self.call_stack.push(Frame {
                                return_ip: self.ip + 1,
                                base: args_start,
                                max_slot: arg_count,
                            });
                            self.ip = target;
                            continue;
                        }
                    }
                    return VmResult::Error("CallClosure: value is not a callable closure".into());
                }

                OpCode::Await => {
                    let val = self.stack.pop().unwrap_or(Value::Unit);
                    match val {
                        Value::Future(_) => {
                            return VmResult::Error(
                                "Await on Future not yet supported in VM".into(),
                            );
                        }
                        Value::JoinHandle(inner) => {
                            self.stack.push(*inner);
                        }
                        other => {
                            self.stack.push(other);
                        }
                    }
                }

                OpCode::TryPop => {
                    let val = self.stack.pop().unwrap_or(Value::Unit);
                    let is_error = matches!(&val,
                        Value::EnumVariant { enum_name, variant, .. }
                            if (enum_name == "Result" && variant == "Err")
                                || (enum_name == "Option" && variant == "None")
                    );
                    if is_error {
                        // Early return with the error/None value
                        self.stack.push(val);
                        let frame = self.call_stack.pop().unwrap();
                        if self.call_stack.is_empty() {
                            return VmResult::Value(self.stack.pop().unwrap_or(Value::Unit));
                        }
                        let ret_val = self.stack.pop().unwrap_or(Value::Unit);
                        self.stack.truncate(frame.base);
                        self.stack.push(ret_val);
                        self.ip = frame.return_ip;
                        continue;
                    }
                    match val {
                        Value::EnumVariant { variant, data, .. }
                            if variant == "Some" || variant == "Ok" =>
                        {
                            self.stack
                                .push(data.first().cloned().unwrap_or(Value::Unit));
                        }
                        other => {
                            self.stack.push(other);
                        }
                    }
                }

                OpCode::Cast(target) => {
                    let val = self.stack.pop().unwrap_or(Value::Unit);
                    let result = match target {
                        0 => match val {
                            Value::Integer(n) => Value::Float(n as f64),
                            v => v,
                        },
                        1 => match val {
                            Value::Float(n) => Value::Integer(n as i64),
                            v => v,
                        },
                        2 => match val {
                            Value::Integer(n) => {
                                Value::Char(char::from_u32(n as u32).unwrap_or('\0'))
                            }
                            v => v,
                        },
                        3 => match val {
                            Value::Char(c) => Value::Integer(c as i64),
                            v => v,
                        },
                        _ => val,
                    };
                    self.stack.push(result);
                }

                OpCode::FieldAccess { field_name } => {
                    let obj = self.stack.pop().unwrap_or(Value::Unit);
                    let result = match &obj {
                        Value::Struct { fields, .. } => {
                            fields.get(&field_name).cloned().unwrap_or(Value::Unit)
                        }
                        Value::HashMap(rc) => rc
                            .borrow()
                            .get(&Value::String(field_name.clone()))
                            .cloned()
                            .unwrap_or(Value::Unit),
                        _ => {
                            return VmResult::Error(format!(
                                "cannot access field '{}' on type {}",
                                field_name,
                                obj.type_name()
                            ));
                        }
                    };
                    self.stack.push(result);
                }

                OpCode::FieldStore(field_name) => {
                    let val = self.stack.pop().unwrap_or(Value::Unit);
                    let mut obj = self.stack.pop().unwrap_or(Value::Unit);
                    match &mut obj {
                        Value::Struct { fields, .. } => {
                            fields.insert(field_name.clone(), val);
                            self.stack.push(obj);
                        }
                        Value::HashMap(rc) => {
                            rc.borrow_mut()
                                .insert(Value::String(field_name.clone()), val);
                            self.stack.push(Value::HashMap(rc.clone()));
                        }
                        _ => {
                            return VmResult::Error(format!(
                                "cannot set field '{}' on type {}",
                                field_name,
                                obj.type_name()
                            ));
                        }
                    }
                }

                OpCode::MethodCall {
                    method_name,
                    arg_count,
                } => {
                    let args_start = self.stack.len() - arg_count;
                    let args: Vec<Value> = self.stack.drain(args_start..).collect();
                    let receiver = self.stack.pop().unwrap_or(Value::Unit);
                    let type_name = receiver.type_name().to_string();
                    let is_struct = matches!(receiver, Value::Struct { .. });
                    let is_enum = matches!(receiver, Value::EnumVariant { .. });

                    // Look up method IP: first check user methods, then built-ins
                    let method_ip = if is_struct || is_enum {
                        // Check user-defined methods
                        let struct_name = match &receiver {
                            Value::Struct { name, .. } => name.clone(),
                            Value::EnumVariant { enum_name, .. } => enum_name.clone(),
                            _ => type_name.clone(),
                        };
                        self.chunk
                            .method_ips
                            .get(&(struct_name.clone(), method_name.clone()))
                            .copied()
                            .or_else(|| {
                                // Fallback: check all impl_methods for trait methods
                                self.chunk
                                    .method_ips
                                    .get(&(struct_name, method_name.clone()))
                                    .copied()
                            })
                    } else {
                        None
                    };

                    match method_ip {
                        Some(target) => {
                            // Push receiver back (as self), then args
                            self.stack.push(receiver);
                            self.stack.extend(args);
                            if self.call_stack.len() >= 1024 {
                                return VmResult::Error(
                                    "recursion limit exceeded (max depth 1024)".into(),
                                );
                            }
                            self.call_stack.push(Frame {
                                return_ip: self.ip + 1,
                                base: self.stack.len() - arg_count - 1,
                                max_slot: arg_count + 1,
                            });
                            self.ip = target;
                            continue;
                        }
                        None => {
                            // Handle built-in methods (Vec, String, HashMap, etc.)
                            match self.builtin_method(receiver, &method_name, args) {
                                Ok(val) => self.stack.push(val),
                                Err(e) => return VmResult::Error(e),
                            }
                        }
                    }
                }

                OpCode::ToString => {
                    let val = self.stack.pop().unwrap_or(Value::Unit);
                    self.stack.push(Value::String(val.to_string()));
                }

                OpCode::FStringConcat { count } => {
                    let start = self.stack.len().saturating_sub(count);
                    let parts: Vec<String> =
                        self.stack.drain(start..).map(|v| v.to_string()).collect();
                    self.stack.push(Value::String(parts.concat()));
                }

                OpCode::Format { arg_count } => {
                    let start = self.stack.len().saturating_sub(arg_count);
                    let args: Vec<Value> = self.stack.drain(start..).collect();
                    let fmt_str = args.first().map(|v| v.to_string()).unwrap_or_default();
                    let mut result = fmt_str.clone();
                    for val in &args[1..] {
                        let s = val.to_string();
                        if let Some(pos) = result.find("{:?}") {
                            result.replace_range(pos..pos + 4, &s);
                        } else if let Some(pos) = result.find("{}") {
                            result.replace_range(pos..pos + 2, &s);
                        }
                    }
                    self.stack.push(Value::String(result));
                }

                OpCode::BindIdent(slot) => {
                    let val = self.stack.pop().unwrap_or(Value::Unit);
                    let base = self.frame_base();
                    let idx = base + slot;
                    while idx >= self.stack.len() {
                        self.stack.push(Value::Unit);
                    }
                    self.stack[idx] = val;
                    if let Some(frame) = self.call_stack.last_mut() {
                        if slot + 1 > frame.max_slot {
                            frame.max_slot = slot + 1;
                        }
                    }
                }

                OpCode::EnumVariantEqual { enum_name, variant } => {
                    // Pop the scrutinee, check match, push data + bool
                    let val = self.stack.pop().unwrap_or(Value::Unit);
                    match &val {
                        Value::EnumVariant {
                            enum_name: en,
                            variant: v,
                            data,
                        } if en == &enum_name && v == &variant => {
                            for d in data.iter() {
                                self.stack.push(d.clone());
                            }
                            self.stack.push(Value::Bool(true));
                        }
                        _ => {
                            self.stack.push(Value::Bool(false));
                        }
                    }
                }

                OpCode::EnumDataGet(index) => {
                    let val = self.stack.pop().unwrap_or(Value::Unit);
                    match val {
                        Value::EnumVariant { data, .. } => {
                            let item = data.get(index).cloned().unwrap_or(Value::Unit);
                            self.stack.push(item);
                        }
                        _ => {
                            return VmResult::Error(format!(
                                "EnumDataGet: expected enum variant, got {}",
                                val.type_name()
                            ));
                        }
                    }
                }

                OpCode::PathCallBuiltin {
                    segments,
                    arg_count,
                } => {
                    let args_start = self.stack.len().saturating_sub(arg_count);
                    let args: Vec<Value> = self.stack.drain(args_start..).collect();
                    let result = self.dispatch_pathcall(&segments, &args);
                    match result {
                        Ok(val) => self.stack.push(val),
                        Err(e) => return VmResult::Error(e),
                    }
                }

                OpCode::Eval(idx) => {
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "warning: Eval fallback used for ast_node {} — add native VM support",
                        idx
                    );
                    let expr = self.chunk.ast_nodes.get(idx).cloned();
                    match expr {
                        Some(expr) => {
                            let env = self.build_env_for_eval();
                            match self.interpreter.eval_expr(&expr, &env) {
                                Ok(val) => self.stack.push(val),
                                Err(e) => {
                                    return VmResult::Error(format!("eval fallback failed: {}", e));
                                }
                            }
                        }
                        None => {
                            return VmResult::Error(format!(
                                "Eval: invalid ast_node index {}",
                                idx
                            ));
                        }
                    }
                }
            }

            self.ip += 1;
        }
    }

    fn binary_op(&mut self, f: fn(Value, Value) -> Result<Value, String>) {
        let (a, b) = self.pop_two();
        match f(a, b) {
            Ok(v) => self.stack.push(v),
            Err(_e) => self.stack.push(Value::Unit),
        }
    }

    /// Reconstruct an interpreter Env from the current VM stack frame.
    fn build_env_for_eval(&self) -> crate::env::Env {
        let env = crate::env::Environment::new();
        let base = self.frame_base();
        let max_slot = self.call_stack.last().map(|f| f.max_slot).unwrap_or(0);
        for slot in 0..max_slot {
            if let Some(name) = self.chunk.local_names.get(slot) {
                if !name.is_empty() {
                    let val = self.stack.get(base + slot).cloned().unwrap_or(Value::Unit);
                    env.borrow_mut().define(name.clone(), val, true);
                }
            }
        }
        env
    }

    /// Built-in method dispatch (Vec, String, HashMap, Option, Result, etc.).
    fn builtin_method(
        &mut self,
        receiver: Value,
        method_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        match &receiver {
            Value::Vec(rc) => {
                // Try builtins first
                let result = builtins::vec::dispatch(
                    Value::Vec(rc.clone()),
                    method_name,
                    &args,
                );
                if result.is_ok() {
                    return result;
                }
                // Fall back to iterator delegation for closure-based methods
                let data = rc.borrow().clone();
                let iter = Value::Iterator(Box::new(
                    crate::types::IteratorState::VecSource { data, index: 0 },
                ));
                builtins::iterator::dispatch(iter, method_name, &args, |func, fargs| {
                    self.interpreter
                        .call_function(func, fargs, 0, 0)
                        .map_err(|e| format!("{e}"))
                })
            },
            Value::String(_) => builtins::string::dispatch(receiver, method_name, &args),
            Value::HashMap(_) => builtins::hashmap::dispatch(receiver, method_name, &args),
            Value::HashSet(_) => builtins::hashset::dispatch(receiver, method_name, &args),
            Value::VecDeque(_) => builtins::vec_deque::dispatch(receiver, method_name, &args),
            Value::BinaryHeap(_) => builtins::binary_heap::dispatch(receiver, method_name, &args),
            Value::Char(c) => match method_name {
                "is_digit" => Ok(Value::Bool(c.is_ascii_digit())),
                "is_alphabetic" => Ok(Value::Bool(c.is_alphabetic())),
                "is_alphanumeric" => Ok(Value::Bool(c.is_alphanumeric())),
                "is_whitespace" => Ok(Value::Bool(c.is_whitespace())),
                "is_lowercase" => Ok(Value::Bool(c.is_lowercase())),
                "is_uppercase" => Ok(Value::Bool(c.is_uppercase())),
                "is_ascii" => Ok(Value::Bool(c.is_ascii())),
                "to_lowercase" => {
                    let lower: String = c.to_lowercase().collect();
                    if lower.len() == 1 {
                        Ok(Value::Char(lower.chars().next().unwrap()))
                    } else {
                        Ok(Value::String(lower))
                    }
                }
                "to_uppercase" => {
                    let upper: String = c.to_uppercase().collect();
                    if upper.len() == 1 {
                        Ok(Value::Char(upper.chars().next().unwrap()))
                    } else {
                        Ok(Value::String(upper))
                    }
                }
                "clone" => Ok(Value::Char(*c)),
                "code" => Ok(Value::Integer(*c as i64)),
                _ => Err(format!("no method '{}' on type char", method_name)),
            },
            Value::Integer(_) | Value::Float(_) => {
                builtins::numeric::dispatch(receiver, method_name, &args)
            }
            Value::EnumVariant { enum_name, .. }
                if enum_name == "Option" || enum_name == "Result" =>
            {
                builtins::option_result::dispatch(receiver, method_name, &args)
            }
            Value::EnumVariant { enum_name, .. } => {
                Err(format!(
                    "no method '{}' on enum '{}'",
                    method_name, enum_name
                ))
            }
            Value::Struct { name, .. } => match method_name {
                "clone" => Ok(receiver.clone()),
                _ => Err(format!("no method '{}' on struct '{}'", method_name, name)),
            },
            // Iterator: native adapters + consumers via builtins.
            // Closure consumers use interpreter's call_function in a Rust loop.
            Value::Iterator(_) => {
                builtins::iterator::dispatch(receiver, method_name, &args, |func, fargs| {
                    self.interpreter
                        .call_function(func, fargs, 0, 0)
                        .map_err(|e| format!("{e}"))
                })
            }
            _ => Err(format!(
                "no method '{}' on type {}",
                method_name,
                receiver.type_name()
            )),
        }
    }

    fn dispatch_pathcall(&self, segments: &[String], args: &[Value]) -> Result<Value, String> {
        let segs: Vec<&str> = segments.iter().map(|s| s.as_str()).collect();
        let to_f64 = |v: &Value| match v {
            Value::Integer(n) => *n as f64,
            Value::Float(x) => *x,
            _ => 0.0,
        };
        match segs.as_slice() {
            // math routes through the stdlib module for float_to_value consistency
            ["math", func] => call_stdlib(crate::stdlib::math::call, func, args),
            ["json", "parse"] => {
                let s = args.first().map(|v| format!("{}", v)).unwrap_or_default();
                crate::json::deserialize(&s).map_err(|e| format!("json::parse: {}", e))
            }
            ["json", "to_string"] => {
                let val = args.first().cloned().unwrap_or(Value::Unit);
                Ok(Value::String(format!("{}", val)))
            }
            ["String", "from"] => {
                let s = args.first().map(|v| format!("{}", v)).unwrap_or_default();
                Ok(Value::String(s))
            }
            ["HashMap", "new"] => Ok(Value::HashMap(std::rc::Rc::new(std::cell::RefCell::new(HashMap::new())))),
            ["HashSet", "new"] => Ok(Value::HashSet(std::rc::Rc::new(std::cell::RefCell::new(HashSet::new())))),
            ["BinaryHeap", "new"] => Ok(Value::BinaryHeap(BinaryHeap::new())),
            ["VecDeque", "new"] => Ok(Value::VecDeque(VecDeque::new())),
            ["ListNode", "new"] => {
                let val = args.first().cloned().unwrap_or(Value::Unit);
                let mut fields = HashMap::new();
                fields.insert("val".to_string(), val);
                fields.insert("next".to_string(), Value::none());
                Ok(Value::Struct {
                    name: "ListNode".to_string(),
                    fields,
                })
            }
            ["TreeNode", "new"] => {
                let val = args.first().cloned().unwrap_or(Value::Unit);
                let mut fields = HashMap::new();
                fields.insert("val".to_string(), val);
                fields.insert("left".to_string(), Value::none());
                fields.insert("right".to_string(), Value::none());
                Ok(Value::Struct {
                    name: "TreeNode".to_string(),
                    fields,
                })
            }
            ["int", "parse"] => {
                let s = args.first().map(|v| v.to_string()).unwrap_or_default();
                match s.trim().parse::<i64>() {
                    Ok(n) => Ok(Value::ok(Value::Integer(n))),
                    Err(_) => Ok(Value::err(Value::String(format!(
                        "cannot parse \"{s}\" as integer"
                    )))),
                }
            }
            ["char", "from_code"] => {
                let n = args
                    .first()
                    .and_then(|v| match v {
                        Value::Integer(n) => Some(*n as u32),
                        _ => None,
                    })
                    .unwrap_or(0);
                match char::from_u32(n) {
                    Some(c) => Ok(Value::Char(c)),
                    None => Err(format!("char::from_code: invalid code point {n}")),
                }
            }
            ["float", "parse"] => {
                let s = args.first().map(|v| v.to_string()).unwrap_or_default();
                match s.trim().parse::<f64>() {
                    Ok(n) => Ok(Value::ok(Value::Float(n))),
                    Err(_) => Ok(Value::err(Value::String(format!(
                        "cannot parse \"{s}\" as float"
                    )))),
                }
            }
            // --- stdlib modules ---
            // db and server are feature-gated, handled via emit_eval fallback
            ["fs", func] => call_stdlib(crate::stdlib::fs::call, func, args),
            ["env", func] => call_stdlib(crate::stdlib::env::call, func, args),
            ["process", func] => call_stdlib(crate::stdlib::process::call, func, args),
            ["regex", func] => call_stdlib(crate::stdlib::regex::call, func, args),
            ["net", func] => call_stdlib(crate::stdlib::net::call, func, args),
            ["time", func] => call_stdlib(crate::stdlib::time::call, func, args),
            ["rand", func] => call_stdlib(crate::stdlib::rand::call, func, args),
            _ => Err(format!("unknown built-in path: {}", segments.join("::"))),
        }
    }

    fn pop_two(&mut self) -> (Value, Value) {
        let b = self.stack.pop().unwrap_or(Value::Unit);
        let a = self.stack.pop().unwrap_or(Value::Unit);
        (a, b)
    }

    fn frame_base(&self) -> usize {
        self.call_stack.last().map(|f| f.base).unwrap_or(0)
    }

    fn frame_protected(&self) -> usize {
        self.call_stack
            .last()
            .map(|f| f.base + f.max_slot)
            .unwrap_or(0)
    }

    fn write_output(&mut self, s: &str) {
        if let Some(ref mut output) = self.output {
            if let Some(last) = output.last_mut() {
                if !last.ends_with('\n') {
                    last.push_str(s);
                    return;
                }
            }
            output.push(s.to_string());
        } else {
            print!("{s}");
        }
    }
}

// --- VM arithmetic helpers (standalone to avoid trait conflicts) ---

/// Call a stdlib module function, converting FerriError to String.
fn call_stdlib(
    f: fn(&str, &[Value], &crate::lexer::Span) -> Result<Value, crate::errors::FerriError>,
    func: &str,
    args: &[Value],
) -> Result<Value, String> {
    let span = crate::lexer::Span {
        start: 0,
        end: 0,
        line: 0,
        column: 0,
    };
    f(func, args, &span).map_err(|e| format!("{e}"))
}

// --- VM arithmetic helpers (standalone to avoid trait conflicts) ---

fn vm_add(a: Value, b: Value) -> Result<Value, String> {
    match (&a, &b) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
        (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
        (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a + *b as f64)),
        _ => Err(format!(
            "cannot add {} and {}",
            a.type_name(),
            b.type_name()
        )),
    }
}

fn vm_sub(a: Value, b: Value) -> Result<Value, String> {
    match (&a, &b) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
        (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
        (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a - *b as f64)),
        _ => Err(format!(
            "cannot subtract {} and {}",
            a.type_name(),
            b.type_name()
        )),
    }
}

fn vm_mul(a: Value, b: Value) -> Result<Value, String> {
    match (&a, &b) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
        (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
        (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a * *b as f64)),
        _ => Err(format!(
            "cannot multiply {} and {}",
            a.type_name(),
            b.type_name()
        )),
    }
}

fn vm_div(a: Value, b: Value) -> Result<Value, String> {
    match (&a, &b) {
        (Value::Integer(a), Value::Integer(b)) => {
            if *b == 0 {
                return Err("division by zero".into());
            }
            Ok(Value::Integer(a / b))
        }
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
        (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 / b)),
        (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a / *b as f64)),
        _ => Err(format!(
            "cannot divide {} and {}",
            a.type_name(),
            b.type_name()
        )),
    }
}

fn vm_rem(a: Value, b: Value) -> Result<Value, String> {
    match (&a, &b) {
        (Value::Integer(a), Value::Integer(b)) => {
            if *b == 0 {
                return Err("modulo by zero".into());
            }
            Ok(Value::Integer(a % b))
        }
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a % b)),
        (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 % b)),
        (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a % *b as f64)),
        _ => Err(format!(
            "cannot compute modulo of {} and {}",
            a.type_name(),
            b.type_name()
        )),
    }
}

fn vm_neg(v: Value) -> Value {
    match v {
        Value::Integer(n) => Value::Integer(-n),
        Value::Float(n) => Value::Float(-n),
        v => v,
    }
}

fn vm_bitand(a: Value, b: Value) -> Result<Value, String> {
    match (&a, &b) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a & b)),
        _ => Err(format!("bitwise AND requires integers, got {} and {}", a.type_name(), b.type_name())),
    }
}

fn vm_bitor(a: Value, b: Value) -> Result<Value, String> {
    match (&a, &b) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a | b)),
        _ => Err(format!("bitwise OR requires integers, got {} and {}", a.type_name(), b.type_name())),
    }
}

fn vm_bitxor(a: Value, b: Value) -> Result<Value, String> {
    match (&a, &b) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a ^ b)),
        _ => Err(format!("bitwise XOR requires integers, got {} and {}", a.type_name(), b.type_name())),
    }
}

fn vm_shl(a: Value, b: Value) -> Result<Value, String> {
    match (&a, &b) {
        (Value::Integer(a), Value::Integer(b)) => {
            Ok(Value::Integer(a.wrapping_shl(*b as u32)))
        }
        _ => Err(format!("shift left requires integers, got {} and {}", a.type_name(), b.type_name())),
    }
}

fn vm_shr(a: Value, b: Value) -> Result<Value, String> {
    match (&a, &b) {
        (Value::Integer(a), Value::Integer(b)) => {
            Ok(Value::Integer(a.wrapping_shr(*b as u32)))
        }
        _ => Err(format!("shift right requires integers, got {} and {}", a.type_name(), b.type_name())),
    }
}

fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.abs()
}

fn lcm(a: i64, b: i64) -> i64 {
    if a == 0 || b == 0 {
        0
    } else {
        (a / gcd(a, b)).abs() * b.abs()
    }
}

pub mod builtins;
#[cfg(test)]
mod tests;
