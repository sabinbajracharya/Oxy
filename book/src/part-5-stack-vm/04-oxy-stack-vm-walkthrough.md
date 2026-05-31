# Oxy's Stack VM: What It Was

<!-- OPUS_FILL
Write a 1-paragraph intro framing this as archaeology — like the tree-walker chapter.
"The stack VM lived for exactly one week: added May 13, removed May 28. It ran the full
feature suite — structs, closures, traits, async — before being replaced by the register IR."
Make it feel significant: a week of real work, a real working system, retired for good reasons.
-->

## The timeline

```
2026-05-13  feat: add bytecode compiler and stack-based VM
2026-05-14  refactor: rename language from Oxide to Oxy
...         (5 commits building on the stack VM)
2026-05-27  refactor: replace bytecode compiler with AST→Register IR generator
2026-05-28  refactor: remove bytecode compiler and VM (OpCode, Chunk, Vm)
```

15 days. In that window, Oxy had a bytecode compiler and stack-based VM as its execution
engine. Then the register IR replaced it.

To read the removed code: `git show fa87d96^:crates/oxy-core/src/vm/mod.rs`

## The `Vm` struct

```rust
// Reconstructed from git history (commit fa87d96^)
struct Vm {
    stack: Vec<Value>,        // operand stack
    frames: Vec<Frame>,       // call stack
    chunk: Chunk,             // compiled bytecode
    ip: usize,                // instruction pointer
}

struct Frame {
    return_ip: usize,
    locals: Vec<Value>,
    caller_op_stack_len: usize,
    fn_ip: usize,
}
```

Two stacks: the **operand stack** (`stack`) for expression evaluation, and the **call stack**
(`frames`) for function call management. These are separate — a design explicitly documented
in the commit messages:

> *"The locals/operand split was introduced to eliminate a recurring class of slot/stack-collision bugs."*
> — from `docs/history/vm-locals-stack-separation.md`

Early versions had both in one buffer, and collision bugs were constant. Separating them into
two `Vec<Value>` eliminated the entire class.

## The execution loop

```rust
fn run(&mut self) -> Result<Value, RuntimeError> {
    loop {
        let op = &self.chunk.code[self.ip];
        self.ip += 1;

        match op {
            OpCode::ConstInt(n, _) => {
                self.stack.push(Value::Int(*n));
            }
            OpCode::LoadLocal(idx) => {
                let val = self.current_frame().locals[*idx].clone();
                self.stack.push(val);
            }
            OpCode::StoreLocal(idx) => {
                let val = self.stack.pop().expect("stack underflow");
                self.current_frame_mut().locals[*idx] = val;
            }
            OpCode::Add => {
                let r = self.stack.pop().unwrap();
                let l = self.stack.pop().unwrap();
                self.stack.push(apply_add(l, r)?);
            }
            OpCode::JumpIfFalse(target) => {
                let val = self.stack.pop().unwrap();
                if !val.is_truthy() {
                    self.ip = *target;
                }
            }
            OpCode::Call { target, arg_count } => {
                let args: Vec<Value> = self.stack
                    .drain(self.stack.len() - arg_count ..)
                    .collect();
                let frame_size = self.chunk.fn_frame_sizes[target];
                let mut locals = vec![Value::Unit; frame_size];
                for (i, arg) in args.into_iter().enumerate() {
                    locals[i] = arg;
                }
                self.frames.push(Frame {
                    return_ip: self.ip,
                    locals,
                    caller_op_stack_len: self.stack.len(),
                    fn_ip: *target,
                });
                self.ip = *target;
            }
            OpCode::Return => {
                let result = self.stack.pop().unwrap_or(Value::Unit);
                let frame = self.frames.pop().unwrap();
                self.stack.truncate(frame.caller_op_stack_len);
                self.ip = frame.return_ip;
                self.stack.push(result);
            }
            // ... ~30 more opcodes
        }
    }
}
```

The core loop: fetch instruction at `ip`, increment `ip`, dispatch on instruction type.
Every instruction is O(1). The loop runs until `Return` from the top-level function.

## What the stack VM could do

In 15 days, the stack VM supported:
- Integers, floats, strings, booleans, unit
- Closures with capture (by value)
- Structs and enums (stored as `Value::Struct`/`Value::Enum`)
- Methods via `impl` blocks
- Traits and operator overloading
- Pattern matching
- Async/await (`MakeFuture`, `Await` opcodes)
- The full stdlib (HTTP, JSON, file I/O — all called as `CallBuiltin`)

The reason it could do all of this: built-in operations were handled by `CallBuiltin(name, arg_count)`,
which called into the same stdlib that the tree-walker used. The VM provided the execution
model; the stdlib provided the built-in behavior. Two largely independent concerns.

## The architecture it foreshadowed

The stack VM introduced a pattern that the register IR inherited directly:

```
AST → compiler → flat instruction sequence → executor → Value
```

The compiler does the tree traversal. The executor is a flat loop over instructions.
This separation is the key architectural advance over tree-walking.

The register IR is the same pattern with a better instruction set — one that maps more
directly to what Cranelift (and CPUs) can do efficiently.
