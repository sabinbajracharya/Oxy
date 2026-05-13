//! Stack-based virtual machine for executing compiled Oxide bytecode.
//!
//! The VM executes a flat sequence of [`OpCode`]s produced by the compiler.
//! It uses a value stack and a call stack. Each call frame tracks its own
//! local variable slots and return address.

use crate::types::Value;

/// Bytecode instructions for the Oxide VM.
#[derive(Debug, Clone)]
pub enum OpCode {
    // --- Constants ---
    ConstInt(i64),
    ConstFloat(f64),
    ConstBool(bool),
    ConstString(String),
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
}

/// A compiled Oxide program: a flat sequence of opcodes.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<OpCode>,
    /// Number of local variable slots needed for the top-level scope.
    pub local_count: usize,
    /// Entry points: function name → instruction index.
    pub functions: std::collections::HashMap<String, usize>,
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
}

struct Frame {
    return_ip: usize,
    base: usize,
    #[allow(dead_code)]
    local_count: usize,
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
        vm.output = Some(Vec::new());
        vm
    }

    /// Get captured output lines.
    pub fn captured_output(&self) -> &[String] {
        self.output.as_deref().unwrap_or(&[])
    }

    /// Execute the chunk, starting at the given instruction index (default: 0).
    pub fn run(&mut self) -> VmResult {
        // Allocate local slots for the top-level scope
        for _ in 0..self.chunk.local_count {
            self.stack.push(Value::Unit);
        }

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
                OpCode::ConstUnit => self.stack.push(Value::Unit),

                OpCode::LoadLocal(slot) => {
                    // Find the correct frame's base
                    let base = self.frame_base();
                    let val = self.stack.get(base + slot).cloned().unwrap_or(Value::Unit);
                    self.stack.push(val);
                }

                OpCode::StoreLocal(slot) => {
                    let val = self.stack.pop().unwrap_or(Value::Unit);
                    let base = self.frame_base();
                    if base + slot < self.stack.len() {
                        self.stack[base + slot] = val;
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
                    // Arguments are on the stack in order (last arg on top).
                    // We leave them on the stack — they become the callee's locals.
                    let args_start = self.stack.len() - arg_count;
                    // Save frame: return IP, stack base (where the callee's locals start), local count
                    self.call_stack.push(Frame {
                        return_ip: self.ip + 1,
                        base: args_start,
                        local_count: 0, // callee will allocate its own locals
                    });
                    self.ip = target;
                    continue;
                }

                OpCode::Return => {
                    let result = self.stack.pop().unwrap_or(Value::Unit);
                    if let Some(frame) = self.call_stack.pop() {
                        // Truncate stack to frame base, push result
                        self.stack.truncate(frame.base);
                        self.stack.push(result);
                        self.ip = frame.return_ip;
                    } else {
                        // Top-level return: stop
                        return VmResult::Value(result);
                    }
                    continue;
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
                    self.stack.pop();
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

    fn pop_two(&mut self) -> (Value, Value) {
        let b = self.stack.pop().unwrap_or(Value::Unit);
        let a = self.stack.pop().unwrap_or(Value::Unit);
        (a, b)
    }

    fn frame_base(&self) -> usize {
        self.call_stack.last().map(|f| f.base).unwrap_or(0)
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

#[cfg(test)]
mod tests;
