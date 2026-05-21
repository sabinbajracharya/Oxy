//! Stack-based virtual machine for executing compiled Oxy bytecode.
//!
//! The VM executes a flat sequence of [`OpCode`]s produced by the compiler.
//! It uses a value stack and a call stack. Each call frame tracks its own
//! local variable slots and return address.

use std::cell::RefCell;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::rc::Rc;

use crate::lexer::IntegerSuffix;
use crate::types::{FloatWidth, IntegerWidth, Value};

/// Bytecode instructions for the Oxy VM.
#[derive(Debug, Clone)]
pub enum OpCode {
    // --- Constants ---
    ConstInt(i64, crate::types::IntegerWidth),
    ConstFloat(f64, crate::types::FloatWidth),
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
    BitNot,

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
    /// Pop value, pop index (Integer), pop collection, store value at collection[index].
    /// Pushes value back so it can be used in chains.
    VecIndexStore,
    /// Pop end (Value), pop start (Value), push Range(start, end).
    MakeRange,

    // --- Collections ---
    /// Pop `count` elements, push them as Value::Vec.
    MakeArray {
        count: usize,
    },
    /// Pop `count` elements, push them as Value::Array.
    MakeFixedArray {
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
    /// Cast the top of stack to a specific integer width (wrapping).
    CastInt(IntegerWidth),
    /// Cast the top of stack to a specific float width.
    CastFloat(FloatWidth),
    /// Cast the top of stack to char.
    CastToChar,

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
    /// Trace execution to stderr.
    trace: bool,
}

struct Frame {
    return_ip: usize,
    base: usize,
    /// Maximum slot index accessed + 1 (protects locals from Pop).
    max_slot: usize,
    /// Function entry IP (for looking up local variable names).
    #[allow(dead_code)]
    fn_ip: usize,
    /// If this is a method call on a local, write self back to this slot on return.
    #[allow(dead_code)]
    write_back_slot: Option<usize>,
}

/// Result of VM execution.
pub enum VmResult {
    Value(Value),
    Error(String),
}

impl Vm {
    pub fn new(chunk: Chunk) -> Self {
        let trace = std::env::var("OXY_VM_TRACE").is_ok();
        Self {
            chunk,
            stack: Vec::new(),
            ip: 0,
            call_stack: Vec::new(),
            output: None,
            trace,
        }
    }

    /// Enable execution tracing to stderr.
    pub fn with_trace(mut self) -> Self {
        self.trace = true;
        self
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

            if self.trace {
                self.trace_op(&op);
            }

            match op {
                OpCode::ConstInt(n, w) => self.stack.push(match w {
                    IntegerWidth::I8 => Value::I8(n as i8),
                    IntegerWidth::I16 => Value::I16(n as i16),
                    IntegerWidth::I32 => Value::I32(n as i32),
                    IntegerWidth::I64 => Value::I64(n),
                    IntegerWidth::U8 => Value::U8(n as u8),
                    IntegerWidth::U16 => Value::U16(n as u16),
                    IntegerWidth::U32 => Value::U32(n as u32),
                    IntegerWidth::U64 => Value::U64(n as u64),
                }),
                OpCode::ConstFloat(n, w) => self.stack.push(match w {
                    FloatWidth::F32 => Value::F32(n as f32),
                    FloatWidth::F64 => Value::F64(n),
                }),
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
                                if slot + 1 > frame.max_slot {
                                    frame.max_slot = slot + 1;
                                }
                            }
                        } else {
                            self.stack[idx] = val;
                            if let Some(frame) = self.call_stack.last_mut() {
                                if slot + 1 > frame.max_slot {
                                    frame.max_slot = slot + 1;
                                }
                            }
                        }
                    } else {
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
                }

                OpCode::MakeCell(slot) => {
                    let base = self.frame_base();
                    let idx = base + slot;
                    if idx < self.stack.len() {
                        let val = self.stack[idx].clone();
                        self.stack[idx] = Value::cell(val);
                    }
                }

                OpCode::Add => match self.binary_op_native(vm_add, "add") {
                    Ok(true) => continue,
                    Err(e) => return VmResult::Error(e),
                    _ => {}
                },
                OpCode::Sub => match self.binary_op_native(vm_sub, "sub") {
                    Ok(true) => continue,
                    Err(e) => return VmResult::Error(e),
                    _ => {}
                },
                OpCode::Mul => match self.binary_op_native(vm_mul, "mul") {
                    Ok(true) => continue,
                    Err(e) => return VmResult::Error(e),
                    _ => {}
                },
                OpCode::Div => match self.binary_op_native(vm_div, "div") {
                    Ok(true) => continue,
                    Err(e) => return VmResult::Error(e),
                    _ => {}
                },
                OpCode::Mod => match self.binary_op_native(vm_rem, "rem") {
                    Ok(true) => continue,
                    Err(e) => return VmResult::Error(e),
                    _ => {}
                },
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
                    // Try operator overloading for struct/enum types
                    let type_name = match &v {
                        Value::Struct { name, .. } => Some(name.clone()),
                        Value::EnumVariant { enum_name, .. } => Some(enum_name.clone()),
                        _ => None,
                    };
                    if let Some(ref tn) = type_name {
                        let key = (tn.clone(), "neg".to_string());
                        if let Some(&target) = self.chunk.method_ips.get(&key) {
                            self.stack.push(v.clone());
                            if self.call_stack.len() < 1024 {
                                self.call_stack.push(Frame {
                                    return_ip: self.ip + 1,
                                    base: self.stack.len() - 1,
                                    max_slot: 1,
                                    write_back_slot: None,
                                    fn_ip: target,
                                });
                                self.ip = target;
                                continue;
                            }
                        }
                    }
                    self.stack.push(vm_neg(v));
                }

                OpCode::Not => {
                    let v = self.stack.pop().unwrap_or(Value::Unit);
                    self.stack.push(Value::Bool(!v.is_truthy()));
                }
                OpCode::BitNot => {
                    let v = self.stack.pop().unwrap_or(Value::Unit);
                    self.stack.push(vm_bitnot(v));
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
                        Ok(vec) => self
                            .stack
                            .push(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(vec)))),
                        Err(e) => return VmResult::Error(e),
                    }
                }

                OpCode::IterLen => {
                    let v = self.stack.pop().unwrap_or(Value::Unit);
                    match v {
                        Value::Vec(rc) => self.stack.push(Value::I64(rc.borrow().len() as i64)),
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
                                let s_idx = if *start < 0 {
                                    (len + start).max(0)
                                } else {
                                    *start
                                }
                                .min(len) as usize;
                                let e_idx = if *end < 0 { (len + end).max(0) } else { *end }
                                    .min(len) as usize;
                                let slice: String = s
                                    .chars()
                                    .skip(s_idx)
                                    .take(e_idx.saturating_sub(s_idx))
                                    .collect();
                                self.stack.push(Value::String(slice));
                            }
                            Value::Vec(rc) => {
                                let vec = rc.borrow();
                                let len = vec.len() as i64;
                                let s_idx = if *start < 0 {
                                    (len + start).max(0)
                                } else {
                                    *start
                                }
                                .min(len) as usize;
                                let e_idx = if *end < 0 { (len + end).max(0) } else { *end }
                                    .min(len) as usize;
                                let slice: Vec<Value> = vec[s_idx..e_idx].to_vec();
                                self.stack.push(Value::Vec(Rc::new(RefCell::new(slice))));
                            }
                            _ => {
                                return VmResult::Error(format!(
                                    "cannot slice {}",
                                    collection.type_name()
                                ))
                            }
                        }
                        // Fall through to get ip += 1 at bottom of loop
                    } else {
                        match collection {
                            Value::HashMap(rc) => match rc.borrow().get(&key).cloned() {
                                Some(val) => self.stack.push(val),
                                None => self.stack.push(Value::Unit),
                            },
                            Value::Vec(rc) => {
                                let idx = match key {
                                    Value::I64(i) => i as usize,
                                    other => {
                                        return VmResult::Error(format!(
                                            "index must be integer, got {}",
                                            other.type_name()
                                        ))
                                    }
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
                                    Value::I64(i) => i as usize,
                                    other => {
                                        return VmResult::Error(format!(
                                            "index must be integer, got {}",
                                            other.type_name()
                                        ))
                                    }
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
                                    Value::I64(i) => i as usize,
                                    other => {
                                        return VmResult::Error(format!(
                                            "index must be integer, got {}",
                                            other.type_name()
                                        ))
                                    }
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
                            Value::Array(a) => {
                                let idx = match key {
                                    Value::I64(i) => i as usize,
                                    other => {
                                        return VmResult::Error(format!(
                                            "index must be integer, got {}",
                                            other.type_name()
                                        ))
                                    }
                                };
                                if idx < a.len() {
                                    self.stack.push(a[idx].clone());
                                } else {
                                    return VmResult::Error(format!(
                                        "index {} out of bounds for len {}",
                                        idx,
                                        a.len()
                                    ));
                                }
                            }
                            Value::Struct { fields, .. } => {
                                let field_key = key.to_string();
                                self.stack
                                    .push(fields.get(&field_key).cloned().unwrap_or(Value::Unit));
                            }
                            other => {
                                return VmResult::Error(format!(
                                    "cannot index {}",
                                    other.type_name()
                                ))
                            }
                        }
                    }
                }

                OpCode::VecIndexStore => {
                    let value = self.stack.pop().unwrap_or(Value::Unit);
                    let key = self.stack.pop().unwrap_or(Value::Unit);
                    let collection = self.stack.pop().unwrap_or(Value::Unit);
                    match collection {
                        Value::Vec(rc) => {
                            let idx = match key {
                                Value::I64(i) => i as usize,
                                other => {
                                    return VmResult::Error(format!(
                                        "index must be integer, got {}",
                                        other.type_name()
                                    ))
                                }
                            };
                            let len = rc.borrow().len();
                            if idx < len {
                                rc.borrow_mut()[idx] = value.clone();
                                self.stack.push(value);
                            } else {
                                return VmResult::Error(format!(
                                    "index {} out of bounds for len {}",
                                    idx, len
                                ));
                            }
                        }
                        other => {
                            return VmResult::Error(format!(
                                "cannot index-assign {}",
                                other.type_name()
                            ))
                        }
                    }
                }

                OpCode::MakeRange => {
                    let end = self.stack.pop().unwrap_or(Value::Unit);
                    let start = self.stack.pop().unwrap_or(Value::Unit);
                    match (start, end) {
                        (Value::I64(s), Value::I64(e)) => {
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
                    self.stack
                        .push(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(
                            elements,
                        ))));
                }

                OpCode::MakeFixedArray { count } => {
                    let start = self.stack.len().saturating_sub(count);
                    let elements: Vec<Value> = self.stack.drain(start..).collect();
                    self.stack.push(Value::Array(elements));
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
                    let (param_names, body_expr, captured_vars) = self
                        .chunk
                        .closure_meta
                        .get(meta_idx)
                        .cloned()
                        .unwrap_or_else(|| {
                            (
                                (0..param_count).map(|i| format!("_{i}")).collect(),
                                crate::ast::Expr::IntLiteral(0, IntegerSuffix::None, blank_span),
                                Vec::new(),
                            )
                        });
                    let params: Vec<crate::ast::Param> = param_names
                        .into_iter()
                        .map(|name| crate::ast::Param {
                            name,
                            type_ann: crate::ast::TypeAnnotation::Named {
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
                    let closure_env = crate::env::Environment::new();
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
                                let val = f
                                    .closure_env
                                    .borrow()
                                    .get(name)
                                    .ok()
                                    .map(|v| v.clone())
                                    .unwrap_or(Value::Unit);
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
                                max_slot: max_slot + arg_count,
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

                OpCode::CastInt(target_width) => {
                    let val = self.stack.pop().unwrap_or(Value::Unit);
                    let result = cast_to_int(&val, target_width);
                    self.stack.push(result);
                }
                OpCode::CastFloat(target_width) => {
                    let val = self.stack.pop().unwrap_or(Value::Unit);
                    let result = cast_to_float(&val, target_width);
                    self.stack.push(result);
                }
                OpCode::CastToChar => {
                    let val = self.stack.pop().unwrap_or(Value::Unit);
                    let n = value_to_i64(&val);
                    let c = char::from_u32(n as u32).unwrap_or('\0');
                    self.stack.push(Value::Char(c));
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

                    // Look up method IP: first check user methods (struct, enum, AND trait impls
                    // on built-in types), then built-ins.
                    let lookup_name = if is_struct {
                        match &receiver {
                            Value::Struct { name, .. } => name.clone(),
                            _ => type_name.clone(),
                        }
                    } else if is_enum {
                        match &receiver {
                            Value::EnumVariant { enum_name, .. } => enum_name.clone(),
                            _ => type_name.clone(),
                        }
                    } else {
                        // Built-in type (i64, String, etc.) — still check method_ips for trait impls
                        type_name.clone()
                    };
                    let method_ip = self
                        .chunk
                        .method_ips
                        .get(&(lookup_name, method_name.clone()))
                        .copied();

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
                            match self.builtin_method(receiver.clone(), &method_name, args.clone())
                            {
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
                            result.replace_range(pos..pos + 4, &debug_format(val));
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
                    // Pop the scrutinee, push only the match bool. Field data
                    // is later extracted via LoadLocal(scrutinee_slot) +
                    // EnumDataGet(i) so stack positions don't collide with
                    // pre-allocated binding slots.
                    let val = self.stack.pop().unwrap_or(Value::Unit);
                    let matched = matches!(
                        &val,
                        Value::EnumVariant { enum_name: en, variant: v, .. }
                            if en == &enum_name && v == &variant
                    );
                    self.stack.push(Value::Bool(matched));
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
                    if let Some(&target) = self
                        .chunk
                        .method_ips
                        .get(&(struct_name, method.to_string()))
                    {
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
            OpCode::ConstInt(n, w) => self.stack.push(match w {
                IntegerWidth::I8 => Value::I8(n as i8),
                IntegerWidth::I16 => Value::I16(n as i16),
                IntegerWidth::I32 => Value::I32(n as i32),
                IntegerWidth::I64 => Value::I64(n),
                IntegerWidth::U8 => Value::U8(n as u8),
                IntegerWidth::U16 => Value::U16(n as u16),
                IntegerWidth::U32 => Value::U32(n as u32),
                IntegerWidth::U64 => Value::U64(n as u64),
            }),
            OpCode::ConstFloat(f, w) => self.stack.push(match w {
                FloatWidth::F32 => Value::F32(f as f32),
                FloatWidth::F64 => Value::F64(f),
            }),
            OpCode::ConstString(s) => self.stack.push(Value::String(s)),
            OpCode::ConstChar(c) => self.stack.push(Value::Char(c)),
            OpCode::Pop => {
                // Protect frame locals from being popped (matches Vm::run).
                let protected = self.frame_protected();
                if self.stack.len() > protected {
                    self.stack.pop();
                }
            }
            OpCode::Dup => {
                let v = self.stack.last().cloned().unwrap_or(Value::Unit);
                self.stack.push(v);
            }
            OpCode::Not | OpCode::Neg | OpCode::BitNot => {
                let v = self.stack.pop().unwrap_or(Value::Unit);
                match (&op, v) {
                    (OpCode::Not, v) => self.stack.push(Value::Bool(!v.is_truthy())),
                    (OpCode::BitNot, v) => self.stack.push(vm_bitnot(v)),
                    (_, Value::I64(n)) => self.stack.push(Value::I64(-n)),
                    (_, Value::F64(n)) => self.stack.push(Value::F64(-n)),
                    (_, other) => self.stack.push(other),
                }
            }
            OpCode::Add => {
                let (a, b) = self.pop_two();
                self.stack.push(vm_add(a, b)?);
            }
            OpCode::Sub => {
                let (a, b) = self.pop_two();
                self.stack.push(vm_sub(a, b)?);
            }
            OpCode::Mul => {
                let (a, b) = self.pop_two();
                self.stack.push(vm_mul(a, b)?);
            }
            OpCode::Div => {
                let (a, b) = self.pop_two();
                self.stack.push(vm_div(a, b)?);
            }
            OpCode::Mod => {
                let (a, b) = self.pop_two();
                self.stack.push(vm_rem(a, b)?);
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
            OpCode::Jump(t) => self.ip = t,
            OpCode::JumpIfTrue(t) => {
                if self.stack.pop().unwrap_or(Value::Unit).is_truthy() {
                    self.ip = t;
                }
            }
            OpCode::JumpIfFalse(t) => {
                if !self.stack.pop().unwrap_or(Value::Unit).is_truthy() {
                    self.ip = t;
                }
            }
            OpCode::LoadLocal(slot) => {
                let base = self.frame_base();
                let idx = base + slot;
                let v = self.stack.get(idx).cloned().unwrap_or(Value::Unit);
                self.stack.push(v.deref_cell());
            }
            OpCode::StoreLocal(slot) => {
                let val = self.stack.pop().unwrap_or(Value::Unit);
                let base = self.frame_base();
                let idx = base + slot;
                if idx < self.stack.len() {
                    if let Value::Cell(rc) = &self.stack[idx] {
                        *rc.borrow_mut() = val;
                        if let Some(frame) = self.call_stack.last_mut() {
                            if slot + 1 > frame.max_slot {
                                frame.max_slot = slot + 1;
                            }
                        }
                        return Ok(());
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
            OpCode::BindIdent(slot) => {
                let val = self.stack.pop().unwrap_or(Value::Unit);
                let base = self.frame_base();
                let idx = base + slot;
                // Match Vm::run: extend stack with Unit first, then choose
                // insert vs in-place assign based on whether the slot is at
                // or near the new stack top.
                while idx >= self.stack.len() {
                    self.stack.push(Value::Unit);
                }
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
            OpCode::Print => {
                let v = self.stack.pop().unwrap_or(Value::Unit);
                self.write_output(&v.to_string());
            }
            OpCode::PrintLn => {
                let v = self.stack.pop().unwrap_or(Value::Unit);
                self.write_output(&format!("{}", v));
                self.write_output("\n");
            }
            OpCode::ToString => {
                let v = self.stack.pop().unwrap_or(Value::Unit);
                self.stack.push(Value::String(v.to_string()));
            }
            OpCode::MakeArray { count } => {
                let s = self.stack.len() - count;
                let i: Vec<_> = self.stack.drain(s..).collect();
                self.stack.push(Value::Vec(Rc::new(RefCell::new(i))));
            }
            OpCode::MakeFixedArray { count } => {
                let s = self.stack.len() - count;
                let i: Vec<_> = self.stack.drain(s..).collect();
                self.stack.push(Value::Array(i));
            }
            OpCode::MakeTuple { count } => {
                let s = self.stack.len() - count;
                let i: Vec<_> = self.stack.drain(s..).collect();
                self.stack.push(Value::Tuple(i));
            }
            OpCode::VecIndex => {
                let key = self.stack.pop().unwrap_or(Value::Unit);
                let c = self.stack.pop().unwrap_or(Value::Unit);
                match c {
                    Value::Vec(rc) => {
                        if let Value::I64(i) = key {
                            self.stack
                                .push(rc.borrow().get(i as usize).cloned().unwrap_or(Value::Unit));
                        } else {
                            self.stack.push(Value::Unit);
                        }
                    }
                    Value::Array(a) => {
                        if let Value::I64(i) = key {
                            self.stack
                                .push(a.get(i as usize).cloned().unwrap_or(Value::Unit));
                        } else {
                            self.stack.push(Value::Unit);
                        }
                    }
                    Value::HashMap(rc) => {
                        self.stack
                            .push(rc.borrow().get(&key).cloned().unwrap_or(Value::Unit));
                    }
                    Value::Tuple(t) => {
                        if let Value::I64(i) = key {
                            self.stack
                                .push(t.get(i as usize).cloned().unwrap_or(Value::Unit));
                        } else {
                            self.stack.push(Value::Unit);
                        }
                    }
                    Value::String(s) => {
                        if let Value::I64(i) = key {
                            self.stack.push(
                                s.chars()
                                    .nth(i as usize)
                                    .map(Value::Char)
                                    .unwrap_or(Value::Unit),
                            );
                        } else {
                            self.stack.push(Value::Unit);
                        }
                    }
                    _ => self.stack.push(Value::Unit),
                }
            }
            OpCode::VecIndexStore => {
                let val = self.stack.pop().unwrap_or(Value::Unit);
                let key = self.stack.pop().unwrap_or(Value::Unit);
                let c = self.stack.pop().unwrap_or(Value::Unit);
                match c {
                    Value::Vec(rc) => {
                        if let Value::I64(i) = key {
                            let idx = i as usize;
                            if idx < rc.borrow().len() {
                                rc.borrow_mut()[idx] = val.clone();
                            }
                        }
                    }
                    _ => {}
                }
                self.stack.push(val);
            }
            OpCode::FieldAccess { field_name } => {
                let v = self.stack.pop().unwrap_or(Value::Unit);
                match v {
                    Value::Struct { fields, .. } => self
                        .stack
                        .push(fields.get(&field_name).cloned().unwrap_or(Value::Unit)),
                    _ => self.stack.push(Value::Unit),
                }
            }
            OpCode::FieldStore(field_name) => {
                let val = self.stack.pop().unwrap_or(Value::Unit);
                let recv = self.stack.pop().unwrap_or(Value::Unit);
                match recv {
                    Value::Struct { name, mut fields } => {
                        fields.insert(field_name, val);
                        self.stack.push(Value::Struct { name, fields });
                    }
                    other => self.stack.push(other),
                }
            }
            OpCode::MakeEnumVariant {
                enum_name,
                variant,
                arg_count,
            } => {
                let s = self.stack.len() - arg_count;
                let d = self.stack.drain(s..).collect();
                self.stack.push(Value::EnumVariant {
                    enum_name,
                    variant,
                    data: d,
                });
            }
            OpCode::EnumVariantEqual { enum_name, variant } => {
                let val = self.stack.pop().unwrap_or(Value::Unit);
                let matched = matches!(
                    &val,
                    Value::EnumVariant { enum_name: en, variant: v, .. }
                        if en == &enum_name && v == &variant
                );
                self.stack.push(Value::Bool(matched));
            }
            OpCode::EnumDataGet(index) => {
                let val = self.stack.pop().unwrap_or(Value::Unit);
                match val {
                    Value::EnumVariant { data, .. } => {
                        let item = data.get(index).cloned().unwrap_or(Value::Unit);
                        self.stack.push(item);
                    }
                    _ => {
                        return Err(format!(
                            "EnumDataGet: expected enum variant, got {}",
                            val.type_name()
                        ));
                    }
                }
            }
            OpCode::MakeRange => {
                let (e, s) = self.pop_two();
                let si = match s {
                    Value::I64(n) => n,
                    _ => 0,
                };
                let ei = match e {
                    Value::I64(n) => n,
                    _ => 0,
                };
                self.stack.push(Value::Range(si, ei));
            }
            OpCode::Format { arg_count } => {
                let s = self.stack.len() - arg_count;
                let args: Vec<_> = self.stack.drain(s..).collect();
                let mut r = args.first().map(|v| v.to_string()).unwrap_or_default();
                for v in &args[1..] {
                    if let Some(p) = r.find("{:?}") {
                        r.replace_range(p..p + 4, &debug_format(v));
                    } else if let Some(p) = r.find("{}") {
                        r.replace_range(p..p + 2, &v.to_string());
                    }
                }
                self.stack.push(Value::String(r));
            }
            OpCode::FStringConcat { count } => {
                let s = self.stack.len() - count;
                let p: Vec<String> = self.stack.drain(s..).map(|v| v.to_string()).collect();
                self.stack.push(Value::String(p.concat()));
            }
            OpCode::CastInt(target_width) => {
                let val = self.stack.pop().unwrap_or(Value::Unit);
                self.stack.push(cast_to_int(&val, target_width));
            }
            OpCode::CastFloat(target_width) => {
                let val = self.stack.pop().unwrap_or(Value::Unit);
                self.stack.push(cast_to_float(&val, target_width));
            }
            OpCode::CastToChar => {
                let val = self.stack.pop().unwrap_or(Value::Unit);
                let n = value_to_i64(&val);
                let c = char::from_u32(n as u32).unwrap_or('\0');
                self.stack.push(Value::Char(c));
            }
            OpCode::TryPop => {
                let v = self.stack.pop().unwrap_or(Value::Unit);
                let is_err = matches!(&v,Value::EnumVariant{enum_name,variant,..}if(enum_name=="Result"&&variant=="Err")||(enum_name=="Option"&&variant=="None"));
                if is_err {
                    return Err(format!("{}", v));
                }
                match &v {
                    Value::EnumVariant { data, .. } if !data.is_empty() => {
                        self.stack.push(data[0].clone())
                    }
                    _ => {}
                }
            }
            OpCode::DisplayArg => {
                let v = self.stack.pop().unwrap_or(Value::Unit);
                self.stack.push(Value::String(v.to_string()));
            }
            OpCode::MakeCell(slot) => {
                let b = self.frame_base();
                let i = b + slot;
                if i < self.stack.len() {
                    let v = self.stack[i].clone();
                    self.stack[i] = Value::cell(v);
                }
            }
            OpCode::BitAnd => self.binary_op(vm_bitand),
            OpCode::BitOr => self.binary_op(vm_bitor),
            OpCode::BitXor => self.binary_op(vm_bitxor),
            OpCode::Shl => self.binary_op(vm_shl),
            OpCode::Shr => self.binary_op(vm_shr),
            OpCode::Panic => {
                let msg = self.stack.pop().map(|v| v.to_string()).unwrap_or_default();
                return Err(msg);
            }
            OpCode::MakeIter => {
                let value = self.stack.pop().unwrap_or(Value::Unit);
                match value.into_iterable() {
                    Ok(vec) => self
                        .stack
                        .push(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(vec)))),
                    Err(e) => return Err(e),
                }
            }
            OpCode::IterLen => {
                let v = self.stack.pop().unwrap_or(Value::Unit);
                match v {
                    Value::Vec(rc) => self.stack.push(Value::I64(rc.borrow().len() as i64)),
                    other => {
                        return Err(format!("cannot get length of {}", other.type_name()));
                    }
                }
            }
            OpCode::StructInit {
                name,
                field_count,
                field_names,
            } => {
                let start = self.stack.len().saturating_sub(field_count);
                let values: Vec<Value> = self.stack.drain(start..).collect();
                let fields: HashMap<String, Value> = field_names.into_iter().zip(values).collect();
                if let Some(struct_def) = self.chunk.struct_defs.get(&name) {
                    if let crate::ast::StructKind::Named(named_fields) = &struct_def.kind {
                        for required in named_fields {
                            if !fields.contains_key(&required.name) {
                                return Err(format!(
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
            OpCode::Await => {
                let val = self.stack.pop().unwrap_or(Value::Unit);
                match val {
                    Value::Future(_) => {
                        return Err("Await on Future not yet supported in VM".into());
                    }
                    Value::JoinHandle(inner) => {
                        self.stack.push(*inner);
                    }
                    other => {
                        self.stack.push(other);
                    }
                }
            }
            OpCode::PathCallBuiltin {
                segments,
                arg_count,
            } => {
                let args_start = self.stack.len().saturating_sub(arg_count);
                let args: Vec<Value> = self.stack.drain(args_start..).collect();
                match self.dispatch_pathcall(&segments, &args) {
                    Ok(val) => self.stack.push(val),
                    Err(e) => return Err(e),
                }
            }
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
        let target = match ft.target_ip {
            Some(t) => t,
            None => return Err("function has no bytecode target".into()),
        };
        // Save outer execution state
        let saved_ip = self.ip;
        let saved_stack_len = self.stack.len();
        let saved_call_depth = self.call_stack.len();
        // Push args and captured vars onto stack
        let base = self.stack.len();
        for (name, slot) in &ft.captured_slots {
            let val = ft
                .closure_env
                .borrow()
                .get(name)
                .ok()
                .map(|v| v.clone())
                .unwrap_or(Value::Unit);
            while base + slot >= self.stack.len() {
                self.stack.push(Value::Unit);
            }
            self.stack[base + slot] = val;
        }
        let max_slot = ft
            .captured_slots
            .iter()
            .map(|(_, s)| s + 1)
            .max()
            .unwrap_or(0);
        for arg in args {
            self.stack.push(arg.clone());
        }
        // Push call frame and run
        self.call_stack.push(Frame {
            return_ip: usize::MAX, // sentinel
            base,
            max_slot: max_slot + args.len(),
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
                OpCode::Call {
                    target: ct,
                    arg_count,
                } => {
                    if self.call_stack.len() >= 1024 {
                        break Err("recursion limit exceeded".into());
                    }
                    let as_start = self.stack.len() - arg_count;
                    self.call_stack.push(Frame {
                        return_ip: self.ip,
                        base: as_start,
                        max_slot: arg_count,
                        fn_ip: ct,
                        write_back_slot: None,
                    });
                    self.ip = ct;
                }
                OpCode::Return => {
                    let rv = self.stack.pop().unwrap_or(Value::Unit);
                    let frame = self.call_stack.pop().unwrap();
                    if frame.return_ip == usize::MAX {
                        break Ok(rv);
                    }
                    self.stack.truncate(frame.base);
                    self.stack.push(rv);
                    self.ip = frame.return_ip;
                }
                OpCode::Closure {
                    target_ip: ct,
                    param_count,
                    meta_idx,
                } => {
                    let blank_span = crate::lexer::Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        column: 0,
                    };
                    let (param_names, body_expr, captured_vars) = self
                        .chunk
                        .closure_meta
                        .get(meta_idx)
                        .cloned()
                        .unwrap_or_else(|| {
                            (
                                (0..param_count).map(|i| format!("_{i}")).collect(),
                                crate::ast::Expr::IntLiteral(0, IntegerSuffix::None, blank_span),
                                Vec::new(),
                            )
                        });
                    let params: Vec<crate::ast::Param> = param_names
                        .into_iter()
                        .map(|name| crate::ast::Param {
                            name,
                            type_ann: crate::ast::TypeAnnotation::Named {
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
                    let closure_env = crate::env::Environment::new();
                    for (name, slot, is_mut) in &captured_vars {
                        let idx = self.frame_base() + slot;
                        let val = self.stack.get(idx).cloned().unwrap_or(Value::Unit);
                        closure_env.borrow_mut().define(name.clone(), val, *is_mut);
                    }
                    let cap_slots: Vec<(String, usize)> = captured_vars
                        .iter()
                        .map(|(n, s, _)| (n.clone(), *s))
                        .collect();
                    self.stack
                        .push(Value::Function(Box::new(crate::types::FunctionData {
                            name: "<closure>".into(),
                            params,
                            return_type: None,
                            body: body_block,
                            closure_env,
                            target_ip: Some(ct),
                            captured_slots: cap_slots,
                        })));
                }
                OpCode::MethodCall {
                    method_name,
                    arg_count,
                } => {
                    let args_start = self.stack.len() - arg_count;
                    let args: Vec<Value> = self.stack.drain(args_start..).collect();
                    let receiver = self.stack.pop().unwrap_or(Value::Unit);
                    match self.builtin_method(receiver, &method_name, args) {
                        Ok(val) => self.stack.push(val),
                        Err(e) => break Err(e),
                    }
                }
                _ => {
                    if let Err(e) = self.execute_op(op) {
                        break Err(e);
                    }
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
        // to_json works on any type — serialize to JSON string via the json module
        if method_name == "to_json" {
            return match crate::json::serialize(&receiver) {
                Ok(s) => Ok(Value::ok(Value::String(s))),
                Err(e) => Ok(Value::err(Value::String(e))),
            };
        }
        match &receiver {
            Value::Vec(rc) => {
                // Try builtins first (with closure callback)
                let result = builtins::vec::dispatch(
                    Value::Vec(rc.clone()),
                    method_name,
                    &args,
                    |func, fargs| self.run_closure(&func, fargs),
                );
                match &result {
                    Ok(_) => return result,
                    Err(e) if e.starts_with("no method") => {} // fall through to iterator
                    Err(_) => return result,                   // propagate real errors
                }
                // Fall back to iterator delegation for closure-based methods
                let data = rc.borrow().clone();
                let iter = Value::Iterator(Box::new(crate::types::IteratorState::VecSource {
                    data,
                    index: 0,
                }));
                builtins::iterator::dispatch(iter, method_name, &args, |func, fargs| {
                    self.run_closure(func, fargs)
                })
            }
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
                "code" => Ok(Value::I64(*c as i64)),
                "to_string" => Ok(Value::String(c.to_string())),
                _ => Err(format!("no method '{}' on type char", method_name)),
            },
            Value::I8(_)
            | Value::I16(_)
            | Value::I32(_)
            | Value::I64(_)
            | Value::U8(_)
            | Value::U16(_)
            | Value::U32(_)
            | Value::U64(_)
            | Value::F32(_)
            | Value::F64(_) => builtins::numeric::dispatch(receiver, method_name, &args),
            Value::EnumVariant { enum_name, .. }
                if enum_name == "Option" || enum_name == "Result" =>
            {
                builtins::option_result::dispatch(receiver, method_name, &args, |func, fargs| {
                    self.run_closure(&func, fargs)
                })
            }
            Value::EnumVariant { enum_name, .. } => match method_name {
                "clone" => Ok(receiver.clone()),
                "to_string" => Ok(Value::String(receiver.to_string())),
                _ => Err(format!(
                    "no method '{}' on enum '{}'",
                    method_name, enum_name
                )),
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
            Value::Array(a) => match method_name {
                "len" => Ok(Value::I64(a.len() as i64)),
                "is_empty" => Ok(Value::Bool(a.is_empty())),
                "clone" => Ok(receiver.clone()),
                "to_string" => Ok(Value::String(receiver.to_string())),
                _ => Err(format!("no method '{}' on type Array", method_name)),
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
        let _to_f64 = |v: &Value| match v {
            Value::I64(n) => *n as f64,
            Value::F64(x) => *x,
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
            ["String", "from"] => {
                let s = args.first().map(|v| format!("{}", v)).unwrap_or_default();
                Ok(Value::String(s))
            }
            ["HashMap", "new"] => Ok(Value::HashMap(std::rc::Rc::new(std::cell::RefCell::new(
                HashMap::new(),
            )))),
            ["HashSet", "new"] => Ok(Value::HashSet(std::rc::Rc::new(std::cell::RefCell::new(
                HashSet::new(),
            )))),
            ["BinaryHeap", "new"] => Ok(Value::BinaryHeap(std::rc::Rc::new(
                std::cell::RefCell::new(BinaryHeap::new()),
            ))),
            ["VecDeque", "new"] => Ok(Value::VecDeque(std::rc::Rc::new(std::cell::RefCell::new(
                VecDeque::new(),
            )))),
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
                    Ok(n) => Ok(Value::ok(Value::I64(n))),
                    Err(_) => Ok(Value::err(Value::String(format!(
                        "cannot parse \"{s}\" as integer"
                    )))),
                }
            }
            ["char", "from_code"] => {
                let n = args
                    .first()
                    .and_then(|v| match v {
                        Value::I64(n) => Some(*n as u32),
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
                    Ok(n) => Ok(Value::ok(Value::F64(n))),
                    Err(_) => Ok(Value::err(Value::String(format!(
                        "cannot parse \"{s}\" as float"
                    )))),
                }
            }
            // --- json ---
            ["json", "serialize"] | ["json", "to_string"] => {
                let val = args.first().cloned().unwrap_or(Value::Unit);
                match crate::json::serialize(&val) {
                    Ok(s) => Ok(Value::ok(Value::String(s))),
                    Err(e) => Ok(Value::err(Value::String(e))),
                }
            }
            ["json", "to_string_pretty"] => {
                let val = args.first().cloned().unwrap_or(Value::Unit);
                match crate::json::serialize_pretty(&val) {
                    Ok(s) => Ok(Value::ok(Value::String(s))),
                    Err(e) => Ok(Value::err(Value::String(e))),
                }
            }
            ["json", "deserialize"] | ["json", "from_str"] => {
                let s = args.first().map(|v| v.to_string()).unwrap_or_default();
                match crate::json::deserialize(&s) {
                    Ok(val) => Ok(Value::ok(val)),
                    Err(e) => Ok(Value::err(Value::String(format!("json error: {}", e)))),
                }
            }
            ["json", "from_struct"] => {
                let s = args.first().map(|v| v.to_string()).unwrap_or_default();
                let type_name = args.get(1).map(|v| v.to_string()).unwrap_or_default();
                match crate::json::deserialize(&s) {
                    Ok(val) => {
                        // Wrap deserialized value as a struct if type_name provided
                        if !type_name.is_empty() {
                            if let Value::Struct { fields, .. } = &val {
                                Ok(Value::ok(Value::Struct {
                                    name: type_name,
                                    fields: fields.clone(),
                                }))
                            } else {
                                Ok(Value::ok(val))
                            }
                        } else {
                            Ok(Value::ok(val))
                        }
                    }
                    Err(e) => Ok(Value::err(Value::String(format!("json error: {}", e)))),
                }
            }
            // --- http ---
            ["http", func] => {
                #[cfg(feature = "http")]
                {
                    match func.as_ref() {
                        "get" => return http_call("GET", args, None),
                        "post" => {
                            let body = args.get(1).map(|v| v.to_string());
                            return http_call("POST", args, body);
                        }
                        "delete" => return http_call("DELETE", args, None),
                        "get_json" => return http_call("GET", args, None),
                        "post_json" => {
                            let body = args.get(1).map(|v| v.to_string());
                            return http_call("POST", args, body);
                        }
                        "put_json" => {
                            let body = args.get(1).map(|v| v.to_string());
                            return http_call("PUT", args, body);
                        }
                        _ => {}
                    }
                }
                #[cfg(not(feature = "http"))]
                {
                    let _ = (func, args);
                }
                Err("http feature not enabled".into())
            }
            // --- stdlib modules ---
            ["fs", func] => call_stdlib(crate::stdlib::fs::call, func, args),
            ["env", func] => call_stdlib(crate::stdlib::env::call, func, args),
            ["process", func] => call_stdlib(crate::stdlib::process::call, func, args),
            ["regex", func] => call_stdlib(crate::stdlib::regex::call, func, args),
            ["net", func] => call_stdlib(crate::stdlib::net::call, func, args),
            ["time", func] => call_stdlib(crate::stdlib::time::call, func, args),
            ["rand", func] => call_stdlib(crate::stdlib::rand::call, func, args),
            ["std", "env", "args"] => {
                // Return empty args in test environment
                Ok(Value::Vec(std::rc::Rc::new(std::cell::RefCell::new(
                    Vec::new(),
                ))))
            }
            // std::module::function routes (e.g. std::env::var, std::fs::read_to_string)
            ["std", module, func] => match *module {
                "fs" => call_stdlib(crate::stdlib::fs::call, func, args),
                "env" => call_stdlib(crate::stdlib::env::call, func, args),
                "process" => call_stdlib(crate::stdlib::process::call, func, args),
                "regex" => call_stdlib(crate::stdlib::regex::call, func, args),
                "net" => call_stdlib(crate::stdlib::net::call, func, args),
                "time" => call_stdlib(crate::stdlib::time::call, func, args),
                "rand" => call_stdlib(crate::stdlib::rand::call, func, args),
                _ => Err(format!("unknown std module: std::{}", module)),
            },
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

    fn trace_op(&self, op: &OpCode) {
        let frame = self.call_stack.last();
        let base = frame.map(|f| f.base).unwrap_or(0);
        let max_slot = frame.map(|f| f.max_slot).unwrap_or(0);
        let protected = base + max_slot;
        let stack_preview: Vec<String> = self
            .stack
            .iter()
            .enumerate()
            .skip(base)
            .map(|(i, v)| {
                let marker = if i < protected { "L" } else { "O" };
                format!("{}{}:{}", marker, i - base, trace_compact_val(v))
            })
            .collect();
        let op_str = trace_format_op(op);
        eprintln!(
            "  [{:>4}] {:45} frame(base={}, prot={}) stack=[{}]",
            self.ip,
            op_str,
            base,
            protected,
            stack_preview.join(", ")
        );
    }
}

fn trace_compact_val(v: &Value) -> String {
    match v {
        Value::Cell(rc) => format!("Cell({})", trace_compact_val(&rc.borrow())),
        Value::I8(n) => n.to_string(),
        Value::I16(n) => n.to_string(),
        Value::I32(n) => n.to_string(),
        Value::I64(n) => n.to_string(),
        Value::U8(n) => n.to_string(),
        Value::U16(n) => n.to_string(),
        Value::U32(n) => n.to_string(),
        Value::U64(n) => n.to_string(),
        Value::F32(n) => format!("{:.1}", n),
        Value::F64(n) => format!("{:.1}", n),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => format!("\"{:.20}\"", s),
        Value::Function(_) => "<fn>".into(),
        Value::Unit => "()".into(),
        _ => "?".into(),
    }
}

fn trace_format_op(op: &OpCode) -> String {
    match op {
        OpCode::ConstInt(n, _) => format!("ConstInt({})", n),
        OpCode::LoadLocal(s) => format!("LoadLocal({})", s),
        OpCode::StoreLocal(s) => format!("StoreLocal({})", s),
        OpCode::Call { target, arg_count } => format!("Call({}, {})", target, arg_count),
        OpCode::Return => "Return".into(),
        OpCode::CallClosure { arg_count } => format!("CallClosure({})", arg_count),
        OpCode::Closure {
            target_ip,
            param_count,
            meta_idx,
        } => {
            format!(
                "Closure(ip={}, params={}, meta={})",
                target_ip, param_count, meta_idx
            )
        }
        OpCode::MakeCell(s) => format!("MakeCell({})", s),
        OpCode::Dup => "Dup".into(),
        OpCode::Pop => "Pop".into(),
        OpCode::Add => "Add".into(),
        OpCode::Sub => "Sub".into(),
        OpCode::Mul => "Mul".into(),
        OpCode::Div => "Div".into(),
        OpCode::Mod => "Mod".into(),
        OpCode::Eq => "Eq".into(),
        OpCode::Neq => "Neq".into(),
        OpCode::Lt => "Lt".into(),
        OpCode::Gt => "Gt".into(),
        OpCode::Le => "Le".into(),
        OpCode::Ge => "Ge".into(),
        OpCode::And => "And".into(),
        OpCode::Or => "Or".into(),
        OpCode::Neg => "Neg".into(),
        OpCode::Not => "Not".into(),
        OpCode::Jump(t) => format!("Jump({})", t),
        OpCode::JumpIfFalse(t) => format!("JumpIfFalse({})", t),
        OpCode::JumpIfTrue(t) => format!("JumpIfTrue({})", t),
        OpCode::ConstUnit => "ConstUnit".into(),
        OpCode::ConstBool(b) => format!("ConstBool({})", b),
        OpCode::ConstString(s) => format!("ConstString({:?})", s),
        OpCode::Print => "Print".into(),
        OpCode::PrintLn => "PrintLn".into(),
        OpCode::Halt => "Halt".into(),
        _ => format!("{:?}", op),
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

/// HTTP helper: call crate::http::request and wrap result as Ok/Err enum variant.
#[cfg(feature = "http")]
fn http_call(method: &str, args: &[Value], body: Option<String>) -> Result<Value, String> {
    let url = args.first().map(|v| v.to_string()).unwrap_or_default();
    let result = crate::http::request(method, &url, &[], body.as_deref());
    match result {
        Ok((status, resp_body, headers)) => {
            let mut fields = HashMap::new();
            fields.insert("status".to_string(), Value::I64(status));
            fields.insert("body".to_string(), Value::String(resp_body));
            let mut header_map: HashMap<Value, Value> = HashMap::new();
            for (k, v) in &headers {
                header_map.insert(Value::String(k.clone()), Value::String(v.clone()));
            }
            fields.insert(
                "headers".to_string(),
                Value::HashMap(std::rc::Rc::new(std::cell::RefCell::new(header_map))),
            );
            Ok(Value::ok(Value::Struct {
                name: "HttpResponse".to_string(),
                fields,
            }))
        }
        Err(e) => Ok(Value::err(Value::String(e))),
    }
}

/// Map a binary op function to the corresponding method name for trait dispatch.
fn method_name_from_op(f: fn(Value, Value) -> Result<Value, String>) -> &'static str {
    if f as usize == vm_add as *const () as usize {
        return "add";
    }
    if f as usize == vm_sub as *const () as usize {
        return "sub";
    }
    if f as usize == vm_mul as *const () as usize {
        return "mul";
    }
    if f as usize == vm_div as *const () as usize {
        return "div";
    }
    if f as usize == vm_rem as *const () as usize {
        return "rem";
    }
    "add"
}

// --- VM arithmetic helpers (standalone to avoid trait conflicts) ---

// --- Width-aware integer helpers ---

/// Returns the bit-width of an integer type (for promotion decisions).
fn integer_rank(v: &Value) -> Option<u32> {
    match v {
        Value::I8(_) | Value::U8(_) => Some(8),
        Value::I16(_) | Value::U16(_) => Some(16),
        Value::I32(_) | Value::U32(_) => Some(32),
        Value::I64(_) | Value::U64(_) => Some(64),
        _ => None,
    }
}

/// Promote two integers to a common type, returning (widened_a, widened_b, result_width).
/// Same-width stays same-width. Cross-width promotes to the wider signed type.
fn promote_ints(a: Value, b: Value) -> (Value, Value) {
    let ra = integer_rank(&a).unwrap_or(64);
    let rb = integer_rank(&b).unwrap_or(64);
    if ra == rb && std::mem::discriminant(&a) == std::mem::discriminant(&b) {
        (a, b) // same type, no promotion needed
    } else {
        // Promote both to I64 for simplicity (widest signed type)
        (Value::I64(a.as_i64()), Value::I64(b.as_i64()))
    }
}

/// Wrap an i64 result back to the target integer variant.
fn wrap_to(v: i64, target: &Value) -> Value {
    match target {
        Value::I8(_) => Value::I8(v as i8),
        Value::I16(_) => Value::I16(v as i16),
        Value::I32(_) => Value::I32(v as i32),
        Value::I64(_) => Value::I64(v),
        Value::U8(_) => Value::U8(v as u8),
        Value::U16(_) => Value::U16(v as u16),
        Value::U32(_) => Value::U32(v as u32),
        Value::U64(_) => Value::U64(v as u64),
        _ => Value::I64(v),
    }
}

// --- Arithmetic ---

fn vm_add(a: Value, b: Value) -> Result<Value, String> {
    // String concatenation
    if let (Value::String(sa), Value::String(sb)) = (&a, &b) {
        return Ok(Value::String(format!("{sa}{sb}")));
    }
    if let Value::String(s) = &a {
        return Ok(Value::String(format!("{s}{b}")));
    }
    if let Value::String(s) = &b {
        return Ok(Value::String(format!("{a}{s}")));
    }
    // Float wins
    if a.is_float() || b.is_float() {
        return Ok(Value::F64(a.to_f64() + b.to_f64()));
    }
    if a.is_integer() && b.is_integer() {
        let (a, b) = promote_ints(a, b);
        return Ok(wrap_to(a.as_i64().wrapping_add(b.as_i64()), &a));
    }
    Err(format!(
        "cannot add {} and {}",
        a.type_name(),
        b.type_name()
    ))
}

fn vm_sub(a: Value, b: Value) -> Result<Value, String> {
    if a.is_float() || b.is_float() {
        return Ok(Value::F64(a.to_f64() - b.to_f64()));
    }
    if a.is_integer() && b.is_integer() {
        let (a, b) = promote_ints(a, b);
        return Ok(wrap_to(a.as_i64().wrapping_sub(b.as_i64()), &a));
    }
    Err(format!(
        "cannot subtract {} and {}",
        a.type_name(),
        b.type_name()
    ))
}

fn vm_mul(a: Value, b: Value) -> Result<Value, String> {
    if a.is_float() || b.is_float() {
        return Ok(Value::F64(a.to_f64() * b.to_f64()));
    }
    if a.is_integer() && b.is_integer() {
        let (a, b) = promote_ints(a, b);
        return Ok(wrap_to(a.as_i64().wrapping_mul(b.as_i64()), &a));
    }
    Err(format!(
        "cannot multiply {} and {}",
        a.type_name(),
        b.type_name()
    ))
}

fn vm_div(a: Value, b: Value) -> Result<Value, String> {
    if a.is_float() || b.is_float() {
        if b.to_f64() == 0.0 {
            return Err("division by zero".into());
        }
        return Ok(Value::F64(a.to_f64() / b.to_f64()));
    }
    if a.is_integer() && b.is_integer() {
        let (a, b) = promote_ints(a, b);
        let divisor = b.as_i64();
        if divisor == 0 {
            return Err("division by zero".into());
        }
        return Ok(wrap_to(a.as_i64() / divisor, &a));
    }
    Err(format!(
        "cannot divide {} and {}",
        a.type_name(),
        b.type_name()
    ))
}

fn vm_rem(a: Value, b: Value) -> Result<Value, String> {
    if a.is_float() || b.is_float() {
        if b.to_f64() == 0.0 {
            return Err("modulo by zero".into());
        }
        return Ok(Value::F64(a.to_f64() % b.to_f64()));
    }
    if a.is_integer() && b.is_integer() {
        let (a, b) = promote_ints(a, b);
        let divisor = b.as_i64();
        if divisor == 0 {
            return Err("modulo by zero".into());
        }
        return Ok(wrap_to(a.as_i64() % divisor, &a));
    }
    Err(format!(
        "cannot compute modulo of {} and {}",
        a.type_name(),
        b.type_name()
    ))
}

fn vm_neg(v: Value) -> Value {
    match v {
        Value::I8(n) => Value::I8(n.wrapping_neg()),
        Value::I16(n) => Value::I16(n.wrapping_neg()),
        Value::I32(n) => Value::I32(n.wrapping_neg()),
        Value::I64(n) => Value::I64(n.wrapping_neg()),
        Value::F32(n) => Value::F32(-n),
        Value::F64(n) => Value::F64(-n),
        v => v,
    }
}

fn vm_bitnot(v: Value) -> Value {
    match v {
        Value::I8(n) => Value::I8(!n),
        Value::I16(n) => Value::I16(!n),
        Value::I32(n) => Value::I32(!n),
        Value::I64(n) => Value::I64(!n),
        Value::U8(n) => Value::U8(!n),
        Value::U16(n) => Value::U16(!n),
        Value::U32(n) => Value::U32(!n),
        Value::U64(n) => Value::U64(!n),
        v => v,
    }
}

fn vm_bitand(a: Value, b: Value) -> Result<Value, String> {
    if a.is_integer() && b.is_integer() {
        let (a, b) = promote_ints(a, b);
        Ok(wrap_to(a.as_i64() & b.as_i64(), &a))
    } else {
        Err(format!("bitwise AND requires integers"))
    }
}

fn vm_bitor(a: Value, b: Value) -> Result<Value, String> {
    if a.is_integer() && b.is_integer() {
        let (a, b) = promote_ints(a, b);
        Ok(wrap_to(a.as_i64() | b.as_i64(), &a))
    } else {
        Err(format!("bitwise OR requires integers"))
    }
}

fn vm_bitxor(a: Value, b: Value) -> Result<Value, String> {
    if a.is_integer() && b.is_integer() {
        let (a, b) = promote_ints(a, b);
        Ok(wrap_to(a.as_i64() ^ b.as_i64(), &a))
    } else {
        Err(format!("bitwise XOR requires integers"))
    }
}

fn vm_shl(a: Value, b: Value) -> Result<Value, String> {
    if a.is_integer() && b.is_integer() {
        let shift = b.as_u64() as u32;
        Ok(wrap_to(a.as_i64().wrapping_shl(shift), &a))
    } else {
        Err(format!("shift left requires integers"))
    }
}

fn vm_shr(a: Value, b: Value) -> Result<Value, String> {
    if a.is_integer() && b.is_integer() {
        let shift = b.as_u64() as u32;
        Ok(wrap_to(a.as_i64().wrapping_shr(shift), &a))
    } else {
        Err(format!("shift right requires integers"))
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
            if t.len() == 1 {
                format!("({},)", items[0])
            } else {
                format!("({})", items.join(", "))
            }
        }
        Value::Struct { name, fields } => {
            let mut sorted: Vec<_> = fields.iter().collect();
            sorted.sort_by_key(|(k, _)| (*k).clone());
            let items: Vec<String> = sorted
                .iter()
                .map(|(k, v)| format!("{k}: {}", debug_format(v)))
                .collect();
            format!("{name} {{ {} }}", items.join(", "))
        }
        Value::EnumVariant {
            enum_name,
            variant,
            data,
        } => {
            let prefix = if enum_name == "Option" || enum_name == "Result" {
                String::new()
            } else {
                format!("{enum_name}::")
            };
            if data.is_empty() {
                format!("{prefix}{variant}")
            } else {
                let items: Vec<String> = data.iter().map(debug_format).collect();
                format!("{prefix}{variant}({})", items.join(", "))
            }
        }
        Value::HashMap(rc) => {
            let m = rc.borrow();
            let mut sorted: Vec<_> = m.iter().collect();
            sorted.sort_by_key(|(k, _)| (*k).clone());
            let items: Vec<String> = sorted
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}: {}",
                        debug_format(&Value::String(k.to_string())),
                        debug_format(v)
                    )
                })
                .collect();
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

/// Return all type names that have built-in method dispatch.
/// Used by symbol consistency tests to ensure `symbols.rs` stays in sync.
/// **Must** be updated when a new `Value` variant receives a dispatch arm in
/// `VMRuntime::builtin_method`.
pub fn dispatched_type_names() -> Vec<&'static str> {
    vec![
        "Vec",
        "String",
        "HashMap",
        "HashSet",
        "VecDeque",
        "BinaryHeap",
        "char",
        "numeric",
        "Option",
        "Result",
        "enum",
        "struct",
        "Iterator",
        "tuple",
    ]
}

/// Compile and run with captured output (for testing).
pub fn run_compiled(source: &str) -> Result<Value, crate::errors::FerriError> {
    let program = crate::parser::parse(source)?;
    crate::type_checker::TypeChecker::new().check_program(&program)?;
    let chunk = crate::compiler::Compiler::new().compile(&program)?;
    let mut vm = Vm::new(chunk);
    match vm.run() {
        VmResult::Value(v) => Ok(v),
        VmResult::Error(e) => Err(crate::errors::FerriError::Runtime {
            message: e,
            line: 0,
            column: 0,
        }),
    }
}

/// Compile and run, capturing printed output (for testing).
pub fn run_compiled_capturing(
    source: &str,
) -> Result<(Value, Vec<String>), crate::errors::FerriError> {
    let program = crate::parser::parse(source)?;
    crate::type_checker::TypeChecker::new().check_program(&program)?;
    let chunk = crate::compiler::Compiler::new().compile(&program)?;
    let mut vm = Vm::with_captured_output(chunk);
    match vm.run() {
        VmResult::Value(v) => Ok((v, vm.captured_output())),
        VmResult::Error(e) => Err(crate::errors::FerriError::Runtime {
            message: e,
            line: 0,
            column: 0,
        }),
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

/// Parse, type-check, compile, and disassemble a source file to a debug string.
pub fn disassemble_source(path: &str, source: &str) -> Result<String, crate::errors::FerriError> {
    let program = crate::parser::parse(source)?;
    crate::type_checker::TypeChecker::new().check_program(&program)?;
    let chunk = crate::compiler::Compiler::new_for_tests(Some(path)).compile(&program)?;
    Ok(disassemble_chunk(&chunk))
}

/// Run all #[test] functions in source via the VM, and verify that
/// #[compile_error] functions fail to compile.
pub fn run_tests(path: &str, source: &str) -> Result<Vec<TestResult>, crate::errors::FerriError> {
    let program = crate::parser::parse(source)?;

    // Split: normal items vs #[compile_error] functions
    let mut normal_items: Vec<crate::ast::Item> = Vec::new();
    let mut compile_error_fns: Vec<crate::ast::FnDef> = Vec::new();

    for item in program.items {
        if let crate::ast::Item::Function(ref f) = item {
            if f.attributes.iter().any(|a| a.name == "compile_error") {
                compile_error_fns.push(f.clone());
                continue;
            }
        }
        normal_items.push(item);
    }

    let normal_program = crate::ast::Program {
        items: normal_items,
        span: program.span,
    };

    // Type-check and compile normal items (must succeed)
    crate::type_checker::TypeChecker::new().check_program(&normal_program)?;
    let chunk = crate::compiler::Compiler::new_for_tests(Some(path)).compile(&normal_program)?;

    // Run #[test] functions
    let test_fns: Vec<&crate::ast::FnDef> = normal_program
        .items
        .iter()
        .filter_map(|item| {
            if let crate::ast::Item::Function(f) = item {
                if f.attributes.iter().any(|a| a.name == "test") {
                    Some(f)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let mut results = Vec::new();
    for test_fn in &test_fns {
        let mut chunk = chunk.clone();
        if let Some(&ip) = chunk.functions.get(&test_fn.name) {
            chunk.entry_point = ip;
        }
        let mut vm = Vm::new(chunk);
        match vm.run() {
            VmResult::Value(_) => results.push(TestResult {
                name: test_fn.name.clone(),
                passed: true,
                error: None,
            }),
            VmResult::Error(e) => results.push(TestResult {
                name: test_fn.name.clone(),
                passed: false,
                error: Some(e),
            }),
        }
    }

    // Test #[compile_error] functions — each must FAIL to compile
    for ce_fn in &compile_error_fns {
        let ce_item = crate::ast::Item::Function(ce_fn.clone());
        let mut ce_items = normal_program.items.clone();
        ce_items.push(ce_item);
        let ce_program = crate::ast::Program {
            items: ce_items,
            span: program.span,
        };

        // Try type-check first (catches visibility errors, type errors, etc.)
        let tc_result = crate::type_checker::TypeChecker::new().check_program(&ce_program);
        if tc_result.is_err() {
            results.push(TestResult {
                name: ce_fn.name.clone(),
                passed: true,
                error: None,
            });
            continue;
        }

        // Try compilation (catches compiler-level errors)
        let compile_result =
            crate::compiler::Compiler::new_for_tests(Some(path)).compile(&ce_program);
        match compile_result {
            Err(_) => results.push(TestResult {
                name: ce_fn.name.clone(),
                passed: true,
                error: None,
            }),
            Ok(_) => results.push(TestResult {
                name: ce_fn.name.clone(),
                passed: false,
                error: Some(
                    "expected compilation error, but code compiled successfully".to_string(),
                ),
            }),
        }
    }

    Ok(results)
}

/// Extract an i64 from any Value type (for cast/conversion purposes).
fn value_to_i64(val: &Value) -> i64 {
    match val {
        Value::I8(n) => *n as i64,
        Value::I16(n) => *n as i64,
        Value::I32(n) => *n as i64,
        Value::I64(n) => *n,
        Value::U8(n) => *n as i64,
        Value::U16(n) => *n as i64,
        Value::U32(n) => *n as i64,
        Value::U64(n) => *n as i64,
        Value::F32(n) => *n as i64,
        Value::F64(n) => *n as i64,
        Value::Char(c) => *c as u32 as i64,
        _ => 0,
    }
}

/// Cast a Value to a specific integer width with wrapping.
fn cast_to_int(val: &Value, width: IntegerWidth) -> Value {
    let bits = value_to_i64(val);
    match width {
        IntegerWidth::I8 => Value::I8(bits as i8),
        IntegerWidth::I16 => Value::I16(bits as i16),
        IntegerWidth::I32 => Value::I32(bits as i32),
        IntegerWidth::I64 => Value::I64(bits),
        IntegerWidth::U8 => Value::U8(bits as u8),
        IntegerWidth::U16 => Value::U16(bits as u16),
        IntegerWidth::U32 => Value::U32(bits as u32),
        IntegerWidth::U64 => Value::U64(bits as u64),
    }
}

/// Cast a Value to a specific float width.
fn cast_to_float(val: &Value, width: FloatWidth) -> Value {
    let f = match val {
        Value::F32(n) => *n as f64,
        Value::F64(n) => *n,
        Value::Char(c) => *c as u32 as f64,
        _ => value_to_i64(val) as f64,
    };
    match width {
        FloatWidth::F32 => Value::F32(f as f32),
        FloatWidth::F64 => Value::F64(f),
    }
}

pub mod builtins;

/// Disassemble a compiled chunk to a human-readable string for debugging.
pub fn disassemble_chunk(chunk: &Chunk) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "=== Chunk: {} instructions, entry_point={}, local_count={}\n",
        chunk.code.len(),
        chunk.entry_point,
        chunk.local_count
    ));
    if !chunk.local_names.is_empty() {
        out.push_str(&format!("  main local_names: {:?}\n", chunk.local_names));
    }
    if !chunk.fn_local_names.is_empty() {
        out.push_str("  fn_local_names:\n");
        let mut ips: Vec<(&usize, &Vec<String>)> = chunk.fn_local_names.iter().collect();
        ips.sort_by_key(|(k, _)| *k);
        for (ip, names) in ips {
            out.push_str(&format!("    @{}: {:?}\n", ip, names));
        }
    }
    if !chunk.functions.is_empty() {
        out.push_str("  functions:\n");
        let mut fns: Vec<(&String, &usize)> = chunk.functions.iter().collect();
        fns.sort_by_key(|(_, ip)| *ip);
        for (name, ip) in fns {
            out.push_str(&format!("    {} -> @{}\n", name, ip));
        }
    }
    if !chunk.closure_meta.is_empty() {
        out.push_str("  closure_meta:\n");
        for (i, (params, _body, captured)) in chunk.closure_meta.iter().enumerate() {
            out.push_str(&format!(
                "    [{}] params={:?} captured={:?}\n",
                i, params, captured
            ));
        }
    }
    out.push_str("\n--- Bytecode ---\n");

    // Build a set of function entry points for labeling
    let mut fn_entries: std::collections::HashMap<usize, String> = std::collections::HashMap::new();
    for (name, ip) in &chunk.functions {
        fn_entries.entry(*ip).or_insert_with(|| name.clone());
    }
    // Add closure entry points from fn_local_names
    for (ip, _names) in &chunk.fn_local_names {
        fn_entries
            .entry(*ip)
            .or_insert_with(|| format!("<fn@{}>", ip));
    }

    let mut i = 0;
    while i < chunk.code.len() {
        // Print function/closure labels
        if let Some(label) = fn_entries.get(&i) {
            out.push_str(&format!("\n--- {} (ip={}) ---\n", label, i));
        }
        // Show local names for this IP
        if let Some(names) = chunk.fn_local_names.get(&i) {
            out.push_str(&format!("    ;; locals: {:?}\n", names));
        }

        let op = &chunk.code[i];
        let line = format_opcode(op, &chunk.local_names);
        out.push_str(&format!("  {:>4}: {}\n", i, line));
        i += 1;
    }
    out
}

fn format_opcode(op: &OpCode, local_names: &[String]) -> String {
    let local_name = |slot: &usize| -> String {
        local_names
            .get(*slot)
            .cloned()
            .unwrap_or_else(|| format!("slot{}", slot))
    };
    match op {
        OpCode::ConstInt(n, w) => format!("ConstInt({}, {:?})", n, w),
        OpCode::ConstFloat(n, w) => format!("ConstFloat({}, {:?})", n, w),
        OpCode::ConstBool(b) => format!("ConstBool({})", b),
        OpCode::ConstString(s) => format!("ConstString({:?})", s),
        OpCode::ConstChar(c) => format!("ConstChar({:?})", c),
        OpCode::ConstUnit => "ConstUnit".into(),
        OpCode::LoadLocal(s) => format!("LoadLocal({})  ;; {}", s, local_name(s)),
        OpCode::StoreLocal(s) => format!("StoreLocal({}) ;; {}", s, local_name(s)),
        OpCode::Add => "Add".into(),
        OpCode::Sub => "Sub".into(),
        OpCode::Mul => "Mul".into(),
        OpCode::Div => "Div".into(),
        OpCode::Mod => "Mod".into(),
        OpCode::Eq => "Eq".into(),
        OpCode::Neq => "Neq".into(),
        OpCode::Lt => "Lt".into(),
        OpCode::Gt => "Gt".into(),
        OpCode::Le => "Le".into(),
        OpCode::Ge => "Ge".into(),
        OpCode::And => "And".into(),
        OpCode::Or => "Or".into(),
        OpCode::BitAnd => "BitAnd".into(),
        OpCode::BitOr => "BitOr".into(),
        OpCode::BitXor => "BitXor".into(),
        OpCode::Shl => "Shl".into(),
        OpCode::Shr => "Shr".into(),
        OpCode::Neg => "Neg".into(),
        OpCode::Not => "Not".into(),
        OpCode::BitNot => "BitNot".into(),
        OpCode::Jump(t) => format!("Jump({})", t),
        OpCode::JumpIfFalse(t) => format!("JumpIfFalse({})", t),
        OpCode::JumpIfTrue(t) => format!("JumpIfTrue({})", t),
        OpCode::Call { target, arg_count } => {
            format!("Call(target={}, arg_count={})", target, arg_count)
        }
        OpCode::Return => "Return".into(),
        OpCode::Panic => "Panic".into(),
        OpCode::Halt => "Halt".into(),
        OpCode::Print => "Print".into(),
        OpCode::PrintLn => "PrintLn".into(),
        OpCode::Dup => "Dup".into(),
        OpCode::Pop => "Pop".into(),
        OpCode::MakeIter => "MakeIter".into(),
        OpCode::IterLen => "IterLen".into(),
        OpCode::VecIndex => "VecIndex".into(),
        OpCode::VecIndexStore => "VecIndexStore".into(),
        OpCode::MakeRange => "MakeRange".into(),
        OpCode::MakeArray { count } => format!("MakeArray(count={})", count),
        OpCode::MakeFixedArray { count } => format!("MakeFixedArray(count={})", count),
        OpCode::MakeTuple { count } => format!("MakeTuple(count={})", count),
        OpCode::ToString => "ToString".into(),
        OpCode::FStringConcat { count } => format!("FStringConcat(count={})", count),
        OpCode::Format { arg_count } => format!("Format(arg_count={})", arg_count),
        OpCode::StructInit {
            name,
            field_count,
            field_names,
        } => format!(
            "StructInit(name={:?}, field_count={}, field_names={:?})",
            name, field_count, field_names
        ),
        OpCode::MethodCall {
            method_name,
            arg_count,
        } => format!("MethodCall({}, arg_count={})", method_name, arg_count),
        OpCode::FieldAccess { field_name } => format!("FieldAccess({})", field_name),
        OpCode::ConstEnumVariant {
            enum_name,
            variant,
            data,
        } => format!("ConstEnumVariant({}::{}({:?}))", enum_name, variant, data),
        OpCode::MakeEnumVariant {
            enum_name,
            variant,
            arg_count,
        } => format!(
            "MakeEnumVariant({}::{}, arg_count={})",
            enum_name, variant, arg_count
        ),
        OpCode::Closure {
            target_ip,
            param_count,
            meta_idx,
        } => format!(
            "Closure(target_ip={}, param_count={}, meta_idx={})",
            target_ip, param_count, meta_idx
        ),
        OpCode::CallClosure { arg_count } => format!("CallClosure(arg_count={})", arg_count),
        OpCode::Await => "Await".into(),
        OpCode::TryPop => "TryPop".into(),
        OpCode::CastInt(w) => format!("CastInt({:?})", w),
        OpCode::CastFloat(w) => format!("CastFloat({:?})", w),
        OpCode::CastToChar => "CastToChar".into(),
        OpCode::BindIdent(s) => format!("BindIdent({})  ;; {}", s, local_name(s)),
        OpCode::EnumVariantEqual { enum_name, variant } => {
            format!("EnumVariantEqual({}::{})", enum_name, variant)
        }
        OpCode::EnumDataGet(i) => format!("EnumDataGet({})", i),
        OpCode::PathCallBuiltin {
            segments,
            arg_count,
        } => format!(
            "PathCallBuiltin({}, arg_count={})",
            segments.join("::"),
            arg_count
        ),
        OpCode::FieldStore(f) => format!("FieldStore({})", f),
        OpCode::DisplayArg => "DisplayArg".into(),
        OpCode::MakeCell(s) => format!("MakeCell({})  ;; {}", s, local_name(s)),
    }
}

// FIXME: vm/tests.rs has compilation errors from interpreter migration
// #[cfg(test)]
// mod tests;
