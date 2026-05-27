// FIXME: remove when JIT is wired into the execution path (Phase 6)
#![allow(dead_code)]
//! Runtime context passed to JIT-compiled functions.
//!
//! # Layout
//!
//! The `JitContext` holds the operand stack, local variables, and async state
//! for a single execution context. It is passed as a mutable pointer to every
//! JIT-compiled function.
//!
//! # Memory layout
//!
//! The stack buffer is laid out as:
//! ```text
//! [locals: local_count * sizeof(Value)] [operand stack: grows upward]
//! ```
//!
//! The `sp` field tracks the current operand stack depth in units of `Value`.

use crate::types::Value;

/// Maximum operand stack depth (in Value units) before reallocation.
const DEFAULT_STACK_CAP: usize = 2048;

/// Runtime execution context for JIT-compiled code.
///
/// This is the only parameter passed to every JIT-compiled Oxy function.
/// It carries the operand stack, locals, result slot, and async yield state.
#[repr(C)]
pub(crate) struct JitContext {
    /// Pointer to the combined locals + operand stack buffer.
    /// Layout: first `local_count` slots are locals; above that grows the operand stack.
    pub buffer: *mut Value,
    /// Number of local variable slots (fixed per function frame).
    pub local_count: usize,
    /// Current operand stack depth (0 = empty stack, grows upward from locals_end).
    pub sp: usize,
    /// Total capacity of the buffer (in Value units).
    pub capacity: usize,

    /// Where to resume execution after a yield (bytecode IP).
    /// 0 means start from the beginning.
    pub resume_ip: usize,
    /// The JIT function's entry bytecode IP (used to look up the native fn pointer).
    pub entry_ip: usize,

    /// Yield reason: 0=none, 1=sleep, 2=await_task, 3=select.
    pub yield_reason: u32,
    /// Associated task ID or wake time for the yield reason.
    pub yield_data: u64,

    /// Completion value (set when a function returns Done).
    pub result: Value,

    /// Error message buffer (fixed-size, no heap allocation in FFI).
    pub error_msg: [u8; 1024],
    /// Length of the error message in the buffer.
    pub error_len: usize,

    /// Pointer to the JIT engine's function pointer table (for closure calls).
    pub fn_table: *const *const u8,
    /// Number of entries in fn_table, indexed by (ip - base_ip) / alignment.
    pub fn_table_len: usize,
    /// Base instruction pointer for indexing into fn_table.
    pub fn_table_base_ip: usize,

    /// Captured output buffer (if non-null, print goes here instead of stdout).
    /// This is a `*const Rc<RefCell<Vec<String>>>` pointer.
    pub output: *const std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}

impl JitContext {
    /// Create a new context with pre-allocated stack buffer.
    pub fn new(local_count: usize) -> Self {
        let capacity = local_count + DEFAULT_STACK_CAP;
        let layout =
            std::alloc::Layout::array::<Value>(capacity).expect("JitContext buffer layout");
        let buffer = unsafe { std::alloc::alloc_zeroed(layout) as *mut Value };

        Self {
            buffer,
            local_count,
            sp: 0,
            capacity,
            resume_ip: 0,
            entry_ip: 0,
            yield_reason: 0,
            yield_data: 0,
            result: Value::Unit,
            error_msg: [0u8; 1024],
            error_len: 0,
            fn_table: std::ptr::null(),
            fn_table_len: 0,
            fn_table_base_ip: 0,
            output: std::ptr::null(),
        }
    }

    /// Push a value onto the operand stack.
    /// Returns a pointer to the slot where the value should be written.
    pub fn push_slot(&mut self) -> *mut Value {
        if self.local_count + self.sp >= self.capacity {
            self.grow();
        }
        let slot = unsafe { self.buffer.add(self.local_count + self.sp) };
        self.sp += 1;
        slot
    }

    /// Pop a value from the operand stack.
    /// Caller is responsible for reading the value from the returned pointer.
    pub fn pop_slot(&mut self) -> *mut Value {
        if self.sp == 0 {
            return std::ptr::null_mut();
        }
        self.sp -= 1;
        unsafe { self.buffer.add(self.local_count + self.sp) }
    }

    /// Get a pointer to a local variable slot.
    pub fn local_slot(&self, index: usize) -> *mut Value {
        assert!(index < self.local_count);
        unsafe { self.buffer.add(index) }
    }

    /// Grow the stack buffer.
    fn grow(&mut self) {
        let new_capacity = self.capacity * 2;
        let new_layout =
            std::alloc::Layout::array::<Value>(new_capacity).expect("JitContext grow layout");
        let new_buffer = unsafe { std::alloc::alloc_zeroed(new_layout) as *mut Value };
        unsafe {
            std::ptr::copy_nonoverlapping(self.buffer, new_buffer, self.capacity);
            std::alloc::dealloc(
                self.buffer as *mut u8,
                std::alloc::Layout::array::<Value>(self.capacity).unwrap(),
            );
        }
        self.buffer = new_buffer;
        self.capacity = new_capacity;
    }

    /// Reset operand stack without deallocating.
    pub fn reset_stack(&mut self) {
        // Drop values on the stack to avoid leaks
        for i in 0..self.sp {
            unsafe {
                std::ptr::drop_in_place(self.buffer.add(self.local_count + i));
            }
        }
        self.sp = 0;
    }

    /// Reset async yield state for a fresh execution.
    pub fn reset_async_state(&mut self) {
        self.resume_ip = 0;
        self.yield_reason = 0;
        self.yield_data = 0;
    }
}

impl Drop for JitContext {
    fn drop(&mut self) {
        // Drop locals
        for i in 0..self.local_count {
            unsafe {
                std::ptr::drop_in_place(self.buffer.add(i));
            }
        }
        // Drop stack values
        for i in 0..self.sp {
            unsafe {
                std::ptr::drop_in_place(self.buffer.add(self.local_count + i));
            }
        }
        // Free buffer
        unsafe {
            let layout = std::alloc::Layout::array::<Value>(self.capacity).unwrap();
            std::alloc::dealloc(self.buffer as *mut u8, layout);
        }
    }
}

// SAFETY: JitContext owns its buffer and doesn't share it across threads
// in the current single-threaded scheduler design.
unsafe impl Send for JitContext {}
