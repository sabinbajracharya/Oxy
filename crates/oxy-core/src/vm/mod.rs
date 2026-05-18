//! Stack-based virtual machine for executing compiled Oxy bytecode.
//!
//! The VM executes a flat sequence of [`OpCode`]s produced by the compiler.
//! It uses a value stack and a call stack. Each call frame tracks its own
//! local variable slots and return address.

use std::cell::RefCell;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::rc::Rc;

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
    FieldStore(String),
    /// Pop a value, push its display string — dispatches to Display::fmt natively
    /// if the receiver implements the trait, otherwise uses Rust Display.
    DisplayArg,
    /// Pop the current value at stack[base+slot], wrap it in Value::Cell,
    /// and store it back. Used for `let mut` to enable shared mutation.
    MakeCell(usize),
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
    /// Closure metadata: (param_names, body_expr, captured_vars_with_slots_and_mutable).
    pub closure_meta: Vec<(Vec<String>, crate::ast::Expr, Vec<(String, usize, bool)>)>,
    /// Local variable names: slot_index → name (for Eval env reconstruction of main).
    pub local_names: Vec<String>,
    /// Per-function local variable names: function entry IP → slot_names.
    pub fn_local_names: std::collections::HashMap<usize, Vec<String>>,
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
    output: Option<Rc<RefCell<Vec<String>>>>,
}

struct Frame {
    return_ip: usize,
    base: usize,
    /// Maximum slot index accessed + 1 (protects locals from Pop).
    max_slot: usize,
    /// Function entry IP (for looking up local variable names).
    fn_ip: usize,
    /// If this is a method call on a local, write self back to this slot on return.
    write_back_slot: Option<usize>,
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
        }
    }

    /// Create a VM that captures printed output (for testing).
    pub fn with_captured_output(chunk: Chunk) -> Self {
        let mut vm = Self::new(chunk);
        let shared = Rc::new(RefCell::new(Vec::new()));
        vm.output = Some(shared);
        vm
    }

    /// Get captured output lines (from shared buffer — already correctly ordered).
    pub fn captured_output(&self) -> Vec<String> {
        match &self.output {
            Some(rc) => rc.borrow().clone(),
            None => Vec::new(),
        }
    }

    /// Execute the chunk, starting at the entry point.
    pub fn run(&mut self) -> VmResult {
        self.ip = self.chunk.entry_point;

        // Push a synthetic top-level frame to protect locals from Pop
        self.call_stack.push(Frame {
            return_ip: 0,
            base: 0,
            max_slot: 0,
            write_back_slot: None,
            fn_ip: self.chunk.entry_point,
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
                    self.stack.push(val.deref_cell());
                }

                OpCode::StoreLocal(slot) => {
                    let val = self.stack.pop().unwrap_or(Value::Unit);
                    let base = self.frame_base();
                    let idx = base + slot;
                    if idx < self.stack.len() {
                        if let Value::Cell(rc) = &self.stack[idx] {
                            *rc.borrow_mut() = val;
                            if let Some(frame) = self.call_stack.last_mut() {
                                if slot + 1 > frame.max_slot { frame.max_slot = slot + 1; }
                            }
                            continue;
                        }
                    }
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

                OpCode::MakeCell(slot) => {
                    let base = self.frame_base();
                    let idx = base + slot;
                    if idx < self.stack.len() {
                        let val = self.stack[idx].clone();
                        self.stack[idx] = Value::cell(val);
                    }
                }

                OpCode::Add => {
                    match self.binary_op_native(vm_add, "add") {
                        Ok(true) => continue,
                        Err(e) => return VmResult::Error(e),
                        _ => {}
                    }
                }
                OpCode::Sub => {
                    match self.binary_op_native(vm_sub, "sub") {
                        Ok(true) => continue,
                        Err(e) => return VmResult::Error(e),
                        _ => {}
                    }
                }
                OpCode::Mul => {
                    match self.binary_op_native(vm_mul, "mul") {
                        Ok(true) => continue,
                        Err(e) => return VmResult::Error(e),
                        _ => {}
                    }
                }
                OpCode::Div => {
                    match self.binary_op_native(vm_div, "div") {
                        Ok(true) => continue,
                        Err(e) => return VmResult::Error(e),
                        _ => {}
                    }
                }
                OpCode::Mod => {
                    match self.binary_op_native(vm_rem, "rem") {
                        Ok(true) => continue,
                        Err(e) => return VmResult::Error(e),
                        _ => {}
                    }
                }
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
                        max_slot: arg_count,
                        write_back_slot: None,
                        fn_ip: target,
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
                    let key = self.stack.pop().unwrap_or(Value::Unit);
                    let collection = self.stack.pop().unwrap_or(Value::Unit);
                    // Handle Range-based slicing
                    if let Value::Range(start, end) = &key {
                        match collection {
                            Value::String(s) => {
                                let len = s.chars().count() as i64;
                                let s_idx = if *start < 0 { (len + start).max(0) } else { *start }.min(len) as usize;
                                let e_idx = if *end < 0 { (len + end).max(0) } else { *end }.min(len) as usize;
                                let slice: String = s.chars().skip(s_idx).take(e_idx.saturating_sub(s_idx)).collect();
                                self.stack.push(Value::String(slice));
                                continue;
                            }
                            Value::Vec(rc) => {
                                let vec = rc.borrow();
                                let len = vec.len() as i64;
                                let s_idx = if *start < 0 { (len + start).max(0) } else { *start }.min(len) as usize;
                                let e_idx = if *end < 0 { (len + end).max(0) } else { *end }.min(len) as usize;
                                let slice: Vec<Value> = vec[s_idx..e_idx].to_vec();
                                self.stack.push(Value::Vec(Rc::new(RefCell::new(slice))));
                                continue;
                            }
                            _ => return VmResult::Error(format!("cannot slice {}", collection.type_name())),
                        }
                    }
                    match collection {
                        Value::HashMap(rc) => {
                            match rc.borrow().get(&key).cloned() {
                                Some(val) => self.stack.push(val),
                                None => self.stack.push(Value::Unit),
                            }
                        }
                        Value::Vec(rc) => {
                            let idx = match key {
                                Value::Integer(i) => i as usize,
                                other => return VmResult::Error(format!("index must be integer, got {}", other.type_name())),
                            };
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
                            let idx = match key {
                                Value::Integer(i) => i as usize,
                                other => return VmResult::Error(format!("index must be integer, got {}", other.type_name())),
                            };
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
                            let idx = match key {
                                Value::Integer(i) => i as usize,
                                other => return VmResult::Error(format!("index must be integer, got {}", other.type_name())),
                            };
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
                        for (name, slot, is_mut) in &captured_vars {
                            let idx = base + slot;
                            let val = self.stack.get(idx).cloned().unwrap_or(Value::Unit);
                            closure_env.borrow_mut().define(name.clone(), val, *is_mut);
                        }
                    }
                    let captured_slots: Vec<(String, usize)> = captured_vars
                        .iter()
                        .map(|(name, slot, _)| (name.clone(), *slot))
                        .collect();
                    self.stack
                        .push(Value::Function(Box::new(crate::types::FunctionData {
                            name: "<closure>".into(),
                            params,
                            return_type: None,
                            body: body_block,
                            closure_env,
                            target_ip: Some(target_ip),
                            captured_slots,
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
                            // Save args and closure from stack
                            let args_start = self.stack.len() - arg_count - 1;
                            let saved: Vec<Value> = self.stack.drain(args_start..).collect();
                            // saved = [closure_fn, arg1, arg2, ...], skip closure_fn
                            let args = &saved[1..];

                            // Push captured vars at frame-relative positions (B + outer_slot)
                            let base = self.stack.len();
                            let mut max_slot = 0usize;
                            for (name, outer_slot) in &f.captured_slots {
                                let val = f.closure_env.borrow().get(name).ok().map(|v| v.clone()).unwrap_or(Value::Unit);
                                while base + outer_slot >= self.stack.len() {
                                    self.stack.push(Value::Unit);
                                }
                                self.stack[base + outer_slot] = val;
                                max_slot = max_slot.max(outer_slot + 1);
                            }
                            // Push args after captured vars
                            for arg in args {
                                self.stack.push(arg.clone());
                            }
                            self.call_stack.push(Frame {
                                return_ip: self.ip + 1,
                                base,
                                max_slot: max_slot.max(arg_count),
                                fn_ip: target,
                                write_back_slot: None,
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
                            Value::Char(c) => Value::Float(c as u32 as f64),
                            v => v,
                        },
                        1 => match val {
                            Value::Float(n) => Value::Integer(n as i64),
                            Value::Char(c) => Value::Integer(c as u32 as i64),
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
                        write_back_slot: None,
                                fn_ip: target,
                            });
                            self.ip = target;
                            continue;
                        }
                        None => {
                            // Handle built-in methods (Vec, String, HashMap, etc.)
                            match self.builtin_method(receiver.clone(), &method_name, args.clone()) {
                                Ok(val) => self.stack.push(val),
                                Err(e) => return VmResult::Error(e),
                            }
                        }
                    }
                }

                OpCode::ToString => {
                    let val = self.stack.pop().unwrap_or(Value::Unit);
                    if self.try_display_trait_dispatch(val) {
                        continue;
                    }
                }

                OpCode::DisplayArg => {
                    let val = self.stack.pop().unwrap_or(Value::Unit);
                    if self.try_display_trait_dispatch(val) {
                        continue;
                    }
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
                        if let Some(pos) = result.find("{:?}") {
                            result.replace_range(
                                pos..pos + 4,
                                &debug_format(val),
                            );
                        } else if let Some(pos) = result.find("{}") {
                            result.replace_range(pos..pos + 2, &val.to_string());
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
                    // Insert rather than assign: assigning at the stack top puts
                    // the value back on top where the next BindIdent would pop it.
                    // insert shifts elements right so remaining data stays above.
                    // Only use insert when actually needed (idx at or near top).
                    if idx + 1 >= self.stack.len() && !self.stack.is_empty() {
                        self.stack.insert(idx, val);
                    } else {
                        self.stack[idx] = val;
                    }
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
                            // Push in reverse: bind_pattern_data processes fields
                            // left-to-right but pops from top (LIFO), so we push
                            // rightmost field first so leftmost is on top
                            for d in data.iter().rev() {
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

            }

            self.ip += 1;
        }
    }

    /// Try native op first; if it fails, dispatch to trait method (operator overloading).
    /// Returns `true` if `continue` should be called (trait method call set up).
    fn binary_op_native(
        &mut self,
        f: fn(Value, Value) -> Result<Value, String>,
        method: &str,
    ) -> Result<bool, String> {
        let (a, b) = self.pop_two();
        match f(a.clone(), b.clone()) {
            Ok(v) => {
                self.stack.push(v);
                Ok(false)
            }
            Err(e) => {
                // Only try operator overloading for struct/enum variant types.
                // For primitives, the error is genuine (e.g. division by zero) — propagate it.
                let a_name = match &a {
                    Value::Struct { name, .. } => Some(name.clone()),
                    Value::EnumVariant { enum_name, .. } => Some(enum_name.clone()),
                    _ => None,
                };
                let b_name = match &b {
                    Value::Struct { name, .. } => Some(name.clone()),
                    Value::EnumVariant { enum_name, .. } => Some(enum_name.clone()),
                    _ => None,
                };
                // Try a's operator overloading
                if let Some(ref name) = a_name {
                    let key = (name.clone(), method.to_string());
                    if let Some(&target) = self.chunk.method_ips.get(&key) {
                        self.stack.push(a.clone());
                        self.stack.push(b.clone());
                        if self.call_stack.len() < 1024 {
                            let base = self.stack.len() - 2;
                            self.call_stack.push(Frame {
                                return_ip: self.ip + 1,
                                base,
                                max_slot: 2,
                                write_back_slot: None,
                                fn_ip: target,
                            });
                            self.ip = target;
                            return Ok(true);
                        }
                    }
                }
                // Try b's operator overloading
                if let Some(ref name) = b_name {
                    let key = (name.clone(), method.to_string());
                    if let Some(&target) = self.chunk.method_ips.get(&key) {
                        self.stack.push(b.clone());
                        self.stack.push(a.clone());
                        if self.call_stack.len() < 1024 {
                            let base = self.stack.len() - 2;
                            self.call_stack.push(Frame {
                                return_ip: self.ip + 1,
                                base,
                                max_slot: 2,
                                write_back_slot: None,
                                fn_ip: target,
                            });
                            self.ip = target;
                            return Ok(true);
                        }
                    }
                }
                // No operator overloading available
                if a_name.is_none() && b_name.is_none() {
                    Err(e)
                } else {
                    self.stack.push(a);
                    self.stack.push(b);
                    self.stack.push(Value::Unit);
                    Ok(false)
                }
            }
        }
    }

    /// Try to call the Display::fmt method natively. Returns true if dispatch was set up.
    fn try_display_trait_dispatch(&mut self, val: Value) -> bool {
        let struct_name = match &val {
            Value::Struct { name, .. } => name.clone(),
            _ => {
                self.stack.push(Value::String(val.to_string()));
                return false;
            }
        };
        if let Some(&target) = self.chunk.method_ips.get(&(struct_name, "fmt".to_string())) {
            self.stack.push(val.clone());
            if self.call_stack.len() < 1024 {
                let base = self.stack.len() - 1;
                self.call_stack.push(Frame {
                    return_ip: self.ip + 1,
                    base,
                    max_slot: 1,
                    fn_ip: target,
                    write_back_slot: None,
                });
                self.ip = target;
                return true;
            }
        }
        self.stack.push(Value::String(val.to_string()));
        false
    }

    fn binary_op(&mut self, f: fn(Value, Value) -> Result<Value, String>) {
        let (a, b) = self.pop_two();
        match f(a.clone(), b.clone()) {
            Ok(v) => self.stack.push(v),
            Err(_) => {
                // Operator overloading: look up trait method on receiver type
                let method = method_name_from_op(f);
                let struct_name = match &a {
                    Value::Struct { name, .. } => name.clone(),
                    Value::EnumVariant { enum_name, .. } => enum_name.clone(),
                    _ => String::new(),
                };
                if !struct_name.is_empty() {
                    if let Some(&target) = self.chunk.method_ips.get(&(struct_name, method.to_string())) {
                        // Call the trait method natively via VM call stack
                        self.stack.push(a);
                        self.stack.push(b);
                        if self.call_stack.len() >= 1024 {
                            self.stack.push(Value::Unit);
                            return;
                        }
                        let base = self.stack.len() - 2;
                        self.call_stack.push(Frame {
                            return_ip: self.ip + 1,
                            base,
                            max_slot: 2,
                                write_back_slot: None,
                            fn_ip: target,
                        });
                        self.ip = target - 1; // -1 because loop does ip += 1
                        return;
                    }
                }
                self.stack.push(Value::Unit);
            }
        }
    }

    /// Execute a single opcode. Shared by inner loops.
    fn execute_op(&mut self, op: OpCode) -> Result<(), String> {
        match op {
            OpCode::ConstUnit => self.stack.push(Value::Unit),
            OpCode::ConstBool(b) => self.stack.push(Value::Bool(b)),
            OpCode::ConstInt(n) => self.stack.push(Value::Integer(n)),
            OpCode::ConstFloat(f) => self.stack.push(Value::Float(f)),
            OpCode::ConstString(s) => self.stack.push(Value::String(s)),
            OpCode::ConstChar(c) => self.stack.push(Value::Char(c)),
            OpCode::Pop => { self.stack.pop(); }
            OpCode::Dup => { let v = self.stack.last().cloned().unwrap_or(Value::Unit); self.stack.push(v); }
            OpCode::Not | OpCode::Neg => {
                let v = self.stack.pop().unwrap_or(Value::Unit);
                match (&op, v) {
                    (OpCode::Not, v) => self.stack.push(Value::Bool(!v.is_truthy())),
                    (_, Value::Integer(n)) => self.stack.push(Value::Integer(-n)),
                    (_, Value::Float(n)) => self.stack.push(Value::Float(-n)),
                    (_, other) => self.stack.push(other),
                }
            }
            OpCode::Add => { let (a,b)=self.pop_two(); self.stack.push(vm_add(a,b)?); }
            OpCode::Sub => { let (a,b)=self.pop_two(); self.stack.push(vm_sub(a,b)?); }
            OpCode::Mul => { let (a,b)=self.pop_two(); self.stack.push(vm_mul(a,b)?); }
            OpCode::Div => { let (a,b)=self.pop_two(); self.stack.push(vm_div(a,b)?); }
            OpCode::Mod => { let (a,b)=self.pop_two(); self.stack.push(vm_rem(a,b)?); }
            OpCode::Eq => { let (a,b)=self.pop_two(); self.stack.push(Value::Bool(a==b)); }
            OpCode::Neq => { let (a,b)=self.pop_two(); self.stack.push(Value::Bool(a!=b)); }
            OpCode::Lt => { let (a,b)=self.pop_two(); self.stack.push(Value::Bool(a<b)); }
            OpCode::Gt => { let (a,b)=self.pop_two(); self.stack.push(Value::Bool(a>b)); }
            OpCode::Le => { let (a,b)=self.pop_two(); self.stack.push(Value::Bool(a<=b)); }
            OpCode::Ge => { let (a,b)=self.pop_two(); self.stack.push(Value::Bool(a>=b)); }
            OpCode::And => { let (a,b)=self.pop_two(); self.stack.push(Value::Bool(a.is_truthy()&&b.is_truthy())); }
            OpCode::Or => { let (a,b)=self.pop_two(); self.stack.push(Value::Bool(a.is_truthy()||b.is_truthy())); }
            OpCode::Jump(t) => self.ip = t,
            OpCode::JumpIfTrue(t) => { if self.stack.pop().unwrap_or(Value::Unit).is_truthy() { self.ip = t; } }
            OpCode::JumpIfFalse(t) => { if !self.stack.pop().unwrap_or(Value::Unit).is_truthy() { self.ip = t; } }
            OpCode::LoadLocal(slot) => {
                let base = self.frame_base(); let idx = base + slot;
                let v = self.stack.get(idx).cloned().unwrap_or(Value::Unit);
                self.stack.push(v.deref_cell());
            }
            OpCode::StoreLocal(slot) => {
                let val = self.stack.pop().unwrap_or(Value::Unit);
                let base = self.frame_base(); let idx = base + slot;
                if idx < self.stack.len() { if let Value::Cell(rc) = &self.stack[idx] { *rc.borrow_mut() = val; return Ok(()); } }
                while idx >= self.stack.len() { self.stack.push(Value::Unit); }
                self.stack[idx] = val;
            }
            OpCode::BindIdent(slot) => {
                let val = self.stack.pop().unwrap_or(Value::Unit);
                let base = self.frame_base(); let idx = base + slot;
                if idx + 1 >= self.stack.len() && !self.stack.is_empty() { self.stack.insert(idx, val); }
                else { while idx >= self.stack.len() { self.stack.push(Value::Unit); } self.stack[idx] = val; }
            }
            OpCode::Print => { let v = self.stack.pop().unwrap_or(Value::Unit); self.write_output(&v.to_string()); }
            OpCode::PrintLn => { let v = self.stack.pop().unwrap_or(Value::Unit); self.write_output(&format!("{}", v)); self.write_output("\n"); }
            OpCode::ToString => { let v = self.stack.pop().unwrap_or(Value::Unit); self.stack.push(Value::String(v.to_string())); }
            OpCode::MakeArray{count} => { let s=self.stack.len()-count; let i:Vec<_>=self.stack.drain(s..).collect(); self.stack.push(Value::Vec(Rc::new(RefCell::new(i)))); }
            OpCode::MakeTuple{count} => { let s=self.stack.len()-count; let i:Vec<_>=self.stack.drain(s..).collect(); self.stack.push(Value::Tuple(i)); }
            OpCode::VecIndex => {
                let key = self.stack.pop().unwrap_or(Value::Unit);
                let c = self.stack.pop().unwrap_or(Value::Unit);
                match c {
                    Value::Vec(rc) => { if let Value::Integer(i) = key { self.stack.push(rc.borrow().get(i as usize).cloned().unwrap_or(Value::Unit)); } else { self.stack.push(Value::Unit); } }
                    Value::HashMap(rc) => { self.stack.push(rc.borrow().get(&key).cloned().unwrap_or(Value::Unit)); }
                    Value::Tuple(t) => { if let Value::Integer(i) = key { self.stack.push(t.get(i as usize).cloned().unwrap_or(Value::Unit)); } else { self.stack.push(Value::Unit); } }
                    Value::String(s) => { if let Value::Integer(i) = key { self.stack.push(s.chars().nth(i as usize).map(Value::Char).unwrap_or(Value::Unit)); } else { self.stack.push(Value::Unit); } }
                    _ => self.stack.push(Value::Unit),
                }
            }
            OpCode::FieldAccess{field_name} => {
                let v = self.stack.pop().unwrap_or(Value::Unit);
                match v { Value::Struct{fields,..} => self.stack.push(fields.get(&field_name).cloned().unwrap_or(Value::Unit)), _ => self.stack.push(Value::Unit) }
            }
            OpCode::FieldStore(field_name) => {
                let val = self.stack.pop().unwrap_or(Value::Unit);
                let recv = self.stack.pop().unwrap_or(Value::Unit);
                match recv { Value::Struct{name,mut fields} => { fields.insert(field_name, val); self.stack.push(Value::Struct{name,fields}); } other => self.stack.push(other) }
            }
            OpCode::MakeEnumVariant{enum_name,variant,arg_count} => { let s=self.stack.len()-arg_count; let d=self.stack.drain(s..).collect(); self.stack.push(Value::EnumVariant{enum_name,variant,data:d}); }
            OpCode::EnumVariantEqual{enum_name,variant} => {
                let val = self.stack.pop().unwrap_or(Value::Unit);
                match &val { Value::EnumVariant{enum_name:en,variant:v,data} if en==&enum_name&&v==&variant => { for d in data.iter().rev() { self.stack.push(d.clone()); } self.stack.push(Value::Bool(true)); } _ => { self.stack.push(Value::Bool(false)); } }
            }
            OpCode::MakeRange => { let (e,s) = self.pop_two(); let si=match s{Value::Integer(n)=>n,_=>0}; let ei=match e{Value::Integer(n)=>n,_=>0}; self.stack.push(Value::Range(si,ei)); }
            OpCode::Format{arg_count} => {
                let s=self.stack.len()-arg_count; let args:Vec<_>=self.stack.drain(s..).collect();
                let mut r=args.first().map(|v|v.to_string()).unwrap_or_default();
                for v in &args[1..] { if let Some(p)=r.find("{:?}") { r.replace_range(p..p+4,&debug_format(v)); } else if let Some(p)=r.find("{}") { r.replace_range(p..p+2,&v.to_string()); } }
                self.stack.push(Value::String(r));
            }
            OpCode::FStringConcat{count} => { let s=self.stack.len()-count; let p:Vec<String>=self.stack.drain(s..).map(|v|v.to_string()).collect(); self.stack.push(Value::String(p.concat())); }
            OpCode::Cast(target) => {
                let v=self.stack.pop().unwrap_or(Value::Unit);
                let r=match target{0=>match v{Value::Integer(n)=>Value::Float(n as f64),Value::Char(c)=>Value::Float(c as u32 as f64),v=>v},1=>match v{Value::Float(n)=>Value::Integer(n as i64),Value::Char(c)=>Value::Integer(c as u32 as i64),v=>v},2=>match v{Value::Integer(n)=>Value::Char(char::from_u32(n as u32).unwrap_or('\0')),v=>v},_=>v};
                self.stack.push(r);
            }
            OpCode::TryPop => {
                let v=self.stack.pop().unwrap_or(Value::Unit);
                let is_err=matches!(&v,Value::EnumVariant{enum_name,variant,..}if(enum_name=="Result"&&variant=="Err")||(enum_name=="Option"&&variant=="None"));
                if is_err { return Err(format!("{}",v)); }
                match &v { Value::EnumVariant{data,..} if !data.is_empty() => self.stack.push(data[0].clone()), _ => {} }
            }
            OpCode::DisplayArg => { let v=self.stack.pop().unwrap_or(Value::Unit); self.stack.push(Value::String(v.to_string())); }
            OpCode::MakeCell(slot) => { let b=self.frame_base(); let i=b+slot; if i<self.stack.len() { let v=self.stack[i].clone(); self.stack[i]=Value::cell(v); } }
            _ => return Err(format!("execute_op: unhandled {:?}", op)),
        }
        Ok(())
    }

    /// Built-in method dispatch (Vec, String, HashMap, Option, Result, etc.).
    /// Call a compiled closure natively through the VM, returning its result.
    /// Used by iterator builtins (for_each, map, sort_by, etc.) for closure args.
    fn run_closure(&mut self, func: &Value, args: &[Value]) -> Result<Value, String> {
        let ft = match func {
            Value::Function(f) => f.clone(),
            _ => return Err("not a callable function".into()),
        };
        let target = match ft.target_ip { Some(t) => t, None => return Err("function has no bytecode target".into()) };
        // Save outer execution state
        let saved_ip = self.ip;
        let saved_stack_len = self.stack.len();
        let saved_call_depth = self.call_stack.len();
        // Push args and captured vars onto stack
        let base = self.stack.len();
        for (name, slot) in &ft.captured_slots {
            let val = ft.closure_env.borrow().get(name).ok().map(|v| v.clone()).unwrap_or(Value::Unit);
            while base + slot >= self.stack.len() { self.stack.push(Value::Unit); }
            self.stack[base + slot] = val;
        }
        let max_slot = ft.captured_slots.iter().map(|(_, s)| s + 1).max().unwrap_or(0);
        for arg in args { self.stack.push(arg.clone()); }
        // Push call frame and run
        self.call_stack.push(Frame {
            return_ip: usize::MAX, // sentinel
            base,
            max_slot: max_slot.max(args.len()),
            fn_ip: target,
            write_back_slot: None,
        });
        self.ip = target;
        // Inner loop — runs until the closure's top-level Return
        let result = loop {
            let op = match self.chunk.code.get(self.ip) {
                Some(op) => op.clone(),
                None => break Err("unexpected end of code in closure".into()),
            };
            self.ip += 1;
            match op {
                OpCode::Call { target: ct, arg_count } => {
                    if self.call_stack.len() >= 1024 { break Err("recursion limit exceeded".into()); }
                    let as_start = self.stack.len() - arg_count;
                    self.call_stack.push(Frame { return_ip: self.ip, base: as_start, max_slot: arg_count, fn_ip: ct, write_back_slot: None });
                    self.ip = ct;
                }
                OpCode::Return => {
                    let rv = self.stack.pop().unwrap_or(Value::Unit);
                    let frame = self.call_stack.pop().unwrap();
                    if frame.return_ip == usize::MAX { break Ok(rv); }
                    self.stack.truncate(frame.base);
                    self.stack.push(rv);
                    self.ip = frame.return_ip;
                }
                OpCode::Closure { target_ip: ct, param_count, meta_idx } => {
                    let blank_span = crate::lexer::Span { start: 0, end: 0, line: 0, column: 0 };
                    let (param_names, body_expr, captured_vars) = self.chunk.closure_meta.get(meta_idx).cloned().unwrap_or_else(|| ((0..param_count).map(|i| format!("_{i}")).collect(), crate::ast::Expr::IntLiteral(0, blank_span), Vec::new()));
                    let params: Vec<crate::ast::Param> = param_names.into_iter().map(|name| crate::ast::Param { name, type_ann: crate::ast::TypeAnnotation { name: "_".into(), span: blank_span }, span: blank_span }).collect();
                    let body_block = crate::ast::Block { stmts: vec![crate::ast::Stmt::Expr { expr: body_expr, has_semicolon: false }], span: blank_span };
                    let mut closure_env = crate::env::Environment::new();
                    for (name, slot, is_mut) in &captured_vars {
                        let idx = self.frame_base() + slot;
                        let val = self.stack.get(idx).cloned().unwrap_or(Value::Unit);
                        closure_env.borrow_mut().define(name.clone(), val, *is_mut);
                    }
                    let cap_slots: Vec<(String, usize)> = captured_vars.iter().map(|(n, s, _)| (n.clone(), *s)).collect();
                    self.stack.push(Value::Function(Box::new(crate::types::FunctionData { name: "<closure>".into(), params, return_type: None, body: body_block, closure_env, target_ip: Some(ct), captured_slots: cap_slots })));
                }
                _ => {
                    if let Err(e) = self.execute_op(op) { break Err(e); }
                }
            }
        };
        // Restore outer execution state
        self.stack.truncate(saved_stack_len);
        self.call_stack.truncate(saved_call_depth);
        self.ip = saved_ip;
        result
    }

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
                    self.run_closure(func, fargs)
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
            Value::EnumVariant { enum_name, .. } => match method_name {
                "clone" => Ok(receiver.clone()),
                "to_string" => Ok(Value::String(receiver.to_string())),
                _ => Err(format!("no method '{}' on enum '{}'", method_name, enum_name)),
            },
            Value::Struct { name, .. } => match method_name {
                "clone" => Ok(receiver.clone()),
                "to_string" => Ok(Value::String(receiver.to_string())),
                _ => Err(format!("no method '{}' on struct '{}'", method_name, name)),
            },
            // Iterator: native adapters + consumers via builtins.
            Value::Iterator(_) => {
                builtins::iterator::dispatch(receiver, method_name, &args, |func, fargs| {
                    self.run_closure(func, fargs)
                })
            }
            Value::Tuple(_) => match method_name {
                "clone" => Ok(receiver.clone()),
                "to_string" => Ok(Value::String(receiver.to_string())),
                _ => Err(format!("no method '{}' on type tuple", method_name)),
            },
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
                match crate::json::deserialize(&s) {
                    Ok(val) => Ok(Value::ok(val)),
                    Err(e) => Ok(Value::err(Value::String(format!("json::parse: {}", e)))),
                }
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
            ["BinaryHeap", "new"] => Ok(Value::BinaryHeap(std::rc::Rc::new(std::cell::RefCell::new(BinaryHeap::new())))),
            ["VecDeque", "new"] => Ok(Value::VecDeque(std::rc::Rc::new(std::cell::RefCell::new(VecDeque::new())))),
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
                let trimmed = s.trim();
                let result = if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
                    i64::from_str_radix(&trimmed[2..], 16).map_err(|_| ())
                } else {
                    trimmed.parse::<i64>().map_err(|_| ())
                };
                match result {
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
            // db and server are feature-gated
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
        if let Some(ref output_rc) = self.output {
            let mut output = output_rc.borrow_mut();
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

/// Map a binary op function to the corresponding method name for trait dispatch.
fn method_name_from_op(f: fn(Value, Value) -> Result<Value, String>) -> &'static str {
    if f as usize == vm_add as usize { return "add"; }
    if f as usize == vm_sub as usize { return "sub"; }
    if f as usize == vm_mul as usize { return "mul"; }
    if f as usize == vm_div as usize { return "div"; }
    if f as usize == vm_rem as usize { return "rem"; }
    "add"
}

// --- VM arithmetic helpers (standalone to avoid trait conflicts) ---

fn vm_add(a: Value, b: Value) -> Result<Value, String> {
    match (&a, &b) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
        (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
        (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a + *b as f64)),
        (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{a}{b}"))),
        (Value::String(a), other) => Ok(Value::String(format!("{a}{other}"))),
        (other, Value::String(b)) => Ok(Value::String(format!("{other}{b}"))),
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

/// Debug format a value (like Rust's `{:?}`). Moved here from interpreter/format.rs.
fn debug_format(val: &Value) -> String {
    match val {
        Value::String(s) => format!("\"{s}\""),
        Value::Char(c) => format!("'{c}'"),
        Value::Vec(rc) => {
            let items: Vec<String> = rc.borrow().iter().map(debug_format).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Tuple(t) => {
            let items: Vec<String> = t.iter().map(debug_format).collect();
            if t.len() == 1 { format!("({},)", items[0]) } else { format!("({})", items.join(", ")) }
        }
        Value::Struct { name, fields } => {
            let mut sorted: Vec<_> = fields.iter().collect();
            sorted.sort_by_key(|(k, _)| (*k).clone());
            let items: Vec<String> = sorted.iter().map(|(k, v)| format!("{k}: {}", debug_format(v))).collect();
            format!("{name} {{ {} }}", items.join(", "))
        }
        Value::EnumVariant { enum_name, variant, data } => {
            let prefix = if enum_name == "Option" || enum_name == "Result" { String::new() } else { format!("{enum_name}::") };
            if data.is_empty() { format!("{prefix}{variant}") }
            else {
                let items: Vec<String> = data.iter().map(debug_format).collect();
                format!("{prefix}{variant}({})", items.join(", "))
            }
        }
        Value::HashMap(rc) => {
            let m = rc.borrow();
            let mut sorted: Vec<_> = m.iter().collect();
            sorted.sort_by_key(|(k, _)| (*k).clone());
            let items: Vec<String> = sorted.iter().map(|(k, v)| format!("{}: {}", debug_format(&Value::String(k.to_string())), debug_format(v))).collect();
            format!("{{{}}}", items.join(", "))
        }
        Value::BinaryHeap(rc) => {
            let sorted = rc.borrow().clone().into_sorted_vec();
            let items: Vec<String> = sorted.iter().map(debug_format).collect();
            format!("BinaryHeap([{}])", items.join(", "))
        }
        Value::VecDeque(rc) => {
            let items: Vec<String> = rc.borrow().iter().map(debug_format).collect();
            format!("VecDeque([{}])", items.join(", "))
        }
        Value::Future(f) => format!("Future<{}>", f.name),
        Value::JoinHandle(v) => format!("JoinHandle({})", debug_format(v)),
        Value::Cell(rc) => debug_format(&rc.borrow()),
        other => format!("{other}"),
    }
}

/// Compile and run with captured output (for testing).
pub fn run_compiled(source: &str) -> Result<Value, crate::errors::FerriError> {
    let program = crate::parser::parse(source)?;
    crate::type_checker::TypeChecker::new().check_program(&program)?;
    let chunk = crate::compiler::Compiler::new().compile(&program)?;
    let mut vm = Vm::new(chunk);
    match vm.run() {
        VmResult::Value(v) => Ok(v),
        VmResult::Error(e) => Err(crate::errors::FerriError::Runtime { message: e, line: 0, column: 0 }),
    }
}

/// Compile and run, capturing printed output (for testing).
pub fn run_compiled_capturing(source: &str) -> Result<(Value, Vec<String>), crate::errors::FerriError> {
    let program = crate::parser::parse(source)?;
    crate::type_checker::TypeChecker::new().check_program(&program)?;
    let chunk = crate::compiler::Compiler::new().compile(&program)?;
    let mut vm = Vm::with_captured_output(chunk);
    match vm.run() {
        VmResult::Value(v) => Ok((v, vm.captured_output())),
        VmResult::Error(e) => Err(crate::errors::FerriError::Runtime { message: e, line: 0, column: 0 }),
    }
}

/// Run a program and capture its output (compatibility alias).
pub fn run_capturing(source: &str) -> Result<(Value, Vec<String>), crate::errors::FerriError> {
    run_compiled_capturing(source)
}

/// Run a program, return its value (compatibility alias).
pub fn run(source: &str) -> Result<Value, crate::errors::FerriError> {
    run_compiled(source)
}

/// Result of running a test suite.
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}

/// Run all #[test] functions in source via the VM.
pub fn run_tests(path: &str, source: &str) -> Result<Vec<TestResult>, crate::errors::FerriError> {
    let program = crate::parser::parse(source)?;
    crate::type_checker::TypeChecker::new().check_program(&program)?;
    let chunk = crate::compiler::Compiler::new_with_source_dir(Some(path)).compile(&program)?;
    let test_fns: Vec<&crate::ast::FnDef> = program.items.iter().filter_map(|item| {
        if let crate::ast::Item::Function(f) = item {
            if f.attributes.iter().any(|a| a.name == "test") { Some(f) } else { None }
        } else { None }
    }).collect();
    let mut results = Vec::new();
    for test_fn in &test_fns {
        let mut vm = Vm::new(chunk.clone());
        match vm.run() {
            VmResult::Value(_) => results.push(TestResult { name: test_fn.name.clone(), passed: true, error: None }),
            VmResult::Error(e) => results.push(TestResult { name: test_fn.name.clone(), passed: false, error: Some(e) }),
        }
    }
    Ok(results)
}

pub mod builtins;
#[cfg(test)]
mod tests;
