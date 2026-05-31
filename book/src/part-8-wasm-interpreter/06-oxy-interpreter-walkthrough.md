# Oxy's Interpreter: A Full Walkthrough

<!-- OPUS_FILL
1-paragraph intro. "Open interp.rs. It's about 1,100 lines. Let's understand the key parts."
-->

**File:** `crates/oxy-core/src/vm/interp.rs`

---

## The `InterpEngine` struct

```rust
pub(crate) struct InterpEngine {
    functions: Vec<IrFunction>,
    name_to_index: HashMap<String, usize>,
    tables: JitTables,
    ffi_table: HashMap<&'static str, (*const u8, FfiRet)>,
}
```

- `functions`: the IR functions from `ir_gen`, same as what the JIT gets
- `name_to_index`: function name → index in `functions`
- `tables`: the `JitTables` struct — shared between JIT and interpreter. On the interpreter path, `fn_table` is empty (no native function pointers), but other tables (closure metadata, etc.) are populated
- `ffi_table`: built from `ffi_symbols()` — maps FFI function names to their raw Rust function pointers

## Entry point: `InterpEngine::compile`

```rust
pub(crate) fn compile(program: &Program) -> Result<Self, String> {
    // Run ir_gen — same as the JIT does
    let mut ir = IrGen::new();
    ir.gen_program(program);
    let functions = ir.functions;

    // Build name index
    let mut name_to_index = HashMap::new();
    for f in &functions {
        name_to_index.insert(f.name.clone(), f.fn_index);
    }

    // Build FFI table from ffi_symbols()
    let ffi_table = ffi::ffi_symbols();

    // Install the closure-invoker hook (see next chapter)
    let engine_ptr = /* ... */;
    ffi::set_interp_invoke(Some(engine_ptr));

    Ok(InterpEngine { functions, name_to_index, tables, ffi_table })
}
```

The critical step: `ffi::set_interp_invoke`. This installs a thread-local hook that
lets the shared Rust runtime (inside `oxy_map`, `oxy_filter`, etc.) call back into the
interpreter when it needs to invoke a user closure. Without this hook, higher-order
builtins would call an empty `fn_table` and do nothing.

## The `call_named` helper

The most-used internal method:

```rust
fn call_named(
    &self,
    ctx: &mut JitContext,
    func: &str,
    args: &[Value],
    immediates: &[usize],
    strings: &[(&[u8], usize)],
) {
    // Push Value args onto the operand stack
    for arg in args {
        unsafe { ffi::push(ctx, arg.clone()); }
    }

    // Look up the FFI function pointer
    let (ptr, abi) = self.ffi_table[func];

    // Call it with ctx + immediates + string pointers
    match abi {
        FfiRet::Void => {
            let f: extern "C" fn(*mut JitContext, ...) = unsafe { std::mem::transmute(ptr) };
            // ... call f with (ctx, immediates..., strings...)
        }
        FfiRet::I64 => {
            // ... returns an i64 result
        }
    }
}
```

The `transmute` is the unavoidable `unsafe` here: we know the function pointer's signature
(from `ffi_decls()`), but Rust's type system cannot express "function with variable argument list."
The transmute reinterprets the raw `*const u8` as the right function type.

## The `binary` helper

For `Add`, `Sub`, `Mul`, etc.:

```rust
fn binary(&self, ctx: &mut JitContext, regs: &mut HashMap<Reg, Value>,
    func: &str, r: Reg, a: Reg, b: Reg)
{
    let av = Self::reg_val(regs, a);
    let bv = Self::reg_val(regs, b);
    self.call_named(ctx, func, &[av, bv], &[], &[]);
    let result = unsafe { ffi::pop(ctx) };
    regs.insert(r, result);
}
```

Push both operands, call `oxy_add` (or whichever operation), pop the result. This is identical
to what the JIT does — the difference is that the JIT emits native call instructions while
the interpreter calls through a function pointer.

## Recursive function calls

When the interpreter executes `IrOp::CallBuiltin { func: "oxy_call", strings: ["add"], ... }`:

1. The FFI function `oxy_call` is called
2. `oxy_call` looks up `"add"` in `tables.fn_table`
3. `fn_table` is empty (interpreter path, no native pointers)
4. `fn_table` miss → falls through to the closure-invoker hook
5. The hook calls `engine.interpret(ctx, &functions[add_fn_index])`

The interpreter calls into itself recursively. Each recursive call creates a new `regs`
map and processes the called function's blocks. The `JitContext` buffer handles local variable
storage; the call stack is Rust's own call stack.

## The register file: `HashMap<Reg, Value>`

Unlike the JIT (which maps registers to Cranelift SSA values), the interpreter stores
registers directly as `Value`:

```rust
let mut regs: HashMap<Reg, Value> = HashMap::new();
```

`regs[r]` is the current value in virtual register `r`. Constants insert directly:
```rust
IrOp::ConstInt(r, n) => { regs.insert(*r, Value::I64(*n)); }
```

FFI results are popped from the operand stack and inserted:
```rust
let result = unsafe { ffi::pop(ctx) };
regs.insert(*result_reg, result);
```

The `HashMap<Reg, Value>` is recreated fresh on each function call. This is O(1) per
access but has allocation overhead. For the wasm playground use case (not performance-critical),
this is acceptable.
